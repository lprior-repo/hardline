//! Action layer for stack sync - I/O operations via git CLI.
//!
//! V1 uses shell commands directly. V2 will use extended VcsBackend trait.

use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

use scp_stack::{BranchName, PrState, Stack};

use super::calc::{
    compute_drift, detect_merged_branches, find_children_to_reparent, plan_restack_order,
    resolve_effective_parent, validate_sync_preconditions,
};
use super::data::{
    DriftReport, MergedBranch, MergedDetectionInput, RestackOutcome, RestackStatus,
    StackSyncOptions, StackSyncResult, SyncError,
};

/// Run the full stack sync operation.
///
/// # Errors
///
/// Returns `SyncError` for any failure during the sync operation.
pub fn run_stack_sync(
    workdir: &Path,
    stack: &Stack,
    options: &StackSyncOptions,
    pr_states: &std::collections::HashMap<BranchName, PrState>,
) -> Result<StackSyncResult, SyncError> {
    let mut result = StackSyncResult::default();
    let mut timings: Vec<(String, std::time::Instant)> = Vec::new();

    // 0. Validate preconditions
    let is_clean = check_workspace_clean(workdir);
    validate_sync_preconditions(stack, is_clean).map_err(|_| SyncError::DirtyWorkspace)?;

    // 1. Stash if dirty
    if !is_clean && options.force {
        stash_push(workdir).map_err(|e| SyncError::IoError(e.to_string()))?;
        result.stash_used = true;
    } else if !is_clean {
        return Err(SyncError::DirtyWorkspace);
    }

    // 2. Fetch from remote
    let fetch_start = std::time::Instant::now();
    fetch_remote(
        workdir,
        &options.remote_name,
        if options.full_fetch {
            None
        } else {
            Some(options.trunk_branch.as_str())
        },
    )?;
    timings.push(("fetch".to_string(), fetch_start));

    // 3. Update trunk
    let update_start = std::time::Instant::now();
    result.trunk_updated = update_trunk(
        workdir,
        options.trunk_branch.as_str(),
        &options.remote_name,
        options.safe,
    )?;
    timings.push(("update_trunk".to_string(), update_start));

    // 4. Detect merged branches
    if options.delete_merged {
        let detect_start = std::time::Instant::now();

        let local_merged = list_merged_branches(workdir, options.trunk_branch.as_str());
        let remote_merged = list_merged_branches(
            workdir,
            &format!("{}/{}", options.remote_name, options.trunk_branch),
        );
        let remote_branches = list_remote_branches(workdir, &options.remote_name);
        let local_branches = list_local_branches(workdir);

        let tracked: Vec<BranchName> = stack
            .branches
            .iter()
            .filter(|b| b.name != options.trunk_branch)
            .map(|b| b.name.clone())
            .collect();

        let input = MergedDetectionInput {
            tracked_branches: tracked,
            local_merged,
            remote_merged,
            pr_states: pr_states.clone(),
            remote_branches,
            local_branches,
        };

        let detected = detect_merged_branches(&options.trunk_branch, &input);

        // 5. Delete merged branches
        for (branch, method) in detected.iter().map(|(b, m)| (b, m)) {
            let merged = delete_merged_branch(workdir, stack, branch, method, options);
            result.merged_branches.push(merged);
        }

        timings.push(("detect_and_delete_merged".to_string(), detect_start));
    }

    // 6. Restack if requested
    if options.restack {
        let restack_start = std::time::Instant::now();
        let needs_restack = stack.needs_restack();
        let order = plan_restack_order(stack, &needs_restack);

        for (idx, branch) in order.iter().enumerate() {
            let parent = stack
                .branches
                .iter()
                .find(|b| &b.name == branch)
                .and_then(|b| b.parent.clone())
                .unwrap_or_else(|| options.trunk_branch.clone());

            match rebase_onto(workdir, branch.as_str(), parent.as_str()) {
                Ok(()) => {
                    result.restack_results.push(RestackOutcome {
                        branch: branch.clone(),
                        status: RestackStatus::Success,
                    });
                }
                Err(SyncError::RebaseConflict { .. }) => {
                    let remaining = order.len() - idx - 1;
                    result.restack_results.push(RestackOutcome {
                        branch: branch.clone(),
                        status: RestackStatus::Conflict { remaining },
                    });
                    result.had_conflicts = true;
                    break;
                }
                Err(e) => {
                    result.restack_results.push(RestackOutcome {
                        branch: branch.clone(),
                        status: RestackStatus::Conflict { remaining: 0 },
                    });
                    result.had_conflicts = true;
                    return Err(e);
                }
            }
        }

        timings.push(("restack".to_string(), restack_start));
    }

    // 7. Restore stash if used
    if result.stash_used {
        stash_pop(workdir).map_err(|e| SyncError::IoError(e.to_string()))?;
    }

    // Convert timings
    result.timings = timings
        .into_iter()
        .map(|(name, start)| (name, start.elapsed()))
        .collect();

    Ok(result)
}

// ============================================================================
// Shell command helpers
// ============================================================================

/// Check if workspace has uncommitted changes.
fn check_workspace_clean(workdir: &Path) -> bool {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(workdir)
        .output();

    match output {
        Ok(out) => out.status.success() && out.stdout.is_empty(),
        Err(_) => false,
    }
}

/// Fetch from remote. If refspec is None, fetch all refs.
fn fetch_remote(workdir: &Path, remote: &str, refspec: Option<&str>) -> Result<(), SyncError> {
    let mut args = vec!["fetch", "--no-tags", remote];
    if let Some(refspec) = refspec {
        args.push(refspec);
    }

    let output = Command::new("git")
        .args(&args)
        .current_dir(workdir)
        .output()
        .map_err(|e| SyncError::FetchFailed {
            remote: remote.to_string(),
            stderr: e.to_string(),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        // Fetch may partially succeed - warn but continue
        if stderr.contains("fatal:") || stderr.contains("Could not read from remote") {
            return Err(SyncError::FetchFailed {
                remote: remote.to_string(),
                stderr,
            });
        }
    }

    Ok(())
}

/// Update trunk branch via fast-forward merge.
fn update_trunk(workdir: &Path, trunk: &str, remote: &str, safe: bool) -> Result<bool, SyncError> {
    let remote_trunk = format!("{}/{}", remote, trunk);

    // Try ff-only merge first
    let output = Command::new("git")
        .args(["merge", "--ff-only", &remote_trunk])
        .current_dir(workdir)
        .output();

    match output {
        Ok(out) if out.status.success() => Ok(true),
        Ok(_) => {
            // ff-only failed
            if safe {
                return Ok(false);
            }
            // Try hard reset as fallback
            let reset = Command::new("git")
                .args(["reset", "--hard", &remote_trunk])
                .current_dir(workdir)
                .output();

            match reset {
                Ok(out) if out.status.success() => Ok(true),
                Ok(out) => Err(SyncError::TrunkUpdateFailed {
                    trunk: trunk.to_string(),
                    stderr: String::from_utf8_lossy(&out.stderr).to_string(),
                }),
                Err(e) => Err(SyncError::TrunkUpdateFailed {
                    trunk: trunk.to_string(),
                    stderr: e.to_string(),
                }),
            }
        }
        Err(e) => Err(SyncError::TrunkUpdateFailed {
            trunk: trunk.to_string(),
            stderr: e.to_string(),
        }),
    }
}

/// List branches merged into target (via `git branch --merged <target>`).
fn list_merged_branches(workdir: &Path, target: &str) -> HashSet<BranchName> {
    let output = Command::new("git")
        .args(["branch", "--merged", target])
        .current_dir(workdir)
        .output();

    match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(|line| line.trim().trim_start_matches("* "))
            .filter(|name| !name.is_empty())
            .map(|name| BranchName::new(name.to_string()))
            .collect(),
        _ => HashSet::new(),
    }
}

/// List remote-tracking branches.
fn list_remote_branches(workdir: &Path, remote: &str) -> HashSet<BranchName> {
    let output = Command::new("git")
        .args(["branch", "-r", "--format=%(refname:short)"])
        .current_dir(workdir)
        .output();

    match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(|line| line.trim())
            .filter(|name| name.starts_with(&format!("{}/", remote)))
            .map(|name| {
                let branch = name.strip_prefix(&format!("{}/", remote)).unwrap_or(name);
                BranchName::new(branch.to_string())
            })
            .collect(),
        _ => HashSet::new(),
    }
}

/// List local branches.
fn list_local_branches(workdir: &Path) -> HashSet<BranchName> {
    let output = Command::new("git")
        .args(["branch", "--format=%(refname:short)"])
        .current_dir(workdir)
        .output();

    match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(|line| line.trim())
            .filter(|name| !name.is_empty())
            .map(|name| BranchName::new(name.to_string()))
            .collect(),
        _ => HashSet::new(),
    }
}

/// Delete a merged branch and reparent its children.
fn delete_merged_branch(
    workdir: &Path,
    stack: &Stack,
    branch: &BranchName,
    method: &super::data::MergedDetectionMethod,
    options: &StackSyncOptions,
) -> MergedBranch {
    let all_local: HashSet<BranchName> = list_local_branches(workdir);

    // Resolve parent for reparenting
    let recorded_parent = stack
        .branches
        .iter()
        .find(|b| &b.name == branch)
        .and_then(|b| b.parent.clone())
        .unwrap_or_else(|| options.trunk_branch.clone());

    let (effective_parent, _fallback) =
        resolve_effective_parent(&recorded_parent, &options.trunk_branch, &all_local);

    // Reparent children
    let children = find_children_to_reparent(stack, branch, &effective_parent);
    let reparented: Vec<BranchName> = children.iter().map(|(c, _)| c.clone()).collect();

    // Delete local branch
    let local_deleted = if all_local.contains(branch) {
        let output = Command::new("git")
            .args(["branch", "-D", branch.as_str()])
            .current_dir(workdir)
            .output();
        matches!(output, Ok(out) if out.status.success())
    } else {
        false
    };

    // Delete remote branch
    let remote_deleted = Command::new("git")
        .args(["push", &options.remote_name, "--delete", branch.as_str()])
        .current_dir(workdir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    MergedBranch {
        name: branch.clone(),
        detection_method: *method,
        deleted_locally: local_deleted,
        deleted_remotely: remote_deleted,
        reparented_children: reparented,
    }
}

/// Rebase branch onto parent.
fn rebase_onto(workdir: &Path, branch: &str, parent: &str) -> Result<(), SyncError> {
    let output = Command::new("git")
        .args(["rebase", "--onto", parent, branch])
        .current_dir(workdir)
        .output();

    match output {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            if stderr.contains("CONFLICT") {
                // Abort the rebase to leave clean state
                let _ = Command::new("git")
                    .args(["rebase", "--abort"])
                    .current_dir(workdir)
                    .status();

                return Err(SyncError::RebaseConflict {
                    branch: BranchName::new(branch.to_string()),
                    parent: BranchName::new(parent.to_string()),
                });
            }
            Err(SyncError::RebaseFailed {
                branch: BranchName::new(branch.to_string()),
                reason: stderr,
            })
        }
        Err(e) => Err(SyncError::RebaseFailed {
            branch: BranchName::new(branch.to_string()),
            reason: e.to_string(),
        }),
    }
}

/// Push stash.
fn stash_push(workdir: &Path) -> Result<(), std::io::Error> {
    let output = Command::new("git")
        .args(["stash", "push"])
        .current_dir(workdir)
        .output()?;

    if output.status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }
}

/// Pop stash.
fn stash_pop(workdir: &Path) -> Result<(), std::io::Error> {
    let output = Command::new("git")
        .args(["stash", "pop"])
        .current_dir(workdir)
        .output()?;

    if output.status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_workspace_clean_command_format() {
        // Verify the command is well-formed - actual execution requires a git repo
        let cmd = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir("/tmp")
            .output();
        // Should not panic even on non-git dir
        assert!(cmd.is_ok());
    }

    #[test]
    fn sync_error_variants_display() {
        let err = SyncError::FetchFailed {
            remote: "origin".to_string(),
            stderr: "timeout".to_string(),
        };
        assert!(err.to_string().contains("origin"));

        let err = SyncError::TrunkUpdateFailed {
            trunk: "main".to_string(),
            stderr: "diverged".to_string(),
        };
        assert!(err.to_string().contains("main"));

        let err = SyncError::RebaseConflict {
            branch: BranchName::new("feat".to_string()),
            parent: BranchName::new("main".to_string()),
        };
        assert!(err.to_string().contains("feat"));
    }
}
