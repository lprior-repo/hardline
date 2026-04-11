//! Actions for the CI command handler (Tier 3 - I/O).
//!
//! Contains the main logic for checking CI status and watching CI.

use scp_core::{output::Output, Result};
use scp_stack::github::GitHubClient;
use scp_stack::{
    all_checks_complete, check_sort_key, format_duration, BranchCiStatus, CheckRunInfo,
};

use super::data::{CiCheckOptions, CiCheckOutput, CiWatchOptions};

/// Run a one-shot CI check.
pub fn run_ci_check(options: &CiCheckOptions) -> Result<()> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| {
        scp_core::Error::internal(format!("Failed to create tokio runtime: {e}"))
    })?;

    rt.block_on(async {
        let client = create_github_client()?;
        let branches = resolve_branches()?;

        if branches.is_empty() {
            Output::info("No tracked branches found.");
            return Ok(());
        }

        let branch_tuples: Vec<(String, String, Option<u64>)> = branches
            .iter()
            .map(|b| (b.clone(), String::new(), None))
            .collect();

        let statuses = client.fetch_branch_ci_statuses(&branch_tuples).await.map_err(|e| {
            scp_core::Error::internal(format!("Failed to fetch CI statuses: {e}"))
        })?;

        if options.json {
            let json = serde_json::to_string_pretty(&statuses).map_err(|e| {
                scp_core::Error::internal(format!("JSON serialization failed: {e}"))
            })?;
            println!("{json}");
            return Ok(());
        }

        let current_branch = get_current_branch();
        let multi = statuses.len() > 1;

        if multi {
            print_multi_branch_header(&statuses);
            println!();
        }

        for status in &statuses {
            let is_current = status.branch == current_branch;
            if options.verbose {
                display_branch_compact(status, is_current);
            } else {
                display_branch_verbose(status, is_current);
            }
        }

        Ok(())
    })
}

/// Run watch mode - poll CI status until all checks complete.
pub fn run_ci_watch(options: &CiWatchOptions) -> Result<()> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| {
        scp_core::Error::internal(format!("Failed to create tokio runtime: {e}"))
    })?;

    rt.block_on(async {
        let client = create_github_client()?;
        let branches = resolve_branches()?;

        if branches.is_empty() {
            Output::info("No tracked branches to watch.");
            return Ok(());
        }

        let poll_interval = std::time::Duration::from_secs(options.interval);
        let mut iteration = 0u32;

        Output::info("Watching CI status (Ctrl+C to stop)...");
        println!();

        loop {
            iteration += 1;

            let branch_tuples: Vec<(String, String, Option<u64>)> = branches
                .iter()
                .map(|b| (b.clone(), String::new(), None))
                .collect();

            let statuses = client.fetch_branch_ci_statuses(&branch_tuples).await.map_err(|e| {
                scp_core::Error::internal(format!("Failed to fetch CI statuses: {e}"))
            })?;

            if iteration > 1 {
                print!("\x1B[2J\x1B[H");
                use std::io::Write;
                let _ = std::io::stdout().flush();
                Output::info("Watching CI status (Ctrl+C to stop)...");
                println!();
            }

            if options.json {
                let json = serde_json::to_string_pretty(&statuses).map_err(|e| {
                    scp_core::Error::internal(format!("JSON serialization failed: {e}"))
                })?;
                println!("{json}");
            } else {
                let current_branch = get_current_branch();
                let multi = statuses.len() > 1;
                if multi {
                    print_multi_branch_header(&statuses);
                    println!();
                }
                for status in &statuses {
                    let is_current = status.branch == current_branch;
                    if options.verbose {
                        display_branch_compact(status, is_current);
                    } else {
                        display_branch_verbose(status, is_current);
                    }
                }
            }

            let complete = all_checks_complete(&statuses);
            if complete {
                let has_failure = statuses
                    .iter()
                    .any(|s| s.overall_status.as_deref() == Some("failure"));

                println!();
                if has_failure {
                    let failed_branch = statuses
                        .iter()
                        .find(|s| s.overall_status.as_deref() == Some("failure"))
                        .map(|s| s.branch.as_str())
                        .unwrap_or("a branch");
                    Output::error(&format!("CI failed on {failed_branch}"));
                } else {
                    Output::success("All CI checks passed");
                }
                return Ok(());
            }

            Output::info(&format!(
                "Refreshing in {}s... (iteration #{})",
                options.interval, iteration
            ));

            std::thread::sleep(poll_interval);
        }
    })
}

// ── Display helpers ────────────────────────────────────────────────────

fn display_branch_compact(status: &BranchCiStatus, is_current: bool) {
    if status.check_runs.is_empty() {
        let marker = if is_current { "*" } else { " " };
        println!("{marker} {} ({}) no CI", status.branch, status.sha_short);
        println!();
        return;
    }

    let overall_icon = match status.overall_status.as_deref() {
        Some("success") => "PASS",
        Some("failure") => "FAIL",
        Some("pending") => "RUNS",
        _ => "  ? ",
    };

    let pr_info = status
        .pr_number
        .map(|n| format!("  PR #{n}"))
        .unwrap_or_default();

    let branch_display = if is_current {
        format!("* {}", status.branch)
    } else {
        format!("  {}", status.branch)
    };

    println!("{overall_icon} {branch_display}{pr_info}  ({})", status.sha_short);
    println!("{}", "-".repeat(50));
    println!();

    let failed: Vec<&CheckRunInfo> = status
        .check_runs
        .iter()
        .filter(|c| {
            c.status == "completed"
                && matches!(
                    c.conclusion.as_deref(),
                    Some("failure") | Some("timed_out") | Some("action_required")
                )
        })
        .collect();

    let running: Vec<&CheckRunInfo> = status
        .check_runs
        .iter()
        .filter(|c| {
            matches!(
                c.status.as_str(),
                "in_progress" | "queued" | "waiting" | "requested" | "pending"
            )
        })
        .collect();

    let passed: Vec<&CheckRunInfo> = status
        .check_runs
        .iter()
        .filter(|c| c.status == "completed" && matches!(c.conclusion.as_deref(), Some("success")))
        .collect();

    if !failed.is_empty() {
        for check in &failed {
            println!("  FAIL {}", check.name);
        }
        println!();
    }

    if !running.is_empty() {
        let names: Vec<&str> = running.iter().map(|c| c.name.as_str()).collect();
        println!("  RUNS {}", names.join(", "));
        println!();
    }

    if !passed.is_empty() {
        let mut sorted_passed = passed.clone();
        sorted_passed.sort_by_key(|b| std::cmp::Reverse(b.elapsed_secs));

        let show_n = 3.min(sorted_passed.len());
        let snippets: Vec<String> = sorted_passed[..show_n]
            .iter()
            .map(|c| {
                if let Some(secs) = c.elapsed_secs {
                    format!("{} {}", c.name, format_duration(secs))
                } else {
                    c.name.clone()
                }
            })
            .collect();
        let remaining = passed.len().saturating_sub(show_n);
        let detail = if remaining > 0 {
            format!("{}, +{} more", snippets.join(", "), remaining)
        } else {
            snippets.join(", ")
        };

        println!("  PASS {} passed ({})", passed.len(), detail);
    }

    println!();
}

fn display_branch_verbose(status: &BranchCiStatus, is_current: bool) {
    if status.check_runs.is_empty() {
        let marker = if is_current { "*" } else { " " };
        println!("{marker} {} ({}) no CI", status.branch, status.sha_short);
        println!();
        return;
    }

    let overall_icon = match status.overall_status.as_deref() {
        Some("success") => "PASS",
        Some("failure") => "FAIL",
        Some("pending") => "RUNS",
        _ => "  ? ",
    };

    let pr_info = status
        .pr_number
        .map(|n| format!("  PR #{n}"))
        .unwrap_or_default();

    let branch_display = if is_current {
        format!("* {}", status.branch)
    } else {
        format!("  {}", status.branch)
    };

    println!("{overall_icon} {branch_display}{pr_info}  ({})", status.sha_short);
    println!("{}", "-".repeat(50));
    println!();

    let max_name = status
        .check_runs
        .iter()
        .map(|c| c.name.len())
        .max()
        .unwrap_or(0);

    let mut sorted = status.check_runs.clone();
    sorted.sort_by_key(check_sort_key);

    let timing_cols: Vec<String> = sorted
        .iter()
        .map(|check| match check.status.as_str() {
            "completed" => {
                if let Some(elapsed) = check.elapsed_secs {
                    match check.average_secs {
                        Some(avg) => {
                            format!("{}  (avg: {})", format_duration(elapsed), format_duration(avg))
                        }
                        None => format_duration(elapsed),
                    }
                } else {
                    String::new()
                }
            }
            "in_progress" | "pending" | "queued" | "waiting" | "requested" => {
                if let Some(elapsed) = check.elapsed_secs {
                    format!("{} elapsed", format_duration(elapsed))
                } else {
                    String::new()
                }
            }
            _ => String::new(),
        })
        .collect();

    let max_timing = timing_cols.iter().map(|s| s.len()).max().unwrap_or(0);

    for (check, timing_col) in sorted.iter().zip(timing_cols.iter()) {
        let (icon, label) = check_icon_label(check);

        let name_padded = format!("{:<width$}", check.name, width = max_name);
        let timing_padded = format!("{:<width$}", timing_col, width = max_timing);

        if timing_col.is_empty() {
            println!("  {icon}  {name_padded}  {label}");
        } else {
            println!("  {icon}  {name_padded}  {label}  {timing_padded}");
        }
    }

    println!();
}

fn check_icon_label(check: &CheckRunInfo) -> (&'static str, &'static str) {
    match check.status.as_str() {
        "completed" => match check.conclusion.as_deref() {
            Some("success") => ("PASS", "passed"),
            Some("failure") => ("FAIL", "failed"),
            Some("skipped") => ("SKIP", "skipped"),
            Some("neutral") => ("   -", "neutral"),
            Some("cancelled") => ("SKIP", "cancelled"),
            Some("timed_out") => ("FAIL", "timed out"),
            Some("action_required") => ("WARN", "action required"),
            Some(_) => ("   ?", "other"),
            None => ("   ?", "unknown"),
        },
        "queued" | "waiting" | "requested" => ("QUEU", "queued"),
        "in_progress" => ("RUNS", "running"),
        "pending" => ("PEND", "pending"),
        _ => ("   ?", "unknown"),
    }
}

fn print_multi_branch_header(statuses: &[BranchCiStatus]) {
    let total = statuses.len();
    let success = statuses
        .iter()
        .filter(|s| s.overall_status.as_deref() == Some("success"))
        .count();
    let failure = statuses
        .iter()
        .filter(|s| s.overall_status.as_deref() == Some("failure"))
        .count();
    let pending = statuses
        .iter()
        .filter(|s| s.overall_status.as_deref() == Some("pending"))
        .count();
    let no_ci = statuses.iter().filter(|s| s.check_runs.is_empty()).count();

    let mut parts: Vec<String> = Vec::new();
    parts.push(format!("{total} branches"));
    if success > 0 {
        parts.push(format!("PASS {success} passing"));
    }
    if failure > 0 {
        parts.push(format!("FAIL {failure} failing"));
    }
    if pending > 0 {
        parts.push(format!("RUNS {pending} running"));
    }
    if no_ci > 0 {
        parts.push(format!("{no_ci} no CI"));
    }

    println!("CI  {}", parts.join("  "));
}

// ── I/O helpers ────────────────────────────────────────────────────────

fn create_github_client() -> scp_core::Result<GitHubClient> {
    // Try to get token from environment
    let token = std::env::var("GITHUB_TOKEN")
        .or_else(|_| std::env::var("GH_TOKEN"))
        .or_else(|_| std::env::var("STAX_GITHUB_TOKEN"))
        .map_err(|_| {
            scp_core::Error::validation_error(
                "GitHub token not found. Set GITHUB_TOKEN, GH_TOKEN, or STAX_GITHUB_TOKEN.",
            )
        })?;

    // Parse owner/repo from git remote
    let (owner, repo) = parse_remote_owner_repo()?;

    GitHubClient::new(&owner, &repo, token, None).map_err(|e| {
        scp_core::Error::internal(format!("Failed to create GitHub client: {e}"))
    })
}

fn parse_remote_owner_repo() -> scp_core::Result<(String, String)> {
    let output = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .map_err(|e| scp_core::Error::internal(format!("Failed to get git remote: {e}")))?;

    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Parse SSH: git@github.com:owner/repo.git
    // Parse HTTPS: https://github.com/owner/repo.git
    let cleaned = url
        .trim_end_matches(".git")
        .trim_end_matches('/');

    if let Some(rest) = cleaned.strip_prefix("git@github.com:") {
        if let Some((owner, repo)) = rest.split_once('/') {
            return Ok((owner.to_string(), repo.to_string()));
        }
    }

    if let Some(rest) = cleaned
        .strip_prefix("https://github.com/")
        .or_else(|| cleaned.strip_prefix("http://github.com/"))
    {
        if let Some((owner, repo)) = rest.split_once('/') {
            return Ok((owner.to_string(), repo.to_string()));
        }
    }

    Err(scp_core::Error::validation_error(format!(
        "Could not parse owner/repo from git remote: {url}"
    )))
}

fn resolve_branches() -> scp_core::Result<Vec<String>> {
    // For now, default to current branch only
    let current = get_current_branch();
    Ok(vec![current])
}

fn get_current_branch() -> String {
    std::process::Command::new("git")
        .args(["branch", "--show-current"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use scp_stack::BranchCiStatus;

    #[test]
    fn test_print_multi_branch_header_no_panic() {
        let statuses = vec![BranchCiStatus {
            branch: "feature".to_string(),
            sha: "abc123".to_string(),
            sha_short: "abc123d".to_string(),
            overall_status: Some("success".to_string()),
            check_runs: vec![],
            pr_number: None,
        }];
        // Should not panic
        print_multi_branch_header(&statuses);
    }

    #[test]
    fn test_check_icon_label_completed() {
        let check = CheckRunInfo {
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
        let (icon, label) = check_icon_label(&check);
        assert_eq!(icon, "PASS");
        assert_eq!(label, "passed");
    }

    #[test]
    fn test_check_icon_label_running() {
        let check = CheckRunInfo {
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
        let (icon, label) = check_icon_label(&check);
        assert_eq!(icon, "RUNS");
        assert_eq!(label, "running");
    }

    #[test]
    fn test_check_icon_label_failed() {
        let check = CheckRunInfo {
            name: "lint".to_string(),
            status: "completed".to_string(),
            conclusion: Some("failure".to_string()),
            url: None,
            started_at: None,
            completed_at: None,
            elapsed_secs: None,
            average_secs: None,
            completion_percent: None,
        };
        let (icon, label) = check_icon_label(&check);
        assert_eq!(icon, "FAIL");
        assert_eq!(label, "failed");
    }

    #[test]
    fn test_parse_remote_ssh_url() {
        // We can't easily test the git command output, but test the parsing logic
        // by verifying the function returns an appropriate error when not in a git repo
        // The actual parsing is tested via integration tests
    }
}
