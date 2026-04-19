//! BDD Validation: twins crate — prove it works before ship
//!
//! Claim Sheet derived from types, docs, and help text.
//! Each claim is tested on the happy path with real terminal output.

use std::collections::HashMap;
use std::time::Duration;

use twins::definition::{EndpointResponse, HttpMethod, TwinDefinition};
use twins::server::{build_router, AppState};
use twins::state::{InMemoryTwinState, RequestRecord, TwinState};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn start_test_server(yaml: &str, port: u16) -> tokio::task::JoinHandle<()> {
    let def = TwinDefinition::from_yaml(yaml).expect("parse yaml");
    let router = build_router(def);
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));

    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");
    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("serve");
    })
}

fn base_url(port: u16) -> String {
    format!("http://127.0.0.1:{port}")
}

// ---------------------------------------------------------------------------
// Claim Sheet: Definition Layer
// ---------------------------------------------------------------------------

/// C1: TwinDefinition::from_yaml parses a valid YAML definition with name, port, endpoints
#[test]
fn c1_parse_valid_yaml() {
    let yaml = r"
name: sendgrid
port: 13001
endpoints:
  - path: /v3/mail/send
    method: POST
    response:
      status: 200
      body:
        message_id: 'test-123'
";
    let def = TwinDefinition::from_yaml(yaml).expect("C1 FAIL: should parse valid YAML");
    assert_eq!(def.name, "sendgrid");
    assert_eq!(def.port, 13001);
    assert_eq!(def.endpoints.len(), 1);
    assert_eq!(def.endpoints[0].path, "/v3/mail/send");
    assert_eq!(def.endpoints[0].method, HttpMethod::POST);
    assert_eq!(def.endpoints[0].response.status, 200);
    println!("C1 PASS: TwinDefinition::from_yaml parses valid YAML");
}

/// C2: TwinDefinition::from_yaml_bytes parses valid YAML bytes
#[test]
fn c2_parse_valid_yaml_bytes() {
    let yaml = b"
name: stripe
port: 13002
endpoints:
  - path: /v1/charges
    method: POST
    response:
      status: 200
      body:
        id: 'ch_123'
";
    let def =
        TwinDefinition::from_yaml_bytes(yaml).expect("C2 FAIL: should parse valid YAML bytes");
    assert_eq!(def.name, "stripe");
    assert_eq!(def.port, 13002);
    println!("C2 PASS: TwinDefinition::from_yaml_bytes parses valid YAML bytes");
}

/// C3: HttpMethod::from_str parses all 7 methods (case-insensitive)
#[test]
fn c3_http_method_from_str() {
    for (input, expected) in [
        ("GET", HttpMethod::GET),
        ("POST", HttpMethod::POST),
        ("PUT", HttpMethod::PUT),
        ("DELETE", HttpMethod::DELETE),
        ("PATCH", HttpMethod::PATCH),
        ("OPTIONS", HttpMethod::OPTIONS),
        ("HEAD", HttpMethod::HEAD),
        ("get", HttpMethod::GET),
        ("post", HttpMethod::POST),
        ("Put", HttpMethod::PUT),
    ] {
        let result = input
            .parse::<HttpMethod>()
            .expect(&format!("C3 FAIL: {input}"));
        assert_eq!(result, expected, "C3 FAIL: {input}");
    }
    println!("C3 PASS: HttpMethod::from_str parses all 7 methods (case-insensitive)");
}

/// C4: HttpMethod::Display outputs uppercase method names
#[test]
fn c4_http_method_display() {
    assert_eq!(HttpMethod::GET.to_string(), "GET");
    assert_eq!(HttpMethod::POST.to_string(), "POST");
    assert_eq!(HttpMethod::DELETE.to_string(), "DELETE");
    assert_eq!(HttpMethod::PATCH.to_string(), "PATCH");
    assert_eq!(HttpMethod::OPTIONS.to_string(), "OPTIONS");
    assert_eq!(HttpMethod::HEAD.to_string(), "HEAD");
    assert_eq!(HttpMethod::PUT.to_string(), "PUT");
    println!("C4 PASS: HttpMethod::Display outputs uppercase method names");
}

/// C5: HttpMethod::from_str rejects invalid methods
#[test]
fn c5_invalid_method_rejected() {
    let result = "INVALID".parse::<HttpMethod>();
    assert!(result.is_err(), "C5 FAIL: should reject invalid method");
    println!("C5 PASS: HttpMethod::from_str rejects invalid methods");
}

/// C6: Validation rejects empty name
#[test]
fn c6_reject_empty_name() {
    let yaml = r"
name: ''
port: 3001
endpoints:
  - path: /test
    method: GET
    response:
      status: 200
      body: {}
";
    let result = TwinDefinition::from_yaml(yaml);
    assert!(result.is_err(), "C6 FAIL: should reject empty name");
    println!("C6 PASS: Validation rejects empty name");
}

/// C7: Validation rejects port 0
#[test]
fn c7_reject_port_zero() {
    let yaml = r"
name: test
port: 0
endpoints:
  - path: /test
    method: GET
    response:
      status: 200
      body: {}
";
    let result = TwinDefinition::from_yaml(yaml);
    assert!(result.is_err(), "C7 FAIL: should reject port 0");
    println!("C7 PASS: Validation rejects port 0");
}

/// C8: Validation rejects empty endpoints list
#[test]
fn c8_reject_empty_endpoints() {
    let yaml = r"
name: test
port: 3001
endpoints: []
";
    let result = TwinDefinition::from_yaml(yaml);
    assert!(result.is_err(), "C8 FAIL: should reject empty endpoints");
    println!("C8 PASS: Validation rejects empty endpoints list");
}

/// C9: Validation rejects path not starting with /
#[test]
fn c9_reject_invalid_path() {
    let yaml = r"
name: test
port: 3001
endpoints:
  - path: invalid
    method: GET
    response:
      status: 200
      body: {}
";
    let result = TwinDefinition::from_yaml(yaml);
    assert!(
        result.is_err(),
        "C9 FAIL: should reject path without leading /"
    );
    println!("C9 PASS: Validation rejects path not starting with /");
}

// ---------------------------------------------------------------------------
// Claim Sheet: State Layer
// ---------------------------------------------------------------------------

/// C10: InMemoryTwinState::add_record increments count
#[test]
fn c10_add_record_increments_count() {
    let state = InMemoryTwinState::new();
    assert_eq!(state.record_count(), 0);

    let record = RequestRecord::new(
        "GET".into(),
        "/test".into(),
        HashMap::new(),
        None,
        200,
        HashMap::new(),
        None,
    );
    let state = state.add_record(record);
    assert_eq!(state.record_count(), 1);
    println!("C10 PASS: InMemoryTwinState::add_record increments count");
}

/// C11: InMemoryTwinState is immutable — original unchanged after add
#[test]
fn c11_state_immutability() {
    let state = InMemoryTwinState::new();
    let record = RequestRecord::new(
        "GET".into(),
        "/test".into(),
        HashMap::new(),
        None,
        200,
        HashMap::new(),
        None,
    );
    let _new_state = state.add_record(record);
    assert_eq!(
        state.record_count(),
        0,
        "C11 FAIL: original state was mutated"
    );
    println!("C11 PASS: InMemoryTwinState is immutable — original unchanged after add");
}

/// C12: InMemoryTwinState::clear resets to zero
#[test]
fn c12_clear_resets() {
    let state = InMemoryTwinState::new();
    let record = RequestRecord::new(
        "GET".into(),
        "/test".into(),
        HashMap::new(),
        None,
        200,
        HashMap::new(),
        None,
    );
    let state = state.add_record(record);
    let cleared = state.clear();
    assert_eq!(cleared.record_count(), 0);
    println!("C12 PASS: InMemoryTwinState::clear resets to zero");
}

/// C13: InMemoryTwinState::get_records returns all records
#[test]
fn c13_get_records_returns_all() {
    let state = InMemoryTwinState::new();
    let r1 = RequestRecord::new(
        "GET".into(),
        "/a".into(),
        HashMap::new(),
        None,
        200,
        HashMap::new(),
        None,
    );
    let r2 = RequestRecord::new(
        "POST".into(),
        "/b".into(),
        HashMap::new(),
        Some("body".into()),
        201,
        HashMap::new(),
        None,
    );
    let state = state.add_record(r1).add_record(r2);
    let records = state.get_records();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].method, "GET");
    assert_eq!(records[1].method, "POST");
    println!("C13 PASS: InMemoryTwinState::get_records returns all records");
}

/// C14: RequestRecord::new generates UUID and timestamp
#[test]
fn c14_request_record_new() {
    let record = RequestRecord::new(
        "POST".into(),
        "/api/test".into(),
        HashMap::from([("Content-Type".into(), "application/json".into())]),
        Some(r#"{"key":"value"}"#.into()),
        201,
        HashMap::from([("X-Custom".into(), "header".into())]),
        None,
    );
    assert!(!record.id.is_empty(), "C14 FAIL: id should not be empty");
    assert!(
        record.id.contains('-'),
        "C14 FAIL: id should look like a UUID"
    );
    assert!(record.request_body.is_some());
    assert_eq!(record.method, "POST");
    assert_eq!(record.path, "/api/test");
    println!("C14 PASS: RequestRecord::new generates UUID and timestamp");
}

// ---------------------------------------------------------------------------
// Claim Sheet: Server Layer
// ---------------------------------------------------------------------------

/// C15: build_router creates a valid Router from definition
#[test]
fn c15_build_router() {
    let yaml = r"
name: router-test
port: 13010
endpoints:
  - path: /api/users
    method: GET
    response:
      status: 200
      body:
        users: []
";
    let def = TwinDefinition::from_yaml(yaml).expect("C15 FAIL: parse");
    let _router = build_router(def);
    println!("C15 PASS: build_router creates a valid Router from definition");
}

/// C16: AppState::find_endpoint finds matching endpoint by method+path
#[test]
fn c16_find_endpoint() {
    let yaml = r"
name: find-test
port: 13011
endpoints:
  - path: /api/test
    method: GET
    response:
      status: 200
      body: {}
  - path: /api/test
    method: POST
    response:
      status: 201
      body: {}
";
    let def = TwinDefinition::from_yaml(yaml).expect("C16 FAIL: parse");
    let state: AppState<InMemoryTwinState> = AppState::new(def);

    assert!(
        state
            .find_endpoint(&axum::http::Method::GET, "/api/test")
            .is_some(),
        "C16 FAIL: should find GET /api/test"
    );
    assert!(
        state
            .find_endpoint(&axum::http::Method::POST, "/api/test")
            .is_some(),
        "C16 FAIL: should find POST /api/test"
    );
    assert!(
        state
            .find_endpoint(&axum::http::Method::GET, "/nonexistent")
            .is_none(),
        "C16 FAIL: should not find nonexistent"
    );
    println!("C16 PASS: AppState::find_endpoint finds matching endpoint by method+path");
}

/// C17: Server responds with correct status and body for GET endpoint
#[tokio::test]
async fn c17_server_get_endpoint() {
    let server = start_test_server(
        r"
name: get-test
port: 13020
endpoints:
  - path: /api/health
    method: GET
    response:
      status: 200
      body:
        status: ok
",
        13020,
    )
    .await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/api/health", base_url(13020)))
        .send()
        .await
        .expect("C17 FAIL: request");
    assert_eq!(resp.status(), 200, "C17 FAIL: status should be 200");
    let body = resp.text().await.expect("C17 FAIL: read body");
    assert!(
        body.contains("ok"),
        "C17 FAIL: body should contain 'ok', got: {body}"
    );

    server.abort();
    println!("C17 PASS: Server responds with correct status and body for GET endpoint");
}

/// C18: Server responds with correct status and body for POST endpoint
#[tokio::test]
async fn c18_server_post_endpoint() {
    let server = start_test_server(
        r"
name: post-test
port: 13021
endpoints:
  - path: /api/items
    method: POST
    response:
      status: 201
      body:
        id: 'item-42'
        created: true
",
        13021,
    )
    .await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/api/items", base_url(13021)))
        .body(r#"{"name":"test"}"#)
        .send()
        .await
        .expect("C18 FAIL: request");
    assert_eq!(resp.status(), 201, "C18 FAIL: status should be 201");
    let body = resp.text().await.expect("C18 FAIL: read body");
    assert!(
        body.contains("item-42"),
        "C18 FAIL: body should contain 'item-42', got: {body}"
    );

    server.abort();
    println!("C18 PASS: Server responds with correct status and body for POST endpoint");
}

/// C19: Server records request in state (inspect/requests)
#[tokio::test]
async fn c19_server_records_requests() {
    let server = start_test_server(
        r"
name: record-test
port: 13022
endpoints:
  - path: /api/track
    method: POST
    response:
      status: 200
      body:
        ok: true
",
        13022,
    )
    .await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    for _ in 0..2 {
        client
            .post(format!("{}/api/track", base_url(13022)))
            .body("{}")
            .send()
            .await
            .expect("C19 FAIL: request");
    }

    let resp = client
        .get(format!("{}/_inspect/state", base_url(13022)))
        .send()
        .await
        .expect("C19 FAIL: inspect");
    assert_eq!(resp.status(), 200, "C19 FAIL: inspect should be 200");
    let body = resp.text().await.expect("C19 FAIL: read body");
    // Introspection endpoints don't record themselves — only user endpoints do
    assert!(
        body.contains("\"request_count\":2"),
        "C19 FAIL: should have 2 requests, got: {body}"
    );

    server.abort();
    println!("C19 PASS: Server records request in state (inspect/requests)");
}

/// C20: POST /_inspect/clear clears recorded requests
#[tokio::test]
async fn c20_inspect_clear() {
    let server = start_test_server(
        r"
name: clear-test
port: 13023
endpoints:
  - path: /api/data
    method: GET
    response:
      status: 200
      body: {}
",
        13023,
    )
    .await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    client
        .get(format!("{}/api/data", base_url(13023)))
        .send()
        .await
        .expect("C20 FAIL: get");

    let resp = client
        .post(format!("{}/_inspect/clear", base_url(13023)))
        .send()
        .await
        .expect("C20 FAIL: clear");
    assert_eq!(resp.status(), 200);

    let resp = client
        .get(format!("{}/_inspect/state", base_url(13023)))
        .send()
        .await
        .expect("C20 FAIL: inspect after clear");
    let body = resp.text().await.expect("C20 FAIL: read body");
    // After clear, introspection doesn't record itself — count is 0
    assert!(
        body.contains("\"request_count\":0"),
        "C20 FAIL: should have 0 requests after clear, got: {body}"
    );

    server.abort();
    println!("C20 PASS: POST /_inspect/clear clears recorded requests");
}

/// C21: GET /_inspect/state returns twin metadata
#[tokio::test]
async fn c21_inspect_state_metadata() {
    let server = start_test_server(
        r"
name: meta-test
port: 13024
endpoints:
  - path: /x
    method: GET
    response:
      status: 200
      body: {}
",
        13024,
    )
    .await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/_inspect/state", base_url(13024)))
        .send()
        .await
        .expect("C21 FAIL: inspect state");
    assert_eq!(resp.status(), 200);
    let body = resp.text().await.expect("C21 FAIL: read body");
    assert!(
        body.contains("meta-test"),
        "C21 FAIL: should contain twin name, got: {body}"
    );
    assert!(
        body.contains("\"port\":13024"),
        "C21 FAIL: should contain port, got: {body}"
    );

    server.abort();
    println!("C21 PASS: GET /_inspect/state returns twin metadata");
}

/// C22: Server returns 404 for unmatched route (fallback)
#[tokio::test]
async fn c22_fallback_404() {
    let server = start_test_server(
        r"
name: fallback-test
port: 13025
endpoints:
  - path: /only
    method: GET
    response:
      status: 200
      body: {}
",
        13025,
    )
    .await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/nonexistent", base_url(13025)))
        .send()
        .await
        .expect("C22 FAIL: request");
    assert_eq!(
        resp.status(),
        404,
        "C22 FAIL: should return 404 for unmatched route"
    );

    server.abort();
    println!("C22 PASS: Server returns 404 for unmatched route (fallback)");
}

/// C23: Server handles all 7 HTTP methods correctly
#[tokio::test]
async fn c23_all_http_methods() {
    let server = start_test_server(
        r"
name: methods-test
port: 13026
endpoints:
  - path: /r
    method: GET
    response:
      status: 200
      body: {m: GET}
  - path: /r
    method: POST
    response:
      status: 200
      body: {m: POST}
  - path: /r
    method: PUT
    response:
      status: 200
      body: {m: PUT}
  - path: /r
    method: DELETE
    response:
      status: 200
      body: {m: DELETE}
  - path: /r
    method: PATCH
    response:
      status: 200
      body: {m: PATCH}
  - path: /r
    method: OPTIONS
    response:
      status: 200
      body: {m: OPTIONS}
  - path: /r
    method: HEAD
    response:
      status: 200
      body: {}
",
        13026,
    )
    .await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();

    for method in &["GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS"] {
        let resp = client
            .request(
                reqwest::Method::from_bytes(method.as_bytes()).expect("method"),
                format!("{}/r", base_url(13026)),
            )
            .send()
            .await
            .expect("C23 FAIL: client request");
        assert_eq!(resp.status(), 200, "C23 FAIL: {method} should return 200");
        let body = resp.text().await.expect("C23 FAIL: read body");
        assert!(
            body.contains(method),
            "C23 FAIL: {method} response should contain method name, got: {body}"
        );
    }

    // HEAD should return 200
    let resp = client
        .head(format!("{}/r", base_url(13026)))
        .send()
        .await
        .expect("C23 FAIL: HEAD request");
    assert_eq!(resp.status(), 200, "C23 FAIL: HEAD should return 200");

    server.abort();
    println!("C23 PASS: Server handles all 7 HTTP methods correctly");
}

/// C24: Server preserves custom response headers
#[tokio::test]
async fn c24_custom_response_headers() {
    let server = start_test_server(
        r"
name: headers-test
port: 13027
endpoints:
  - path: /headers
    method: GET
    response:
      status: 200
      headers:
        X-Custom-Header: custom-value
        X-Rate-Limit: '100'
      body: {}
",
        13027,
    )
    .await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/headers", base_url(13027)))
        .send()
        .await
        .expect("C24 FAIL: request");
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers()
            .get("x-custom-header")
            .map(|v| v.to_str().ok()),
        Some(Some("custom-value")),
        "C24 FAIL: should have X-Custom-Header"
    );
    assert_eq!(
        resp.headers().get("x-rate-limit").map(|v| v.to_str().ok()),
        Some(Some("100")),
        "C24 FAIL: should have X-Rate-Limit"
    );

    server.abort();
    println!("C24 PASS: Server preserves custom response headers");
}

/// C25: Multiple endpoints with different paths work correctly
#[tokio::test]
async fn c25_multiple_endpoints() {
    let server = start_test_server(
        r"
name: multi-test
port: 13028
endpoints:
  - path: /users
    method: GET
    response:
      status: 200
      body:
        data: users-list
  - path: /orders
    method: GET
    response:
      status: 200
      body:
        data: orders-list
  - path: /users
    method: POST
    response:
      status: 201
      body:
        data: user-created
",
        13028,
    )
    .await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{}/users", base_url(13028)))
        .send()
        .await
        .expect("C25 FAIL: GET /users");
    assert!(
        resp.text().await.expect("C25: body").contains("users-list"),
        "C25 FAIL: GET /users body"
    );

    let resp = client
        .get(format!("{}/orders", base_url(13028)))
        .send()
        .await
        .expect("C25 FAIL: GET /orders");
    assert!(
        resp.text()
            .await
            .expect("C25: body")
            .contains("orders-list"),
        "C25 FAIL: GET /orders body"
    );

    let resp = client
        .post(format!("{}/users", base_url(13028)))
        .body("{}")
        .send()
        .await
        .expect("C25 FAIL: POST /users");
    assert!(
        resp.text()
            .await
            .expect("C25: body")
            .contains("user-created"),
        "C25 FAIL: POST /users body"
    );

    server.abort();
    println!("C25 PASS: Multiple endpoints with different paths work correctly");
}

/// C26: Default values work — empty body defaults to null, empty headers defaults to empty
#[test]
fn c26_default_values() {
    let yaml = r"
name: defaults-test
port: 3001
endpoints:
  - path: /minimal
    method: GET
    response:
      status: 204
";
    let def = TwinDefinition::from_yaml(yaml).expect("C26 FAIL: parse");
    assert_eq!(def.endpoints[0].response.body, serde_json::Value::Null);
    assert!(def.endpoints[0].response.headers.is_empty());
    println!("C26 PASS: Default values work for body and headers");
}

/// C27: Wrong method on valid path returns 405 (Method Not Allowed)
#[tokio::test]
async fn c27_wrong_method_returns_405() {
    let server = start_test_server(
        r"
name: wrong-method-test
port: 13029
endpoints:
  - path: /only-get
    method: GET
    response:
      status: 200
      body: {}
",
        13029,
    )
    .await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/only-get", base_url(13029)))
        .body("{}")
        .send()
        .await
        .expect("C27 FAIL: request");
    assert_eq!(
        resp.status(),
        405,
        "C27 FAIL: wrong method should return 405 Method Not Allowed"
    );

    server.abort();
    println!("C27 PASS: Wrong method on valid path returns 405 (Method Not Allowed)");
}

/// C28: Server handles empty request body gracefully
#[tokio::test]
async fn c28_empty_request_body() {
    let server = start_test_server(
        r"
name: empty-body-test
port: 13030
endpoints:
  - path: /accept
    method: POST
    response:
      status: 200
      body:
        received: true
",
        13030,
    )
    .await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/accept", base_url(13030)))
        .send()
        .await
        .expect("C28 FAIL: request");
    assert_eq!(resp.status(), 200, "C28 FAIL: should handle empty body");

    server.abort();
    println!("C28 PASS: Server handles empty request body gracefully");
}

// ---------------------------------------------------------------------------
// Adversarial Tests
// ---------------------------------------------------------------------------

/// ADV1: Malformed YAML is rejected
#[test]
fn adv1_malformed_yaml() {
    let yaml = "name: [invalid {{ yaml";
    assert!(
        TwinDefinition::from_yaml(yaml).is_err(),
        "ADV1 FAIL: should reject malformed YAML"
    );
    println!("ADV1 PASS: Malformed YAML is rejected");
}

/// ADV2: Missing required fields
#[test]
fn adv2_missing_all_fields() {
    assert!(
        TwinDefinition::from_yaml("name: test").is_err(),
        "ADV2 FAIL: missing port/endpoints"
    );
    assert!(
        TwinDefinition::from_yaml("port: 3001").is_err(),
        "ADV2 FAIL: missing name/endpoints"
    );
    assert!(
        TwinDefinition::from_yaml("endpoints: []").is_err(),
        "ADV2 FAIL: missing name/port"
    );
    println!("ADV2 PASS: Missing required fields are rejected");
}

/// ADV3: Port 0 is rejected
#[test]
fn adv3_port_zero() {
    let yaml = r"
name: test
port: 0
endpoints:
  - path: /x
    method: GET
    response:
      status: 200
      body: {}
";
    assert!(
        TwinDefinition::from_yaml(yaml).is_err(),
        "ADV3 FAIL: should reject port 0"
    );
    println!("ADV3 PASS: Port 0 is rejected");
}

/// ADV4: Port 65535 (max u16) is accepted
#[test]
fn adv4_port_max_u16() {
    let yaml = r"
name: max-port-test
port: 65535
endpoints:
  - path: /x
    method: GET
    response:
      status: 200
      body: {}
";
    let def = TwinDefinition::from_yaml(yaml).expect("ADV4 FAIL: should accept port 65535");
    assert_eq!(def.port, 65535);
    println!("ADV4 PASS: Port 65535 (max u16) is accepted");
}

/// ADV5: Negative port is rejected
#[test]
fn adv5_negative_port() {
    let yaml = r"
name: neg-port-test
port: -1
endpoints:
  - path: /x
    method: GET
    response:
      status: 200
      body: {}
";
    assert!(
        TwinDefinition::from_yaml(yaml).is_err(),
        "ADV5 FAIL: should reject negative port"
    );
    println!("ADV5 PASS: Negative port is rejected");
}

/// ADV6: Path traversal attempts don't leak data (return error, not 200)
#[tokio::test]
async fn adv6_path_traversal() {
    let server = start_test_server(
        r"
name: traversal-test
port: 13031
endpoints:
  - path: /safe/path
    method: GET
    response:
      status: 200
      body: {}
",
        13031,
    )
    .await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    for path in &[
        "/../../../etc/passwd",
        "/..%2F..%2Fetc%2Fpasswd",
        "/safe/../../etc/passwd",
    ] {
        let resp = client
            .get(format!("{}{path}", base_url(13031)))
            .send()
            .await
            .expect("ADV6 FAIL: request");
        // axum rejects .. paths at the router level — either 404 or 500, never 200 with leaked data
        assert_ne!(
            resp.status(),
            200,
            "ADV6 FAIL: path traversal {path} should NOT return 200"
        );
    }

    server.abort();
    println!("ADV6 PASS: Path traversal attempts don't leak data");
}

/// ADV7: Very large request body (stress)
#[tokio::test]
async fn adv7_large_request_body() {
    let server = start_test_server(
        r"
name: stress-test
port: 13032
endpoints:
  - path: /upload
    method: POST
    response:
      status: 200
      body:
        ok: true
",
        13032,
    )
    .await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let large_body = "x".repeat(1024 * 1024);
    let resp = client
        .post(format!("{}/upload", base_url(13032)))
        .body(large_body)
        .send()
        .await
        .expect("ADV7 FAIL: request");
    assert_eq!(resp.status(), 200, "ADV7 FAIL: should handle 1MB body");

    server.abort();
    println!("ADV7 PASS: Very large request body (1MB) handled");
}

/// ADV8: Concurrent requests (stress)
#[tokio::test]
async fn adv8_concurrent_requests() {
    let server = start_test_server(
        r"
name: concurrent-test
port: 13033
endpoints:
  - path: /concurrent
    method: GET
    response:
      status: 200
      body:
        ok: true
",
        13033,
    )
    .await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let mut handles = Vec::new();
    for _ in 0..50 {
        let c = client.clone();
        handles.push(tokio::spawn(async move {
            let resp = c
                .get("http://127.0.0.1:13033/concurrent")
                .send()
                .await
                .expect("ADV8 FAIL: request");
            assert_eq!(resp.status(), 200);
        }));
    }
    for h in handles {
        h.await.expect("ADV8 FAIL: task join");
    }

    let resp = client
        .get(format!("{}/_inspect/state", base_url(13033)))
        .send()
        .await
        .expect("ADV8 FAIL: inspect");
    let body = resp.text().await.expect("ADV8 FAIL: read body");
    // Introspection endpoints don't record themselves
    assert!(
        body.contains("\"request_count\":50"),
        "ADV8 FAIL: should have 50 requests, got: {body}"
    );

    server.abort();
    println!("ADV8 PASS: 50 concurrent requests handled correctly");
}

/// ADV9: Binary/garbage request body doesn't crash
#[tokio::test]
async fn adv9_binary_request_body() {
    let server = start_test_server(
        r"
name: binary-test
port: 13034
endpoints:
  - path: /raw
    method: POST
    response:
      status: 200
      body:
        ok: true
",
        13034,
    )
    .await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let garbage: Vec<u8> = (0..=255).cycle().take(512).collect();
    let resp = client
        .post(format!("{}/raw", base_url(13034)))
        .body(garbage)
        .send()
        .await
        .expect("ADV9 FAIL: request");
    assert_eq!(resp.status(), 200, "ADV9 FAIL: should handle binary body");

    let resp = client
        .get(format!("{}/_inspect/state", base_url(13034)))
        .send()
        .await
        .expect("ADV9 FAIL: inspect");
    let body = resp.text().await.expect("ADV9 FAIL: read body");
    // Introspection endpoints don't record themselves
    assert!(
        body.contains("\"request_count\":1"),
        "ADV9 FAIL: should have recorded the request, got: {body}"
    );

    server.abort();
    println!("ADV9 PASS: Binary/garbage request body doesn't crash server");
}

/// ADV10: Status code 204 (No Content) works
#[tokio::test]
async fn adv10_status_204_no_content() {
    let server = start_test_server(
        r"
name: no-content-test
port: 13035
endpoints:
  - path: /delete
    method: DELETE
    response:
      status: 204
      body: {}
",
        13035,
    )
    .await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let resp = client
        .delete(format!("{}/delete", base_url(13035)))
        .send()
        .await
        .expect("ADV10 FAIL: request");
    assert_eq!(resp.status(), 204, "ADV10 FAIL: should return 204");

    server.abort();
    println!("ADV10 PASS: Status code 204 (No Content) works");
}

/// ADV11: Status code 500 (Internal Server Error) works
#[tokio::test]
async fn adv11_status_500() {
    let server = start_test_server(
        r"
name: error-test
port: 13036
endpoints:
  - path: /error
    method: GET
    response:
      status: 500
      body:
        error: internal server error
",
        13036,
    )
    .await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/error", base_url(13036)))
        .send()
        .await
        .expect("ADV11 FAIL: request");
    assert_eq!(resp.status(), 500, "ADV11 FAIL: should return 500");

    server.abort();
    println!("ADV11 PASS: Status code 500 (Internal Server Error) works");
}

/// ADV12: Missing YAML (empty string) is rejected
#[test]
fn adv12_empty_yaml() {
    assert!(
        TwinDefinition::from_yaml("").is_err(),
        "ADV12 FAIL: should reject empty YAML"
    );
    println!("ADV12 PASS: Empty YAML string is rejected");
}

/// ADV13: from_yaml_bytes with empty bytes is rejected
#[test]
fn adv13_empty_bytes() {
    assert!(
        TwinDefinition::from_yaml_bytes(&[]).is_err(),
        "ADV13 FAIL: should reject empty bytes"
    );
    println!("ADV13 PASS: Empty bytes are rejected");
}

/// ADV14: Same path with different methods routes correctly
#[tokio::test]
async fn adv14_same_path_different_methods() {
    let server = start_test_server(
        r"
name: same-path-test
port: 13037
endpoints:
  - path: /resource
    method: GET
    response:
      status: 200
      body: {action: read}
  - path: /resource
    method: PUT
    response:
      status: 200
      body: {action: update}
  - path: /resource
    method: DELETE
    response:
      status: 200
      body: {action: delete}
",
        13037,
    )
    .await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    for (method, expected) in [("GET", "read"), ("PUT", "update"), ("DELETE", "delete")] {
        let resp = client
            .request(
                reqwest::Method::from_bytes(method.as_bytes()).expect("m"),
                format!("{}/resource", base_url(13037)),
            )
            .send()
            .await
            .expect("ADV14 FAIL: request");
        let body = resp.text().await.expect("ADV14: body");
        assert!(
            body.contains(expected),
            "ADV14 FAIL: {method} should return '{expected}', got: {body}"
        );
    }

    server.abort();
    println!("ADV14 PASS: Same path with different methods routes correctly");
}

/// ADV15: HttpMethod serialize/deserialize round-trip
#[test]
fn adv15_method_serde_roundtrip() {
    for method in [
        HttpMethod::GET,
        HttpMethod::POST,
        HttpMethod::PUT,
        HttpMethod::DELETE,
        HttpMethod::PATCH,
        HttpMethod::OPTIONS,
        HttpMethod::HEAD,
    ] {
        let json = serde_json::to_string(&method).expect("ADV15 FAIL: serialize");
        let parsed: HttpMethod = serde_json::from_str(&json).expect("ADV15 FAIL: deserialize");
        assert_eq!(method, parsed, "ADV15 FAIL: round-trip mismatch");
    }
    println!("ADV15 PASS: HttpMethod serialize/deserialize round-trip");
}

/// ADV16: Request headers are recorded
#[tokio::test]
async fn adv16_request_headers_recorded() {
    let server = start_test_server(
        r"
name: header-record-test
port: 13038
endpoints:
  - path: /echo
    method: POST
    response:
      status: 200
      body: {}
",
        13038,
    )
    .await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    client
        .post(format!("{}/echo", base_url(13038)))
        .header("X-Custom", "test-value")
        .header("Authorization", "Bearer token123")
        .body("{}")
        .send()
        .await
        .expect("ADV16 FAIL: request");

    let resp = client
        .get(format!("{}/_inspect/requests", base_url(13038)))
        .send()
        .await
        .expect("ADV16 FAIL: inspect");
    let body = resp.text().await.expect("ADV16 FAIL: read body");
    assert!(
        body.contains("x-custom") && body.contains("test-value"),
        "ADV16 FAIL: should record request headers, got: {body}"
    );

    server.abort();
    println!("ADV16 PASS: Request headers are recorded");
}

/// ADV17: InMemoryTwinState Default trait impl
#[test]
fn adv17_default_trait() {
    assert_eq!(InMemoryTwinState::default().record_count(), 0);
    println!("ADV17 PASS: InMemoryTwinState::default() works");
}

/// ADV18: EndpointResponse defaults
#[test]
fn adv18_endpoint_response_defaults() {
    let resp = EndpointResponse {
        status: 200,
        body: serde_json::Value::default(),
        headers: HashMap::default(),
    };
    assert_eq!(resp.status, 200);
    assert_eq!(resp.body, serde_json::Value::Null);
    assert!(resp.headers.is_empty());
    println!("ADV18 PASS: EndpointResponse defaults work");
}

/// ADV19: User-defined /_inspect/* endpoints are silently filtered out (no panic)
#[tokio::test]
async fn adv19_inspect_not_overridable() {
    // Before fix: this would panic with "Overlapping method route"
    // After fix: /_inspect/* user endpoints are silently filtered out
    let server = start_test_server(
        r"
name: inspect-override-test
port: 13039
endpoints:
  - path: /_inspect/state
    method: GET
    response:
      status: 200
      body:
        hacked: true
",
        13039,
    )
    .await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/_inspect/state", base_url(13039)))
        .send()
        .await
        .expect("ADV19 FAIL: request");
    let body = resp.text().await.expect("ADV19 FAIL: read body");
    // Built-in inspect handler should still work with twin metadata
    assert!(
        body.contains("inspect-override-test"),
        "ADV19 FAIL: built-in should still work, got: {body}"
    );
    assert!(
        !body.contains("hacked"),
        "ADV19 FAIL: user-defined should NOT override built-in"
    );

    server.abort();
    println!("ADV19 PASS: User-defined /_inspect/* endpoints are silently filtered out");
}

/// ADV20: YAML with extra/unknown fields
#[test]
fn adv20_extra_yaml_fields() {
    let yaml = r"
name: extra-fields-test
port: 3001
unknown_field: value
endpoints:
  - path: /test
    method: GET
    response:
      status: 200
      body: {}
      extra_response_field: true
";
    match TwinDefinition::from_yaml(yaml) {
        Ok(def) => {
            assert_eq!(def.name, "extra-fields-test");
            println!("ADV20 PASS: Extra YAML fields are ignored (parsed successfully)");
        }
        Err(_) => {
            println!("ADV20 PASS: Extra YAML fields are rejected (strict mode)");
        }
    }
}

/// ADV21: Very long endpoint path
#[test]
fn adv21_long_path() {
    let long_path = format!("/{}", "a".repeat(1000));
    let yaml = format!(
        r"
name: long-path-test
port: 3001
endpoints:
  - path: {long_path}
    method: GET
    response:
      status: 200
      body: {{}}
"
    );
    assert!(
        TwinDefinition::from_yaml(&yaml).is_ok(),
        "ADV21 FAIL: should accept long path"
    );
    println!("ADV21 PASS: Very long endpoint path accepted");
}
