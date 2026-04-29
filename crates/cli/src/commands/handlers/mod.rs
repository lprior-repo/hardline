//! CLI command handlers module
//!
//! This module bridges between `clap` and internal logic, providing handlers
//! adapted from the isolate project to hardline's architecture.
//!
//! Hardline uses clap's `Parser` derive macro and a dispatch-based architecture,
//! unlike isolate's `build_cli()` builder pattern. This module provides the
//! handler functions that are dispatched by `cli::dispatch` and `cli::dispatch_workspace`.
//!
//! Module organization:
//! - `workspace`: Session/workspace management (init, add, remove, switch, etc.)
//! - `sync`: Sync, diff, submit, done, abort
//! - `bookmark`: Bookmark operations
//! - `integrity`: Integrity, doctor, clean, prune
//! - `checkpoint`: Checkpoint, undo, revert, recover, retry, rollback
//! - `coordination`: Coordination commands
//! - `introspection`: AI, introspect, context, whereami, whoami, etc.
//! - `batch`: Batch and events operations
//! - `backup`: Backup, export, import
//! - `utility`: Config, query, schema, completions, wait
//! - `json_format`: Shared JSON format extraction helper

use std::process;

use serde_json::json;

/// CLI command handlers module organization.
///
/// This module is organized into logical submodules:
/// - `ai`: AI-first entry point
/// - `backup`: Backup, export, import operations
/// - `batch`: Batch command execution and event streaming
/// - `bookmark`: Bookmark operations
/// - `branch`: Branch management
/// - `can_i`: Permission checking
/// - `checkpoint`: Checkpoint, undo, revert, recover, retry, rollback
/// - `clean`: Clean operations
/// - `completions`: Shell completion generation
/// - `config_ports`: Port configuration
/// - `contract`: Contract documentation
/// - `coordination`: Coordination commands (claim, yield, lock, unlock)
/// - `done`: Done command
/// - `events`: Event streaming
/// - `examples`: Examples listing
/// - `export_import`: Export and import operations
/// - `integrity`: Integrity checking and repair
/// - `introspect`: Introspection commands
/// - `json_format`: Shared JSON format extraction helper
/// - `prune`: Prune invalid items
/// - `query`: Query operations
/// - `recover`: Recovery operations
/// - `rename`: Rename operations
/// - `revert`: Revert operations
/// - `schema`: Schema operations
/// - `session`: Session management
/// - `stack_auth`: Stack authentication
/// - `stack_sync`: Stack synchronization
/// - `sync`: Synchronization operations
/// - `task`: Task operations
/// - `undo`: Undo operations
/// - `validate`: Validation operations
/// - `wait`: Wait operations
/// - `whatif`: What-if analysis
/// - `whoami`: Identity information
/// - `work`: Work session management
/// - `workspace`: Workspace management
pub mod ai;
pub mod backup;
pub mod backup_cli;
pub mod batch;
pub mod bookmark;
pub mod branch;
pub mod can_i;
pub mod checkpoint;
pub mod clean;
pub mod completions;
pub mod config_ports;
pub mod contract;
pub mod coordination;
pub mod done;
pub mod events;
pub mod examples;
pub mod export_import;
pub mod integrity;
pub mod introspect;
pub mod json_format;
pub mod prune;
pub mod query;
pub mod recover;
pub mod rename;
pub mod retry;
pub mod revert;
pub mod schema;
pub mod session;
pub mod stack_auth;
pub mod stack_sync;
pub mod sync;
pub mod sync_submit;
pub mod task;
pub mod undo;
pub mod validate;
pub mod wait;
pub mod whatif;
pub mod whoami;
pub mod work;
pub mod workspace;

/// Command exit error type for CLI-specific exit codes.
///
/// This allows handlers to return a `CommandExit` error to cause the CLI
/// to exit with a specific exit code rather than a generic error.
#[derive(Debug)]
pub struct CommandExit {
    exit_code: i32,
}

impl CommandExit {
    /// Create a new `CommandExit` with the given exit code.
    #[must_use]
    pub const fn new(exit_code: i32) -> Self {
        Self { exit_code }
    }

    /// Get the exit code.
    #[must_use]
    pub const fn exit_code(&self) -> i32 {
        self.exit_code
    }
}

impl std::fmt::Display for CommandExit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Command failed with exit code {}", self.exit_code)
    }
}

impl std::error::Error for CommandExit {}

/// Format an error for user display (no stack traces).
///
/// This provides a clean error message without full context for CLI output.
#[must_use]
pub fn format_error(err: &anyhow::Error) -> String {
    let msg = err.to_string();
    if let Some(source) = err.source() {
        let source_msg = source.to_string();
        if !msg.contains(&source_msg) && !source_msg.is_empty() {
            return format!("{msg}\nCause: {source_msg}");
        }
    }
    msg
}

/// Output JSON display payload for CLI responses.
///
/// Wraps content in a schema envelope for structured CLI output.
fn output_json_display(display_type: &str, content: &str) {
    let payload = json!({
        "display_type": display_type,
        "content": content,
    });

    if let Ok(json_output) = serde_json::to_string_pretty(&payload) {
        println!("{json_output}");
    } else {
        // Fallback without schema envelope
        let fallback = json!({
            "$schema": "hardline://cli-display-response/v1",
            "_schema_version": "1.0",
            "schema_type": "single",
            "success": true,
            "display_type": display_type,
            "content": content,
        });
        if let Ok(serialized) = serde_json::to_string_pretty(&fallback) {
            println!("{serialized}");
        }
    }
}

/// Output a JSON parse error and return the exit code.
pub fn output_json_parse_error(_error: String) -> i32 {
    eprintln!("Error: Failed to parse arguments");
    2
}

/// Output a JSON error and return the exit code.
pub fn output_json_error(_error: &anyhow::Error) -> i32 {
    eprintln!("Error: Command failed");
    1
}
