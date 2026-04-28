//! JSON output module for isolate commands
//!
//! This module provides structured JSON output types and error handling
//! for all isolate CLI commands. It is organized into two submodules:
//!
//! - [`serializers`]: Command output structures (`InitOutput`, `AddOutput`, etc.)
//! - [`error`]: Error conversion and JSON error formatting

pub mod error;
pub mod serializers;

// Re-export commonly used types for convenience
#[allow(unused_imports)] // Keep for test module in serializers.rs
pub use error::SyncError;
pub use error::{output_json_error, output_json_parse_error, semantic_exit_code};
#[allow(unused_imports)] // Keep for test module in serializers.rs
pub use serializers::SyncOutput;
pub use serializers::{AddOutput, ConfigSetOutput, ConfigValueOutput, FocusOutput};
// Note: RemoveOutput removed - remove command uses JSONL output

/// Output a JSON success response to stdout
///
/// This is a convenience function for serializing any serializable type
/// to pretty-printed JSON and outputting it to stdout.
///
/// # Errors
///
/// Returns an error if serialization fails
#[allow(dead_code)]
#[allow(clippy::print_stdout)]
pub fn output_json_success<T: serde::Serialize>(data: &T) -> Result<(), anyhow::Error> {
    let json_str = serde_json::to_string_pretty(data)
        .map_err(|e| anyhow::anyhow!("Failed to serialize JSON: {e}"))?;
    println!("{json_str}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_json_success_basic() {
        let data = serde_json::json!({
            "message": "success",
            "value": 42
        });

        let result = super::output_json_success(&data);
        assert!(result.is_ok(), "output_json_success should succeed");
    }

    #[test]
    fn test_output_json_success_with_struct() {
        let output = AddOutput {
            name: "test".to_string(),
            workspace_path: "/path/to/workspace".to_string(),
            status: "active".to_string(),
            created: true,
        };

        let result = super::output_json_success(&output);
        assert!(
            result.is_ok(),
            "output_json_success should succeed with AddOutput"
        );
    }

    #[test]
    fn test_output_json_success_complex_nested() {
        let data = serde_json::json!({
            "level1": {
                "level2": {
                    "level3": "deep value"
                },
                "array": [1, 2, 3]
            }
        });

        let result = super::output_json_success(&data);
        assert!(
            result.is_ok(),
            "output_json_success should handle nested structures"
        );
    }
}
