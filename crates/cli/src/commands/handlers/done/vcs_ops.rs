//! VCS operations for the done command handler.
//!
//! Lower-level helpers for workspace resolution, file introspection,
//! commit parsing, undo history, and the WorkspaceGitExecutor wrapper.

use std::{
    path::{Path, PathBuf},
    time::SystemTime,
};

use scp_core::{output::Output, Error, Result};

use super::{
    data::{CommitInfo, UndoEntry},
    executor::{detect_conflicts, ExecutorError, GitExecutor},
};

// ============================================================================
// Workspace Resolution
// ============================================================================

/// Resolve workspace name from option or current workspace.
pub fn resolve_workspace(
    backend: &dyn scp_core::vcs::VcsBackend,
    name: Option<&str>,
) -> Result<String> {
    match name {
        Some(n) => Ok(n.to_string()),
        None => {
            let workspaces = backend.list_workspaces()?;
            workspaces
                .iter()
                .find(|w| w.is_current)
                .map(|w| w.name.clone())
                .ok_or_else(|| Error::workspace_not_found("no current workspace"))
        }
    }
}

/// Get the workspace path for a given workspace name.
///
/// The fork/spawn layout puts a workspace at `<repo>/<name>`, so that
/// directory — when it exists — is the workspace path. The old fallback
/// returned `.git/worktrees/<name>`, which is git's metadata directory,
/// not the worktree; it only ever "worked" because the old current-flag
/// bug made every workspace look current and short-circuited to cwd.
pub fn get_workspace_path(
    cwd: &Path,
    workspace_name: &str,
    backend: &dyn scp_core::vcs::VcsBackend,
) -> Result<PathBuf> {
    let workspaces = backend.list_workspaces()?;
    let is_current = workspaces
        .iter()
        .any(|w| w.name == workspace_name && w.is_current);

    if is_current || workspace_name.is_empty() {
        Ok(cwd.to_path_buf())
    } else {
        let worktree_dir = cwd.join(workspace_name);
        if worktree_dir.is_dir() {
            Ok(worktree_dir)
        } else {
            Ok(cwd.to_path_buf())
        }
    }
}

// ============================================================================
// File & Commit Introspection
// ============================================================================

/// Get list of uncommitted files via Git status.
///
/// Uses `--porcelain`: the previous `status --no-pager` invocation is
/// not valid git (the `--no-pager` flag must precede the subcommand),
/// so it exited 129 and every dry-run failed here.
pub fn get_uncommitted_files(executor: &dyn GitExecutor) -> Result<Vec<String>> {
    let output = executor
        .run(&["status", "--porcelain"])
        .map_err(Error::from)?;

    Ok(parse_status_lines(&output))
}

/// Parse Git status output lines into file names.
///
/// Two formats are recognised, both yielding the same file lists:
/// - Porcelain (`status --porcelain`): two status columns `XY` followed
///   by a space and the path, e.g. ` M src/lib.rs`, `MM src/both.rs`,
///   `R  old.rs -> new.rs` (rename yields the new name). Untracked
///   (`??`) entries are ignored, as before.
/// - Single-letter lines (`M path`, `A path`, `D path`, `R path`) as
///   produced by jj-style status output and used by the older tests.
///   Everything else is ignored.
fn parse_status_lines(output: &str) -> Vec<String> {
    output
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            let chars: Vec<char> = trimmed.chars().collect();
            let is_status = |c: char| matches!(c, 'A' | 'M' | 'D' | 'R');
            // Porcelain: XY + space + path (e.g. "MM file", "R  old -> new").
            let path_part = if chars.len() >= 3
                && is_status(chars[0])
                && (chars[1] == ' ' || is_status(chars[1]))
                && chars[2] == ' '
            {
                trimmed.get(3..)
            // Single-letter: X + space + path (e.g. "M file").
            } else if chars.len() >= 2 && is_status(chars[0]) && chars[1] == ' ' {
                trimmed.get(2..)
            } else {
                None
            };
            let path_part = path_part?;
            // Renames report "old -> new": the merged file is the new name.
            if let Some(idx) = path_part.rfind(" -> ") {
                Some(path_part[idx + 4..].to_string())
            } else {
                // Single-letter format keeps the old token behaviour:
                // only the first whitespace-delimited token is a file.
                // Porcelain paths are taken verbatim (spaces preserved)
                // when the line came from the XY branch.
                if chars.len() >= 3 && chars[2] == ' ' {
                    Some(path_part.trim().to_string())
                } else {
                    path_part
                        .split_ascii_whitespace()
                        .next()
                        .map(std::string::ToString::to_string)
                }
            }
        })
        .collect()
}

/// Get commits that will be merged.
///
/// Commits reachable from HEAD but not from `main` — the git
/// equivalent of the workspace's unmerged changes. Git has no
/// change-id concept, so `change_id` mirrors the full commit hash.
pub fn get_commits_to_merge(executor: &dyn GitExecutor) -> Result<Vec<CommitInfo>> {
    let output = executor
        .run(&[
            "log",
            "main..HEAD",
            "--format=%H%n%H%n%s%n%cI",
        ])
        .map_err(Error::from)?;

    Ok(parse_commits_output(&output))
}

/// Parse Git log output into `CommitInfo` entries.
///
/// Each commit is 4 consecutive lines: change_id, commit_id, description,
/// timestamp.
fn parse_commits_output(output: &str) -> Vec<CommitInfo> {
    let mut commits = Vec::new();
    let mut lines = output.lines().peekable();

    while lines.peek().is_some() {
        let change_id = lines.next().map_or("", |s| s).trim().to_string();
        let commit_id = lines.next().map_or("", |s| s).trim().to_string();
        let description = lines.next().map_or("", |s| s).trim().to_string();
        let timestamp = lines.next().map_or("", |s| s).trim().to_string();

        if !change_id.is_empty() {
            commits.push(CommitInfo {
                change_id,
                commit_id,
                description,
                timestamp,
            });
        }
    }

    commits
}

/// Get potential conflicts via conflict detection.
///
/// Best-effort: logs a warning on failure but returns an empty list.
pub fn get_potential_conflicts(executor: &dyn GitExecutor) -> Vec<String> {
    match detect_conflicts(executor) {
        Ok(result) => {
            let mut conflicts = result.existing_conflicts;
            conflicts.extend(result.overlapping_files);
            conflicts
        }
        Err(e) => {
            Output::warn(&format!("Conflict detection failed: {e}"));
            Vec::new()
        }
    }
}

// ============================================================================
// Undo History
// ============================================================================

/// The main worktree of the repo containing `path_inside_repo`: the
/// parent of the common git dir. Every undo-log reader and writer must
/// agree on this path: the log lives in the main worktree, not in
/// whichever worktree the caller happens to stand in.
pub fn main_worktree_path(path_inside_repo: &Path) -> Option<PathBuf> {
    let output = std::process::Command::new("git")
        .args([
            "-C",
            &path_inside_repo.to_string_lossy(),
            "rev-parse",
            "--path-format=absolute",
            "--git-common-dir",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let common = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Path::new(&common).parent().map(Path::to_path_buf)
}

/// Main worktree of the repo the caller stands in, if resolvable.
pub fn main_worktree_path_for_cwd() -> Option<PathBuf> {
    std::env::current_dir()
        .ok()
        .and_then(|cwd| main_worktree_path(&cwd))
}

/// Undo log path for the repo the caller stands in.
pub fn undo_log_path_for_cwd() -> PathBuf {
    match std::env::current_dir() {
        Ok(cwd) => undo_log_path(&cwd),
        Err(_) => PathBuf::from(".scp/undo.log"),
    }
}

/// Path of the undo log for the repo containing `path_inside_repo`: the
/// main worktree's `.scp/undo.log`, falling back to the caller's
/// `.scp/undo.log` when git cannot resolve the repo (outside a repo the
/// readers fail with "no history" exactly as before).
pub fn undo_log_path(path_inside_repo: &Path) -> PathBuf {
    match main_worktree_path(path_inside_repo) {
        Some(main) => main.join(".scp/undo.log"),
        None => match std::env::current_dir() {
            Ok(cwd) => cwd.join(".scp/undo.log"),
            Err(_) => PathBuf::from(".scp/undo.log"),
        },
    }
}

/// Capture the undo basis for a merge before it runs: main's pre-merge
/// HEAD, the workspace branch tip, and the workspace's checked-out branch.
///
/// This runs `git -C <workspace>` directly instead of through the
/// workflow executor on purpose: the capture is bookkeeping about refs,
/// not a workflow step, and it must never perturb the executor's command
/// stream (in tests the executor is a positional mock, and a capture
/// that consumed its responses would shift every later workflow call).
/// It is also what keeps test runs from writing undo entries: outside a
/// real repo both queries fail and the caller skips the log write.
/// `rev-parse main` is repo-wide, so it reads main's HEAD from the
/// workspace even though main is checked out elsewhere.
pub fn capture_merge_undo_basis(workspace_path: &str) -> (String, String, String) {
    fn rev_parse(workspace_path: &str, args: &[&str]) -> String {
        let mut full_args = vec!["-C", workspace_path, "rev-parse"];
        full_args.extend_from_slice(args);
        match std::process::Command::new("git").args(&full_args).output() {
            Ok(output) if output.status.success() => {
                String::from_utf8_lossy(&output.stdout).trim().to_string()
            }
            _ => String::new(),
        }
    }
    (
        rev_parse(workspace_path, &["main"]),
        rev_parse(workspace_path, &["HEAD"]),
        rev_parse(workspace_path, &["--abbrev-ref", "HEAD"]),
    )
}

/// The session name an undo entry is filed under.
///
/// `done`'s workspace name is a worktree path relative to the repo root,
/// so an `hd work` session arrives as `.scp/workspaces/<name>` while the
/// user knows it as `<name>` — and `revert <name>` matches on the session
/// name. The last path component is the session name in both layouts (a
/// spawn-layout workspace `ws1` is already just `ws1`).
pub fn undo_session_name(workspace_name: &str) -> &str {
    Path::new(workspace_name)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(workspace_name)
}

/// The session name an undo entry is filed under, given the workspace
/// name `done` resolved and the workspace's checked-out branch.
///
/// Done-from-inside resolves the workspace name to the empty string (a
/// worktree's listed name is its path relative to itself), which would
/// file the entry under no name at all and make it unrevertable. In that
/// case the checked-out branch is the session name: an `hd work` session
/// checks out a branch named after itself, and so does a spawn-layout
/// workspace. A detached HEAD (`HEAD`) or an empty branch carries no
/// name, so it keeps the (empty) workspace name rather than inventing one.
pub fn undo_session_name_for<'a>(workspace_name: &'a str, workspace_branch: &'a str) -> &'a str {
    if workspace_name.is_empty() && !workspace_branch.is_empty() && workspace_branch != "HEAD" {
        workspace_branch
    } else {
        undo_session_name(workspace_name)
    }
}

/// Append one entry to the undo log at `undo_log_path`.
///
/// The caller supplies the pre-merge commit and the merged tip captured
/// BEFORE the merge: capturing them here, after the merge and after the
/// workspace was deleted, used to yield an empty pre-merge id (rev-parse
/// in a deleted worktree fails) and a post-merge HEAD, so the entry could
/// never drive a real revert. Callers must not write an entry with an
/// empty pre-merge id — it is worse than no entry, because `revert` then
/// fails inside git instead of reporting "not found".
pub fn log_undo_history(
    undo_log_path: &Path,
    session_name: &str,
    pre_merge_commit_id: &str,
    commit_id: &str,
    pushed_to_remote: bool,
) -> Result<()> {
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let undo_entry = UndoEntry {
        session_name: session_name.to_string(),
        commit_id: commit_id.to_string(),
        pre_merge_commit_id: pre_merge_commit_id.to_string(),
        timestamp,
        pushed_to_remote,
        status: "completed".to_string(),
    };

    let json = serde_json::to_string(&undo_entry).map_err(|e| Error::io_error(e.to_string()))?;

    let mut content = if undo_log_path.exists() {
        std::fs::read_to_string(undo_log_path)
            .map_err(|e| Error::io_error(format!("Failed to read undo log: {e}")))?
    } else {
        String::new()
    };
    content.push_str(&json);
    content.push('\n');

    if let Some(parent) = undo_log_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| Error::io_error(format!("Failed to create undo log directory: {e}")))?;
    }

    std::fs::write(undo_log_path, &content)
        .map_err(|e| Error::io_error(format!("Failed to write undo log: {e}")))?;

    Ok(())
}

/// Update workspace state to Merged.
///
/// Placeholder for future session state management integration.
#[allow(clippy::unused_self)]
pub const fn update_workspace_state(_workspace_name: &str) -> bool {
    false
}

// ============================================================================
// Workspace Git Executor Wrapper
// ============================================================================

/// Executor that runs Git commands in a specific workspace directory.
pub struct WorkspaceGitExecutor<'a> {
    inner: &'a dyn GitExecutor,
    workspace_path: String,
}

impl<'a> WorkspaceGitExecutor<'a> {
    pub fn new(inner: &'a dyn GitExecutor, workspace_path: &str) -> Self {
        Self {
            inner,
            workspace_path: workspace_path.to_string(),
        }
    }
}

impl GitExecutor for WorkspaceGitExecutor<'_> {
    fn run(&self, args: &[&str]) -> std::result::Result<String, ExecutorError> {
        self.inner.run_in_workspace(args, &self.workspace_path)
    }

    fn run_in_workspace(
        &self,
        args: &[&str],
        workspace_path: &str,
    ) -> std::result::Result<String, ExecutorError> {
        self.inner.run_in_workspace(args, workspace_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::handlers::done::test_support::*;

    // -----------------------------------------------------------------------
    // parse_status_lines
    // -----------------------------------------------------------------------

    // -----------------------------------------------------------------------
    // undo_session_name
    // -----------------------------------------------------------------------

    #[test]
    fn undo_session_name_strips_work_session_path() {
        assert_eq!(undo_session_name(".scp/workspaces/feat2"), "feat2");
    }

    #[test]
    fn undo_session_name_keeps_spawn_layout_name() {
        assert_eq!(undo_session_name("ws1"), "ws1");
    }

    #[test]
    fn undo_session_name_empty_stays_empty() {
        assert_eq!(undo_session_name(""), "");
    }

    #[test]
    fn undo_session_name_for_empty_uses_branch() {
        assert_eq!(undo_session_name_for("", "feat3"), "feat3");
    }

    #[test]
    fn undo_session_name_for_detached_head_keeps_empty() {
        assert_eq!(undo_session_name_for("", "HEAD"), "");
    }

    #[test]
    fn undo_session_name_for_named_ignores_branch() {
        assert_eq!(
            undo_session_name_for(".scp/workspaces/feat2", "other-branch"),
            "feat2"
        );
    }

    // -----------------------------------------------------------------------
    // parse_status_lines
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_status_lines_basic() {
        let output = "M src/lib.rs\nA src/new.rs\nD src/old.rs\nR src/renamed.rs";
        let files = parse_status_lines(output);
        assert_eq!(
            files,
            vec!["src/lib.rs", "src/new.rs", "src/old.rs", "src/renamed.rs",]
        );
    }

    #[test]
    fn test_parse_status_lines_empty() {
        let files = parse_status_lines("");
        assert!(files.is_empty());
    }

    #[test]
    fn test_parse_status_lines_with_noise() {
        let output = "M src/lib.rs\nThe working copy is clean\nA src/new.rs\n";
        let files = parse_status_lines(output);
        assert_eq!(files, vec!["src/lib.rs", "src/new.rs"]);
    }

    #[test]
    fn test_parse_status_lines_untracked_ignored() {
        let output = "? untracked.rs\nM tracked.rs\n? another.untracked";
        let files = parse_status_lines(output);
        assert_eq!(files, vec!["tracked.rs"]);
    }

    #[test]
    fn test_parse_status_lines_file_with_spaces() {
        let output = "M path with spaces/file.txt";
        let files = parse_status_lines(output);
        assert_eq!(files, vec!["path"]);
    }

    #[test]
    fn test_parse_status_lines_copied_prefix_ignored() {
        let output = "C src/copied.rs\nM src/modified.rs";
        let files = parse_status_lines(output);
        assert_eq!(files, vec!["src/modified.rs"]);
    }

    #[test]
    fn test_parse_status_lines_various_unrecognized_prefixes() {
        let output = "L src/locked.rs\nA src/added.rs\nT src/type_changed.rs\nD src/deleted.rs";
        let files = parse_status_lines(output);
        assert_eq!(files, vec!["src/added.rs", "src/deleted.rs"]);
    }

    #[test]
    fn test_parse_status_lines_lowercase_prefixes_ignored() {
        let output = "m src/lowercase.rs\nM src/uppercase.rs";
        let files = parse_status_lines(output);
        assert_eq!(files, vec!["src/uppercase.rs"]);
    }

    #[test]
    fn test_parse_status_lines_only_unrecognized_prefixes() {
        let output = "C copy.rs\nL lock.rs\nT type.rs\n? untracked.rs";
        let files = parse_status_lines(output);
        assert!(files.is_empty());
    }

    #[test]
    fn test_parse_status_lines_tab_separated() {
        let output = "M\tsrc/tab_separated.rs\nA\tsrc/other.rs";
        let files = parse_status_lines(output);
        assert!(files.is_empty(), "tab-separated lines should not be parsed");
    }

    #[test]
    fn test_parse_status_lines_porcelain() {
        let output = " M src/lib.rs\nA  src/new.rs\nD  src/old.rs\n?? untracked.rs\n";
        let files = parse_status_lines(output);
        assert_eq!(
            files,
            vec!["src/lib.rs", "src/new.rs", "src/old.rs"]
        );
    }

    #[test]
    fn test_parse_status_lines_porcelain_both_columns() {
        // Staged + unstaged modification shows as "MM".
        let output = "MM src/both.rs\nAM src/added_modified.rs\n";
        let files = parse_status_lines(output);
        assert_eq!(files, vec!["src/both.rs", "src/added_modified.rs"]);
    }

    #[test]
    fn test_parse_status_lines_porcelain_rename_yields_new_name() {
        let output = "R  src/old.rs -> src/new.rs\n";
        let files = parse_status_lines(output);
        assert_eq!(files, vec!["src/new.rs"]);
    }

    #[test]
    fn test_parse_status_lines_porcelain_path_with_spaces() {
        // Porcelain paths are taken verbatim, unlike the single-letter
        // format which only keeps the first token.
        let output = "M  path with spaces/file.txt\n";
        let files = parse_status_lines(output);
        assert_eq!(files, vec!["path with spaces/file.txt"]);
    }

    #[test]
    fn test_get_uncommitted_files_porcelain_input() {
        let executor = MockGitExecutor::new(vec![Ok(
            " M src/lib.rs\nA  src/new.rs\n?? untracked.rs\n".to_string(),
        )]);
        let files = get_uncommitted_files(&executor).expect("should succeed");
        assert_eq!(files, vec!["src/lib.rs", "src/new.rs"]);
        let calls = executor.run_calls();
        assert_eq!(calls[0].0, "status --porcelain");
    }

    #[test]
    fn test_get_commits_to_merge_uses_main_dotdot_head() {
        let executor = MockGitExecutor::new(vec![Ok(
            "abc\ndef\nfeat: x\n2024-01-15T10:00:00+00:00\n".to_string(),
        )]);
        let commits = get_commits_to_merge(&executor).expect("should succeed");
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].commit_id, "def");
        let calls = executor.run_calls();
        assert_eq!(calls[0].0, "log main..HEAD --format=%H%n%H%n%s%n%cI");
    }

    // -----------------------------------------------------------------------
    // parse_commits_output
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_commits_output_basic() {
        let output = "abc123\ndef456\nfeat: add widget\n2024-01-15 10:00:00\n";
        let commits = parse_commits_output(output);
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].change_id, "abc123");
        assert_eq!(commits[0].commit_id, "def456");
        assert_eq!(commits[0].description, "feat: add widget");
        assert_eq!(commits[0].timestamp, "2024-01-15 10:00:00");
    }

    #[test]
    fn test_parse_commits_output_multiple() {
        let output = "abc123\ndef456\nfeat: first\n2024-01-15 10:00:00\n\
                      xyz789\nghi012\nfix: second\n2024-01-15 11:00:00\n";
        let commits = parse_commits_output(output);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].change_id, "abc123");
        assert_eq!(commits[1].change_id, "xyz789");
    }

    #[test]
    fn test_parse_commits_output_empty() {
        let commits = parse_commits_output("");
        assert!(commits.is_empty());
    }

    #[test]
    fn test_parse_commits_output_incomplete_trailing() {
        let output = "abc123\ndef456\nfeat: add widget\n2024-01-15 10:00:00\n\
                      partial_id";
        let commits = parse_commits_output(output);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[1].change_id, "partial_id");
        assert!(commits[1].commit_id.is_empty());
        assert!(commits[1].description.is_empty());
        assert!(commits[1].timestamp.is_empty());
    }

    #[test]
    fn test_parse_commits_output_skips_empty_leading() {
        let output = "\n\nabc123\ndef456\ndesc\nts\n";
        let commits = parse_commits_output(output);
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].change_id, "desc");
        assert_eq!(commits[0].commit_id, "ts");
    }

    #[test]
    fn test_parse_commits_output_multiline_description() {
        let output = "abc123\ndef456\nfeat: add widget\nwith details\n2024-01-15 10:00:00\n";
        let commits = parse_commits_output(output);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].change_id, "abc123");
        assert_eq!(commits[0].description, "feat: add widget");
        assert_eq!(commits[0].timestamp, "with details");
        assert_eq!(commits[1].change_id, "2024-01-15 10:00:00");
    }

    #[test]
    fn test_parse_commits_output_single_line() {
        let output = "lone_change_id";
        let commits = parse_commits_output(output);
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].change_id, "lone_change_id");
        assert!(commits[0].commit_id.is_empty());
    }

    #[test]
    fn test_parse_commits_output_two_lines() {
        let output = "ch_id\ncm_id";
        let commits = parse_commits_output(output);
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].change_id, "ch_id");
        assert_eq!(commits[0].commit_id, "cm_id");
    }

    #[test]
    fn test_parse_commits_output_three_lines() {
        let output = "ch_id\ncm_id\ndesc line";
        let commits = parse_commits_output(output);
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].description, "desc line");
    }

    #[test]
    fn test_parse_commits_output_empty_change_id_is_skipped() {
        let output = "\nabc\ndef\nghi";
        let commits = parse_commits_output(output);
        assert!(commits.is_empty());
    }

    // -----------------------------------------------------------------------
    // WorkspaceGitExecutor delegation
    // -----------------------------------------------------------------------

    #[test]
    fn test_workspace_executor_delegates_run() {
        let mock = MockGitExecutor::new(vec![Ok("result".to_string())]);
        let ws = WorkspaceGitExecutor::new(&mock, "/tmp/workspace");

        let result = ws.run(&["status", "--porcelain"]);
        assert_eq!(result.unwrap(), "result");

        let calls = mock.run_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "status --porcelain");
        assert_eq!(calls[0].1.as_deref(), Some("/tmp/workspace"));
    }

    #[test]
    fn test_workspace_executor_delegates_run_in_workspace() {
        let mock = MockGitExecutor::new(vec![Ok("in-workspace-result".to_string())]);
        let ws = WorkspaceGitExecutor::new(&mock, "/tmp/ws-a");

        let result = ws.run_in_workspace(&["diff", "--summary"], "/tmp/ws-b");
        assert_eq!(result.unwrap(), "in-workspace-result");

        let calls = mock.run_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "diff --summary");
        assert_eq!(calls[0].1.as_deref(), Some("/tmp/ws-b"));
    }

    #[test]
    fn test_workspace_executor_propagates_errors() {
        let mock = MockGitExecutor::new(vec![Err(ExecutorError::CommandNotFound(
            "git not found".to_string(),
        ))]);
        let ws = WorkspaceGitExecutor::new(&mock, "/tmp/workspace");

        let result = ws.run(&["log"]);
        assert_eq!(
            result.unwrap_err(),
            ExecutorError::CommandNotFound("git not found".to_string()),
        );
    }

    // -----------------------------------------------------------------------
    // get_uncommitted_files via mock executor
    // -----------------------------------------------------------------------

    #[test]
    fn test_get_uncommitted_files_clean_tree() {
        let executor = MockGitExecutor::new(vec![Ok("The working copy is clean\n".to_string())]);
        let files = get_uncommitted_files(&executor).expect("should succeed");
        assert!(files.is_empty());
    }

    #[test]
    fn test_get_uncommitted_files_with_changes() {
        let executor = MockGitExecutor::new(vec![Ok(
            "M src/lib.rs\nA src/new.rs\nD src/old.rs\nR src/renamed.rs\n".to_string(),
        )]);
        let files = get_uncommitted_files(&executor).expect("should succeed");
        assert_eq!(files.len(), 4);
        assert!(files.contains(&"src/lib.rs".to_string()));
        assert!(files.contains(&"src/new.rs".to_string()));
    }

    #[test]
    fn test_get_uncommitted_files_executor_error_propagates() {
        let executor = MockGitExecutor::new(vec![Err(ExecutorError::CommandFailed {
            code: 1,
            stderr: "git status failed".to_string(),
        })]);
        let result = get_uncommitted_files(&executor);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // get_commits_to_merge via mock executor
    // -----------------------------------------------------------------------

    #[test]
    fn test_get_commits_to_merge_multiple() {
        let executor =
            MockGitExecutor::new(vec![Ok("ch1\ncm1\nfeat: first\n2024-01-15 10:00:00\n\
             ch2\ncm2\nfix: second\n2024-01-15 11:00:00\n"
                .to_string())]);
        let commits = get_commits_to_merge(&executor).expect("should succeed");
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].change_id, "ch1");
        assert_eq!(commits[1].description, "fix: second");
    }

    #[test]
    fn test_get_commits_to_merge_empty() {
        let executor = MockGitExecutor::new(vec![Ok(String::new())]);
        let commits = get_commits_to_merge(&executor).expect("should succeed");
        assert!(commits.is_empty());
    }

    #[test]
    fn test_get_commits_to_merge_executor_error_propagates() {
        let executor = MockGitExecutor::new(vec![Err(ExecutorError::CommandFailed {
            code: 1,
            stderr: "git log failed".to_string(),
        })]);
        let result = get_commits_to_merge(&executor);
        assert!(result.is_err());
    }
}
