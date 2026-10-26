//! Workspace operations - pure helper functions

use std::{path::Path, process::Command};

use scp_core::{
    output::Output,
    vcs::{self, VcsBackend, VcsStatus},
    Error,
};

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
///
/// Sync rebases the NEW worktree's branch onto main from inside the new
/// worktree. The old code switched the caller's workspace to the new one
/// and rebased there — switching the caller's checkout is the wrong
/// worktree anyway, since the new branch is already checked out in its
/// own worktree (`switch_workspace` now resolves a target path for the
/// CLI to print, it does not move a checkout).
pub fn spawn_with_sync(
    backend: &dyn VcsBackend,
    name: &str,
    sync: bool,
    parent: &Path,
) -> Result<(), Error> {
    backend.create_workspace(name)?;
    if !Output::json_output() {
        Output::success(&format!("Created workspace '{name}'"));
    }

    if sync {
        let worktree_path = parent.join(name);
        let output = Command::new("git")
            .args([
                "-C",
                &worktree_path.to_string_lossy(),
                "rebase",
                "main",
            ])
            .output()?;
        if !output.status.success() {
            return Err(Error::vcs_conflict(
                "rebase",
                String::from_utf8_lossy(&output.stderr),
            ));
        }
        if !Output::json_output() {
            Output::success("Synced with main");
        }
    }

    if Output::json_output() {
        Output::json(&serde_json::json!({
            "workspace": name,
            "result": "created",
            "synced": sync,
        }));
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

/// Whether one `git status --porcelain` line is just a registered workspace's
/// own worktree directory showing up as untracked in the current repo.
///
/// Worktrees are created as subdirectories of the repo root, so from the
/// main worktree `git status --porcelain` always reports `?? <name>/` for
/// every linked worktree. That is not dirty state: it is the workspace
/// itself. A line counts as worktree-owned when its path equals a
/// workspace name, lies under one, or is an ancestor of one: git collapses
/// a fully-untracked directory to its highest untracked ancestor, so a
/// worktree at `.scp/workspaces/ws1` shows up as `?? .scp/workspaces/`
/// (comparing only the first segment, `.scp`, missed that and self-blocked
/// every switch for the `hd work` layout). Segment boundaries are exact,
/// so a file merely sharing a workspace name's prefix (e.g. `ws1file`)
/// still counts as dirty.
#[must_use]
pub fn porcelain_line_is_worktree_dir(line: &str, workspace_names: &[String]) -> bool {
    let Some(path_field) = line.get(3..) else {
        return false;
    };
    let path_field = path_field.trim_matches('"').trim_end_matches('/');
    // Rename/copy lines report `old -> new`; the old side is what git
    // reports for a path, and a worktree dir never appears on either side
    // of a rename, so the old side is the right comparison either way.
    let path = match path_field.split(" -> ").next() {
        Some(part) => part,
        None => path_field,
    };
    workspace_names.iter().any(|name| {
        !name.is_empty()
            && (path == name
                || path.starts_with(&format!("{name}/"))
                || name.starts_with(&format!("{path}/")))
    })
}

/// Whether `git status --porcelain` output contains any dirty state other
/// than registered worktree directories.
#[must_use]
pub fn porcelain_has_real_dirty_state(porcelain: &str, workspace_names: &[String]) -> bool {
    porcelain
        .lines()
        .any(|line| !line.is_empty() && !porcelain_line_is_worktree_dir(line, workspace_names))
}

/// Helper: Validate clean working copy, ignoring registered workspace
/// worktree directories.
///
/// `workspace switch` runs from the main worktree, where every forked
/// worktree directory is untracked by construction. Treating those
/// directories as dirty state self-blocks every switch, so the guard here
/// scopes to real dirty state: modified, staged, conflicting, or untracked
/// paths that are not a registered workspace's directory. If the porcelain
/// query itself fails, fall back to the strict backend status so a failure
/// to inspect never silently passes the guard.
pub fn require_clean_working_copy_excluding_worktrees(
    backend: &dyn VcsBackend,
) -> Result<(), Error> {
    let workspaces = backend.list_workspaces()?;
    let names: Vec<String> = workspaces.into_iter().map(|ws| ws.name).collect();
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .output();
    match output {
        Ok(output) if output.status.success() => {
            let porcelain = String::from_utf8_lossy(&output.stdout);
            if porcelain_has_real_dirty_state(&porcelain, &names) {
                return Err(Error::working_copy_dirty());
            }
            Ok(())
        }
        _ => require_clean_working_copy(backend),
    }
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
    name.map_or_else(
        || get_current_workspace_name(backend),
        |n| Ok(n.to_string()),
    )
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
    if Output::json_output() {
        Output::json(&serde_json::json!({
            "workspace": name,
            "result": "aborted",
        }));
    } else {
        Output::success(&format!("Aborted workspace '{}'", name));
    }
    Ok(())
}

/// Build git diff command
#[must_use]
pub fn build_git_diff_command(cwd: &Path, path: Option<&str>) -> Command {
    let mut cmd = Command::new("git");
    cmd.args(["diff", "HEAD"]).current_dir(cwd);

    if let Some(p) = path {
        cmd.arg(p);
    }

    cmd
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use scp_core::vcs::Workspace;

    use super::*;

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

    // ---- porcelain worktree-dir filtering (workspace switch guard) ----

    fn ws_names(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| n.to_string()).collect()
    }

    #[test]
    fn porcelain_empty_is_clean() {
        assert!(!porcelain_has_real_dirty_state("", &ws_names(&["ws1"])));
    }

    #[test]
    fn porcelain_worktree_dir_only_is_not_dirty() {
        assert!(!porcelain_has_real_dirty_state(
            "?? ws1/\n",
            &ws_names(&["ws1"])
        ));
    }

    #[test]
    fn porcelain_file_under_worktree_dir_is_not_dirty() {
        assert!(!porcelain_has_real_dirty_state(
            "?? ws1/nested.txt\n",
            &ws_names(&["ws1"])
        ));
    }

    #[test]
    fn porcelain_real_modified_file_is_dirty() {
        assert!(porcelain_has_real_dirty_state(
            " M src/main.rs\n",
            &ws_names(&["ws1"])
        ));
    }

    #[test]
    fn porcelain_conflict_is_dirty() {
        assert!(porcelain_has_real_dirty_state(
            "UU src/main.rs\n",
            &ws_names(&["ws1"])
        ));
    }

    #[test]
    fn porcelain_other_untracked_dir_is_dirty() {
        assert!(porcelain_has_real_dirty_state(
            "?? other/\n",
            &ws_names(&["ws1"])
        ));
    }

    #[test]
    fn porcelain_name_prefix_only_is_dirty() {
        // A file sharing the workspace name's prefix is not the worktree dir.
        assert!(porcelain_has_real_dirty_state(
            "?? ws1file.txt\n",
            &ws_names(&["ws1"])
        ));
    }

    #[test]
    fn porcelain_worktree_dir_plus_real_change_is_dirty() {
        assert!(porcelain_has_real_dirty_state(
            "?? ws1/\n M src/main.rs\n",
            &ws_names(&["ws1"])
        ));
    }

    #[test]
    fn porcelain_nested_worktree_collapsed_parent_is_not_dirty() {
        // git collapses the `hd work` layout's untracked worktree to its
        // parent dir; that parent must not self-block a switch.
        assert!(!porcelain_has_real_dirty_state(
            "?? .scp/workspaces/\n",
            &ws_names(&[".scp/workspaces/ws1"])
        ));
    }

    #[test]
    fn porcelain_nested_worktree_file_is_not_dirty() {
        assert!(!porcelain_has_real_dirty_state(
            "?? .scp/workspaces/ws1/file.txt\n",
            &ws_names(&[".scp/workspaces/ws1"])
        ));
    }

    #[test]
    fn porcelain_sibling_under_worktree_root_is_dirty() {
        assert!(porcelain_has_real_dirty_state(
            "?? .scp/other.txt\n",
            &ws_names(&[".scp/workspaces/ws1"])
        ));
    }

    #[test]
    fn porcelain_nested_prefix_only_is_dirty() {
        assert!(porcelain_has_real_dirty_state(
            "?? .scp/workspaces-ws1/\n",
            &ws_names(&[".scp/workspaces/ws1"])
        ));
    }

    #[test]
    fn porcelain_empty_workspace_name_ignores_nothing() {
        // The main worktree reports an empty name; it must not match anything.
        assert!(porcelain_has_real_dirty_state(
            "?? ws1/\n",
            &ws_names(&[""])
        ));
    }
}

/// Current local branch names, or `None` if git cannot list them.
/// Used only to diff branch state around a fallible `worktree add`.
fn list_git_branches() -> Option<Vec<String>> {
    let output = Command::new("git")
        .args(["branch", "--format=%(refname:short)"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect(),
    )
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

    let canonical = std::fs::canonicalize(workspace_path)
        .map_err(|e| Error::io_error(format!("Failed to resolve path: {e}")))?;
    let canonical_path = canonical.to_string_lossy();

    let workspaces = backend.list_workspaces()?;
    let path_str = canonical_path.to_string();

    for ws in workspaces {
        if ws.name == path_str || ws.branch == path_str {
            return Err(Error::workspace_exists(ws.name));
        }
    }

    // `git worktree add <path>` with no branch creates a branch named after
    // the path's basename BEFORE checking the directory out, so adding a
    // populated path fails but leaves that branch behind. Fail cleanly
    // up front instead: a non-empty directory can never become a worktree
    // by this path, and no branch is created to clean up.
    let path_non_empty = match std::fs::read_dir(workspace_path) {
        Ok(mut entries) => entries.next().is_some(),
        Err(_) => false,
    };
    if path_non_empty {
        return Err(Error::invalid_state(format!(
            "Path is not empty, cannot add workspace there: {canonical_path}"
        )));
    }

    if !Output::json_output() {
        Output::info(&format!("Adding workspace at '{}'...", canonical_path));
    }

    // Backstop for any other worktree-add failure that happens after git
    // creates the basename branch: remove exactly the branches that did
    // not exist before, so a failed add never leaves a stray branch.
    // A pre-existing branch of the same name is never touched.
    let branches_before = list_git_branches();

    let output = Command::new("git")
        .args(["worktree", "add", canonical_path.as_ref()])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if let Some(before) = branches_before {
            if let Some(after) = list_git_branches() {
                for branch in after.iter().filter(|b| !before.contains(b)) {
                    let _ = Command::new("git")
                        .args(["branch", "-D", branch])
                        .output();
                }
                let _ = Command::new("git")
                    .args(["worktree", "prune"])
                    .output();
            }
        }
        return Err(Error::vcs_conflict("worktree add", stderr));
    }

    // A worktree without a session row is invisible to `query sessions`
    // / `session-exists` while `work` refuses the name as "already
    // exists" — the same two-views contradiction import had. Register
    // the row (name and branch are the basename git just checked out)
    // before claiming success; a registration failure unwinds the
    // worktree and any branch git created, so a failed add leaves
    // nothing behind.
    let basename = match Path::new(canonical_path.as_ref())
        .file_name()
        .and_then(|name| name.to_str())
    {
        Some(name) => name.to_string(),
        None => String::new(),
    };
    if let Err(e) = register_added_session(&basename, &basename) {
        let _ = Command::new("git")
            .args(["worktree", "remove", "--force", canonical_path.as_ref()])
            .output();
        if let Some(before) = &branches_before {
            if let Some(after) = list_git_branches() {
                for branch in after.iter().filter(|b| !before.contains(b)) {
                    let _ = Command::new("git")
                        .args(["branch", "-D", branch])
                        .output();
                }
            }
        }
        let _ = Command::new("git")
            .args(["worktree", "prune"])
            .output();
        return Err(e);
    }

    if Output::json_output() {
        Output::json(&serde_json::json!({
            "workspace": canonical_path,
            "result": "added",
        }));
    } else {
        Output::success(&format!("Added workspace at '{}'", canonical_path));
    }

    Ok(())
}

/// Register an added workspace in the session database, mirroring
/// import's registration: a `Created` session named `name`, whose
/// workspace is the short name and whose branch is the branch the
/// worktree actually checks out.
fn register_added_session(name: &str, branch: &str) -> Result<(), Error> {
    use scp_core::infrastructure::database::{DatabaseConfig, DatabaseService, SqliteDatabaseService};

    let cwd = std::env::current_dir()?;
    let db_path = crate::commands::handlers::config_ports::resolve_state_db_path(&cwd)?;
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| Error::internal(format!("Failed to create runtime: {e}")))?;
    runtime.block_on(async move {
        use scp_session::{
            domain::{
                entities::{BranchState, Session},
                value_objects::{SessionName, WorkspaceId},
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

        let session_name = SessionName::parse(name)
            .map_err(|e| Error::validation_error(format!("Invalid session name: {e:?}")))?;
        let mut session = Session::create(session_name)
            .map_err(|e| Error::validation_error(format!("Invalid session: {e}")))?;
        session.workspace = Some(
            WorkspaceId::parse(name)
                .map_err(|e| Error::validation_error(format!("Invalid workspace: {e:?}")))?,
        );
        session.branch = BranchState::OnBranch {
            name: branch.to_string(),
        };
        repo.save(&session)
            .await
            .map_err(|e| Error::database(e.to_string()))?;
        Ok::<(), Error>(())
    })
}
