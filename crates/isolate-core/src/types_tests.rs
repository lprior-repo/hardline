use std::str::FromStr;

use super::types::SessionName;

// Behavior: SessionName::new accepts valid names
#[test]
fn given_valid_session_name_when_create_then_success() {
    let result = SessionName::new("valid-name");
    assert!(result.is_ok());
    let name = result.ok().unwrap();
    assert_eq!(name.as_str(), "valid-name");
}

// Behavior: SessionName::new accepts alphanumeric names
#[test]
fn given_alphanumeric_name_when_create_then_success() {
    let result = SessionName::new("myFeature123");
    assert!(result.is_ok());
}

// Behavior: SessionName::new accepts names with underscores
#[test]
fn given_underscore_name_when_create_then_success() {
    let result = SessionName::new("my_feature");
    assert!(result.is_ok());
}

// Behavior: SessionName::new accepts names with dashes
#[test]
fn given_dash_name_when_create_then_success() {
    let result = SessionName::new("my-feature");
    assert!(result.is_ok());
}

// Behavior: SessionName::new rejects empty names
#[test]
fn given_empty_name_when_create_then_error() {
    let result = SessionName::new("");
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert!(err.to_string().to_lowercase().contains("empty"));
}

// Behavior: SessionName::new rejects names starting with number
#[test]
fn given_number_prefix_when_create_then_error() {
    let result = SessionName::new("123feature");
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert!(err.to_string().to_lowercase().contains("letter"));
}

// Behavior: SessionName::new rejects names with special characters
#[test]
fn given_special_chars_when_create_then_error() {
    let result = SessionName::new("feature@test");
    assert!(result.is_err());

    let result2 = SessionName::new("feature!name");
    assert!(result2.is_err());

    let result3 = SessionName::new("name with space");
    assert!(result3.is_err());
}

// Behavior: SessionName::new rejects names exceeding max length
#[test]
fn given_name_too_long_when_create_then_error() {
    let long_name = "a".repeat(64); // MAX_LENGTH is 63
    let result = SessionName::new(long_name);
    assert!(result.is_err());
}

// Behavior: SessionName::new accepts names at max length
#[test]
fn given_name_at_max_length_when_create_then_success() {
    let max_name = "a".repeat(63);
    let result = SessionName::new(max_name);
    assert!(result.is_ok());
}

// Behavior: SessionName FromStr works
#[test]
fn given_string_when_from_str_then_parses() {
    let result = SessionName::from_str("test-name");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().as_str(), "test-name");
}

// Behavior: SessionName as_str returns inner value
#[test]
fn given_session_name_when_as_str_then_returns_inner() {
    let name = SessionName::new("test-name").ok().unwrap();
    assert_eq!(name.as_str(), "test-name");
}

// Behavior: SessionName Clone works
#[test]
fn given_session_name_when_clone_then_independent() {
    let name1 = SessionName::new("test").ok().unwrap();
    let name2 = name1.clone();
    assert_eq!(name1.as_str(), name2.as_str());
}

// Behavior: SessionName equality works
#[test]
fn given_same_name_when_compare_then_equal() {
    let name1 = SessionName::new("test").ok().unwrap();
    let name2 = SessionName::new("test").ok().unwrap();
    assert_eq!(name1, name2);
}

// Behavior: SessionName inequality works
#[test]
fn given_different_names_when_compare_then_not_equal() {
    let name1 = SessionName::new("test1").ok().unwrap();
    let name2 = SessionName::new("test2").ok().unwrap();
    assert_ne!(name1, name2);
}

// Behavior: SessionName MAX_LENGTH constant is 63
#[test]
fn given_max_length_constant_then_is_63() {
    assert_eq!(SessionName::MAX_LENGTH, 63);
}
