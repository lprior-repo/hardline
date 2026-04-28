//! Coordination handlers: claim, yield, lock, unlock
//!
//! Adapted from isolate's coordination handlers for hardline's architecture.
//!
//! Hardline does not have separate claim/yield commands, so those are stubbed.
//! Lock/unlock operations use hardline's existing lock module.

use anyhow::Result;
use clap::ArgMatches;
use scp_core::OutputFormat;

use super::json_format::get_format;
use crate::commands::lock;

/// Handle claim subcommand.
///
/// In hardline, "claim" is not a separate concept - agents just acquire locks.
/// This handler is a stub that acquires a lock on the resource.
pub async fn handle_claim(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let resource = sub_m
        .get_one::<String>("resource")
        .ok_or_else(|| anyhow::anyhow!("Resource is required"))?
        .clone();
    let timeout: u64 = sub_m
        .get_one::<String>("timeout")
        .and_then(|s| s.parse().ok())
        .map_or(30, |v| v);
    let agent_id = sub_m
        .get_one::<String>("agent-id")
        .cloned()
        .unwrap_or_else(|| "default-agent".to_string());

    // Claim = acquire lock with TTL
    let ttl = if timeout > 0 { Some(timeout) } else { None };
    lock::acquire(&resource, &agent_id, ttl)?;

    if format.is_json() {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "command": "claim",
                "resource": resource,
                "agent_id": agent_id,
                "status": "claimed"
            }))?
        );
    } else {
        println!("✓ Claimed resource '{}'", resource);
    }
    Ok(())
}

/// Handle yield subcommand.
///
/// In hardline, "yield" is not implemented as a separate concept.
/// This handler is a stub that returns an error indicating unimplemented.
pub async fn handle_yield(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let resource = sub_m
        .get_one::<String>("resource")
        .ok_or_else(|| anyhow::anyhow!("Resource is required"))?
        .clone();

    if format.is_json() {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "command": "yield",
                "resource": resource,
                "status": "unimplemented",
                "message": "yield command not yet implemented in hardline"
            }))?
        );
    } else {
        println!("yield command not yet implemented in hardline");
    }
    Ok(())
}

/// Handle lock subcommand.
///
/// Acquires an exclusive lock on a session for an agent.
pub async fn handle_lock(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let session = sub_m
        .get_one::<String>("session")
        .ok_or_else(|| anyhow::anyhow!("Session is required"))?
        .clone();
    let agent_id = sub_m
        .get_one::<String>("agent-id")
        .cloned()
        .unwrap_or_else(|| "default-agent".to_string());
    let ttl = sub_m.get_one::<u64>("ttl").copied();

    lock::acquire(&session, &agent_id, ttl)?;

    if format.is_json() {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "command": "lock",
                "session": session,
                "holder": agent_id,
                "status": "locked"
            }))?
        );
    } else {
        println!("✓ Locked session '{}' for agent '{}'", session, agent_id);
    }
    Ok(())
}

/// Handle unlock subcommand.
///
/// Releases a lock on a session.
pub async fn handle_unlock(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let session = sub_m
        .get_one::<String>("session")
        .ok_or_else(|| anyhow::anyhow!("Session is required"))?
        .clone();
    let agent_id = sub_m
        .get_one::<String>("agent-id")
        .cloned()
        .unwrap_or_else(|| "default-agent".to_string());

    lock::release(&session, &agent_id)?;

    if format.is_json() {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "command": "unlock",
                "session": session,
                "status": "unlocked"
            }))?
        );
    } else {
        println!("✓ Unlocked session '{}'", session);
    }
    Ok(())
}
