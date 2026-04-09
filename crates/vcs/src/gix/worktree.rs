//! Gitoxide Worktree Operations
//!
//! Provides worktree management via gix with porcelain parsing fallback.

use std::path::PathBuf;
use std::process::Command;

use crate::error::{GitError, GitResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Worktree {
    pub path: PathBuf,
    pub is_main: bool,
    pub branch: Option<String>,
    pub head: Option<String>,
}

fn parse_porcelain_worktree_list(output: &str) -> GitResult<Vec<Worktree>> {
    let mut worktrees = Vec::new();
    let mut current_worktree: Option<WorktreeBuilder> = None;
    let mut is_first = true;

    for line in output.lines() {
        if line.starts_with("worktree ") {
            if let Some(wt) = current_worktree.take() {
                worktrees.push(wt.build());
            }
            let path = line.strip_prefix("worktree ").unwrap().trim();
            current_worktree = Some(WorktreeBuilder {
                path: PathBuf::from(path),
                branch: None,
                head: None,
                is_first,
            });
            is_first = false;
        } else if let Some(ref mut wt) = current_worktree {
            if line.starts_with("HEAD ") {
                wt.head = Some(line.strip_prefix("HEAD ").unwrap().trim().to_string());
            } else if line.starts_with("branch ") {
                let branch_ref = line.strip_prefix("branch ").unwrap().trim();
                wt.branch = branch_ref.strip_prefix("refs/heads/").map(String::from);
            } else if line == "detached" {
                wt.branch = None;
            }
        }
    }

    if let Some(wt) = current_worktree {
        worktrees.push(wt.build());
    }

    if worktrees.is_empty() && !output.trim().is_empty() {
        return Err(GitError::ParseError(format!(
            "Failed to parse worktree list output: {output}"
        )));
    }

    Ok(worktrees)
}

struct WorktreeBuilder {
    path: PathBuf,
    branch: Option<String>,
    head: Option<String>,
    is_first: bool,
}

impl WorktreeBuilder {
    fn build(self) -> Worktree {
        Worktree {
            path: self.path,
            is_main: self.is_first,
            branch: self.branch,
            head: self.head,
        }
    }
}

pub fn add(repo: &gix::Repository, path: &PathBuf, branch: Option<&str>) -> GitResult<()> {
    let parent = repo.workdir().ok_or_else(|| GitError::InvalidRef {
        name: "worktree".to_string(),
        reason: "repository has no working directory".to_string(),
    })?;

    let mut cmd = Command::new("git");
    cmd.args(["worktree", "add"]);

    if let Some(branch_name) = branch {
        cmd.arg("-b").arg(branch_name);
    }

    cmd.arg(path).current_dir(parent);

    let output = cmd.output().map_err(|e| {
        GitError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("git command not found: {e}"),
        ))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::InvalidRef {
            name: "worktree".to_string(),
            reason: stderr.to_string(),
        });
    }

    Ok(())
}

pub fn list(repo: &gix::Repository) -> GitResult<Vec<Worktree>> {
    let workdir = repo.workdir().ok_or_else(|| GitError::InvalidRef {
        name: "worktree".to_string(),
        reason: "repository has no working directory".to_string(),
    })?;

    let output = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(workdir)
        .output()
        .map_err(|e| {
            GitError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("git command not found: {e}"),
            ))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::InvalidRef {
            name: "worktree".to_string(),
            reason: stderr.to_string(),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_porcelain_worktree_list(&stdout)
}

pub fn remove(repo: &gix::Repository, path: &PathBuf, force: bool) -> GitResult<()> {
    let path_str = path.to_string_lossy();
    let workdir = repo.workdir().ok_or_else(|| GitError::InvalidRef {
        name: "worktree".to_string(),
        reason: "repository has no working directory".to_string(),
    })?;

    let mut cmd = Command::new("git");
    cmd.args(["worktree", "remove", &path_str]);

    if force {
        cmd.arg("--force");
    }

    let output = cmd.current_dir(workdir).output().map_err(|e| {
        GitError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("git command not found: {e}"),
        ))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::InvalidRef {
            name: "worktree".to_string(),
            reason: stderr.to_string(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_porcelain_single_worktree() {
        let output = "worktree /path/to/repo\nHEAD abc123\nbranch refs/heads/main\n";
        let worktrees = parse_porcelain_worktree_list(output).unwrap();
        assert_eq!(worktrees.len(), 1);
        assert_eq!(worktrees[0].path, PathBuf::from("/path/to/repo"));
        assert!(worktrees[0].is_main);
    }

    #[test]
    fn parse_porcelain_multiple_worktrees() {
        let output = "worktree /path/to/repo\nHEAD abc123\nbranch refs/heads/main\n\nworktree /path/to/repo/feature\nHEAD def456\nbranch refs/heads/feature-branch\n";
        let worktrees = parse_porcelain_worktree_list(output).unwrap();
        assert_eq!(worktrees.len(), 2);
    }

    #[test]
    fn parse_porcelain_detached_head() {
        let output = "worktree /path/to/repo\nHEAD abc123\ndetached\n";
        let worktrees = parse_porcelain_worktree_list(output).unwrap();
        assert_eq!(worktrees.len(), 1);
        assert!(worktrees[0].branch.is_none());
    }

    #[test]
    fn parse_porcelain_empty_output() {
        let worktrees = parse_porcelain_worktree_list("").unwrap();
        assert!(worktrees.is_empty());
    }

    #[test]
    fn parse_porcelain_main_worktree() {
        let output = "worktree /path/to/main\nHEAD abc123\nbranch refs/heads/main\n";
        let worktrees = parse_porcelain_worktree_list(output).unwrap();
        assert!(worktrees[0].is_main);
    }
}
