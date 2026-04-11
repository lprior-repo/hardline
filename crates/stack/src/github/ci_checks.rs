//! Detailed CI check fetching with timing information.
//!
//! Extends GitHubClient with methods for fetching individual check run
//! details including elapsed time, completion status, and deduplication.

use chrono::{DateTime, Utc};
use serde::Deserialize;

use super::client::GitHubClient;
use crate::domain::value_objects::{dedup_check_runs, BranchCiStatus, CheckRunInfo};
use crate::error::{Result, StackError};

// ── API response types ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct DetailedCheckRunsResponse {
    total_count: usize,
    check_runs: Vec<DetailedCheckRun>,
}

#[derive(Debug, Deserialize)]
struct DetailedCheckRun {
    name: String,
    status: String,
    conclusion: Option<String>,
    html_url: Option<String>,
    started_at: Option<String>,
    completed_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CommitStatus {
    context: String,
    state: String,
    target_url: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
}

// ── Impl ───────────────────────────────────────────────────────────────

impl GitHubClient {
    /// Fetch detailed CI status for multiple branches.
    ///
    /// Returns a `BranchCiStatus` for each branch with individual check run
    /// details including timing information.
    pub async fn fetch_branch_ci_statuses(
        &self,
        branches: &[(String, String, Option<u64>)], // (branch, sha, pr_number)
    ) -> Result<Vec<BranchCiStatus>> {
        let mut statuses = Vec::new();

        for (branch, sha, pr_number) in branches {
            let sha_short: String = sha.chars().take(7).collect();

            let (overall_status, check_runs) = self
                .fetch_detailed_checks(sha)
                .await
                .unwrap_or((None, Vec::new()));

            statuses.push(BranchCiStatus {
                branch: branch.clone(),
                sha: sha.clone(),
                sha_short,
                overall_status,
                check_runs,
                pr_number: *pr_number,
            });
        }

        statuses.sort_by(|a, b| a.branch.cmp(&b.branch));
        Ok(statuses)
    }

    /// Fetch all checks (both check runs and commit statuses) with timing.
    ///
    /// Returns combined overall status and deduplicated check runs.
    pub async fn fetch_detailed_checks(
        &self,
        commit_sha: &str,
    ) -> Result<(Option<String>, Vec<CheckRunInfo>)> {
        let (check_runs_overall, mut all_checks) =
            self.fetch_check_runs_detailed(commit_sha).await?;
        let (statuses_overall, status_checks) =
            self.fetch_commit_statuses_detailed(commit_sha).await?;

        all_checks.extend(status_checks);
        all_checks = dedup_check_runs(all_checks);

        let combined_overall = match (check_runs_overall, statuses_overall) {
            (Some(ref a), Some(ref b)) if a == "failure" || b == "failure" => {
                Some("failure".to_string())
            }
            (Some(ref a), Some(ref b)) if a == "pending" || b == "pending" => {
                Some("pending".to_string())
            }
            (Some(a), Some(_)) => Some(a),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };

        Ok((combined_overall, all_checks))
    }

    async fn fetch_check_runs_detailed(
        &self,
        commit_sha: &str,
    ) -> Result<(Option<String>, Vec<CheckRunInfo>)> {
        let url = format!(
            "/repos/{}/{}/commits/{}/check-runs",
            self.owner, self.repo, commit_sha
        );

        let response: DetailedCheckRunsResponse = self
            .octocrab
            .get(&url, None::<&()>)
            .await
            .map_err(|e| StackError::GitHubError(format!("Check runs request failed: {e}")))?;

        if response.total_count == 0 {
            return Ok((None, Vec::new()));
        }

        let now = Utc::now();
        let mut check_runs: Vec<CheckRunInfo> = Vec::new();

        for r in response.check_runs {
            let (elapsed_secs, completed_at_str) =
                compute_elapsed(&r.started_at, &r.completed_at, &now);

            let completion_percent = if r.status == "in_progress" {
                elapsed_secs.and_then(|e| {
                    let avg = 0u64; // No history in hardline yet
                    if avg > 0 && e > 0 {
                        Some(((e * 100) / avg).min(99) as u8)
                    } else {
                        None
                    }
                })
            } else {
                None
            };

            check_runs.push(CheckRunInfo {
                name: r.name,
                status: r.status,
                conclusion: r.conclusion,
                url: r.html_url,
                started_at: r.started_at,
                completed_at: completed_at_str,
                elapsed_secs,
                average_secs: None, // History tracking not yet ported
                completion_percent,
            });
        }

        check_runs = dedup_check_runs(check_runs);

        let overall = compute_overall_from_checks(&check_runs);
        Ok((overall, check_runs))
    }

    async fn fetch_commit_statuses_detailed(
        &self,
        commit_sha: &str,
    ) -> Result<(Option<String>, Vec<CheckRunInfo>)> {
        let url = format!(
            "/repos/{}/{}/commits/{}/statuses",
            self.owner, self.repo, commit_sha
        );

        let statuses: Vec<CommitStatus> = match self.octocrab.get(&url, None::<&()>).await {
            Ok(s) => s,
            Err(_) => return Ok((None, Vec::new())),
        };

        if statuses.is_empty() {
            return Ok((None, Vec::new()));
        }

        let mut check_runs: Vec<CheckRunInfo> = Vec::new();

        for status in statuses {
            let (status_str, conclusion, elapsed_secs) = match status.state.as_str() {
                "success" => {
                    let elapsed = compute_status_elapsed(&status.created_at, &status.updated_at);
                    (
                        "completed".to_string(),
                        Some("success".to_string()),
                        elapsed,
                    )
                }
                "failure" | "error" => ("completed".to_string(), Some("failure".to_string()), None),
                "pending" => ("in_progress".to_string(), None, None),
                _ => ("queued".to_string(), None, None),
            };

            check_runs.push(CheckRunInfo {
                name: status.context,
                status: status_str,
                conclusion,
                url: status.target_url,
                started_at: status.created_at,
                completed_at: status.updated_at.clone(),
                elapsed_secs,
                average_secs: None,
                completion_percent: None,
            });
        }

        let overall = compute_overall_from_checks(&check_runs);
        Ok((overall, check_runs))
    }
}

// ── Pure helper functions ──────────────────────────────────────────────

fn compute_elapsed(
    started_at: &Option<String>,
    completed_at: &Option<String>,
    now: &DateTime<Utc>,
) -> (Option<u64>, Option<String>) {
    match (started_at.as_ref(), completed_at.as_ref()) {
        (Some(started), Some(completed)) => {
            let started_time = started.parse::<DateTime<Utc>>();
            let completed_time = completed.parse::<DateTime<Utc>>();
            match (started_time, completed_time) {
                (Ok(st), Ok(ct)) => {
                    let secs = ct.signed_duration_since(st).num_seconds();
                    if secs >= 0 {
                        (Some(secs as u64), Some(completed.clone()))
                    } else {
                        (None, Some(completed.clone()))
                    }
                }
                _ => (None, Some(completed.clone())),
            }
        }
        (Some(started), None) => {
            if let Ok(started_time) = started.parse::<DateTime<Utc>>() {
                let secs = now.signed_duration_since(started_time).num_seconds();
                if secs >= 0 {
                    (Some(secs as u64), None)
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            }
        }
        _ => (None, None),
    }
}

fn compute_status_elapsed(created_at: &Option<String>, updated_at: &Option<String>) -> Option<u64> {
    match (created_at.as_ref(), updated_at.as_ref()) {
        (Some(created), Some(updated)) => {
            let created_time = created.parse::<DateTime<Utc>>();
            let updated_time = updated.parse::<DateTime<Utc>>();
            match (created_time, updated_time) {
                (Ok(ct), Ok(ut)) => {
                    let duration = ut.signed_duration_since(ct);
                    Some(duration.num_seconds().max(0) as u64)
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn compute_overall_from_checks(check_runs: &[CheckRunInfo]) -> Option<String> {
    let mut has_pending = false;
    let mut has_failure = false;
    let mut all_success = true;

    for run in check_runs {
        match run.status.as_str() {
            "completed" => match run.conclusion.as_deref() {
                Some("success") | Some("skipped") | Some("neutral") | Some("cancelled") => {}
                Some("failure") | Some("timed_out") | Some("action_required") => {
                    has_failure = true;
                    all_success = false;
                }
                _ => {
                    all_success = false;
                }
            },
            "queued" | "in_progress" | "waiting" | "requested" | "pending" => {
                has_pending = true;
                all_success = false;
            }
            _ => {
                all_success = false;
            }
        }
    }

    if has_failure {
        Some("failure".to_string())
    } else if has_pending {
        Some("pending".to_string())
    } else if all_success && !check_runs.is_empty() {
        Some("success".to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_elapsed_with_both_timestamps() {
        let now = Utc::now();
        let started = Some("2026-01-16T12:00:00Z".to_string());
        let completed = Some("2026-01-16T12:02:30Z".to_string());
        let (elapsed, completed_at) = compute_elapsed(&started, &completed, &now);
        assert_eq!(elapsed, Some(150));
        assert_eq!(completed_at, Some("2026-01-16T12:02:30Z".to_string()));
    }

    #[test]
    fn test_compute_elapsed_with_started_only() {
        let started = Some("2026-01-16T12:00:00Z".to_string());
        let (elapsed, completed_at) = compute_elapsed(&started, &None, &Utc::now());
        assert!(elapsed.is_some());
        assert!(completed_at.is_none());
    }

    #[test]
    fn test_compute_elapsed_no_timestamps() {
        let now = Utc::now();
        let (elapsed, completed_at) = compute_elapsed(&None, &None, &now);
        assert_eq!(elapsed, None);
        assert_eq!(completed_at, None);
    }

    #[test]
    fn test_compute_overall_all_success() {
        let checks = vec![CheckRunInfo {
            name: "build".to_string(),
            status: "completed".to_string(),
            conclusion: Some("success".to_string()),
            url: None,
            started_at: None,
            completed_at: None,
            elapsed_secs: None,
            average_secs: None,
            completion_percent: None,
        }];
        assert_eq!(
            compute_overall_from_checks(&checks),
            Some("success".to_string())
        );
    }

    #[test]
    fn test_compute_overall_has_failure() {
        let checks = vec![CheckRunInfo {
            name: "build".to_string(),
            status: "completed".to_string(),
            conclusion: Some("failure".to_string()),
            url: None,
            started_at: None,
            completed_at: None,
            elapsed_secs: None,
            average_secs: None,
            completion_percent: None,
        }];
        assert_eq!(
            compute_overall_from_checks(&checks),
            Some("failure".to_string())
        );
    }

    #[test]
    fn test_compute_overall_has_pending() {
        let checks = vec![CheckRunInfo {
            name: "build".to_string(),
            status: "in_progress".to_string(),
            conclusion: None,
            url: None,
            started_at: None,
            completed_at: None,
            elapsed_secs: None,
            average_secs: None,
            completion_percent: None,
        }];
        assert_eq!(
            compute_overall_from_checks(&checks),
            Some("pending".to_string())
        );
    }

    #[test]
    fn test_compute_overall_empty() {
        assert_eq!(compute_overall_from_checks(&[]), None);
    }

    #[test]
    fn test_compute_status_elapsed() {
        let created = Some("2026-01-16T12:00:00Z".to_string());
        let updated = Some("2026-01-16T12:01:00Z".to_string());
        assert_eq!(compute_status_elapsed(&created, &updated), Some(60));
    }

    #[test]
    fn test_compute_status_elapsed_no_timestamps() {
        assert_eq!(compute_status_elapsed(&None, &None), None);
    }
}
