//! Action functions for the query command handler (Tier 3).
//!
//! I/O operations that execute structured queries.

use scp_core::{output::Output, Error, Result};

use super::data::{QueryOptions, QueryOutput, QueryType};

/// Execute the query command.
///
/// # Errors
///
/// Returns errors for invalid query types or query execution failures.
pub fn run_query(options: &QueryOptions) -> Result<()> {
    match options.query_type {
        QueryType::SessionExists => run_session_exists(options),
        QueryType::Sessions => run_sessions(options),
        QueryType::SessionInfo => run_session_info(options),
        QueryType::Blockers => run_blockers(),
        QueryType::SessionCount => run_session_count(),
        QueryType::Help => run_help(),
    }
}

fn run_session_exists(options: &QueryOptions) -> Result<()> {
    let name = options
        .argument
        .as_deref()
        .ok_or_else(|| Error::validation_error("Session name required for session-exists query"))?;

    // TODO: Wire to actual session database
    // For now, check if a worktree directory exists
    let cwd = std::env::current_dir()?;
    let worktree_path = cwd.join(".git").join("worktrees").join(name);
    let exists = worktree_path.exists();

    let output = QueryOutput {
        success: true,
        query_type: "session-exists".to_string(),
        data: serde_json::json!({
            "name": name,
            "exists": exists
        }),
    };

    if exists {
        Output::success(&format!("Session '{name}' exists"));
    } else {
        Output::info(&format!("Session '{name}' does not exist"));
    }

    let _ = output; // Available for JSON output when needed
    Ok(())
}

fn run_sessions(options: &QueryOptions) -> Result<()> {
    // TODO: Wire to actual session database
    Output::info("Sessions query:");

    if let Some(status) = &options.status_filter {
        Output::info(&format!("  Filter: status={status}"));
    }
    if let Some(agent) = &options.agent_filter {
        Output::info(&format!("  Filter: agent={agent}"));
    }

    Output::info("  No sessions found (database not yet connected)");
    Ok(())
}

fn run_session_info(options: &QueryOptions) -> Result<()> {
    let name = options
        .argument
        .as_deref()
        .ok_or_else(|| Error::validation_error("Session name required for session-info query"))?;

    Output::info(&format!("Session info for '{name}':"));
    Output::info("  (Session database not yet connected)");
    Ok(())
}

fn run_blockers() -> Result<()> {
    Output::info("Blockers query:");
    Output::info("  No blockers found (database not yet connected)");
    Ok(())
}

fn run_session_count() -> Result<()> {
    Output::info("Session count: 0 (database not yet connected)");
    Ok(())
}

fn run_help() -> Result<()> {
    Output::info("Available query types:");
    for name in QueryType::all_names() {
        Output::info(&format!("  {name}"));
    }
    Output::info("\nUsage: scp query <type> [argument] [options]");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_query_help() {
        let options = QueryOptions {
            query_type: QueryType::Help,
            argument: None,
            status_filter: None,
            agent_filter: None,
        };
        assert!(run_query(&options).is_ok());
    }

    #[test]
    fn run_query_session_exists_no_arg() {
        let options = QueryOptions {
            query_type: QueryType::SessionExists,
            argument: None,
            status_filter: None,
            agent_filter: None,
        };
        assert!(run_query(&options).is_err());
    }

    #[test]
    fn run_query_session_exists_with_arg() {
        let options = QueryOptions {
            query_type: QueryType::SessionExists,
            argument: Some("nonexistent-session".to_string()),
            status_filter: None,
            agent_filter: None,
        };
        // Should succeed (session just doesn't exist)
        assert!(run_query(&options).is_ok());
    }

    #[test]
    fn run_query_sessions() {
        let options = QueryOptions {
            query_type: QueryType::Sessions,
            argument: None,
            status_filter: Some("active".to_string()),
            agent_filter: None,
        };
        assert!(run_query(&options).is_ok());
    }

    #[test]
    fn run_query_session_count() {
        let options = QueryOptions {
            query_type: QueryType::SessionCount,
            argument: None,
            status_filter: None,
            agent_filter: None,
        };
        assert!(run_query(&options).is_ok());
    }

    #[test]
    fn run_query_blockers() {
        let options = QueryOptions {
            query_type: QueryType::Blockers,
            argument: None,
            status_filter: None,
            agent_filter: None,
        };
        assert!(run_query(&options).is_ok());
    }

    #[test]
    fn run_query_session_info_no_arg() {
        let options = QueryOptions {
            query_type: QueryType::SessionInfo,
            argument: None,
            status_filter: None,
            agent_filter: None,
        };
        assert!(run_query(&options).is_err());
    }
}
