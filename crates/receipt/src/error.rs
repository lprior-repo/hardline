use thiserror::Error;

#[derive(Error, Debug)]
pub enum ReceiptError {
    #[error("Receipt not found: {0}")]
    NotFound(String),

    #[error("Receipt corrupt: {0}")]
    Corrupt(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Deserialization error: {0}")]
    DeserializationError(String),
}

pub type Result<T> = std::result::Result<T, ReceiptError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_display() {
        let err = ReceiptError::NotFound("op-123".to_string());
        assert_eq!(err.to_string(), "Receipt not found: op-123");
    }

    #[test]
    fn storage_error_display() {
        let err = ReceiptError::StorageError("disk full".to_string());
        assert_eq!(err.to_string(), "Storage error: disk full");
    }

    #[test]
    fn corrupt_display() {
        let err = ReceiptError::Corrupt("bad data".to_string());
        assert_eq!(err.to_string(), "Receipt corrupt: bad data");
    }

    #[test]
    fn serialization_error_display() {
        let err = ReceiptError::SerializationError("bad json".to_string());
        assert_eq!(err.to_string(), "Serialization error: bad json");
    }

    #[test]
    fn deserialization_error_display() {
        let err = ReceiptError::DeserializationError("invalid".to_string());
        assert_eq!(err.to_string(), "Deserialization error: invalid");
    }

    #[test]
    fn receipt_error_implements_error_trait() {
        let err: Box<dyn std::error::Error> = Box::new(ReceiptError::NotFound("test".to_string()));
        let _msg = err.to_string();
    }

    #[test]
    fn receipt_error_is_debug() {
        let err = ReceiptError::StorageError("test".to_string());
        let debug_str = format!("{err:?}");
        assert!(debug_str.contains("StorageError"));
    }

    #[test]
    fn result_type_ok() {
        let val: Result<i32> = Ok(42);
        assert_eq!(val.expect("should be Ok"), 42);
    }

    #[test]
    fn result_type_err() {
        let val: Result<i32> = Err(ReceiptError::NotFound("x".to_string()));
        assert!(val.is_err());
    }

    #[test]
    fn result_type_map() {
        let val: Result<i32> = Ok(42);
        let mapped = val.map(|v| v * 2);
        assert_eq!(mapped.expect("should be Ok"), 84);
    }

    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig::with_cases(32))]
        #[test]
        fn receipt_error_display_always_includes_message(msg in ".{0,500}") {
            let err = ReceiptError::NotFound(msg.clone());
            let display = err.to_string();
            proptest::prop_assert!(display.contains(&msg));
        }
    }
}
