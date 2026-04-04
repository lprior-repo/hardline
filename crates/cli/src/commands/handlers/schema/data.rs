//! Data types for the schema command handler (Tier 1).
//!
//! Inert, serializable types with no business logic.

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]
#![warn(clippy::pedantic)]

use serde::{Deserialize, Serialize};

use scp_core::OutputFormat;

/// Options for the schema command (parsed from CLI).
#[derive(Debug, Clone)]
pub struct SchemaOptions {
    /// Specific schema to get (None for list/all).
    pub schema_name: Option<String>,
    /// List all available schemas.
    pub list: bool,
    /// Get all schemas.
    pub all: bool,
    /// Output format.
    pub format: OutputFormat,
}

/// Schema listing output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaListOutput {
    /// Available schemas.
    pub schemas: Vec<SchemaInfo>,
    /// Base URL for schema resolution.
    pub base_url: String,
}

/// Metadata for a single schema entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaInfo {
    /// Schema name (e.g., "add-response").
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Schema version (e.g., "1.0").
    pub version: String,
}

/// All schemas output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllSchemasOutput {
    /// Map of schema name to JSON Schema definition.
    pub schemas: serde_json::Value,
}

/// Returns the list of available schemas known to the system.
///
/// This is a pure data function that produces the canonical list of schema
/// metadata entries. It lives in the Data tier because it constructs inert
/// values with no I/O or side effects.
#[must_use]
pub fn available_schemas() -> Vec<SchemaInfo> {
    vec![
        SchemaInfo {
            name: "add-response".to_string(),
            description: "Response from scp add command".to_string(),
            version: "1.0".to_string(),
        },
        SchemaInfo {
            name: "remove-response".to_string(),
            description: "Response from scp remove command".to_string(),
            version: "1.0".to_string(),
        },
        SchemaInfo {
            name: "list-response".to_string(),
            description: "Response from scp list command".to_string(),
            version: "1.0".to_string(),
        },
        SchemaInfo {
            name: "status-response".to_string(),
            description: "Response from scp status command".to_string(),
            version: "1.0".to_string(),
        },
        SchemaInfo {
            name: "sync-response".to_string(),
            description: "Response from scp sync command".to_string(),
            version: "1.0".to_string(),
        },
        SchemaInfo {
            name: "context-response".to_string(),
            description: "Response from scp context command".to_string(),
            version: "1.0".to_string(),
        },
        SchemaInfo {
            name: "done-response".to_string(),
            description: "Response from scp done command".to_string(),
            version: "1.0".to_string(),
        },
        SchemaInfo {
            name: "spawn-response".to_string(),
            description: "Response from scp spawn command".to_string(),
            version: "1.0".to_string(),
        },
        SchemaInfo {
            name: "revert-response".to_string(),
            description: "Response from scp revert command".to_string(),
            version: "1.0".to_string(),
        },
        SchemaInfo {
            name: "error-response".to_string(),
            description: "Error response format".to_string(),
            version: "1.0".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_options_list_mode() {
        let options = SchemaOptions {
            schema_name: None,
            list: true,
            all: false,
            format: OutputFormat::Json,
        };
        assert!(options.list);
        assert!(!options.all);
        assert!(options.schema_name.is_none());
    }

    #[test]
    fn schema_options_all_mode() {
        let options = SchemaOptions {
            schema_name: None,
            list: false,
            all: true,
            format: OutputFormat::Json,
        };
        assert!(options.all);
        assert!(!options.list);
    }

    #[test]
    fn schema_options_single_schema_mode() {
        let options = SchemaOptions {
            schema_name: Some("add-response".to_string()),
            list: false,
            all: false,
            format: OutputFormat::Json,
        };
        assert_eq!(options.schema_name, Some("add-response".to_string()));
    }

    #[test]
    fn available_schemas_not_empty() {
        let schemas = available_schemas();
        assert!(!schemas.is_empty());
    }

    #[test]
    fn available_schemas_include_core_types() {
        let schemas = available_schemas();
        let names: Vec<&str> = schemas.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"add-response"), "must include add-response");
        assert!(
            names.contains(&"remove-response"),
            "must include remove-response"
        );
        assert!(names.contains(&"list-response"), "must include list-response");
        assert!(names.contains(&"error-response"), "must include error-response");
    }

    #[test]
    fn each_schema_has_required_metadata() {
        for schema in available_schemas() {
            assert!(!schema.name.is_empty(), "schema must have a name");
            assert!(
                !schema.description.is_empty(),
                "schema {} must have a description",
                schema.name
            );
            assert!(
                !schema.version.is_empty(),
                "schema {} must have a version",
                schema.name
            );
        }
    }

    #[test]
    fn schema_versions_are_valid() {
        for schema in available_schemas() {
            let parts: Vec<&str> = schema.version.split('.').collect();
            assert!(
                parts.len() >= 2,
                "version {} should have at least major.minor",
                schema.version
            );
            assert!(
                parts[0].parse::<u32>().is_ok(),
                "major version should be numeric in {}",
                schema.version
            );
        }
    }

    #[test]
    fn schema_info_serialization_roundtrip() {
        let info = SchemaInfo {
            name: "test-response".to_string(),
            description: "Test schema".to_string(),
            version: "1.0".to_string(),
        };
        let json = serde_json::to_string(&info).expect("serialize");
        let deserialized: SchemaInfo = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.name, "test-response");
        assert_eq!(deserialized.description, "Test schema");
        assert_eq!(deserialized.version, "1.0");
    }

    #[test]
    fn schema_list_output_serialization() {
        let output = SchemaListOutput {
            schemas: available_schemas(),
            base_url: "scp://".to_string(),
        };
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&output).expect("serialize"))
                .expect("deserialize");
        assert!(json.get("schemas").is_some(), "must have schemas array");
        assert!(json.get("base_url").is_some(), "must have base_url");
        assert!(json["schemas"].is_array(), "schemas must be array");
    }

    #[test]
    fn all_schemas_output_serialization() {
        let output = AllSchemasOutput {
            schemas: serde_json::json!({"test": {}}),
        };
        let json = serde_json::to_string(&output).expect("serialize");
        assert!(json.contains("test"));
    }
}
