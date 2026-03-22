//! Tests for contract validation




// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

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
            min: Some(1),
            max: Some(64),
        };

        assert!(constraint.validate_string("").is_err());
    }

    #[test]
    fn test_length_constraint_too_long() {
        let constraint = Constraint::Length {
            min: Some(1),
            max: Some(64),
        };

        assert!(constraint.validate_string(&"x".repeat(65)).is_err());
    }

    #[test]
    fn test_range_constraint_valid() {
        let constraint = Constraint::Range {
            min: Some(10),
            max: Some(5000),
            inclusive: true,
        };

        assert!(constraint.validate_number(10).is_ok());
        assert!(constraint.validate_number(100).is_ok());
        assert!(constraint.validate_number(5000).is_ok());
    }

    #[test]
    fn test_range_constraint_too_low() {
        let constraint = Constraint::Range {
            min: Some(10),
            max: Some(5000),
            inclusive: true,
        };

        assert!(constraint.validate_number(9).is_err());
    }

    #[test]
    fn test_range_constraint_too_high() {
        let constraint = Constraint::Range {
            min: Some(10),
            max: Some(5000),
            inclusive: true,
        };

        assert!(constraint.validate_number(5001).is_err());
    }

    #[test]
    fn test_enum_constraint_valid() {
        let constraint = Constraint::Enum {
            values: vec![
                "active".to_string(),
                "paused".to_string(),
                "completed".to_string(),
            ],
        };

        assert!(constraint.validate_string("active").is_ok());
        assert!(constraint.validate_string("paused").is_ok());
        assert!(constraint.validate_string("completed").is_ok());
    }

    #[test]
    fn test_enum_constraint_invalid() {
        let constraint = Constraint::Enum {
            values: vec!["active".to_string(), "paused".to_string()],
        };

        assert!(constraint.validate_string("invalid").is_err());
    }

    #[test]
    fn test_path_absolute_constraint() {
        let constraint = Constraint::PathAbsolute;

        assert!(constraint
            .validate_path(std::path::Path::new("/absolute/path"))
            .is_ok());
        assert!(constraint
            .validate_path(std::path::Path::new("relative/path"))
            .is_err());
    }

    #[test]
    fn test_contract_builder() {
        let contract = TypeContract::builder("TestType")
            .description("A test type")
            .example("example1")
            .build();

        assert_eq!(contract.name, "TestType");
        assert_eq!(contract.description, "A test type");
        assert_eq!(contract.examples.len(), 1);
    }

    #[test]
    fn test_field_contract_builder() {
        let field = FieldContract::builder("name", "String")
            .required()
            .description("The name field")
            .constraint(Constraint::Length {
                min: Some(1),
                max: Some(64),
            })
            .example("my-session")
            .build();

        assert_eq!(field.name, "name");
        assert_eq!(field.field_type, "String");
        assert!(field.required);
        assert_eq!(field.constraints.len(), 1);
        assert_eq!(field.examples.len(), 1);
    }

    #[test]
    fn test_json_schema_generation() {
        let field = FieldContract::builder("name", "String")
            .required()
            .description("Session name")
            .constraint(Constraint::Regex {
                pattern: r"^[a-z0-9_-]+$".to_string(),
                description: "alphanumeric".to_string(),
            })
            .build();

        let contract = TypeContract::builder("Session")
            .description("A session")
            .field("name", field)
            .build();

        let schema = contract.to_json_schema();

        #[allow(clippy::indexing_slicing)]
        {
            assert_eq!(schema["type"], "object");
            assert_eq!(schema["title"], "Session");
            assert!(schema["properties"].is_object());
            assert!(schema["required"].is_array());
        }
    }
}
