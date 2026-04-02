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

        // Build command
        let mut cmd = Command::new(&hook.command);
        cmd.args(&hook.args);

        // Set environment
        for (key, value) in env.to_env() {
            cmd.env(key, value);
        }

        // Run with timeout
        let output = match std::process::Command::new("timeout")
            .args([hook.timeout_ms.to_string(), hook.command.clone()])
            .args(&hook.args)
            .envs(env.to_env())
            .output()
        {
            Ok(o) => o,
            Err(e) => {
                return HookResult::failure(
                    hook.event,
                    format!("Failed to execute hook: {}", e),
                    start.elapsed().as_millis() as u64,
                );
            }
        };

        let duration = start.elapsed().as_millis() as u64;

        if output.status.success() {
            HookResult::success(
                hook.event,
                String::from_utf8_lossy(&output.stdout).to_string(),
                duration,
            )
        } else {
            HookResult::failure(
                hook.event,
                String::from_utf8_lossy(&output.stderr).to_string(),
                duration,
            )
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

        for entry in std::fs::read_dir(dir).map_err(Error::from)? {
            let entry = entry.map_err(Error::from)?;
            let path = entry.path();

            if path.is_file() {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                // Determine event from filename
                let event = Self::event_from_name(&name).unwrap_or(HookEvent::PostCommit);

                hooks.push(Hook::new(name, event, path.to_string_lossy().to_string()));
            }
        }

        Ok(hooks)
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
    use std::fs;

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

    // -- HookEvent tests --

    #[test]
    fn hook_event_all_returns_all_variants() {
        let all = HookEvent::all();
        assert_eq!(all.len(), 16);
        assert!(all.contains(&HookEvent::PreCommit));
        assert!(all.contains(&HookEvent::PostCommit));
        assert!(all.contains(&HookEvent::PreRebase));
        assert!(all.contains(&HookEvent::PostRebase));
    }

    #[test]
    fn hook_event_default_is_post_commit() {
        assert_eq!(HookEvent::default(), HookEvent::PostCommit);
    }

    #[test]
    fn hook_event_display_matches_name() {
        for event in HookEvent::all() {
            let display = format!("{event}");
            assert_eq!(display, event.name());
        }
    }

    #[test]
    fn hook_event_hash_and_eq() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for event in HookEvent::all() {
            set.insert(event);
        }
        assert_eq!(set.len(), 16);
    }

    #[test]
    fn hook_event_clone() {
        let event = HookEvent::PreCommit;
        let cloned = event;
        assert_eq!(event, cloned);
    }

    // -- Hook builder tests --

    #[test]
    fn hook_new_defaults() {
        let hook = Hook::new("my-hook", HookEvent::PrePush, "/bin/true");
        assert_eq!(hook.name, "my-hook");
        assert_eq!(hook.event, HookEvent::PrePush);
        assert_eq!(hook.command, "/bin/true");
        assert!(hook.args.is_empty());
        assert!(hook.enabled);
        assert_eq!(hook.timeout_ms, 30000);
    }

    #[test]
    fn hook_builder_arg() {
        let hook = Hook::new("test", HookEvent::PreCommit, "echo")
            .arg("--verbose");
        assert_eq!(hook.args, vec!["--verbose"]);
    }

    #[test]
    fn hook_builder_multiple_args() {
        let hook = Hook::new("test", HookEvent::PreCommit, "echo")
            .arg("a")
            .arg("b")
            .arg("c");
        assert_eq!(hook.args, vec!["a", "b", "c"]);
    }

    #[test]
    fn hook_builder_timeout() {
        let hook = Hook::new("test", HookEvent::PreCommit, "echo")
            .timeout(5000);
        assert_eq!(hook.timeout_ms, 5000);
    }

    #[test]
    fn hook_builder_disabled() {
        let hook = Hook::new("test", HookEvent::PreCommit, "echo")
            .disabled();
        assert!(!hook.enabled);
    }

    #[test]
    fn hook_builder_chain() {
        let hook = Hook::new("test", HookEvent::PreCommit, "echo")
            .arg("arg1")
            .timeout(1000)
            .disabled();
        assert_eq!(hook.args, vec!["arg1"]);
        assert_eq!(hook.timeout_ms, 1000);
        assert!(!hook.enabled);
    }

    #[test]
    fn hook_clone() {
        let hook = Hook::new("test", HookEvent::PreCommit, "echo")
            .arg("arg1")
            .timeout(1000);
        let cloned = hook.clone();
        assert_eq!(cloned.name, hook.name);
        assert_eq!(cloned.args, hook.args);
        assert_eq!(cloned.timeout_ms, hook.timeout_ms);
    }

    // -- HookRunner tests --

    #[test]
    fn hook_runner_new_is_empty() {
        let runner = HookRunner::new();
        assert!(runner.list_hooks().is_empty());
    }

    #[test]
    fn hook_runner_register_multiple() {
        let mut runner = HookRunner::new();
        runner.register(Hook::new("h1", HookEvent::PreCommit, "echo"));
        runner.register(Hook::new("h2", HookEvent::PreCommit, "echo"));
        runner.register(Hook::new("h3", HookEvent::PostCommit, "echo"));
        assert_eq!(runner.list_hooks().len(), 3);
    }

    #[test]
    fn hook_runner_unregister_existing() {
        let mut runner = HookRunner::new();
        runner.register(Hook::new("h1", HookEvent::PreCommit, "echo"));
        runner.register(Hook::new("h2", HookEvent::PreCommit, "echo"));
        assert!(runner.unregister(HookEvent::PreCommit, "h1"));
        assert_eq!(runner.get_hooks(HookEvent::PreCommit).len(), 1);
    }

    #[test]
    fn hook_runner_unregister_nonexistent() {
        let mut runner = HookRunner::new();
        assert!(!runner.unregister(HookEvent::PreCommit, "nonexistent"));
    }

    #[test]
    fn hook_runner_unregister_from_empty_event() {
        let mut runner = HookRunner::new();
        assert!(!runner.unregister(HookEvent::PostPush, "any"));
    }

    #[test]
    fn hook_runner_get_hooks_empty() {
        let runner = HookRunner::new();
        assert!(runner.get_hooks(HookEvent::PreCommit).is_empty());
    }

    #[test]
    fn hook_runner_run_no_hooks_returns_empty() {
        let runner = HookRunner::new();
        let env = HookEnv::default();
        let results = runner.run(HookEvent::PreCommit, &env);
        assert!(results.is_empty());
    }

    #[test]
    fn hook_runner_disabled_hooks_skipped() {
        let mut runner = HookRunner::new();
        runner.register(
            Hook::new("disabled", HookEvent::PreCommit, "echo")
                .disabled()
        );
        let env = HookEnv {
            event: HookEvent::PreCommit,
            vcs_type: "git".to_string(),
            ..Default::default()
        };
        let results = runner.run(HookEvent::PreCommit, &env);
        assert!(results.is_empty());
    }

    #[test]
    fn hook_runner_default() {
        let runner = HookRunner::default();
        assert!(runner.list_hooks().is_empty());
    }

    // -- HookResult tests --

    #[test]
    fn hook_result_success() {
        let result = HookResult::success(HookEvent::PreCommit, "output".to_string(), 100);
        assert!(result.success);
        assert_eq!(result.output, "output");
        assert!(result.error.is_none());
        assert_eq!(result.duration_ms, 100);
        assert_eq!(result.event, HookEvent::PreCommit);
    }

    #[test]
    fn hook_result_failure() {
        let result = HookResult::failure(HookEvent::PrePush, "error msg".to_string(), 50);
        assert!(!result.success);
        assert_eq!(result.output, "");
        assert_eq!(result.error, Some("error msg".to_string()));
        assert_eq!(result.duration_ms, 50);
    }

    #[test]
    fn hook_result_clone() {
        let result = HookResult::success(HookEvent::PostCommit, "out".to_string(), 10);
        let cloned = result.clone();
        assert_eq!(result.success, cloned.success);
        assert_eq!(result.output, cloned.output);
    }

    #[test]
    fn hook_result_has_timestamp() {
        let before = chrono::Utc::now();
        let result = HookResult::success(HookEvent::PreCommit, "out".to_string(), 0);
        let after = chrono::Utc::now();
        assert!(result.timestamp >= before);
        assert!(result.timestamp <= after);
    }

    // -- HookEnv tests --

    #[test]
    fn hook_env_default() {
        let env = HookEnv::default();
        assert!(env.workspace.is_none());
        assert!(env.branch.is_none());
        assert!(env.repo_path.is_none());
        assert!(env.target.is_none());
        assert_eq!(env.vcs_type, "");
    }

    #[test]
    fn hook_env_to_env_basic() {
        let env = HookEnv {
            event: HookEvent::PreCommit,
            vcs_type: "git".to_string(),
            workspace: Some("default".to_string()),
            branch: Some("main".to_string()),
            repo_path: Some(PathBuf::from("/repo")),
            target: None,
        };
        let map = env.to_env();
        assert_eq!(map.get("SCP_HOOK_EVENT").map(String::as_str), Some("pre-commit"));
        assert_eq!(map.get("SCP_HOOK_VCS").map(String::as_str), Some("git"));
        assert_eq!(map.get("SCP_HOOK_WORKSPACE").map(String::as_str), Some("default"));
        assert_eq!(map.get("SCP_HOOK_BRANCH").map(String::as_str), Some("main"));
        assert_eq!(map.get("SCP_HOOK_REPO_PATH").map(String::as_str), Some("/repo"));
        assert!(!map.contains_key("SCP_HOOK_TARGET"));
    }

    #[test]
    fn hook_env_to_env_with_target() {
        let env = HookEnv {
            event: HookEvent::PrePush,
            vcs_type: "jj".to_string(),
            target: Some("origin".to_string()),
            ..Default::default()
        };
        let map = env.to_env();
        assert_eq!(map.get("SCP_HOOK_TARGET").map(String::as_str), Some("origin"));
    }

    // -- HookConfig tests --

    #[test]
    fn hook_config_default() {
        let config = HookConfig::default();
        assert!(config.hooks_dir.is_none());
        assert!(config.disabled_events.is_empty());
    }

    #[test]
    fn hook_config_load_hooks_nonexistent_dir() {
        let config = HookConfig::new();
        let hooks = config.load_hooks(Path::new("/nonexistent/path")).expect("ok");
        assert!(hooks.is_empty());
    }

    #[test]
    fn hook_config_load_hooks_empty_dir() {
        let dir = tempfile::TempDir::new().expect("temp dir");
        let config = HookConfig::new();
        let hooks = config.load_hooks(dir.path()).expect("ok");
        assert!(hooks.is_empty());
    }

    #[test]
    fn hook_config_load_hooks_with_scripts() {
        let dir = tempfile::TempDir::new().expect("temp dir");
        // Create script files
        let pre_commit = dir.path().join("pre-commit-check");
        fs::write(&pre_commit, "#!/bin/bash\necho ok").expect("write");
        let post_commit = dir.path().join("post-commit-notify");
        fs::write(&post_commit, "#!/bin/bash\necho done").expect("write");

        let config = HookConfig::new();
        let hooks = config.load_hooks(dir.path()).expect("ok");
        assert_eq!(hooks.len(), 2);
        let events: Vec<&HookEvent> = hooks.iter().map(|h| &h.event).collect();
        assert!(events.contains(&&HookEvent::PreCommit));
        assert!(events.contains(&&HookEvent::PostCommit));
    }

    #[test]
    fn hook_config_event_from_name() {
        assert_eq!(HookConfig::event_from_name("pre-commit-lint"), Some(HookEvent::PreCommit));
        assert_eq!(HookConfig::event_from_name("my-post-push-hook"), Some(HookEvent::PostPush));
        assert_eq!(HookConfig::event_from_name("random-script"), None);
        assert_eq!(HookConfig::event_from_name(""), None);
    }

    // -- HookManager tests --

    #[test]
    fn hook_manager_new() {
        let manager = HookManager::new();
        assert!(manager.list_hooks().is_empty());
    }

    #[test]
    fn hook_manager_default() {
        let manager = HookManager::default();
        assert!(manager.list_hooks().is_empty());
    }

    #[test]
    fn hook_manager_register() {
        let mut manager = HookManager::new();
        manager.register(Hook::new("test", HookEvent::PreCommit, "echo"));
        assert_eq!(manager.list_hooks().len(), 1);
    }

    #[test]
    fn hook_manager_run_pre_maps_post_to_pre() {
        let manager = HookManager::new();
        let env = HookEnv {
            event: HookEvent::PostCommit,
            vcs_type: "git".to_string(),
            ..Default::default()
        };
        // No hooks registered, so result is empty
        let results = manager.run_pre(HookEvent::PostCommit, &env);
        assert!(results.is_empty());
    }

    #[test]
    fn hook_manager_run_post() {
        let manager = HookManager::new();
        let env = HookEnv {
            event: HookEvent::PostCommit,
            vcs_type: "git".to_string(),
            ..Default::default()
        };
        let results = manager.run_post(HookEvent::PostCommit, &env);
        assert!(results.is_empty());
    }

    // -- Serde roundtrip tests --

    #[test]
    fn hook_event_serde_roundtrip_all() {
        for event in HookEvent::all() {
            let json = serde_json::to_string(&event).expect("serialize");
            let deserialized: HookEvent = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(*event, deserialized);
        }
    }

    #[test]
    fn hook_result_serde_roundtrip() {
        let result = HookResult::success(HookEvent::PreCommit, "output".to_string(), 100);
        let json = serde_json::to_string(&result).expect("serialize");
        let deserialized: HookResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(result.success, deserialized.success);
        assert_eq!(result.output, deserialized.output);
    }

    #[test]
    fn hook_serde_roundtrip() {
        let hook = Hook::new("test", HookEvent::PrePush, "/bin/true")
            .arg("--verbose")
            .timeout(5000);
        let json = serde_json::to_string(&hook).expect("serialize");
        let deserialized: Hook = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(hook.name, deserialized.name);
        assert_eq!(hook.event, deserialized.event);
        assert_eq!(hook.command, deserialized.command);
        assert_eq!(hook.args, deserialized.args);
        assert_eq!(hook.timeout_ms, deserialized.timeout_ms);
        assert_eq!(hook.enabled, deserialized.enabled);
    }

    #[test]
    fn hook_config_serde_roundtrip() {
        let config = HookConfig {
            hooks_dir: Some(PathBuf::from("/project/.scp/hooks")),
            disabled_events: vec![HookEvent::PrePush, HookEvent::PostMerge],
        };
        let json = serde_json::to_string(&config).expect("serialize");
        let deserialized: HookConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(config.hooks_dir, deserialized.hooks_dir);
        assert_eq!(config.disabled_events, deserialized.disabled_events);
    }
}
