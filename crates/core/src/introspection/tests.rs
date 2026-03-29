//! Introspection module tests
//!
//! Tests for introspection types, health checks, and name suggestion.

#[cfg(test)]
mod tests {
    use crate::introspection::{
        suggest_name, ArgumentSpec, Capabilities, CheckStatus, CommandExample,
        CommandIntrospection, DependencyInfo, DoctorCheck, DoctorOutput, ErrorCondition, FlagSpec,
        IntrospectOutput, Prerequisites, SystemState,
    };
    use crate::Error;

    #[test]
    fn test_introspect_output_new() {
        let output = IntrospectOutput::new("0.1.0");
        assert_eq!(output.isolate_version, "0.1.0");
        assert!(!output.capabilities.session_management.commands.is_empty());
    }

    #[test]
    fn test_capabilities_default() {
        let caps = Capabilities::default();
        assert!(caps
            .session_management
            .commands
            .contains(&"add".to_string()));
        assert!(caps.introspection.commands.contains(&"doctor".to_string()));
    }

    #[test]
    fn test_prerequisites_all_met() {
        let prereqs = Prerequisites {
            initialized: true,
            jj_installed: true,
            custom: vec![],
        };
        assert!(prereqs.all_met());
    }

    #[test]
    fn test_prerequisites_not_met() {
        let prereqs = Prerequisites {
            initialized: false,
            jj_installed: true,
            custom: vec![],
        };
        assert!(!prereqs.all_met());
    }

    #[test]
    fn test_prerequisites_count() {
        let prereqs = Prerequisites {
            initialized: true,
            jj_installed: true,
            custom: vec![],
        };
        assert_eq!(prereqs.count_met(), 2);
        assert_eq!(prereqs.total(), 2);
    }

    #[test]
    fn test_doctor_output_from_checks() {
        let checks = vec![
            DoctorCheck {
                name: "Check 1".to_string(),
                status: CheckStatus::Pass,
                message: "OK".to_string(),
                suggestion: None,
                auto_fixable: false,
                details: None,
            },
            DoctorCheck {
                name: "Check 2".to_string(),
                status: CheckStatus::Warn,
                message: "Warning".to_string(),
                suggestion: Some("Fix it".to_string()),
                auto_fixable: true,
                details: None,
            },
            DoctorCheck {
                name: "Check 3".to_string(),
                status: CheckStatus::Fail,
                message: "Error".to_string(),
                suggestion: None,
                auto_fixable: false,
                details: None,
            },
        ];

        let output = DoctorOutput::from_checks(checks);
        assert!(!output.healthy);
        assert_eq!(output.warnings, 1);
        assert_eq!(output.errors, 1);
        assert_eq!(output.auto_fixable_issues, 1);
    }

    #[test]
    fn test_suggest_name_basic() -> Result<(), Error> {
        let existing = vec!["feature-1".to_string(), "feature-2".to_string()];
        let result = suggest_name("feature-{n}", &existing)?;
        assert_eq!(result.suggested, "feature-3");
        assert_eq!(result.next_available_n, 3);
        assert_eq!(result.existing_matches.len(), 2);
        Ok(())
    }

    #[test]
    fn test_suggest_name_gap() -> Result<(), Error> {
        let existing = vec!["test-1".to_string(), "test-3".to_string()];
        let result = suggest_name("test-{n}", &existing)?;
        assert_eq!(result.suggested, "test-2");
        assert_eq!(result.next_available_n, 2);
        Ok(())
    }

    #[test]
    fn test_suggest_name_no_existing() -> Result<(), Error> {
        let existing = vec![];
        let result = suggest_name("bug-{n}", &existing)?;
        assert_eq!(result.suggested, "bug-1");
        assert_eq!(result.next_available_n, 1);
        assert_eq!(result.existing_matches.len(), 0);
        Ok(())
    }

    #[test]
    fn test_suggest_name_invalid_pattern() {
        let result = suggest_name("no-placeholder", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_suggest_name_multiple_placeholders() {
        let result = suggest_name("test-{n}-{n}", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_suggest_name_requires_braced_placeholder() {
        let result = suggest_name("feat", &[]);
        assert!(result.is_err());
        assert!(matches!(result, Err(Error::State(_))));
    }

    #[test]
    fn test_suggest_name_with_feat_placeholder() {
        let existing = vec!["feat1".to_string(), "feat2".to_string()];
        let result = suggest_name("feat{n}", &existing);
        let result = match result {
            Ok(r) => r,
            Err(e) => panic!("suggest_name failed: {e}"),
        };
        assert_eq!(result.suggested, "feat3");
        assert_eq!(result.next_available_n, 3);
        assert_eq!(result.existing_matches.len(), 2);
    }

    #[test]
    fn test_flag_validate_category_valid() {
        assert!(FlagSpec::validate_category("behavior").is_ok());
        assert!(FlagSpec::validate_category("configuration").is_ok());
        assert!(FlagSpec::validate_category("filter").is_ok());
        assert!(FlagSpec::validate_category("output").is_ok());
        assert!(FlagSpec::validate_category("advanced").is_ok());
    }

    #[test]
    fn test_flag_validate_category_invalid() {
        assert!(FlagSpec::validate_category("invalid").is_err());
        assert!(FlagSpec::validate_category("").is_err());
    }

    #[test]
    fn test_doctor_check_serialization() {
        let check = DoctorCheck {
            name: "test".to_string(),
            status: CheckStatus::Pass,
            message: "OK".to_string(),
            suggestion: None,
            auto_fixable: false,
            details: None,
        };
        let json = serde_json::to_string(&check).unwrap();
        assert!(json.contains("\"status\":\"pass\""));
    }

    #[test]
    fn test_command_introspection_full() {
        let cmd = CommandIntrospection {
            command: "test".to_string(),
            description: "A test command".to_string(),
            aliases: vec!["t".to_string()],
            arguments: vec![ArgumentSpec {
                name: "name".to_string(),
                arg_type: "string".to_string(),
                required: true,
                description: "Test argument".to_string(),
                validation: None,
                examples: vec![],
            }],
            flags: vec![FlagSpec {
                long: "verbose".to_string(),
                short: Some("v".to_string()),
                description: "Enable verbose output".to_string(),
                flag_type: "boolean".to_string(),
                default: None,
                possible_values: vec![],
                category: Some("output".to_string()),
            }],
            examples: vec![CommandExample {
                command: "scp test --verbose".to_string(),
                description: "Run with verbose output".to_string(),
            }],
            prerequisites: Prerequisites {
                initialized: true,
                jj_installed: false,
                custom: vec![],
            },
            side_effects: vec![],
            error_conditions: vec![ErrorCondition {
                code: "ERR_TEST".to_string(),
                description: "Test error".to_string(),
                resolution: "Fix it".to_string(),
            }],
        };

        assert_eq!(cmd.command, "test");
        assert!(!cmd.aliases.is_empty());
        assert!(!cmd.arguments.is_empty());
        assert!(!cmd.flags.is_empty());
        assert!(!cmd.examples.is_empty());
        assert!(!cmd.error_conditions.is_empty());
    }

    #[test]
    fn test_system_state_default() {
        let state = SystemState::default();
        assert!(!state.initialized);
        assert!(!state.jj_repo);
        assert!(state.config_path.is_none());
        assert!(state.state_db.is_none());
        assert_eq!(state.sessions_count, 0);
        assert_eq!(state.active_sessions, 0);
    }

    #[test]
    fn test_dependency_info_serialization() {
        let dep = DependencyInfo {
            required: true,
            installed: true,
            version: Some("1.0.0".to_string()),
            command: "jj".to_string(),
        };
        let json = serde_json::to_string(&dep).unwrap();
        assert!(json.contains("\"required\":true"));
        assert!(json.contains("\"installed\":true"));
        assert!(json.contains("\"version\":\"1.0.0\""));
    }
}
