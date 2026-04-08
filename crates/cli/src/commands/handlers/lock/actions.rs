//! Action functions for lock command handler (Tier 3).
//!
//! I/O operations that interact with the lock manager and produce output.
//!
//! # Architecture Note
//!
//! This module contains the Actions layer - operations that produce side effects.
//! It delegates pure calculations to the `calculations` module and uses data
//! types from the `data` module.

use crate::commands::handlers::lock::calculations::{
    format_error_output, format_force_unlock_output, format_heartbeat_output,
    format_lock_list_output, format_lock_metadata, format_locked_output, format_unlocked_output,
    validate_agent_id, validate_session_name, validate_ttl,
};
use crate::commands::handlers::lock::data::{
    AgentId, ForceUnlockOutput, HeartbeatOutput, LockCommand, LockListOutput, LockMetadata,
    LockOutput,
};

use scp_core::error::Error;
use scp_core::output::Output;
use scp_core::Result;

/// Execute a lock command and produce output.
///
/// # Errors
///
/// Returns `Error::validation_error` if command parameters are invalid.
/// Returns `Error::invalid_state` if the lock operation fails.
pub fn run_lock_command(command: LockCommand) -> Result<LockOutput> {
    match command {
        LockCommand::Acquire {
            session,
            agent,
            ttl,
        } => run_acquire(&session, &agent, ttl),
        LockCommand::Release { session, agent } => run_release(&session, &agent),
        LockCommand::Heartbeat { session, agent } => run_heartbeat(&session, &agent),
        LockCommand::Status { session } => run_status(&session),
        LockCommand::List => run_list(),
        LockCommand::ForceUnlock { session, admin } => run_force_unlock(&session, &admin),
        LockCommand::Metadata { session } => run_metadata(&session),
    }
}

/// Run acquire command.
fn run_acquire(session: &str, agent: &str, ttl: Option<u64>) -> Result<LockOutput> {
    validate_session_name(session)?;
    validate_agent_id(agent)?;

    if let Some(t) = ttl {
        validate_ttl(t)?;
    }

    // Delegate to the actual lock manager using the default db path
    let result = crate::commands::lock::acquire(session, agent, ttl);

    match result {
        Ok(()) => Ok(LockOutput {
            status: crate::commands::handlers::lock::data::LockStatus::Locked,
            session: session.to_string(),
            agent: Some(agent.to_string()),
            expires_at: None,
            ttl,
            remaining_ttl: None,
            error: None,
        }),
        Err(e) => Ok(format_error_output(session, &e.to_string())),
    }
}

/// Run release command.
fn run_release(session: &str, agent: &str) -> Result<LockOutput> {
    validate_session_name(session)?;
    validate_agent_id(agent)?;

    let result = crate::commands::lock::release_with_path(session, agent, &get_db_path());

    match result {
        Ok(()) => Ok(format_unlocked_output(session)),
        Err(e) => Ok(format_error_output(session, &e.to_string())),
    }
}

/// Run heartbeat command.
fn run_heartbeat(session: &str, agent: &str) -> Result<LockOutput> {
    validate_session_name(session)?;
    validate_agent_id(agent)?;

    let result = crate::commands::lock::heartbeat_with_path(session, agent, &get_db_path());

    match result {
        Ok(()) => {
            Output::info(&format!("Heartbeat sent: {}", session));
            Ok(format_unlocked_output(session))
        }
        Err(e) => {
            Output::warn(&format!("Heartbeat failed: {}", e));
            Ok(format_error_output(session, &e.to_string()))
        }
    }
}

/// Run status command.
fn run_status(session: &str) -> Result<LockOutput> {
    validate_session_name(session)?;

    let result = crate::commands::lock::status_with_path(session, &get_db_path());

    match result {
        Ok(()) => Ok(format_locked_output(
            session, "unknown", "unknown", None, None,
        )),
        Err(e) => {
            // Status should never fail - return unlocked with error
            let mut output = format_unlocked_output(session);
            output.error = Some(e.to_string());
            Ok(output)
        }
    }
}

/// Run list command.
fn run_list() -> Result<LockOutput> {
    let result = crate::commands::lock::list_with_path(&get_db_path());

    match result {
        Ok(()) => Ok(format_unlocked_output("list")),
        Err(e) => Ok(format_error_output("list", &e.to_string())),
    }
}

/// Run force unlock command.
fn run_force_unlock(session: &str, admin: &str) -> Result<LockOutput> {
    validate_session_name(session)?;
    validate_agent_id(admin)?;

    // Force unlock by releasing with any agent
    let result = crate::commands::lock::release_with_path(session, admin, &get_db_path());

    match result {
        Ok(()) => {
            let _output = ForceUnlockOutput {
                session: session.to_string(),
                admin: admin.to_string(),
                success: true,
                previous_holder: None,
                error: None,
            };
            Output::info(&format!("Force unlock: {} by admin {}", session, admin));
            Ok(format_unlocked_output(session))
        }
        Err(e) => {
            let _output = ForceUnlockOutput {
                session: session.to_string(),
                admin: admin.to_string(),
                success: false,
                previous_holder: None,
                error: Some(e.to_string()),
            };
            Output::warn(&format!("Force unlock failed: {}", e));
            Ok(format_error_output(session, &e.to_string()))
        }
    }
}

/// Run metadata command.
fn run_metadata(session: &str) -> Result<LockOutput> {
    validate_session_name(session)?;

    // Use status to get metadata
    let result = crate::commands::lock::status_with_path(session, &get_db_path());

    match result {
        Ok(()) => {
            // Status already printed metadata, return success
            Ok(format_unlocked_output(session))
        }
        Err(e) => Ok(format_error_output(session, &e.to_string())),
    }
}

/// Get the database path from environment or default.
fn get_db_path() -> String {
    std::env::var("SCP_DATABASE_PATH").unwrap_or_else(|_| {
        let mut path = std::env::var("HOME")
            .map_or_else(|_| std::path::PathBuf::from("."), std::path::PathBuf::from);
        path.push(".scp");
        path.push("hardline.db");
        path.to_string_lossy().to_string()
    })
}
