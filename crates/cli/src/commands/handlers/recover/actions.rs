//! Action functions for the recover command handler (Tier 3).
//!
//! I/O operations that orchestrate recovery of corrupted/incomplete sessions.
//! All validation and pure computation is in data.rs (Tier 2).
//!
//! This module is Git-only (no JJ operations). Recovery strategies use
//! `git reflog`, `git reset`, `git stash`, and workspace inspection.

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use scp_core::{vcs, Error, Result};

use super::data::{
    compute_status, count_fixed, count_remaining, Issue, RecoverOptions, RecoverOutput,
    RollbackOptions, RollbackOutput,
};
use crate::commands::handlers::done::executor::{ExecutorError, GitExecutor, RealGitExecutor};

// ============================================================================
// Public API
// ============================================================================

/// Execute the recover command with the given options.
///
/// Diagnoses common issues (missing Git, uninitialized repo, orphaned worktrees,
/// detached HEAD, uncommitted changes in stale sessions) and optionally fixes them.
///
/// # Errors
///
/// Returns errors for VCS operation failures or workspace validation failures.
pub fn run_recover(options: &RecoverOptions) -> Result<RecoverOutput> {
    let cwd = std::env::current_dir()?;
    let executor = RealGitExecutor::new();

    let mut issues = diagnose_issues(&cwd, &executor)?;

    // If a specific target was provided, filter to that target's issues
    if let Some(ref target) = options.target {
        issues.retain(|issue| {
            issue.description.contains(target.as_str())
                || issue.code.contains("WORKTREE")
                || issue.code.contains("REPO")
                || issue.code.contains("GIT")
        });
    }

    // Apply fixes unless diagnose-only or dry-run
    let issues = if options.diagnose_only || options.dry_run {
        issues
    } else {
        fix_issues(issues, &executor)?
    };

    let fixed_count = count_fixed(&issues);
    let remaining_count = count_remaining(&issues);
    let status = compute_status(&issues);

    Ok(RecoverOutput {
        issues,
        fixed_count,
        remaining_count,
        status,
    })
}

/// Execute the rollback command with the given options.
///
/// Rolls back a workspace/session to a specific commit using `git reset --hard`.
/// In dry-run mode, only validates that the commit exists.
///
/// # Errors
///
/// Returns errors if the session workspace is not found, the commit doesn't exist,
/// or the Git reset operation fails.
pub fn run_rollback(options: &RollbackOptions) -> Result<RollbackOutput> {
    let cwd = std::env::current_dir()?;
    let backend = vcs::create_backend(&cwd)?;
    let executor = RealGitExecutor::new();

    // Resolve workspace path from workspace name
    let workspace_path_buf = resolve_workspace_path(&cwd, backend.as_ref(), &options.session)?;
    let workspace_path = workspace_path_buf
        .to_str()
        .ok_or_else(|| Error::invalid_state("workspace path contains invalid UTF-8"))?;

    // Verify the workspace directory exists
    if !Path::new(workspace_path).exists() {
        return Ok(RollbackOutput {
            session: options.session.clone(),
            commit: options.commit.clone(),
            dry_run: options.dry_run,
            succeeded: false,
            message: format!("Workspace directory '{}' does not exist", workspace_path),
        });
    }

    // Verify the commit exists in the workspace
    let verify_result =
        executor.run_in_workspace(&["cat-file", "-t", &options.commit], workspace_path);

    match verify_result {
        Ok(output) if output.trim() == "commit" => {}
        Ok(_) => {
            return Ok(RollbackOutput {
                session: options.session.clone(),
                commit: options.commit.clone(),
                dry_run: options.dry_run,
                succeeded: false,
                message: format!("'{}' is not a valid commit", options.commit),
            });
        }
        Err(_) => {
            return Ok(RollbackOutput {
                session: options.session.clone(),
                commit: options.commit.clone(),
                dry_run: options.dry_run,
                succeeded: false,
                message: format!("Commit '{}' not found in workspace", options.commit),
            });
        }
    }

    if options.dry_run {
        return Ok(RollbackOutput {
            session: options.session.clone(),
            commit: options.commit.clone(),
            dry_run: true,
            succeeded: true,
            message: format!(
                "Would roll back session '{}' to commit '{}'",
                options.session, options.commit
            ),
        });
    }

    // Perform the rollback using git reset --hard
    let reset_result =
        executor.run_in_workspace(&["reset", "--hard", &options.commit], workspace_path);

    match reset_result {
        Ok(_) => Ok(RollbackOutput {
            session: options.session.clone(),
            commit: options.commit.clone(),
            dry_run: false,
            succeeded: true,
            message: format!(
                "Rolled back session '{}' to commit '{}'",
                options.session, options.commit
            ),
        }),
        Err(ExecutorError::CommandFailed { stderr, .. }) => Ok(RollbackOutput {
            session: options.session.clone(),
            commit: options.commit.clone(),
            dry_run: false,
            succeeded: false,
            message: format!("Rollback failed: {}", stderr.trim()),
        }),
        Err(e) => Err(Error::from(e)),
    }
}

// ============================================================================
// Diagnosis
// ============================================================================

/// Diagnose common issues with the repository and worktrees.
fn diagnose_issues(repo_path: &Path, executor: &dyn GitExecutor) -> Result<Vec<Issue>> {
    let mut issues = Vec::new();

    // Check 1: Is git available?
    check_git_available(&mut issues, executor);

    // Check 2: Is the repo initialized?
    check_repo_initialized(repo_path, &mut issues);

    // Check 3: Is HEAD detached?
    check_detached_head(&mut issues, executor);

    // Check 4: Check for orphaned/stale worktrees
    check_worktrees(&mut issues, executor);

    // Check 5: Check for merge conflicts
    check_merge_conflicts(&mut issues, executor);

    Ok(issues)
}

/// Check if the git command is available.
fn check_git_available(issues: &mut Vec<Issue>, executor: &dyn GitExecutor) {
    if let Err(ExecutorError::CommandNotFound(_)) = executor.run(&["--version"]) {
        issues.push(Issue {
            code: "GIT_NOT_INSTALLED".to_string(),
            description: "Git is not installed or not in PATH".to_string(),
            severity: "critical".to_string(),
            fix_command: Some("Install git: https://git-scm.com/downloads".to_string()),
            fixed: false,
        });
    }
}

/// Check if the current directory is a Git repository.
fn check_repo_initialized(repo_path: &Path, issues: &mut Vec<Issue>) {
    let git_dir = repo_path.join(".git");
    if !git_dir.exists() {
        issues.push(Issue {
            code: "GIT_NOT_INITIALIZED".to_string(),
            description: "Current directory is not a Git repository".to_string(),
            severity: "critical".to_string(),
            fix_command: Some("git init".to_string()),
            fixed: false,
        });
    }
}

/// Check if HEAD is detached (not on a branch).
fn check_detached_head(issues: &mut Vec<Issue>, executor: &dyn GitExecutor) {
    match executor.run(&["symbolic-ref", "-q", "HEAD"]) {
        Ok(_) => {
            // On a branch - all good
        }
        Err(ExecutorError::CommandFailed { code: 1, .. }) => {
            // Detached HEAD
            issues.push(Issue {
                code: "DETACHED_HEAD".to_string(),
                description: "HEAD is detached (not on any branch)".to_string(),
                severity: "warning".to_string(),
                fix_command: Some("git checkout <branch-name>".to_string()),
                fixed: false,
            });
        }
        Err(_) => {
            // Some other error (e.g., not in a repo) - skip, handled elsewhere
        }
    }
}

/// Check worktrees for orphaned or stale entries.
fn check_worktrees(issues: &mut Vec<Issue>, executor: &dyn GitExecutor) {
    match executor.run(&["worktree", "list", "--porcelain"]) {
        Ok(output) => {
            let mut worktree_paths: Vec<String> = Vec::new();
            let mut current_path: Option<String> = None;

            for line in output.lines() {
                if line.starts_with("worktree ") {
                    current_path = Some(line.strip_prefix("worktree ").unwrap_or(line).to_string());
                } else if line.is_empty() {
                    if let Some(path) = current_path.take() {
                        worktree_paths.push(path);
                    }
                }
            }
            // Handle last entry if no trailing newline
            if let Some(path) = current_path.take() {
                worktree_paths.push(path);
            }

            for wt_path in &worktree_paths {
                if !Path::new(wt_path).exists() {
                    issues.push(Issue {
                        code: "ORPHANED_WORKTREE".to_string(),
                        description: format!("Worktree directory missing: {}", wt_path),
                        severity: "warning".to_string(),
                        fix_command: Some("git worktree prune".to_string()),
                        fixed: false,
                    });
                }
            }

            // Check for stale worktree entries (prunable)
            match executor.run(&["worktree", "prune", "--dry-run"]) {
                Ok(output) if !output.trim().is_empty() => {
                    let count = output.trim().lines().count();
                    issues.push(Issue {
                        code: "STALE_WORKTREES".to_string(),
                        description: format!("{} stale worktree(s) can be pruned", count),
                        severity: "info".to_string(),
                        fix_command: Some("git worktree prune".to_string()),
                        fixed: false,
                    });
                }
                _ => {}
            }
        }
        Err(_) => {
            // worktree list not supported or not in repo - skip
        }
    }
}

/// Check for unresolved merge conflicts.
fn check_merge_conflicts(issues: &mut Vec<Issue>, executor: &dyn GitExecutor) {
    match executor.run(&["ls-files", "--unmerged"]) {
        Ok(output) if !output.trim().is_empty() => {
            let conflicted_files: Vec<String> = output
                .lines()
                .filter_map(|line| {
                    let parts: Vec<&str> = line.splitn(2, '\t').collect();
                    parts.get(1).map(|f| (*f).to_string())
                })
                .collect();

            let unique_files: Vec<String> = {
                let mut set = BTreeSet::new();
                for f in conflicted_files {
                    set.insert(f);
                }
                set.into_iter().collect()
            };

            if !unique_files.is_empty() {
                issues.push(Issue {
                    code: "MERGE_CONFLICTS".to_string(),
                    description: format!(
                        "{} file(s) with unresolved merge conflicts: {}",
                        unique_files.len(),
                        unique_files.join(", ")
                    ),
                    severity: "critical".to_string(),
                    fix_command: Some(
                        "Resolve conflicts, then git add <files> && git commit".to_string(),
                    ),
                    fixed: false,
                });
            }
        }
        _ => {}
    }
}

// ============================================================================
// Fixing
// ============================================================================

/// Attempt to fix issues where possible.
fn fix_issues(issues: Vec<Issue>, executor: &dyn GitExecutor) -> Result<Vec<Issue>> {
    issues
        .into_iter()
        .map(|issue| try_fix_issue(issue, executor))
        .collect()
}

/// Try to fix a single issue, returning the updated issue.
fn try_fix_issue(issue: Issue, executor: &dyn GitExecutor) -> Result<Issue> {
    match issue.code.as_str() {
        "STALE_WORKTREES" => match executor.run(&["worktree", "prune"]) {
            Ok(_) => Ok(Issue {
                fixed: true,
                ..issue
            }),
            Err(_) => Ok(issue),
        },
        "DETACHED_HEAD" => {
            // Attempt to find the default branch and checkout
            let branch = find_default_branch(executor);
            match branch {
                Some(ref b) => match executor.run(&["checkout", b]) {
                    Ok(_) => Ok(Issue {
                        fixed: true,
                        fix_command: Some(format!("git checkout {b}")),
                        ..issue
                    }),
                    Err(_) => Ok(issue),
                },
                None => Ok(issue),
            }
        }
        // All other issues require user intervention
        _ => Ok(issue),
    }
}

/// Find the default branch name (main or master).
fn find_default_branch(executor: &dyn GitExecutor) -> Option<String> {
    for branch in &["main", "master"] {
        if let Ok(output) = executor.run(&["rev-parse", "--verify", branch]) {
            if !output.trim().is_empty() {
                return Some(branch.to_string());
            }
        }
    }
    None
}

// ============================================================================
// Helpers
// ============================================================================

/// Resolve a workspace name to its filesystem path.
///
/// Follows the same pattern as the done handler:
/// - If the workspace is the current one, returns cwd.
/// - Otherwise, looks for `<cwd>/.git/worktrees/<name>`.
fn resolve_workspace_path(
    cwd: &Path,
    backend: &dyn vcs::VcsBackend,
    workspace_name: &str,
) -> Result<PathBuf> {
    let workspaces = backend.list_workspaces()?;
    let is_current = workspaces
        .iter()
        .any(|w| w.name == workspace_name && w.is_current);

    if is_current {
        Ok(cwd.to_path_buf())
    } else {
        // For non-current workspaces, the path is typically <repo>/.git/worktrees/<name>
        let workspace_path = cwd.join(".git").join("worktrees").join(workspace_name);
        if workspace_path.exists() {
            Ok(workspace_path)
        } else {
            Ok(cwd.to_path_buf())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    // ---- Mock GitExecutor for recover tests ----

    struct MockRecoverExecutor {
        responses: HashMap<String, std::result::Result<String, ExecutorError>>,
    }

    impl MockRecoverExecutor {
        fn new() -> Self {
            Self {
                responses: HashMap::new(),
            }
        }

        fn with_response(
            mut self,
            key: &str,
            response: std::result::Result<String, ExecutorError>,
        ) -> Self {
            self.responses.insert(key.to_string(), response);
            self
        }

        fn with_ok(self, key: &str, response: &str) -> Self {
            self.with_response(key, Ok(response.to_string()))
        }

        fn with_err(self, key: &str, err: ExecutorError) -> Self {
            self.with_response(key, Err(err))
        }
    }

    impl GitExecutor for MockRecoverExecutor {
        fn run(&self, args: &[&str]) -> std::result::Result<String, ExecutorError> {
            let key = args.join(" ");
            self.responses.get(&key).cloned().unwrap_or_else(|| {
                Err(ExecutorError::CommandFailed {
                    code: 1,
                    stderr: format!("no mock for: {key}"),
                })
            })
        }

        fn run_in_workspace(
            &self,
            args: &[&str],
            _workspace_path: &str,
        ) -> std::result::Result<String, ExecutorError> {
            self.run(args)
        }
    }

    // ---- check_git_available ----

    #[test]
    fn check_git_available_when_git_present_no_issue() {
        let mock = MockRecoverExecutor::new().with_ok("--version", "git version 2.45.0");
        let mut issues = Vec::new();
        check_git_available(&mut issues, &mock);
        assert!(issues.is_empty());
    }

    #[test]
    fn check_git_available_when_git_missing_adds_critical() {
        let mock = MockRecoverExecutor::new().with_err(
            "--version",
            ExecutorError::CommandNotFound("git not found".to_string()),
        );
        let mut issues = Vec::new();
        check_git_available(&mut issues, &mock);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, "GIT_NOT_INSTALLED");
        assert_eq!(issues[0].severity, "critical");
    }

    // ---- check_detached_head ----

    #[test]
    fn check_detached_head_on_branch_no_issue() {
        let mock = MockRecoverExecutor::new().with_ok("symbolic-ref -q HEAD", "refs/heads/main");
        let mut issues = Vec::new();
        check_detached_head(&mut issues, &mock);
        assert!(issues.is_empty());
    }

    #[test]
    fn check_detached_head_detached_adds_warning() {
        let mock = MockRecoverExecutor::new().with_err(
            "symbolic-ref -q HEAD",
            ExecutorError::CommandFailed {
                code: 1,
                stderr: "not a symbolic ref".to_string(),
            },
        );
        let mut issues = Vec::new();
        check_detached_head(&mut issues, &mock);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, "DETACHED_HEAD");
        assert_eq!(issues[0].severity, "warning");
    }

    // ---- check_merge_conflicts ----

    #[test]
    fn check_merge_conflicts_clean_no_issue() {
        let mock = MockRecoverExecutor::new().with_ok("ls-files --unmerged", "");
        let mut issues = Vec::new();
        check_merge_conflicts(&mut issues, &mock);
        assert!(issues.is_empty());
    }

    #[test]
    fn check_merge_conflicts_with_conflicts_adds_critical() {
        let mock = MockRecoverExecutor::new().with_ok(
            "ls-files --unmerged",
            "100644 abc123 1\tfile.rs\n100644 def456 2\tfile.rs\n100644 ghi789 3\tfile.rs",
        );
        let mut issues = Vec::new();
        check_merge_conflicts(&mut issues, &mock);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, "MERGE_CONFLICTS");
        assert_eq!(issues[0].severity, "critical");
        assert!(issues[0].description.contains("file.rs"));
    }

    // ---- check_worktrees ----

    #[test]
    fn check_worktrees_stale_adds_info() {
        let mock = MockRecoverExecutor::new()
            .with_ok("worktree list --porcelain", "worktree /tmp/main\n\n")
            .with_ok("worktree prune --dry-run", "Removing /tmp/stale\n");
        let mut issues = Vec::new();
        check_worktrees(&mut issues, &mock);
        assert!(issues.iter().any(|i| i.code == "STALE_WORKTREES"));
    }

    // ---- find_default_branch ----

    #[test]
    fn find_default_branch_main_first() {
        let mock = MockRecoverExecutor::new().with_ok("rev-parse --verify main", "abc123\n");
        let result = find_default_branch(&mock);
        assert_eq!(result.as_deref(), Some("main"));
    }

    #[test]
    fn find_default_branch_falls_back_to_master() {
        let mock = MockRecoverExecutor::new()
            .with_err(
                "rev-parse --verify main",
                ExecutorError::CommandFailed {
                    code: 128,
                    stderr: "unknown revision".to_string(),
                },
            )
            .with_ok("rev-parse --verify master", "def456\n");
        let result = find_default_branch(&mock);
        assert_eq!(result.as_deref(), Some("master"));
    }

    #[test]
    fn find_default_branch_none_when_no_branches() {
        let mock = MockRecoverExecutor::new()
            .with_err(
                "rev-parse --verify main",
                ExecutorError::CommandFailed {
                    code: 128,
                    stderr: "not found".to_string(),
                },
            )
            .with_err(
                "rev-parse --verify master",
                ExecutorError::CommandFailed {
                    code: 128,
                    stderr: "not found".to_string(),
                },
            );
        let result = find_default_branch(&mock);
        assert!(result.is_none());
    }

    // ---- try_fix_issue ----

    #[test]
    fn try_fix_stale_worktrees_succeeds() {
        let mock = MockRecoverExecutor::new().with_ok("worktree prune", "");
        let issue = Issue {
            code: "STALE_WORKTREES".to_string(),
            description: "2 stale worktrees".to_string(),
            severity: "info".to_string(),
            fix_command: Some("git worktree prune".to_string()),
            fixed: false,
        };
        let result = try_fix_issue(issue, &mock).expect("should succeed");
        assert!(result.fixed);
    }

    #[test]
    fn try_fix_stale_worktrees_fails_gracefully() {
        let mock = MockRecoverExecutor::new().with_err(
            "worktree prune",
            ExecutorError::CommandFailed {
                code: 1,
                stderr: "permission denied".to_string(),
            },
        );
        let issue = Issue {
            code: "STALE_WORKTREES".to_string(),
            description: "stale".to_string(),
            severity: "info".to_string(),
            fix_command: Some("git worktree prune".to_string()),
            fixed: false,
        };
        let result = try_fix_issue(issue, &mock).expect("should succeed");
        assert!(!result.fixed);
    }

    #[test]
    fn try_fix_detached_head_succeeds_with_main() {
        let mock = MockRecoverExecutor::new()
            .with_ok("rev-parse --verify main", "abc123\n")
            .with_ok("checkout main", "Switched to branch 'main'\n");
        let issue = Issue {
            code: "DETACHED_HEAD".to_string(),
            description: "HEAD detached".to_string(),
            severity: "warning".to_string(),
            fix_command: Some("git checkout <branch>".to_string()),
            fixed: false,
        };
        let result = try_fix_issue(issue, &mock).expect("should succeed");
        assert!(result.fixed);
    }

    #[test]
    fn try_fix_unfixable_issue_unchanged() {
        let mock = MockRecoverExecutor::new();
        let issue = Issue {
            code: "GIT_NOT_INSTALLED".to_string(),
            description: "git missing".to_string(),
            severity: "critical".to_string(),
            fix_command: Some("install git".to_string()),
            fixed: false,
        };
        let result = try_fix_issue(issue, &mock).expect("should succeed");
        assert!(!result.fixed);
        assert_eq!(result.code, "GIT_NOT_INSTALLED");
    }

    // ---- compute_status integration ----

    #[test]
    fn run_recover_diagnose_only_does_not_fix() {
        let issues = vec![Issue {
            code: "STALE_WORKTREES".to_string(),
            description: "stale".to_string(),
            severity: "info".to_string(),
            fix_command: Some("git worktree prune".to_string()),
            fixed: false,
        }];
        let status = compute_status(&issues);
        // Info-severity unfixed -> healthy
        assert_eq!(status, "healthy");
    }
}
