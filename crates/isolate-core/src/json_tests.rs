use super::json::{
    schemas::{self, all_valid_schemas, is_valid_schema, uri},
    ErrorDetail, JsonError, JsonSuccess, SchemaEnvelope,
};

// Behavior: JsonSuccess wraps data with success flag
#[test]
fn given_data_when_create_json_success_then_success_is_true() {
    let response = JsonSuccess::new("test data");
    assert!(response.success);
    assert_eq!(response.data, "test data");
}

// Behavior: JsonSuccess serializes with flattened data
#[test]
fn given_json_success_when_serialize_then_data_is_flattened() {
    #[derive(serde::Serialize)]
    struct TestData {
        field: String,
    }

    let response = JsonSuccess::new(TestData {
        field: "value".to_string(),
    });

    let json = serde_json::to_value(&response).ok();
    assert!(json.is_some());

    if let Some(v) = json {
        assert_eq!(v.get("success"), Some(&serde_json::json!(true)));
        assert_eq!(v.get("field"), Some(&serde_json::json!("value")));
    }
}

// Behavior: JsonError has success false by default
#[test]
fn given_json_error_when_created_then_success_is_false() {
    let error = JsonError::default();
    assert!(!error.success);
}

// Behavior: ErrorDetail contains code, message, and exit_code
#[test]
fn given_error_detail_when_created_then_has_required_fields() {
    let detail = ErrorDetail {
        code: "TEST_ERROR".to_string(),
        message: "Test error message".to_string(),
        exit_code: 1,
        details: None,
        suggestion: None,
    };

    assert_eq!(detail.code, "TEST_ERROR");
    assert_eq!(detail.message, "Test error message");
    assert_eq!(detail.exit_code, 1);
}

// Behavior: Schema URI is constructed correctly
#[test]
fn given_schema_name_when_uri_then_returns_full_uri() {
    let uri_result = uri("test-schema");
    assert_eq!(uri_result, "isolate://test-schema/v1");
}

// Behavior: All schema constants are valid
#[test]
fn given_schema_constants_when_check_validity_then_all_are_valid() {
    assert!(is_valid_schema(schemas::INIT_RESPONSE));
    assert!(is_valid_schema(schemas::ADD_RESPONSE));
    assert!(is_valid_schema(schemas::LIST_RESPONSE));
    assert!(is_valid_schema(schemas::STATUS_RESPONSE));
    assert!(is_valid_schema(schemas::ERROR_RESPONSE));
}

// Behavior: Invalid schema names are rejected
#[test]
fn given_invalid_schema_name_when_check_validity_then_returns_false() {
    assert!(!is_valid_schema("invalid-schema"));
    assert!(!is_valid_schema(""));
    assert!(!is_valid_schema("random-name"));
}

// Behavior: All valid schemas list contains all constants
#[test]
fn given_all_valid_schemas_when_called_then_contains_all_response_types() {
    let all = all_valid_schemas();

    assert!(all.contains(&schemas::INIT_RESPONSE));
    assert!(all.contains(&schemas::ADD_RESPONSE));
    assert!(all.contains(&schemas::LIST_RESPONSE));
    assert!(all.contains(&schemas::REMOVE_RESPONSE));
    assert!(all.contains(&schemas::ERROR_RESPONSE));
    assert!(all.len() > 20); // Should have many schemas
}

// Behavior: SchemaEnvelope wraps data with metadata
#[test]
fn given_data_when_create_schema_envelope_then_contains_metadata_and_data() {
    let envelope = SchemaEnvelope::new("test-schema", "single", "test data");

    assert!(envelope.schema.contains("test-schema"));
    assert_eq!(envelope.schema_type, "single");
    assert_eq!(envelope.data, "test data");
}

// Behavior: SchemaEnvelope has schema type field
#[test]
fn given_schema_envelope_when_created_then_has_schema_type() {
    let envelope = SchemaEnvelope::new("test", "single", 42);

    assert_eq!(envelope.schema_type, "single");
    assert!(envelope.success);
    assert_eq!(envelope.data, 42);
}

// Behavior: JsonError::new creates error with defaults
#[test]
fn given_code_and_message_when_create_json_error_then_has_default_exit_code() {
    let error = JsonError::new("TEST_CODE", "Test message");

    assert!(!error.success);
    assert_eq!(error.error.code, "TEST_CODE");
    assert_eq!(error.error.message, "Test message");
    assert_eq!(error.error.exit_code, 4); // Default
}

// Behavior: JsonError with_details adds details
#[test]
fn given_json_error_when_with_details_then_details_are_set() {
    let error = JsonError::new("TEST", "message").with_details(serde_json::json!({"key": "value"}));

    assert!(error.error.details.is_some());
    if let Some(details) = error.error.details {
        assert_eq!(details.get("key"), Some(&serde_json::json!("value")));
    }
}

// Behavior: JsonError with_suggestion adds suggestion
#[test]
fn given_json_error_when_with_suggestion_then_suggestion_is_set() {
    let error = JsonError::new("TEST", "message").with_suggestion("Try this instead");

    assert_eq!(error.error.suggestion, Some("Try this instead".to_string()));
}

// Behavior: JsonError with_exit_code sets exit code
#[test]
fn given_json_error_when_with_exit_code_then_exit_code_is_set() {
    let error = JsonError::new("TEST", "message").with_exit_code(2);

    assert_eq!(error.error.exit_code, 2);
}

// Behavior: JsonError::to_json produces valid JSON string
#[test]
fn given_json_error_when_to_json_then_produces_valid_json_string() {
    let error = JsonError::new("TEST", "message");
    let json_str = error.to_json();

    assert!(json_str.is_ok());
    if let Ok(s) = json_str {
        assert!(s.contains("\"success\""));
        assert!(s.contains("\"error\""));
        assert!(s.contains("TEST"));
    }
}

// Behavior: Base URI constant is correct
#[test]
fn given_base_uri_when_accessed_then_is_isolate_protocol() {
    assert_eq!(schemas::BASE_URI, "isolate://");
}

// Behavior: Schema version is consistent
#[test]
fn given_schema_version_when_accessed_then_is_1_0() {
    assert_eq!(schemas::SCHEMA_VERSION, "1.0");
}

// Behavior: JsonError default has UNKNOWN code
#[test]
fn given_json_error_default_when_created_then_has_unknown_code() {
    let error = JsonError::default();

    assert_eq!(error.error.code, "UNKNOWN");
    assert_eq!(error.error.exit_code, 4);
    assert!(!error.error.message.is_empty());
}

// Behavior: SchemaEnvelope is generic over data type
#[test]
fn given_different_data_types_when_create_envelope_then_works_for_all() {
    let string_env = SchemaEnvelope::new("test", "single", "string data");
    let int_env = SchemaEnvelope::new("test", "single", 123);
    let bool_env = SchemaEnvelope::new("test", "single", true);

    assert_eq!(string_env.data, "string data");
    assert_eq!(int_env.data, 123);
    assert!(bool_env.data);
}

// Behavior: Query schema names are included in all_valid_schemas
#[test]
fn given_all_valid_schemas_when_called_then_includes_query_schemas() {
    let all = all_valid_schemas();

    assert!(all.contains(&schemas::QUERY_SESSION_EXISTS));
    assert!(all.contains(&schemas::QUERY_CAN_RUN));
    assert!(all.contains(&schemas::QUERY_SUGGEST_NAME));
    assert!(all.contains(&schemas::QUERY_LOCK_STATUS));
}

// Behavior: Diff schema names are included
#[test]
fn given_all_valid_schemas_when_called_then_includes_diff_schemas() {
    let all = all_valid_schemas();

    assert!(all.contains(&schemas::DIFF_RESPONSE));
    assert!(all.contains(&schemas::DIFF_STAT_RESPONSE));
}

// Behavior: JsonError can be serialized
#[test]
fn given_json_error_when_serialize_then_produces_valid_json() {
    let error = JsonError::new("ERR_001", "Test error");

    let json = serde_json::to_value(&error).ok();
    assert!(json.is_some());

    if let Some(v) = json {
        assert_eq!(v.get("success"), Some(&serde_json::json!(false)));
        assert!(v.get("error").is_some());
    }
}

// Behavior: URI format is consistent for all schemas
#[test]
fn given_various_schema_names_when_create_uri_then_format_is_consistent() {
    let uri1 = uri("init");
    let uri2 = uri("add");
    let uri3 = uri("status");

    assert!(uri1.starts_with("isolate://"));
    assert!(uri2.starts_with("isolate://"));
    assert!(uri3.starts_with("isolate://"));

    assert!(uri1.ends_with("/v1"));
    assert!(uri2.ends_with("/v1"));
    assert!(uri3.ends_with("/v1"));
}

// Behavior: JsonError builder pattern chains methods
#[test]
fn given_json_error_when_chain_methods_then_all_fields_set() {
    let error = JsonError::new("CHAIN_TEST", "Chain test")
        .with_exit_code(1)
        .with_suggestion("Do this")
        .with_details(serde_json::json!({"context": "test"}));

    assert_eq!(error.error.code, "CHAIN_TEST");
    assert_eq!(error.error.exit_code, 1);
    assert!(error.error.suggestion.is_some());
    assert!(error.error.details.is_some());
}

// Behavior: ErrorDetail serialization skips None fields
#[test]
fn given_error_detail_with_none_fields_when_serialize_then_omits_none() {
    let detail = ErrorDetail {
        code: "TEST".to_string(),
        message: "Test".to_string(),
        exit_code: 1,
        details: None,
        suggestion: None,
    };

    let json = serde_json::to_value(&detail).ok();
    assert!(json.is_some());

    if let Some(v) = json {
        assert!(v.get("details").is_none());
        assert!(v.get("suggestion").is_none());
        assert!(v.get("code").is_some());
        assert!(v.get("message").is_some());
    }
}

// ============================================================================
// REGRESSION TESTS for Red Queen adversarial hardening
// These tests verify fixes for issues discovered through hostile input testing
// ============================================================================

/// REGRESSION: ValidationError details were lost in JSON output
/// Previously, map_error_to_parts() discarded all ValidationError fields
/// and just returned "Validation error" as the message.
///
/// Fix: Now includes full message with field, value, and constraints.
#[test]
fn given_validation_error_with_field_when_converted_to_json_then_includes_details() {
    use crate::Error;

    let err = Error::ValidationError {
        message: "Invalid workspace state".to_string(),
        field: Some("state".to_string()),
        value: Some("invalid-state".to_string()),
        constraints: vec!["created".to_string(), "working".to_string()],
    };

    let json_err = JsonError::from(&err);

    // Message should include the actual error message
    assert!(
        json_err.error.message.contains("Invalid workspace state"),
        "Message should include original error message, got: {}",
        json_err.error.message
    );

    // Field should be included
    assert!(
        json_err.error.message.contains("field: state"),
        "Message should include field name, got: {}",
        json_err.error.message
    );

    // Value should be included
    assert!(
        json_err.error.message.contains("value: invalid-state"),
        "Message should include invalid value, got: {}",
        json_err.error.message
    );

    // Exit code should be 1 for validation errors (usage/validation errors)
    assert_eq!(json_err.error.exit_code, 1);
}

/// REGRESSION: ValidationError without field/value still works
#[test]
fn given_validation_error_without_field_when_converted_to_json_then_includes_message() {
    use crate::Error;

    let err = Error::ValidationError {
        message: "Something went wrong".to_string(),
        field: None,
        value: None,
        constraints: vec![],
    };

    let json_err = JsonError::from(&err);

    assert!(
        json_err.error.message.contains("Something went wrong"),
        "Message should include original error message, got: {}",
        json_err.error.message
    );
}

/// REGRESSION: ValidationError with constraints includes them as suggestion
#[test]
fn given_validation_error_with_constraints_when_converted_to_json_then_suggests_valid_values() {
    use crate::Error;

    let err = Error::ValidationError {
        message: "Invalid state".to_string(),
        field: Some("state".to_string()),
        value: Some("bad".to_string()),
        constraints: vec![
            "created".to_string(),
            "working".to_string(),
            "merged".to_string(),
        ],
    };

    let json_err = JsonError::from(&err);

    assert!(
        json_err.error.suggestion.is_some(),
        "Should have suggestion when constraints exist"
    );

    let suggestion = json_err.error.suggestion.unwrap();
    assert!(
        suggestion.contains("created") && suggestion.contains("working"),
        "Suggestion should list valid values, got: {}",
        suggestion
    );
}

/// REGRESSION: JSON error should include context_map in details field
#[test]
fn given_any_error_when_converted_to_json_then_includes_context_details() {
    use crate::Error;

    let err = Error::ValidationError {
        message: "Test error".to_string(),
        field: Some("test_field".to_string()),
        value: Some("test_value".to_string()),
        constraints: vec![],
    };

    let json_err = JsonError::from(&err);

    // The details field should be populated from context_map()
    // This allows AI agents to programmatically access error details
    assert!(
        json_err.error.details.is_some(),
        "JSON error should include details from context_map()"
    );

    let details = json_err.error.details.unwrap();
    // Should have structured error information
    assert!(details.is_object(), "Details should be a JSON object");
}
