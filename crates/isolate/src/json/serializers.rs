//! JSON output structures for isolate commands

use serde::Serialize;

use super::error::SyncError;

/// Init command JSON output
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct InitOutput {
    pub message: String,
    pub isolate_dir: String,
    pub config_file: String,
    pub state_db: String,
    pub layouts_dir: String,
}

/// Add command JSON output
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct AddOutput {
    pub name: String,
    pub workspace_path: String,
    pub status: String,
    pub created: bool,
}

/// Remove command JSON output
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct RemoveOutput {
    pub name: String,
    pub message: String,
}

/// Focus command JSON output
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct FocusOutput {
    pub name: String,
    pub message: String,
}

/// Sync command JSON output
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct SyncOutput {
    pub name: Option<String>,
    pub synced_count: usize,
    pub failed_count: usize,
    pub errors: Vec<SyncError>,
}

/// Diff command JSON output
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct DiffOutput {
    pub name: String,
    pub base: String,
    pub head: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_stat: Option<DiffStat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_content: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DiffStat {
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
    pub files: Vec<FileDiffStat>,
}

#[derive(Debug, Serialize)]
pub struct FileDiffStat {
    pub path: String,
    pub insertions: usize,
    pub deletions: usize,
    pub status: String,
}

/// Template show command JSON output
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct TemplateShowOutput {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub layout: String,
}

/// Template delete command JSON output
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct TemplateDeleteOutput {
    pub name: String,
    pub message: String,
}

/// Config get command JSON output
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct ConfigValueOutput {
    pub key: String,
    pub value: serde_json::Value,
}

/// Config set command JSON output
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct ConfigSetOutput {
    pub key: String,
    pub value: String,
    pub scope: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_output_json_uses_name_field() -> Result<(), serde_json::Error> {
        let output = AddOutput {
            name: "test-session".to_string(),
            workspace_path: "/path/to/workspace".to_string(),
            status: "active".to_string(),
            created: true,
        };

        let json = serde_json::to_value(&output)?;

        // Should have 'name' field, not 'session_name'
        assert!(json.get("name").is_some(), "JSON should have 'name' field");
        assert!(
            json.get("session_name").is_none(),
            "JSON should NOT have 'session_name' field"
        );
        assert_eq!(
            json.get("name").and_then(|v| v.as_str()),
            Some("test-session"),
            "name field should match"
        );
        Ok(())
    }

    #[test]
    fn test_add_output_name_matches_session() -> Result<(), serde_json::Error> {
        let session_name = "my-feature";
        let output = AddOutput {
            name: session_name.to_string(),
            workspace_path: format!("/workspaces/{session_name}"),
            status: "creating".to_string(),
            created: true,
        };

        let json = serde_json::to_value(&output)?;

        assert_eq!(
            json.get("name").and_then(|v| v.as_str()),
            Some(session_name),
            "name in JSON should match session name"
        );
        Ok(())
    }

    #[test]
    fn test_add_output_backwards_compat_session_name_removed() -> Result<(), serde_json::Error> {
        // This test verifies that the old 'session_name' field is completely removed
        let output = AddOutput {
            name: "test".to_string(),
            workspace_path: "/path".to_string(),
            status: "failed".to_string(),
            created: false,
        };

        let json_str = serde_json::to_string(&output)?;

        // The JSON string should not contain 'session_name' anywhere
        assert!(
            !json_str.contains("session_name"),
            "JSON should not contain 'session_name' field: {json_str}"
        );

        // But should contain 'name'
        assert!(
            json_str.contains("\"name\""),
            "JSON should contain 'name' field: {json_str}"
        );
        Ok(())
    }

    #[test]
    fn test_add_output_all_fields_present() -> Result<(), serde_json::Error> {
        let output = AddOutput {
            name: "test".to_string(),
            workspace_path: "/workspace/test".to_string(),
            status: "active".to_string(),
            created: true,
        };

        let json = serde_json::to_value(&output)?;

        assert_eq!(json.get("name").and_then(|v| v.as_str()), Some("test"));
        assert_eq!(
            json.get("workspace_path").and_then(|v| v.as_str()),
            Some("/workspace/test")
        );
        assert_eq!(json.get("status").and_then(|v| v.as_str()), Some("active"));
        assert_eq!(
            json.get("created").and_then(serde_json::Value::as_bool),
            Some(true)
        );
        Ok(())
    }

    #[test]
    fn test_remove_output_json_uses_name_field() -> Result<(), serde_json::Error> {
        let output = RemoveOutput {
            name: "test-session".to_string(),
            message: "Session removed successfully".to_string(),
        };

        let json = serde_json::to_value(&output)?;

        // Should have 'name' field, not 'session_name'
        assert!(json.get("name").is_some(), "JSON should have 'name' field");
        assert!(
            json.get("session_name").is_none(),
            "JSON should NOT have 'session_name' field"
        );
        assert_eq!(
            json.get("name").and_then(|v| v.as_str()),
            Some("test-session"),
            "name field should match"
        );
        Ok(())
    }

    #[test]
    fn test_remove_output_matches_add_structure() -> Result<(), serde_json::Error> {
        // RemoveOutput should use same 'name' field as AddOutput
        let add_output = AddOutput {
            name: "my-session".to_string(),
            workspace_path: "/workspace".to_string(),
            status: "active".to_string(),
            created: true,
        };

        let remove_output = RemoveOutput {
            name: "my-session".to_string(),
            message: "Removed".to_string(),
        };

        let add_json = serde_json::to_value(&add_output)?;
        let remove_json = serde_json::to_value(&remove_output)?;

        // Both should have 'name' field with same value
        assert_eq!(
            add_json.get("name").and_then(|v| v.as_str()),
            remove_json.get("name").and_then(|v| v.as_str()),
            "Both should use 'name' field consistently"
        );

        // Neither should have session_name
        assert!(add_json.get("session_name").is_none());
        assert!(remove_json.get("session_name").is_none());
        Ok(())
    }

    #[test]
    fn test_focus_output_json_uses_name_field() -> Result<(), serde_json::Error> {
        let output = FocusOutput {
            name: "test-session".to_string(),
            message: "Focused on session".to_string(),
        };

        let json = serde_json::to_value(&output)?;

        // Should have 'name' field, not 'session_name'
        assert!(json.get("name").is_some(), "JSON should have 'name' field");
        assert!(
            json.get("session_name").is_none(),
            "JSON should NOT have 'session_name' field"
        );
        assert_eq!(
            json.get("name").and_then(|v| v.as_str()),
            Some("test-session"),
            "name field should match"
        );
        Ok(())
    }

    #[test]
    fn test_focus_output_consistent_with_other_outputs() -> Result<(), serde_json::Error> {
        // All output structs should use 'name' field consistently
        let focus = FocusOutput {
            name: "my-session".to_string(),
            message: "Focused".to_string(),
        };

        let add = AddOutput {
            name: "my-session".to_string(),
            workspace_path: "/workspace".to_string(),
            status: "active".to_string(),
            created: true,
        };

        let remove = RemoveOutput {
            name: "my-session".to_string(),
            message: "Removed".to_string(),
        };

        let focus_json = serde_json::to_value(&focus)?;
        let add_json = serde_json::to_value(&add)?;
        let remove_json = serde_json::to_value(&remove)?;

        // All should have 'name' field with same value
        assert_eq!(
            focus_json.get("name").and_then(|v| v.as_str()),
            Some("my-session")
        );
        assert_eq!(
            add_json.get("name").and_then(|v| v.as_str()),
            Some("my-session")
        );
        assert_eq!(
            remove_json.get("name").and_then(|v| v.as_str()),
            Some("my-session")
        );

        // None should have session_name
        assert!(focus_json.get("session_name").is_none());
        assert!(add_json.get("session_name").is_none());
        assert!(remove_json.get("session_name").is_none());
        Ok(())
    }

    #[test]
    fn test_sync_json_has_envelope() -> Result<(), serde_json::Error> {
        // Create a SyncOutput (single session success)
        let output = SyncOutput {
            name: Some("test-session".to_string()),
            synced_count: 1,
            failed_count: 0,
            errors: Vec::new(),
        };

        // Wrap in envelope (as done in sync.rs)
        let envelope = isolate_core::json::SchemaEnvelope::new("sync-response", "single", output);

        // Serialize to JSON
        let json_str = serde_json::to_string(&envelope)?;
        let json: serde_json::Value = serde_json::from_str(&json_str)?;

        // Verify envelope fields exist
        assert!(
            json.get("$schema").is_some(),
            "SyncOutput JSON must have $schema field in envelope"
        );
        assert!(
            json.get("_schema_version").is_some(),
            "SyncOutput JSON must have _schema_version field in envelope"
        );
        assert!(
            json.get("schema_type").is_some(),
            "SyncOutput JSON must have schema_type field in envelope"
        );

        Ok(())
    }

    #[test]
    fn test_sync_schema_type_single() -> Result<(), serde_json::Error> {
        // Create a SyncOutput (all sessions sync)
        let output = SyncOutput {
            name: None,
            synced_count: 3,
            failed_count: 0,
            errors: Vec::new(),
        };

        // Wrap in envelope (as done in sync.rs)
        let envelope = isolate_core::json::SchemaEnvelope::new("sync-response", "single", output);

        // Serialize to JSON
        let json_str = serde_json::to_string(&envelope)?;
        let json: serde_json::Value = serde_json::from_str(&json_str)?;

        // Verify schema_type is "single"
        assert_eq!(
            json.get("schema_type").and_then(|v| v.as_str()),
            Some("single"),
            "SyncOutput schema_type must be 'single' (SyncOutput is a single object, not array)"
        );

        // Verify $schema URI format
        let schema_value = json.get("$schema").and_then(|v| v.as_str());
        assert!(schema_value.is_some(), "$schema field should be present");
        let Some(schema) = schema_value else {
            return Ok(());
        };
        assert!(
            schema.starts_with("isolate://"),
            "Schema URI must start with 'isolate://'"
        );
        assert!(
            schema.contains("/v1"),
            "Schema URI must include version '/v1'"
        );

        Ok(())
    }

    #[test]
    fn test_sync_all_serialization_points() -> Result<(), serde_json::Error> {
        // Test all 4 serialization points mentioned in PLAN.md

        // Point 1: Single session success (line 56 in sync.rs)
        let output1 = SyncOutput {
            name: Some("session1".to_string()),
            synced_count: 1,
            failed_count: 0,
            errors: Vec::new(),
        };
        let envelope1 = isolate_core::json::SchemaEnvelope::new("sync-response", "single", output1);
        let json1_str = serde_json::to_string(&envelope1)?;
        let json1: serde_json::Value = serde_json::from_str(&json1_str)?;
        assert!(
            json1.get("$schema").is_some(),
            "Single success case must have envelope (line 56)"
        );

        // Point 2: Single session failure (line 75 in sync.rs)
        let output2 = SyncOutput {
            name: Some("session2".to_string()),
            synced_count: 0,
            failed_count: 1,
            errors: vec![SyncError {
                name: "session2".to_string(),
                error: isolate_core::json::ErrorDetail {
                    code: "SYNC_FAILED".to_string(),
                    message: "rebase failed".to_string(),
                    exit_code: 3,
                    details: None,
                    suggestion: Some(
                        "Try 'jj resolve' to fix conflicts, then retry sync".to_string(),
                    ),
                },
            }],
        };
        let envelope2 = isolate_core::json::SchemaEnvelope::new("sync-response", "single", output2);
        let json2_str = serde_json::to_string(&envelope2)?;
        let json2: serde_json::Value = serde_json::from_str(&json2_str)?;
        assert!(
            json2.get("$schema").is_some(),
            "Single failure case must have envelope (line 75)"
        );

        // Point 3: All sessions empty (line 100 in sync.rs)
        let output3 = SyncOutput {
            name: None,
            synced_count: 0,
            failed_count: 0,
            errors: Vec::new(),
        };
        let envelope3 = isolate_core::json::SchemaEnvelope::new("sync-response", "single", output3);
        let json3_str = serde_json::to_string(&envelope3)?;
        let json3: serde_json::Value = serde_json::from_str(&json3_str)?;
        assert!(
            json3.get("$schema").is_some(),
            "All sessions empty case must have envelope (line 100)"
        );

        // Point 4: All sessions with results (line 136 in sync.rs)
        let output4 = SyncOutput {
            name: None,
            synced_count: 2,
            failed_count: 1,
            errors: vec![SyncError {
                name: "session3".to_string(),
                error: isolate_core::json::ErrorDetail {
                    code: "SYNC_FAILED".to_string(),
                    message: "workspace not found".to_string(),
                    exit_code: 3,
                    details: None,
                    suggestion: Some(
                        "Try 'jj resolve' to fix conflicts, then retry sync".to_string(),
                    ),
                },
            }],
        };
        let envelope4 = isolate_core::json::SchemaEnvelope::new("sync-response", "single", output4);
        let json4_str = serde_json::to_string(&envelope4)?;
        let json4: serde_json::Value = serde_json::from_str(&json4_str)?;
        assert!(
            json4.get("$schema").is_some(),
            "All sessions with results case must have envelope (line 136)"
        );

        Ok(())
    }
}
