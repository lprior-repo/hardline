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
use thiserror::Error;
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;

use crate::{
    definition::{Endpoint, HttpMethod, TwinDefinition},
    state::{InMemoryTwinState, RequestRecord, TwinState},
};

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
pub struct AppState {
    pub definition: TwinDefinition,
    pub state: Arc<RwLock<InMemoryTwinState>>,
}

impl AppState {
    #[must_use]
    pub fn new(definition: TwinDefinition) -> Self {
        Self {
            definition,
            state: Arc::new(RwLock::new(InMemoryTwinState::new())),
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
}

async fn twin_handler(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    request: Request<Body>,
) -> Result<Response, ServerError> {
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

    let request_headers: HashMap<String, String> = headers
        .iter()
        .filter_map(|(k, v)| v.to_str().ok().map(|s| (k.to_string(), s.to_string())))
        .collect();

    let response = &endpoint.response;
    let status = response.status;

    let builder = Response::builder().status(status);

    let builder = response
        .headers
        .iter()
        .try_fold(builder, |acc, (key, value)| {
            let name = HeaderName::from_bytes(key.as_bytes())
                .map_err(|_| ServerError::InvalidHeader(key.clone()))?;
            Ok(acc.header(&name, value.as_str()))
        })?;

    let response_body = serde_json::to_string(&response.body)
        .map_err(|e| ServerError::SerializationError(e.to_string()))?;

    let builder = if !response_body.is_empty() {
        builder.header("content-type", "application/json")
    } else {
        builder
    };

    let record = RequestRecord::new(
        method.to_string(),
        path,
        request_headers,
        request_body_str,
        response.status,
        response.headers.clone(),
        Some(response_body.clone()),
    );

    let new_state = {
        let state_guard = state.state.read().await;
        state_guard.add_record(record)
    };
    *state.state.write().await = new_state;

    let body = Body::from(response_body);
    builder
        .body(body)
        .map_err(|e| ServerError::StateError(e.to_string()))
}

async fn not_found_handler(method: Method, Path(path): Path<String>) -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        format!("No endpoint found for {method} {path}"),
    )
}

async fn inspect_state(State(state): State<AppState>) -> Result<impl IntoResponse, ServerError> {
    let records;
    let count;
    {
        let state_guard = state.state.read().await;
        records = state_guard.get_records();
        count = state_guard.record_count();
    }

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

async fn inspect_requests(State(state): State<AppState>) -> Result<impl IntoResponse, ServerError> {
    let records;
    {
        let state_guard = state.state.read().await;
        records = state_guard.get_records();
    }
    let records_vec: Vec<_> = records.into_iter().collect();

    let response = serde_json::json!({
        "requests": records_vec
    });

    let body = serde_json::to_string(&response)
        .map_err(|e| ServerError::SerializationError(e.to_string()))?;

    Ok((StatusCode::OK, body))
}

async fn clear_state(State(state): State<AppState>) -> impl IntoResponse {
    let new_state = {
        let state_guard = state.state.read().await;
        state_guard.clear()
    };
    *state.state.write().await = new_state;

    (StatusCode::OK, r#"{"status":"cleared"}"#)
}

pub fn build_router(definition: TwinDefinition) -> Router {
    let app_state = AppState::new(definition);

    let base_router = Router::new()
        .route("/_inspect/state", get(inspect_state))
        .route("/_inspect/requests", get(inspect_requests))
        .route("/_inspect/clear", post(clear_state));

    let routes: Vec<_> = app_state
        .definition
        .endpoints
        .iter()
        .map(|endpoint| {
            let path = endpoint.path.clone();
            let method = endpoint.method;
            (path, method)
        })
        .collect();

    let mut router = base_router;
    for (path, method) in routes {
        router = match method {
            HttpMethod::GET => router.route(&path, get(twin_handler)),
            HttpMethod::POST => router.route(&path, post(twin_handler)),
            HttpMethod::PUT => router.route(&path, put(twin_handler)),
            HttpMethod::DELETE => router.route(&path, delete(twin_handler)),
            HttpMethod::PATCH => router.route(&path, patch(twin_handler)),
            HttpMethod::OPTIONS => router.route(&path, options(twin_handler)),
            HttpMethod::HEAD => router.route(&path, head(twin_handler)),
        };
    }

    router
        .fallback(any(not_found_handler))
        .with_state(app_state)
        .layer(TraceLayer::new_for_http())
}

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
        let definition = TwinDefinition::from_yaml(TEST_YAML).expect("Should parse");
        let _router = build_router(definition);
    }

    #[tokio::test]
    async fn test_find_endpoint() {
        let definition = TwinDefinition::from_yaml(TEST_YAML).expect("Should parse");
        let state = AppState::new(definition);

        let endpoint = state.find_endpoint(&Method::GET, "/api/test");
        assert!(endpoint.is_some());

        let endpoint = state.find_endpoint(&Method::POST, "/api/test");
        assert!(endpoint.is_some());

        let endpoint = state.find_endpoint(&Method::GET, "/nonexistent");
        assert!(endpoint.is_none());
    }
}
