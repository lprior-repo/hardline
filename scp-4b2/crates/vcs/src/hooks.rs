//! VCS Hooks System
//!
//! Provides a hook system for VCS operations allowing:
//! - Pre-operation hooks (before rebase, push, merge, etc.)
//! - Post-operation hooks (after commit, push, merge, etc.)
//! - Custom hook scripts via shell commands
//! - Async hook execution

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tap::Pipe;

use scp_core::{Error, Result};

/// Hook event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HookEvent {
    /// Before a rebase operation
    PreRebase,
    /// After a rebase operation
    PostRebase,
    /// Before a push operation
    PrePush,
    /// After a push operation
    PostPush,
    /// Before a pull/fetch operation
    PrePull,
    /// After a pull/fetch operation
    PostPull,
    /// Before a merge operation
    PreMerge,
    /// After a merge operation
    PostMerge,
    /// Before a commit
    PreCommit,
    /// After a commit
    PostCommit,
    /// Before workspace switch
    PreSwitch,
    /// After workspace switch
    PostSwitch,
    /// Before workspace create
    PreWorkspaceCreate,
    /// After workspace create
    PostWorkspaceCreate,
    /// Before workspace delete
    PreWorkspaceDelete,
    /// After workspace delete
    PostWorkspaceDelete,
}

impl HookEvent {
    /// Get the hook name
    pub fn name(&self) -> &'static str {
        match self {
            Self::PreRebase => "pre-rebase",
            Self::PostRebase => "post-rebase",
            Self::PrePush => "pre-push",
            Self::PostPush => "post-push",
            Self::PrePull => "pre-pull",
            Self::PostPull => "post-pull",
            Self::PreMerge => "pre-merge",
            Self::PostMerge => "post-merge",
            Self::PreCommit => "pre-commit",
            Self::PostCommit => "post-commit",
            Self::PreSwitch => "pre-switch",
            Self::PostSwitch => "post-switch",
            Self::PreWorkspaceCreate => "pre-workspace-create",
            Self::PostWorkspaceCreate => "post-workspace-create",
            Self::PreWorkspaceDelete => "pre-workspace-delete",
            Self::PostWorkspaceDelete => "post-workspace-delete",
        }
    }

    /// Get all events
    pub fn all() -> &'static [Self] {
        &[
            Self::PreRebase,
            Self::PostRebase,
            Self::PrePush,
            Self::PostPush,
            Self::PrePull,
            Self::PostPull,
            Self::PreMerge,
            Self::PostMerge,
            Self::PreCommit,
            Self::PostCommit,
            Self::PreSwitch,
            Self::PostSwitch,
            Self::PreWorkspaceCreate,
            Self::PostWorkspaceCreate,
            Self::PreWorkspaceDelete,
            Self::PostWorkspaceDelete,
        ]
    }
}

impl Default for HookEvent {
    fn default() -> Self {
        Self::PostCommit
    }
}

impl std::fmt::Display for HookEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Hook result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookResult {
    pub event: HookEvent,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub timestamp: DateTime<Utc>,
}

impl HookResult {
    pub fn success(event: HookEvent, output: String, duration_ms: u64) -> Self {
        Self {
            event,
            success: true,
            output,
            error: None,
            duration_ms,
            timestamp: Utc::now(),
        }
    }

    pub fn failure(event: HookEvent, error: String, duration_ms: u64) -> Self {
        Self {
            event,
            success: false,
            output: String::new(),
            error: Some(error),
            duration_ms,
            timestamp: Utc::now(),
        }
    }
}

/// A hook definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hook {
    pub name: String,
    pub event: HookEvent,
    pub command: String,
    pub args: Vec<String>,
    pub enabled: bool,
    pub timeout_ms: u64,
}

impl Hook {
    /// Create a new hook
    pub fn new(name: impl Into<String>, event: HookEvent, command: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            event,
            command: command.into(),
            args: Vec::new(),
            enabled: true,
            timeout_ms: 30000, // 30 second default
        }
    }

    /// Add an argument
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Set timeout
    pub fn timeout(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }

    /// Disable the hook
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

/// Hook environment variables
#[derive(Debug, Clone, Default)]
pub struct HookEnv {
    pub event: HookEvent,
    pub workspace: Option<String>,
    pub branch: Option<String>,
    pub vcs_type: String,
    pub repo_path: Option<PathBuf>,
    pub target: Option<String>,
}

impl HookEnv {
    pub fn to_env(&self) -> HashMap<String, String> {
        use std::iter::once;
        once(("SCP_HOOK_EVENT".to_string(), self.event.name().to_string()))
            .chain(once(("SCP_HOOK_VCS".to_string(), self.vcs_type.clone())))
            .chain(
                self.workspace
                    .iter()
                    .map(|ws| ("SCP_HOOK_WORKSPACE".to_string(), ws.clone())),
            )
            .chain(
                self.branch
                    .iter()
                    .map(|b| ("SCP_HOOK_BRANCH".to_string(), b.clone())),
            )
            .chain(self.repo_path.iter().map(|p| {
                (
                    "SCP_HOOK_REPO_PATH".to_string(),
                    p.to_string_lossy().to_string(),
                )
            }))
            .chain(
                self.target
                    .iter()
                    .map(|t| ("SCP_HOOK_TARGET".to_string(), t.clone())),
            )
            .collect()
    }
}

/// Hook runner
pub struct HookRunner {
    hooks: HashMap<HookEvent, Vec<Hook>>,
}

impl HookRunner {
    pub fn new() -> Self {
        Self {
            hooks: HashMap::new(),
        }
    }

    /// Register a hook
    pub fn register(&mut self, hook: Hook) {
        self.hooks
            .entry(hook.event)
            .or_insert_with(Vec::new)
            .push(hook);
    }

    /// Unregister a hook by name
    pub fn unregister(&mut self, event: HookEvent, name: &str) -> bool {
        if let Some(hooks) = self.hooks.get_mut(&event) {
            let initial_len = hooks.len();
            hooks.retain(|h| h.name != name);
            hooks.len() < initial_len
        } else {
            false
        }
    }

    /// Run hooks for an event
    pub fn run(&self, event: HookEvent, env: &HookEnv) -> Vec<HookResult> {
        self.hooks
            .get(&event)
            .map(|hooks| {
                hooks
                    .iter()
                    .filter(|hook| hook.enabled)
                    .map(|hook| self.execute_hook_command(hook, env))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Run a single hook (action wrapper)
    fn run_hook(&self, hook: &Hook, env: &HookEnv) -> HookResult {
        let start = std::time::Instant::now();
        self.execute_hook_command(hook, env)
    }

    /// Execute hook command (calculation)
    fn execute_hook_command(&self, hook: &Hook, env: &HookEnv) -> HookResult {
        let output = std::process::Command::new("timeout")
            .args([hook.timeout_ms.to_string(), hook.command.clone()])
            .args(&hook.args)
            .envs(env.to_env())
            .output();

        match output {
            Ok(o) => Self::create_hook_result(hook.event, o),
            Err(e) => HookResult::failure(hook.event, format!("Failed to execute hook: {}", e), 0),
        }
    }

    /// Create hook result from command output (pure calculation)
    fn create_hook_result(event: HookEvent, output: std::process::Output) -> HookResult {
        let duration = output.duration.map_or(0, |d| d.as_millis() as u64);
        if output.status.success() {
            HookResult::success(
                event,
                String::from_utf8_lossy(&output.stdout).to_string(),
                duration,
            )
        } else {
            HookResult::failure(
                event,
                String::from_utf8_lossy(&output.stderr).to_string(),
                duration,
            )
        }
    }

    /// Get hooks for an event
    pub fn get_hooks(&self, event: HookEvent) -> &[Hook] {
        self.hooks
            .get(&event)
            .map(Vec::as_slice)
            .unwrap_or_else(|| &[])
    }

    /// List all registered hooks
    pub fn list_hooks(&self) -> Vec<(&HookEvent, &Hook)> {
        self.hooks
            .iter()
            .flat_map(|(event, hooks)| hooks.iter().map(move |hook| (event, hook)))
            .collect()
    }
}

impl Default for HookRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Hook configuration for a project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookConfig {
    pub hooks_dir: Option<PathBuf>,
    pub disabled_events: Vec<HookEvent>,
}

impl HookConfig {
    pub fn new() -> Self {
        Self {
            hooks_dir: None,
            disabled_events: Vec::new(),
        }
    }

    /// Load hooks from a directory
    pub fn load_hooks(&self, dir: &Path) -> Result<Vec<Hook>> {
        if !dir.exists() {
            return Ok(Vec::new());
        }

        std::fs::read_dir(dir)
            .map_err(Error::Io)?
            .filter_map(std::result::Result::ok)
            .map(|entry| entry.path())
            .filter(Path::is_file)
            .filter_map(|path| {
                let name = path.file_stem()?.to_str()?.to_string();
                let event = Self::event_from_name(&name).map_or(HookEvent::PostCommit, |e| e);
                Some(Hook::new(name, event, path.to_string_lossy().to_string()))
            })
            .collect::<Vec<_>>()
            .pipe(Ok)
    }

    /// Determine hook event from filename
    fn event_from_name(name: &str) -> Option<HookEvent> {
        let lower = name.to_lowercase();

        for event in HookEvent::all() {
            if lower.contains(event.name()) {
                return Some(*event);
            }
        }

        None
    }
}

impl Default for HookConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Global hook manager
pub struct HookManager {
    runner: HookRunner,
    config: HookConfig,
}

impl HookManager {
    pub fn new() -> Self {
        Self {
            runner: HookRunner::new(),
            config: HookConfig::new(),
        }
    }

    /// Initialize from project config
    pub fn from_project(project_path: &Path) -> Result<Self> {
        let mut manager = Self::new();

        // Load hooks from .scp/hooks
        let hooks_dir = project_path.join(".scp").join("hooks");
        if hooks_dir.exists() {
            let hooks = manager.config.load_hooks(&hooks_dir)?;
            for hook in hooks {
                manager.runner.register(hook);
            }
        }

        Ok(manager)
    }

    /// Run pre-operation hooks
    pub fn run_pre(&self, event: HookEvent, env: &HookEnv) -> Vec<HookResult> {
        // Only run pre- hooks
        let pre_event = match event {
            HookEvent::PostRebase => HookEvent::PreRebase,
            HookEvent::PostPush => HookEvent::PrePush,
            HookEvent::PostPull => HookEvent::PrePull,
            HookEvent::PostMerge => HookEvent::PreMerge,
            HookEvent::PostCommit => HookEvent::PreCommit,
            HookEvent::PostSwitch => HookEvent::PreSwitch,
            HookEvent::PostWorkspaceCreate => HookEvent::PreWorkspaceCreate,
            HookEvent::PostWorkspaceDelete => HookEvent::PreWorkspaceDelete,
            _ => event,
        };

        self.runner.run(pre_event, env)
    }

    /// Run post-operation hooks
    pub fn run_post(&self, event: HookEvent, env: &HookEnv) -> Vec<HookResult> {
        self.runner.run(event, env)
    }

    /// Register a hook
    pub fn register(&mut self, hook: Hook) {
        self.runner.register(hook);
    }

    /// Get hook results for debugging
    pub fn list_hooks(&self) -> Vec<(&HookEvent, &Hook)> {
        self.runner.list_hooks()
    }
}

impl Default for HookManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_event_names() {
        assert_eq!(HookEvent::PreRebase.name(), "pre-rebase");
        assert_eq!(HookEvent::PostPush.name(), "post-push");
    }

    #[test]
    fn test_hook_creation() {
        let hook = Hook::new("test-hook", HookEvent::PreCommit, "/bin/true");
        assert_eq!(hook.name, "test-hook");
        assert_eq!(hook.event, HookEvent::PreCommit);
        assert!(hook.enabled);
    }

    #[test]
    fn test_hook_runner() {
        let mut runner = HookRunner::new();
        runner.register(Hook::new("test", HookEvent::PreCommit, "echo"));

        let env = HookEnv {
            event: HookEvent::PreCommit,
            vcs_type: "jj".to_string(),
            ..Default::default()
        };

        let results = runner.run(HookEvent::PreCommit, &env);
        assert!(!results.is_empty());
    }
}
