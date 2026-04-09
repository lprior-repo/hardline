//! Worktree hook system — lifecycle hooks for create/remove operations.
//!
//! Defines a trait-based hook system that the WorktreeService calls before and
//! after worktree creation and removal. Implementations can execute shell
//! commands, send notifications, or perform any side-effect.
//!
//! # Hook contract
//!
//! - **Pre-hooks** run *before* the operation. A failed pre-hook aborts the
//!   operation by returning `Err(WorktreeDomainError::HookFailed)`.
//! - **Post-hooks** run *after* the operation succeeds. A failed post-hook
//!   does **not** roll back the operation — it is informational only.
//!
//! # Implementations
//!
//! - [`NoOpWorktreeHooks`] — default; does nothing.
//! - [`ShellWorktreeHooks`] — executes hook scripts from a directory, matching
//!   the same convention as the VCS hook system (filenames contain event names).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::domain::{WorktreeDomainError, WorktreeId};

// ---------------------------------------------------------------------------
// Hook event
// ---------------------------------------------------------------------------

/// Lifecycle events that can trigger hooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum WorktreeHookEvent {
    /// Before a worktree is created.
    #[default]
    PreCreate,
    /// After a worktree has been created.
    PostCreate,
    /// Before a worktree is removed.
    PreRemove,
    /// After a worktree has been removed.
    PostRemove,
}

impl WorktreeHookEvent {
    /// Machine-readable kebab-case name used in filenames and env vars.
    pub fn name(&self) -> &'static str {
        match self {
            Self::PreCreate => "pre-create",
            Self::PostCreate => "post-create",
            Self::PreRemove => "pre-remove",
            Self::PostRemove => "post-remove",
        }
    }

    /// All variants in order.
    pub fn all() -> &'static [Self] {
        &[
            Self::PreCreate,
            Self::PostCreate,
            Self::PreRemove,
            Self::PostRemove,
        ]
    }
}

impl std::fmt::Display for WorktreeHookEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

// ---------------------------------------------------------------------------
// Hook context — passed to every hook invocation
// ---------------------------------------------------------------------------

/// Contextual information available to hook implementations.
#[derive(Debug, Clone)]
pub struct HookContext {
    /// Which lifecycle event triggered the hook.
    pub event: WorktreeHookEvent,
    /// ID of the worktree being operated on (set for remove hooks; may be
    /// absent during pre-create when the ID hasn't been assigned yet).
    pub worktree_id: Option<WorktreeId>,
    /// Name of the worktree.
    pub worktree_name: Option<String>,
    /// Absolute path of the worktree on disk.
    pub worktree_path: Option<PathBuf>,
    /// Absolute path of the parent (repository root).
    pub parent_path: Option<PathBuf>,
    /// Worktree type as a string (e.g. "development", "agent").
    pub worktree_type: Option<String>,
    /// Branch the worktree is associated with.
    pub branch: Option<String>,
}

impl HookContext {
    /// Convert the context into environment variables suitable for a child
    /// process. All values are prefixed with `SCP_WORKTREE_`.
    pub fn to_env(&self) -> HashMap<String, String> {
        let mut env = HashMap::new();
        env.insert("SCP_WORKTREE_EVENT".to_string(), self.event.name().to_string());

        if let Some(id) = &self.worktree_id {
            env.insert("SCP_WORKTREE_ID".to_string(), id.as_string());
        }
        if let Some(name) = &self.worktree_name {
            env.insert("SCP_WORKTREE_NAME".to_string(), name.clone());
        }
        if let Some(path) = &self.worktree_path {
            env.insert(
                "SCP_WORKTREE_PATH".to_string(),
                path.to_string_lossy().to_string(),
            );
        }
        if let Some(pp) = &self.parent_path {
            env.insert(
                "SCP_WORKTREE_PARENT_PATH".to_string(),
                pp.to_string_lossy().to_string(),
            );
        }
        if let Some(wt) = &self.worktree_type {
            env.insert("SCP_WORKTREE_TYPE".to_string(), wt.clone());
        }
        if let Some(branch) = &self.branch {
            env.insert("SCP_WORKTREE_BRANCH".to_string(), branch.clone());
        }

        env
    }
}

// ---------------------------------------------------------------------------
// Hook result
// ---------------------------------------------------------------------------

/// Outcome of a single hook execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookOutcome {
    pub event: WorktreeHookEvent,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub duration_ms: u64,
}

impl HookOutcome {
    pub fn success(event: WorktreeHookEvent, output: String, duration_ms: u64) -> Self {
        Self {
            event,
            success: true,
            output,
            error: None,
            duration_ms,
        }
    }

    pub fn failure(event: WorktreeHookEvent, error: String, duration_ms: u64) -> Self {
        Self {
            event,
            success: false,
            output: String::new(),
            error: Some(error),
            duration_ms,
        }
    }
}

// ---------------------------------------------------------------------------
// Hook trait
// ---------------------------------------------------------------------------

/// Trait that worktree lifecycle hooks must implement.
///
/// All methods have default no-op implementations so that implementors only
/// need to override the events they care about.
pub trait WorktreeHooks: Send + Sync {
    /// Run hooks for the given event and context.
    ///
    /// Returns a list of outcomes (one per hook that ran). If the list is
    /// empty, no hooks were registered for this event.
    ///
    /// Implementations should return `Err` only when a **pre-hook** fails and
    /// the operation must be aborted. Post-hook failures are reported via the
    /// `HookOutcome` list but should not error the overall call.
    fn run(
        &self,
        event: WorktreeHookEvent,
        ctx: &HookContext,
    ) -> Result<Vec<HookOutcome>, WorktreeDomainError>;
}

// ---------------------------------------------------------------------------
// NoOpWorktreeHooks — default implementation
// ---------------------------------------------------------------------------

/// A no-op hook implementation that does nothing. Used as the default when no
/// hooks are configured.
#[derive(Debug, Clone, Default)]
pub struct NoOpWorktreeHooks;

impl WorktreeHooks for NoOpWorktreeHooks {
    fn run(
        &self,
        _event: WorktreeHookEvent,
        _ctx: &HookContext,
    ) -> Result<Vec<HookOutcome>, WorktreeDomainError> {
        Ok(Vec::new())
    }
}

// ---------------------------------------------------------------------------
// ShellWorktreeHooks — executes scripts from a directory
// ---------------------------------------------------------------------------

/// A hook implementation that discovers and runs executable scripts from a
/// hooks directory. Filenames containing an event name (e.g. `pre-create-setup`)
/// are matched to the corresponding event.
pub struct ShellWorktreeHooks {
    hooks_dir: PathBuf,
    timeout_ms: u64,
}

impl ShellWorktreeHooks {
    pub fn new(hooks_dir: PathBuf) -> Self {
        Self {
            hooks_dir,
            timeout_ms: 30_000,
        }
    }

    pub fn timeout(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }

    /// Discover hook scripts for a given event.
    fn discover(&self, event: WorktreeHookEvent) -> Vec<PathBuf> {
        let Ok(entries) = std::fs::read_dir(&self.hooks_dir) else {
            return Vec::new();
        };

        let event_name = event.name();
        let mut scripts: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.is_file()
                    && p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|name| {
                            name.to_lowercase().contains(event_name)
                        })
            })
            .collect();

        scripts.sort();
        scripts
    }

    fn run_script(
        &self,
        script: &Path,
        event: WorktreeHookEvent,
        env_vars: &HashMap<String, String>,
    ) -> HookOutcome {
        let start = std::time::Instant::now();
        let script_str = script.to_string_lossy().to_string();

        let output = std::process::Command::new("timeout")
            .arg(self.timeout_ms.to_string())
            .arg(&script_str)
            .envs(env_vars)
            .output();

        let duration = start.elapsed().as_millis() as u64;

        match output {
            Ok(o) if o.status.success() => HookOutcome::success(
                event,
                String::from_utf8_lossy(&o.stdout).to_string(),
                duration,
            ),
            Ok(o) => HookOutcome::failure(
                event,
                String::from_utf8_lossy(&o.stderr).to_string(),
                duration,
            ),
            Err(e) => HookOutcome::failure(
                event,
                format!("Failed to execute hook {}: {e}", script.display()),
                duration,
            ),
        }
    }
}

impl WorktreeHooks for ShellWorktreeHooks {
    fn run(
        &self,
        event: WorktreeHookEvent,
        ctx: &HookContext,
    ) -> Result<Vec<HookOutcome>, WorktreeDomainError> {
        let scripts = self.discover(event);
        if scripts.is_empty() {
            return Ok(Vec::new());
        }

        let env_vars = ctx.to_env();
        let mut outcomes = Vec::with_capacity(scripts.len());

        for script in scripts {
            let outcome = self.run_script(&script, event, &env_vars);
            let failed = !outcome.success;
            outcomes.push(outcome);
            // Abort on first failure for pre-hooks; continue for post-hooks.
            if failed && matches!(event, WorktreeHookEvent::PreCreate | WorktreeHookEvent::PreRemove) {
                return Ok(outcomes);
            }
        }

        Ok(outcomes)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- WorktreeHookEvent --

    #[test]
    fn hook_event_name_format() {
        assert_eq!(WorktreeHookEvent::PreCreate.name(), "pre-create");
        assert_eq!(WorktreeHookEvent::PostCreate.name(), "post-create");
        assert_eq!(WorktreeHookEvent::PreRemove.name(), "pre-remove");
        assert_eq!(WorktreeHookEvent::PostRemove.name(), "post-remove");
    }

    #[test]
    fn hook_event_all_contains_every_variant() {
        let all = WorktreeHookEvent::all();
        assert_eq!(all.len(), 4);
        assert!(all.contains(&WorktreeHookEvent::PreCreate));
        assert!(all.contains(&WorktreeHookEvent::PostCreate));
        assert!(all.contains(&WorktreeHookEvent::PreRemove));
        assert!(all.contains(&WorktreeHookEvent::PostRemove));
    }

    #[test]
    fn hook_event_display_matches_name() {
        for event in WorktreeHookEvent::all() {
            assert_eq!(event.to_string(), event.name());
        }
    }

    #[test]
    fn hook_event_names_unique() {
        let names: Vec<&str> = WorktreeHookEvent::all().iter().map(|e| e.name()).collect();
        let mut unique = names.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), names.len());
    }

    #[test]
    fn hook_event_serde_roundtrip() {
        for event in WorktreeHookEvent::all() {
            let json = serde_json::to_string(event).unwrap();
            let de: WorktreeHookEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(*event, de);
        }
    }

    // -- HookContext --

    #[test]
    fn hook_context_to_env_minimal() {
        let ctx = HookContext {
            event: WorktreeHookEvent::PreCreate,
            worktree_id: None,
            worktree_name: None,
            worktree_path: None,
            parent_path: None,
            worktree_type: None,
            branch: None,
        };
        let env = ctx.to_env();
        assert_eq!(env.len(), 1);
        assert_eq!(env.get("SCP_WORKTREE_EVENT"), Some(&"pre-create".to_string()));
    }

    #[test]
    fn hook_context_to_env_full() {
        let ctx = HookContext {
            event: WorktreeHookEvent::PostCreate,
            worktree_id: Some(WorktreeId::new_random()),
            worktree_name: Some("my-wt".to_string()),
            worktree_path: Some(PathBuf::from("/tmp/my-wt")),
            parent_path: Some(PathBuf::from("/home/user/repo")),
            worktree_type: Some("development".to_string()),
            branch: Some("main".to_string()),
        };
        let env = ctx.to_env();
        assert_eq!(env.len(), 7);
        assert_eq!(env.get("SCP_WORKTREE_NAME"), Some(&"my-wt".to_string()));
        assert_eq!(env.get("SCP_WORKTREE_PATH"), Some(&"/tmp/my-wt".to_string()));
        assert_eq!(env.get("SCP_WORKTREE_PARENT_PATH"), Some(&"/home/user/repo".to_string()));
        assert_eq!(env.get("SCP_WORKTREE_TYPE"), Some(&"development".to_string()));
        assert_eq!(env.get("SCP_WORKTREE_BRANCH"), Some(&"main".to_string()));
    }

    // -- HookOutcome --

    #[test]
    fn hook_outcome_success() {
        let o = HookOutcome::success(WorktreeHookEvent::PreCreate, "ok".to_string(), 5);
        assert!(o.success);
        assert_eq!(o.output, "ok");
        assert!(o.error.is_none());
        assert_eq!(o.duration_ms, 5);
    }

    #[test]
    fn hook_outcome_failure() {
        let o = HookOutcome::failure(WorktreeHookEvent::PostRemove, "boom".to_string(), 10);
        assert!(!o.success);
        assert!(o.output.is_empty());
        assert_eq!(o.error, Some("boom".to_string()));
    }

    #[test]
    fn hook_outcome_serde_roundtrip() {
        let o = HookOutcome::success(WorktreeHookEvent::PreRemove, "out".to_string(), 42);
        let json = serde_json::to_string(&o).unwrap();
        let de: HookOutcome = serde_json::from_str(&json).unwrap();
        assert_eq!(o.success, de.success);
        assert_eq!(o.output, de.output);
        assert_eq!(o.event, de.event);
    }

    // -- NoOpWorktreeHooks --

    #[test]
    fn no_op_returns_empty() {
        let hooks = NoOpWorktreeHooks;
        let ctx = HookContext {
            event: WorktreeHookEvent::PreCreate,
            worktree_id: None,
            worktree_name: None,
            worktree_path: None,
            parent_path: None,
            worktree_type: None,
            branch: None,
        };
        let results = hooks.run(WorktreeHookEvent::PreCreate, &ctx).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn no_op_default() {
        let hooks = NoOpWorktreeHooks::default();
        let ctx = HookContext {
            event: WorktreeHookEvent::PostCreate,
            worktree_id: None,
            worktree_name: None,
            worktree_path: None,
            parent_path: None,
            worktree_type: None,
            branch: None,
        };
        assert!(hooks.run(WorktreeHookEvent::PostCreate, &ctx).unwrap().is_empty());
    }

    // -- ShellWorktreeHooks --

    #[test]
    fn shell_hooks_empty_dir_returns_empty() {
        let dir = tempfile::TempDir::new().unwrap();
        let hooks = ShellWorktreeHooks::new(dir.path().to_path_buf());
        let ctx = HookContext {
            event: WorktreeHookEvent::PreCreate,
            worktree_id: None,
            worktree_name: None,
            worktree_path: None,
            parent_path: None,
            worktree_type: None,
            branch: None,
        };
        let results = hooks.run(WorktreeHookEvent::PreCreate, &ctx).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn shell_hooks_nonexistent_dir_returns_empty() {
        let hooks = ShellWorktreeHooks::new(PathBuf::from("/nonexistent/hooks/dir"));
        let ctx = HookContext {
            event: WorktreeHookEvent::PreCreate,
            worktree_id: None,
            worktree_name: None,
            worktree_path: None,
            parent_path: None,
            worktree_type: None,
            branch: None,
        };
        let results = hooks.run(WorktreeHookEvent::PreCreate, &ctx).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn shell_hooks_discovers_matching_scripts() {
        let dir = tempfile::TempDir::new().unwrap();
        // Create a matching script
        let script = dir.path().join("pre-create-setup");
        std::fs::write(&script, "#!/bin/bash\necho 'created'").unwrap();

        let hooks = ShellWorktreeHooks::new(dir.path().to_path_buf());
        let ctx = HookContext {
            event: WorktreeHookEvent::PreCreate,
            worktree_id: None,
            worktree_name: Some("test-wt".to_string()),
            worktree_path: None,
            parent_path: None,
            worktree_type: None,
            branch: None,
        };
        let results = hooks.run(WorktreeHookEvent::PreCreate, &ctx).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn shell_hooks_ignores_non_matching_scripts() {
        let dir = tempfile::TempDir::new().unwrap();
        let script = dir.path().join("post-remove-cleanup");
        std::fs::write(&script, "#!/bin/bash\necho 'done'").unwrap();

        let hooks = ShellWorktreeHooks::new(dir.path().to_path_buf());
        let ctx = HookContext {
            event: WorktreeHookEvent::PreCreate,
            worktree_id: None,
            worktree_name: None,
            worktree_path: None,
            parent_path: None,
            worktree_type: None,
            branch: None,
        };
        let results = hooks.run(WorktreeHookEvent::PreCreate, &ctx).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn shell_hooks_timeout_builder() {
        let dir = tempfile::TempDir::new().unwrap();
        let hooks = ShellWorktreeHooks::new(dir.path().to_path_buf()).timeout(5000);
        assert_eq!(hooks.timeout_ms, 5000);
    }
}
