//! Gitoxide Diff and File Operations
//!
//! Port of stax/src/git/repo.rs diff/file operations.
//!
//! Some operations use CLI fallback where gix does not yet provide
//! the required diff/index inspection APIs.

use crate::error::{GitError, GitResult};

/// List files currently in an unmerged (conflicted) state.
///
/// Uses `git diff --name-only --diff-filter=U` via CLI since gix
/// does not expose conflict state directly.
pub fn conflicted_files(repo: &gix::Repository) -> GitResult<Vec<String>> {
    let workdir = repo.workdir().ok_or_else(|| GitError::InvalidRef {
        name: "workdir".to_string(),
        reason: "Bare repository has no workdir".to_string(),
    })?;

    let output = std::process::Command::new("git")
        .args(["diff", "--name-only", "--diff-filter=U"])
        .current_dir(workdir)
        .output()
        .map_err(GitError::Io)?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect())
}

/// List paths currently modified, staged, unmerged, or untracked.
///
/// Uses `git status --porcelain` via CLI.
pub fn changed_files(repo: &gix::Repository) -> GitResult<Vec<String>> {
    let workdir = repo.workdir().ok_or_else(|| GitError::InvalidRef {
        name: "workdir".to_string(),
        reason: "Bare repository has no workdir".to_string(),
    })?;

    let output = std::process::Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=all"])
        .current_dir(workdir)
        .output()
        .map_err(GitError::Io)?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let mut seen = std::collections::HashSet::new();
    let mut files = Vec::new();

    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if line.len() < 4 {
            continue;
        }
        let mut path = line[3..].trim().to_string();
        if path.is_empty() {
            continue;
        }

        // Handle renames: "old -> new"
        if let Some((_, new_path)) = path.rsplit_once(" -> ") {
            path = new_path.to_string();
        }

        if seen.insert(path.clone()) {
            files.push(path);
        }
    }

    Ok(files)
}

/// Stage an explicit list of files.
pub fn add_files(repo: &gix::Repository, paths: &[String]) -> GitResult<()> {
    if paths.is_empty() {
        return Ok(());
    }

    let workdir = repo.workdir().ok_or_else(|| GitError::InvalidRef {
        name: "workdir".to_string(),
        reason: "Bare repository has no workdir".to_string(),
    })?;

    let status = std::process::Command::new("git")
        .arg("add")
        .arg("--")
        .args(paths)
        .current_dir(workdir)
        .status()
        .map_err(GitError::Io)?;

    if !status.success() {
        return Err(GitError::InvalidRef {
            name: "add".to_string(),
            reason: "git add failed".to_string(),
        });
    }

    Ok(())
}

/// Get diff between two refs using three-dot syntax (merge-base diff).
///
/// Returns diff lines.
pub fn diff_against_parent(repo: &gix::Repository, branch: &str, parent: &str) -> GitResult<Vec<String>> {
    let workdir = repo.workdir().ok_or_else(|| GitError::InvalidRef {
        name: "workdir".to_string(),
        reason: "Bare repository has no workdir".to_string(),
    })?;

    let range = format!("{parent}...{branch}");
    let output = std::process::Command::new("git")
        .args(["diff", "--color=never", &range])
        .current_dir(workdir)
        .output()
        .map_err(GitError::Io)?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| s.to_string())
        .collect())
}

/// Get diff stat (numstat) between two refs using three-dot syntax.
///
/// Returns `(file, additions, deletions)` tuples.
pub fn diff_stat(
    repo: &gix::Repository,
    branch: &str,
    parent: &str,
) -> GitResult<Vec<(String, usize, usize)>> {
    let workdir = repo.workdir().ok_or_else(|| GitError::InvalidRef {
        name: "workdir".to_string(),
        reason: "Bare repository has no workdir".to_string(),
    })?;

    let range = format!("{parent}...{branch}");
    let output = std::process::Command::new("git")
        .args(["diff", "--numstat", &range])
        .current_dir(workdir)
        .output()
        .map_err(GitError::Io)?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let mut results = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 3 {
            let additions: usize = parts[0].parse().unwrap_or(0);
            let deletions: usize = parts[1].parse().unwrap_or(0);
            let file = parts[2].to_string();
            results.push((file, additions, deletions));
        }
    }

    Ok(results)
}

/// Get files modified between two refs.
pub fn files_modified(repo: &gix::Repository, branch: &str, parent: &str) -> GitResult<Vec<String>> {
    let workdir = repo.workdir().ok_or_else(|| GitError::InvalidRef {
        name: "workdir".to_string(),
        reason: "Bare repository has no workdir".to_string(),
    })?;

    let output = std::process::Command::new("git")
        .args(["diff", "--name-only", parent, branch])
        .current_dir(workdir)
        .output()
        .map_err(GitError::Io)?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| s.to_string())
        .collect())
}
