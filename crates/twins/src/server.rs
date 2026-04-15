#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! HTTP server module for twin runtime
//!
//! Provides an axum-based HTTP server that serves twin endpoints.

use std::{collections::HashMap, sync::Arc};

use axum::{
    body::Body,
    extract::{Path, State},
    http::{header::HeaderName, HeaderMap, Method, Request, StatusCode},
    response::{IntoResponse, Response},
    routing::{any, delete, get, head, options, patch, post, put},
    Router,
};
use im::Vector;
use thiserror::Error;
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;

use crate::{
    definition::{Endpoint, HttpMethod, TwinDefinition},
    state::{InMemoryTwinState, RequestRecord, TwinState},
};

// ---------------------------------------------------------------------------
// Pure calculation functions (Data → Calc)
// ---------------------------------------------------------------------------

/// Converts `HeaderMap` to a `HashMap` of String → String
fn extract_headers(headers: &HeaderMap) -> HashMap<String, String> {
    headers
        .iter()
        .filter_map(|(k, v)| v.to_str().ok().map(|s| (k.to_string(), s.to_string())))
        .collect()
}

/// Serializes response body to JSON string
fn serialize_response_body(body: &serde_json::Value) -> Result<String, ServerError> {
    serde_json::to_string(body).map_err(|e| ServerError::SerializationError(e.to_string()))
}

/// Builds a Response with headers and optional body
fn build_response(
    status: u16,
    headers: &HashMap<String, String>,
    body: String,
) -> Result<Response, ServerError> {
    let builder =
        headers
            .iter()
            .try_fold(Response::builder().status(status), |acc, (key, value)| {
                let name = HeaderName::from_bytes(key.as_bytes())
                    .map_err(|_| ServerError::InvalidHeader(key.clone()))?;
                Ok(acc.header(&name, value.as_str()))
            })?;

    if body.is_empty() {
        builder
            .body(Body::empty())
            .map_err(|e| ServerError::StateError(e.to_string()))
    } else {
        builder
            .header("content-type", "application/json")
            .body(Body::from(body))
            .map_err(|e| ServerError::StateError(e.to_string()))
    }
}

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("Failed to parse request body: {0}")]
    BodyParseError(String),
    #[error("Endpoint not found: {0}")]
    EndpointNotFound(String),
    #[error("Failed to start server: {0}")]
    StartupError(String),
    #[error("Invalid state: {0}")]
    StateError(String),
    #[error("Failed to serialize response: {0}")]
    SerializationError(String),
    #[error("Invalid HTTP status code: {0}")]
    InvalidStatusCode(u16),
    #[error("Invalid header: {0}")]
    InvalidHeader(String),
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let body = self.to_string();
        (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
    }
}

#[derive(Clone)]
pub struct AppState<T = InMemoryTwinState>
where
    T: TwinState + Send + Sync,
{
    pub definition: TwinDefinition,
    pub state: Arc<RwLock<T>>,
}

impl<T> AppState<T>
where
    T: TwinState + Send + Sync,
{
    #[must_use]
    pub fn new(definition: TwinDefinition) -> Self
    where
        T: Default,
    {
        Self {
            definition,
            state: Arc::new(RwLock::new(T::default())),
        }
    }

    #[must_use]
    pub fn with_state(definition: TwinDefinition, state: T) -> Self {
        Self {
            definition,
            state: Arc::new(RwLock::new(state)),
        }
    }

    #[must_use]
    pub fn find_endpoint(&self, method: &Method, path: &str) -> Option<&Endpoint> {
        let http_method = match method.as_str() {
            "GET" => HttpMethod::GET,
            "POST" => HttpMethod::POST,
            "PUT" => HttpMethod::PUT,
            "DELETE" => HttpMethod::DELETE,
            "PATCH" => HttpMethod::PATCH,
            "OPTIONS" => HttpMethod::OPTIONS,
            "HEAD" => HttpMethod::HEAD,
            _ => return None,
        };

        self.definition
            .endpoints
            .iter()
            .find(|e| e.method == http_method && e.path == path)
    }

    pub async fn add_record(&self, record: RequestRecord)
    where
        T: Clone,
    {
        let new_state = {
            let state_guard = self.state.read().await;
            state_guard.add_record(record)
        };
        *self.state.write().await = new_state;
    }

    pub async fn get_records(&self) -> Vector<RequestRecord>
    where
        T: Clone,
    {
        let state_guard = self.state.read().await;
        state_guard.get_records()
    }

    pub async fn record_count(&self) -> usize {
        let state_guard = self.state.read().await;
        state_guard.record_count()
    }

    pub async fn clear_state(&self)
    where
        T: Clone,
    {
        let new_state = {
            let state_guard = self.state.read().await;
            state_guard.clear()
        };
        *self.state.write().await = new_state;
    }
}

async fn twin_handler<T>(
    State(state): State<AppState<T>>,
    method: Method,
    headers: HeaderMap,
    request: Request<Body>,
) -> Result<Response, ServerError>
where
    T: TwinState + Clone + Send + Sync,
{
    let path = request.uri().path().to_string();

    let Some(endpoint) = state.find_endpoint(&method, &path) else {
        return Ok((
            StatusCode::NOT_FOUND,
            format!("No endpoint found for {method} {path}"),
        )
            .into_response());
    };

    let body_bytes = axum::body::to_bytes(request.into_body(), 1024 * 1024)
        .await
        .map_err(|e| ServerError::BodyParseError(e.to_string()))?;

    let request_body_str = if body_bytes.is_empty() {
        None
    } else {
        String::from_utf8(body_bytes.to_vec()).ok()
    };

    let request_headers = extract_headers(&headers);

    let record = RequestRecord::new(
        method.to_string(),
        path.clone(),
        request_headers,
        request_body_str,
        endpoint.response.status,
        endpoint.response.headers.clone(),
        None,
    );

    state.add_record(record).await;

    let response_body = serialize_response_body(&endpoint.response.body)?;

    build_response(
        endpoint.response.status,
        &endpoint.response.headers,
        response_body,
    )
}

async fn not_found_handler(method: Method, Path(path): Path<String>) -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        format!("No endpoint found for {method} {path}"),
    )
}

async fn inspect_state<T>(State(state): State<AppState<T>>) -> Result<impl IntoResponse, ServerError>
where
    T: TwinState + Clone + Send + Sync,
{
    let records = state.get_records().await;
    let count = state.record_count().await;

    let response = serde_json::json!({
        "twin": state.definition.name,
        "port": state.definition.port,
        "request_count": count,
        "requests": records
    });

    let body = serde_json::to_string(&response)
        .map_err(|e| ServerError::SerializationError(e.to_string()))?;

    Ok((StatusCode::OK, body))
}

async fn inspect_requests<T>(State(state): State<AppState<T>>) -> Result<impl IntoResponse, ServerError>
where
    T: TwinState + Clone + Send + Sync,
{
    let records = state.get_records().await;
    let records_vec: Vec<_> = records.into_iter().collect();

    let response = serde_json::json!({
        "requests": records_vec
    });

    let body = serde_json::to_string(&response)
        .map_err(|e| ServerError::SerializationError(e.to_string()))?;

    Ok((StatusCode::OK, body))
}

async fn clear_state<T>(State(state): State<AppState<T>>) -> impl IntoResponse
where
    T: TwinState + Clone + Send + Sync,
{
    state.clear_state().await;

    (StatusCode::OK, r#"{"status":"cleared"}"#)
}

pub fn build_router(definition: TwinDefinition) -> Router {
    let app_state: AppState<InMemoryTwinState> = AppState::new(definition);

    let base_router = Router::new()
        .route("/_inspect/state", get(inspect_state))
        .route("/_inspect/requests", get(inspect_requests))
        .route("/_inspect/clear", post(clear_state));

    let routes: Vec<_> = app_state
        .definition
        .endpoints
        .iter()
        .map(|endpoint| (endpoint.path.clone(), endpoint.method))
        .collect();

    let built_router = routes
        .iter()
        .fold(base_router, |acc, (path, method)| match method {
            HttpMethod::GET => acc.route(path, get(twin_handler)),
            HttpMethod::POST => acc.route(path, post(twin_handler)),
            HttpMethod::PUT => acc.route(path, put(twin_handler)),
            HttpMethod::DELETE => acc.route(path, delete(twin_handler)),
            HttpMethod::PATCH => acc.route(path, patch(twin_handler)),
            HttpMethod::OPTIONS => acc.route(path, options(twin_handler)),
            HttpMethod::HEAD => acc.route(path, head(twin_handler)),
        });

    built_router
        .fallback(any(not_found_handler))
        .with_state(app_state)
        .layer(TraceLayer::new_for_http())
}

/// Starts the twin server with the given definition.
///
/// # Errors
///
/// Returns `ServerError::StartupError` if binding to the port fails.
pub async fn start_server(definition: TwinDefinition) -> Result<(), ServerError> {
    let port = definition.port;
    let router = build_router(definition);

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| ServerError::StartupError(e.to_string()))?;

    tracing::info!("Starting twin server on http://{addr}");

    axum::serve(listener, router)
        .await
        .map_err(|e| ServerError::StartupError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_YAML: &str = r"
name: test-twin
port: 3002
endpoints:
  - path: /api/test
    method: GET
    response:
      status: 200
      body:
        message: 'test response'
  - path: /api/test
    method: POST
    response:
      status: 201
      body:
        created: true
";

    #[test]
    fn test_build_router() {
        let definition = TwinDefinition::from_yaml(TEST_YAML);
        assert!(
            definition.is_ok(),
            "Should parse valid YAML: {:?}",
            definition.err()
        );
        let _router = build_router(definition.unwrap());
    }

    #[tokio::test]
    async fn test_find_endpoint() {
        let definition = TwinDefinition::from_yaml(TEST_YAML);
        assert!(
            definition.is_ok(),
            "Should parse valid YAML: {:?}",
            definition.err()
        );
        let definition = definition.unwrap();
        let state: AppState<InMemoryTwinState> = AppState::new(definition);

        let endpoint = state.find_endpoint(&Method::GET, "/api/test");
        assert!(endpoint.is_some());

        let endpoint = state.find_endpoint(&Method::POST, "/api/test");
        assert!(endpoint.is_some());

        let endpoint = state.find_endpoint(&Method::GET, "/nonexistent");
        assert!(endpoint.is_none());
    }
}
