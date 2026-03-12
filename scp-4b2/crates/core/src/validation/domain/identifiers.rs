//! Identifier validation types
//!
//! This module provides identifier-specific error types.

use crate::error::Error;

pub type IdentifierError = Error;

pub fn validate_identifier(s: &str) -> Result<(), Error> {
    if s.is_empty() {
        return Err(Error::InvalidIdentifier(
            "identifier cannot be empty".to_string(),
        ));
    }

    if !s.is_ascii() {
        return Err(Error::InvalidIdentifier(
            "identifier must contain only ASCII characters".to_string(),
        ));
    }

    if s.contains('\0') {
        return Err(Error::InvalidIdentifier(
            "identifier cannot contain null bytes".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_identifier_valid() {
        assert!(validate_identifier("my-identifier").is_ok());
        assert!(validate_identifier("ID_123").is_ok());
    }

    #[test]
    fn test_validate_identifier_empty() {
        assert!(validate_identifier("").is_err());
    }

    #[test]
    fn test_validate_identifier_non_ascii() {
        assert!(validate_identifier("ident-日本語").is_err());
    }
}
