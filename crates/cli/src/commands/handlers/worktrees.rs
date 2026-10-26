//! Shared git-worktree resolution for handlers that must operate inside a
//! specific worktree.
//!
//! The backend's `Workspace.name` is the worktree path relative to the
//! backend's root, so it can never equal a short workspace name such as
//! `probe1` (it is `.scp/workspaces/probe1` from the main repo, and
//! something else entirely from inside a sibling worktree). Commands that
//! take a workspace name therefore need the absolute worktree path, which
//! only `git worktree list --porcelain` gives directly, from any cwd.
//!
//! The worktree model also decides how sync/batch run: they operate inside
//! the target worktree directory (the same pattern as `spawn_with_sync`,
//! which rebases via `git -C <worktree>`). Switching the caller's checkout
//! to the workspace branch is wrong — git refuses to check a branch out
//! twice, and the branch already lives in its worktree.

use std::path::{Path, PathBuf};

use scp_core::{Error, Result};

/// One entry of `git worktree list --porcelain`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeEntry {
    /// Absolute path to the worktree root.
    pub path: PathBuf,
    /// Branch checked out here, without the `refs/heads/` prefix
    /// (empty for a detached HEAD).
    pub branch: String,
    /// True for a bare entry (not a usable workspace).
    pub bare: bool,
}

/// Parse `git worktree list --porcelain` output.
///
/// Blocks are separated by blank lines; each block starts with
/// `worktree <path>`, optionally followed by `branch <ref>` and `bare`.
/// Mirrors the parsing in `scp-core`'s `parse_git_worktree_list`, minus
/// the relative-name/current computation, which needs a backend root.
pub fn parse_worktree_list(stdout: &str) -> Vec<WorktreeEntry> {
    let mut entries = Vec::new();
    let mut current_path: Option<PathBuf> = None;
    let mut current_branch = String::new();
    let mut is_bare = false;
    let mut flush = |path: &mut Option<PathBuf>, branch: &mut String, bare: &mut bool| {
        if let Some(p) = path.take() {
            entries.push(WorktreeEntry {
                path: p,
                branch: std::mem::take(branch),
                bare: *bare,
            });
            *bare = false;
        }
    };
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            flush(&mut current_path, &mut current_branch, &mut is_bare);
            continue;
        }
        if let Some(path) = line.strip_prefix("worktree ") {
            flush(&mut current_path, &mut current_branch, &mut is_bare);
            current_path = Some(PathBuf::from(path));
        } else if let Some(branch) = line.strip_prefix("branch ") {
            current_branch = branch
                .strip_prefix("refs/heads/")
                .unwrap_or(branch)
                .to_string();
        } else if line == "bare" {
            is_bare = true;
        }
    }
    flush(&mut current_path, &mut current_branch, &mut is_bare);
    entries
}

/// Run `git worktree list --porcelain` in `cwd` and parse it.
pub fn list_worktrees(cwd: &Path) -> Result<Vec<WorktreeEntry>> {
    let output = std::process::Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(cwd)
        .output()
        .map_err(|e| Error::io_error(e.to_string()))?;
    if !output.status.success() {
        return Err(Error::vcs_conflict(
            "worktree list",
            String::from_utf8_lossy(&output.stderr),
        ));
    }
    Ok(parse_worktree_list(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

/// Find a worktree by short name: its directory file name, its branch,
/// or its exact path, in that order.
pub fn find_worktree<'a>(entries: &'a [WorktreeEntry], name: &str) -> Option<&'a WorktreeEntry> {
    entries
        .iter()
        .find(|e| !e.bare && e.path.file_name().is_some_and(|f| f == name))
        .or_else(|| entries.iter().find(|e| !e.bare && e.branch == name))
        .or_else(|| entries.iter().find(|e| !e.bare && e.path == Path::new(name)))
}

/// The worktree containing `cwd`: the longest worktree path that is a
/// prefix of it. This is the same rule `scp-core` uses for `is_current`.
pub fn current_worktree<'a>(entries: &'a [WorktreeEntry], cwd: &Path) -> Option<&'a WorktreeEntry> {
    entries
        .iter()
        .filter(|e| !e.bare && cwd.starts_with(&e.path))
        .max_by_key(|e| e.path.as_os_str().len())
}

/// The main worktree: git always lists it first.
pub fn main_worktree(entries: &[WorktreeEntry]) -> Option<&WorktreeEntry> {
    entries.iter().find(|e| !e.bare)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
worktree /repo\n\
branch refs/heads/main\n\
\n\
worktree /repo/.scp/workspaces/probe1\n\
branch refs/heads/probe1\n\
\n\
worktree /repo/other\n\
detached\n\
";

    #[test]
    fn parse_entries() {
        let entries = parse_worktree_list(SAMPLE);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].path, PathBuf::from("/repo"));
        assert_eq!(entries[0].branch, "main");
        assert_eq!(entries[1].branch, "probe1");
        assert_eq!(entries[2].branch, "");
    }

    #[test]
    fn find_by_dir_name_branch_or_path() {
        let entries = parse_worktree_list(SAMPLE);
        assert_eq!(
            find_worktree(&entries, "probe1").map(|e| e.path.as_path()),
            Some(Path::new("/repo/.scp/workspaces/probe1"))
        );
        assert_eq!(
            find_worktree(&entries, "main").map(|e| e.path.as_path()),
            Some(Path::new("/repo"))
        );
        assert_eq!(
            find_worktree(&entries, "/repo/other").map(|e| e.path.as_path()),
            Some(Path::new("/repo/other"))
        );
        assert!(find_worktree(&entries, "nope").is_none());
    }

    #[test]
    fn current_is_longest_prefix() {
        let entries = parse_worktree_list(SAMPLE);
        let cur = current_worktree(&entries, Path::new("/repo/.scp/workspaces/probe1/src"));
        assert_eq!(cur.map(|e| e.path.as_path()), Some(Path::new("/repo/.scp/workspaces/probe1")));
        let cur = current_worktree(&entries, Path::new("/repo"));
        assert_eq!(cur.map(|e| e.path.as_path()), Some(Path::new("/repo")));
        assert!(current_worktree(&entries, Path::new("/elsewhere")).is_none());
    }

    #[test]
    fn main_is_first_entry() {
        let entries = parse_worktree_list(SAMPLE);
        assert_eq!(
            main_worktree(&entries).map(|e| e.path.as_path()),
            Some(Path::new("/repo"))
        );
    }
}
