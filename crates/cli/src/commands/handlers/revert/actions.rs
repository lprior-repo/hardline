//! Action functions for the revert command handler (Tier 3).
//!
//! I/O operations that orchestrate the session revert workflow.
//! Reverts a specific session's merge using `git reset --hard`.

use std::path::Path;

use scp_core::output::Output;
use scp_core::{Error, Result};

use super::data::{RevertOptions, RevertOutput, UndoEntry};
use super::executor::{GitExecutor, RealGitExecutor};

/// Path to the undo log relative to the project root.
const UNDO_LOG_PATH: &str = ".scp/undo.log";

// ============================================================================
// Public API
// ============================================================================

/// Execute the revert command with the given options.
///
/// This is the main entry point. It reads the undo history, finds the
/// target session entry, validates revert is possible, and executes
/// `git reset --hard` to revert to the pre-merge state.
///
/// # Errors
///
/// Returns errors for undo log read failures, session not found,
/// already-pushed-to-remote, or Git command failures.
pub fn run_revert(options: &RevertOptions) -> Result<RevertOutput> {
    let executor = RealGitExecutor::new();

    // Step 1: Read undo history
    let history = read_undo_history()?;

    // Step 2: Find target session entry
    let entry = find_session_entry(&history, &options.session_name)?;

    // Step 3: Validate revert is possible
    validate_revert_possible(&entry)?;

    // Step 4: Handle dry-run mode
    if options.dry_run {
        Output::info(&format!(
            "Dry-run: would revert session '{}'",
            options.session_name
        ));
        Output::info(&format!("  Commit to revert: {}", entry.commit_id));
        Output::info(&format!(
            "  Would reset to: {}",
            entry.pre_merge_commit_id
        ));

        return Ok(RevertOutput {
            session_name: options.session_name.clone(),
            dry_run: true,
            commit_id: entry.commit_id,
            pre_merge_commit_id: entry.pre_merge_commit_id,
            pushed_to_remote: false,
            error: None,
        });
    }

    // Step 5: Execute the revert via git reset --hard
    execute_revert(&entry, &executor)?;

    // Step 6: Update undo history to mark entry as reverted
    update_undo_history(&history, &entry)?;

    Output::success(&format!(
        "Reverted merge from session '{}'",
        options.session_name
    ));
    Output::info(&format!("  Reset to commit: {}", entry.pre_merge_commit_id));
    Output::info("NEXT: Verify changes and re-commit if needed:");
    Output::info("  git status");
    Output::info(&format!(
        "  git commit -m 'Revert: {}'",
        options.session_name
    ));

    Ok(RevertOutput {
        session_name: options.session_name.clone(),
        dry_run: false,
        commit_id: entry.commit_id,
        pre_merge_commit_id: entry.pre_merge_commit_id,
        pushed_to_remote: false,
        error: None,
    })
}

// ============================================================================
// Internal Helpers
// ============================================================================

/// Read undo history from `.scp/undo.log`.
///
/// Returns an empty vector if the file does not exist.
fn read_undo_history() -> Result<Vec<UndoEntry>> {
    let undo_log_path = Path::new(UNDO_LOG_PATH);

    if !undo_log_path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(undo_log_path)
        .map_err(|e| Error::io_error(format!("Failed to read undo log: {e}")))?;

    let entries = content
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(idx, line)| {
            serde_json::from_str::<UndoEntry>(line).map_err(|e| {
                Error::io_error(format!("Failed to parse undo log entry at line {}: {e}", idx + 1))
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(entries)
}

/// Find a specific session entry in the undo history.
///
/// Looks for an entry matching the session name with status "completed".
fn find_session_entry(history: &[UndoEntry], session_name: &str) -> Result<UndoEntry> {
    history
        .iter()
        .find(|entry| entry.session_name == session_name && entry.status == "completed")
        .cloned()
        .ok_or_else(|| {
            Error::not_found(format!(
                "Session '{session_name}' not found in undo history"
            ))
        })
}

/// Validate that revert is possible.
///
/// Returns an error if the commit has already been pushed to remote.
fn validate_revert_possible(entry: &UndoEntry) -> Result<()> {
    if entry.pushed_to_remote {
        return Err(Error::invalid_state(format!(
            "Cannot revert: commit {} has already been pushed to remote",
            entry.commit_id
        )));
    }

    Ok(())
}

/// Execute the revert by running `git reset --hard` to the pre-merge commit.
fn execute_revert(entry: &UndoEntry, executor: &dyn GitExecutor) -> Result<()> {
    executor
        .run(&["reset", "--hard", &entry.pre_merge_commit_id])
        .map_err(Error::from)?;

    Ok(())
}

/// Update undo history after a successful revert.
///
/// Marks the matching entry's status as "reverted".
fn update_undo_history(history: &[UndoEntry], entry: &UndoEntry) -> Result<()> {
    let undo_log_path = Path::new(UNDO_LOG_PATH);

    let new_content = history
        .iter()
        .map(|hist_entry| {
            if hist_entry.session_name == entry.session_name {
                let mut updated = hist_entry.clone();
                updated.status = "reverted".to_string();
                serde_json::to_string(&updated)
            } else {
                serde_json::to_string(hist_entry)
            }
        })
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| Error::io_error(format!("Failed to serialize undo entry: {e}")))?
        .join("\n")
        + "\n";

    std::fs::write(undo_log_path, &new_content)
        .map_err(|e| Error::io_error(format!("Failed to write undo log: {e}")))?;

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::handlers::done::executor::ExecutorError;

    // ---- Mock executor for testing ----

    struct MockGitExecutor {
        responses: std::collections::HashMap<String, std::result::Result<String, ExecutorError>>,
    }

    impl MockGitExecutor {
        fn new() -> Self {
            Self {
                responses: std::collections::HashMap::new(),
            }
        }

        fn with_response(mut self, key: &str, response: &str) -> Self {
            self.responses.insert(key.to_string(), Ok(response.to_string()));
            self
        }

        fn with_error(mut self, key: &str, err: ExecutorError) -> Self {
            self.responses.insert(key.to_string(), Err(err));
            self
        }
    }

    impl GitExecutor for MockGitExecutor {
        fn run(&self, args: &[&str]) -> std::result::Result<String, ExecutorError> {
            let key = args.join(" ");
            match self.responses.get(&key) {
                Some(result) => result.clone(),
                None => Err(ExecutorError::CommandFailed {
                    code: 1,
                    stderr: format!("no mock for: {key}"),
                }),
            }
        }

        fn run_in_workspace(
            &self,
            args: &[&str],
            _workspace_path: &str,
        ) -> std::result::Result<String, ExecutorError> {
            self.run(args)
        }
    }

    // ---- find_session_entry tests ----

    #[test]
    fn find_session_entry_finds_completed_match() {
        let history = vec![UndoEntry {
            session_name: "feature-x".to_string(),
            commit_id: "abc123".to_string(),
            pre_merge_commit_id: "def456".to_string(),
            timestamp: 100,
            pushed_to_remote: false,
            status: "completed".to_string(),
        }];

        let result = find_session_entry(&history, "feature-x");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().commit_id, "abc123");
    }

    #[test]
    fn find_session_entry_skips_already_reverted_entry() {
        let history = vec![UndoEntry {
            session_name: "feature-x".to_string(),
            commit_id: "abc123".to_string(),
            pre_merge_commit_id: "def456".to_string(),
            timestamp: 100,
            pushed_to_remote: false,
            status: "reverted".to_string(),
        }];

        let result = find_session_entry(&history, "feature-x");
        assert!(result.is_err());
    }

    #[test]
    fn find_session_entry_returns_error_for_missing_session() {
        let history: Vec<UndoEntry> = vec![];
        let result = find_session_entry(&history, "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn find_session_entry_skips_non_completed_entries() {
        let history = vec![UndoEntry {
            session_name: "feature-x".to_string(),
            commit_id: "abc123".to_string(),
            pre_merge_commit_id: "def456".to_string(),
            timestamp: 100,
            pushed_to_remote: false,
            status: "pending".to_string(),
        }];

        let result = find_session_entry(&history, "feature-x");
        assert!(result.is_err());
    }

    #[test]
    fn find_session_entry_picks_first_match() {
        let history = vec![
            UndoEntry {
                session_name: "feature-x".to_string(),
                commit_id: "first".to_string(),
                pre_merge_commit_id: "base1".to_string(),
                timestamp: 100,
                pushed_to_remote: false,
                status: "completed".to_string(),
            },
            UndoEntry {
                session_name: "feature-x".to_string(),
                commit_id: "second".to_string(),
                pre_merge_commit_id: "base2".to_string(),
                timestamp: 200,
                pushed_to_remote: false,
                status: "completed".to_string(),
            },
        ];

        let result = find_session_entry(&history, "feature-x");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().commit_id, "first");
    }

    // ---- validate_revert_possible tests ----

    #[test]
    fn validate_revert_possible_succeeds_when_not_pushed() {
        let entry = UndoEntry {
            session_name: "test".to_string(),
            commit_id: "abc".to_string(),
            pre_merge_commit_id: "def".to_string(),
            timestamp: 100,
            pushed_to_remote: false,
            status: "completed".to_string(),
        };

        assert!(validate_revert_possible(&entry).is_ok());
    }

    #[test]
    fn validate_revert_possible_fails_when_pushed() {
        let entry = UndoEntry {
            session_name: "test".to_string(),
            commit_id: "abc".to_string(),
            pre_merge_commit_id: "def".to_string(),
            timestamp: 100,
            pushed_to_remote: true,
            status: "completed".to_string(),
        };

        let result = validate_revert_possible(&entry);
        assert!(result.is_err());
    }

    // ---- execute_revert tests ----

    #[test]
    fn execute_revert_calls_git_reset_hard() {
        let entry = UndoEntry {
            session_name: "test".to_string(),
            commit_id: "abc".to_string(),
            pre_merge_commit_id: "def456".to_string(),
            timestamp: 100,
            pushed_to_remote: false,
            status: "completed".to_string(),
        };

        let mock =
            MockGitExecutor::new().with_response("reset --hard def456", "HEAD is now at def456");
        let result = execute_revert(&entry, &mock);
        assert!(result.is_ok());
    }

    #[test]
    fn execute_revert_propagates_git_error() {
        let entry = UndoEntry {
            session_name: "test".to_string(),
            commit_id: "abc".to_string(),
            pre_merge_commit_id: "def456".to_string(),
            timestamp: 100,
            pushed_to_remote: false,
            status: "completed".to_string(),
        };

        let mock = MockGitExecutor::new().with_error(
            "reset --hard def456",
            ExecutorError::CommandFailed {
                code: 128,
                stderr: "unknown revision".to_string(),
            },
        );
        let result = execute_revert(&entry, &mock);
        assert!(result.is_err());
    }

    // ---- update_undo_history (serialization logic) tests ----

    #[test]
    fn update_undo_history_marks_entry_as_reverted() {
        let history = vec![
            UndoEntry {
                session_name: "feature-x".to_string(),
                commit_id: "abc".to_string(),
                pre_merge_commit_id: "def".to_string(),
                timestamp: 100,
                pushed_to_remote: false,
                status: "completed".to_string(),
            },
            UndoEntry {
                session_name: "feature-y".to_string(),
                commit_id: "ghi".to_string(),
                pre_merge_commit_id: "jkl".to_string(),
                timestamp: 200,
                pushed_to_remote: false,
                status: "completed".to_string(),
            },
        ];

        let target = UndoEntry {
            session_name: "feature-x".to_string(),
            commit_id: "abc".to_string(),
            pre_merge_commit_id: "def".to_string(),
            timestamp: 100,
            pushed_to_remote: false,
            status: "completed".to_string(),
        };

        // Test the serialization logic directly (no disk I/O).
        let new_content = history
            .iter()
            .map(|hist_entry| {
                if hist_entry.session_name == target.session_name {
                    let mut updated = hist_entry.clone();
                    updated.status = "reverted".to_string();
                    serde_json::to_string(&updated)
                } else {
                    serde_json::to_string(hist_entry)
                }
            })
            .collect::<std::result::Result<Vec<_>, _>>()
            .expect("serialize")
            .join("\n");

        assert!(new_content.contains("\"status\":\"reverted\""));
        assert!(new_content.contains("feature-x"));
        assert!(new_content.contains("feature-y"));
    }

    // ---- RevertOutput tests ----

    #[test]
    fn revert_output_dry_run_construction() {
        let output = RevertOutput {
            session_name: "test-session".to_string(),
            dry_run: true,
            commit_id: "abc123".to_string(),
            pre_merge_commit_id: "def456".to_string(),
            pushed_to_remote: false,
            error: None,
        };
        assert_eq!(output.session_name, "test-session");
        assert!(output.dry_run);
        assert_eq!(output.commit_id, "abc123");
        assert_eq!(output.pre_merge_commit_id, "def456");
    }

    #[test]
    fn revert_output_serialization_roundtrip() {
        let output = RevertOutput {
            session_name: "test-ws".to_string(),
            dry_run: true,
            commit_id: "xyz789".to_string(),
            pre_merge_commit_id: "aaa111".to_string(),
            pushed_to_remote: false,
            error: None,
        };
        let json = serde_json::to_string(&output).expect("serialize");
        let deserialized: RevertOutput =
            serde_json::from_str(&json).expect("deserialize roundtrip");
        assert_eq!(deserialized.session_name, "test-ws");
        assert_eq!(deserialized.commit_id, "xyz789");
        assert_eq!(deserialized.pre_merge_commit_id, "aaa111");
        assert!(deserialized.dry_run);
    }
}
