//! Data types for the schema command handler (Tier 1).
//!
//! Inert, serializable types with no business logic.

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]
#![warn(clippy::pedantic)]

use serde::{Deserialize, Serialize};

use scp_core::OutputFormat;

/// Execution mode for the schema command.
///
/// Models mutually exclusive operational modes as a single enum
/// rather than multiple boolean flags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaMode {
    /// List all available schemas.
    List,
    /// Dump all schema definitions.
    All,
    /// Show a single named schema.
    Single(String),
}

/// Options for the schema command (parsed from CLI).
#[derive(Debug, Clone)]
pub struct SchemaOptions {
    /// Execution mode (list / all / single schema).
    pub mode: SchemaMode,
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

/// Create a `SchemaInfo` from a name and description (version defaults to "1.0").
#[must_use]
fn schema_entry(name: &str, description: &str) -> SchemaInfo {
    SchemaInfo {
        name: name.to_string(),
        description: description.to_string(),
        version: "1.0".to_string(),
    }
}

/// Returns the list of available schemas known to the system.
///
/// This is a pure data function that produces the canonical list of schema
/// metadata entries. It lives in the Data tier because it constructs inert
/// values with no I/O or side effects.
#[must_use]
pub fn available_schemas() -> Vec<SchemaInfo> {
    vec![
        schema_entry("add-response", "Response from scp add command"),
        schema_entry("remove-response", "Response from scp remove command"),
        schema_entry("list-response", "Response from scp list command"),
        schema_entry("status-response", "Response from scp status command"),
        schema_entry("sync-response", "Response from scp sync command"),
        schema_entry("context-response", "Response from scp context command"),
        schema_entry("done-response", "Response from scp done command"),
        schema_entry("spawn-response", "Response from scp spawn command"),
        schema_entry("revert-response", "Response from scp revert command"),
        schema_entry("error-response", "Error response format"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_options_list_mode() {
        let options = SchemaOptions {
            mode: SchemaMode::List,
            format: OutputFormat::Json,
        };
        assert_eq!(options.mode, SchemaMode::List);
    }

    #[test]
    fn schema_options_all_mode() {
        let options = SchemaOptions {
            mode: SchemaMode::All,
            format: OutputFormat::Json,
        };
        assert_eq!(options.mode, SchemaMode::All);
    }

    #[test]
    fn schema_options_single_schema_mode() {
        let options = SchemaOptions {
            mode: SchemaMode::Single("add-response".to_string()),
            format: OutputFormat::Json,
        };
        assert_eq!(options.mode, SchemaMode::Single("add-response".to_string()));
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

    // ── Exhaustive tests: SchemaMode ──────────────────────────────────────

    #[test]
    fn schema_mode_equality() {
        assert_eq!(SchemaMode::List, SchemaMode::List);
        assert_eq!(SchemaMode::All, SchemaMode::All);
        assert_eq!(
            SchemaMode::Single("foo".to_string()),
            SchemaMode::Single("foo".to_string())
        );
    }

    #[test]
    fn schema_mode_inequality() {
        assert_ne!(SchemaMode::List, SchemaMode::All);
        assert_ne!(SchemaMode::All, SchemaMode::Single("x".to_string()));
        assert_ne!(
            SchemaMode::Single("a".to_string()),
            SchemaMode::Single("b".to_string())
        );
    }

    #[test]
    fn schema_mode_clone_preserves_value() {
        let modes = [
            SchemaMode::List,
            SchemaMode::All,
            SchemaMode::Single("test".to_string()),
        ];
        for mode in modes {
            let cloned = mode.clone();
            assert_eq!(mode, cloned);
        }
    }

    #[test]
    fn schema_mode_debug_format() {
        assert!(format!("{:?}", SchemaMode::List).contains("List"));
        assert!(format!("{:?}", SchemaMode::All).contains("All"));
        let debug = format!("{:?}", SchemaMode::Single("foo".to_string()));
        assert!(debug.contains("Single"));
        assert!(debug.contains("foo"));
    }

    // ── Exhaustive tests: SchemaOptions ───────────────────────────────────

    #[test]
    fn schema_options_debug_includes_mode_and_format() {
        let opts = SchemaOptions {
            mode: SchemaMode::List,
            format: OutputFormat::Json,
        };
        let debug = format!("{:?}", opts);
        assert!(debug.contains("mode"));
        assert!(debug.contains("format"));
    }

    #[test]
    fn schema_options_clone_independent() {
        let opts = SchemaOptions {
            mode: SchemaMode::Single("original".to_string()),
            format: OutputFormat::Json,
        };
        let mut cloned = opts.clone();
        // Modify cloned mode — original should be unaffected
        cloned.mode = SchemaMode::All;
        assert_eq!(opts.mode, SchemaMode::Single("original".to_string()));
        assert_eq!(cloned.mode, SchemaMode::All);
    }

    #[test]
    fn schema_options_with_each_format() {
        let formats = [OutputFormat::Json];
        for fmt in formats {
            let opts = SchemaOptions {
                mode: SchemaMode::List,
                format: fmt,
            };
            assert_eq!(opts.format, fmt);
        }
    }

    // ── Exhaustive tests: SchemaInfo ──────────────────────────────────────

    #[test]
    fn schema_info_all_fields_populated() {
        let info = SchemaInfo {
            name: "test-schema".to_string(),
            description: "A test schema".to_string(),
            version: "2.1".to_string(),
        };
        assert_eq!(info.name, "test-schema");
        assert_eq!(info.description, "A test schema");
        assert_eq!(info.version, "2.1");
    }

    #[test]
    fn schema_info_clone_is_independent() {
        let info = SchemaInfo {
            name: "orig".to_string(),
            description: "desc".to_string(),
            version: "1.0".to_string(),
        };
        let mut cloned = info.clone();
        cloned.name = "modified".to_string();
        assert_eq!(info.name, "orig");
        assert_eq!(cloned.name, "modified");
    }

    #[test]
    fn schema_info_serialization_all_fields_preserved() {
        let info = SchemaInfo {
            name: "my-response".to_string(),
            description: "Test description".to_string(),
            version: "3.0".to_string(),
        };
        let json = serde_json::to_value(&info).expect("serialize to Value");
        assert_eq!(json["name"], "my-response");
        assert_eq!(json["description"], "Test description");
        assert_eq!(json["version"], "3.0");
    }

    #[test]
    fn schema_info_deserialization_rejects_missing_fields() {
        // Missing description
        let json = serde_json::json!({"name": "x", "version": "1.0"});
        let result: Result<SchemaInfo, _> = serde_json::from_value(json);
        assert!(result.is_err(), "should fail without description");

        // Missing name
        let json = serde_json::json!({"description": "x", "version": "1.0"});
        let result: Result<SchemaInfo, _> = serde_json::from_value(json);
        assert!(result.is_err(), "should fail without name");

        // Missing version
        let json = serde_json::json!({"name": "x", "description": "x"});
        let result: Result<SchemaInfo, _> = serde_json::from_value(json);
        assert!(result.is_err(), "should fail without version");
    }

    #[test]
    fn schema_info_roundtrip_via_json_string() {
        let info = SchemaInfo {
            name: "round-trip".to_string(),
            description: "round trip desc".to_string(),
            version: "4.2".to_string(),
        };
        let serialized = serde_json::to_string(&info).expect("serialize");
        let deserialized: SchemaInfo = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(deserialized.name, info.name);
        assert_eq!(deserialized.description, info.description);
        assert_eq!(deserialized.version, info.version);
    }

    // ── Exhaustive tests: SchemaListOutput ────────────────────────────────

    #[test]
    fn schema_list_output_empty_schemas() {
        let output = SchemaListOutput {
            schemas: vec![],
            base_url: "scp://".to_string(),
        };
        let json = serde_json::to_value(&output).expect("serialize");
        assert!(json["schemas"].as_array().map_or(false, |a| a.is_empty()));
        assert_eq!(json["base_url"], "scp://");
    }

    #[test]
    fn schema_list_output_many_schemas() {
        let schemas: Vec<SchemaInfo> = (0..50)
            .map(|i| SchemaInfo {
                name: format!("schema-{i}"),
                description: format!("Schema {i}"),
                version: "1.0".to_string(),
            })
            .collect();
        let output = SchemaListOutput {
            schemas,
            base_url: "scp://".to_string(),
        };
        let json = serde_json::to_value(&output).expect("serialize");
        assert_eq!(json["schemas"].as_array().map_or(0, |a| a.len()), 50);
    }

    #[test]
    fn schema_list_output_base_url_custom() {
        let output = SchemaListOutput {
            schemas: vec![],
            base_url: "custom://host/path".to_string(),
        };
        let json = serde_json::to_value(&output).expect("serialize");
        assert_eq!(json["base_url"], "custom://host/path");
    }

    #[test]
    fn schema_list_output_deserialization() {
        let json = serde_json::json!({
            "schemas": [{"name": "a", "description": "b", "version": "1.0"}],
            "base_url": "scp://"
        });
        let output: SchemaListOutput = serde_json::from_value(json).expect("deserialize");
        assert_eq!(output.schemas.len(), 1);
        assert_eq!(output.schemas[0].name, "a");
        assert_eq!(output.base_url, "scp://");
    }

    // ── Exhaustive tests: AllSchemasOutput ────────────────────────────────

    #[test]
    fn all_schemas_output_empty() {
        let output = AllSchemasOutput {
            schemas: serde_json::json!({}),
        };
        let json = serde_json::to_value(&output).expect("serialize");
        assert!(json["schemas"].as_object().map_or(false, |o| o.is_empty()));
    }

    #[test]
    fn all_schemas_output_multiple_entries() {
        let schemas = serde_json::json!({
            "a": {"type": "object"},
            "b": {"type": "array"},
            "c": {"type": "string"}
        });
        let output = AllSchemasOutput { schemas };
        let json = serde_json::to_value(&output).expect("serialize");
        assert_eq!(json["schemas"].as_object().map_or(0, |o| o.len()), 3);
    }

    #[test]
    fn all_schemas_output_nested_json() {
        let nested = serde_json::json!({
            "deep": {
                "nested": {
                    "value": 42,
                    "arr": [1, 2, 3]
                }
            }
        });
        let output = AllSchemasOutput { schemas: nested };
        let serialized = serde_json::to_string(&output).expect("serialize");
        let deserialized: AllSchemasOutput =
            serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(
            deserialized.schemas["deep"]["nested"]["value"],
            42
        );
    }

    #[test]
    fn all_schemas_output_deserialization() {
        let json = serde_json::json!({"schemas": {"x": {"$schema": "test"}}});
        let output: AllSchemasOutput = serde_json::from_value(json).expect("deserialize");
        assert!(output.schemas["x"]["$schema"].is_string());
    }

    // ── Exhaustive tests: available_schemas() ─────────────────────────────

    #[test]
    fn available_schemas_returns_10_entries() {
        let schemas = available_schemas();
        assert_eq!(schemas.len(), 10, "expected exactly 10 schemas");
    }

    #[test]
    fn available_schemas_all_names_are_lowercase_kebab() {
        for schema in available_schemas() {
            assert!(
                schema.name.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
                "name '{}' should be lowercase kebab-case",
                schema.name
            );
        }
    }

    #[test]
    fn available_schemas_all_names_end_with_response() {
        for schema in available_schemas() {
            assert!(
                schema.name.ends_with("-response"),
                "name '{}' should end with '-response'",
                schema.name
            );
        }
    }

    #[test]
    fn available_schemas_no_duplicate_names() {
        let schemas = available_schemas();
        let names: Vec<&str> = schemas.iter().map(|s| s.name.as_str()).collect();
        let unique: std::collections::HashSet<&str> = names.iter().copied().collect();
        assert_eq!(names.len(), unique.len(), "no duplicate schema names");
    }

    #[test]
    fn available_schemas_versions_all_1_0() {
        for schema in available_schemas() {
            assert_eq!(
                schema.version, "1.0",
                "schema '{}' version should be '1.0'",
                schema.name
            );
        }
    }

    #[test]
    fn available_schemas_descriptions_are_non_trivial() {
        for schema in available_schemas() {
            assert!(
                schema.description.len() > 10,
                "schema '{}' description too short: '{}'",
                schema.name,
                schema.description
            );
        }
    }

    #[test]
    fn available_schemas_stable_ordering() {
        // Call twice and verify same order
        let first: Vec<String> = available_schemas().iter().map(|s| s.name.clone()).collect();
        let second: Vec<String> = available_schemas().iter().map(|s| s.name.clone()).collect();
        assert_eq!(first, second, "schema ordering must be deterministic");
    }

    #[test]
    fn available_schemas_complete_coverage() {
        let expected = [
            "add-response",
            "remove-response",
            "list-response",
            "status-response",
            "sync-response",
            "context-response",
            "done-response",
            "spawn-response",
            "revert-response",
            "error-response",
        ];
        let schemas = available_schemas();
        let names: std::collections::HashSet<&str> =
            schemas.iter().map(|s| s.name.as_str()).collect();
        for name in expected {
            assert!(names.contains(name), "missing schema '{name}'");
        }
    }

    // ── Property-based tests ──────────────────────────────────────────────

    #[test]
    fn schema_info_serialization_idempotent_over_many_roundtrips() {
        let info = SchemaInfo {
            name: "stress-test".to_string(),
            description: "Testing multiple roundtrips".to_string(),
            version: "1.0".to_string(),
        };
        let mut current = serde_json::to_string(&info).expect("serialize");
        for _ in 0..10 {
            let deserialized: SchemaInfo = serde_json::from_str(&current).expect("deserialize");
            current = serde_json::to_string(&deserialized).expect("re-serialize");
        }
        let final_info: SchemaInfo = serde_json::from_str(&current).expect("final deserialize");
        assert_eq!(final_info.name, info.name);
        assert_eq!(final_info.description, info.description);
        assert_eq!(final_info.version, info.version);
    }

    #[test]
    fn schema_mode_single_with_empty_string() {
        let mode = SchemaMode::Single(String::new());
        assert_eq!(mode, SchemaMode::Single(String::new()));
        let opts = SchemaOptions {
            mode: mode.clone(),
            format: OutputFormat::Json,
        };
        assert_eq!(opts.mode, SchemaMode::Single(String::new()));
    }

    #[test]
    fn schema_mode_single_with_special_characters() {
        let name = "test/with/slashes&and&ampersands";
        let mode = SchemaMode::Single(name.to_string());
        assert_eq!(mode, SchemaMode::Single(name.to_string()));
    }

    #[test]
    fn schema_mode_single_with_unicode() {
        let name = "schema-日本語-テスト";
        let mode = SchemaMode::Single(name.to_string());
        assert_eq!(mode, SchemaMode::Single(name.to_string()));
    }

    #[test]
    fn schema_list_output_preserves_schema_ordering() {
        let schemas: Vec<SchemaInfo> = ["c", "b", "a"]
            .iter()
            .map(|&name| SchemaInfo {
                name: name.to_string(),
                description: format!("Schema {name}"),
                version: "1.0".to_string(),
            })
            .collect();
        let output = SchemaListOutput {
            schemas,
            base_url: "scp://".to_string(),
        };
        let json = serde_json::to_value(&output).expect("serialize");
        let arr = json["schemas"].as_array().expect("array");
        assert_eq!(arr[0]["name"], "c");
        assert_eq!(arr[1]["name"], "b");
        assert_eq!(arr[2]["name"], "a");
    }

    #[test]
    fn schema_info_debug_format() {
        let info = SchemaInfo {
            name: "debug-test".to_string(),
            description: "debug desc".to_string(),
            version: "1.0".to_string(),
        };
        let debug = format!("{info:?}");
        assert!(debug.contains("debug-test"));
        assert!(debug.contains("debug desc"));
        assert!(debug.contains("1.0"));
    }

    #[test]
    fn schema_list_output_debug_format() {
        let output = SchemaListOutput {
            schemas: vec![],
            base_url: "scp://".to_string(),
        };
        let debug = format!("{output:?}");
        assert!(debug.contains("schemas"));
        assert!(debug.contains("base_url"));
    }

    #[test]
    fn all_schemas_output_debug_format() {
        let output = AllSchemasOutput {
            schemas: serde_json::json!({}),
        };
        let debug = format!("{output:?}");
        assert!(debug.contains("schemas"));
    }
}
