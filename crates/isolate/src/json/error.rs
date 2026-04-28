//! JSON error conversion and handling

use isolate_core::json::ErrorDetail;
use serde::Serialize;

/// Sync command error details
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct SyncError {
    pub name: String,
    pub error: ErrorDetail,
}

/// Output a JSON error and return the appropriate semantic exit code
///
/// Converts an `anyhow::Error` to a JSON error structure and outputs it to stdout.
/// Returns the semantic exit code from the error:
/// - 1: Validation errors (user input issues)
/// - 2: Not found errors (missing resources)
/// - 3: System errors (IO, database issues)
/// - 4: External command errors
#[allow(clippy::print_stdout)]
pub fn output_json_error(error: &anyhow::Error) -> i32 {
    let error_str = error.to_string();

    // If the error message is already a JSON object (e.g. from doctor command),
    // output it as-is instead of wrapping it in another error envelope.
    // This prevents double-enveloping of JSON responses.
    if error_str.trim().starts_with('{') {
        println!("{error_str}");
        return semantic_exit_code(error);
    }

    // For now, output a simple error structure
    println!(r#"{{"error":{{"message":"{error_str}"}}}}"#);
    semantic_exit_code(error)
}

/// Output a CLI parse error as JSON and return clap-compatible exit code 2.
#[allow(clippy::print_stdout)]
pub fn output_json_parse_error(message: impl Into<String>) -> i32 {
    let msg = message.into();
    println!(r#"{{"error":{{"message":"{msg}","exit_code":2}}}}"#);
    2
}

/// Return the semantic exit code for an error.
#[allow(unused_variables)]
pub fn semantic_exit_code(error: &anyhow::Error) -> i32 {
    // Default to exit code 4 (external command error) for unknown errors
    4
}

/// Create an error envelope payload
#[derive(Debug, Serialize)]
struct ErrorEnvelopePayload {
    error: ErrorDetail,
}
