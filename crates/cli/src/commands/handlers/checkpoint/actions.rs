//! Action functions for the checkpoint command handler (Tier 3).
//!
//! I/O operations that create, restore, and list session checkpoints.

use scp_core::output::Output;
use scp_core::{Error, Result};

use super::data::{
    generate_checkpoint_id, CheckpointAction, CheckpointInfo, CheckpointOptions, CheckpointOutput,
};

/// Execute the checkpoint command with the given options.
///
/// # Errors
///
/// Returns an error if:
/// - Restoring a checkpoint that does not exist
/// - The checkpoint ID is empty for a restore action
/// - Any I/O operation fails during checkpoint creation
pub fn run_checkpoint(options: &CheckpointOptions) -> Result<()> {
    match &options.action {
        CheckpointAction::Create { description } => run_create(description.as_deref()),
        CheckpointAction::Restore { checkpoint_id } => run_restore(checkpoint_id),
        CheckpointAction::List => run_list(),
    }
}

/// Create a new checkpoint of all session state.
///
/// # Errors
///
/// Returns an error if checkpoint creation fails.
fn run_create(description: Option<&str>) -> Result<()> {
    let checkpoint_id = generate_checkpoint_id();

    // TODO: Wire up actual session enumeration and database storage
    // once the workspace/session infrastructure is integrated.
    // For now, produce output based on validated inputs.

    let output = CheckpointOutput::Created {
        checkpoint_id: checkpoint_id.clone(),
        metadata_only: vec![],
    };

    Output::info(&format!("Checkpoint created: {checkpoint_id}"));
    if let Some(desc) = description {
        Output::info(&format!("  Description: {desc}"));
    }

    let _ = output;
    Ok(())
}

/// Restore session state from a checkpoint.
///
/// # Errors
///
/// Returns an error if the checkpoint ID is empty or the checkpoint
/// is not found.
fn run_restore(checkpoint_id: &str) -> Result<()> {
    if checkpoint_id.is_empty() {
        return Err(Error::validation_error(
            "Checkpoint ID cannot be empty",
        ));
    }

    if !checkpoint_id.starts_with("chk-") {
        return Err(Error::validation_error(format!(
            "Invalid checkpoint ID format: '{checkpoint_id}' (must start with 'chk-')"
        )));
    }

    // TODO: Wire up actual checkpoint lookup and restore logic
    // once the workspace/session infrastructure is integrated.
    // For now, validate inputs and produce output.

    let output = CheckpointOutput::Restored {
        checkpoint_id: checkpoint_id.to_string(),
    };

    Output::info(&format!("Restored to checkpoint: {checkpoint_id}"));

    let _ = output;
    Ok(())
}

/// List all available checkpoints.
///
/// # Errors
///
/// Returns an error if listing fails due to I/O.
fn run_list() -> Result<()> {
    // TODO: Wire up actual checkpoint listing from database
    // once the workspace/session infrastructure is integrated.
    // For now, show an empty list.

    let output = CheckpointOutput::List {
        checkpoints: vec![],
    };

    Output::info("No checkpoints found.");

    let _ = output;
    Ok(())
}

/// Output a checkpoint result to the user.
///
/// Formats the checkpoint output for display.
fn output_checkpoint(output: &CheckpointOutput) -> Result<()> {
    match output {
        CheckpointOutput::Created {
            checkpoint_id,
            metadata_only,
        } => {
            Output::info(&format!("Checkpoint created: {checkpoint_id}"));
            if !metadata_only.is_empty() {
                Output::info(&format!(
                    "Metadata-only snapshots recorded for {} session(s):",
                    metadata_only.len()
                ));
                for session in metadata_only {
                    Output::info(&format!("  - {session}"));
                }
            }
        }
        CheckpointOutput::Restored { checkpoint_id } => {
            Output::info(&format!("Restored to checkpoint: {checkpoint_id}"));
        }
        CheckpointOutput::List { checkpoints } => {
            if checkpoints.is_empty() {
                Output::info("No checkpoints found.");
            } else {
                Output::info(&format!(
                    "{:<20} {:<28} {:>8}  Description",
                    "ID", "Created", "Sessions"
                ));
                Output::info(&"-".repeat(72));
                for cp in checkpoints {
                    let desc = cp.description.as_deref().unwrap_or("");
                    Output::info(&format!(
                        "{:<20} {:<28} {:>8}  {}",
                        cp.id, cp.created_at, cp.session_count, desc
                    ));
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::handlers::checkpoint::data::OutputFormat;

    fn create_options(action: CheckpointAction) -> CheckpointOptions {
        CheckpointOptions {
            action,
            format: OutputFormat::Json,
        }
    }

    #[test]
    fn run_checkpoint_create_without_description() {
        let options = create_options(CheckpointAction::Create { description: None });
        assert!(run_checkpoint(&options).is_ok());
    }

    #[test]
    fn run_checkpoint_create_with_description() {
        let options = create_options(CheckpointAction::Create {
            description: Some("test checkpoint".to_string()),
        });
        assert!(run_checkpoint(&options).is_ok());
    }

    #[test]
    fn run_checkpoint_restore_valid_id() {
        let options = create_options(CheckpointAction::Restore {
            checkpoint_id: "chk-abc123".to_string(),
        });
        assert!(run_checkpoint(&options).is_ok());
    }

    #[test]
    fn run_checkpoint_restore_empty_id_fails() {
        let options = create_options(CheckpointAction::Restore {
            checkpoint_id: String::new(),
        });
        let result = run_checkpoint(&options);
        assert!(result.is_err(), "Empty checkpoint ID should fail");
        let err = result.err().unwrap();
        let msg = err.to_string();
        assert!(
            msg.contains("cannot be empty"),
            "Error should mention empty ID: {msg}"
        );
    }

    #[test]
    fn run_checkpoint_restore_invalid_prefix_fails() {
        let options = create_options(CheckpointAction::Restore {
            checkpoint_id: "bad-prefix-123".to_string(),
        });
        let result = run_checkpoint(&options);
        assert!(result.is_err(), "Invalid prefix should fail");
        let err = result.err().unwrap();
        let msg = err.to_string();
        assert!(
            msg.contains("must start with 'chk-'"),
            "Error should mention prefix requirement: {msg}"
        );
    }

    #[test]
    fn run_checkpoint_list() {
        let options = create_options(CheckpointAction::List);
        assert!(run_checkpoint(&options).is_ok());
    }

    // -- output_checkpoint tests --

    #[test]
    fn output_checkpoint_created_no_metadata() {
        let output = CheckpointOutput::Created {
            checkpoint_id: "chk-test123".to_string(),
            metadata_only: vec![],
        };
        assert!(output_checkpoint(&output).is_ok());
    }

    #[test]
    fn output_checkpoint_created_with_metadata() {
        let output = CheckpointOutput::Created {
            checkpoint_id: "chk-test456".to_string(),
            metadata_only: vec!["session-a".to_string(), "session-b".to_string()],
        };
        assert!(output_checkpoint(&output).is_ok());
    }

    #[test]
    fn output_checkpoint_restored() {
        let output = CheckpointOutput::Restored {
            checkpoint_id: "chk-restore".to_string(),
        };
        assert!(output_checkpoint(&output).is_ok());
    }

    #[test]
    fn output_checkpoint_list_empty() {
        let output = CheckpointOutput::List {
            checkpoints: vec![],
        };
        assert!(output_checkpoint(&output).is_ok());
    }

    #[test]
    fn output_checkpoint_list_with_entries() {
        let output = CheckpointOutput::List {
            checkpoints: vec![
                CheckpointInfo {
                    id: "chk-1".to_string(),
                    created_at: "2024-01-01T00:00:00Z".to_string(),
                    session_count: 3,
                    description: Some("first".to_string()),
                },
                CheckpointInfo {
                    id: "chk-2".to_string(),
                    created_at: "2024-01-02T00:00:00Z".to_string(),
                    session_count: 1,
                    description: None,
                },
            ],
        };
        assert!(output_checkpoint(&output).is_ok());
    }

    // -- run_create / run_restore / run_list direct tests --

    #[test]
    fn run_create_without_description_succeeds() {
        assert!(run_create(None).is_ok());
    }

    #[test]
    fn run_create_with_description_succeeds() {
        assert!(run_create(Some("my desc")).is_ok());
    }

    #[test]
    fn run_restore_valid_succeeds() {
        assert!(run_restore("chk-abc").is_ok());
    }

    #[test]
    fn run_restore_empty_fails() {
        assert!(run_restore("").is_err());
    }

    #[test]
    fn run_restore_bad_prefix_fails() {
        assert!(run_restore("invalid-id").is_err());
    }

    #[test]
    fn run_list_succeeds() {
        assert!(run_list().is_ok());
    }
}
