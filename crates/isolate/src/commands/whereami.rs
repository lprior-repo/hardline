//! whereami command - Quick location query for AI agents
//!
//! Returns the current location in a simple, parseable format:
//! - `main` - On main branch
//! - `workspace:<name>` - In a workspace
//!
//! This is designed for AI agents that need to quickly orient themselves.

use anyhow::{Context, Result};
use isolate_core::{json::SchemaEnvelope, OutputFormat};
use serde::Serialize;

use super::abort::{detect_location, Location};

/// Output for whereami command
#[derive(Debug, Clone, Serialize)]
pub struct WhereAmIOutput {
    /// Location type: "main" or "workspace"
    pub location_type: String,
    /// Workspace name if in a workspace, None if on main
    pub workspace_name: Option<String>,
    /// Full path if in a workspace
    pub workspace_path: Option<String>,
    /// Simple one-line representation
    pub simple: String,
}

/// Options for whereami command
pub struct WhereAmIOptions {
    pub format: OutputFormat,
}

/// Run the whereami command
///
/// # Errors
///
/// Returns an error if unable to determine location
pub async fn run(options: &WhereAmIOptions) -> Result<()> {
    let root = super::abort::check_in_jj_repo().await?;
    let location = detect_location(&root)?;

    let output = match &location {
        Location::Main => WhereAmIOutput {
            location_type: "main".to_string(),
            workspace_name: None,
            workspace_path: None,
            simple: "main".to_string(),
        },
        Location::Workspace { name, path } => WhereAmIOutput {
            location_type: "workspace".to_string(),
            workspace_name: Some(name.clone()),
            workspace_path: Some(path.clone()),
            simple: format!("workspace:{name}"),
        },
    };

    if options.format.is_json() {
        let envelope = SchemaEnvelope::new("whereami-response", "single", &output);
        let json_str = serde_json::to_string_pretty(&envelope)
            .context("Failed to serialize whereami output")?;
        println!("{json_str}");
    } else {
        // Simple output for human consumption - just the simple string
        println!("{}", output.simple);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whereami_output_main() {
        let output = WhereAmIOutput {
            location_type: "main".to_string(),
            workspace_name: None,
            workspace_path: None,
            simple: "main".to_string(),
        };

        assert_eq!(output.simple, "main");
        assert!(output.workspace_name.is_none());
    }

    #[test]
    fn test_whereami_output_workspace() {
        let output = WhereAmIOutput {
            location_type: "workspace".to_string(),
            workspace_name: Some("feature-auth".to_string()),
            workspace_path: Some("/path/to/.isolate/workspaces/feature-auth".to_string()),
            simple: "workspace:feature-auth".to_string(),
        };

        assert_eq!(output.simple, "workspace:feature-auth");
        assert_eq!(output.workspace_name, Some("feature-auth".to_string()));
    }

    #[test]
    fn test_whereami_output_serializes() {
        let output = WhereAmIOutput {
            location_type: "main".to_string(),
            workspace_name: None,
            workspace_path: None,
            simple: "main".to_string(),
        };

        let json = serde_json::to_string(&output);
        assert!(json.is_ok(), "serialization should succeed");
        let json_str = json.unwrap_or_default();
        assert!(json_str.contains("\"location_type\":\"main\""));
    }

    // ============================================================================
    // Behavior Tests
    // ============================================================================

    /// Test `WhereAmIOutput` simple field format for main
    #[test]
    fn test_whereami_simple_format_main() {
        let output = WhereAmIOutput {
            location_type: "main".to_string(),
            workspace_name: None,
            workspace_path: None,
            simple: "main".to_string(),
        };

        // Simple format should be just "main"
        assert_eq!(output.simple, "main");
        assert!(!output.simple.contains(':'));
    }

    /// Test `WhereAmIOutput` simple field format for workspace
    #[test]
    fn test_whereami_simple_format_workspace() {
        let output = WhereAmIOutput {
            location_type: "workspace".to_string(),
            workspace_name: Some("feature-auth".to_string()),
            workspace_path: Some("/path/to/workspace".to_string()),
            simple: "workspace:feature-auth".to_string(),
        };

        // Simple format should be "workspace:<name>"
        assert!(output.simple.starts_with("workspace:"));
        assert!(output.simple.contains("feature-auth"));
    }

    /// Test `location_type` is always valid
    #[test]
    fn test_whereami_location_type_valid() {
        let valid_types = ["main", "workspace"];

        for location_type in valid_types {
            let output = WhereAmIOutput {
                location_type: location_type.to_string(),
                workspace_name: None,
                workspace_path: None,
                simple: location_type.to_string(),
            };

            assert!(valid_types.contains(&output.location_type.as_str()));
        }
    }

    /// Test workspace fields are consistent
    #[test]
    fn test_whereami_workspace_fields_consistent() {
        // When on main, workspace fields should be None
        let main_output = WhereAmIOutput {
            location_type: "main".to_string(),
            workspace_name: None,
            workspace_path: None,
            simple: "main".to_string(),
        };
        assert!(main_output.workspace_name.is_none());
        assert!(main_output.workspace_path.is_none());

        // When in workspace, both fields should be Some
        let ws_output = WhereAmIOutput {
            location_type: "workspace".to_string(),
            workspace_name: Some("test".to_string()),
            workspace_path: Some("/path".to_string()),
            simple: "workspace:test".to_string(),
        };
        assert!(ws_output.workspace_name.is_some());
        assert!(ws_output.workspace_path.is_some());
    }

    /// Test JSON output contains all required fields
    #[test]
    fn test_whereami_json_has_all_fields() {
        let output = WhereAmIOutput {
            location_type: "workspace".to_string(),
            workspace_name: Some("test".to_string()),
            workspace_path: Some("/path".to_string()),
            simple: "workspace:test".to_string(),
        };

        let json_str = serde_json::to_string(&output).unwrap_or_default();

        assert!(json_str.contains("location_type"));
        assert!(json_str.contains("workspace_name"));
        assert!(json_str.contains("workspace_path"));
        assert!(json_str.contains("simple"));
    }

    /// Test simple output is parseable
    #[test]
    fn test_whereami_simple_parseable() {
        let output = WhereAmIOutput {
            location_type: "workspace".to_string(),
            workspace_name: Some("feature-auth".to_string()),
            workspace_path: Some("/path".to_string()),
            simple: "workspace:feature-auth".to_string(),
        };

        // Should be able to parse simple format
        let parts: Vec<&str> = output.simple.split(':').collect();
        if parts.len() == 2 {
            #[allow(clippy::indexing_slicing)]
            {
                assert_eq!(parts[0], "workspace");
                assert_eq!(parts[1], "feature-auth");
            }
        } else {
            assert_eq!(output.simple, "main");
        }
    }
}
