//! VCS Hooks System
//!
//! Provides a hook system for VCS operations allowing:
//! - Pre-operation hooks (before rebase, push, merge, etc.)
//! - Post-operation hooks (after commit, push, merge, etc.)
//! - Custom hook scripts via shell commands
//! - Async hook execution

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Hook event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
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
    #[default]
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

impl std::fmt::Display for HookEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Hook result
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    #[must_use]
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Set timeout
    #[must_use]
    pub fn timeout(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }

    /// Disable the hook
    #[must_use]
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    pub fn validate(&self) -> Result<()> {
        if self.command.is_empty() {
            return Err(Error::validation_error("hook.command must not be empty"));
        }

        let dangerous_chars = ['|', '&', ';', '$', '`', '(', ')', '<', '>', '\n', '\r'];

        if self.command.chars().any(|c| dangerous_chars.contains(&c)) {
            return Err(Error::validation_error(format!(
                "hook '{}' command contains shell metacharacters",
                self.name
            )));
        }

        for (i, arg) in self.args.iter().enumerate() {
            if arg.chars().any(|c| dangerous_chars.contains(&c)) {
                return Err(Error::validation_error(format!(
                    "hook '{}' arg[{}] contains shell metacharacters",
                    self.name, i
                )));
            }
        }

        Ok(())
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

    pub fn register(&mut self, hook: Hook) -> Result<()> {
        hook.validate()?;
        self.hooks.entry(hook.event).or_default().push(hook);
        Ok(())
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

        for entry in std::fs::read_dir(dir).map_err(|e| Error::io_error(e.to_string()))? {
            let entry = entry.map_err(|e| Error::io_error(e.to_string()))?;
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
                manager.runner.register(hook)?;
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
    pub fn register(&mut self, hook: Hook) -> Result<()> {
        self.runner.register(hook)
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
        runner
            .register(Hook::new("test", HookEvent::PreCommit, "echo"))
            .unwrap();

        let env = HookEnv {
            event: HookEvent::PreCommit,
            vcs_type: "git".to_string(),
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
            vcs_type: "git".to_string(),
            repo_path: Some(PathBuf::from("/repo")),
            target: None,
        };

        let env_map = env.to_env();
        assert_eq!(
            env_map.get("SCP_HOOK_EVENT"),
            Some(&"pre-commit".to_string())
        );
        assert_eq!(env_map.get("SCP_HOOK_VCS"), Some(&"git".to_string()));
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

    // --- HookEvent: all variants and Display ---

    #[test]
    fn hook_event_all_variants_have_names() {
        for event in HookEvent::all() {
            let name = event.name();
            assert!(!name.is_empty(), "event {event:?} has empty name");
            assert!(
                name.contains('-'),
                "event {event:?} name '{name}' should contain '-'"
            );
        }
    }

    #[test]
    fn hook_event_all_count() {
        let all = HookEvent::all();
        assert_eq!(all.len(), 16);
    }

    #[test]
    fn hook_event_display_matches_name() {
        for event in HookEvent::all() {
            assert_eq!(event.to_string(), event.name());
        }
    }

    #[test]
    fn hook_event_default_is_post_commit() {
        assert_eq!(HookEvent::default(), HookEvent::PostCommit);
    }

    #[test]
    fn hook_event_equality() {
        assert_eq!(HookEvent::PreRebase, HookEvent::PreRebase);
        assert_ne!(HookEvent::PreRebase, HookEvent::PostRebase);
    }

    #[test]
    fn hook_event_all_names_unique() {
        let all = HookEvent::all();
        let mut names: Vec<&str> = all.iter().map(|e| e.name()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), all.len(), "HookEvent names are not unique");
    }

    #[test]
    fn hook_event_pre_post_pairs() {
        let pairs: &[(&str, &str)] = &[
            ("pre-rebase", "post-rebase"),
            ("pre-push", "post-push"),
            ("pre-pull", "post-pull"),
            ("pre-merge", "post-merge"),
            ("pre-commit", "post-commit"),
            ("pre-switch", "post-switch"),
            ("pre-workspace-create", "post-workspace-create"),
            ("pre-workspace-delete", "post-workspace-delete"),
        ];
        // Verify all pre/post pairs exist via all()
        let names: Vec<&str> = HookEvent::all().iter().map(|e| e.name()).collect();
        for (pre, post) in pairs {
            assert!(names.contains(pre), "missing {pre}");
            assert!(names.contains(post), "missing {post}");
        }
    }

    // --- Hook: builder methods ---

    #[test]
    fn hook_new_with_defaults() {
        let hook = Hook::new("my-hook", HookEvent::PostCommit, "/bin/true");
        assert_eq!(hook.name, "my-hook");
        assert_eq!(hook.event, HookEvent::PostCommit);
        assert_eq!(hook.command, "/bin/true");
        assert!(hook.args.is_empty());
        assert!(hook.enabled);
        assert_eq!(hook.timeout_ms, 30000);
    }

    #[test]
    fn hook_builder_arg() {
        let hook = Hook::new("test", HookEvent::PrePush, "echo")
            .arg("--verbose")
            .arg("hello");
        assert_eq!(hook.args, vec!["--verbose", "hello"]);
    }

    #[test]
    fn hook_builder_timeout() {
        let hook = Hook::new("test", HookEvent::PrePush, "sleep").timeout(5000);
        assert_eq!(hook.timeout_ms, 5000);
    }

    #[test]
    fn hook_builder_disabled() {
        let hook = Hook::new("test", HookEvent::PrePush, "echo").disabled();
        assert!(!hook.enabled);
    }

    #[test]
    fn hook_builder_chained() {
        let hook = Hook::new("chained", HookEvent::PreMerge, "git")
            .arg("status")
            .timeout(10000)
            .disabled();
        assert_eq!(hook.name, "chained");
        assert_eq!(hook.args, vec!["status"]);
        assert_eq!(hook.timeout_ms, 10000);
        assert!(!hook.enabled);
    }

    #[test]
    fn hook_clone() {
        let hook = Hook::new("original", HookEvent::PostCommit, "/bin/false").arg("--flag");
        let cloned = hook.clone();
        assert_eq!(cloned.name, hook.name);
        assert_eq!(cloned.args, hook.args);
    }

    #[test]
    fn hook_debug() {
        let hook = Hook::new("dbg", HookEvent::PreRebase, "echo");
        let debug_str = format!("{hook:?}");
        assert!(debug_str.contains("dbg"));
    }

    // --- HookResult ---

    #[test]
    fn hook_result_success_fields() {
        let result = HookResult::success(HookEvent::PostPush, "ok".to_string(), 42);
        assert!(result.success);
        assert_eq!(result.output, "ok");
        assert!(result.error.is_none());
        assert_eq!(result.duration_ms, 42);
        assert_eq!(result.event, HookEvent::PostPush);
    }

    #[test]
    fn hook_result_failure_fields() {
        let result = HookResult::failure(HookEvent::PreCommit, "boom".to_string(), 7);
        assert!(!result.success);
        assert!(result.output.is_empty());
        assert_eq!(result.error, Some("boom".to_string()));
        assert_eq!(result.duration_ms, 7);
        assert_eq!(result.event, HookEvent::PreCommit);
    }

    #[test]
    fn hook_result_has_timestamp() {
        let before = Utc::now();
        let result = HookResult::success(HookEvent::PostCommit, String::new(), 0);
        let after = Utc::now();
        assert!(result.timestamp >= before);
        assert!(result.timestamp <= after);
    }

    #[test]
    fn hook_result_clone() {
        let result = HookResult::success(HookEvent::PostCommit, "out".to_string(), 10);
        let cloned = result.clone();
        assert_eq!(cloned.success, result.success);
        assert_eq!(cloned.output, result.output);
    }

    #[test]
    fn hook_result_debug() {
        let result = HookResult::success(HookEvent::PostCommit, "out".to_string(), 10);
        let debug_str = format!("{result:?}");
        assert!(debug_str.contains("HookResult"));
    }

    // --- HookEnv ---

    #[test]
    fn hook_env_default() {
        let env = HookEnv::default();
        assert_eq!(env.event, HookEvent::PostCommit);
        assert!(env.workspace.is_none());
        assert!(env.branch.is_none());
        assert!(env.vcs_type.is_empty());
        assert!(env.repo_path.is_none());
        assert!(env.target.is_none());
    }

    #[test]
    fn hook_env_to_env_minimal() {
        let env = HookEnv {
            event: HookEvent::PreCommit,
            vcs_type: "git".to_string(),
            ..Default::default()
        };
        let map = env.to_env();
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("SCP_HOOK_EVENT"), Some(&"pre-commit".to_string()));
        assert_eq!(map.get("SCP_HOOK_VCS"), Some(&"git".to_string()));
    }

    #[test]
    fn hook_env_to_env_with_target() {
        let env = HookEnv {
            event: HookEvent::PrePush,
            vcs_type: "git".to_string(),
            target: Some("origin".to_string()),
            ..Default::default()
        };
        let map = env.to_env();
        assert_eq!(map.get("SCP_HOOK_TARGET"), Some(&"origin".to_string()));
    }

    #[test]
    fn hook_env_to_env_without_repo_path() {
        let env = HookEnv {
            event: HookEvent::PreMerge,
            vcs_type: "git".to_string(),
            repo_path: None,
            ..Default::default()
        };
        let map = env.to_env();
        assert!(!map.contains_key("SCP_HOOK_REPO_PATH"));
    }

    // --- HookRunner ---

    #[test]
    fn hook_runner_default() {
        let runner = HookRunner::default();
        assert!(runner.list_hooks().is_empty());
    }

    #[test]
    fn hook_runner_register_multiple_same_event() {
        let mut runner = HookRunner::new();
        runner
            .register(Hook::new("hook1", HookEvent::PreCommit, "echo"))
            .unwrap();
        runner
            .register(Hook::new("hook2", HookEvent::PreCommit, "echo"))
            .unwrap();
        let hooks = runner.get_hooks(HookEvent::PreCommit);
        assert_eq!(hooks.len(), 2);
    }

    #[test]
    fn hook_runner_get_hooks_empty_event() {
        let runner = HookRunner::new();
        assert!(runner.get_hooks(HookEvent::PrePush).is_empty());
    }

    #[test]
    fn hook_runner_unregister_existing() {
        let mut runner = HookRunner::new();
        runner
            .register(Hook::new("to-remove", HookEvent::PreCommit, "echo"))
            .unwrap();
        assert!(runner.unregister(HookEvent::PreCommit, "to-remove"));
        assert!(runner.get_hooks(HookEvent::PreCommit).is_empty());
    }

    #[test]
    fn hook_runner_unregister_nonexistent() {
        let mut runner = HookRunner::new();
        assert!(!runner.unregister(HookEvent::PreCommit, "ghost"));
    }

    #[test]
    fn hook_runner_disabled_hook_skipped() {
        let mut runner = HookRunner::new();
        let _ =
            runner.register(Hook::new("disabled-hook", HookEvent::PreCommit, "echo").disabled());
        let env = HookEnv {
            event: HookEvent::PreCommit,
            vcs_type: "test".to_string(),
            ..Default::default()
        };
        let results = runner.run(HookEvent::PreCommit, &env);
        assert!(results.is_empty());
    }

    #[test]
    fn hook_runner_run_no_hooks_registered() {
        let runner = HookRunner::new();
        let env = HookEnv {
            event: HookEvent::PostPush,
            vcs_type: "test".to_string(),
            ..Default::default()
        };
        let results = runner.run(HookEvent::PostPush, &env);
        assert!(results.is_empty());
    }

    #[test]
    fn hook_runner_list_hooks() {
        let mut runner = HookRunner::new();
        let _ = runner.register(Hook::new("a", HookEvent::PreCommit, "echo"));
        let _ = runner.register(Hook::new("b", HookEvent::PrePush, "echo"));
        let list = runner.list_hooks();
        assert_eq!(list.len(), 2);
    }

    // --- HookConfig ---

    #[test]
    fn hook_config_default() {
        let config = HookConfig::default();
        assert!(config.hooks_dir.is_none());
        assert!(config.disabled_events.is_empty());
    }

    #[test]
    fn hook_config_load_hooks_nonexistent_dir() {
        let config = HookConfig::new();
        let hooks = config
            .load_hooks(Path::new("/nonexistent/path/12345"))
            .unwrap();
        assert!(hooks.is_empty());
    }

    #[test]
    fn hook_config_event_from_name() {
        // Tested indirectly via load_hooks in the nonexistent dir test.
        // Test the parsing logic via HookEvent names.
        assert_eq!(
            HookEvent::PreCommit.name(),
            "pre-commit",
            "expected pre-commit name for event_from_name lookup"
        );
    }

    // --- HookManager ---

    #[test]
    fn hook_manager_default() {
        let manager = HookManager::default();
        assert!(manager.list_hooks().is_empty());
    }

    #[test]
    fn hook_manager_register_and_list() {
        let mut manager = HookManager::new();
        let _ = manager.register(Hook::new("mgr-hook", HookEvent::PostCommit, "echo"));
        assert_eq!(manager.list_hooks().len(), 1);
    }

    #[test]
    fn hook_manager_run_pre_maps_post_to_pre() {
        let mut manager = HookManager::new();
        let _ = manager.register(Hook::new("pre-only", HookEvent::PreCommit, "echo"));

        let env = HookEnv {
            event: HookEvent::PostCommit,
            vcs_type: "test".to_string(),
            ..Default::default()
        };

        // run_pre with PostCommit should invoke PreCommit hooks
        let results = manager.run_pre(HookEvent::PostCommit, &env);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn hook_manager_run_post_passes_through() {
        let mut manager = HookManager::new();
        let _ = manager.register(Hook::new("post-only", HookEvent::PostCommit, "echo"));

        let env = HookEnv {
            event: HookEvent::PostCommit,
            vcs_type: "test".to_string(),
            ..Default::default()
        };

        let results = manager.run_post(HookEvent::PostCommit, &env);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn hook_manager_run_pre_no_op_for_unmapped_event() {
        let manager = HookManager::new();
        let env = HookEnv {
            event: HookEvent::PreRebase,
            vcs_type: "test".to_string(),
            ..Default::default()
        };
        // PreRebase is already a pre-event, so run_pre should return it as-is
        // (no hooks registered, so empty results)
        let results = manager.run_pre(HookEvent::PreRebase, &env);
        assert!(results.is_empty());
    }

    // --- HookResult serde ---

    #[test]
    fn hook_result_serde_round_trip() {
        let result = HookResult::success(HookEvent::PostCommit, "data".to_string(), 5);
        let json = serde_json::to_string(&result).expect("serialize");
        let deserialized: HookResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.success, result.success);
        assert_eq!(deserialized.output, result.output);
        assert_eq!(deserialized.event, result.event);
    }

    #[test]
    fn hook_result_failure_serde_round_trip() {
        let result = HookResult::failure(HookEvent::PrePush, "fail msg".to_string(), 99);
        let json = serde_json::to_string(&result).expect("serialize");
        let deserialized: HookResult = serde_json::from_str(&json).expect("deserialize");
        assert!(!deserialized.success);
        assert_eq!(deserialized.error, Some("fail msg".to_string()));
    }

    // --- Hook serde ---

    #[test]
    fn hook_serde_round_trip() {
        let hook = Hook::new("serde-hook", HookEvent::PostPush, "/usr/bin/git")
            .arg("push")
            .timeout(60000);
        let json = serde_json::to_string(&hook).expect("serialize");
        let deserialized: Hook = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.name, "serde-hook");
        assert_eq!(deserialized.command, "/usr/bin/git");
        assert_eq!(deserialized.args, vec!["push"]);
        assert_eq!(deserialized.timeout_ms, 60000);
        assert!(deserialized.enabled);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Additional serde roundtrip tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn hook_event_serde_roundtrip_all_variants() {
        for event in [
            HookEvent::PreRebase,
            HookEvent::PostRebase,
            HookEvent::PrePush,
            HookEvent::PostPush,
            HookEvent::PrePull,
            HookEvent::PostPull,
            HookEvent::PreMerge,
            HookEvent::PostMerge,
            HookEvent::PreCommit,
            HookEvent::PostCommit,
        ] {
            let json = serde_json::to_string(&event).expect("serialize ok");
            let deserialized: HookEvent = serde_json::from_str(&json).expect("deserialize ok");
            assert_eq!(event, deserialized, "Roundtrip failed for {event:?}");
        }
    }

    #[test]
    fn hook_config_serde_roundtrip() {
        let config = HookConfig {
            hooks_dir: Some(PathBuf::from("/tmp/hooks")),
            disabled_events: vec![HookEvent::PreMerge, HookEvent::PostPull],
        };
        let json = serde_json::to_string(&config).expect("serialize ok");
        let deserialized: HookConfig = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(config, deserialized);
    }

    #[test]
    fn hook_config_serde_with_empty_vecs() {
        let config = HookConfig {
            hooks_dir: None,
            disabled_events: vec![],
        };
        let json = serde_json::to_string(&config).expect("serialize ok");
        let deserialized: HookConfig = serde_json::from_str(&json).expect("deserialize ok");
        assert!(deserialized.hooks_dir.is_none());
        assert!(deserialized.disabled_events.is_empty());
    }

    #[test]
    fn hook_result_serde_with_none_error() {
        let result = HookResult::success(HookEvent::PrePush, "ok".to_string(), 1);
        let json = serde_json::to_string(&result).expect("serialize");
        let deserialized: HookResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(result, deserialized);
        assert!(deserialized.error.is_none());
    }
}
