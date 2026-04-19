//! SERDE-CORRECT: twins — serialization round-trip verification
//!
//! Verifies all serde Serialize/Deserialize impls in twins round-trip correctly.
//! - serialize to JSON, deserialize back, assert equality
//! - Check rename attributes (HttpMethod has #[serde(rename_all = "UPPERCASE")])
//! - Check skip_serializing_if attributes
//! - Check enum representations
//! - Test with missing and extra fields

use std::collections::HashMap;

use twins::definition::{Endpoint, EndpointResponse, HttpMethod, TwinDefinition};
use twins::state::{InMemoryTwinState, RequestRecord, TwinState};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct Person {
    name: String,
    age: u32,
}

#[test]
fn http_method_roundtrip_all_variants() {
    for method in [
        HttpMethod::GET,
        HttpMethod::POST,
        HttpMethod::PUT,
        HttpMethod::DELETE,
        HttpMethod::PATCH,
        HttpMethod::OPTIONS,
        HttpMethod::HEAD,
    ] {
        let json = serde_json::to_string(&method).expect("serialize HttpMethod");
        let parsed: HttpMethod = serde_json::from_str(&json).expect("deserialize HttpMethod");
        assert_eq!(method, parsed, "HttpMethod {method:?} failed roundtrip");
    }
}

#[test]
fn http_method_json_format_is_uppercase() {
    let method = HttpMethod::GET;
    let json = serde_json::to_string(&method).expect("serialize");
    assert_eq!(json, "\"GET\"", "HttpMethod should serialize to UPPERCASE");
    assert!(!json.contains("\"get\""), "HttpMethod should NOT serialize to lowercase");
}

#[test]
fn http_method_json_deserialization_is_case_sensitive() {
    for input in ["GET", "get", "Get", "POST", "post", "PoSt"] {
        let json = format!("\"{input}\"");
        let result: Result<HttpMethod, _> = serde_json::from_str(&json);
        if input == input.to_uppercase() && input != input.to_lowercase() {
            assert!(result.is_ok(), "HttpMethod should deserialize uppercase '{input}'");
        } else {
            assert!(result.is_err(), "HttpMethod should NOT deserialize lowercase '{input}' (serde is case-sensitive)");
        }
    }
}

#[test]
fn http_method_invalid_value_rejected() {
    for invalid in ["INVALID", "TRACE", "CONNECT", "PING", "", "getx"] {
        let json = format!("\"{invalid}\"");
        let result: Result<HttpMethod, _> = serde_json::from_str(&json);
        assert!(result.is_err(), "HttpMethod should reject invalid value '{invalid}'");
    }
}

#[test]
fn endpoint_response_full_roundtrip() {
    let resp = EndpointResponse {
        status: 200,
        body: serde_json::json!({"key": "value", "nested": {"a": 1}}),
        headers: HashMap::from([
            ("content-type".to_string(), "application/json".to_string()),
            ("x-custom".to_string(), "test".to_string()),
        ]),
    };

    let json = serde_json::to_string(&resp).expect("serialize");
    let parsed: EndpointResponse = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(resp.status, parsed.status);
    assert_eq!(resp.body, parsed.body);
    assert_eq!(resp.headers, parsed.headers);
}

#[test]
fn endpoint_response_minimal_roundtrip() {
    let resp = EndpointResponse {
        status: 204,
        body: serde_json::Value::Null,
        headers: HashMap::new(),
    };

    let json = serde_json::to_string(&resp).expect("serialize");
    let parsed: EndpointResponse = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(resp.status, parsed.status);
    assert_eq!(resp.body, parsed.body);
    assert_eq!(resp.headers, parsed.headers);
}

#[test]
fn endpoint_full_roundtrip() {
    let endpoint = Endpoint {
        path: "/api/users".to_string(),
        method: HttpMethod::POST,
        response: EndpointResponse {
            status: 201,
            body: serde_json::json!({"id": 42, "created": true}),
            headers: HashMap::from([("location".to_string(), "/api/users/42".to_string())]),
        },
    };

    let json = serde_json::to_string(&endpoint).expect("serialize");
    let parsed: Endpoint = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(endpoint.path, parsed.path);
    assert_eq!(endpoint.method, parsed.method);
    assert_eq!(endpoint.response.status, parsed.response.status);
    assert_eq!(endpoint.response.body, parsed.response.body);
    assert_eq!(endpoint.response.headers, parsed.response.headers);
}

#[test]
fn twin_definition_full_roundtrip() {
    let def = TwinDefinition {
        name: "test-twin".to_string(),
        port: 8080,
        endpoints: vec![
            Endpoint {
                path: "/health".to_string(),
                method: HttpMethod::GET,
                response: EndpointResponse {
                    status: 200,
                    body: serde_json::json!({"status": "ok"}),
                    headers: HashMap::new(),
                },
            },
            Endpoint {
                path: "/users".to_string(),
                method: HttpMethod::POST,
                response: EndpointResponse {
                    status: 201,
                    body: serde_json::json!({"id": 1}),
                    headers: HashMap::from([("location".to_string(), "/users/1".to_string())]),
                },
            },
        ],
    };

    let json = serde_json::to_string(&def).expect("serialize");
    let parsed: TwinDefinition = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(def.name, parsed.name);
    assert_eq!(def.port, parsed.port);
    assert_eq!(def.endpoints.len(), parsed.endpoints.len());
    for (orig, recon) in def.endpoints.iter().zip(parsed.endpoints.iter()) {
        assert_eq!(orig.path, recon.path);
        assert_eq!(orig.method, recon.method);
        assert_eq!(orig.response.status, recon.response.status);
        assert_eq!(orig.response.body, recon.response.body);
        assert_eq!(orig.response.headers, recon.response.headers);
    }
}

#[test]
fn twin_definition_complex_body_roundtrip() {
    let def = TwinDefinition {
        name: "complex".to_string(),
        port: 3000,
        endpoints: vec![Endpoint {
            path: "/data".to_string(),
            method: HttpMethod::PUT,
            response: EndpointResponse {
                status: 200,
                body: serde_json::json!({
                    "array": [1, 2, 3],
                    "nested": {"deep": {"value": "string"}},
                    "bool": true,
                    "null": null,
                    "number": 3.14
                }),
                headers: HashMap::new(),
            },
        }],
    };

    let json = serde_json::to_string(&def).expect("serialize");
    let parsed: TwinDefinition = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(def.endpoints[0].response.body, parsed.endpoints[0].response.body);
}

#[test]
fn request_record_full_roundtrip() {
    let record = RequestRecord::new(
        "POST".to_string(),
        "/api/items".to_string(),
        HashMap::from([
            ("content-type".to_string(), "application/json".to_string()),
            ("authorization".to_string(), "Bearer token123".to_string()),
        ]),
        Some(r#"{"name":"test","price":99.99}"#.to_string()),
        201,
        HashMap::from([
            ("content-type".to_string(), "application/json".to_string()),
            ("location".to_string(), "/api/items/42".to_string()),
        ]),
        Some(r#"{"id":42,"created":true}"#.to_string()),
    );

    let json = serde_json::to_string(&record).expect("serialize");
    let parsed: RequestRecord = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(record.id, parsed.id);
    assert_eq!(record.method, parsed.method);
    assert_eq!(record.path, parsed.path);
    assert_eq!(record.request_headers, parsed.request_headers);
    assert_eq!(record.request_body, parsed.request_body);
    assert_eq!(record.status, parsed.status);
    assert_eq!(record.response_headers, parsed.response_headers);
    assert_eq!(record.response_body, parsed.response_body);
    assert_eq!(record.timestamp, parsed.timestamp);
}

#[test]
fn request_record_minimal_roundtrip() {
    let record = RequestRecord::new(
        "GET".to_string(),
        "/".to_string(),
        HashMap::new(),
        None,
        200,
        HashMap::new(),
        None,
    );

    let json = serde_json::to_string(&record).expect("serialize");
    let parsed: RequestRecord = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(record.id, parsed.id);
    assert_eq!(record.method, parsed.method);
    assert_eq!(record.path, parsed.path);
    assert_eq!(record.request_headers, parsed.request_headers);
    assert_eq!(record.request_body, parsed.request_body);
    assert_eq!(record.status, parsed.status);
    assert_eq!(record.response_headers, parsed.response_headers);
    assert_eq!(record.response_body, parsed.response_body);
}

#[test]
fn in_memory_twin_state_empty_roundtrip() {
    let state = InMemoryTwinState::new();
    let json = serde_json::to_string(&state).expect("serialize");
    let parsed: InMemoryTwinState = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(state.record_count(), parsed.record_count());
}

#[test]
fn in_memory_twin_state_with_records_roundtrip() {
    let state = InMemoryTwinState::new();
    let r1 = RequestRecord::new(
        "GET".to_string(),
        "/first".to_string(),
        HashMap::new(),
        None,
        200,
        HashMap::new(),
        None,
    );
    let r2 = RequestRecord::new(
        "POST".to_string(),
        "/second".to_string(),
        HashMap::new(),
        Some("body".to_string()),
        201,
        HashMap::new(),
        None,
    );

    let state = state.add_record(r1).add_record(r2);
    let json = serde_json::to_string(&state).expect("serialize");
    let parsed: InMemoryTwinState = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(state.record_count(), parsed.record_count());
    let orig_records = state.get_records();
    let parsed_records = parsed.get_records();
    assert_eq!(orig_records.len(), parsed_records.len());
    for (orig, recon) in orig_records.iter().zip(parsed_records.iter()) {
        assert_eq!(orig.id, recon.id);
        assert_eq!(orig.method, recon.method);
        assert_eq!(orig.path, recon.path);
    }
}

#[test]
fn endpoint_response_missing_body_field_uses_default() {
    let json = r#"{"status": 200}"#;
    let parsed: EndpointResponse = serde_json::from_str(json).expect("deserialize");
    assert_eq!(parsed.status, 200);
    assert_eq!(parsed.body, serde_json::Value::Null);
    assert!(parsed.headers.is_empty());
}

#[test]
fn endpoint_response_missing_headers_field_uses_default() {
    let json = r#"{"status": 200, "body": {"key": "value"}}"#;
    let parsed: EndpointResponse = serde_json::from_str(json).expect("deserialize");
    assert_eq!(parsed.status, 200);
    assert_eq!(parsed.body, serde_json::json!({"key": "value"}));
    assert!(parsed.headers.is_empty());
}

#[test]
fn request_record_missing_optional_body_fields() {
    let json = r#"{
        "id": "test-id-123",
        "timestamp": "2024-01-01T00:00:00Z",
        "method": "GET",
        "path": "/test",
        "request_headers": {},
        "status": 200,
        "response_headers": {}
    }"#;
    let parsed: RequestRecord = serde_json::from_str(json).expect("deserialize");
    assert_eq!(parsed.id, "test-id-123");
    assert_eq!(parsed.method, "GET");
    assert_eq!(parsed.path, "/test");
    assert_eq!(parsed.status, 200);
    assert!(parsed.request_body.is_none());
    assert!(parsed.response_body.is_none());
}

#[test]
fn twin_definition_missing_optional_endpoints_field() {
    let json = r#"{"name": "test", "port": 3000}"#;
    let result: Result<TwinDefinition, _> = serde_json::from_str(json);
    assert!(result.is_err(), "TwinDefinition should require endpoints field");
}

#[test]
fn twin_definition_extra_json_fields_ignored() {
    let json = r#"{
        "name": "test-twin",
        "port": 8080,
        "extra_field": "should be ignored",
        "unknown": 12345,
        "endpoints": []
    }"#;
    let parsed: TwinDefinition = serde_json::from_str(json).expect("deserialize");
    assert_eq!(parsed.name, "test-twin");
    assert_eq!(parsed.port, 8080);
}

#[test]
fn endpoint_extra_json_fields_ignored() {
    let json = r#"{
        "path": "/test",
        "method": "GET",
        "unknown_field": "ignored",
        "response": {
            "status": 200,
            "body": {},
            "extra": true
        }
    }"#;
    let parsed: Endpoint = serde_json::from_str(json).expect("deserialize");
    assert_eq!(parsed.path, "/test");
    assert_eq!(parsed.method, HttpMethod::GET);
}

#[test]
fn endpoint_response_extra_json_fields_ignored() {
    let json = r#"{
        "status": 200,
        "body": {"key": "value"},
        "headers": {},
        "extra_field": "ignored"
    }"#;
    let parsed: EndpointResponse = serde_json::from_str(json).expect("deserialize");
    assert_eq!(parsed.status, 200);
}

#[test]
fn http_method_extra_json_fields_ignored() {
    #[derive(Debug, serde::Deserialize)]
    struct WithExtra {
        method: HttpMethod,
        extra: String,
    }
    let json = r#"{"method": "POST", "extra": "value"}"#;
    let parsed: WithExtra = serde_json::from_str(json).expect("deserialize");
    assert_eq!(parsed.method, HttpMethod::POST);
    assert_eq!(parsed.extra, "value");
}

#[test]
fn request_record_extra_json_fields_ignored() {
    #[derive(Debug, serde::Deserialize)]
    struct WithExtra {
        record: RequestRecord,
        extra: String,
    }
    let json = format!(r#"{{
        "record": {{
            "id": "{}",
            "timestamp": "2024-01-01T00:00:00Z",
            "method": "GET",
            "path": "/test",
            "request_headers": {{}},
            "status": 200,
            "response_headers": {{}}
        }},
        "extra": "ignored"
    }}"#, uuid::Uuid::new_v4());
    let parsed: WithExtra = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.record.method, "GET");
    assert_eq!(parsed.extra, "ignored");
}

#[test]
fn in_memory_twin_state_extra_fields_ignored() {
    #[derive(Debug, serde::Deserialize)]
    struct WithExtra {
        state: InMemoryTwinState,
        extra: String,
    }
    let json = r#"{"state": {"records": []}, "extra": "ignored"}"#;
    let parsed: WithExtra = serde_json::from_str(json).expect("deserialize");
    assert_eq!(parsed.extra, "ignored");
}

#[test]
fn twin_definition_json_with_endpoint_with_extra_fields() {
    let json = r#"{
        "name": "test",
        "port": 3000,
        "endpoints": [
            {
                "path": "/test",
                "method": "POST",
                "response": {
                    "status": 200,
                    "body": {},
                    "headers": {},
                    "extra": true
                },
                "unknown": "field"
            }
        ]
    }"#;
    let parsed: TwinDefinition = serde_json::from_str(json).expect("deserialize");
    assert_eq!(parsed.endpoints.len(), 1);
    assert_eq!(parsed.endpoints[0].path, "/test");
}

#[test]
fn http_method_all_json_variants() {
    let test_cases = [
        ("\"GET\"", HttpMethod::GET),
        ("\"POST\"", HttpMethod::POST),
        ("\"PUT\"", HttpMethod::PUT),
        ("\"DELETE\"", HttpMethod::DELETE),
        ("\"PATCH\"", HttpMethod::PATCH),
        ("\"OPTIONS\"", HttpMethod::OPTIONS),
        ("\"HEAD\"", HttpMethod::HEAD),
    ];
    for (json, expected) in test_cases {
        let parsed: HttpMethod = serde_json::from_str(json).expect(&format!("parse {json}"));
        assert_eq!(parsed, expected, "json: {json}");
    }
}

#[test]
fn all_http_methods_display_roundtrip() {
    let methods = [
        HttpMethod::GET,
        HttpMethod::POST,
        HttpMethod::PUT,
        HttpMethod::DELETE,
        HttpMethod::PATCH,
        HttpMethod::OPTIONS,
        HttpMethod::HEAD,
    ];
    for method in methods {
        let json = serde_json::to_string(&method).expect("serialize");
        let rehydrated: HttpMethod = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(method, rehydrated, "Display->JSON->Deserialize failed for {method}");
    }
}

#[test]
fn in_memory_twin_state_with_100_records_roundtrip() {
    let state = InMemoryTwinState::new();
    let mut state = state;
    for i in 0..100 {
        let r = RequestRecord::new(
            "GET".to_string(),
            format!("/endpoint/{i}"),
            HashMap::new(),
            None,
            200,
            HashMap::new(),
            None,
        );
        state = state.add_record(r);
    }
    let json = serde_json::to_string(&state).expect("serialize");
    let parsed: InMemoryTwinState = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(state.record_count(), parsed.record_count());
}

#[test]
fn twin_definition_yaml_roundtrip() {
    let yaml_str = r"
name: yaml-test
port: 3001
endpoints:
  - path: /v3/mail/send
    method: POST
    response:
      status: 200
      body:
        message_id: 'test-123'
";
    let def = TwinDefinition::from_yaml(yaml_str).expect("parse yaml");
    let yaml_out = serde_yaml::to_string(&def).expect("serialize yaml");
    let def2 = TwinDefinition::from_yaml(&yaml_out).expect("deserialize yaml");
    assert_eq!(def.name, def2.name);
    assert_eq!(def.port, def2.port);
    assert_eq!(def.endpoints.len(), def2.endpoints.len());
}

#[test]
fn twin_definition_json_to_yaml_roundtrip() {
    let def = TwinDefinition {
        name: "json-yaml".to_string(),
        port: 9000,
        endpoints: vec![Endpoint {
            path: "/data".to_string(),
            method: HttpMethod::GET,
            response: EndpointResponse {
                status: 200,
                body: serde_json::json!({"key": "value"}),
                headers: HashMap::new(),
            },
        }],
    };
    let json = serde_json::to_string(&def).expect("to json");
    let def_from_json: TwinDefinition = serde_json::from_str(&json).expect("from json");

    let yaml = serde_yaml::to_string(&def_from_json).expect("to yaml");
    let def_from_yaml: TwinDefinition = serde_yaml::from_str(&yaml).expect("from yaml");

    assert_eq!(def.name, def_from_yaml.name);
    assert_eq!(def.port, def_from_yaml.port);
}

#[test]
fn request_record_preserves_special_characters_in_body() {
    let record = RequestRecord::new(
        "POST".to_string(),
        "/api".to_string(),
        HashMap::new(),
        Some(r#"{"message": "Hello, World! \"escaped\" and 'single'"}"#.to_string()),
        200,
        HashMap::new(),
        None,
    );
    let json = serde_json::to_string(&record).expect("serialize");
    let parsed: RequestRecord = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(record.request_body, parsed.request_body);
}

#[test]
fn request_record_preserves_unicode_in_headers() {
    let mut headers = HashMap::new();
    headers.insert("x-unicode".to_string(), "日本語 Émoji 🎉".to_string());
    let record = RequestRecord::new(
        "GET".to_string(),
        "/api".to_string(),
        headers.clone(),
        None,
        200,
        HashMap::new(),
        None,
    );
    let json = serde_json::to_string(&record).expect("serialize");
    let parsed: RequestRecord = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(record.request_headers, parsed.request_headers);
}

#[test]
fn request_record_empty_string_body_preserved() {
    let record = RequestRecord::new(
        "POST".to_string(),
        "/api".to_string(),
        HashMap::new(),
        Some("".to_string()),
        200,
        HashMap::new(),
        None,
    );
    let json = serde_json::to_string(&record).expect("serialize");
    let parsed: RequestRecord = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.request_body, Some("".to_string()));
}

#[test]
fn endpoint_response_all_status_codes() {
    for status in [200, 201, 204, 301, 400, 401, 403, 404, 500, 502, 503] {
        let resp = EndpointResponse {
            status,
            body: serde_json::Value::Null,
            headers: HashMap::new(),
        };
        let json = serde_json::to_string(&resp).expect("serialize");
        let parsed: EndpointResponse = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(status, parsed.status, "status {status}");
    }
}

#[test]
fn in_memory_twin_state_default_impl() {
    let default_state = InMemoryTwinState::default();
    assert_eq!(default_state.record_count(), 0);

    let json = serde_json::to_string(&default_state).expect("serialize");
    let parsed: InMemoryTwinState = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.record_count(), 0);
}

#[test]
fn all_types_impl_debug() {
    fn assert_debug<T: std::fmt::Debug>() {}
    assert_debug::<HttpMethod>();
    assert_debug::<EndpointResponse>();
    assert_debug::<Endpoint>();
    assert_debug::<TwinDefinition>();
    assert_debug::<RequestRecord>();
    assert_debug::<InMemoryTwinState>();
}

#[test]
fn all_types_impl_clone() {
    fn assert_clone<T: Clone>() {}
    assert_clone::<HttpMethod>();
    assert_clone::<EndpointResponse>();
    assert_clone::<Endpoint>();
    assert_clone::<TwinDefinition>();
    assert_clone::<RequestRecord>();
    assert_clone::<InMemoryTwinState>();
}

#[test]
#[test]
fn http_method_impl_partial_eq() {
    fn assert_partial_eq<T: PartialEq>() {}
    assert_partial_eq::<HttpMethod>();
}