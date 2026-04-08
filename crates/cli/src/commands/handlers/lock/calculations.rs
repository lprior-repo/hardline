//! Calculation functions for lock command handler (Tier 2).
//!
//! Pure functions with no I/O operations.

use crate::commands::handlers::lock::data::{
    AgentId, ForceUnlockOutput, HeartbeatOutput, LockCommand, LockEntry, LockListOutput,
    LockMetadata, LockOutput, LockStatus,
};

use scp_core::error::Error;

/// Maximum session name length.
const MAX_SESSION_LENGTH: usize = 255;

/// Maximum TTL value (24 hours in seconds).
const MAX_TTL: u64 = 86400;

/// Minimum TTL value (1 second).
const MIN_TTL: u64 = 1;

/// Validate a session name.
///
/// # Errors
///
/// Returns `Error::validation_error` if:
/// - Session name is empty
/// - Session name exceeds maximum length
/// - Session name contains invalid characters (newline, control chars)
pub fn validate_session_name(session: &str) -> Result<(), Error> {
    if session.trim().is_empty() {
        return Err(Error::validation_error("Session name cannot be empty"));
    }

    if session.len() > MAX_SESSION_LENGTH {
        return Err(Error::validation_error(format!(
            "Session name exceeds maximum length of {MAX_SESSION_LENGTH} characters"
        )));
    }

    if session.contains('\n') || session.contains('\r') {
        return Err(Error::validation_error(
            "Session name cannot contain newline characters",
        ));
    }

    // Check for control characters (ASCII 0-31 except tab which we allow)
    for ch in session.chars() {
        if ch.is_control() && ch != '\t' {
            return Err(Error::validation_error(
                "Session name cannot contain control characters",
            ));
        }
    }

    Ok(())
}

/// Validate an agent ID.
///
/// # Errors
///
/// Returns `Error::validation_error` if the agent ID is empty or whitespace-only.
pub fn validate_agent_id(agent: &str) -> Result<(), Error> {
    if agent.trim().is_empty() {
        return Err(Error::validation_error("Agent ID cannot be empty"));
    }

    Ok(())
}

/// Validate a TTL value.
///
/// # Errors
///
/// Returns `Error::validation_error` if TTL is outside valid range.
pub fn validate_ttl(ttl: u64) -> Result<(), Error> {
    if ttl == 0 {
        return Err(Error::validation_error("TTL must be greater than zero"));
    }

    if ttl > MAX_TTL {
        let hours = MAX_TTL / 3600;
        return Err(Error::validation_error(format!(
            "TTL exceeds maximum of {MAX_TTL} seconds ({hours} hours)"
        )));
    }

    Ok(())
}

/// Format a lock output for a locked session.
#[must_use]
pub fn format_locked_output(
    session: &str,
    agent: &str,
    expires_at: &str,
    ttl: Option<u64>,
    remaining: Option<u64>,
) -> LockOutput {
    LockOutput {
        status: LockStatus::Locked,
        session: session.to_string(),
        agent: Some(agent.to_string()),
        expires_at: Some(expires_at.to_string()),
        ttl,
        remaining_ttl: remaining,
        error: None,
    }
}

/// Format a lock output for an unlocked session.
#[must_use]
pub fn format_unlocked_output(session: &str) -> LockOutput {
    LockOutput {
        status: LockStatus::Unlocked,
        session: session.to_string(),
        agent: None,
        expires_at: None,
        ttl: None,
        remaining_ttl: None,
        error: None,
    }
}

/// Format a lock output with an error.
#[must_use]
pub fn format_error_output(session: &str, error: &str) -> LockOutput {
    LockOutput {
        status: LockStatus::Unlocked,
        session: session.to_string(),
        agent: None,
        expires_at: None,
        ttl: None,
        remaining_ttl: None,
        error: Some(error.to_string()),
    }
}

/// Format a heartbeat output.
#[must_use]
pub fn format_heartbeat_output(
    session: &str,
    expires_at: &str,
    success: bool,
    error: Option<&str>,
) -> HeartbeatOutput {
    HeartbeatOutput {
        session: session.to_string(),
        expires_at: expires_at.to_string(),
        success,
        error: error.map(String::from),
    }
}

/// Format a force unlock output.
#[must_use]
pub fn format_force_unlock_output(
    session: &str,
    admin: &str,
    success: bool,
    previous_holder: Option<&str>,
    error: Option<&str>,
) -> ForceUnlockOutput {
    ForceUnlockOutput {
        session: session.to_string(),
        admin: admin.to_string(),
        success,
        previous_holder: previous_holder.map(String::from),
        error: error.map(String::from),
    }
}

/// Format a lock metadata output.
#[must_use]
pub fn format_lock_metadata(
    session: &str,
    agent: &str,
    acquired_at: &str,
    ttl: u64,
    expires_at: &str,
    heartbeat_count: u64,
    is_expired: bool,
) -> LockMetadata {
    LockMetadata {
        session: session.to_string(),
        agent_id: agent.to_string(),
        acquired_at: acquired_at.to_string(),
        ttl,
        expires_at: expires_at.to_string(),
        heartbeat_count,
        is_expired,
    }
}

/// Format a lock entry for list output.
#[must_use]
pub fn format_lock_entry(
    session: &str,
    agent: &str,
    expires_at: &str,
    is_expired: bool,
) -> LockEntry {
    LockEntry {
        session: session.to_string(),
        agent: agent.to_string(),
        expires_at: expires_at.to_string(),
        is_expired,
    }
}

/// Format a lock list output.
#[must_use]
pub fn format_lock_list_output(locks: &[LockEntry]) -> LockListOutput {
    let count = locks.len();
    LockListOutput {
        count,
        locks: locks.to_vec(),
        has_locks: count > 0,
    }
}

/// Check if a session name contains only valid characters.
#[must_use]
pub fn is_valid_session_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '-' || ch == '_' || ch == '.'
}

/// Check if an agent ID contains only valid characters.
#[must_use]
pub fn is_valid_agent_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '-' || ch == '_'
}

/// Sanitize a session name by removing invalid characters.
///
/// # Note
///
/// This is a fallback for edge cases. Prefer rejecting invalid input at the boundary.
#[must_use]
pub fn sanitize_session_name(name: &str) -> String {
    name.chars()
        .filter(|ch| ch.is_alphanumeric() || *ch == '-' || *ch == '_' || *ch == '.')
        .collect()
}

/// Truncate a session name to maximum length.
#[must_use]
pub fn truncate_session_name(name: &str, max_len: usize) -> String {
    if name.len() <= max_len {
        name.to_string()
    } else {
        name[..max_len].to_string()
    }
}
