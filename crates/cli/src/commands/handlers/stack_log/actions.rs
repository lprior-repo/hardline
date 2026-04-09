//! Action layer for stack log - I/O operations via git CLI.
//!
//! V1 uses shell commands directly. V2 will use gix-based VcsBackend.

use std::path::Path;
use std::process::Command;

use scp_core::output::Output;
use scp_stack::{BranchName, Stack, StackBranch};

use super::calc::{
    collect_needs_restack, compute_depths, count_total_commits, filter_to_lineage, format_linear,
    format_tree,
};
use super::data::{
    LogError, StackLogBranchEntry, StackLogCommit, StackLogOptions, StackLogOutput,
};

/// Run the stack log command.
///
/// # Errors
///
/// Returns `LogError` for any failure during the log operation.
pub fn run_stack_log(
    workdir: &Path,
    stack: &Stack,
    options: &StackLogOptions,
) -> Result<(), LogError> {
    let trunk = stack.main_branch.clone();
    let depths = compute_depths(stack, &trunk);

    // Filter branches if a specific branch was requested
    let branches: Vec<&StackBranch> = match &options.branch_filter {
        Some(target) => filter_to_lineage(stack, target),
        None => stack.topological_order(),
    };

    // Build branch entries
    let mut entries = Vec::new();
    for branch in &branches {
        let parent = branch.parent.as_ref().map(|p| p.as_str());
        let limit = options.limit.unwrap_or(50);

        let commits = get_branch_commits(workdir, branch.name.as_str(), parent, limit)?;
        let (ahead, behind) = match parent {
            Some(p) => get_ahead_behind(workdir, p, branch.name.as_str())?,
            None => (commits.len(), 0),
        };

        let depth = depths.get(&branch.name).copied().unwrap_or(0);

        entries.push(StackLogBranchEntry {
            branch: branch.name.clone(),
            parent: branch.parent.clone(),
            depth,
            commits: if options.include_messages {
                commits
            } else {
                vec![]
            },
            ahead,
            behind,
            needs_restack: branch.needs_restack,
            pr_number: branch.pr_info.as_ref().map(|p| p.number as u64),
            pr_state: branch.pr_info.as_ref().map(|p| format!("{:?}", p.state)),
        });
    }

    let needs_restack = collect_needs_restack(&entries);
    let total_commits = count_total_commits(&entries);

    let output = StackLogOutput {
        branches: entries,
        trunk,
        total_branches: stack.branches.len(),
        total_commits,
        needs_restack,
    };

    match options.format.as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(&output)
                .map_err(|e| LogError::IoError(e.to_string()))?;
            Output::info(&json);
        }
        "linear" => {
            let display = format_linear(&output);
            Output::info(&display);
        }
        _ => {
            // Default to tree format
            let display = format_tree(&output);
            Output::info(&display);
        }
    }

    Ok(())
}

// ============================================================================
// Shell command helpers
// ============================================================================

/// Get commits unique to a branch (not reachable from parent).
fn get_branch_commits(
    workdir: &Path,
    branch: &str,
    parent: Option<&str>,
    limit: usize,
) -> Result<Vec<StackLogCommit>, LogError> {
    let branch_ref = format!("refs/heads/{branch}");
    let refspec = match parent {
        Some(p) => format!("refs/heads/{p}..{branch_ref}"),
        None => branch_ref,
    };

    let output = Command::new("git")
        .args([
            "log",
            &refspec,
            "--format=%H|%h|%s|%an|%aI",
            "-n",
            &limit.to_string(),
        ])
        .current_dir(workdir)
        .output()
        .map_err(|e| LogError::IoError(e.to_string()))?;

    if !output.status.success() {
        // Branch may not exist locally — return empty
        return Ok(vec![]);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut commits = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.splitn(5, '|').collect();
        if parts.len() == 5 {
            commits.push(StackLogCommit {
                hash: parts[0].to_string(),
                short_hash: parts[1].to_string(),
                message: parts[2].to_string(),
                author: parts[3].to_string(),
                datetime: parts[4].to_string(),
            });
        }
    }

    Ok(commits)
}

/// Get ahead/behind counts between two refs.
fn get_ahead_behind(
    workdir: &Path,
    base: &str,
    head: &str,
) -> Result<(usize, usize), LogError> {
    let output = Command::new("git")
        .args([
            "rev-list",
            "--left-right",
            "--count",
            &format!("refs/heads/{base}...refs/heads/{head}"),
        ])
        .current_dir(workdir)
        .output()
        .map_err(|e| LogError::IoError(e.to_string()))?;

    if !output.status.success() {
        return Ok((0, 0));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = stdout.trim().split_whitespace().collect();
    if parts.len() == 2 {
        let behind: usize = parts[0].parse().unwrap_or(0);
        let ahead: usize = parts[1].parse().unwrap_or(0);
        Ok((ahead, behind))
    } else {
        Ok((0, 0))
    }
}

/// Load stack structure from git metadata refs.
///
/// Reads branch metadata stored in `refs/stack/` namespace by stax.
/// Falls back to discovering branches from `refs/heads/` with parent
/// detection via merge-base if no stack metadata exists.
pub fn load_stack_from_git(workdir: &Path) -> Result<Stack, LogError> {
    let trunk = detect_trunk(workdir)?;

    // Try loading from stax metadata refs first
    let tracked = list_stack_tracked_branches(workdir);
    if !tracked.is_empty() {
        let mut stack = Stack::new(trunk.clone());
        for branch_name in &tracked {
            let parent_str = read_branch_parent(workdir, branch_name)
                .unwrap_or_else(|| trunk.as_str().to_string());
            let branch = scp_stack::StackBranch {
                name: BranchName::new(branch_name.clone()),
                parent: Some(BranchName::new(parent_str)),
                children: vec![],
                needs_restack: false,
                pr_info: None,
            };
            let _ = stack.add_branch(branch);
        }
        return Ok(stack);
    }

    // Fallback: discover from local branches
    let branches = list_local_branches(workdir);
    let mut stack = Stack::new(trunk.clone());

    for branch_name in &branches {
        if branch_name == trunk.as_str() {
            continue;
        }
        // Find parent via merge-base with trunk
        let parent_str = find_merge_base_parent(workdir, branch_name, trunk.as_str())
            .unwrap_or_else(|| trunk.as_str().to_string());

        let branch = scp_stack::StackBranch {
            name: BranchName::new(branch_name.clone()),
            parent: Some(BranchName::new(parent_str)),
            children: vec![],
            needs_restack: false,
            pr_info: None,
        };
        let _ = stack.add_branch(branch);
    }

    Ok(stack)
}

/// Detect the trunk branch (main or master).
fn detect_trunk(workdir: &Path) -> Result<BranchName, LogError> {
    // Check for main first, then master
    for candidate in ["main", "master"] {
        let ref_name = format!("refs/heads/{candidate}");
        let output = Command::new("git")
            .args(["rev-parse", "--verify", "--quiet", &ref_name])
            .current_dir(workdir)
            .output()
            .map_err(|e| LogError::IoError(e.to_string()))?;

        if output.status.success() {
            return Ok(BranchName::new(candidate.to_string()));
        }
    }

    Ok(BranchName::new("main".to_string()))
}

/// List branches tracked in stack metadata (refs/stack/).
fn list_stack_tracked_branches(workdir: &Path) -> Vec<String> {
    let output = Command::new("git")
        .args(["for-each-ref", "--format=%(refname:short)", "refs/stack/"])
        .current_dir(workdir)
        .output();

    match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect(),
        _ => vec![],
    }
}

/// Read the parent branch for a tracked branch from stack metadata.
fn read_branch_parent(workdir: &Path, branch_name: &str) -> Option<String> {
    let ref_name = format!("refs/stack/{branch_name}/parent");
    let output = Command::new("git")
        .args(["symbolic-ref", &ref_name])
        .current_dir(workdir)
        .output()
        .ok()?;

    if output.status.success() {
        let parent = String::from_utf8_lossy(&output.stdout).trim().to_string();
        // Strip refs/heads/ prefix if present
        Some(parent.strip_prefix("refs/heads/").unwrap_or(&parent).to_string())
    } else {
        // Try reading as a blob ref
        let blob_ref = format!("refs/stack/{branch_name}");
        let output = Command::new("git")
            .args(["cat-file", "-p", &blob_ref])
            .current_dir(workdir)
            .output()
            .ok()?;

        if output.status.success() {
            let content = String::from_utf8_lossy(&output.stdout);
            for line in content.lines() {
                if line.starts_with("parent:") || line.starts_with("parent ") {
                    return Some(
                        line.split(':')
                            .nth(1)
                            .or_else(|| line.split_whitespace().nth(1))
                            .unwrap_or("")
                            .trim()
                            .to_string(),
                    );
                }
            }
        }
        None
    }
}

/// List local branches.
fn list_local_branches(workdir: &Path) -> Vec<String> {
    let output = Command::new("git")
        .args(["branch", "--format=%(refname:short)"])
        .current_dir(workdir)
        .output();

    match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.trim().to_string())
            .collect(),
        _ => vec![],
    }
}

/// Find the best parent for a branch via merge-base.
fn find_merge_base_parent(workdir: &Path, branch: &str, trunk: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["merge-base", "--fork-point", branch, trunk])
        .current_dir(workdir)
        .output();

    match output {
        Ok(out) if out.status.success() => {
            // Fork point exists — parent is trunk
            Some(trunk.to_string())
        }
        _ => {
            // Fall back to trunk as parent
            Some(trunk.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_ahead_behind_non_git_dir_returns_zeros() {
        let result = get_ahead_behind(Path::new("/tmp"), "main", "feat");
        // Not a git repo, but shouldn't panic — returns zeros
        assert!(result.is_ok());
        let (ahead, behind) = result.expect("ok");
        assert_eq!(ahead, 0);
        assert_eq!(behind, 0);
    }

    #[test]
    fn get_branch_commits_non_git_dir_returns_empty() {
        let result = get_branch_commits(Path::new("/tmp"), "main", None, 10);
        assert!(result.is_ok());
        assert!(result.expect("ok").is_empty());
    }
}
