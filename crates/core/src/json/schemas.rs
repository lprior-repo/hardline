//! Schema registry - Single Source of Truth for Schema IDs
//!
//! This module provides consistent schema URI constants across all CLI JSON outputs.
//!
//! # Conformance
//!
//! - Commands MUST use these constants when creating `SchemaEnvelope`
//! - Contract.rs MUST reference these for `output_schema` documentation
//! - Tests verify both contract and runtime use the same values

/// Schema version for all responses
pub const SCHEMA_VERSION: &str = "1.0";

/// Base URI for all schemas
pub const BASE_URI: &str = "scp://";

// Command response schemas
pub const INIT_RESPONSE: &str = "init-response";
pub const ADD_RESPONSE: &str = "add-response";
pub const LIST_RESPONSE: &str = "list-response";
pub const REMOVE_RESPONSE: &str = "remove-response";
pub const FOCUS_RESPONSE: &str = "focus-response";
pub const STATUS_RESPONSE: &str = "status-response";
pub const SYNC_RESPONSE: &str = "sync-response";
pub const DONE_RESPONSE: &str = "done-response";
pub const UNDO_RESPONSE: &str = "undo-response";
pub const REVERT_RESPONSE: &str = "revert-response";
pub const WORK_RESPONSE: &str = "work-response";
pub const ABORT_RESPONSE: &str = "abort-response";
pub const SPAWN_RESPONSE: &str = "spawn-response";
pub const WHEREAMI_RESPONSE: &str = "whereami-response";
pub const WHOAMI_RESPONSE: &str = "whoami-response";
pub const DOCTOR_RESPONSE: &str = "doctor-response";
pub const CLEAN_RESPONSE: &str = "clean-response";
pub const CONTEXT_RESPONSE: &str = "context-response";
pub const INTROSPECT_RESPONSE: &str = "introspect-response";
pub const CHECKPOINT_RESPONSE: &str = "checkpoint-response";
pub const CONTRACT_RESPONSE: &str = "contract-response";
pub const CONTRACTS_RESPONSE: &str = "contracts-response";
pub const SUBMIT_RESPONSE: &str = "submit-response";
pub const EXPORT_RESPONSE: &str = "export-response";
pub const IMPORT_RESPONSE: &str = "import-response";
pub const CLI_DISPLAY_RESPONSE: &str = "cli-display-response";

// Diff schemas
pub const DIFF_RESPONSE: &str = "diff-response";
pub const DIFF_STAT_RESPONSE: &str = "diff-stat-response";

// Query schemas
pub const QUERY_SESSION_EXISTS: &str = "query-session-exists";
pub const QUERY_CAN_RUN: &str = "query-can-run";
pub const QUERY_SUGGEST_NAME: &str = "query-suggest-name";
pub const QUERY_LOCK_STATUS: &str = "query-lock-status";
pub const QUERY_CAN_SPAWN: &str = "query-can-spawn";
pub const QUERY_PENDING_MERGES: &str = "query-pending-merges";
pub const QUERY_LOCATION: &str = "query-location";

// Error schema
pub const ERROR_RESPONSE: &str = "error-response";

/// Build a full schema URI from a schema name
#[must_use]
pub fn uri(schema_name: &str) -> String {
    format!("{BASE_URI}{schema_name}/v1")
}

/// Get all valid schema names for validation
///
/// # Returns
///
/// Returns a vector of all valid schema names. The result should be used
/// for validation or schema discovery purposes.
#[must_use]
pub fn all_valid_schemas() -> Vec<&'static str> {
    vec![
        INIT_RESPONSE,
        ADD_RESPONSE,
        LIST_RESPONSE,
        REMOVE_RESPONSE,
        FOCUS_RESPONSE,
        STATUS_RESPONSE,
        SYNC_RESPONSE,
        DONE_RESPONSE,
        UNDO_RESPONSE,
        REVERT_RESPONSE,
        WORK_RESPONSE,
        ABORT_RESPONSE,
        SPAWN_RESPONSE,
        WHEREAMI_RESPONSE,
        WHOAMI_RESPONSE,
        DOCTOR_RESPONSE,
        CLEAN_RESPONSE,
        CONTEXT_RESPONSE,
        INTROSPECT_RESPONSE,
        CHECKPOINT_RESPONSE,
        CONTRACT_RESPONSE,
        CONTRACTS_RESPONSE,
        SUBMIT_RESPONSE,
        EXPORT_RESPONSE,
        IMPORT_RESPONSE,
        CLI_DISPLAY_RESPONSE,
        DIFF_RESPONSE,
        DIFF_STAT_RESPONSE,
        QUERY_SESSION_EXISTS,
        QUERY_CAN_RUN,
        QUERY_SUGGEST_NAME,
        QUERY_LOCK_STATUS,
        QUERY_CAN_SPAWN,
        QUERY_PENDING_MERGES,
        QUERY_LOCATION,
        ERROR_RESPONSE,
    ]
}

/// Check if a schema name is valid
#[must_use]
pub fn is_valid_schema(schema_name: &str) -> bool {
    all_valid_schemas().contains(&schema_name)
}
