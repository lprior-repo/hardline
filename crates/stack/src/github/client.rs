//! Async GitHub API client ported from stax.
//!
//! Wraps `octocrab` with structured error handling via `StackError`.
//! All public methods are async. API call tracking is included.

use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, Utc};
use octocrab::params::pulls::Sort;
use octocrab::params::repos::Reference;
use octocrab::params::State;
use octocrab::Octocrab;
use serde::Deserialize;

use super::pr::{
    CiStatus, GitHubPrInfo, GitHubPrInfoWithHead, IssueComment, MergeMethod, PrMergeStatus,
    ReviewComment,
};
use crate::error::{Result, StackError};

const GITHUB_API_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const GITHUB_API_READ_TIMEOUT: Duration = Duration::from_secs(30);
const GITHUB_API_WRITE_TIMEOUT: Duration = Duration::from_secs(30);
const GITHUB_API_RETRY_COUNT: usize = 1;

// ── API call tracking ─────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ApiCallStats {
    pub total_requests: usize,
    pub by_operation: Vec<(String, usize)>,
}

#[derive(Default)]
struct ApiCallTracker {
    total_requests: AtomicUsize,
    by_operation: Mutex<BTreeMap<String, usize>>,
}

impl ApiCallTracker {
    fn record(&self, operation: &'static str, count: usize) {
        if count == 0 {
            return;
        }
        self.total_requests.fetch_add(count, Ordering::Relaxed);
        let mut by_operation = self.by_operation.lock().unwrap_or_else(|e| e.into_inner());
        *by_operation.entry(operation.to_string()).or_insert(0) += count;
    }

    fn snapshot(&self) -> ApiCallStats {
        let by_operation = self
            .by_operation
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .map(|(op, count)| (op.clone(), *count))
            .collect();

        ApiCallStats {
            total_requests: self.total_requests.load(Ordering::Relaxed),
            by_operation,
        }
    }
}

// ── Internal API response types ───────────────────────────────────────

#[derive(Debug, Deserialize)]
struct CheckRunsResponse {
    total_count: usize,
    check_runs: Vec<CheckRun>,
}

#[derive(Debug, Deserialize)]
struct CheckRun {
    id: u64,
    name: String,
    status: String,
    conclusion: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ReviewUser {
    login: String,
}

#[derive(Debug, Deserialize)]
struct Review {
    state: String,
    submitted_at: Option<DateTime<Utc>>,
    user: Option<ReviewUser>,
}

#[derive(Debug, Deserialize)]
struct SearchIssuesResponse {
    items: Vec<SearchIssue>,
}

#[derive(Debug, Deserialize)]
struct SearchIssue {
    number: u64,
    title: String,
    html_url: String,
    created_at: DateTime<Utc>,
    closed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct RepoListUser {
    login: String,
}

#[derive(Debug, Deserialize)]
struct RepoListPullRef {
    #[serde(rename = "ref")]
    ref_field: String,
}

#[derive(Debug, Deserialize)]
struct RepoListPullRequest {
    number: u64,
    title: String,
    html_url: String,
    user: RepoListUser,
    head: RepoListPullRef,
    base: RepoListPullRef,
    state: String,
    draft: Option<bool>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct RepoListLabel {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RepoListIssue {
    number: u64,
    title: String,
    html_url: String,
    user: RepoListUser,
    labels: Vec<RepoListLabel>,
    updated_at: DateTime<Utc>,
    pull_request: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct ApiIssueComment {
    id: u64,
    body: Option<String>,
    user: ReviewUser,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct ApiReviewComment {
    id: u64,
    body: Option<String>,
    user: ReviewUser,
    path: String,
    line: Option<u32>,
    start_line: Option<u32>,
    created_at: DateTime<Utc>,
    diff_hunk: Option<String>,
}

/// Open PR info for tracking.
#[derive(Debug, Clone)]
pub struct OpenPrInfo {
    pub number: u64,
    pub head_branch: String,
    pub base_branch: String,
    pub state: String,
    pub is_draft: bool,
}

/// PR activity for standup/reporting.
#[derive(Debug, Clone)]
pub struct PrActivity {
    pub number: u64,
    pub title: String,
    pub timestamp: DateTime<Utc>,
    pub url: String,
}

/// Review activity.
#[derive(Debug, Clone)]
pub struct ReviewActivity {
    pub pr_number: u64,
    pub pr_title: String,
    pub reviewer: String,
    pub state: String,
    pub timestamp: DateTime<Utc>,
    pub is_received: bool,
}

/// List item for open PRs.
#[derive(Debug, Clone)]
pub struct RepoPrListItem {
    pub number: u64,
    pub title: String,
    pub url: String,
    pub author: String,
    pub head_branch: String,
    pub base_branch: String,
    pub state: String,
    pub is_draft: bool,
    pub created_at: DateTime<Utc>,
}

/// List item for open issues.
#[derive(Debug, Clone)]
pub struct RepoIssueListItem {
    pub number: u64,
    pub title: String,
    pub url: String,
    pub author: String,
    pub labels: Vec<String>,
    pub updated_at: DateTime<Utc>,
}

// ── GraphQL types ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct GraphQLResponse<T> {
    data: Option<T>,
    errors: Option<Vec<GraphQLError>>,
}

#[derive(Debug, Deserialize)]
struct GraphQLError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct PrReviewData {
    repository: Option<RepositoryData>,
}

#[derive(Debug, Deserialize)]
struct RepositoryData {
    #[serde(rename = "pullRequest")]
    pull_request: Option<PullRequestData>,
}

#[derive(Debug, Deserialize)]
struct PullRequestData {
    #[serde(rename = "reviewDecision")]
    review_decision: Option<String>,
    reviews: ReviewConnection,
}

#[derive(Debug, Deserialize)]
struct ReviewConnection {
    nodes: Vec<ReviewNode>,
}

#[derive(Debug, Deserialize)]
struct ReviewNode {
    state: String,
}

// ── GitHub client ─────────────────────────────────────────────────────

/// Async GitHub API client backed by octocrab.
pub struct GitHubClient {
    pub(crate) octocrab: Octocrab,
    pub(crate) owner: String,
    pub(crate) repo: String,
    api_call_tracker: Arc<ApiCallTracker>,
}

impl GitHubClient {
    /// Create a new client with a personal token.
    pub fn new(
        owner: &str,
        repo: &str,
        token: String,
        api_base_url: Option<String>,
    ) -> Result<Self> {
        use octocrab::service::middleware::retry::RetryConfig;

        let mut builder = Octocrab::builder()
            .personal_token(token)
            .add_retry_config(RetryConfig::Simple(GITHUB_API_RETRY_COUNT))
            .set_connect_timeout(Some(GITHUB_API_CONNECT_TIMEOUT))
            .set_read_timeout(Some(GITHUB_API_READ_TIMEOUT))
            .set_write_timeout(Some(GITHUB_API_WRITE_TIMEOUT));

        if let Some(api_base) = api_base_url {
            builder = builder.base_uri(api_base).map_err(|e| {
                StackError::GitHubError(format!("Failed to set GitHub API base URL: {e}"))
            })?;
        }

        let octocrab = builder
            .build()
            .map_err(|e| StackError::GitHubError(format!("Failed to create GitHub client: {e}")))?;

        Ok(Self {
            octocrab,
            owner: owner.to_string(),
            repo: repo.to_string(),
            api_call_tracker: Arc::new(ApiCallTracker::default()),
        })
    }

    /// Create a client with a pre-configured octocrab instance (for testing).
    pub fn with_octocrab(octocrab: Octocrab, owner: &str, repo: &str) -> Self {
        Self {
            octocrab,
            owner: owner.to_string(),
            repo: repo.to_string(),
            api_call_tracker: Arc::new(ApiCallTracker::default()),
        }
    }

    pub fn api_call_stats(&self) -> ApiCallStats {
        self.api_call_tracker.snapshot()
    }

    fn record_api_call(&self, operation: &'static str) {
        self.api_call_tracker.record(operation, 1);
    }

    // ── CI status ─────────────────────────────────────────────────────

    /// Get combined CI status from both commit statuses AND check runs.
    pub async fn combined_status_state(&self, commit_sha: &str) -> Result<Option<String>> {
        let commit_status = self
            .octocrab
            .repos(&self.owner, &self.repo)
            .combined_status_for_ref(&Reference::Branch(commit_sha.to_string()))
            .await
            .ok();

        let check_runs_status = self.get_check_runs_status(commit_sha).await.ok().flatten();

        match (check_runs_status, commit_status) {
            (Some(cr_status), _) => Ok(Some(cr_status)),
            (None, Some(status)) => Ok(Some(format!("{:?}", status.state).to_lowercase())),
            (None, None) => Ok(None),
        }
    }

    /// Get status from GitHub Actions check runs.
    pub async fn get_check_runs_status(&self, commit_sha: &str) -> Result<Option<String>> {
        let url = format!(
            "/repos/{}/{}/commits/{}/check-runs",
            self.owner, self.repo, commit_sha
        );

        let response: CheckRunsResponse = self
            .octocrab
            .get(&url, None::<&()>)
            .await
            .map_err(|e| StackError::GitHubError(format!("Check runs request failed: {e}")))?;

        if response.total_count == 0 {
            return Ok(None);
        }

        // Deduplicate by name, keeping the latest (highest id).
        let mut latest_by_name: HashMap<&str, &CheckRun> = HashMap::new();
        for run in &response.check_runs {
            let entry = latest_by_name.entry(&run.name).or_insert(run);
            if run.id > entry.id {
                *entry = run;
            }
        }

        let mut has_pending = false;
        let mut has_failure = false;
        let mut all_success = true;

        for run in latest_by_name.values() {
            match run.status.as_str() {
                "completed" => match run.conclusion.as_deref() {
                    Some("success") | Some("skipped") | Some("neutral") => {}
                    Some("failure")
                    | Some("timed_out")
                    | Some("cancelled")
                    | Some("action_required") => {
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
            Ok(Some("failure".to_string()))
        } else if has_pending {
            Ok(Some("pending".to_string()))
        } else if all_success {
            Ok(Some("success".to_string()))
        } else {
            Ok(Some("pending".to_string()))
        }
    }

    // ── User info ─────────────────────────────────────────────────────

    /// Get the authenticated user's login name.
    pub async fn get_current_user(&self) -> Result<String> {
        let user = self
            .octocrab
            .current()
            .user()
            .await
            .map_err(|e| StackError::GitHubError(format!("Get current user failed: {e}")))?;
        Ok(user.login)
    }

    // ── PR operations ─────────────────────────────────────────────────

    /// Find an open PR by head owner and branch name.
    pub async fn find_open_pr_by_head(
        &self,
        head_owner: &str,
        branch: &str,
    ) -> Result<Option<GitHubPrInfoWithHead>> {
        self.record_api_call("pulls.list.head");
        let prs = self
            .octocrab
            .pulls(&self.owner, &self.repo)
            .list()
            .state(State::Open)
            .head(format!("{head_owner}:{branch}"))
            .per_page(100u8)
            .sort(Sort::Created)
            .send()
            .await
            .map_err(|e| StackError::GitHubError(format!("List PRs by head failed: {e}")))?;

        for pr in &prs.items {
            if pr.head.ref_field != branch {
                continue;
            }
            let owner_matches = pr
                .head
                .label
                .as_ref()
                .and_then(|label| label.split_once(':').map(|(owner, _)| owner == head_owner))
                .unwrap_or(true);
            if !owner_matches {
                continue;
            }

            return Ok(Some(GitHubPrInfoWithHead {
                head_label: pr.head.label.clone(),
                info: GitHubPrInfo {
                    number: pr.number,
                    state: pr
                        .state
                        .as_ref()
                        .map(|s| format!("{s:?}"))
                        .unwrap_or_default(),
                    is_draft: pr.draft.unwrap_or(false),
                    base: pr.base.ref_field.clone(),
                },
                head: pr.head.ref_field.clone(),
            }));
        }

        Ok(None)
    }

    /// Find an open PR for a branch.
    pub async fn find_pr(&self, branch: &str) -> Result<Option<GitHubPrInfo>> {
        if let Some(pr) = self.find_open_pr_by_head(&self.owner, branch).await? {
            return Ok(Some(pr.info));
        }
        let prs_by_head = self.list_open_prs_by_head().await?;
        Ok(prs_by_head.get(branch).cloned().map(|pr| pr.info))
    }

    /// List all open PRs indexed by head branch name.
    pub async fn list_open_prs_by_head(&self) -> Result<HashMap<String, GitHubPrInfoWithHead>> {
        let mut page = 1u32;
        const PER_PAGE: u8 = 100;
        let mut prs_by_head = HashMap::new();

        loop {
            self.record_api_call("pulls.list.open.page");
            let prs = self
                .octocrab
                .pulls(&self.owner, &self.repo)
                .list()
                .state(State::Open)
                .per_page(PER_PAGE)
                .page(page)
                .sort(Sort::Created)
                .send()
                .await
                .map_err(|e| StackError::GitHubError(format!("List PRs failed: {e}")))?;

            for pr in &prs.items {
                let head = pr.head.ref_field.clone();
                if prs_by_head.contains_key(&head) {
                    continue;
                }
                prs_by_head.insert(
                    head,
                    GitHubPrInfoWithHead {
                        head_label: pr.head.label.clone(),
                        info: GitHubPrInfo {
                            number: pr.number,
                            state: pr
                                .state
                                .as_ref()
                                .map(|s| format!("{s:?}"))
                                .unwrap_or_default(),
                            is_draft: pr.draft.unwrap_or(false),
                            base: pr.base.ref_field.clone(),
                        },
                        head: pr.head.ref_field.clone(),
                    },
                );
            }

            if (prs.items.len() as u8) < PER_PAGE {
                break;
            }
            page += 1;
        }

        Ok(prs_by_head)
    }

    /// Create a new PR.
    pub async fn create_pr(
        &self,
        branch: &str,
        base: &str,
        title: &str,
        body: &str,
        draft: bool,
    ) -> Result<GitHubPrInfo> {
        self.record_api_call("pulls.create");
        let pr = self
            .octocrab
            .pulls(&self.owner, &self.repo)
            .create(title, branch, base)
            .body(body)
            .draft(Some(draft))
            .send()
            .await
            .map_err(|e| StackError::GitHubError(format!("Create PR failed: {e}")))?;

        Ok(GitHubPrInfo {
            number: pr.number,
            state: pr
                .state
                .as_ref()
                .map(|s| format!("{s:?}"))
                .unwrap_or_default(),
            is_draft: pr.draft.unwrap_or(false),
            base: pr.base.ref_field.clone(),
        })
    }

    /// Get a PR by number.
    pub async fn get_pr(&self, pr_number: u64) -> Result<GitHubPrInfo> {
        self.record_api_call("pulls.get");
        let pr = self
            .octocrab
            .pulls(&self.owner, &self.repo)
            .get(pr_number)
            .await
            .map_err(|e| StackError::GitHubError(format!("Get PR failed: {e}")))?;

        Ok(GitHubPrInfo {
            number: pr.number,
            state: pr
                .state
                .as_ref()
                .map(|s| format!("{s:?}"))
                .unwrap_or_default(),
            is_draft: pr.draft.unwrap_or(false),
            base: pr.base.ref_field.clone(),
        })
    }

    /// Get a PR by number with head branch info.
    pub async fn get_pr_with_head(&self, pr_number: u64) -> Result<GitHubPrInfoWithHead> {
        self.record_api_call("pulls.get");
        let pr = self
            .octocrab
            .pulls(&self.owner, &self.repo)
            .get(pr_number)
            .await
            .map_err(|e| StackError::GitHubError(format!("Get PR with head failed: {e}")))?;

        Ok(GitHubPrInfoWithHead {
            head: pr.head.ref_field.clone(),
            head_label: pr.head.label.clone(),
            info: GitHubPrInfo {
                number: pr.number,
                state: pr
                    .state
                    .as_ref()
                    .map(|s| format!("{s:?}"))
                    .unwrap_or_default(),
                is_draft: pr.draft.unwrap_or(false),
                base: pr.base.ref_field.clone(),
            },
        })
    }

    /// Update the PR base branch.
    pub async fn update_pr_base(&self, pr_number: u64, new_base: &str) -> Result<()> {
        self.record_api_call("pulls.update.base");
        self.octocrab
            .pulls(&self.owner, &self.repo)
            .update(pr_number)
            .base(new_base)
            .send()
            .await
            .map_err(|e| StackError::GitHubError(format!("Update PR base failed: {e}")))?;
        Ok(())
    }

    /// Merge the base branch into the PR head branch (GitHub "Update branch").
    pub async fn update_pr_branch(&self, pr_number: u64) -> Result<()> {
        self.record_api_call("pulls.update-branch");
        let route = format!(
            "/repos/{}/{}/pulls/{}/update-branch",
            self.owner, self.repo, pr_number
        );
        let result = self
            .octocrab
            .put::<serde_json::Value, _, serde_json::Value>(&route, Some(&serde_json::json!({})))
            .await;

        match result {
            Ok(_) => Ok(()),
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("Update is not required")
                    || msg.contains("There are no new commits")
                {
                    Ok(())
                } else {
                    Err(StackError::GitHubError(format!(
                        "Update PR branch failed: {e}"
                    )))
                }
            }
        }
    }

    /// Update PR body text.
    pub async fn update_pr_body(&self, pr_number: u64, body: &str) -> Result<()> {
        self.record_api_call("pulls.update.body");
        self.octocrab
            .pulls(&self.owner, &self.repo)
            .update(pr_number)
            .body(body)
            .send()
            .await
            .map_err(|e| StackError::GitHubError(format!("Update PR body failed: {e}")))?;
        Ok(())
    }

    /// Get the current PR body text.
    pub async fn get_pr_body(&self, pr_number: u64) -> Result<String> {
        self.record_api_call("pulls.get.body");
        let pr = self
            .octocrab
            .pulls(&self.owner, &self.repo)
            .get(pr_number)
            .await
            .map_err(|e| StackError::GitHubError(format!("Get PR body failed: {e}")))?;

        Ok(pr.body.unwrap_or_default())
    }

    /// Add or update the stack comment on a PR.
    pub async fn update_stack_comment(&self, pr_number: u64, stack_comment: &str) -> Result<()> {
        let marker = super::pr::stack_comment_marker();
        if let Some(comment_id) = self.find_stack_comment_id(pr_number).await? {
            let full_comment = format!("{marker}\n{stack_comment}");
            self.record_api_call("issues.comments.update");
            let route = format!(
                "/repos/{}/{}/issues/comments/{}",
                self.owner, self.repo, comment_id
            );
            self.octocrab
                .patch::<serde_json::Value, _, _>(
                    &route,
                    Some(&serde_json::json!({ "body": full_comment })),
                )
                .await
                .map_err(|e| StackError::GitHubError(format!("Update comment failed: {e}")))?;
            return Ok(());
        }

        self.create_stack_comment(pr_number, stack_comment).await
    }

    /// Create a stax stack comment on a PR.
    pub async fn create_stack_comment(&self, pr_number: u64, stack_comment: &str) -> Result<()> {
        self.record_api_call("issues.comments.create");
        let marker = super::pr::stack_comment_marker();
        let full_comment = format!("{marker}\n{stack_comment}");
        self.octocrab
            .issues(&self.owner, &self.repo)
            .create_comment(pr_number, &full_comment)
            .await
            .map_err(|e| StackError::GitHubError(format!("Create comment failed: {e}")))?;

        Ok(())
    }

    /// Delete the stax-managed stack comment on a PR, if present.
    pub async fn delete_stack_comment(&self, pr_number: u64) -> Result<()> {
        let Some(comment_id) = self.find_stack_comment_id(pr_number).await? else {
            return Ok(());
        };

        self.record_api_call("issues.comments.delete");
        self.octocrab
            .issues(&self.owner, &self.repo)
            .delete_comment(comment_id)
            .await
            .map_err(|e| StackError::GitHubError(format!("Delete comment failed: {e}")))?;

        Ok(())
    }

    async fn find_stack_comment_id(
        &self,
        pr_number: u64,
    ) -> Result<Option<octocrab::models::CommentId>> {
        self.record_api_call("issues.comments.list");
        let url = format!(
            "/repos/{}/{}/issues/{}/comments",
            self.owner, self.repo, pr_number
        );
        let comments: Vec<ApiIssueComment> = self
            .octocrab
            .get(&url, None::<&()>)
            .await
            .map_err(|e| StackError::GitHubError(format!("List comments failed: {e}")))?;

        let marker = super::pr::stack_comment_marker();
        Ok(comments.into_iter().find_map(|comment| {
            comment
                .body
                .as_ref()
                .filter(|body| body.contains(marker))
                .map(|_| octocrab::models::CommentId::from(comment.id))
        }))
    }

    /// Request reviewers on a PR.
    pub async fn request_reviewers(&self, pr_number: u64, reviewers: &[String]) -> Result<()> {
        if reviewers.is_empty() {
            return Ok(());
        }
        self.record_api_call("pulls.request_reviewers");
        self.octocrab
            .pulls(&self.owner, &self.repo)
            .request_reviews(pr_number, reviewers.to_vec(), Vec::<String>::new())
            .await
            .map_err(|e| StackError::GitHubError(format!("Request reviewers failed: {e}")))?;

        Ok(())
    }

    /// Add labels to a PR.
    pub async fn add_labels(&self, pr_number: u64, labels: &[String]) -> Result<()> {
        if labels.is_empty() {
            return Ok(());
        }
        self.record_api_call("issues.add_labels");
        self.octocrab
            .issues(&self.owner, &self.repo)
            .add_labels(pr_number, labels)
            .await
            .map_err(|e| StackError::GitHubError(format!("Add labels failed: {e}")))?;

        Ok(())
    }

    /// Merge a PR with the specified method.
    pub async fn merge_pr(
        &self,
        pr_number: u64,
        method: MergeMethod,
        commit_title: Option<String>,
        commit_message: Option<String>,
    ) -> Result<()> {
        let merge_method = match method {
            MergeMethod::Squash => octocrab::params::pulls::MergeMethod::Squash,
            MergeMethod::Merge => octocrab::params::pulls::MergeMethod::Merge,
            MergeMethod::Rebase => octocrab::params::pulls::MergeMethod::Rebase,
        };

        let pulls = self.octocrab.pulls(&self.owner, &self.repo);
        let mut merge_builder = pulls.merge(pr_number).method(merge_method);

        if let Some(ref title) = commit_title {
            merge_builder = merge_builder.title(title);
        }
        if let Some(ref message) = commit_message {
            merge_builder = merge_builder.message(message);
        }

        merge_builder
            .send()
            .await
            .map_err(|e| StackError::GitHubError(format!("Merge PR failed: {e}")))?;

        Ok(())
    }

    /// Get detailed merge status for a PR.
    pub async fn get_pr_merge_status(&self, pr_number: u64) -> Result<PrMergeStatus> {
        let pr = self
            .octocrab
            .pulls(&self.owner, &self.repo)
            .get(pr_number)
            .await
            .map_err(|e| StackError::GitHubError(format!("Get PR for merge status failed: {e}")))?;

        let head_sha = pr.head.sha.clone();

        let ci_status = self
            .combined_status_state(&head_sha)
            .await
            .ok()
            .flatten()
            .as_deref()
            .map(CiStatus::from_api_str)
            .unwrap_or(CiStatus::NoCi);

        let (review_decision, approvals, changes_requested) = self
            .get_pr_reviews(pr_number)
            .await
            .unwrap_or((None, 0, false));

        Ok(PrMergeStatus {
            number: pr.number,
            title: pr.title.clone().unwrap_or_default(),
            state: pr
                .state
                .as_ref()
                .map(|s| format!("{s:?}"))
                .unwrap_or_default(),
            is_draft: pr.draft.unwrap_or(false),
            mergeable: pr.mergeable,
            mergeable_state: pr
                .mergeable_state
                .map(|s| format!("{s:?}").to_lowercase())
                .unwrap_or_default(),
            ci_status,
            review_decision,
            approvals,
            changes_requested,
            head_sha,
        })
    }

    /// Get PR review info via GraphQL.
    async fn get_pr_reviews(&self, pr_number: u64) -> Result<(Option<String>, usize, bool)> {
        let query = format!(
            r#"
            query {{
                repository(owner: "{}", name: "{}") {{
                    pullRequest(number: {}) {{
                        reviewDecision
                        reviews(last: 100) {{
                            nodes {{
                                state
                            }}
                        }}
                    }}
                }}
            }}
            "#,
            self.owner, self.repo, pr_number
        );

        let response: GraphQLResponse<PrReviewData> = self
            .octocrab
            .graphql(&serde_json::json!({ "query": query }))
            .await
            .map_err(|e| StackError::GitHubError(format!("GraphQL reviews failed: {e}")))?;

        if let Some(errors) = response.errors {
            if !errors.is_empty() {
                return Err(StackError::GitHubError(format!(
                    "GraphQL error: {}",
                    errors[0].message
                )));
            }
        }

        Ok(response
            .data
            .and_then(|d| d.repository)
            .and_then(|r| r.pull_request)
            .map(|pr| {
                let approvals = pr
                    .reviews
                    .nodes
                    .iter()
                    .filter(|r| r.state == "APPROVED")
                    .count();
                let changes_requested = pr
                    .reviews
                    .nodes
                    .iter()
                    .any(|r| r.state == "CHANGES_REQUESTED");
                (pr.review_decision, approvals, changes_requested)
            })
            .unwrap_or((None, 0, false)))
    }

    /// Check if a PR is already merged.
    pub async fn is_pr_merged(&self, pr_number: u64) -> Result<bool> {
        let pr = self
            .octocrab
            .pulls(&self.owner, &self.repo)
            .get(pr_number)
            .await
            .map_err(|e| StackError::GitHubError(format!("Check PR merged failed: {e}")))?;

        Ok(pr.merged_at.is_some())
    }

    // ── Comment operations ────────────────────────────────────────────

    /// List issue comments on a PR.
    pub async fn list_issue_comments(&self, pr_number: u64) -> Result<Vec<IssueComment>> {
        let url = format!(
            "/repos/{}/{}/issues/{}/comments",
            self.owner, self.repo, pr_number
        );
        let comments: Vec<ApiIssueComment> = self
            .octocrab
            .get(&url, None::<&()>)
            .await
            .map_err(|e| StackError::GitHubError(format!("List issue comments failed: {e}")))?;

        Ok(comments
            .into_iter()
            .map(|c| IssueComment {
                id: c.id,
                body: c.body.unwrap_or_default(),
                user: c.user.login,
                created_at: c.created_at,
            })
            .collect())
    }

    /// List review comments on a PR.
    pub async fn list_review_comments(&self, pr_number: u64) -> Result<Vec<ReviewComment>> {
        let url = format!(
            "/repos/{}/{}/pulls/{}/comments",
            self.owner, self.repo, pr_number
        );
        let comments: Vec<ApiReviewComment> =
            self.octocrab.get(&url, None::<&()>).await.map_err(|e| {
                StackError::GitHubError(format!("List review comments failed: {e}"))
            })?;

        Ok(comments
            .into_iter()
            .map(|c| ReviewComment {
                id: c.id,
                body: c.body.unwrap_or_default(),
                user: c.user.login,
                path: c.path,
                line: c.line,
                start_line: c.start_line,
                created_at: c.created_at,
                diff_hunk: c.diff_hunk,
            })
            .collect())
    }

    // ── List operations ───────────────────────────────────────────────

    /// List open pull requests.
    pub async fn list_open_pull_requests(&self, limit: u8) -> Result<Vec<RepoPrListItem>> {
        self.record_api_call("pulls.list");
        let per_page = limit.clamp(1, 100);
        let url = format!(
            "/repos/{}/{}/pulls?state=open&sort=created&direction=desc&per_page={}",
            self.owner, self.repo, per_page
        );

        let response: Vec<RepoListPullRequest> = self
            .octocrab
            .get(&url, None::<&()>)
            .await
            .map_err(|e| StackError::GitHubError(format!("List pull requests failed: {e}")))?;

        Ok(response
            .into_iter()
            .take(per_page as usize)
            .map(|pr| RepoPrListItem {
                number: pr.number,
                title: pr.title,
                url: pr.html_url,
                author: pr.user.login,
                head_branch: pr.head.ref_field,
                base_branch: pr.base.ref_field,
                state: pr.state,
                is_draft: pr.draft.unwrap_or(false),
                created_at: pr.created_at,
            })
            .collect())
    }

    /// List open issues (filters out PRs).
    pub async fn list_open_issues(&self, limit: u8) -> Result<Vec<RepoIssueListItem>> {
        self.record_api_call("issues.list");
        let per_page = limit.clamp(1, 100);
        let fetch_per_page = (usize::from(per_page) * 2).min(100) as u8;
        let url = format!(
            "/repos/{}/{}/issues?state=open&sort=updated&direction=desc&per_page={}",
            self.owner, self.repo, fetch_per_page
        );

        let response: Vec<RepoListIssue> = self
            .octocrab
            .get(&url, None::<&()>)
            .await
            .map_err(|e| StackError::GitHubError(format!("List issues failed: {e}")))?;

        Ok(response
            .into_iter()
            .filter(|issue| issue.pull_request.is_none())
            .take(usize::from(per_page))
            .map(|issue| RepoIssueListItem {
                number: issue.number,
                title: issue.title,
                url: issue.html_url,
                author: issue.user.login,
                labels: issue.labels.into_iter().filter_map(|l| l.name).collect(),
                updated_at: issue.updated_at,
            })
            .collect())
    }
}
