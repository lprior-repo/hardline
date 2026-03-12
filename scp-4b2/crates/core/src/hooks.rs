//! VCS Hooks System
//!
//! Provides a hook system for VCS operations allowing:
//! - Pre-operation hooks (before rebase, push, merge, etc.)
//! - Post-operation hooks (after commit, push, merge, etc.)
//! - Custom hook scripts via shell commands
//! - Async hook execution

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

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
            timeout_ms: 30000,
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
        let mut env = HashMap::new();
        env.insert("SCP_HOOK_EVENT".to_string(), self.event.name().to_string());
        env.insert("SCP_HOOK_VCS".to_string(), self.vcs_type.clone());

        if let Some(ws) = &self.workspace {
            env.insert("SCP_HOOK_WORKSPACE".to_string(), ws.clone());
        }
        if let Some(branch) = &self.branch {
            env.insert("SCP_HOOK_BRANCH".to_string(), branch.clone());
        }
        if let Some(path) = &self.repo_path {
            env.insert(
                "SCP_HOOK_REPO_PATH".to_string(),
                path.to_string_lossy().to_string(),
            );
        }
        if let Some(target) = &self.target {
            env.insert("SCP_HOOK_TARGET".to_string(), target.clone());
        }

        env
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
        let mut results = Vec::new();
        let hooks = self.hooks.get(&event);

        if let Some(hooks) = hooks {
            for hook in hooks {
                if !hook.enabled {
                    continue;
                }

                let result = self.run_hook(hook, env);
                results.push(result);
            }
        }

        results
    }

    /// Run a single hook
    fn run_hook(&self, hook: &Hook, env: &HookEnv) -> HookResult {
        let start = std::time::Instant::now();

        let output = std::process::Command::new(&hook.command)
            .args(&hook.args)
            .envs(env.to_env())
            .output();

        let duration = start.elapsed().as_millis() as u64;

        match output {
            Ok(output) if output.status.success() => HookResult::success(
                hook.event,
                String::from_utf8_lossy(&output.stdout).to_string(),
                duration,
            ),
            Ok(output) => HookResult::failure(
                hook.event,
                String::from_utf8_lossy(&output.stderr).to_string(),
                duration,
            ),
            Err(e) => HookResult::failure(
                hook.event,
                format!("Failed to execute hook: {}", e),
                duration,
            ),
        }
    }

    /// Get hooks for an event
    pub fn get_hooks(&self, event: HookEvent) -> &[Hook] {
        self.hooks.get(&event).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// List all registered hooks
    pub fn list_hooks(&self) -> Vec<(&HookEvent, &Hook)> {
        let mut result = Vec::new();
        for (event, hooks) in &self.hooks {
            for hook in hooks {
                result.push((event, hook));
            }
        }
        result
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
        let mut hooks = Vec::new();

        if !dir.exists() {
            return Ok(hooks);
        }

        for entry in std::fs::read_dir(dir).map_err(Error::Io)? {
            let entry = entry.map_err(Error::Io)?;
            let path = entry.path();

            if path.is_file() {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                let event = Self::event_from_name(&name).unwrap_or(HookEvent::PostCommit);

                hooks.push(Hook::new(name, event, path.to_string_lossy().to_string()));
            }
        }

        Ok(hooks)
    }

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

    #[test]
    fn test_hook_env_to_env() {
        let env = HookEnv {
            event: HookEvent::PreCommit,
            workspace: Some("test-workspace".to_string()),
            branch: Some("main".to_string()),
            vcs_type: "jj".to_string(),
            repo_path: Some(PathBuf::from("/repo")),
            target: None,
        };

        let env_map = env.to_env();
        assert_eq!(
            env_map.get("SCP_HOOK_EVENT"),
            Some(&"pre-commit".to_string())
        );
        assert_eq!(env_map.get("SCP_HOOK_VCS"), Some(&"jj".to_string()));
        assert_eq!(
            env_map.get("SCP_HOOK_WORKSPACE"),
            Some(&"test-workspace".to_string())
        );
        assert_eq!(env_map.get("SCP_HOOK_BRANCH"), Some(&"main".to_string()));
        assert_eq!(
            env_map.get("SCP_HOOK_REPO_PATH"),
            Some(&"/repo".to_string())
        );
    }

    #[test]
    fn test_hook_config_new() {
        let config = HookConfig::new();
        assert!(config.hooks_dir.is_none());
        assert!(config.disabled_events.is_empty());
    }

    #[test]
    fn test_hook_manager_new() {
        let manager = HookManager::new();
        assert!(manager.list_hooks().is_empty());
    }

    #[test]
    fn test_hook_result_success() {
        let result = HookResult::success(HookEvent::PostCommit, "output".to_string(), 100);
        assert!(result.success);
        assert_eq!(result.output, "output");
        assert!(result.error.is_none());
    }

    #[test]
    fn test_hook_result_failure() {
        let result = HookResult::failure(HookEvent::PostCommit, "error".to_string(), 100);
        assert!(!result.success);
        assert!(result.output.is_empty());
        assert_eq!(result.error, Some("error".to_string()));
    }
}
