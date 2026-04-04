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
}
