//! Action functions for the schema command handler (Tier 3).
//!
//! I/O operations that display JSON Schema definitions for AI agents.

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]
#![warn(clippy::pedantic)]

use serde_json::json;
use scp_core::output::Output;
use scp_core::{Error, OutputFormat, Result};

use super::data::{available_schemas, SchemaOptions};

/// Execute the schema command with the given options.
///
/// Delegates to the appropriate sub-command based on `SchemaOptions` flags:
/// - `list` -> list all available schemas
/// - `all` -> dump all schema definitions
/// - `schema_name` -> show a single schema definition
/// - default -> list (same as `list`)
///
/// # Errors
///
/// Returns `Error::not_found` if a requested schema name does not exist.
/// Returns `Error::io_error` if JSON serialization fails.
pub fn run_schema(options: &SchemaOptions) -> Result<()> {
    if options.list {
        return run_list();
    }

    if options.all {
        return run_all();
    }

    match &options.schema_name {
        Some(name) => run_single(name),
        None => run_list(),
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

/// Dump all schema definitions.
fn run_all() -> Result<()> {
    let schemas = serde_json::json!({
        "add-response": build_add_response_schema(),
        "remove-response": build_remove_response_schema(),
        "list-response": build_list_response_schema(),
        "status-response": build_status_response_schema(),
        "sync-response": build_sync_response_schema(),
        "context-response": build_context_response_schema(),
        "done-response": build_done_response_schema(),
        "spawn-response": build_spawn_response_schema(),
        "revert-response": build_revert_response_schema(),
        "error-response": build_error_response_schema(),
    });

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

/// Resolve a schema name to its JSON Schema definition.
///
/// Returns `None` if the name is not recognized.
fn resolve_schema(name: &str) -> Option<serde_json::Value> {
    match name {
        "add-response" => Some(build_add_response_schema()),
        "remove-response" => Some(build_remove_response_schema()),
        "list-response" => Some(build_list_response_schema()),
        "status-response" => Some(build_status_response_schema()),
        "sync-response" => Some(build_sync_response_schema()),
        "context-response" => Some(build_context_response_schema()),
        "done-response" => Some(build_done_response_schema()),
        "spawn-response" => Some(build_spawn_response_schema()),
        "revert-response" => Some(build_revert_response_schema()),
        "error-response" => Some(build_error_response_schema()),
        _ => None,
    }
}

fn build_add_response_schema() -> serde_json::Value {
    serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "scp://add-response/v1",
        "title": "Add Response",
        "type": "object",
        "required": ["$schema", "_schema_version", "schema_type", "success", "data"],
        "properties": {
            "$schema": { "type": "string" },
            "_schema_version": { "type": "string", "const": "1.0" },
            "schema_type": { "type": "string", "const": "single" },
            "success": { "type": "boolean" },
            "data": {
                "type": "object",
                "required": ["name", "status", "workspace_path"],
                "properties": {
                    "name": { "type": "string", "description": "Session name" },
                    "status": { "type": "string", "enum": ["active", "creating", "failed"] },
                    "workspace_path": { "type": "string", "description": "Path to workspace" },
                    "branch": { "type": "string", "description": "Git branch name" },
                    "bead_id": { "type": ["string", "null"], "description": "Associated bead ID" }
                }
            }
        }
    })
}

fn build_remove_response_schema() -> serde_json::Value {
    serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "scp://remove-response/v1",
        "title": "Remove Response",
        "type": "object",
        "required": ["$schema", "_schema_version", "schema_type", "success", "data"],
        "properties": {
            "$schema": { "type": "string" },
            "_schema_version": { "type": "string", "const": "1.0" },
            "schema_type": { "type": "string", "const": "single" },
            "success": { "type": "boolean" },
            "data": {
                "type": "object",
                "required": ["name", "removed"],
                "properties": {
                    "name": { "type": "string" },
                    "removed": { "type": "boolean" },
                    "merged": { "type": "boolean" },
                    "workspace_deleted": { "type": "boolean" }
                }
            }
        }
    })
}

fn build_list_response_schema() -> serde_json::Value {
    serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "scp://list-response/v1",
        "title": "List Response",
        "type": "object",
        "required": ["$schema", "_schema_version", "schema_type", "success", "data"],
        "properties": {
            "$schema": { "type": "string" },
            "_schema_version": { "type": "string", "const": "1.0" },
            "schema_type": { "type": "string", "const": "list" },
            "success": { "type": "boolean" },
            "data": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["name", "status"],
                    "properties": {
                        "name": { "type": "string" },
                        "status": { "type": "string" },
                        "branch": { "type": "string" },
                        "bead_id": { "type": ["string", "null"] },
                        "created_at": { "type": "string", "format": "date-time" }
                    }
                }
            }
        }
    })
}

fn build_status_response_schema() -> serde_json::Value {
    serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "scp://status-response/v1",
        "title": "Status Response",
        "type": "object",
        "required": ["$schema", "_schema_version", "schema_type", "success", "data"],
        "properties": {
            "$schema": { "type": "string" },
            "_schema_version": { "type": "string", "const": "1.0" },
            "schema_type": { "type": "string", "const": "single" },
            "success": { "type": "boolean" },
            "data": {
                "type": "object",
                "required": ["name", "status"],
                "properties": {
                    "name": { "type": "string" },
                    "status": { "type": "string" },
                    "branch": { "type": "string" },
                    "workspace_path": { "type": "string" },
                    "last_synced": { "type": ["string", "null"], "format": "date-time" },
                    "changes": { "type": "integer" }
                }
            }
        }
    })
}

fn build_sync_response_schema() -> serde_json::Value {
    serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "scp://sync-response/v1",
        "title": "Sync Response",
        "type": "object",
        "required": ["$schema", "_schema_version", "schema_type", "success", "data"],
        "properties": {
            "$schema": { "type": "string" },
            "_schema_version": { "type": "string", "const": "1.0" },
            "schema_type": { "type": "string", "const": "single" },
            "success": { "type": "boolean" },
            "data": {
                "type": "object",
                "required": ["name", "synced"],
                "properties": {
                    "name": { "type": "string" },
                    "synced": { "type": "boolean" },
                    "conflicts": { "type": "boolean" },
                    "commits_rebased": { "type": "integer" }
                }
            }
        }
    })
}

fn build_context_response_schema() -> serde_json::Value {
    serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "scp://context-response/v1",
        "title": "Context Response",
        "type": "object",
        "required": ["$schema", "_schema_version", "schema_type", "success", "data"],
        "properties": {
            "$schema": { "type": "string" },
            "_schema_version": { "type": "string", "const": "1.0" },
            "schema_type": { "type": "string", "const": "single" },
            "success": { "type": "boolean" },
            "data": {
                "type": "object",
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
            }
        }
    })
}

fn build_done_response_schema() -> serde_json::Value {
    serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "scp://done-response/v1",
        "title": "Done Response",
        "type": "object",
        "required": ["$schema", "_schema_version", "schema_type", "success", "data"],
        "properties": {
            "$schema": { "type": "string" },
            "_schema_version": { "type": "string", "const": "1.0" },
            "schema_type": { "type": "string", "const": "single" },
            "success": { "type": "boolean" },
            "data": {
                "type": "object",
                "required": ["name", "completed"],
                "properties": {
                    "name": { "type": "string" },
                    "completed": { "type": "boolean" },
                    "merged": { "type": "boolean" },
                    "squashed": { "type": "boolean" },
                    "workspace_removed": { "type": "boolean" }
                }
            }
        }
    })
}

fn build_spawn_response_schema() -> serde_json::Value {
    serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "scp://spawn-response/v1",
        "title": "Spawn Response",
        "type": "object",
        "required": ["$schema", "_schema_version", "schema_type", "success", "data"],
        "properties": {
            "$schema": { "type": "string" },
            "_schema_version": { "type": "string", "const": "1.0" },
            "schema_type": { "type": "string", "const": "single" },
            "success": { "type": "boolean" },
            "data": {
                "type": "object",
                "required": ["name", "status", "workspace_path"],
                "properties": {
                    "name": { "type": "string", "description": "Workspace name" },
                    "status": { "type": "string", "enum": ["active", "creating", "failed"] },
                    "workspace_path": { "type": "string", "description": "Path to workspace" },
                    "branch": { "type": "string", "description": "Git branch name" },
                    "bead_id": { "type": ["string", "null"], "description": "Associated bead ID" }
                }
            }
        }
    })
}

fn build_revert_response_schema() -> serde_json::Value {
    serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "scp://revert-response/v1",
        "title": "Revert Response",
        "type": "object",
        "required": ["$schema", "_schema_version", "schema_type", "success", "data"],
        "properties": {
            "$schema": { "type": "string" },
            "_schema_version": { "type": "string", "const": "1.0" },
            "schema_type": { "type": "string", "const": "single" },
            "success": { "type": "boolean" },
            "data": {
                "type": "object",
                "required": ["name", "reverted"],
                "properties": {
                    "name": { "type": "string" },
                    "reverted": { "type": "boolean" },
                    "reset_to": { "type": "string", "description": "Commit hash reset to" },
                    "commits_removed": { "type": "integer" }
                }
            }
        }
    })
}

fn build_error_response_schema() -> serde_json::Value {
    serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "scp://error-response/v1",
        "title": "Error Response",
        "type": "object",
        "required": ["$schema", "_schema_version", "success", "error"],
        "properties": {
            "$schema": { "type": "string" },
            "_schema_version": { "type": "string", "const": "1.0" },
            "success": { "type": "boolean", "const": false },
            "error": {
                "type": "object",
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
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_schema_list_mode() {
        let options = SchemaOptions {
            schema_name: None,
            list: true,
            all: false,
            format: OutputFormat::Json,
        };
        assert!(run_schema(&options).is_ok());
    }

    #[test]
    fn run_schema_default_mode_is_list() {
        let options = SchemaOptions {
            schema_name: None,
            list: false,
            all: false,
            format: OutputFormat::Json,
        };
        assert!(run_schema(&options).is_ok());
    }

    #[test]
    fn run_schema_all_mode() {
        let options = SchemaOptions {
            schema_name: None,
            list: false,
            all: true,
            format: OutputFormat::Json,
        };
        assert!(run_schema(&options).is_ok());
    }

    #[test]
    fn run_schema_single_known() {
        let options = SchemaOptions {
            schema_name: Some("add-response".to_string()),
            list: false,
            all: false,
            format: OutputFormat::Json,
        };
        assert!(run_schema(&options).is_ok());
    }

    #[test]
    fn run_schema_single_unknown_returns_not_found() {
        let options = SchemaOptions {
            schema_name: Some("nonexistent".to_string()),
            list: false,
            all: false,
            format: OutputFormat::Json,
        };
        let result = run_schema(&options);
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
            assert!(
                resolve_schema(name).is_some(),
                "should resolve '{name}'"
            );
        }
    }

    #[test]
    fn resolve_unknown_schema_returns_none() {
        assert!(resolve_schema("does-not-exist").is_none());
    }

    #[test]
    fn all_schemas_have_json_schema_field() {
        let getters: Vec<(&str, fn() -> serde_json::Value)> = vec![
            ("add-response", build_add_response_schema),
            ("remove-response", build_remove_response_schema),
            ("list-response", build_list_response_schema),
            ("status-response", build_status_response_schema),
            ("sync-response", build_sync_response_schema),
            ("context-response", build_context_response_schema),
            ("done-response", build_done_response_schema),
            ("spawn-response", build_spawn_response_schema),
            ("revert-response", build_revert_response_schema),
            ("error-response", build_error_response_schema),
        ];

        for (name, getter) in getters {
            let schema = getter();
            assert!(
                schema.get("$schema").is_some(),
                "schema '{name}' must have $schema"
            );
            assert!(
                schema.get("type").is_some(),
                "schema '{name}' must have type"
            );
            assert!(
                schema.get("properties").is_some(),
                "object schema '{name}' must have properties"
            );
        }
    }

    #[test]
    fn required_fields_present_in_properties() {
        let schema = build_add_response_schema();
        let required = schema
            .get("required")
            .and_then(|r| r.as_array());
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
        let schema = build_error_response_schema();
        let props = schema.get("properties").and_then(|p| p.get("error"));
        assert!(props.is_some(), "must have error field in properties");

        let error_props = props
            .and_then(|e| e.get("properties"));
        assert!(error_props.is_some(), "error must have properties");

        let error_props = error_props;
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
        let schemas = [
            build_add_response_schema(),
            build_error_response_schema(),
            build_done_response_schema(),
        ];
        for schema in schemas {
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
        let options = SchemaOptions {
            schema_name: Some("revert-response".to_string()),
            list: false,
            all: false,
            format: OutputFormat::Json,
        };
        assert!(run_schema(&options).is_ok());
    }

    #[test]
    fn run_schema_done_response() {
        let options = SchemaOptions {
            schema_name: Some("done-response".to_string()),
            list: false,
            all: false,
            format: OutputFormat::Json,
        };
        assert!(run_schema(&options).is_ok());
    }
}
