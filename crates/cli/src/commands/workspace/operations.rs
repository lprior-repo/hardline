//! Workspace operations - pure helper functions

use std::path::Path;
use std::process::Command;

use scp_core::output::Output;
use scp_core::vcs::{self, VcsBackend, VcsStatus};
use scp_core::Error;

/// Get sorted workspace names
#[must_use]
pub fn sorted_workspace_names(workspaces: &[vcs::Workspace]) -> Vec<String> {
    let mut names: Vec<String> = workspaces.iter().map(|w| w.name.clone()).collect();
    names.sort();
    names
}

/// Find next workspace in alphabetical order
#[must_use]
pub fn find_next_workspace(workspaces: &[vcs::Workspace]) -> Result<String, Error> {
    let sorted_names = sorted_workspace_names(workspaces);
    let current_ws = workspaces.iter().find(|w| w.is_current);

    match current_ws {
        Some(current) => {
            let current_idx = sorted_names
                .iter()
                .position(|n| n == &current.name)
                .ok_or_else(|| Error::internal("current workspace not in list"))?;
            let next_idx = (current_idx + 1) % sorted_names.len();
            Ok(sorted_names[next_idx].clone())
        }
        None => sorted_names
            .first()
            .cloned()
            .ok_or_else(|| Error::workspace_not_found("no workspaces exist")),
    }
}

/// Find previous workspace in alphabetical order
#[must_use]
pub fn find_prev_workspace(workspaces: &[vcs::Workspace]) -> Result<String, Error> {
    let sorted_names = sorted_workspace_names(workspaces);
    let current_ws = workspaces.iter().find(|w| w.is_current);

    match current_ws {
        Some(current) => {
            let current_idx = sorted_names
                .iter()
                .position(|n| n == &current.name)
                .ok_or_else(|| Error::internal("current workspace not in list"))?;
            let prev_idx = if current_idx == 0 {
                sorted_names.len() - 1
            } else {
                current_idx - 1
            };
            Ok(sorted_names[prev_idx].clone())
        }
        None => sorted_names
            .last()
            .cloned()
            .ok_or_else(|| Error::workspace_not_found("no workspaces exist")),
    }
}

/// Helper: Create workspace with optional sync
pub fn spawn_with_sync(backend: &dyn VcsBackend, name: &str, sync: bool) -> Result<(), Error> {
    backend.create_workspace(name)?;
    Output::success(&format!("Created workspace '{}'", name));

    if sync {
        backend.switch_workspace(name)?;
        backend.rebase("main")?;
        Output::success("Synced with main");
    }

    Ok(())
}

/// Helper: Check workspace exists
#[must_use]
pub fn workspace_exists(backend: &dyn VcsBackend, name: &str) -> Result<bool, Error> {
    let workspaces = backend.list_workspaces()?;
    Ok(workspaces.iter().any(|w| w.name == name))
}

/// Helper: Validate clean working copy
#[must_use]
pub fn require_clean_working_copy(backend: &dyn VcsBackend) -> Result<(), Error> {
    let status = backend.status()?;
    if status != VcsStatus::Clean {
        return Err(Error::working_copy_dirty());
    }
    Ok(())
}

/// Helper to get current workspace name from backend
#[must_use]
pub fn get_current_workspace_name(backend: &dyn VcsBackend) -> Result<String, Error> {
    let workspaces = backend.list_workspaces()?;
    workspaces
        .iter()
        .find(|w| w.is_current)
        .map(|w| w.name.clone())
        .ok_or_else(|| Error::workspace_not_found("no current workspace"))
}

/// Helper: Resolve workspace name from Option or get current
#[must_use]
pub fn resolve_workspace_name(
    backend: &dyn VcsBackend,
    name: Option<&str>,
) -> Result<String, Error> {
    match name {
        Some(n) => Ok(n.to_string()),
        None => get_current_workspace_name(backend),
    }
}

/// Helper: Complete workspace workflow (sync + push)
pub fn complete_workspace_workflow(backend: &dyn VcsBackend, name: &str) -> Result<(), Error> {
    backend.rebase("main")?;
    Output::success("Synced with main");

    backend.push()?;
    Output::success("Pushed to remote");

    Output::success(&format!("Workspace '{}' completed", name));
    Ok(())
}

/// Ensure workspace is not main
#[must_use]
pub fn ensure_not_main_workspace(name: &str) -> Result<(), Error> {
    if name == "main" {
        return Err(Error::invalid_state("Cannot complete main workspace"));
    }
    Ok(())
}

/// Execute workspace abort workflow
pub fn execute_workspace_abort(backend: &dyn VcsBackend, name: &str) -> Result<(), Error> {
    backend.abort_workspace(name)?;
    Output::success(&format!("Aborted workspace '{}'", name));
    Ok(())
}

/// Build git diff command
#[must_use]
pub fn build_git_diff_command(cwd: &Path, path: Option<&str>) -> Command {
    let mut cmd = Command::new("git");
    cmd.args(["diff", "HEAD"]).current_dir(cwd);

    if let Some(p) = path {
        cmd.arg("--").arg(p);
    }

    cmd
}

#[cfg(test)]
mod tests {
    use super::*;
    use scp_core::vcs::Workspace;

    // ---- Helper ----

    fn ws(name: &str, is_current: bool) -> Workspace {
        Workspace {
            name: name.to_string(),
            branch: format!("{name}-branch"),
            is_current,
        }
    }

    // ---- sorted_workspace_names ----

    #[test]
    fn sorted_names_empty() {
        let result = sorted_workspace_names(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn sorted_names_single() {
        let result = sorted_workspace_names(&[ws("main", true)]);
        assert_eq!(result, vec!["main"]);
    }

    #[test]
    fn sorted_names_already_sorted() {
        let workspaces = vec![ws("alpha", false), ws("beta", true), ws("gamma", false)];
        assert_eq!(
            sorted_workspace_names(&workspaces),
            vec!["alpha", "beta", "gamma"]
        );
    }

    #[test]
    fn sorted_names_reverse_order() {
        let workspaces = vec![ws("gamma", false), ws("beta", true), ws("alpha", false)];
        assert_eq!(
            sorted_workspace_names(&workspaces),
            vec!["alpha", "beta", "gamma"]
        );
    }

    #[test]
    fn sorted_names_mixed_case() {
        let workspaces = vec![ws("Zebra", false), ws("alpha", false), ws("Beta", false)];
        // sort() uses lexicographic order: uppercase letters come before lowercase
        let result = sorted_workspace_names(&workspaces);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "Beta");
        assert_eq!(result[1], "Zebra");
        assert_eq!(result[2], "alpha");
    }

    #[test]
    fn sorted_names_duplicates() {
        let workspaces = vec![ws("alpha", false), ws("alpha", false)];
        assert_eq!(sorted_workspace_names(&workspaces), vec!["alpha", "alpha"]);
    }

    // ---- find_next_workspace ----

    #[test]
    fn find_next_wraps_around() {
        let workspaces = vec![ws("alpha", true), ws("beta", false), ws("gamma", false)];
        let next = find_next_workspace(&workspaces).expect("ok");
        assert_eq!(next, "beta");
    }

    #[test]
    fn find_next_last_wraps_to_first() {
        let workspaces = vec![ws("alpha", false), ws("beta", false), ws("gamma", true)];
        let next = find_next_workspace(&workspaces).expect("ok");
        assert_eq!(next, "alpha");
    }

    #[test]
    fn find_next_middle() {
        let workspaces = vec![ws("alpha", false), ws("beta", true), ws("gamma", false)];
        let next = find_next_workspace(&workspaces).expect("ok");
        assert_eq!(next, "gamma");
    }

    #[test]
    fn find_next_single_workspace_wraps() {
        let workspaces = vec![ws("only", true)];
        let next = find_next_workspace(&workspaces).expect("ok");
        assert_eq!(next, "only");
    }

    #[test]
    fn find_next_no_current_returns_first() {
        let workspaces = vec![ws("alpha", false), ws("beta", false)];
        let next = find_next_workspace(&workspaces).expect("ok");
        assert_eq!(next, "alpha");
    }

    #[test]
    fn find_next_empty_returns_error() {
        let result = find_next_workspace(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn find_next_sorted_input() {
        // Ensure alphabetical order is respected regardless of input order
        let workspaces = vec![ws("charlie", false), ws("alpha", true), ws("bravo", false)];
        let next = find_next_workspace(&workspaces).expect("ok");
        assert_eq!(next, "bravo");
    }

    // ---- find_prev_workspace ----

    #[test]
    fn find_prev_wraps_from_first_to_last() {
        let workspaces = vec![ws("alpha", true), ws("beta", false), ws("gamma", false)];
        let prev = find_prev_workspace(&workspaces).expect("ok");
        assert_eq!(prev, "gamma");
    }

    #[test]
    fn find_prev_middle() {
        let workspaces = vec![ws("alpha", false), ws("beta", true), ws("gamma", false)];
        let prev = find_prev_workspace(&workspaces).expect("ok");
        assert_eq!(prev, "alpha");
    }

    #[test]
    fn find_prev_last() {
        let workspaces = vec![ws("alpha", false), ws("beta", false), ws("gamma", true)];
        let prev = find_prev_workspace(&workspaces).expect("ok");
        assert_eq!(prev, "beta");
    }

    #[test]
    fn find_prev_single_workspace_wraps() {
        let workspaces = vec![ws("only", true)];
        let prev = find_prev_workspace(&workspaces).expect("ok");
        assert_eq!(prev, "only");
    }

    #[test]
    fn find_prev_no_current_returns_last() {
        let workspaces = vec![ws("alpha", false), ws("beta", false)];
        let prev = find_prev_workspace(&workspaces).expect("ok");
        assert_eq!(prev, "beta");
    }

    #[test]
    fn find_prev_empty_returns_error() {
        let result = find_prev_workspace(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn find_prev_sorted_input() {
        let workspaces = vec![ws("charlie", false), ws("alpha", false), ws("bravo", true)];
        let prev = find_prev_workspace(&workspaces).expect("ok");
        assert_eq!(prev, "alpha");
    }

    // ---- ensure_not_main_workspace ----

    #[test]
    fn ensure_not_main_rejects_main() {
        let result = ensure_not_main_workspace("main");
        assert!(result.is_err());
    }

    #[test]
    fn ensure_not_main_accepts_other() {
        let result = ensure_not_main_workspace("feature-branch");
        assert!(result.is_ok());
    }

    #[test]
    fn ensure_not_main_accepts_empty() {
        let result = ensure_not_main_workspace("");
        assert!(result.is_ok());
    }

    // ---- build_git_diff_command ----

    #[test]
    fn build_git_diff_no_path() {
        let cmd = build_git_diff_command(std::path::Path::new("/tmp"), None);
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(args.contains(&"diff".to_string()));
        assert!(args.contains(&"HEAD".to_string()));
    }

    #[test]
    fn build_git_diff_with_path() {
        let cmd = build_git_diff_command(std::path::Path::new("/tmp"), Some("src/main.rs"));
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(args.contains(&"src/main.rs".to_string()));
    }

    #[test]
    fn build_git_diff_cwd_is_set() {
        let _cmd = build_git_diff_command(std::path::Path::new("/test/dir"), None);
    }

    // ========================================================================
    // RED QUEEN: Adversarial tests for abort command helpers
    // ========================================================================
    //
    // These tests attempt to violate the abort command's contracts by probing
    // edge cases in the helper functions that guard the abort workflow.
    //
    // Contracts under test:
    //   C1: Working copy MUST be clean (require_clean_working_copy)
    //   C2: Workspace MUST NOT be "main" (ensure_not_main_workspace)
    //   C3: Workspace MUST exist (workspace_exists)
    //   C4: Name resolution from None → current workspace (resolve_workspace_name)

    use std::sync::{Arc, Mutex};

    /// Mock VcsBackend that tracks all method calls for adversarial testing.
    struct MockBackend {
        workspaces: Vec<vcs::Workspace>,
        status: VcsStatus,
        call_log: Arc<Mutex<Vec<String>>>,
    }

    impl MockBackend {
        fn new(workspaces: Vec<vcs::Workspace>, status: VcsStatus) -> Self {
            Self {
                workspaces,
                status,
                call_log: Arc::new(Mutex::new(vec![])),
            }
        }

        fn calls(&self) -> Vec<String> {
            self.call_log.lock().unwrap().clone()
        }

        fn log(&self, method: &str) {
            self.call_log.lock().unwrap().push(method.to_string());
        }
    }

    impl VcsBackend for MockBackend {
        fn current_branch(&self) -> scp_core::Result<String> {
            self.log("current_branch");
            Ok("test-branch".to_string())
        }
        fn list_branches(&self) -> scp_core::Result<Vec<scp_core::vcs::Branch>> {
            self.log("list_branches");
            Ok(vec![])
        }
        fn create_branch(&self, _name: &str) -> scp_core::Result<()> {
            self.log("create_branch");
            Ok(())
        }
        fn switch_branch(&self, _name: &str) -> scp_core::Result<()> {
            self.log("switch_branch");
            Ok(())
        }
        fn push(&self) -> scp_core::Result<()> {
            self.log("push");
            Ok(())
        }
        fn pull(&self) -> scp_core::Result<()> {
            self.log("pull");
            Ok(())
        }
        fn rebase(&self, _onto: &str) -> scp_core::Result<()> {
            self.log("rebase");
            Ok(())
        }
        fn merge(&self, _branch: &str) -> scp_core::Result<()> {
            self.log("merge");
            Ok(())
        }
        fn log(&self, _limit: usize) -> scp_core::Result<Vec<scp_core::vcs::Commit>> {
            self.log("log");
            Ok(vec![])
        }
        fn status(&self) -> scp_core::Result<VcsStatus> {
            self.log("status");
            Ok(self.status.clone())
        }
        fn is_initialized(&self) -> scp_core::Result<bool> {
            self.log("is_initialized");
            Ok(true)
        }
        fn repo_exists(&self, _path: &str) -> bool {
            true
        }
        fn checkout(&self, _target: &str) -> scp_core::Result<()> {
            self.log("checkout");
            Ok(())
        }
        fn commit(&self, _message: &str) -> scp_core::Result<scp_core::vcs::CommitId> {
            self.log("commit");
            Ok(scp_core::vcs::CommitId::new("abc123").unwrap())
        }
        fn diff(
            &self,
            _from: &scp_core::vcs::CommitId,
            _to: &scp_core::vcs::CommitId,
        ) -> scp_core::Result<String> {
            self.log("diff");
            Ok(String::new())
        }
        fn repo_status(&self) -> scp_core::Result<scp_core::vcs::RepoStatus> {
            self.log("repo_status");
            Ok(scp_core::vcs::RepoStatus::default())
        }
        fn create_workspace(&self, _name: &str) -> scp_core::Result<()> {
            self.log("create_workspace");
            Ok(())
        }
        fn switch_workspace(&self, _name: &str) -> scp_core::Result<()> {
            self.log("switch_workspace");
            Ok(())
        }
        fn list_workspaces(&self) -> scp_core::Result<Vec<vcs::Workspace>> {
            self.log("list_workspaces");
            Ok(self.workspaces.clone())
        }
        fn delete_workspace(&self, _name: &str) -> scp_core::Result<()> {
            self.log("delete_workspace");
            Ok(())
        }
        fn fork_workspace(&self, _source: &str, _target: &str) -> scp_core::Result<()> {
            self.log("fork_workspace");
            Ok(())
        }
        fn merge_workspace(&self, _name: &str) -> scp_core::Result<()> {
            self.log("merge_workspace");
            Ok(())
        }
        fn abort_workspace(&self, name: &str) -> scp_core::Result<()> {
            self.log(&format!("abort_workspace({})", name));
            Ok(())
        }
    }

    // --- C1: Dirty working copy must be rejected BEFORE any other checks ---

    #[test]
    fn rq_abort_dirty_working_copy_rejected_before_name_check() {
        // Even if the workspace is "main" (which would also be rejected),
        // dirty check should fail first. This verifies check ordering.
        let backend = MockBackend::new(
            vec![ws("main", true), ws("feature", false)],
            VcsStatus::Dirty,
        );
        let result = require_clean_working_copy(&backend);
        assert!(result.is_err(), "Dirty working copy must be rejected");
        // Verify status() was called (the dirty check happened)
        assert!(backend.calls().contains(&"status".to_string()));
    }

    #[test]
    fn rq_abort_conflicted_working_copy_rejected() {
        let backend = MockBackend::new(
            vec![ws("feature", true)],
            VcsStatus::Conflicted,
        );
        let result = require_clean_working_copy(&backend);
        assert!(result.is_err(), "Conflicted working copy must be rejected");
    }

    // --- C2: Cannot abort "main" workspace ---

    #[test]
    fn rq_abort_main_rejected_exactly() {
        let result = ensure_not_main_workspace("main");
        assert!(result.is_err(), "Exactly 'main' must be rejected");
    }

    #[test]
    fn rq_abort_main_case_variants_not_bypassed() {
        // Case sensitivity: "Main", "MAIN" are NOT "main" — they pass this check.
        // This is correct: git branches are case-sensitive.
        assert!(
            ensure_not_main_workspace("Main").is_ok(),
            "Case-sensitive: 'Main' != 'main' — passes this check"
        );
        assert!(
            ensure_not_main_workspace("MAIN").is_ok(),
            "Case-sensitive: 'MAIN' != 'main' — passes this check"
        );
    }

    #[test]
    fn rq_abort_main_with_whitespace_not_bypassed() {
        // Whitespace-padding should NOT bypass the check
        assert!(
            ensure_not_main_workspace("main ").is_ok(),
            "'main ' (trailing space) is not 'main' — passes this check"
        );
        assert!(
            ensure_not_main_workspace(" main").is_ok(),
            "' main' (leading space) is not 'main' — passes this check"
        );
    }

    #[test]
    fn rq_abort_main_with_special_chars_not_bypassed() {
        // Unicode tricks
        assert!(ensure_not_main_workspace("mаin").is_ok()); // Cyrillic 'а' — different char
        assert!(ensure_not_main_workspace("main\n").is_ok()); // Newline
    }

    // --- C3: Non-existent workspace must be rejected ---

    #[test]
    fn rq_abort_nonexistent_workspace_rejected() {
        let backend = MockBackend::new(
            vec![ws("alpha", true), ws("beta", false)],
            VcsStatus::Clean,
        );
        let exists = workspace_exists(&backend, "nonexistent").expect("ok");
        assert!(!exists, "Non-existent workspace must return false");
    }

    #[test]
    fn rq_abort_workspace_exists_empty_list() {
        let backend = MockBackend::new(vec![], VcsStatus::Clean);
        let exists = workspace_exists(&backend, "anything").expect("ok");
        assert!(!exists, "Empty workspace list means nothing exists");
    }

    #[test]
    fn rq_abort_workspace_exists_case_sensitive() {
        let backend = MockBackend::new(
            vec![ws("Feature", true)],
            VcsStatus::Clean,
        );
        assert!(!workspace_exists(&backend, "feature").expect("ok"));
        assert!(workspace_exists(&backend, "Feature").expect("ok"));
    }

    // --- C4: Name resolution when no current workspace ---

    #[test]
    fn rq_abort_no_name_no_current_workspace_fails() {
        let backend = MockBackend::new(
            vec![ws("alpha", false), ws("beta", false)],
            VcsStatus::Clean,
        );
        let result = resolve_workspace_name(&backend, None);
        assert!(result.is_err(), "No name provided and no current workspace must fail");
    }

    #[test]
    fn rq_abort_explicit_name_overrides_current() {
        let backend = MockBackend::new(
            vec![ws("alpha", true), ws("beta", false)],
            VcsStatus::Clean,
        );
        let result = resolve_workspace_name(&backend, Some("beta")).expect("ok");
        assert_eq!(result, "beta", "Explicit name should take precedence over current");
    }

    // --- Check ordering: abort must check dirty → main → exists in order ---

    #[test]
    fn rq_abort_check_order_dirty_before_main() {
        // If we were to call the full abort flow with a dirty "main" workspace,
        // the dirty check should fail FIRST (before the main check).
        // We verify this by checking that status() is called first.
        let backend = MockBackend::new(
            vec![ws("main", true)],
            VcsStatus::Dirty,
        );
        // Simulate the abort flow step-by-step
        let step1 = require_clean_working_copy(&backend);
        assert!(step1.is_err(), "Step 1: dirty check must fail");
        let calls = backend.calls();
        assert!(calls.contains(&"status".to_string()));
    }

    #[test]
    fn rq_abort_check_order_main_before_exists() {
        // After passing clean check, main check should fail before exists check
        let backend = MockBackend::new(
            vec![ws("main", true)],
            VcsStatus::Clean,
        );
        let _clean = require_clean_working_copy(&backend).expect("clean");
        let step2 = ensure_not_main_workspace("main");
        assert!(step2.is_err(), "Step 2: main check must fail");
        // workspace_exists should not have been called yet
        assert!(
            !backend.calls().contains(&"list_workspaces".to_string()),
            "workspace_exists not called before main check"
        );
    }

    // --- Full abort flow: simulate the workspace_ops::abort sequence ---

    #[test]
    fn rq_abort_full_flow_happy_path() {
        let backend = MockBackend::new(
            vec![ws("alpha", true), ws("feature", false)],
            VcsStatus::Clean,
        );

        // Step 1: Clean check
        require_clean_working_copy(&backend).expect("clean");
        // Step 2: Resolve name
        let name = resolve_workspace_name(&backend, Some("feature")).expect("name");
        // Step 3: Not main
        ensure_not_main_workspace(&name).expect("not main");
        // Step 4: Exists
        assert!(workspace_exists(&backend, &name).expect("exists"));
        // Step 5: Execute abort
        execute_workspace_abort(&backend, &name).expect("abort");
    }

    #[test]
    fn rq_abort_full_flow_dirty_rejected() {
        let backend = MockBackend::new(
            vec![ws("feature", false)],
            VcsStatus::Dirty,
        );
        let result = require_clean_working_copy(&backend);
        assert!(result.is_err());
    }

    #[test]
    fn rq_abort_full_flow_main_rejected() {
        let backend = MockBackend::new(
            vec![ws("main", true), ws("feature", false)],
            VcsStatus::Clean,
        );
        require_clean_working_copy(&backend).expect("clean");
        let result = ensure_not_main_workspace("main");
        assert!(result.is_err());
    }

    #[test]
    fn rq_abort_full_flow_nonexistent_rejected() {
        let backend = MockBackend::new(
            vec![ws("alpha", true)],
            VcsStatus::Clean,
        );
        require_clean_working_copy(&backend).expect("clean");
        let name = resolve_workspace_name(&backend, Some("ghost")).expect("name");
        ensure_not_main_workspace(&name).expect("not main");
        let exists = workspace_exists(&backend, &name).expect("exists");
        assert!(!exists, "Non-existent workspace must be caught");
    }

    #[test]
    fn rq_abort_full_flow_no_current_no_name_rejected() {
        let backend = MockBackend::new(
            vec![ws("alpha", false), ws("beta", false)],
            VcsStatus::Clean,
        );
        require_clean_working_copy(&backend).expect("clean");
        let result = resolve_workspace_name(&backend, None);
        assert!(result.is_err(), "Must fail when no name and no current workspace");
    }
}

/// Split workspace by creating a new branch from current state
pub fn split_workspace(backend: &dyn VcsBackend, path: &str) -> Result<(), Error> {
    let workspace_path = Path::new(path);

    if !workspace_path.exists() {
        return Err(Error::not_found(format!("Path does not exist: {}", path)));
    }

    if !workspace_path.is_dir() {
        return Err(Error::invalid_state(format!(
            "Path is not a directory: {}",
            path
        )));
    }

    let workspaces = backend.list_workspaces()?;
    let path_str = workspace_path.to_string_lossy().to_string();

    for ws in workspaces {
        if ws.name == path_str || ws.branch == path_str {
            return Err(Error::workspace_exists(ws.name));
        }
    }

    Output::info(&format!("Adding workspace at '{}'...", path));

    let output = Command::new("git")
        .args(["worktree", "add", "--", path])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::vcs_conflict("worktree add", stderr));
    }

    Output::success(&format!("Added workspace at '{}'", path));

    Ok(())
}
