//! Helper functions for JSON error construction and output

use serde::Serialize;

use super::error_code::ErrorCode;
use super::error_mapping::classify_exit_code;
use super::error_types::JsonError;
use crate::error::Error;

/// Output a JSON success response to stdout.
///
/// Serializes any `Serialize` type to pretty-printed JSON and outputs
/// it to stdout. Returns `Err` if serialization fails.
///
/// # Errors
///
/// Returns `Error` if serialization fails.
#[allow(clippy::print_stdout)]
pub fn output_json_success<T: Serialize>(data: &T) -> Result<(), Error> {
    let json_str = serde_json::to_string_pretty(data)
        .map_err(|e| Error::io_error(format!("Failed to serialize JSON: {e}")))?;
    println!("{json_str}");
    Ok(())
}

/// Output a CLI parse error as JSON and return clap-compatible exit code 2.
///
/// Used for argument parsing errors, producing a structured JSON error
/// with an `INVALID_ARGUMENT` code and a suggestion to use `--help`.
#[allow(clippy::print_stdout)]
pub fn output_json_parse_error(message: impl Into<String>) -> i32 {
    let error = JsonError::new(ErrorCode::InvalidArgument, message.into())
        .with_suggestion("Use --help to view valid flags and arguments")
        .with_exit_code(2);

    if let Ok(json_str) = serde_json::to_string_pretty(&error) {
        println!("{json_str}");
    } else {
        println!(
            r#"{{"success":false,"error":{{"message":"Failed to serialize parse error","exit_code":2}}}}"#
        );
    }

    2
}

/// Return the semantic exit code for an error.
///
/// This is shared by JSON and non-JSON output modes so both paths
/// return the same process status for the same failure.
///
/// Exit codes:
/// - 1: Usage/validation errors
/// - 2: Not found errors
/// - 3: System errors
/// - 4: External command errors
/// - 5: Lock contention errors
#[must_use]
pub fn semantic_exit_code(error: &Error) -> i32 {
    classify_exit_code(error)
}

/// Helper to create error details with available sessions
pub fn error_with_available_sessions(
    code: ErrorCode,
    message: impl Into<String>,
    session_name: impl Into<String>,
    available: &[String],
) -> JsonError {
    let details = serde_json::json!({
        "session_name": session_name.into(),
        "available_sessions": available,
    });

    JsonError::new(code, message)
        .with_details(details)
        .with_suggestion("Use 'scp session list' to see available sessions")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── output_json_success produces valid JSON ──────────────────────

    #[test]
    fn test_output_json_success_serializes_struct() {
        let data = serde_json::json!({
            "status": "ok",
            "value": 42,
        });
        let result = output_json_success(&data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_output_json_success_with_string() {
        let data = "hello world";
        let result = output_json_success(&data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_output_json_success_with_number() {
        let data = 42_i64;
        let result = output_json_success(&data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_output_json_success_with_nested_object() {
        let data = serde_json::json!({
            "users": [
                {"name": "alice", "id": 1},
                {"name": "bob", "id": 2},
            ],
            "total": 2,
        });
        let result = output_json_success(&data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_output_json_success_with_unit_struct() {
        #[derive(Serialize)]
        struct EmptyOutput {
            message: String,
        }
        let data = EmptyOutput {
            message: "done".to_string(),
        };
        let result = output_json_success(&data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_output_json_success_with_bool() {
        let data = true;
        let result = output_json_success(&data);
        assert!(result.is_ok());
    }

    // ── output_json_parse_error produces valid JSON with error info ──

    #[test]
    fn test_output_json_parse_error_returns_exit_code_2() {
        let code = output_json_parse_error("unknown flag --foo");
        assert_eq!(code, 2);
    }

    #[test]
    fn test_output_json_parse_error_message() {
        // We can't capture stdout easily in unit tests, but we verify the return value
        let code = output_json_parse_error("missing argument");
        assert_eq!(code, 2);
    }

    #[test]
    fn test_output_json_parse_error_with_various_messages() {
        assert_eq!(output_json_parse_error("test1"), 2);
        assert_eq!(output_json_parse_error(""), 2);
        assert_eq!(output_json_parse_error("very long error message with details"), 2);
    }

    // ── semantic_exit_code classifies correctly ───────────────────────

    #[test]
    fn test_semantic_exit_code_config() {
        let err = crate::error::Error::config_invalid("bad value");
        assert_eq!(semantic_exit_code(&err), 1);
    }

    #[test]
    fn test_semantic_exit_code_config_not_found() {
        let err = crate::error::Error::config_not_found("file.toml");
        assert_eq!(semantic_exit_code(&err), 1);
    }

    #[test]
    fn test_semantic_exit_code_session_not_found() {
        let err = crate::error::Error::session("missing-session");
        assert_eq!(semantic_exit_code(&err), 2);
    }

    #[test]
    fn test_semantic_exit_code_session_exists() {
        let err = crate::error::Error::session_exists("existing");
        assert_eq!(semantic_exit_code(&err), 2);
    }

    #[test]
    fn test_semantic_exit_code_workspace_not_found() {
        let err = crate::error::Error::workspace_not_found("missing");
        assert_eq!(semantic_exit_code(&err), 2);
    }

    #[test]
    fn test_semantic_exit_code_workspace_exists() {
        let err = crate::error::Error::workspace_exists("existing");
        assert_eq!(semantic_exit_code(&err), 2);
    }

    #[test]
    fn test_semantic_exit_code_vcs_branch_not_found() {
        let err = crate::error::Error::branch_not_found("missing-branch");
        assert_eq!(semantic_exit_code(&err), 2);
    }

    #[test]
    fn test_semantic_exit_code_vcs_commit_not_found() {
        let err = crate::error::Error::commit_not_found("abc123");
        assert_eq!(semantic_exit_code(&err), 2);
    }

    #[test]
    fn test_semantic_exit_code_vcs_not_initialized() {
        let err = crate::error::Error::vcs_not_initialized();
        assert_eq!(semantic_exit_code(&err), 1);
    }

    #[test]
    fn test_semantic_exit_code_vcs_working_copy_dirty() {
        let err = crate::error::Error::working_copy_dirty();
        assert_eq!(semantic_exit_code(&err), 1);
    }

    #[test]
    fn test_semantic_exit_code_state_validation() {
        let err = crate::error::Error::validation_error("bad input");
        assert_eq!(semantic_exit_code(&err), 1);
    }

    #[test]
    fn test_semantic_exit_code_state_invalid_identifier() {
        let err = crate::error::Error::invalid_identifier("bad-id!");
        assert_eq!(semantic_exit_code(&err), 1);
    }

    #[test]
    fn test_semantic_exit_code_state_not_found() {
        let err = crate::error::Error::not_found("resource");
        assert_eq!(semantic_exit_code(&err), 2);
    }

    #[test]
    fn test_semantic_exit_code_io() {
        let err = crate::error::Error::io_error("disk full");
        assert_eq!(semantic_exit_code(&err), 3);
    }

    #[test]
    fn test_semantic_exit_code_database() {
        let err = crate::error::Error::database("corrupt");
        assert_eq!(semantic_exit_code(&err), 3);
    }

    #[test]
    fn test_semantic_exit_code_agent() {
        let err = crate::error::Error::agent_not_found("agent-1");
        assert_eq!(semantic_exit_code(&err), 3);
    }

    #[test]
    fn test_semantic_exit_code_queue() {
        let err = crate::error::Error::queue_empty();
        assert_eq!(semantic_exit_code(&err), 3);
    }

    #[test]
    fn test_semantic_exit_code_task() {
        use crate::error_task::TaskErrorKind;
        let err: crate::error::Error = TaskErrorKind::NotFound("t".into()).into();
        assert_eq!(semantic_exit_code(&err), 3);
    }

    #[test]
    fn test_semantic_exit_code_wait() {
        let err = crate::error::Error::batch_empty();
        assert_eq!(semantic_exit_code(&err), 3);
    }

    #[test]
    fn test_semantic_exit_code_jj() {
        let err = crate::error::Error::jj_command_error("status", "failed", false);
        assert_eq!(semantic_exit_code(&err), 4);
    }

    #[test]
    fn test_semantic_exit_code_internal() {
        let err = crate::error::Error::internal("invariant violated");
        assert_eq!(semantic_exit_code(&err), 4);
    }

    #[test]
    fn test_semantic_exit_code_lock() {
        use crate::coordination::locks::errors::LockErrorKind;
        let err: crate::error::Error = LockErrorKind::SessionLocked {
            session: "s".into(),
            holder: "h".into(),
        }
        .into();
        assert_eq!(semantic_exit_code(&err), 5);
    }

    // ── error_with_available_sessions ─────────────────────────────────

    #[test]
    fn test_error_with_available_sessions() {
        let err = error_with_available_sessions(
            ErrorCode::SessionNotFound,
            "Session not found",
            "my-session",
            &["default".to_string(), "dev".to_string()],
        );

        assert_eq!(err.success, false);
        assert_eq!(err.error.code, "SESSION_NOT_FOUND");
        assert_eq!(err.error.message, "Session not found");

        let details = err.error.details.expect("should have details");
        assert_eq!(details["session_name"], "my-session");
        assert_eq!(details["available_sessions"].as_array().unwrap().len(), 2);
        assert_eq!(details["available_sessions"][0], "default");

        let suggestion = err.error.suggestion.expect("should have suggestion");
        assert!(suggestion.contains("scp session list"));
    }

    #[test]
    fn test_error_with_available_sessions_empty_list() {
        let err = error_with_available_sessions(
            ErrorCode::SessionNotFound,
            "No sessions",
            "my-session",
            &[],
        );

        let details = err.error.details.expect("should have details");
        assert_eq!(details["available_sessions"].as_array().unwrap().len(), 0);
    }
}
