#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::should_implement_trait)]
#![allow(unknown_lints)]
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Payload(serde_json::Value);

impl Payload {
    pub fn new(value: serde_json::Value) -> Result<Self, JobCreationError> {
        Ok(Self(value))
    }

    pub fn from_str(s: &str) -> Result<Self, JobCreationError> {
        let value: serde_json::Value = serde_json::from_str(s)
            .map_err(|_| JobCreationError::InvalidPayload(PayloadError::MalformedJson))?;
        Ok(Self(value))
    }

    pub fn empty() -> Result<Self, JobCreationError> {
        Err(JobCreationError::InvalidPayload(PayloadError::Empty))
    }

    #[must_use]
    pub fn as_value(&self) -> &serde_json::Value {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> serde_json::Value {
        self.0
    }
}

impl std::fmt::Display for Payload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub enum PayloadError {
    Empty,
    MalformedJson,
}

impl std::fmt::Display for PayloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "payload cannot be empty"),
            Self::MalformedJson => write!(f, "payload must be valid JSON"),
        }
    }
}

impl std::error::Error for PayloadError {}

use super::job_id::JobCreationError;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_valid_json() {
        let payload = Payload::from_str(r#"{"key":"value"}"#);
        assert!(payload.is_ok());
    }

    #[test]
    fn payload_malformed_json_rejected() {
        let payload = Payload::from_str("not json {{");
        assert!(payload.is_err());
    }

    #[test]
    fn payload_empty_rejected() {
        let payload = Payload::empty();
        assert!(payload.is_err());
    }

    #[test]
    fn payload_new_with_value() {
        let value = serde_json::json!({"key": "value"});
        let payload = Payload::new(value.clone());
        assert!(payload.is_ok());
        let p = payload.unwrap();
        assert_eq!(p.as_value(), &value);
    }

    #[test]
    fn payload_display() {
        let payload = Payload::from_str(r#"{"a":1}"#).unwrap();
        let display = format!("{payload}");
        assert!(display.contains("a"));
    }

    #[test]
    fn payload_as_value() {
        let payload = Payload::from_str(r#"{"x":42}"#).unwrap();
        let val = payload.as_value();
        assert_eq!(val["x"], 42);
    }

    #[test]
    fn payload_into_inner() {
        let payload = Payload::from_str(r#"{"y":true}"#).unwrap();
        let inner = payload.into_inner();
        assert_eq!(inner["y"], true);
    }

    #[test]
    fn payload_clone_and_eq() {
        let a = Payload::from_str(r#"{"z":1}"#).unwrap();
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn payload_serde_roundtrip() {
        let payload = Payload::from_str(r#"{"nested":{"val":99}}"#).unwrap();
        let json = serde_json::to_string(&payload).unwrap();
        let back: Payload = serde_json::from_str(&json).unwrap();
        assert_eq!(back.as_value()["nested"]["val"], 99);
    }

    #[test]
    fn payload_empty_json_object_valid() {
        let payload = Payload::from_str(r#"{}"#);
        assert!(payload.is_ok());
    }

    #[test]
    fn payload_json_array_valid() {
        let payload = Payload::from_str(r#"[1,2,3]"#);
        assert!(payload.is_ok());
    }

    #[test]
    fn payload_json_string_valid() {
        let payload = Payload::from_str(r#""hello""#);
        assert!(payload.is_ok());
    }

    #[test]
    fn payload_error_empty_display() {
        let err = PayloadError::Empty;
        assert_eq!(format!("{err}"), "payload cannot be empty");
    }

    #[test]
    fn payload_error_malformed_json_display() {
        let err = PayloadError::MalformedJson;
        assert_eq!(format!("{err}"), "payload must be valid JSON");
    }

    #[test]
    fn payload_error_implements_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(PayloadError::Empty);
        let _ = format!("{err:?}");
    }

    #[test]
    fn payload_error_clone() {
        let a = PayloadError::Empty;
        let b = a.clone();
        assert_eq!(format!("{a}"), format!("{b}"));
    }

    #[test]
    fn payload_from_str_partial_json_rejected() {
        let payload = Payload::from_str(r#"{"incomplete""#);
        assert!(payload.is_err());
        if let Err(JobCreationError::InvalidPayload(PayloadError::MalformedJson)) = payload {
            // expected
        } else {
            panic!("Expected MalformedJson error variant");
        }
    }

    #[test]
    fn payload_empty_returns_correct_error_variant() {
        let result = Payload::empty();
        assert!(matches!(
            result,
            Err(JobCreationError::InvalidPayload(PayloadError::Empty))
        ));
    }

    // --- Additional comprehensive tests ---

    #[test]
    fn payload_from_str_valid_json_number() {
        let payload = Payload::from_str("42");
        assert!(payload.is_ok());
        assert_eq!(payload.unwrap().as_value(), 42);
    }

    #[test]
    fn payload_from_str_valid_json_bool() {
        let payload = Payload::from_str("true");
        assert!(payload.is_ok());
        assert_eq!(payload.unwrap().as_value(), true);
    }

    #[test]
    fn payload_from_str_valid_json_null() {
        let payload = Payload::from_str("null");
        assert!(payload.is_ok());
        assert!(payload.unwrap().as_value().is_null());
    }

    #[test]
    fn payload_from_str_empty_string_rejected() {
        let payload = Payload::from_str("");
        assert!(payload.is_err());
        assert!(matches!(
            payload,
            Err(JobCreationError::InvalidPayload(
                PayloadError::MalformedJson
            ))
        ));
    }

    #[test]
    fn payload_from_str_whitespace_rejected() {
        let payload = Payload::from_str("   ");
        assert!(payload.is_err());
    }

    #[test]
    fn payload_new_with_complex_object() {
        let value = serde_json::json!({
            "name": "test",
            "nested": {"key": "value"},
            "array": [1, 2, 3]
        });
        let payload = Payload::new(value.clone()).unwrap();
        assert_eq!(payload.as_value(), &value);
    }

    #[test]
    fn payload_display_empty_object() {
        let payload = Payload::from_str("{}").unwrap();
        let display = format!("{payload}");
        assert_eq!(display, "{}");
    }

    #[test]
    fn payload_display_string_value() {
        let payload = Payload::from_str("\"hello world\"").unwrap();
        let display = format!("{payload}");
        assert!(display.contains("hello world"));
    }

    #[test]
    fn payload_clone_independence() {
        let a = Payload::from_str(r#"{"x":1}"#).unwrap();
        let _b = a.clone();
        // Both should still be usable
        assert_eq!(a.as_value()["x"], 1);
    }

    #[test]
    fn payload_serde_roundtrip_complex_object() {
        let payload = Payload::from_str(r#"{"nested":{"deep":{"val":42}}}"#).unwrap();
        let json = serde_json::to_string(&payload).unwrap();
        let back: Payload = serde_json::from_str(&json).unwrap();
        assert_eq!(back.as_value()["nested"]["deep"]["val"], 42);
    }

    #[test]
    fn payload_serde_roundtrip_array() {
        let payload = Payload::from_str(r#"[1,"two",true,null]"#).unwrap();
        let json = serde_json::to_string(&payload).unwrap();
        let back: Payload = serde_json::from_str(&json).unwrap();
        assert_eq!(back.as_value().as_array().unwrap().len(), 4);
    }

    #[test]
    fn payload_error_empty_debug() {
        let err = PayloadError::Empty;
        let debug = format!("{err:?}");
        assert!(debug.contains("Empty"));
    }

    #[test]
    fn payload_error_malformed_debug() {
        let err = PayloadError::MalformedJson;
        let debug = format!("{err:?}");
        assert!(debug.contains("MalformedJson"));
    }

    #[test]
    fn payload_error_match_on_variants() {
        let empty = PayloadError::Empty;
        let malformed = PayloadError::MalformedJson;
        // PayloadError doesn't derive PartialEq, use match
        match empty {
            PayloadError::Empty => {}
            PayloadError::MalformedJson => panic!("Expected Empty"),
        }
        match malformed {
            PayloadError::MalformedJson => {}
            PayloadError::Empty => panic!("Expected MalformedJson"),
        }
    }

    #[test]
    fn payload_as_value_immutable_borrow() {
        let payload = Payload::from_str(r#"{"key":"value"}"#).unwrap();
        let v1 = payload.as_value();
        let v2 = payload.as_value();
        // Both borrows should be valid simultaneously
        assert_eq!(v1["key"], "value");
        assert_eq!(v2["key"], "value");
    }

    #[test]
    fn payload_into_inner_consumes() {
        let payload = Payload::from_str(r#"{"consumed":true}"#).unwrap();
        let inner = payload.into_inner();
        assert_eq!(inner["consumed"], true);
    }

    #[test]
    fn payload_eq_same_content() {
        let a = Payload::from_str(r#"{"same":1}"#).unwrap();
        let b = Payload::from_str(r#"{"same":1}"#).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn payload_ne_different_content() {
        let a = Payload::from_str(r#"{"a":1}"#).unwrap();
        let b = Payload::from_str(r#"{"b":2}"#).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn payload_from_str_json_with_unicode() {
        let payload = Payload::from_str(r#"{"message":"Hello, 世界! 🌍"}"#);
        assert!(payload.is_ok());
    }

    // --- Proptests ---

    use proptest::prelude::*;
    use proptest::{prop_assert, prop_assert_eq};

    proptest! {
        #[test]
        fn proptest_payload_roundtrip_json(
            json_key in "[a-zA-Z0-9]{1,20}",
            json_val in "[a-zA-Z0-9]{1,20}"
        ) {
            let valid_json = format!(r#"{{"{}":"{}"}}"#, json_key, json_val);
            let payload = Payload::from_str(&valid_json).unwrap();
            let json_str = serde_json::to_string(&payload).unwrap();
            let back: Payload = serde_json::from_str(&json_str).unwrap();
            prop_assert_eq!(&back.as_value()[json_key], &json_val);
        }

        #[test]
        fn proptest_payload_display_not_empty(
            json_val in "[a-zA-Z0-9]+"
        ) {
            let valid_json = format!(r#"{{"key":"{}"}}"#, json_val);
            let payload = Payload::from_str(&valid_json).unwrap();
            let display = format!("{payload}");
            prop_assert!(!display.is_empty());
        }
    }
}
