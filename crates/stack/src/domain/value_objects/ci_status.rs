//! CI check value objects for detailed status monitoring.
//!
//! Ported from stax commands/ci.rs. Contains data types for representing
//! individual check runs and per-branch CI status aggregates.

use serde::{Deserialize, Serialize};

/// Detailed info about a single CI check run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckRunInfo {
    pub name: String,
    /// "completed", "in_progress", "queued", "waiting", "requested", "pending"
    pub status: String,
    /// "success", "failure", "timed_out", "skipped", "neutral", "cancelled", "action_required"
    pub conclusion: Option<String>,
    pub url: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub elapsed_secs: Option<u64>,
    pub average_secs: Option<u64>,
    pub completion_percent: Option<u8>,
}

/// Aggregated CI status for a branch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchCiStatus {
    pub branch: String,
    pub sha: String,
    pub sha_short: String,
    pub overall_status: Option<String>,
    pub check_runs: Vec<CheckRunInfo>,
    pub pr_number: Option<u64>,
}

/// Sort key: failures first (0), running (1), passed (2), skipped/other (3).
pub fn check_sort_key(c: &CheckRunInfo) -> u8 {
    match c.status.as_str() {
        "completed" => match c.conclusion.as_deref() {
            Some("failure") | Some("timed_out") | Some("action_required") => 0,
            Some("success") => 2,
            _ => 3,
        },
        "in_progress" | "queued" | "waiting" | "requested" | "pending" => 1,
        _ => 3,
    }
}

/// Deduplicate check runs by name, keeping only the most recent for each.
pub fn dedup_check_runs(check_runs: Vec<CheckRunInfo>) -> Vec<CheckRunInfo> {
    use chrono::{DateTime, Utc};
    use std::collections::HashMap;

    let mut unique_checks: HashMap<String, CheckRunInfo> = HashMap::new();
    for check in check_runs {
        let should_replace = if let Some(existing) = unique_checks.get(&check.name) {
            match (&check.started_at, &existing.started_at) {
                (Some(new_start), Some(existing_start)) => {
                    let new_time = new_start.parse::<DateTime<Utc>>();
                    let existing_time = existing_start.parse::<DateTime<Utc>>();
                    match (new_time, existing_time) {
                        (Ok(nt), Ok(et)) => nt > et,
                        _ => false,
                    }
                }
                (Some(_), None) => true,
                (None, Some(_)) => false,
                (None, None) => true,
            }
        } else {
            true
        };

        if should_replace {
            unique_checks.insert(check.name.clone(), check);
        }
    }

    let mut result: Vec<CheckRunInfo> = unique_checks.into_values().collect();
    result.sort_by(|a, b| a.name.cmp(&b.name));
    result
}

/// Format duration in seconds to human-readable string.
pub fn format_duration(secs: u64) -> String {
    match secs {
        0..60 => format!("{secs}s"),
        60..3600 => {
            let mins = secs / 60;
            let secs_remainder = secs % 60;
            if secs_remainder == 0 {
                format!("{mins}m")
            } else {
                format!("{mins}m {secs_remainder}s")
            }
        }
        _ => {
            let hours = secs / 3600;
            let mins = (secs % 3600) / 60;
            if mins == 0 {
                format!("{hours}h")
            } else {
                format!("{hours}h {mins}m")
            }
        }
    }
}

/// Check if all CI checks are complete (not pending).
pub fn all_checks_complete(statuses: &[BranchCiStatus]) -> bool {
    statuses.iter().all(|s| {
        s.check_runs.is_empty() || s.overall_status.as_deref() != Some("pending")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration_seconds() {
        assert_eq!(format_duration(0), "0s");
        assert_eq!(format_duration(30), "30s");
        assert_eq!(format_duration(59), "59s");
    }

    #[test]
    fn test_format_duration_minutes() {
        assert_eq!(format_duration(60), "1m");
        assert_eq!(format_duration(90), "1m 30s");
        assert_eq!(format_duration(150), "2m 30s");
        assert_eq!(format_duration(3599), "59m 59s");
    }

    #[test]
    fn test_format_duration_hours() {
        assert_eq!(format_duration(3600), "1h");
        assert_eq!(format_duration(3660), "1h 1m");
        assert_eq!(format_duration(7200), "2h");
        assert_eq!(format_duration(7320), "2h 2m");
    }

    #[test]
    fn test_dedup_check_runs_keeps_most_recent() {
        let older = CheckRunInfo {
            name: "build".to_string(),
            status: "completed".to_string(),
            conclusion: Some("success".to_string()),
            url: None,
            started_at: Some("2026-01-16T12:00:00Z".to_string()),
            completed_at: Some("2026-01-16T12:02:00Z".to_string()),
            elapsed_secs: Some(120),
            average_secs: None,
            completion_percent: None,
        };
        let newer = CheckRunInfo {
            name: "build".to_string(),
            status: "completed".to_string(),
            conclusion: Some("failure".to_string()),
            url: None,
            started_at: Some("2026-01-16T13:00:00Z".to_string()),
            completed_at: Some("2026-01-16T13:02:00Z".to_string()),
            elapsed_secs: Some(120),
            average_secs: None,
            completion_percent: None,
        };

        let result = dedup_check_runs(vec![older, newer]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].conclusion, Some("failure".to_string()));
    }

    #[test]
    fn test_dedup_check_runs_different_names() {
        let build = CheckRunInfo {
            name: "build".to_string(),
            status: "completed".to_string(),
            conclusion: Some("success".to_string()),
            url: None,
            started_at: None,
            completed_at: None,
            elapsed_secs: None,
            average_secs: None,
            completion_percent: None,
        };
        let test = CheckRunInfo {
            name: "test".to_string(),
            status: "in_progress".to_string(),
            conclusion: None,
            url: None,
            started_at: None,
            completed_at: None,
            elapsed_secs: None,
            average_secs: None,
            completion_percent: None,
        };

        let result = dedup_check_runs(vec![build, test]);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_check_sort_key_ordering() {
        let failed = CheckRunInfo {
            name: "a".to_string(),
            status: "completed".to_string(),
            conclusion: Some("failure".to_string()),
            url: None,
            started_at: None,
            completed_at: None,
            elapsed_secs: None,
            average_secs: None,
            completion_percent: None,
        };
        let running = CheckRunInfo {
            name: "b".to_string(),
            status: "in_progress".to_string(),
            conclusion: None,
            url: None,
            started_at: None,
            completed_at: None,
            elapsed_secs: None,
            average_secs: None,
            completion_percent: None,
        };
        let passed = CheckRunInfo {
            name: "c".to_string(),
            status: "completed".to_string(),
            conclusion: Some("success".to_string()),
            url: None,
            started_at: None,
            completed_at: None,
            elapsed_secs: None,
            average_secs: None,
            completion_percent: None,
        };

        assert!(check_sort_key(&failed) < check_sort_key(&running));
        assert!(check_sort_key(&running) < check_sort_key(&passed));
    }

    #[test]
    fn test_all_checks_complete_empty() {
        assert!(all_checks_complete(&[]));
    }

    #[test]
    fn test_all_checks_complete_no_ci() {
        let status = BranchCiStatus {
            branch: "foo".to_string(),
            sha: "abc".to_string(),
            sha_short: "abc".to_string(),
            overall_status: None,
            check_runs: vec![],
            pr_number: None,
        };
        assert!(all_checks_complete(&[status]));
    }

    #[test]
    fn test_all_checks_complete_pending() {
        let status = BranchCiStatus {
            branch: "foo".to_string(),
            sha: "abc".to_string(),
            sha_short: "abc".to_string(),
            overall_status: Some("pending".to_string()),
            check_runs: vec![CheckRunInfo {
                name: "build".to_string(),
                status: "in_progress".to_string(),
                conclusion: None,
                url: None,
                started_at: None,
                completed_at: None,
                elapsed_secs: None,
                average_secs: None,
                completion_percent: None,
            }],
            pr_number: None,
        };
        assert!(!all_checks_complete(&[status]));
    }

    #[test]
    fn test_check_run_info_serialization() {
        let info = CheckRunInfo {
            name: "build".to_string(),
            status: "completed".to_string(),
            conclusion: Some("success".to_string()),
            url: Some("https://github.com/test/test/runs/123".to_string()),
            started_at: Some("2026-01-16T12:00:00Z".to_string()),
            completed_at: Some("2026-01-16T12:02:30Z".to_string()),
            elapsed_secs: Some(150),
            average_secs: Some(160),
            completion_percent: None,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("build"));
        assert!(json.contains("completed"));
        assert!(json.contains("success"));

        let deserialized: CheckRunInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "build");
        assert_eq!(deserialized.status, "completed");
        assert_eq!(deserialized.conclusion, Some("success".to_string()));
        assert_eq!(deserialized.elapsed_secs, Some(150));
    }

    #[test]
    fn test_branch_ci_status_serialization() {
        let status = BranchCiStatus {
            branch: "feature-branch".to_string(),
            sha: "abc123def456".to_string(),
            sha_short: "abc123d".to_string(),
            overall_status: Some("success".to_string()),
            check_runs: vec![CheckRunInfo {
                name: "build".to_string(),
                status: "completed".to_string(),
                conclusion: Some("success".to_string()),
                url: None,
                started_at: None,
                completed_at: None,
                elapsed_secs: None,
                average_secs: None,
                completion_percent: None,
            }],
            pr_number: Some(42),
        };

        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("feature-branch"));
        assert!(json.contains("abc123def456"));
        assert!(json.contains("success"));
        assert!(json.contains("42"));
    }
}
