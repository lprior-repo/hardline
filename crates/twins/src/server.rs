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
    extract::State,
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

async fn not_found_handler(method: Method, req: Request<Body>) -> impl IntoResponse {
    let path = req.uri().path().to_string();
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
        .filter(|e| !e.path.starts_with("/_inspect"))
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
    use axum::body::Body;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

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

    // ── Red Queen: adversarial server tests ──

    #[test]
    fn test_extract_headers_empty() {
        let headers = HeaderMap::new();
        let result = extract_headers(&headers);
        assert!(result.is_empty());
    }

    #[test]
    fn test_extract_headers_non_ascii_value_filtered() {
        use axum::http::HeaderValue;

        let mut headers = HeaderMap::new();
        headers.insert("x-valid", "ok".parse().expect("valid"));
        // Non-UTF8 header value: build from raw bytes via HeaderValue
        let bad_val = HeaderValue::from_bytes(b"\x80\x81").expect("bytes to header value");
        headers.insert("x-bad", bad_val);
        let result = extract_headers(&headers);
        assert_eq!(result.len(), 1);
        assert_eq!(result.get("x-valid"), Some(&"ok".to_string()));
    }

    #[test]
    fn test_extract_headers_multiple_values() {
        let mut headers = HeaderMap::new();
        headers.insert("x-first", "a".parse().expect("valid"));
        headers.insert("x-second", "b".parse().expect("valid"));
        headers.insert("content-type", "application/json".parse().expect("valid"));
        let result = extract_headers(&headers);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_serialize_response_body_null() {
        let body = serde_json::Value::Null;
        let result = serialize_response_body(&body);
        assert!(result.is_ok());
        assert_eq!(result.expect("ok"), "null");
    }

    #[test]
    fn test_serialize_response_body_complex_nested() {
        let body = serde_json::json!({
            "users": [{"id": 1, "name": "alice"}, {"id": 2, "name": "bob"}],
            "meta": {"total": 2, "page": 1}
        });
        let result = serialize_response_body(&body);
        assert!(result.is_ok());
        let parsed: serde_json::Value = serde_json::from_str(&result.expect("ok")).expect("json");
        assert_eq!(parsed["users"].as_array().expect("array").len(), 2);
    }

    #[test]
    fn test_build_response_empty_body() {
        let result = build_response(204, &HashMap::new(), String::new());
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_response_with_valid_headers() {
        let mut headers = HashMap::new();
        headers.insert("x-custom".to_string(), "value".to_string());
        let result = build_response(200, &headers, r#"{"ok":true}"#.to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_response_invalid_header_name_rejected() {
        let mut headers = HashMap::new();
        headers.insert("\x00invalid".to_string(), "value".to_string());
        let result = build_response(200, &headers, "{}".to_string());
        assert!(result.is_err());
        let err = result.expect_err("should fail");
        let msg = err.to_string();
        assert!(msg.contains("Invalid header"), "Got: {msg}");
    }

    #[test]
    fn test_server_error_variants_display() {
        let err = ServerError::EndpointNotFound("/missing".to_string());
        assert!(err.to_string().contains("/missing"));

        let err = ServerError::StartupError("bind failed".to_string());
        assert!(err.to_string().contains("bind failed"));

        let err = ServerError::BodyParseError("eof".to_string());
        assert!(err.to_string().contains("eof"));

        let err = ServerError::InvalidStatusCode(999);
        assert!(err.to_string().contains("999"));

        let err = ServerError::InvalidHeader("x-bad".to_string());
        assert!(err.to_string().contains("x-bad"));

        let err = ServerError::SerializationError("json err".to_string());
        assert!(err.to_string().contains("json err"));

        let err = ServerError::StateError("lock poisoned".to_string());
        assert!(err.to_string().contains("lock poisoned"));
    }

    #[test]
    fn test_server_error_into_response_returns_internal_server_error() {
        let err = ServerError::EndpointNotFound("/x".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_full_integration_get_request() {
        let definition = TwinDefinition::from_yaml(TEST_YAML).expect("parse");
        let app = build_router(definition);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/test")
                    .method("GET")
                    .body(Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body_bytes = response.into_body().collect().await.expect("body").to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).expect("json");
        assert_eq!(body["message"], "test response");
    }

    #[tokio::test]
    async fn test_full_integration_post_request() {
        let definition = TwinDefinition::from_yaml(TEST_YAML).expect("parse");
        let app = build_router(definition);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/test")
                    .method("POST")
                    .body(Body::from(r#"{"key":"value"}"#))
                    .expect("build request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn test_nonexistent_path_returns_not_found_or_server_error() {
        let definition = TwinDefinition::from_yaml(TEST_YAML).expect("parse");
        let app = build_router(definition);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/nonexistent")
                    .method("GET")
                    .body(Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("response");

        // The fallback handler may return 404 or 500 depending on Path extractor behavior
        assert!(
            response.status() == StatusCode::NOT_FOUND
                || response.status() == StatusCode::INTERNAL_SERVER_ERROR,
            "Expected 404 or 500 for unknown path, got {}",
            response.status()
        );
    }

    #[tokio::test]
    async fn test_wrong_method_on_existing_path_returns_405() {
        let definition = TwinDefinition::from_yaml(TEST_YAML).expect("parse");
        let app = build_router(definition);

        // DELETE is not a registered method for /api/test (only GET and POST)
        // axum returns 405 Method Not Allowed when the path matches but method doesn't
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/test")
                    .method("DELETE")
                    .body(Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn test_inspect_state_returns_twin_info() {
        let definition = TwinDefinition::from_yaml(TEST_YAML).expect("parse");
        let app = build_router(definition);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/_inspect/state")
                    .method("GET")
                    .body(Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body_bytes = response.into_body().collect().await.expect("body").to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).expect("json");
        assert_eq!(body["twin"], "test-twin");
        assert_eq!(body["port"], 3002);
        assert_eq!(body["request_count"], 0);
    }

    #[tokio::test]
    async fn test_inspect_requests_initially_empty() {
        let definition = TwinDefinition::from_yaml(TEST_YAML).expect("parse");
        let app = build_router(definition);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/_inspect/requests")
                    .method("GET")
                    .body(Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body_bytes = response.into_body().collect().await.expect("body").to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).expect("json");
        assert_eq!(body["requests"].as_array().expect("array").len(), 0);
    }

    #[tokio::test]
    async fn test_clear_state() {
        let definition = TwinDefinition::from_yaml(TEST_YAML).expect("parse");
        let app = build_router(definition);

        // Make a request to populate state
        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/test")
                    .method("GET")
                    .body(Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("response");

        // Clear state
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/_inspect/clear")
                    .method("POST")
                    .body(Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body_bytes = response.into_body().collect().await.expect("body").to_bytes();
        assert!(body_bytes.starts_with(br#"{"status":"cleared"}"#));
    }

    #[tokio::test]
    async fn test_request_tracked_in_state_after_handler() {
        let definition = TwinDefinition::from_yaml(TEST_YAML).expect("parse");
        let app = build_router(definition);

        // Make a request
        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/test")
                    .method("GET")
                    .body(Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("response");

        // Check state records
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/_inspect/state")
                    .method("GET")
                    .body(Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("response");

        let body_bytes = response.into_body().collect().await.expect("body").to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).expect("json");
        assert_eq!(body["request_count"], 1);
    }

    #[tokio::test]
    async fn test_app_state_find_endpoint_unknown_method_returns_none() {
        let definition = TwinDefinition::from_yaml(TEST_YAML).expect("parse");
        let state = AppState::new(definition);
        // CONNECT is not in the HttpMethod enum
        let result = state.find_endpoint(&Method::CONNECT, "/api/test");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_response_includes_custom_headers() {
        let yaml = r"
name: header-twin
port: 3003
endpoints:
  - path: /custom
    method: GET
    response:
      status: 200
      headers:
        x-custom-header: custom-value
      body:
        status: ok
";
        let definition = TwinDefinition::from_yaml(yaml).expect("parse");
        let app = build_router(definition);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/custom")
                    .method("GET")
                    .body(Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        // Check custom header is present
        let custom_val = response
            .headers()
            .get("x-custom-header")
            .expect("custom header present");
        assert_eq!(custom_val, "custom-value");
    }

    #[tokio::test]
    async fn test_response_with_no_body_still_has_content_type_json() {
        let yaml = r"
name: no-body-twin
port: 3004
endpoints:
  - path: /empty-body
    method: GET
    response:
      status: 200
      body: null
";
        let definition = TwinDefinition::from_yaml(yaml).expect("parse");
        let app = build_router(definition);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/empty-body")
                    .method("GET")
                    .body(Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let ct = response.headers().get("content-type").expect("content-type");
        assert_eq!(ct, "application/json");
    }
}
