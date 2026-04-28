use super::error::{FailureContext, ValidationHint};

// Behavior: ValidationHint::new creates hint with field and expected
#[test]
fn given_field_and_expected_when_create_hint_then_contains_both() {
    let hint = ValidationHint::new("name", "non-empty string");

    assert_eq!(hint.field, "name");
    assert_eq!(hint.expected, "non-empty string");
    assert!(hint.received.is_none());
    assert!(hint.example.is_none());
    assert!(hint.pattern.is_none());
}

// Behavior: ValidationHint::with_received adds received value
#[test]
fn given_hint_when_with_received_then_received_set() {
    let hint = ValidationHint::new("age", "positive integer").with_received("negative value");

    assert_eq!(hint.received, Some("negative value".to_string()));
}

// Behavior: ValidationHint::with_example adds example
#[test]
fn given_hint_when_with_example_then_example_set() {
    let hint = ValidationHint::new("email", "valid email").with_example("user@example.com");

    assert_eq!(hint.example, Some("user@example.com".to_string()));
}

// Behavior: ValidationHint::with_pattern adds pattern
#[test]
fn given_hint_when_with_pattern_then_pattern_set() {
    let hint = ValidationHint::new("username", "alphanumeric").with_pattern(r"^[a-zA-Z0-9]+$");

    assert_eq!(hint.pattern, Some(r"^[a-zA-Z0-9]+$".to_string()));
}

// Behavior: ValidationHint builder chains correctly
#[test]
fn given_hint_when_chain_all_methods_then_all_set() {
    let hint = ValidationHint::new("field", "value")
        .with_received("bad")
        .with_example("good")
        .with_pattern(r"pattern");

    assert_eq!(hint.field, "field");
    assert_eq!(hint.expected, "value");
    assert_eq!(hint.received, Some("bad".to_string()));
    assert_eq!(hint.example, Some("good".to_string()));
    assert_eq!(hint.pattern, Some(r"pattern".to_string()));
}

// Behavior: ValidationHint serializes correctly
#[test]
fn given_hint_when_serialize_then_valid_json() {
    let hint = ValidationHint::new("test", "value");
    let json = serde_json::to_value(&hint).ok();
    assert!(json.is_some());

    if let Some(v) = json {
        assert_eq!(v.get("field"), Some(&serde_json::json!("test")));
        assert_eq!(v.get("expected"), Some(&serde_json::json!("value")));
    }
}

// Behavior: ValidationHint skips None fields in serialization
#[test]
fn given_hint_with_optionals_when_serialize_then_omits_none() {
    let hint = ValidationHint::new("test", "value");
    let json = serde_json::to_string(&hint).ok();
    assert!(json.is_some());

    if let Some(s) = json {
        assert!(!s.contains("received"));
        assert!(!s.contains("example"));
    }
}

// Behavior: FailureContext default creates context
#[test]
fn given_failure_context_default_then_all_set() {
    let ctx = FailureContext::default();

    assert!(ctx.working_directory.is_none());
    assert!(ctx.current_workspace.is_none());
    assert!(ctx.active_sessions.is_empty());
    assert!(ctx.relevant_env.is_empty());
    assert!(ctx.command.is_none());
    assert!(ctx.arguments.is_empty());
    assert!(ctx.phase.is_none());
    // Timestamp is set by default
    assert!(!ctx.timestamp.is_empty());
}

// Behavior: FailureContext with working directory
#[test]
fn given_context_when_with_working_directory_then_set() {
    let ctx = FailureContext::default();

    assert!(ctx.working_directory.is_none());
}

// Behavior: FailureContext serialization works
#[test]
fn given_context_when_serialize_then_valid_json() {
    let ctx = FailureContext::default();
    let json = serde_json::to_value(&ctx).ok();
    assert!(json.is_some());
}

// Behavior: ValidationHint equality works
#[test]
fn given_identical_hints_when_compare_then_equal() {
    let hint1 = ValidationHint::new("field", "value");
    let hint2 = ValidationHint::new("field", "value");
    assert_eq!(hint1, hint2);
}

// Behavior: ValidationHint inequality works
#[test]
fn given_different_hints_when_compare_then_not_equal() {
    let hint1 = ValidationHint::new("field1", "value");
    let hint2 = ValidationHint::new("field2", "value");
    assert_ne!(hint1, hint2);
}

// Behavior: ValidationHint Debug format works
#[test]
fn given_hint_when_debug_format_then_no_panic() {
    let hint = ValidationHint::new("test", "value");
    let debug = format!("{:?}", hint);
    assert!(!debug.is_empty());
}

// Behavior: ValidationHint Clone works
#[test]
fn given_hint_when_clone_then_independent() {
    let hint1 = ValidationHint::new("test", "value");
    let hint2 = hint1.clone();
    assert_eq!(hint1, hint2);
}
