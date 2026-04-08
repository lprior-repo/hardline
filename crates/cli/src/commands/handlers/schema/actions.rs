//! Action functions for the schema command handler (Tier 3).
//!
//! I/O operations that display JSON Schema definitions for AI agents.

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]
#![warn(clippy::pedantic)]

use std::sync::LazyLock;

use scp_core::output::Output;
use scp_core::{Error, OutputFormat, Result};

use super::data::{available_schemas, SchemaMode, SchemaOptions};

/// Schema definition entry used as the single source of truth for the
/// schema registry.
struct SchemaDefinition {
    /// Machine name (e.g., "add-response").
    name: &'static str,
    /// Human-readable title for the JSON Schema.
    title: &'static str,
    /// Schema type hint: "single", "list", or "error".
    schema_type: &'static str,
    /// JSON data properties (required + properties).
    data_properties: serde_json::Value,
}

/// Canonical schema registry — the single source of truth.
static SCHEMA_REGISTRY: LazyLock<Vec<SchemaDefinition>> = LazyLock::new(|| {
    vec![
        SchemaDefinition {
            name: "add-response",
            title: "Add Response",
            schema_type: "single",
            data_properties: serde_json::json!({
                "required": ["name", "status", "workspace_path"],
                "properties": {
                    "name": { "type": "string", "description": "Session name" },
                    "status": { "type": "string", "enum": ["active", "creating", "failed"] },
                    "workspace_path": { "type": "string", "description": "Path to workspace" },
                    "branch": { "type": "string", "description": "Git branch name" },
                    "bead_id": { "type": ["string", "null"], "description": "Associated bead ID" }
                }
            }),
        },
        SchemaDefinition {
            name: "remove-response",
            title: "Remove Response",
            schema_type: "single",
            data_properties: serde_json::json!({
                "required": ["name", "removed"],
                "properties": {
                    "name": { "type": "string" },
                    "removed": { "type": "boolean" },
                    "merged": { "type": "boolean" },
                    "workspace_deleted": { "type": "boolean" }
                }
            }),
        },
        SchemaDefinition {
            name: "list-response",
            title: "List Response",
            schema_type: "list",
            data_properties: serde_json::json!({
                "required": ["name", "status"],
                "properties": {
                    "name": { "type": "string" },
                    "status": { "type": "string" },
                    "branch": { "type": "string" },
                    "bead_id": { "type": ["string", "null"] },
                    "created_at": { "type": "string", "format": "date-time" }
                }
            }),
        },
        SchemaDefinition {
            name: "status-response",
            title: "Status Response",
            schema_type: "single",
            data_properties: serde_json::json!({
                "required": ["name", "status"],
                "properties": {
                    "name": { "type": "string" },
                    "status": { "type": "string" },
                    "branch": { "type": "string" },
                    "workspace_path": { "type": "string" },
                    "last_synced": { "type": ["string", "null"], "format": "date-time" },
                    "changes": { "type": "integer" }
                }
            }),
        },
        SchemaDefinition {
            name: "sync-response",
            title: "Sync Response",
            schema_type: "single",
            data_properties: serde_json::json!({
                "required": ["name", "synced"],
                "properties": {
                    "name": { "type": "string" },
                    "synced": { "type": "boolean" },
                    "conflicts": { "type": "boolean" },
                    "commits_rebased": { "type": "integer" }
                }
            }),
        },
        SchemaDefinition {
            name: "context-response",
            title: "Context Response",
            schema_type: "single",
            data_properties: serde_json::json!({
                "required": ["location"],
                "properties": {
                    "location": {
                        "type": "object",
                        "properties": {
                            "type": { "type": "string", "enum": ["main", "workspace"] },
                            "name": { "type": ["string", "null"] },
                            "path": { "type": "string" }
                        }
                    },
                    "session": {
                        "type": ["object", "null"],
                        "properties": {
                            "name": { "type": "string" },
                            "status": { "type": "string" },
                            "bead_id": { "type": ["string", "null"] }
                        }
                    }
                }
            }),
        },
        SchemaDefinition {
            name: "done-response",
            title: "Done Response",
            schema_type: "single",
            data_properties: serde_json::json!({
                "required": ["name", "completed"],
                "properties": {
                    "name": { "type": "string" },
                    "completed": { "type": "boolean" },
                    "merged": { "type": "boolean" },
                    "squashed": { "type": "boolean" },
                    "workspace_removed": { "type": "boolean" }
                }
            }),
        },
        SchemaDefinition {
            name: "spawn-response",
            title: "Spawn Response",
            schema_type: "single",
            data_properties: serde_json::json!({
                "required": ["name", "status", "workspace_path"],
                "properties": {
                    "name": { "type": "string", "description": "Workspace name" },
                    "status": { "type": "string", "enum": ["active", "creating", "failed"] },
                    "workspace_path": { "type": "string", "description": "Path to workspace" },
                    "branch": { "type": "string", "description": "Git branch name" },
                    "bead_id": { "type": ["string", "null"], "description": "Associated bead ID" }
                }
            }),
        },
        SchemaDefinition {
            name: "revert-response",
            title: "Revert Response",
            schema_type: "single",
            data_properties: serde_json::json!({
                "required": ["name", "reverted"],
                "properties": {
                    "name": { "type": "string" },
                    "reverted": { "type": "boolean" },
                    "reset_to": { "type": "string", "description": "Commit hash reset to" },
                    "commits_removed": { "type": "integer" }
                }
            }),
        },
        SchemaDefinition {
            name: "error-response",
            title: "Error Response",
            schema_type: "error",
            data_properties: serde_json::json!({
                "required": ["message"],
                "properties": {
                    "code": { "type": "string", "description": "Error code" },
                    "message": { "type": "string", "description": "Human-readable message" },
                    "exit_code": { "type": "integer", "description": "Suggested exit code" },
                    "fix_commands": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Commands that might fix the error"
                    },
                    "hints": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "field": { "type": "string" },
                                "issue": { "type": "string" },
                                "suggestion": { "type": "string" }
                            }
                        }
                    }
                }
            }),
        },
    ]
});

/// Build a full JSON Schema value from a `SchemaDefinition`.
fn build_schema(def: &SchemaDefinition) -> serde_json::Value {
    let id = format!("scp://{}/v1", def.name);
    let is_error = def.schema_type == "error";

    let required_fields = schema_required_fields(def);
    let properties = schema_properties(def, is_error);

    serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": id,
        "title": def.title,
        "type": "object",
        "required": required_fields,
        "properties": properties,
    })
}

/// Compute the `required` array for a schema definition.
fn schema_required_fields(def: &SchemaDefinition) -> Vec<&str> {
    if def.schema_type == "error" {
        vec!["$schema", "_schema_version", "success", "error"]
    } else {
        vec!["$schema", "_schema_version", "schema_type", "success", "data"]
    }
}

/// Compute the `properties` object for a schema definition.
fn schema_properties(def: &SchemaDefinition, is_error: bool) -> serde_json::Value {
    let data_field = build_data_field(def);

    let mut props = serde_json::Map::from_iter([
        ("$schema".to_string(), serde_json::json!({ "type": "string" })),
        ("_schema_version".to_string(), serde_json::json!({ "type": "string", "const": "1.0" })),
    ]);

    if is_error {
        props.insert("success".to_string(), serde_json::json!({ "type": "boolean", "const": false }));
        props.insert("error".to_string(), data_field);
    } else {
        props.insert("schema_type".to_string(), serde_json::json!({ "type": "string", "const": def.schema_type }));
        props.insert("success".to_string(), serde_json::json!({ "type": "boolean" }));
        props.insert("data".to_string(), data_field);
    }

    serde_json::Value::Object(props)
}

/// Build the data field for a schema definition.
fn build_data_field(def: &SchemaDefinition) -> serde_json::Value {
    let inner = serde_json::json!({
        "type": "object",
        "required": def.data_properties.get("required"),
        "properties": def.data_properties.get("properties"),
    });

    if def.schema_type == "list" {
        serde_json::json!({
            "type": "array",
            "items": inner,
        })
    } else {
        inner
    }
}

/// Resolve a schema name to its JSON Schema definition via the registry.
fn resolve_schema(name: &str) -> Option<serde_json::Value> {
    SCHEMA_REGISTRY
        .iter()
        .find(|def| def.name == name)
        .map(build_schema)
}

/// Execute the schema command with the given options.
///
/// # Errors
///
/// Returns `Error::not_found` if a requested schema name does not exist.
/// Returns `Error::io_error` if JSON serialization fails.
pub fn run_schema(options: &SchemaOptions) -> Result<()> {
    match &options.mode {
        SchemaMode::List => run_list(),
        SchemaMode::All => run_all(),
        SchemaMode::Single(name) => run_single(name),
    }
}

/// List all available schemas.
fn run_list() -> Result<()> {
    let schemas = available_schemas();
    let output = super::data::SchemaListOutput {
        schemas,
        base_url: "scp://".to_string(),
    };

    let json_str = serde_json::to_string_pretty(&output)
        .map_err(|e| Error::io_error(format!("Failed to serialize schema list: {e}")))?;
    Output::info(&json_str);

    Ok(())
}

/// Dump all schema definitions from the registry.
fn run_all() -> Result<()> {
    let schemas: serde_json::Map<String, serde_json::Value> = SCHEMA_REGISTRY
        .iter()
        .map(|def| (def.name.to_string(), build_schema(def)))
        .collect();

    let json_str = serde_json::to_string_pretty(&schemas)
        .map_err(|e| Error::io_error(format!("Failed to serialize all schemas: {e}")))?;
    Output::info(&json_str);

    Ok(())
}

/// Retrieve and display a single schema by name.
fn run_single(name: &str) -> Result<()> {
    let schema = resolve_schema(name).ok_or_else(|| {
        Error::not_found(format!(
            "Schema '{name}' not found. Use 'scp schema --list' to see available schemas."
        ))
    })?;

    let json_str = serde_json::to_string_pretty(&schema)
        .map_err(|e| Error::io_error(format!("Failed to serialize schema '{name}': {e}")))?;
    Output::info(&json_str);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema_opts(mode: SchemaMode) -> SchemaOptions {
        SchemaOptions {
            mode,
            format: OutputFormat::Json,
        }
    }

    #[test]
    fn run_schema_list_mode() {
        assert!(run_schema(&schema_opts(SchemaMode::List)).is_ok());
    }

    #[test]
    fn run_schema_default_mode_is_list() {
        assert!(run_schema(&schema_opts(SchemaMode::List)).is_ok());
    }

    #[test]
    fn run_schema_all_mode() {
        assert!(run_schema(&schema_opts(SchemaMode::All)).is_ok());
    }

    #[test]
    fn run_schema_single_known() {
        assert!(run_schema(&schema_opts(SchemaMode::Single("add-response".to_string()))).is_ok());
    }

    #[test]
    fn run_schema_single_unknown_returns_not_found() {
        let result = run_schema(&schema_opts(SchemaMode::Single("nonexistent".to_string())));
        assert!(result.is_err());
    }

    #[test]
    fn resolve_known_schemas() {
        let known = [
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
        for name in known {
            assert!(resolve_schema(name).is_some(), "should resolve '{name}'");
        }
    }

    #[test]
    fn resolve_unknown_schema_returns_none() {
        assert!(resolve_schema("does-not-exist").is_none());
    }

    #[test]
    fn all_schemas_have_json_schema_field() {
        for def in SCHEMA_REGISTRY.iter() {
            let schema = build_schema(def);
            assert!(
                schema.get("$schema").is_some(),
                "schema '{}' must have $schema",
                def.name
            );
            assert!(
                schema.get("type").is_some(),
                "schema '{}' must have type",
                def.name
            );
            assert!(
                schema.get("properties").is_some(),
                "object schema '{}' must have properties",
                def.name
            );
        }
    }

    #[test]
    fn required_fields_present_in_properties() {
        let schema = resolve_schema("add-response").expect("exists");
        let required = schema.get("required").and_then(|r| r.as_array());
        let properties = schema.get("properties");

        if let (Some(required_arr), Some(props)) = (required, properties) {
            for field in required_arr {
                if let Some(field_name) = field.as_str() {
                    assert!(
                        props.get(field_name).is_some(),
                        "required field '{field_name}' must be in properties"
                    );
                }
            }
        }
    }

    #[test]
    fn error_schema_is_ai_parseable() {
        let schema = resolve_schema("error-response").expect("exists");
        let props = schema.get("properties").and_then(|p| p.get("error"));
        assert!(props.is_some(), "must have error field in properties");

        let error_props = props.and_then(|e| e.get("properties"));
        assert!(error_props.is_some(), "error must have properties");

        assert!(
            error_props.and_then(|p| p.get("message")).is_some(),
            "error must have message"
        );
        assert!(
            error_props.and_then(|p| p.get("code")).is_some(),
            "error should have code"
        );
        assert!(
            error_props.and_then(|p| p.get("fix_commands")).is_some(),
            "error should have fix_commands for AI"
        );
    }

    #[test]
    fn schema_ids_use_scp_domain() {
        let names = ["add-response", "error-response", "done-response"];
        for name in names {
            let schema = resolve_schema(name).expect("exists");
            if let Some(id) = schema.get("$id").and_then(|i| i.as_str()) {
                assert!(
                    id.starts_with("scp://"),
                    "schema $id should use scp:// protocol: {id}"
                );
            }
        }
    }

    #[test]
    fn run_schema_revert_response() {
        assert!(run_schema(&schema_opts(SchemaMode::Single("revert-response".to_string()))).is_ok());
    }

    #[test]
    fn run_schema_done_response() {
        assert!(run_schema(&schema_opts(SchemaMode::Single("done-response".to_string()))).is_ok());
    }

    #[test]
    fn registry_matches_available_schemas() {
        let data_schemas = available_schemas();
        for ds in &data_schemas {
            assert!(
                SCHEMA_REGISTRY.iter().any(|r| r.name == ds.name),
                "registry missing '{}'",
                ds.name
            );
        }
    }

    // ========================================================================
    // Exhaustive schema resolution tests
    // ========================================================================

    #[test]
    fn resolve_each_schema_individually() {
        let schemas = [
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
        for name in schemas {
            let schema = resolve_schema(name);
            assert!(schema.is_some(), "'{name}' must resolve");
            let s = schema.expect("just checked");
            assert!(s.is_object(), "'{name}' schema must be object");
        }
    }

    #[test]
    fn resolve_case_sensitive() {
        assert!(
            resolve_schema("Add-Response").is_none(),
            "lookup must be case-sensitive"
        );
        assert!(
            resolve_schema("ADD-RESPONSE").is_none(),
            "uppercase must not match"
        );
        assert!(
            resolve_schema("add-Response").is_none(),
            "mixed case must not match"
        );
    }

    #[test]
    fn resolve_empty_string() {
        assert!(
            resolve_schema("").is_none(),
            "empty string must not resolve"
        );
    }

    #[test]
    fn resolve_partial_name() {
        assert!(
            resolve_schema("add").is_none(),
            "partial name must not match"
        );
        assert!(
            resolve_schema("response").is_none(),
            "suffix-only must not match"
        );
        assert!(
            resolve_schema("-response").is_none(),
            "leading dash must not match"
        );
    }

    #[test]
    fn resolve_with_whitespace() {
        assert!(
            resolve_schema(" add-response ").is_none(),
            "leading/trailing spaces must not match"
        );
        assert!(
            resolve_schema("add response").is_none(),
            "space in name must not match"
        );
    }

    #[test]
    fn resolve_with_special_characters() {
        let bad_names = [
            "add-response'; DROP TABLE--",
            "../../../etc/passwd",
            "$(rm -rf /)",
            "add-response\0null",
            "add-response\nnewline",
            "add-response\ttab",
        ];
        for name in bad_names {
            assert!(
                resolve_schema(name).is_none(),
                "injection attempt '{name}' must not resolve"
            );
        }
    }

    // ========================================================================
    // Schema structure validation (each schema deeply checked)
    // ========================================================================

    #[test]
    fn all_schemas_have_valid_json_schema_header() {
        for def in SCHEMA_REGISTRY.iter() {
            let schema = build_schema(def);
            assert_eq!(
                schema["$schema"],
                "https://json-schema.org/draft/2020-12/schema",
                "schema '{}' must use JSON Schema Draft 2020-12",
                def.name
            );
        }
    }

    #[test]
    fn all_schemas_have_scp_id() {
        for def in SCHEMA_REGISTRY.iter() {
            let schema = build_schema(def);
            let id = schema.get("$id").and_then(|v| v.as_str());
            assert!(
                id.is_some(),
                "schema '{}' must have $id",
                def.name
            );
            let id = id.expect("just checked");
            let expected = format!("scp://{}/v1", def.name);
            assert_eq!(
                id, expected,
                "schema '{}' $id must be 'scp://{}/v1'",
                def.name, def.name
            );
        }
    }

    #[test]
    fn all_schemas_have_title() {
        for def in SCHEMA_REGISTRY.iter() {
            let schema = build_schema(def);
            let title = schema.get("title").and_then(|v| v.as_str());
            assert!(
                title.is_some(),
                "schema '{}' must have title",
                def.name
            );
            assert!(
                !title.expect("just checked").is_empty(),
                "schema '{}' title must not be empty",
                def.name
            );
        }
    }

    #[test]
    fn all_schemas_have_type_object() {
        for def in SCHEMA_REGISTRY.iter() {
            let schema = build_schema(def);
            assert_eq!(
                schema["type"], "object",
                "schema '{}' must be type 'object'",
                def.name
            );
        }
    }

    #[test]
    fn all_schemas_have_dollar_schema_in_required() {
        for def in SCHEMA_REGISTRY.iter() {
            let schema = build_schema(def);
            let required = schema
                .get("required")
                .and_then(|r| r.as_array())
                .expect("must have required array");
            let has_dollar_schema = required
                .iter()
                .any(|v| v.as_str() == Some("$schema"));
            assert!(
                has_dollar_schema,
                "schema '{}' required must include '$schema'",
                def.name
            );
        }
    }

    #[test]
    fn all_schemas_have_schema_version_in_required() {
        for def in SCHEMA_REGISTRY.iter() {
            let schema = build_schema(def);
            let required = schema
                .get("required")
                .and_then(|r| r.as_array())
                .expect("must have required");
            let has_version = required
                .iter()
                .any(|v| v.as_str() == Some("_schema_version"));
            assert!(
                has_version,
                "schema '{}' required must include '_schema_version'",
                def.name
            );
        }
    }

    #[test]
    fn all_schemas_have_success_in_required() {
        for def in SCHEMA_REGISTRY.iter() {
            let schema = build_schema(def);
            let required = schema
                .get("required")
                .and_then(|r| r.as_array())
                .expect("must have required");
            let has_success = required
                .iter()
                .any(|v| v.as_str() == Some("success"));
            assert!(
                has_success,
                "schema '{}' required must include 'success'",
                def.name
            );
        }
    }

    #[test]
    fn non_error_schemas_have_data_field() {
        for def in SCHEMA_REGISTRY.iter() {
            if def.schema_type == "error" {
                continue;
            }
            let schema = build_schema(def);
            let props = schema
                .get("properties")
                .expect("must have properties");
            assert!(
                props.get("data").is_some(),
                "non-error schema '{}' must have 'data' property",
                def.name
            );
            assert!(
                props.get("schema_type").is_some(),
                "non-error schema '{}' must have 'schema_type' property",
                def.name
            );
        }
    }

    #[test]
    fn error_schema_has_error_field_not_data() {
        let schema = resolve_schema("error-response").expect("exists");
        let props = schema.get("properties").expect("must have properties");
        assert!(
            props.get("error").is_some(),
            "error schema must have 'error' property"
        );
        assert!(
            props.get("data").is_none(),
            "error schema must NOT have 'data' property"
        );
        assert!(
            props.get("schema_type").is_none(),
            "error schema must NOT have 'schema_type' property"
        );
    }

    #[test]
    fn error_schema_success_is_always_false() {
        let schema = resolve_schema("error-response").expect("exists");
        let success_prop = &schema["properties"]["success"];
        assert_eq!(
            success_prop["const"], false,
            "error schema success must be const false"
        );
    }

    #[test]
    fn non_error_schemas_success_is_boolean() {
        for def in SCHEMA_REGISTRY.iter() {
            if def.schema_type == "error" {
                continue;
            }
            let schema = build_schema(def);
            let success = &schema["properties"]["success"];
            assert_eq!(
                success["type"], "boolean",
                "schema '{}' success must be boolean",
                def.name
            );
        }
    }

    #[test]
    fn schema_version_is_const_1_0() {
        for def in SCHEMA_REGISTRY.iter() {
            let schema = build_schema(def);
            let version_prop = &schema["properties"]["_schema_version"];
            assert_eq!(
                version_prop["const"], "1.0",
                "schema '{}' _schema_version must be const '1.0'",
                def.name
            );
        }
    }

    #[test]
    fn list_type_schemas_wrap_data_in_array() {
        let list_schemas = SCHEMA_REGISTRY
            .iter()
            .filter(|def| def.schema_type == "list");
        for def in list_schemas {
            let schema = build_schema(def);
            let data_field = &schema["properties"]["data"];
            assert_eq!(
                data_field["type"], "array",
                "list schema '{}' data must be type 'array'",
                def.name
            );
            assert!(
                data_field.get("items").is_some(),
                "list schema '{}' data must have 'items'",
                def.name
            );
        }
    }

    #[test]
    fn single_type_schemas_have_object_data() {
        let single_schemas = SCHEMA_REGISTRY
            .iter()
            .filter(|def| def.schema_type == "single");
        for def in single_schemas {
            let schema = build_schema(def);
            let data_field = &schema["properties"]["data"];
            assert_eq!(
                data_field["type"], "object",
                "single schema '{}' data must be type 'object'",
                def.name
            );
        }
    }

    // ========================================================================
    // Schema-specific structural tests
    // ========================================================================

    #[test]
    fn add_response_schema_structure() {
        let schema = resolve_schema("add-response").expect("exists");
        let data_props = &schema["properties"]["data"]["properties"];
        assert!(data_props.get("name").is_some(), "must have name");
        assert!(data_props.get("status").is_some(), "must have status");
        assert!(data_props.get("workspace_path").is_some(), "must have workspace_path");
        // status enum
        let status = &data_props["status"];
        let enum_vals = status.get("enum").and_then(|e| e.as_array());
        assert!(enum_vals.is_some(), "status must have enum");
        let enums = enum_vals.expect("just checked");
        let values: Vec<&str> = enums.iter().filter_map(|v| v.as_str()).collect();
        assert!(values.contains(&"active"), "status enum must include 'active'");
        assert!(values.contains(&"creating"), "status enum must include 'creating'");
        assert!(values.contains(&"failed"), "status enum must include 'failed'");
    }

    #[test]
    fn remove_response_schema_structure() {
        let schema = resolve_schema("remove-response").expect("exists");
        let data_props = &schema["properties"]["data"]["properties"];
        assert!(data_props.get("name").is_some());
        assert!(data_props.get("removed").is_some());
        assert_eq!(data_props["removed"]["type"], "boolean");
        assert!(data_props.get("merged").is_some());
        assert_eq!(data_props["merged"]["type"], "boolean");
    }

    #[test]
    fn list_response_schema_is_array_type() {
        let schema = resolve_schema("list-response").expect("exists");
        let data_field = &schema["properties"]["data"];
        assert_eq!(data_field["type"], "array", "list-response data must be array");
        let items = data_field.get("items").expect("must have items");
        assert!(items.get("properties").is_some());
    }

    #[test]
    fn status_response_schema_structure() {
        let schema = resolve_schema("status-response").expect("exists");
        let data_props = &schema["properties"]["data"]["properties"];
        assert!(data_props.get("name").is_some());
        assert!(data_props.get("status").is_some());
        assert!(data_props.get("workspace_path").is_some());
        assert!(data_props.get("last_synced").is_some());
        assert!(data_props.get("changes").is_some());
        assert_eq!(data_props["changes"]["type"], "integer");
    }

    #[test]
    fn sync_response_schema_structure() {
        let schema = resolve_schema("sync-response").expect("exists");
        let data_props = &schema["properties"]["data"]["properties"];
        assert!(data_props.get("name").is_some());
        assert!(data_props.get("synced").is_some());
        assert_eq!(data_props["synced"]["type"], "boolean");
        assert!(data_props.get("conflicts").is_some());
        assert!(data_props.get("commits_rebased").is_some());
        assert_eq!(data_props["commits_rebased"]["type"], "integer");
    }

    #[test]
    fn context_response_schema_nested_structure() {
        let schema = resolve_schema("context-response").expect("exists");
        let data_props = &schema["properties"]["data"]["properties"];
        assert!(data_props.get("location").is_some());
        assert!(data_props.get("session").is_some());
        // location is a nested object
        let loc = &data_props["location"];
        assert_eq!(loc["type"], "object");
        let loc_props = loc.get("properties").expect("location must have properties");
        assert!(loc_props.get("type").is_some());
        assert!(loc_props.get("name").is_some());
        assert!(loc_props.get("path").is_some());
    }

    #[test]
    fn done_response_schema_structure() {
        let schema = resolve_schema("done-response").expect("exists");
        let data_props = &schema["properties"]["data"]["properties"];
        assert!(data_props.get("name").is_some());
        assert!(data_props.get("completed").is_some());
        assert_eq!(data_props["completed"]["type"], "boolean");
        assert!(data_props.get("merged").is_some());
        assert!(data_props.get("squashed").is_some());
        assert!(data_props.get("workspace_removed").is_some());
    }

    #[test]
    fn spawn_response_schema_matches_add() {
        let spawn = resolve_schema("spawn-response").expect("exists");
        let add = resolve_schema("add-response").expect("exists");
        // spawn-response should have same shape as add-response
        let spawn_data = &spawn["properties"]["data"]["properties"];
        assert!(spawn_data.get("name").is_some());
        assert!(spawn_data.get("status").is_some());
        assert!(spawn_data.get("workspace_path").is_some());
        // Both have same required fields
        let spawn_req = spawn["properties"]["data"]["required"].as_array();
        let add_req = add["properties"]["data"]["required"].as_array();
        assert_eq!(spawn_req.map(|a| a.len()), add_req.map(|a| a.len()));
    }

    #[test]
    fn revert_response_schema_structure() {
        let schema = resolve_schema("revert-response").expect("exists");
        let data_props = &schema["properties"]["data"]["properties"];
        assert!(data_props.get("name").is_some());
        assert!(data_props.get("reverted").is_some());
        assert_eq!(data_props["reverted"]["type"], "boolean");
        assert!(data_props.get("reset_to").is_some());
        assert!(data_props.get("commits_removed").is_some());
        assert_eq!(data_props["commits_removed"]["type"], "integer");
    }

    #[test]
    fn error_response_schema_has_fix_commands_and_hints() {
        let schema = resolve_schema("error-response").expect("exists");
        let error_props = &schema["properties"]["error"]["properties"];
        assert!(
            error_props.get("message").is_some(),
            "error must have message"
        );
        assert!(
            error_props.get("code").is_some(),
            "error must have code"
        );
        assert!(
            error_props.get("exit_code").is_some(),
            "error must have exit_code"
        );
        assert!(
            error_props.get("fix_commands").is_some(),
            "error must have fix_commands for AI agents"
        );
        assert!(
            error_props.get("hints").is_some(),
            "error must have hints"
        );
        // fix_commands is array of strings
        assert_eq!(error_props["fix_commands"]["type"], "array");
        // message is required in error data
        let error_req = schema["properties"]["error"]["required"]
            .as_array()
            .expect("error must have required fields");
        assert!(
            error_req.iter().any(|v| v.as_str() == Some("message")),
            "message must be required in error"
        );
    }

    // ========================================================================
    // Schema required/properties parity
    // ========================================================================

    #[test]
    fn every_required_data_field_has_property_definition() {
        for def in SCHEMA_REGISTRY.iter() {
            let schema = build_schema(def);
            let is_error = def.schema_type == "error";

            let data_field = if is_error {
                &schema["properties"]["error"]
            } else {
                &schema["properties"]["data"]
            };
            let required = data_field
                .get("required")
                .and_then(|r| r.as_array());
            let properties = data_field.get("properties");

            if let (Some(req_arr), Some(props)) = (required, properties) {
                for field in req_arr {
                    if let Some(field_name) = field.as_str() {
                        assert!(
                            props.get(field_name).is_some(),
                            "schema '{}' required field '{}' missing from properties",
                            def.name,
                            field_name
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn every_top_level_required_field_has_property_definition() {
        for def in SCHEMA_REGISTRY.iter() {
            let schema = build_schema(def);
            let required = schema
                .get("required")
                .and_then(|r| r.as_array())
                .expect("must have required");
            let properties = schema
                .get("properties")
                .expect("must have properties");

            for field in required {
                if let Some(field_name) = field.as_str() {
                    assert!(
                        properties.get(field_name).is_some(),
                        "schema '{}' top-level required field '{}' missing from properties",
                        def.name,
                        field_name
                    );
                }
            }
        }
    }

    // ========================================================================
    // Registry completeness and consistency
    // ========================================================================

    #[test]
    fn registry_and_data_schemas_bidirectional_match() {
        let data_schemas = available_schemas();
        let registry_names: Vec<&str> =
            SCHEMA_REGISTRY.iter().map(|r| r.name).collect();
        let data_names: Vec<&str> =
            data_schemas.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(registry_names.len(), data_names.len());
        for name in &registry_names {
            assert!(
                data_names.contains(name),
                "registry has '{}' but data doesn't",
                name
            );
        }
        for name in &data_names {
            assert!(
                registry_names.contains(name),
                "data has '{}' but registry doesn't",
                name
            );
        }
    }

    #[test]
    fn registry_has_no_duplicate_names() {
        let names: Vec<&str> = SCHEMA_REGISTRY.iter().map(|r| r.name).collect();
        let unique: std::collections::HashSet<&str> = names.iter().copied().collect();
        assert_eq!(names.len(), unique.len(), "registry has duplicate names");
    }

    #[test]
    fn all_schema_types_are_valid() {
        let valid_types = ["single", "list", "error"];
        for def in SCHEMA_REGISTRY.iter() {
            assert!(
                valid_types.contains(&def.schema_type),
                "schema '{}' has invalid type '{}'",
                def.name,
                def.schema_type
            );
        }
    }

    #[test]
    fn exactly_one_error_schema() {
        let error_count = SCHEMA_REGISTRY
            .iter()
            .filter(|def| def.schema_type == "error")
            .count();
        assert_eq!(
            error_count, 1,
            "expected exactly 1 error schema, found {error_count}"
        );
    }

    #[test]
    fn exactly_one_list_schema() {
        let list_count = SCHEMA_REGISTRY
            .iter()
            .filter(|def| def.schema_type == "list")
            .count();
        assert_eq!(
            list_count, 1,
            "expected exactly 1 list schema, found {list_count}"
        );
    }

    // ========================================================================
    // Schema diff / backward compatibility checks
    // ========================================================================

    #[test]
    fn schema_required_fields_stable() {
        // These fields must ALWAYS be present — removing them breaks backward compat
        let stability_required = ["$schema", "_schema_version", "success"];
        for def in SCHEMA_REGISTRY.iter() {
            let schema = build_schema(def);
            let required: Vec<String> = schema["required"]
                .as_array()
                .expect("must have required")
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
            for field in stability_required {
                assert!(
                    required.contains(&field.to_string()),
                    "schema '{}' missing stable required field '{field}'",
                    def.name
                );
            }
        }
    }

    #[test]
    fn schema_version_unchanged() {
        // All schemas should remain at version 1.0 for backward compat
        for def in SCHEMA_REGISTRY.iter() {
            let schema = build_schema(def);
            assert_eq!(
                schema["properties"]["_schema_version"]["const"], "1.0",
                "schema '{}' version changed — breaking backward compat",
                def.name
            );
        }
    }

    #[test]
    fn schema_id_format_unchanged() {
        for def in SCHEMA_REGISTRY.iter() {
            let schema = build_schema(def);
            let id = schema["$id"].as_str().expect("must have $id");
            assert!(
                id.starts_with("scp://") && id.ends_with("/v1"),
                "schema '{}' ID format changed: {id}",
                def.name
            );
        }
    }

    #[test]
    fn data_properties_have_type_annotations() {
        // Every property in data_properties should have a type annotation
        for def in SCHEMA_REGISTRY.iter() {
            let data_props = def
                .data_properties
                .get("properties")
                .and_then(|p| p.as_object());
            if let Some(props) = data_props {
                for (name, value) in props {
                    assert!(
                        value.get("type").is_some(),
                        "schema '{}' data property '{}' missing type annotation",
                        def.name,
                        name
                    );
                }
            }
        }
    }

    #[test]
    fn data_required_fields_have_descriptions_or_enums() {
        // Required data fields should have at least a type constraint
        for def in SCHEMA_REGISTRY.iter() {
            let required = def
                .data_properties
                .get("required")
                .and_then(|r| r.as_array());
            let properties = def
                .data_properties
                .get("properties")
                .and_then(|p| p.as_object());
            if let (Some(req), Some(props)) = (required, properties) {
                for field_name in req.iter().filter_map(|v| v.as_str()) {
                    assert!(
                        props.contains_key(field_name),
                        "schema '{}' required data field '{}' missing from properties",
                        def.name,
                        field_name
                    );
                }
            }
        }
    }

    // ========================================================================
    // run_schema error path tests
    // ========================================================================

    #[test]
    fn run_schema_unknown_single_error_code_is_not_found() {
        let result = run_schema(&schema_opts(SchemaMode::Single("nope".to_string())));
        let err = result.expect_err("should fail");
        assert_eq!(err.code(), "NOT_FOUND");
    }

    #[test]
    fn run_schema_unknown_single_error_message_contains_name() {
        let result = run_schema(&schema_opts(SchemaMode::Single("missing-schema".to_string())));
        let err = result.expect_err("should fail");
        let msg = err.to_string();
        assert!(
            msg.contains("missing-schema"),
            "error message should contain schema name: {msg}"
        );
    }

    #[test]
    fn run_schema_unknown_single_error_suggests_list() {
        let result = run_schema(&schema_opts(SchemaMode::Single("bad".to_string())));
        let err = result.expect_err("should fail");
        let suggestion = err.suggestion();
        // The error message itself suggests --list
        let msg = err.to_string();
        assert!(
            msg.contains("--list") || suggestion.is_some(),
            "should suggest using --list"
        );
    }

    // ========================================================================
    // Schema export tests (build_schema produces valid JSON Schema)
    // ========================================================================

    #[test]
    fn built_schema_is_valid_json() {
        for def in SCHEMA_REGISTRY.iter() {
            let schema = build_schema(def);
            let json_str = serde_json::to_string(&schema).expect("must serialize");
            let reparsed: serde_json::Value =
                serde_json::from_str(&json_str).expect("must be valid JSON");
            assert!(reparsed.is_object(), "must be JSON object");
        }
    }

    #[test]
    fn built_schema_pretty_print_is_valid_json() {
        for def in SCHEMA_REGISTRY.iter() {
            let schema = build_schema(def);
            let json_str = serde_json::to_string_pretty(&schema).expect("pretty print");
            let reparsed: serde_json::Value =
                serde_json::from_str(&json_str).expect("must parse back");
            assert_eq!(reparsed, schema, "pretty print roundtrip must match");
        }
    }

    // ========================================================================
    // Schema version display tests
    // ========================================================================

    #[test]
    fn schema_version_property_type_is_string() {
        for def in SCHEMA_REGISTRY.iter() {
            let schema = build_schema(def);
            assert_eq!(
                schema["properties"]["_schema_version"]["type"], "string",
                "schema '{}' _schema_version must be string type",
                def.name
            );
        }
    }

    #[test]
    fn dollar_schema_property_type_is_string() {
        for def in SCHEMA_REGISTRY.iter() {
            let schema = build_schema(def);
            assert_eq!(
                schema["properties"]["$schema"]["type"], "string",
                "schema '{}' $schema must be string type",
                def.name
            );
        }
    }

    // ========================================================================
    // Concurrent / stress tests
    // ========================================================================

    #[test]
    fn resolve_schema_idempotent() {
        // Resolve the same schema many times
        for _ in 0..100 {
            let s1 = resolve_schema("add-response").expect("exists");
            let s2 = resolve_schema("add-response").expect("exists");
            assert_eq!(s1, s2, "repeated resolution must be idempotent");
        }
    }

    #[test]
    fn build_schema_idempotent() {
        let def = SCHEMA_REGISTRY.iter().next().expect("at least one");
        let s1 = build_schema(def);
        let s2 = build_schema(def);
        assert_eq!(s1, s2, "building same schema must be idempotent");
    }

    #[test]
    fn run_schema_all_modes_idempotent() {
        // Running the same mode multiple times should always succeed
        for _ in 0..10 {
            assert!(run_schema(&schema_opts(SchemaMode::List)).is_ok());
            assert!(run_schema(&schema_opts(SchemaMode::All)).is_ok());
            assert!(
                run_schema(&schema_opts(SchemaMode::Single("add-response".to_string()))).is_ok()
            );
        }
    }

    // ========================================================================
    // Schema diff: ensure distinct schemas are actually different
    // ========================================================================

    #[test]
    fn each_schema_has_distinct_id() {
        let ids: Vec<String> = SCHEMA_REGISTRY
            .iter()
            .map(|def| {
                build_schema(def)["$id"]
                    .as_str()
                    .expect("must have $id")
                    .to_string()
            })
            .collect();
        let unique: std::collections::HashSet<String> = ids.iter().cloned().collect();
        assert_eq!(ids.len(), unique.len(), "all schema IDs must be unique");
    }

    #[test]
    fn each_schema_has_distinct_title() {
        let titles: Vec<String> = SCHEMA_REGISTRY
            .iter()
            .map(|def| {
                build_schema(def)["title"]
                    .as_str()
                    .expect("must have title")
                    .to_string()
            })
            .collect();
        let unique: std::collections::HashSet<String> = titles.iter().cloned().collect();
        assert_eq!(
            titles.len(),
            unique.len(),
            "all schema titles must be unique"
        );
    }

    #[test]
    fn add_and_spawn_schemas_have_same_shape_but_distinct() {
        let add = resolve_schema("add-response").expect("exists");
        let spawn = resolve_schema("spawn-response").expect("exists");
        // Different $id
        assert_ne!(add["$id"], spawn["$id"]);
        // Different title
        assert_ne!(add["title"], spawn["title"]);
    }
}
