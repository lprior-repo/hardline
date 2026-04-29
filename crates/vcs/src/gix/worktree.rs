//! Gitoxide Worktree Operations
//!
//! Worktree management uses the `git worktree` CLI as a fallback because gix
//! does not natively support creating or removing worktrees.  `list()` scans
//! the `.git/worktrees/` directory for linked worktrees and combines them
//! with the main worktree from gix.

use std::path::PathBuf;

use crate::error::{GitError, GitResult};
use crate::gix::cli::{cli_error, require_workdir, run_git};

/// Add a worktree.
///
/// Uses `git worktree add` because gix lacks worktree management support.
pub fn add(
    repo: &gix::Repository,
    path: &PathBuf,
    branch: Option<&str>,
) -> GitResult<()> {
    let workdir = require_workdir(repo, "worktree add")?;

    let mut args: Vec<&str> = vec!["worktree", "add"];
    if let Some(b) = branch {
        args.push("-b");
        args.push(b);
    }
    args.push(path.to_str().ok_or_else(|| GitError::InvalidRef {
        name: "worktree".to_string(),
        reason: "worktree path is not valid UTF-8".to_string(),
    })?);
    // If no branch name given, use HEAD
    if branch.is_none() {
        args.push("HEAD");
    }

    let output = run_git(workdir, &args)?;

    if !output.success {
        return Err(cli_error(&output, "worktree add"));
    }
    Ok(())
}

/// List all worktrees (main + linked).
///
/// Scans `.git/worktrees/` for linked worktrees and adds the main worktree
/// discovered via gix.
pub fn list(repo: &gix::Repository) -> GitResult<Vec<Worktree>> {
    let mut worktrees = Vec::new();

    // Main worktree
    if let Some(workdir) = repo.workdir() {
        let main_branch = repo
            .head_name()
            .ok()
            .flatten()
            .map(|n| n.shorten().to_string());

        worktrees.push(Worktree {
            path: workdir.to_path_buf(),
            is_main: true,
            branch: main_branch,
        });
    }

    // Linked worktrees from .git/worktrees/
    let git_dir = repo.git_dir();
    let worktrees_dir = git_dir.join("worktrees");
    if worktrees_dir.is_dir() {
        let entries = list_worktree_entries(&worktrees_dir)?;
        worktrees.extend(entries);
    }

    Ok(worktrees)
}

/// Remove a worktree.
///
/// Uses `git worktree remove` because gix lacks worktree management support.
pub fn remove(repo: &gix::Repository, path: &PathBuf, force: bool) -> GitResult<()> {
    let workdir = require_workdir(repo, "worktree remove")?;

    let mut args: Vec<&str> = vec!["worktree", "remove"];
    if force {
        args.push("--force");
    }
    args.push(path.to_str().ok_or_else(|| GitError::InvalidRef {
        name: "worktree".to_string(),
        reason: "worktree path is not valid UTF-8".to_string(),
    })?);

    let output = run_git(workdir, &args)?;

    if !output.success {
        return Err(cli_error(&output, "worktree remove"));
    }
    Ok(())
}

// -- domain type --------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Worktree {
    pub path: PathBuf,
    pub is_main: bool,
    pub branch: Option<String>,
}

// -- internal helpers ---------------------------------------------------------

/// Read each subdirectory of `.git/worktrees/` and extract the worktree path
/// and branch from the `gitdir` and `HEAD` files.
fn list_worktree_entries(worktrees_dir: &std::path::Path) -> GitResult<Vec<Worktree>> {
    let mut entries = Vec::new();
    let dir_entries = std::fs::read_dir(worktrees_dir).map_err(GitError::Io)?;

    for dir_entry in dir_entries {
        let entry = dir_entry.map_err(GitError::Io)?;
        let meta = entry.metadata().map_err(GitError::Io)?;
        if !meta.is_dir() {
            continue;
        }

        let wt_path = entry.path();
        let gitdir_file = wt_path.join("gitdir");

        let gitdir_content = match std::fs::read_to_string(&gitdir_file) {
            Ok(content) => content,
            Err(_) => continue,
        };

        // The gitdir file contains the absolute path to the worktree's
        // .git file, e.g. `/path/to/worktree/.git/worktrees/name`.
        // The actual worktree root is the parent of the `.git` directory.
        let worktree_path = extract_worktree_root(&gitdir_content);
        let branch = read_worktree_head(&wt_path);

        entries.push(Worktree {
            path: worktree_path,
            is_main: false,
            branch,
        });
    }

    Ok(entries)
}

/// Given a gitdir file content (path to `.git/worktrees/<name>`), derive
/// the worktree root directory by walking up to the parent of the enclosing
/// `.git` directory.
fn extract_worktree_root(gitdir_content: &str) -> PathBuf {
    let trimmed = gitdir_content.trim();
    let gitdir_path = PathBuf::from(trimmed);

    // gitdir_content is typically:
    //   /abs/path/to/worktree/.git/worktrees/<name>
    // We want /abs/path/to/worktree
    if let Some(parent) = gitdir_path.parent() {
        // parent is .../worktree/.git/worktrees
        if let Some(grandparent) = parent.parent() {
            // grandparent is .../worktree/.git
            if let Some(ggparent) = grandparent.parent() {
                return ggparent.to_path_buf();
            }
        }
    }

    gitdir_path
}

/// Read the HEAD file inside a worktree entry to resolve the branch name.
fn read_worktree_head(wt_entry_dir: &std::path::Path) -> Option<String> {
    let head_file = wt_entry_dir.join("HEAD");
    let content = std::fs::read_to_string(&head_file).ok()?;
    let trimmed = content.trim();

    // HEAD is typically "ref: refs/heads/<branch>\n"
    if let Some(rest) = trimmed.strip_prefix("ref: refs/heads/") {
        Some(rest.trim().to_string())
    } else {
        None
    }
}
