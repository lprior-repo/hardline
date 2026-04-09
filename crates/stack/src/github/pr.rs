//! PR domain types and operations ported from stax.
//!
//! Contains value types for PR state, CI status, merge methods,
//! and pure functions for stack link body management.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::{Result, StackError};

// ── Merge method ──────────────────────────────────────────────────────

/// How to merge a PR.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MergeMethod {
    #[default]
    Squash,
    Merge,
    Rebase,
}

impl MergeMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Squash => "squash",
            Self::Merge => "merge",
            Self::Rebase => "rebase",
        }
    }
}

impl std::str::FromStr for MergeMethod {
    type Err = StackError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "squash" => Ok(Self::Squash),
            "merge" => Ok(Self::Merge),
            "rebase" => Ok(Self::Rebase),
            _ => Err(StackError::GitHubError(format!(
                "Invalid merge method: {s}. Use: squash, merge, or rebase"
            ))),
        }
    }
}

impl fmt::Display for MergeMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ── CI status ─────────────────────────────────────────────────────────

/// CI check status for a commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CiStatus {
    Pending,
    Success,
    Failure,
    /// No CI checks configured — treat as passing.
    NoCi,
}

impl CiStatus {
    pub fn from_api_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "success" => Self::Success,
            "pending" => Self::Pending,
            "failure" | "error" => Self::Failure,
            "neutral" | "skipped" | "cancelled" => Self::Success,
            "" | "none" | "unknown" => Self::NoCi,
            _ => Self::NoCi,
        }
    }

    pub fn is_success(self) -> bool {
        matches!(self, Self::Success | Self::NoCi)
    }

    pub fn is_pending(self) -> bool {
        matches!(self, Self::Pending)
    }

    pub fn is_failure(self) -> bool {
        matches!(self, Self::Failure)
    }

    pub fn display_text(self) -> &'static str {
        match self {
            Self::Success => "passed",
            Self::Pending => "running",
            Self::Failure => "failed",
            Self::NoCi => "no checks",
        }
    }
}

// ── PR info (GitHub-level) ────────────────────────────────────────────

/// PR info returned from GitHub API lookups.
///
/// Lighter than the domain `domain::stack::PrInfo` — focused on
/// GitHub-specific fields needed for PR operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubPrInfo {
    pub number: u64,
    pub state: String,
    pub is_draft: bool,
    pub base: String,
}

/// PR info including the head branch name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubPrInfoWithHead {
    pub info: GitHubPrInfo,
    pub head: String,
    pub head_label: Option<String>,
}

// ── PR merge status ───────────────────────────────────────────────────

/// Detailed PR merge readiness status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrMergeStatus {
    pub number: u64,
    pub title: String,
    pub state: String,
    pub is_draft: bool,
    pub mergeable: Option<bool>,
    pub mergeable_state: String,
    pub ci_status: CiStatus,
    pub review_decision: Option<String>,
    pub approvals: usize,
    pub changes_requested: bool,
    pub head_sha: String,
}

impl PrMergeStatus {
    pub fn is_ready(&self) -> bool {
        self.ci_status.is_success()
            && !self.is_draft
            && self.mergeable.unwrap_or(false)
            && !self.changes_requested
            && self.state.to_lowercase() == "open"
    }

    pub fn is_waiting(&self) -> bool {
        self.ci_status.is_pending() || self.mergeable.is_none()
    }

    pub fn is_blocked(&self) -> bool {
        self.ci_status.is_failure()
            || self.changes_requested
            || self.is_draft
            || self.mergeable == Some(false)
    }

    pub fn status_text(&self) -> &'static str {
        if self.state.to_lowercase() != "open" {
            return "Closed";
        }
        if self.is_draft {
            return "Draft";
        }
        if self.changes_requested {
            return "Changes requested";
        }
        if self.ci_status.is_failure() {
            return "CI failed";
        }
        if self.mergeable == Some(false) {
            return "Has conflicts";
        }
        if self.is_waiting() {
            return "Waiting";
        }
        "Ready"
    }
}

// ── PR comment types ──────────────────────────────────────────────────

use chrono::{DateTime, Utc};

/// A comment on a PR issue thread (conversation comment).
#[derive(Debug, Clone)]
pub struct IssueComment {
    pub id: u64,
    pub body: String,
    pub user: String,
    pub created_at: DateTime<Utc>,
}

/// A review comment on a PR (inline code comment).
#[derive(Debug, Clone)]
pub struct ReviewComment {
    pub id: u64,
    pub body: String,
    pub user: String,
    pub path: String,
    pub line: Option<u32>,
    pub start_line: Option<u32>,
    pub created_at: DateTime<Utc>,
    pub diff_hunk: Option<String>,
}

/// Combined comment for unified display.
#[derive(Debug, Clone)]
pub enum PrComment {
    Issue(IssueComment),
    Review(ReviewComment),
}

impl PrComment {
    pub fn created_at(&self) -> DateTime<Utc> {
        match self {
            Self::Issue(c) => c.created_at,
            Self::Review(c) => c.created_at,
        }
    }

    pub fn user(&self) -> &str {
        match self {
            Self::Issue(c) => &c.user,
            Self::Review(c) => &c.user,
        }
    }

    pub fn body(&self) -> &str {
        match self {
            Self::Issue(c) => &c.body,
            Self::Review(c) => &c.body,
        }
    }
}

// ── Stack link body management (pure functions) ───────────────────────

const STACK_COMMENT_MARKER: &str = "<!-- stax-stack-comment -->";
const STACK_LINKS_BODY_START_MARKER: &str = "<!-- stax-stack-links:start -->";
const STACK_LINKS_BODY_END_MARKER: &str = "<!-- stax-stack-links:end -->";

/// PR info for stack comment generation.
#[derive(Debug, Clone)]
pub struct StackPrInfo {
    pub branch: String,
    pub pr_number: Option<u64>,
}

/// Generate the stack links markdown.
pub fn generate_stack_links_markdown(
    prs: &[StackPrInfo],
    current_pr_number: u64,
    trunk: &str,
) -> String {
    let mut lines = vec![
        "## Stack Links".to_string(),
        String::new(),
        "This PR is part of a stacked series:".to_string(),
        String::new(),
        format!("* `{trunk}`"),
    ];

    for (i, pr_info) in prs.iter().enumerate() {
        let is_current = pr_info.pr_number == Some(current_pr_number);
        let pointer = if is_current { " \u{1f448}" } else { "" };

        let pr_text = match pr_info.pr_number {
            Some(num) => format!("**PR #{num}**{pointer}"),
            None => format!("`{}`{pointer}", pr_info.branch),
        };

        let indent = "  ".repeat(i + 1);
        lines.push(format!("{indent}* {pr_text}"));
    }

    lines.push(String::new());
    lines.push(
        "This comment was autogenerated by [stax](https://github.com/cesarferreira/stax)"
            .to_string(),
    );

    lines.join("\n")
}

/// Backward-compatible alias.
pub fn generate_stack_comment(prs: &[StackPrInfo], current_pr_number: u64, trunk: &str) -> String {
    generate_stack_links_markdown(prs, current_pr_number, trunk)
}

/// Insert or replace the stack links block in a PR body.
pub fn upsert_stack_links_in_body(existing_body: &str, stack_links: &str) -> String {
    let managed_block = format!(
        "{STACK_LINKS_BODY_START_MARKER}\n{}\n{STACK_LINKS_BODY_END_MARKER}",
        stack_links.trim()
    );

    let body_without_existing = remove_stack_links_from_body(existing_body);
    if body_without_existing.is_empty() {
        return managed_block;
    }

    if body_without_existing.ends_with("\n\n") {
        format!("{body_without_existing}{managed_block}")
    } else if body_without_existing.ends_with('\n') {
        format!("{body_without_existing}\n{managed_block}")
    } else {
        format!("{body_without_existing}\n\n{managed_block}")
    }
}

/// Remove the stack links block from a PR body.
pub fn remove_stack_links_from_body(existing_body: &str) -> String {
    let start_idx = match existing_body.find(STACK_LINKS_BODY_START_MARKER) {
        Some(idx) => idx,
        None => return existing_body.to_string(),
    };
    let end_marker_idx = match existing_body[start_idx..].find(STACK_LINKS_BODY_END_MARKER) {
        Some(idx) => idx,
        None => return existing_body.to_string(),
    };

    let end_idx = start_idx + end_marker_idx + STACK_LINKS_BODY_END_MARKER.len();
    let mut remove_start = start_idx;
    let mut remove_end = end_idx;

    if existing_body[..start_idx].ends_with("\n\n") {
        remove_start -= 2;
    } else if existing_body[..start_idx].ends_with('\n') {
        remove_start -= 1;
    } else if existing_body[end_idx..].starts_with("\n\n") {
        remove_end += 2;
    } else if existing_body[end_idx..].starts_with('\n') {
        remove_end += 1;
    }

    let mut result = String::with_capacity(existing_body.len());
    result.push_str(&existing_body[..remove_start]);
    result.push_str(&existing_body[remove_end..]);
    result
}

/// Get the stack comment marker constant (for tests and external use).
pub fn stack_comment_marker() -> &'static str {
    STACK_COMMENT_MARKER
}

/// Get the body start marker constant.
pub fn body_start_marker() -> &'static str {
    STACK_LINKS_BODY_START_MARKER
}

/// Get the body end marker constant.
pub fn body_end_marker() -> &'static str {
    STACK_LINKS_BODY_END_MARKER
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_method_from_str_squash() {
        let method: MergeMethod = "squash".parse().expect("parse");
        assert_eq!(method, MergeMethod::Squash);
    }

    #[test]
    fn test_merge_method_from_str_merge() {
        let method: MergeMethod = "merge".parse().expect("parse");
        assert_eq!(method, MergeMethod::Merge);
    }

    #[test]
    fn test_merge_method_from_str_rebase() {
        let method: MergeMethod = "rebase".parse().expect("parse");
        assert_eq!(method, MergeMethod::Rebase);
    }

    #[test]
    fn test_merge_method_from_str_case_insensitive() {
        let method: MergeMethod = "SQUASH".parse().expect("parse");
        assert_eq!(method, MergeMethod::Squash);

        let method: MergeMethod = "Merge".parse().expect("parse");
        assert_eq!(method, MergeMethod::Merge);

        let method: MergeMethod = "REBASE".parse().expect("parse");
        assert_eq!(method, MergeMethod::Rebase);
    }

    #[test]
    fn test_merge_method_from_str_invalid() {
        let result: Result<MergeMethod> = "invalid".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_merge_method_as_str() {
        assert_eq!(MergeMethod::Squash.as_str(), "squash");
        assert_eq!(MergeMethod::Merge.as_str(), "merge");
        assert_eq!(MergeMethod::Rebase.as_str(), "rebase");
    }

    #[test]
    fn test_merge_method_default() {
        assert_eq!(MergeMethod::default(), MergeMethod::Squash);
    }

    #[test]
    fn test_merge_method_display() {
        assert_eq!(format!("{}", MergeMethod::Squash), "squash");
        assert_eq!(format!("{}", MergeMethod::Merge), "merge");
        assert_eq!(format!("{}", MergeMethod::Rebase), "rebase");
    }

    #[test]
    fn test_ci_status_from_api_str() {
        assert_eq!(CiStatus::from_api_str("success"), CiStatus::Success);
        assert_eq!(CiStatus::from_api_str("pending"), CiStatus::Pending);
        assert_eq!(CiStatus::from_api_str("failure"), CiStatus::Failure);
        assert_eq!(CiStatus::from_api_str("error"), CiStatus::Failure);
        assert_eq!(CiStatus::from_api_str("neutral"), CiStatus::Success);
        assert_eq!(CiStatus::from_api_str("skipped"), CiStatus::Success);
        assert_eq!(CiStatus::from_api_str("unknown"), CiStatus::NoCi);
        assert_eq!(CiStatus::from_api_str(""), CiStatus::NoCi);
    }

    #[test]
    fn test_ci_status_is_methods() {
        assert!(CiStatus::Success.is_success());
        assert!(!CiStatus::Success.is_pending());
        assert!(!CiStatus::Success.is_failure());

        assert!(!CiStatus::Pending.is_success());
        assert!(CiStatus::Pending.is_pending());

        assert!(CiStatus::Failure.is_failure());

        assert!(CiStatus::NoCi.is_success());
    }

    #[test]
    fn test_ci_status_display_text() {
        assert_eq!(CiStatus::Success.display_text(), "passed");
        assert_eq!(CiStatus::Pending.display_text(), "running");
        assert_eq!(CiStatus::Failure.display_text(), "failed");
        assert_eq!(CiStatus::NoCi.display_text(), "no checks");
    }

    #[test]
    fn test_pr_merge_status_is_ready() {
        let status = PrMergeStatus {
            number: 1,
            title: "Test".to_string(),
            state: "Open".to_string(),
            is_draft: false,
            mergeable: Some(true),
            mergeable_state: "clean".to_string(),
            ci_status: CiStatus::Success,
            review_decision: Some("APPROVED".to_string()),
            approvals: 1,
            changes_requested: false,
            head_sha: "abc123".to_string(),
        };
        assert!(status.is_ready());
        assert!(!status.is_waiting());
        assert!(!status.is_blocked());
    }

    #[test]
    fn test_pr_merge_status_is_blocked_ci_failed() {
        let status = PrMergeStatus {
            number: 1,
            title: "Test".to_string(),
            state: "Open".to_string(),
            is_draft: false,
            mergeable: Some(true),
            mergeable_state: "clean".to_string(),
            ci_status: CiStatus::Failure,
            review_decision: None,
            approvals: 0,
            changes_requested: false,
            head_sha: "abc".to_string(),
        };
        assert!(status.is_blocked());
        assert!(!status.is_ready());
    }

    #[test]
    fn test_pr_merge_status_is_blocked_draft() {
        let status = PrMergeStatus {
            number: 1,
            title: "Test".to_string(),
            state: "Open".to_string(),
            is_draft: true,
            mergeable: Some(true),
            mergeable_state: "clean".to_string(),
            ci_status: CiStatus::Success,
            review_decision: None,
            approvals: 0,
            changes_requested: false,
            head_sha: "abc".to_string(),
        };
        assert!(status.is_blocked());
    }

    #[test]
    fn test_pr_merge_status_text() {
        let ready = PrMergeStatus {
            number: 1,
            title: "T".to_string(),
            state: "Open".to_string(),
            is_draft: false,
            mergeable: Some(true),
            mergeable_state: "clean".to_string(),
            ci_status: CiStatus::Success,
            review_decision: None,
            approvals: 0,
            changes_requested: false,
            head_sha: "abc".to_string(),
        };
        assert_eq!(ready.status_text(), "Ready");

        let draft = PrMergeStatus {
            is_draft: true,
            ..ready.clone()
        };
        assert_eq!(draft.status_text(), "Draft");

        let closed = PrMergeStatus {
            state: "Closed".to_string(),
            is_draft: false,
            ..ready.clone()
        };
        assert_eq!(closed.status_text(), "Closed");
    }

    #[test]
    fn test_generate_stack_comment_single_pr() {
        let prs = vec![StackPrInfo {
            branch: "feature".to_string(),
            pr_number: Some(1),
        }];
        let comment = generate_stack_comment(&prs, 1, "main");
        assert!(comment.contains("## Stack Links"));
        assert!(comment.contains("`main`"));
        assert!(comment.contains("PR #1"));
        assert!(comment.contains("\u{1f448}"));
    }

    #[test]
    fn test_generate_stack_comment_multiple_prs() {
        let prs = vec![
            StackPrInfo {
                branch: "a".to_string(),
                pr_number: Some(1),
            },
            StackPrInfo {
                branch: "b".to_string(),
                pr_number: Some(2),
            },
        ];
        let comment = generate_stack_comment(&prs, 2, "main");
        assert!(comment.contains("PR #1"));
        assert!(comment.contains("PR #2"));
        assert!(comment.contains("#2** \u{1f448}"));
        assert!(!comment.contains("#1** \u{1f448}"));
    }

    #[test]
    fn test_upsert_stack_links_in_empty_body() {
        let body = upsert_stack_links_in_body("", "## Stack Links\n\n- item");
        assert!(body.contains(STACK_LINKS_BODY_START_MARKER));
        assert!(body.contains("## Stack Links"));
        assert!(body.contains(STACK_LINKS_BODY_END_MARKER));
    }

    #[test]
    fn test_upsert_stack_links_appends_to_existing_body() {
        let body = upsert_stack_links_in_body("## Summary\n\nhello", "## Stack Links\n\n- item");
        assert!(body.starts_with("## Summary\n\nhello"));
        assert!(body.ends_with(STACK_LINKS_BODY_END_MARKER));
    }

    #[test]
    fn test_upsert_stack_links_replaces_existing_block() {
        let existing = format!(
            "## Summary\n\nhello\n\n{}\nold\n{}\n",
            STACK_LINKS_BODY_START_MARKER, STACK_LINKS_BODY_END_MARKER
        );
        let body = upsert_stack_links_in_body(&existing, "## Stack Links\n\nnew");
        assert!(!body.contains("\nold\n"));
        assert!(body.contains("new"));
        assert_eq!(body.matches(STACK_LINKS_BODY_START_MARKER).count(), 1);
    }

    #[test]
    fn test_remove_stack_links_from_body_preserves_surrounding() {
        let existing = format!(
            "## Summary\n\nhello\n\n{}\nmanaged\n{}\n\n## Testing\n\nok",
            STACK_LINKS_BODY_START_MARKER, STACK_LINKS_BODY_END_MARKER
        );
        let body = remove_stack_links_from_body(&existing);
        assert_eq!(body, "## Summary\n\nhello\n\n## Testing\n\nok");
    }

    #[test]
    fn test_remove_stack_links_from_body_no_markers() {
        let body = "Hello world".to_string();
        assert_eq!(remove_stack_links_from_body(&body), "Hello world");
    }
}
