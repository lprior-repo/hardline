//! Tests for contract validation

#[cfg(test)]
mod tests {
    use crate::contracts::types::Constraint;

    #[test]
    fn test_regex_constraint_valid() {
        let constraint = Constraint::Regex {
            pattern: r"^[a-z0-9_-]+$".to_string(),
            description: "alphanumeric with hyphens and underscores".to_string(),
        };

        assert!(constraint.validate_string("my-session").is_ok());
        assert!(constraint.validate_string("test_123").is_ok());
    }

    #[test]
    fn test_regex_constraint_invalid() {
        let constraint = Constraint::Regex {
            pattern: r"^[a-z0-9_-]+$".to_string(),
            description: "alphanumeric with hyphens and underscores".to_string(),
        };

        assert!(constraint.validate_string("invalid session").is_err());
        assert!(constraint.validate_string("UPPERCASE").is_err());
    }

    #[test]
    fn test_length_constraint_valid() {
        let constraint = Constraint::Length {
            min: Some(1),
            max: Some(64),
        };

        assert!(constraint.validate_string("valid").is_ok());
        assert!(constraint.validate_string("a").is_ok());
        assert!(constraint.validate_string(&"x".repeat(64)).is_ok());
    }

    #[test]
    fn test_length_constraint_too_short() {
        let constraint = Constraint::Length {
            min: Some(5),
            max: Some(64),
        };

        assert!(constraint.validate_string("ab").is_err());
    }

    #[test]
    fn test_length_constraint_too_long() {
        let constraint = Constraint::Length {
            min: Some(1),
            max: Some(10),
        };

        assert!(constraint.validate_string(&"x".repeat(11)).is_err());
    }
}
