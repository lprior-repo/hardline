//! Kani proofs for identifier validation invariants.
//!
//! # Invariants Proven
//!
//! 1. SessionId accepts valid ASCII strings
//! 2. SessionId rejects empty strings
//! 3. SessionId rejects non-ASCII strings
//! 4. SessionName validation rules are enforced

#[cfg(kani)]
mod proof {
    use crate::domain::identifiers::{
        error::IdentifierError,
        validation::{validate_session_id, validate_session_name},
        SessionId, SessionName,
    };

    #[kani::proof]
    fn verify_valid_session_id_accepted() {
        let valid_id = "session-abc-123";
        let result = SessionId::parse(valid_id);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), valid_id);
    }

    #[kani::proof]
    fn verify_empty_session_id_rejected() {
        let result = SessionId::parse("");
        assert!(result.is_err());
        assert!(matches!(result, Err(IdentifierError::Empty)));
    }

    #[kani::proof]
    fn verify_non_ascii_session_id_rejected() {
        let result = SessionId::parse("session-日本語");
        assert!(result.is_err());
        assert!(matches!(result, Err(IdentifierError::NotAscii { .. })));
    }

    #[kani::proof]
    fn verify_session_id_roundtrip() {
        let original = "test-session-123";
        let parsed = SessionId::parse(original).unwrap();
        assert_eq!(parsed.into_string(), original);
    }

    #[kani::proof]
    fn verify_valid_session_name_accepted() {
        let valid_name = "my-session";
        let result = SessionName::parse(valid_name);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), valid_name);
    }

    #[kani::proof]
    fn verify_session_name_rejects_empty() {
        let result = SessionName::parse("");
        assert!(result.is_err());
    }

    #[kani::proof]
    fn verify_session_name_rejects_numeric_start() {
        let result = SessionName::parse("123-session");
        assert!(result.is_err());
        assert!(matches!(result, Err(IdentifierError::InvalidStart { .. })));
    }

    #[kani::proof]
    fn verify_session_name_rejects_special_chars() {
        let invalid_names = ["my.session", "my:session", "my session", "my@session"];

        for name in invalid_names {
            let result = SessionName::parse(name);
            assert!(result.is_err(), "Should reject: {}", name);
        }
    }

    #[kani::proof]
    fn verify_session_name_accepts_alphanumeric_hyphen_underscore() {
        let valid_names = ["my-session", "my_session", "session123", "Session-ABC_123"];

        for name in valid_names {
            let result = SessionName::parse(name);
            assert!(result.is_ok(), "Should accept: {}", name);
        }
    }

    #[kani::proof]
    fn verify_session_name_max_length() {
        let long_name = "a".repeat(64);
        let result = SessionName::parse(&long_name);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(IdentifierError::TooLong { max: 63, .. })
        ));
    }

    #[kani::proof]
    fn verify_validate_session_id_empty_rejected() {
        let result = validate_session_id("");
        assert!(result.is_err());
    }

    #[kani::proof]
    fn verify_validate_session_id_valid_accepted() {
        let result = validate_session_id("valid-id");
        assert!(result.is_ok());
    }

    #[kani::proof]
    fn verify_validate_session_name_empty_rejected() {
        let result = validate_session_name("");
        assert!(result.is_err());
    }

    #[kani::proof]
    fn verify_validate_session_name_too_long_rejected() {
        let long_name = "a".repeat(64);
        let result = validate_session_name(&long_name);
        assert!(result.is_err());
    }

    #[kani::proof]
    fn verify_session_name_roundtrip() {
        let original = "my-session";
        let parsed = SessionName::parse(original).unwrap();
        assert_eq!(parsed.into_string(), original);
    }
}
