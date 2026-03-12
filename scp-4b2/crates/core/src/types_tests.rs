use crate::types::SessionName;

#[test]
fn given_valid_session_name_when_create_then_success() {
    let result = SessionName::new("valid-name");
    assert!(result.is_ok());
    let name = result.ok().unwrap();
    assert_eq!(name.as_str(), "valid-name");
}

#[test]
fn given_alphanumeric_name_when_create_then_success() {
    let result = SessionName::new("myFeature123");
    assert!(result.is_ok());
}

#[test]
fn given_underscore_name_when_create_then_success() {
    let result = SessionName::new("my_feature");
    assert!(result.is_ok());
}

#[test]
fn given_dash_name_when_create_then_success() {
    let result = SessionName::new("my-feature");
    assert!(result.is_ok());
}

#[test]
fn given_empty_name_when_create_then_error() {
    let result = SessionName::new("");
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert!(err.to_string().to_lowercase().contains("empty"));
}

#[test]
fn given_number_prefix_when_create_then_error() {
    let result = SessionName::new("123feature");
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert!(err.to_string().to_lowercase().contains("letter"));
}

#[test]
fn given_special_chars_when_create_then_error() {
    let result = SessionName::new("feature@test");
    assert!(result.is_err());

    let result2 = SessionName::new("feature!name");
    assert!(result2.is_err());

    let result3 = SessionName::new("name with space");
    assert!(result3.is_err());
}

#[test]
fn given_name_too_long_when_create_then_error() {
    let long_name = "a".repeat(64);
    let result = SessionName::new(long_name);
    assert!(result.is_err());
}

#[test]
fn given_name_at_max_length_when_create_then_success() {
    let max_name = "a".repeat(63);
    let result = SessionName::new(max_name);
    assert!(result.is_ok());
}

#[test]
fn given_string_when_from_str_then_parses() {
    let result = SessionName::parse("test-name");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().as_str(), "test-name");
}

#[test]
fn given_session_name_when_as_str_then_returns_inner() {
    let name = SessionName::new("test-name").ok().unwrap();
    assert_eq!(name.as_str(), "test-name");
}

#[test]
fn given_session_name_when_clone_then_independent() {
    let name1 = SessionName::new("test").ok().unwrap();
    let name2 = name1.clone();
    assert_eq!(name1.as_str(), name2.as_str());
}

#[test]
fn given_same_name_when_compare_then_equal() {
    let name1 = SessionName::new("test").ok().unwrap();
    let name2 = SessionName::new("test").ok().unwrap();
    assert_eq!(name1, name2);
}

#[test]
fn given_different_names_when_compare_then_not_equal() {
    let name1 = SessionName::new("test1").ok().unwrap();
    let name2 = SessionName::new("test2").ok().unwrap();
    assert_ne!(name1, name2);
}

#[test]
fn given_max_length_constant_then_is_63() {
    let exactly_63: String = "a".repeat(63);
    assert!(
        SessionName::new(&exactly_63).is_ok(),
        "63 chars should be valid"
    );

    let too_long: String = "a".repeat(64);
    assert!(
        SessionName::new(&too_long).is_err(),
        "64 chars should be invalid"
    );
}
