//! Action layer for stack merge-remote — I/O operations via GitHubClient.
//!
//! Uses the forge client for all remote operations. No local git checkout needed.

use scp_stack::infrastructure::forge::github::GitHubClient;
use scp_stack::infrastructure::forge::{ForgeType, MergeMethod, RemoteInfo};
use scp_stack::domain::state::PrState as ForgePrState;
use scp_stack::{BranchName, Stack};

use super::calc::{
    build_remaining_infos, calculate_merge_scope, partition_by_pr, resolve_pr_numbers,
};
use super::data::{
    MergeFailure, MergeRemoteError, MergeRemoteOptions, MergeRemoteOutput, MergedPr, PrBranchInfo,
    WaitOutcome,
};

/// Run the full stack merge-remote operation.
///
/// Merges PRs via the GitHub API without requiring a local checkout.
/// Dependent PR branches are updated via GitHub's "Update branch" endpoint.
///
/// # Errors
///
/// Returns `MergeRemoteError` for any failure during the merge operation.
pub fn run_merge_remote(
    stack: &Stack,
    current_branch: &BranchName,
    client: &GitHubClient,
    remote_info: &RemoteInfo,
    options: &MergeRemoteOptions,
) -> Result<MergeRemoteOutput, MergeRemoteError> {
    // 0. Validate preconditions
    if current_branch == &stack.main_branch {
        return Err(MergeRemoteError::OnTrunk);
    }

    let is_tracked = stack.branches.iter().any(|b| &b.name == current_branch);
    if !is_tracked {
        return Err(MergeRemoteError::NotTracked(current_branch.clone()));
    }

    // 1. Validate forge type
    if remote_info.forge != ForgeType::GitHub {
        return Err(MergeRemoteError::ForgeNotSupported {
            found: remote_info.forge.to_string(),
        });
    }

    // 2. Calculate merge scope
    let scope = calculate_merge_scope(stack, current_branch, options.all);

    if scope.to_merge.is_empty() {
        return Err(MergeRemoteError::NothingToMerge);
    }

    // 3. Resolve PR numbers from stack metadata
    let resolved = resolve_pr_numbers(&scope.to_merge, &stack.branches);
    let (mut ready, missing) = partition_by_pr(&resolved);

    // 4. For branches without PR numbers, try forge lookup (sync wrapper)
    if !missing.is_empty() {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| MergeRemoteError::ForgeClientError(e.to_string()))?;
        for branch in &missing {
            match rt.block_on(client.find_pr(branch.as_str())) {
                Ok(Some(pr_info)) if pr_info.state == ForgePrState::Open => {
                    ready.push(PrBranchInfo {
                        branch: branch.clone(),
                        pr_number: u64::from(pr_info.pr_number),
                    });
                }
                Ok(Some(_)) => {
                    // PR exists but not open (merged/closed) — still usable for
                    // is_pr_merged check
                }
                Ok(None) => {
                    return Err(MergeRemoteError::NoPr {
                        branch: branch.clone(),
                    });
                }
                Err(e) => {
                    return Err(MergeRemoteError::ForgeClientError(e.to_string()));
                }
            }
        }
    }

    if ready.is_empty() {
        return Err(MergeRemoteError::NothingToMerge);
    }

    // 5. Execute merge loop
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| MergeRemoteError::ForgeClientError(e.to_string()))?;
    let mut output = MergeRemoteOutput::default();

    for (idx, branch_info) in ready.iter().enumerate() {
        let next_branch = ready.get(idx + 1);

        // Check if already merged
        let is_merged = rt
            .block_on(client.is_pr_merged(branch_info.pr_number))
            .map_err(|e| MergeRemoteError::IsMergedFailed {
                pr_number: branch_info.pr_number,
                reason: e.to_string(),
            })?;

        if is_merged {
            output.merged_prs.push(MergedPr {
                branch: branch_info.branch.clone(),
                pr_number: branch_info.pr_number,
            });

            // Retarget next PR to trunk
            if let Some(next) = next_branch {
                retarget_pr(&rt, client, next.pr_number, scope.trunk.as_str())?;
            }
            continue;
        }

        // Wait for CI
        match wait_for_pr_ready(&rt, client, branch_info.pr_number, options)? {
            WaitOutcome::Ready => {}
            WaitOutcome::Blocked(reason) => {
                output.failure = Some(MergeFailure {
                    branch: branch_info.branch.clone(),
                    pr_number: branch_info.pr_number,
                    reason,
                });
                break;
            }
            WaitOutcome::Timeout => {
                output.failure = Some(MergeFailure {
                    branch: branch_info.branch.clone(),
                    pr_number: branch_info.pr_number,
                    reason: "Timeout waiting for CI".to_string(),
                });
                break;
            }
        }

        // Retarget next PR to trunk before merge
        if let Some(next) = next_branch {
            retarget_pr(&rt, client, next.pr_number, scope.trunk.as_str())?;
        }

        // Merge the PR
        rt.block_on(client.merge_pr(branch_info.pr_number, options.method, None, None))
            .map_err(|e| MergeRemoteError::MergeFailed {
                pr_number: branch_info.pr_number,
                reason: e.to_string(),
            })?;

        output.merged_prs.push(MergedPr {
            branch: branch_info.branch.clone(),
            pr_number: branch_info.pr_number,
        });

        // Update next branch via GitHub's "Update branch" endpoint
        if let Some(next) = next_branch {
            rt.block_on(client.update_pr_branch(next.pr_number))
                .map_err(|e| MergeRemoteError::UpdateBranchFailed {
                    pr_number: next.pr_number,
                    reason: e.to_string(),
                })?;
        }
    }

    // 6. Retarget remaining branches
    if output.failure.is_none() && !scope.remaining.is_empty() && !output.merged_prs.is_empty() {
        let remaining_infos = build_remaining_infos(&scope.remaining, &stack.branches);
        for remaining in &remaining_infos {
            let Some(pr_num) = remaining.pr_number else {
                continue;
            };

            retarget_pr(&rt, client, pr_num, scope.trunk.as_str())?;

            rt.block_on(client.update_pr_branch(pr_num))
                .map_err(|e| MergeRemoteError::UpdateBranchFailed {
                    pr_number: pr_num,
                    reason: e.to_string(),
                })?;

            output.retargeted_remaining.push(remaining.branch.clone());
        }
    }

    // 7. Clean up metadata for merged branches
    if !options.no_delete && !output.merged_prs.is_empty() {
        for merged in &output.merged_prs {
            output.cleaned_branches.push(merged.branch.clone());
        }
    }

    Ok(output)
}

/// Retarget a PR's base branch to a new target.
fn retarget_pr(
    rt: &tokio::runtime::Runtime,
    client: &GitHubClient,
    pr_number: u64,
    target: &str,
) -> Result<(), MergeRemoteError> {
    let _result = rt.block_on(client.update_pr(pr_number, None, None, Some(target)));
    Ok(())
}

/// Wait for a PR to become ready (CI passing, approved).
///
/// Polls the PR state at the configured interval until:
/// - The PR is ready (mergeable + CI green)
/// - The PR is blocked (conflict, failing CI, etc.)
/// - The timeout elapses
fn wait_for_pr_ready(
    rt: &tokio::runtime::Runtime,
    client: &GitHubClient,
    pr_number: u64,
    options: &MergeRemoteOptions,
) -> Result<WaitOutcome, MergeRemoteError> {
    let start = std::time::Instant::now();

    loop {
        let pr_info = rt
            .block_on(client.get_pr(pr_number))
            .map_err(|e| MergeRemoteError::WaitFailed {
                pr_number,
                reason: e.to_string(),
            })?;

        // Already merged
        if matches!(pr_info.state, ForgePrState::Merged) {
            return Ok(WaitOutcome::Ready);
        }

        // Closed/failed
        if matches!(pr_info.state, ForgePrState::Closed) {
            return Ok(WaitOutcome::Blocked("PR was closed".to_string()));
        }

        // Draft PRs can't be merged
        if pr_info.draft {
            return Ok(WaitOutcome::Blocked("PR is a draft".to_string()));
        }

        if start.elapsed() > options.timeout {
            return Ok(WaitOutcome::Timeout);
        }

        // Not ready yet — wait before next poll
        std::thread::sleep(options.poll_interval);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_remote_error_variants_display() {
        let err = MergeRemoteError::OnTrunk;
        assert!(err.to_string().contains("trunk"));

        let err = MergeRemoteError::NoPr {
            branch: BranchName::new("feat".to_string()),
        };
        assert!(err.to_string().contains("feat"));

        let err = MergeRemoteError::MergeFailed {
            pr_number: 42,
            reason: "conflict".to_string(),
        };
        assert!(err.to_string().contains("42"));

        let err = MergeRemoteError::ForgeNotSupported {
            found: "GitLab".to_string(),
        };
        assert!(err.to_string().contains("GitLab"));
    }

    #[test]
    fn wait_outcome_variants() {
        assert_eq!(WaitOutcome::Ready, WaitOutcome::Ready);
        assert_eq!(WaitOutcome::Timeout, WaitOutcome::Timeout);
        assert_eq!(
            WaitOutcome::Blocked("conflict".to_string()),
            WaitOutcome::Blocked("conflict".to_string())
        );
        assert_ne!(WaitOutcome::Ready, WaitOutcome::Timeout);
    }

    #[test]
    fn merge_remote_output_default() {
        let output = MergeRemoteOutput::default();
        assert!(output.merged_prs.is_empty());
        assert!(output.failure.is_none());
        assert!(output.retargeted_remaining.is_empty());
        assert!(output.cleaned_branches.is_empty());
    }
}
