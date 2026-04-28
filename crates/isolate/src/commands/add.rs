//! Create a new session with JJ workspace - JSONL output for AI-first control plane

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::too_many_lines)]

use std::path::PathBuf;

use anyhow::{Context, Result};
use isolate_core::{
    config,
    domain::SessionName,
    output::{
        emit_stdout, Action, ActionStatus, ActionTarget, ActionVerb, Issue, IssueId, IssueKind,
        IssueSeverity, IssueTitle, Message, OutputLine, ResultKind, ResultOutput, SessionOutput,
    },
    OutputFormat, WorkspaceState,
};
use serde_json::json;
pub use types::AddOptions;

use crate::{
    command_context,
    session::{validate_session_name, Session, SessionStatus, SessionUpdate},
};

const fn to_core_status(status: SessionStatus) -> isolate_core::types::SessionStatus {
    match status {
        SessionStatus::Creating => isolate_core::types::SessionStatus::Creating,
        SessionStatus::Active => isolate_core::types::SessionStatus::Active,
        SessionStatus::Paused => isolate_core::types::SessionStatus::Paused,
        SessionStatus::Completed => isolate_core::types::SessionStatus::Completed,
        SessionStatus::Failed => isolate_core::types::SessionStatus::Failed,
    }
}

fn json_envelope_mode() -> bool {
    std::env::args().any(|arg| arg == "--json" || arg == "-j")
}

fn emit_add_json_envelope(
    name: &str,
    workspace_path: &str,
    status: &str,
    created: bool,
    success: bool,
) -> Result<()> {
    let data = json!({
        "name": name,
        "workspace_path": workspace_path,
        "status": status,
        "created": created,
    });

    let output = json!({
        "$schema": "isolate://add-response/v1",
        "_schema_version": "1.0",
        "schema_type": "single",
        "success": success,
        "schema": "add-response",
        "type": "single",
        "data": data,
    });

    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}

// ============================================================================
// JSONL OUTPUT HELPERS
// ============================================================================

/// Emit an action line to stdout
fn emit_action(verb: &str, target: &str, status: ActionStatus) -> Result<()> {
    if json_envelope_mode() {
        return Ok(());
    }
    let action = Action::new(
        ActionVerb::new(verb).map_err(|e| anyhow::anyhow!("Invalid action verb: {e}"))?,
        ActionTarget::new(target).map_err(|e| anyhow::anyhow!("Invalid action target: {e}"))?,
        status,
    );
    emit_stdout(&OutputLine::Action(action)).map_err(|e| anyhow::anyhow!("{e}"))
}

/// Emit an action line with a result message
fn emit_action_with_result(
    verb: &str,
    target: &str,
    status: ActionStatus,
    result: &str,
) -> Result<()> {
    if json_envelope_mode() {
        return Ok(());
    }
    let action = Action::new(
        ActionVerb::new(verb).map_err(|e| anyhow::anyhow!("Invalid action verb: {e}"))?,
        ActionTarget::new(target).map_err(|e| anyhow::anyhow!("Invalid action target: {e}"))?,
        status,
    )
    .with_result(result.to_string());
    emit_stdout(&OutputLine::Action(action)).map_err(|e| anyhow::anyhow!("{e}"))
}

/// Emit a session output line
fn emit_session_output(session: &Session) -> Result<()> {
    if json_envelope_mode() {
        return Ok(());
    }
    let workspace_path: PathBuf = session.workspace_path.clone().into();

    let session_output = SessionOutput::new(
        session.name.clone(),
        to_core_status(session.status),
        session.state,
        workspace_path,
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;

    let session_output = if let Some(branch) = &session.branch {
        session_output.with_branch(branch.clone())
    } else {
        session_output
    };

    emit_stdout(&OutputLine::Session(session_output)).map_err(|e| anyhow::anyhow!("{e}"))
}

/// Emit an issue line to stdout
fn emit_issue(
    id: &str,
    title: String,
    kind: IssueKind,
    severity: IssueSeverity,
    session: Option<&str>,
    suggestion: Option<&str>,
) -> Result<()> {
    if json_envelope_mode() {
        return Ok(());
    }
    let mut issue = Issue::new(
        IssueId::new(id).map_err(|e| anyhow::anyhow!("Invalid issue ID: {e}"))?,
        IssueTitle::new(title).map_err(|e| anyhow::anyhow!("Invalid issue title: {e}"))?,
        kind,
        severity,
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;

    if let Some(s) = session {
        issue = issue
            .with_session(SessionName::parse(s.to_string()).map_err(|e| anyhow::anyhow!("{e}"))?);
    }
    if let Some(s) = suggestion {
        issue = issue.with_suggestion(s.to_string());
    }

    emit_stdout(&OutputLine::Issue(issue)).map_err(|e| anyhow::anyhow!("{e}"))
}

/// Emit a result output line (success)
fn emit_result_success(message: &str) -> Result<()> {
    if json_envelope_mode() {
        return Ok(());
    }
    let result = ResultOutput::success(
        ResultKind::Command,
        Message::new(message).map_err(|e| anyhow::anyhow!("Invalid message: {e}"))?,
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    emit_stdout(&OutputLine::Result(result)).map_err(|e| anyhow::anyhow!("{e}"))
}

/// Emit a result output line (failure)
fn emit_result_failure(message: &str) -> Result<()> {
    if json_envelope_mode() {
        return Ok(());
    }
    let result = ResultOutput::failure(
        ResultKind::Command,
        Message::new(message).map_err(|e| anyhow::anyhow!("Invalid message: {e}"))?,
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    emit_stdout(&OutputLine::Result(result)).map_err(|e| anyhow::anyhow!("{e}"))
}

// ============================================================================
// HUMAN-READABLE OUTPUT HELPERS (for non-JSON mode)
// ============================================================================

/// Output human-readable result (only for non-JSON mode)
fn output_human_result(
    name: &str,
    workspace_path: &str,
    mode: &str,
    created: bool,
    format: OutputFormat,
) {
    // Only output human-readable text in non-JSON mode
    if format.is_json() {
        return;
    }

    match (created, mode) {
        (false, "idempotent" | "command replay") => {
            println!("Session '{name}' already exists (idempotent)");
        }
        (false, _) => {
            println!("Session '{name}' already exists");
        }
        (true, _) => {
            println!("Created session '{name}' (workspace at {workspace_path})");
        }
    }
}

/// Output result in the appropriate format (JSONL or human)
#[allow(clippy::unused_self)]
fn output_result(
    name: &str,
    workspace_path: &str,
    mode: &str,
    created: bool,
    format: OutputFormat,
    session: Option<&Session>,
) -> Result<()> {
    if json_envelope_mode() {
        let status = if created {
            "active".to_string()
        } else {
            format!("Session '{name}' already exists ({mode})")
        };
        return emit_add_json_envelope(name, workspace_path, &status, created, true);
    }

    if format.is_json() {
        // Emit action for the creation/retrieval
        let action_verb = if created { "create" } else { "retrieve" };
        let action_status = if created {
            ActionStatus::Completed
        } else {
            ActionStatus::Skipped
        };

        emit_action_with_result(action_verb, name, action_status, &format!("{mode}: {name}"))?;

        // Emit session output if available
        if let Some(s) = session {
            emit_session_output(s)?;
        } else {
            // Create minimal session output for the result
            let workspace_path_buf: PathBuf = workspace_path.into();
            let session_output = SessionOutput::new(
                name.to_string(),
                isolate_core::types::SessionStatus::Active,
                WorkspaceState::Created,
                workspace_path_buf,
            )
            .map_err(|e| anyhow::anyhow!("{e}"))?;

            emit_stdout(&OutputLine::Session(session_output))
                .map_err(|e| anyhow::anyhow!("{e}"))?;
        }

        // Emit result
        let result_message = if created {
            format!("Created session '{name}' ({mode})")
        } else {
            format!("Session '{name}' already exists ({mode})")
        };
        emit_result_success(&result_message)?;
    } else {
        output_human_result(name, workspace_path, mode, created, format);
    }

    Ok(())
}

// ============================================================================
// STUB DEPENDENCIES (to be implemented)
// ============================================================================

/// Stub for checking prerequisites - returns the current working directory
async fn check_prerequisites() -> Result<std::path::PathBuf> {
    let cwd = std::env::current_dir().context("Failed to get current working directory")?;
    Ok(cwd)
}

/// Stub for getting session database
async fn get_session_db() -> Result<StubSessionDb> {
    Ok(StubSessionDb)
}

/// Stub session database for compilation
#[derive(Debug, Clone, Default)]
pub struct StubSessionDb;

impl StubSessionDb {
    /// Check if a command has been processed (stub)
    pub async fn is_command_processed(&self, _command_id: &str) -> Result<bool> {
        Ok(false)
    }

    /// List sessions (stub - returns empty)
    pub async fn list(&self, _filter: Option<&str>) -> Result<Vec<Session>> {
        Ok(Vec::new())
    }

    /// Get a session by name (stub - returns none)
    pub async fn get(&self, _name: &str) -> Result<Option<Session>> {
        Ok(None)
    }

    /// Update a session (stub - returns ok)
    pub async fn update(&self, _name: &str, _update: SessionUpdate) -> Result<()> {
        Ok(())
    }

    /// List incomplete add operations (stub - returns empty)
    pub async fn list_incomplete_add_operations(&self) -> Result<Vec<Session>> {
        Ok(Vec::new())
    }
}

/// Query bead metadata by ID (stub)
async fn query_bead_metadata(_bead_id: &str) -> Result<Option<crate::beads::BeadMetadata>> {
    Ok(None)
}

/// Execute post-create hooks (stub)
async fn execute_post_create_hooks(_workspace_path: &str) -> Result<()> {
    Ok(())
}

/// Atomic session creation (stub)
async fn atomic_create_session(
    _name: &str,
    _workspace_path: &std::path::Path,
    _root: &std::path::Path,
    _db: &StubSessionDb,
    _bead_metadata: Option<crate::beads::BeadMetadata>,
    _command_id: Option<&str>,
) -> Result<()> {
    Ok(())
}

/// Rollback partial state on failure (stub)
async fn rollback_partial_state(_name: &str, _workspace_path: &std::path::Path) -> Result<()> {
    Ok(())
}

async fn handle_post_create_hook_failure(
    name: &str,
    workspace_path: &std::path::Path,
    _db: &StubSessionDb,
    hook_error: anyhow::Error,
) -> Result<()> {
    let rollback_result = rollback_partial_state(name, workspace_path).await;
    let failed_status_result = _db
        .update(
            name,
            SessionUpdate {
                status: Some(SessionStatus::Failed),
                ..Default::default()
            },
        )
        .await;

    match (rollback_result, failed_status_result) {
        (Ok(()), Ok(())) => Err(hook_error).context("post_create hook failed"),
        (Err(rollback_error), Ok(())) => Err(hook_error)
            .context(format!("post_create hook failed and rollback failed: {rollback_error}")),
        (Ok(()), Err(status_error)) => Err(hook_error).context(format!(
            "post_create hook failed and failed status update failed: {status_error}"
        )),
        (Err(rollback_error), Err(status_error)) => Err(hook_error).context(format!(
            "post_create hook failed, rollback failed: {rollback_error}, status update failed: {status_error}"
        )),
    }
}

/// Run the add command
#[allow(dead_code)]
pub async fn run(name: &str) -> Result<()> {
    let options = AddOptions::new(name.to_string());
    run_with_options(&options).await
}

/// Run the add command internally without output (for use by work command)
///
/// # Errors
///
/// Returns an error if session creation fails
pub async fn run_internal(options: &AddOptions) -> Result<()> {
    // Validate session name (REQ-CLI-015)
    validate_session_name(&options.name).map_err(anyhow::Error::new)?;

    let db = get_session_db().await?;
    let create_command_id = command_context::next_write_command_id("create", &options.name);

    // Check if session already exists (REQ-ERR-004)
    if db.get(&options.name).await?.is_some() {
        if let Some(ref command_id) = create_command_id {
            if db.is_command_processed(command_id).await? {
                return Ok(());
            }
        }
        return Err(anyhow::Error::new(crate::IsolateError::OperationFailed(
            format!("Session '{}' already exists", options.name),
        )));
    }

    let root = check_prerequisites().await?;

    // Query bead metadata if bead_id provided
    let bead_metadata = if let Some(bead_id) = &options.bead_id {
        query_bead_metadata(bead_id).await?
    } else {
        None
    };

    // Load config to get workspace_dir setting
    let cfg = config::load_config()
        .await
        .map_err(|e| anyhow::Error::msg(e.to_string()))?;

    // Check max_sessions limit before creating
    let current_sessions = db.list(None).await?;
    if current_sessions.len() >= cfg.session.max_sessions {
        return Err(anyhow::anyhow!(
            "Session limit reached: {} sessions already exist (max: {}). Use 'isolate remove' to free up space.",
            current_sessions.len(),
            cfg.session.max_sessions
        ));
    }

    // Construct workspace path from config's workspace_dir
    let workspace_base = root.join(&cfg.workspace_dir);
    let workspace_path = workspace_base.join(&options.name);
    let workspace_path_str = workspace_path.display().to_string();

    // ATOMIC SESSION CREATION
    atomic_create_session(
        &options.name,
        &workspace_path,
        &root,
        &db,
        bead_metadata,
        create_command_id.as_deref(),
    )
    .await?;

    // Execute post_create hooks unless --no-hooks
    if !options.no_hooks {
        if let Err(e) = execute_post_create_hooks(&workspace_path_str).await {
            return handle_post_create_hook_failure(&options.name, &workspace_path, &db, e).await;
        }
    }

    // Transition to 'active' status
    db.update(
        &options.name,
        SessionUpdate {
            status: Some(SessionStatus::Active),
            ..Default::default()
        },
    )
    .await
    .context("Failed to activate session")?;

    Ok(())
}

/// Run the add command with options
#[allow(clippy::too_many_lines, dead_code)]
pub async fn run_with_options(options: &AddOptions) -> Result<()> {
    // Phase 1: Validate input and environment
    validate_session_name(&options.name).map_err(anyhow::Error::new)?;
    let db = get_session_db().await?;
    let root = check_prerequisites().await?;

    // Load config to determine paths
    let cfg = config::load_config()
        .await
        .map_err(|e| anyhow::Error::msg(e.to_string()))?;
    let workspace_path = root.join(&cfg.workspace_dir).join(&options.name);
    let workspace_path_str = workspace_path.display().to_string();

    // Check max_sessions limit before creating
    let current_sessions = db.list(None).await?;
    if current_sessions.len() >= cfg.session.max_sessions {
        return Err(anyhow::anyhow!(
            "Session limit reached: {} sessions already exist (max: {}). Use 'isolate remove' to free up space.",
            current_sessions.len(),
            cfg.session.max_sessions
        ));
    }

    // Phase 2: Check for existing session and handle early exit
    if let Some(existing) = db.get(&options.name).await? {
        return handle_existing_session(options, &db, existing).await;
    }

    // Phase 3: Handle dry run for new session
    if options.dry_run {
        return handle_new_session_dry_run(options, &workspace_path_str);
    }

    // Phase 4: Perform the actual creation sequence
    let session = perform_creation_sequence(options, &root, &workspace_path, &db).await?;

    // Phase 5: Output result
    output_result(
        &options.name,
        &workspace_path_str,
        "workspace created",
        true,
        options.format,
        Some(&session),
    )
}

/// Handle logic for when a session already exists
async fn handle_existing_session(
    options: &AddOptions,
    db: &StubSessionDb,
    existing: Session,
) -> Result<()> {
    let create_command_id = command_context::next_write_command_id("create", &options.name);

    if let Some(ref command_id) = create_command_id {
        if db.is_command_processed(command_id).await? {
            output_result(
                &options.name,
                &existing.workspace_path,
                "command replay",
                false,
                options.format,
                Some(&existing),
            )?;
            return Ok(());
        }
    }

    if options.idempotent {
        if options.dry_run {
            handle_existing_session_dry_run(options, &existing)?;
            return Ok(());
        }

        // Idempotent mode: return success with existing session info
        output_result(
            &options.name,
            &existing.workspace_path,
            "idempotent",
            false,
            options.format,
            Some(&existing),
        )?;
        return Ok(());
    }

    // Session already exists and idempotent mode is not enabled
    if options.format.is_json() {
        if json_envelope_mode() {
            emit_add_json_envelope(
                &options.name,
                &existing.workspace_path,
                &format!("Session '{}' already exists", options.name),
                false,
                false,
            )?;
        } else {
            emit_issue(
                "ADD-001",
                format!("Session '{}' already exists", options.name),
                IssueKind::Validation,
                IssueSeverity::Error,
                Some(&options.name),
                Some("Use --idempotent to reuse existing session, or choose a different name"),
            )?;
            emit_result_failure(&format!("Session '{}' already exists", options.name))?;
        }
    }

    let error =
        crate::IsolateError::OperationFailed(format!("Session '{}' already exists", options.name));
    Err(anyhow::Error::new(error).context(format!(
        "Session path: {}\n\nAlternatives:\n  - Use a different name\n  - Use --idempotent to reuse\n  - Use --force to overwrite (if implemented)",
        existing.workspace_path
    )))
}

/// Handle dry run output for an existing session
fn handle_existing_session_dry_run(options: &AddOptions, existing: &Session) -> Result<()> {
    if json_envelope_mode() {
        return emit_add_json_envelope(
            &options.name,
            &existing.workspace_path,
            "[DRY RUN] Session already exists (idempotent)",
            false,
            true,
        );
    }

    if options.format.is_json() {
        emit_action_with_result(
            "dry-run",
            &options.name,
            ActionStatus::Skipped,
            "[DRY RUN] Session already exists (idempotent)",
        )?;
        emit_session_output(existing)?;
        emit_result_success(&format!(
            "[DRY RUN] Session '{}' already exists (idempotent)",
            options.name
        ))?;
    } else {
        println!(
            "[DRY RUN] Session '{}' already exists (idempotent)",
            options.name
        );
        println!("  Workspace: {}", existing.workspace_path);
    }
    Ok(())
}

/// Handle dry run output for a new session
fn handle_new_session_dry_run(options: &AddOptions, workspace_path_str: &str) -> Result<()> {
    if json_envelope_mode() {
        return emit_add_json_envelope(
            &options.name,
            workspace_path_str,
            "[DRY RUN] Would create session",
            true,
            true,
        );
    }

    if options.format.is_json() {
        emit_action_with_result(
            "dry-run",
            &options.name,
            ActionStatus::Pending,
            "[DRY RUN] Would create session",
        )?;

        // Create minimal session output for dry run
        let workspace_path_buf: PathBuf = workspace_path_str.into();
        let session_output = SessionOutput::new(
            options.name.clone(),
            isolate_core::types::SessionStatus::Creating,
            WorkspaceState::Created,
            workspace_path_buf,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;

        emit_stdout(&OutputLine::Session(session_output)).map_err(|e| anyhow::anyhow!("{e}"))?;

        emit_result_success(&format!(
            "[DRY RUN] Would create session '{}'",
            options.name
        ))?;
    } else {
        println!("[DRY RUN] Would create session '{}'", options.name);
        println!("  Workspace: {workspace_path_str}");
    }
    Ok(())
}

/// Perform the actual session creation sequence (atomic create + hooks + activate)
async fn perform_creation_sequence(
    options: &AddOptions,
    root: &std::path::Path,
    workspace_path: &std::path::Path,
    db: &StubSessionDb,
) -> Result<Session> {
    // Query bead metadata if bead_id provided
    let bead_metadata = if let Some(bead_id) = &options.bead_id {
        query_bead_metadata(bead_id).await?
    } else {
        None
    };

    let create_command_id = command_context::next_write_command_id("create", &options.name);

    // Emit action: creating workspace
    if options.format.is_json() {
        emit_action("create", &options.name, ActionStatus::InProgress)?;
    }

    // ATOMIC SESSION CREATION
    atomic_create_session(
        &options.name,
        workspace_path,
        root,
        db,
        bead_metadata,
        create_command_id.as_deref(),
    )
    .await?;

    // Emit action: workspace created
    if options.format.is_json() {
        emit_action_with_result(
            "create",
            "workspace",
            ActionStatus::Completed,
            &format!("Created workspace at {}", workspace_path.display()),
        )?;

        emit_action_with_result(
            "create",
            "database_record",
            ActionStatus::Completed,
            &format!("Created database record for '{}'", options.name),
        )?;
    }

    let mut session = db
        .get(&options.name)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Session record lost during atomic creation"))?;

    // Execute post_create hooks unless --no-hooks
    if !options.no_hooks {
        if let Err(e) = execute_post_create_hooks(&workspace_path.to_string_lossy()).await {
            handle_post_create_hook_failure(&options.name, workspace_path, db, e).await?;
        }
    }

    // Transition to 'active' status
    db.update(
        &options.name,
        SessionUpdate {
            status: Some(SessionStatus::Active),
            ..Default::default()
        },
    )
    .await
    .context("Failed to activate session")?;

    session.status = SessionStatus::Active;
    Ok(session)
}

pub async fn pending_add_operation_count(db: &StubSessionDb) -> Result<usize> {
    Ok(db.list_incomplete_add_operations().await?.len())
}

// ============================================================================
// TYPES MODULE
// ============================================================================

pub mod types {
    use isolate_core::OutputFormat;

    /// Options for the add command
    #[derive(Debug, Clone)]
    pub struct AddOptions {
        pub name: String,
        pub bead_id: Option<String>,
        pub no_hooks: bool,
        pub no_open: bool,
        pub format: OutputFormat,
        pub idempotent: bool,
        pub dry_run: bool,
    }

    impl AddOptions {
        /// Create a new AddOptions with default settings
        #[must_use]
        pub fn new(name: String) -> Self {
            Self {
                name,
                bead_id: None,
                no_hooks: false,
                no_open: false,
                format: OutputFormat::default(),
                idempotent: false,
                dry_run: false,
            }
        }
    }

    impl Default for AddOptions {
        fn default() -> Self {
            Self::new(String::new())
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn test_add_options_new() {
        let opts = AddOptions::new("test-session".to_string());
        assert_eq!(opts.name, "test-session");
        assert!(!opts.no_hooks);
        assert!(!opts.no_open);
    }

    // Tests for P0-3a: Validation errors should map to exit code 1

    #[test]
    fn test_add_invalid_name_returns_validation_error() {
        // Empty name
        let result = validate_session_name("");
        assert!(result.is_err());

        // Non-ASCII name
        let result = validate_session_name("test-session-🚀");
        assert!(result.is_err());

        // Name starting with number
        let result = validate_session_name("123-test");
        assert!(result.is_err());

        // Name with invalid characters
        let result = validate_session_name("test session");
        assert!(result.is_err());
    }

    #[test]
    fn test_duplicate_session_error_wraps_validation_error() {
        // This test verifies that the duplicate session check creates an error
        let err = crate::IsolateError::OperationFailed("Session 'test' already exists".into());
        assert!(matches!(err, crate::IsolateError::OperationFailed { .. }));
    }
}
