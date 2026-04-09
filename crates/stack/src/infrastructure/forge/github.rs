use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    aggregate_ci_overall, build_http_client, delete_empty, encode_query_value, forge_token,
    stack_comment_body, AuthStyle, CheckRunInfo,
    ForgeType, MergeMethod, RemoteInfo, STACK_COMMENT_MARKER,
};
use crate::domain::stack::PrInfo;
use crate::domain::state::PrState;
use crate::error::{Result, StackError};

const GITHUB_API_BASE_URL: &str = "https://api.github.com";
const RETRY_MAX_ATTEMPTS: usize = 3;
const RETRY_BASE_DELAY_MS: u64 = 500;

#[derive(Clone)]
pub struct GitHubClient {
    client: reqwest::Client,
    api_base_url: String,
    owner: String,
    repo: String,
    rate_limiter: Arc<RateLimiter>,
}

struct RateLimiter {
    remaining: AtomicU64,
    reset_at: Mutex<Instant>,
}

impl RateLimiter {
    fn new() -> Self {
        Self {
            remaining: AtomicU64::new(5000),
            reset_at: Mutex::new(Instant::now()),
        }
    }

    fn update_from_headers(&self, remaining: Option<u64>, reset: Option<u64>) {
        if let Some(r) = remaining {
            self.remaining.store(r, Ordering::Relaxed);
        }
        if let Some(reset_epoch) = reset {
            if let Ok(mut reset_at) = self.reset_at.lock() {
                let now = Utc::now().timestamp() as u64;
                if reset_epoch > now {
                    *reset_at = Instant::now()
                        + Duration::from_secs(reset_epoch - now);
                }
            }
        }
    }

    async fn wait_if_needed(&self) {
        loop {
            let remaining = self.remaining.load(Ordering::Relaxed);
            if remaining > 10 {
                return;
            }
            let reset_at = {
                let guard = self.reset_at.lock().unwrap_or_else(|e| e.into_inner());
                *guard
            };
            let now = Instant::now();
            if now >= reset_at {
                return;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct GitHubPullRequest {
    number: u64,
    state: Option<String>,
    title: Option<String>,
    body: Option<String>,
    draft: Option<bool>,
    merged: Option<bool>,
    mergeable: Option<bool>,
    mergeable_state: Option<String>,
    head: GitHubBranchRef,
    base: GitHubBranchRef,
    user: Option<GitHubUser>,
    html_url: Option<String>,
    merged_at: Option<DateTime<Utc>>,
    created_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct GitHubBranchRef {
    #[serde(rename = "ref")]
    ref_name: String,
    sha: Option<String>,
    label: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubUser {
    login: String,
}

#[derive(Debug, Deserialize)]
pub struct GitHubComment {
    id: u64,
    body: Option<String>,
    user: GitHubUser,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct GitHubReviewComment {
    id: u64,
    body: Option<String>,
    user: GitHubUser,
    path: String,
    line: Option<u32>,
    start_line: Option<u32>,
    created_at: DateTime<Utc>,
    diff_hunk: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubCommitStatus {
    state: String,
    statuses: Vec<GitHubStatus>,
}

#[derive(Debug, Deserialize)]
struct GitHubStatus {
    context: Option<String>,
    state: String,
    target_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubCheckRuns {
    total_count: usize,
    check_runs: Vec<GitHubCheckRun>,
}

#[derive(Debug, Deserialize)]
struct GitHubCheckRun {
    id: u64,
    name: String,
    status: String,
    conclusion: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubIssue {
    number: u64,
    title: String,
    html_url: Option<String>,
    user: Option<GitHubUser>,
    labels: Vec<GitHubLabel>,
    updated_at: DateTime<Utc>,
    pull_request: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct GitHubLabel {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubCurrentUser {
    login: String,
}

#[derive(Debug, Deserialize)]
struct GitHubPullRequestUpdate {
    title: Option<String>,
    body: Option<String>,
    state: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubMergeRequest {
    merge_method: Option<String>,
    commit_title: Option<String>,
    commit_message: Option<String>,
}

#[derive(Serialize)]
struct CreatePullRequest<'a> {
    title: &'a str,
    head: &'a str,
    base: &'a str,
    body: &'a str,
    draft: bool,
}

#[derive(Serialize)]
struct UpdatePullRequest<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    base: Option<&'a str>,
}

#[derive(Serialize)]
struct MergePullRequest<'a> {
    merge_method: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    commit_title: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    commit_message: Option<&'a str>,
}

#[derive(Serialize)]
struct CreateCommentRequest<'a> {
    body: &'a str,
}

#[derive(Debug, Deserialize)]
struct GitHubIssueComment {
    id: u64,
    body: Option<String>,
    user: GitHubUser,
    created_at: DateTime<Utc>,
}

fn normalize_github_state(pr: &GitHubPullRequest) -> String {
    if pr.merged.unwrap_or(false) {
        return "MERGED".to_string();
    }
    pr.state
        .as_deref()
        .map(|s| s.to_uppercase())
        .unwrap_or_else(|| "OPEN".to_string())
}

fn github_state_to_pr_state(pr: &GitHubPullRequest) -> PrState {
    if pr.merged.unwrap_or(false) {
        return PrState::Merged;
    }
    match pr
        .state
        .as_deref()
        .unwrap_or("open")
        .to_lowercase()
        .as_str()
    {
        "closed" => PrState::Closed,
        "open" if pr.draft.unwrap_or(false) => PrState::Draft,
        "open" => PrState::Open,
        _ => PrState::Open,
    }
}

fn pr_to_info(pr: &GitHubPullRequest) -> PrInfo {
    PrInfo::new(
        pr.number as u32,
        pr.html_url.clone().unwrap_or_default(),
        pr.title.clone().unwrap_or_default(),
        pr.body.clone().unwrap_or_default(),
        pr.user
            .as_ref()
            .map(|u| u.login.clone())
            .unwrap_or_default(),
        pr.draft.unwrap_or(false),
    )
    .with_state(github_state_to_pr_state(pr))
}

fn github_check_status(status: &str) -> String {
    match status {
        "queued" | "in_progress" | "waiting" | "requested" | "pending" => {
            "in_progress".to_string()
        }
        _ => "completed".to_string(),
    }
}

fn github_check_conclusion(conclusion: Option<&str>) -> Option<String> {
    conclusion.map(|c| match c {
        "success" | "skipped" | "neutral" => "success".to_string(),
        "failure" | "timed_out" | "cancelled" | "action_required" => "failure".to_string(),
        _ => c.to_string(),
    })
}

impl GitHubClient {
    pub fn new(remote: &RemoteInfo) -> Result<Self> {
        if remote.forge != ForgeType::GitHub {
            return Err(StackError::ForgeError(
                "Internal error: expected GitHub remote".to_string(),
            ));
        }

        let token = forge_token(ForgeType::GitHub).ok_or_else(|| {
            StackError::ForgeError(
                "GitHub auth not configured. Set STAX_GITHUB_TOKEN or GITHUB_TOKEN.".to_string(),
            )
        })?;

        let api_base_url = if remote.api_base_url.is_empty() {
            GITHUB_API_BASE_URL.to_string()
        } else {
            remote.api_base_url.clone()
        };

        Ok(Self {
            client: build_http_client(&token, AuthStyle::AuthorizationToken)?,
            api_base_url,
            owner: remote.owner.clone(),
            repo: remote.repo.clone(),
            rate_limiter: Arc::new(RateLimiter::new()),
        })
    }

    pub fn with_token(remote: &RemoteInfo, token: &str) -> Result<Self> {
        let api_base_url = if remote.api_base_url.is_empty() {
            GITHUB_API_BASE_URL.to_string()
        } else {
            remote.api_base_url.clone()
        };

        Ok(Self {
            client: build_http_client(token, AuthStyle::AuthorizationToken)?,
            api_base_url,
            owner: remote.owner.clone(),
            repo: remote.repo.clone(),
            rate_limiter: Arc::new(RateLimiter::new()),
        })
    }

    fn repo_url(&self, suffix: &str) -> String {
        format!("{}/repos/{}/{}{}", self.api_base_url, self.owner, self.repo, suffix)
    }

    async fn request_with_retry<R, F, Fut>(&self, request_fn: F) -> Result<R>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<R>>,
    {
        let mut attempts = 0;
        loop {
            self.rate_limiter.wait_if_needed().await;
            match request_fn().await {
                Ok(result) => return Ok(result),
                Err(e) if attempts < RETRY_MAX_ATTEMPTS && is_retryable_error(&e) => {
                    attempts += 1;
                    let delay = Duration::from_millis(
                        RETRY_BASE_DELAY_MS * 2u64.pow(attempts as u32 - 1),
                    );
                    tokio::time::sleep(delay).await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    async fn get_with_retry<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T> {
        self.request_with_retry(|| async {
            let response = self
                .client
                .get(url)
                .send()
                .await
                .map_err(|e| StackError::ForgeError(format!("GET {url}: {e}")))?;
            self.update_rate_limiter(&response);
            parse_json_response(response).await
        })
        .await
    }

    async fn post_with_retry<T: for<'de> Deserialize<'de>, B: Serialize>(
        &self,
        url: &str,
        body: &B,
    ) -> Result<T> {
        self.request_with_retry(|| async {
            let response = self
                .client
                .post(url)
                .json(body)
                .send()
                .await
                .map_err(|e| StackError::ForgeError(format!("POST {url}: {e}")))?;
            self.update_rate_limiter(&response);
            parse_json_response(response).await
        })
        .await
    }

    async fn put_with_retry<T: for<'de> Deserialize<'de>, B: Serialize>(
        &self,
        url: &str,
        body: &B,
    ) -> Result<T> {
        self.request_with_retry(|| async {
            let response = self
                .client
                .put(url)
                .json(body)
                .send()
                .await
                .map_err(|e| StackError::ForgeError(format!("PUT {url}: {e}")))?;
            self.update_rate_limiter(&response);
            parse_json_response(response).await
        })
        .await
    }

    async fn patch_with_retry<T: for<'de> Deserialize<'de>, B: Serialize>(
        &self,
        url: &str,
        body: &B,
    ) -> Result<T> {
        self.request_with_retry(|| async {
            let response = self
                .client
                .patch(url)
                .json(body)
                .send()
                .await
                .map_err(|e| StackError::ForgeError(format!("PATCH {url}: {e}")))?;
            self.update_rate_limiter(&response);
            parse_json_response(response).await
        })
        .await
    }

    fn update_rate_limiter(&self, response: &reqwest::Response) {
        let remaining = response
            .headers()
            .get("x-ratelimit-remaining")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok());
        let reset = response
            .headers()
            .get("x-ratelimit-reset")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok());
        self.rate_limiter.update_from_headers(remaining, reset);
    }

    pub async fn find_open_pr_by_head(&self, head_owner: &str, branch: &str) -> Result<Option<PrInfo>> {
        let url = format!(
            "{}?state=open&head={}:{}&per_page=100",
            self.repo_url("/pulls"),
            head_owner,
            encode_query_value(branch)
        );
        let prs: Vec<GitHubPullRequest> = self.get_with_retry(&url).await?;
        Ok(prs
            .into_iter()
            .find(|pr| {
                pr.head.ref_name == branch
                    && pr.head.label.as_ref().map(|l| l.starts_with(head_owner)).unwrap_or(true)
            })
            .map(|pr| pr_to_info(&pr)))
    }

    pub async fn find_pr(&self, branch: &str) -> Result<Option<PrInfo>> {
        if let Some(pr) = self.find_open_pr_by_head(&self.owner, branch).await? {
            return Ok(Some(pr));
        }
        Ok(self.list_open_prs_by_head()
            .await?
            .get(branch)
            .cloned())
    }

    pub async fn list_open_prs_by_head(&self) -> Result<std::collections::HashMap<String, PrInfo>> {
        let url = format!("{}?state=open&per_page=100", self.repo_url("/pulls"));
        let prs: Vec<GitHubPullRequest> = self.get_with_retry(&url).await?;
        Ok(prs
            .into_iter()
            .map(|pr| {
                let head = pr.head.ref_name.clone();
                (head, pr_to_info(&pr))
            })
            .collect())
    }

    pub async fn create_pr(
        &self,
        head: &str,
        base: &str,
        title: &str,
        body: &str,
        draft: bool,
    ) -> Result<PrInfo> {
        let request = CreatePullRequest {
            title,
            head,
            base,
            body,
            draft,
        };
        let pr: GitHubPullRequest = self
            .post_with_retry(&self.repo_url("/pulls"), &request)
            .await?;
        Ok(pr_to_info(&pr))
    }

    pub async fn get_pr(&self, pr_number: u64) -> Result<PrInfo> {
        let pr: GitHubPullRequest = self.get_with_retry(&self.repo_url(&format!("/pulls/{pr_number}"))).await?;
        Ok(pr_to_info(&pr))
    }

    pub async fn update_pr(
        &self,
        pr_number: u64,
        title: Option<&str>,
        body: Option<&str>,
        base: Option<&str>,
    ) -> Result<PrInfo> {
        let request = UpdatePullRequest {
            title,
            body,
            base,
        };
        let pr: GitHubPullRequest = self
            .patch_with_retry(&self.repo_url(&format!("/pulls/{pr_number}")), &request)
            .await?;
        Ok(pr_to_info(&pr))
    }

    pub async fn update_pr_body(&self, pr_number: u64, body: &str) -> Result<()> {
        let request = UpdatePullRequest {
            title: None,
            body: Some(body),
            base: None,
        };
        let _: GitHubPullRequest = self
            .patch_with_retry(&self.repo_url(&format!("/pulls/{pr_number}")), &request)
            .await?;
        Ok(())
    }

    pub async fn get_pr_body(&self, pr_number: u64) -> Result<String> {
        let pr: GitHubPullRequest = self.get_with_retry(&self.repo_url(&format!("/pulls/{pr_number}"))).await?;
        Ok(pr.body.unwrap_or_default())
    }

    pub async fn merge_pr(
        &self,
        pr_number: u64,
        method: MergeMethod,
        commit_title: Option<&str>,
        commit_message: Option<&str>,
    ) -> Result<()> {
        let merge_method = match method {
            MergeMethod::Merge => "merge",
            MergeMethod::Squash => "squash",
            MergeMethod::Rebase => "rebase",
        };
        let request = MergePullRequest {
            merge_method,
            commit_title,
            commit_message,
        };
        let _: Value = self
            .put_with_retry(&self.repo_url(&format!("/pulls/{pr_number}/merge")), &request)
            .await?;
        Ok(())
    }

    pub async fn is_pr_merged(&self, pr_number: u64) -> Result<bool> {
        let pr: GitHubPullRequest = self.get_with_retry(&self.repo_url(&format!("/pulls/{pr_number}"))).await?;
        Ok(pr.merged.unwrap_or(false))
    }

    pub async fn update_stack_comment(&self, pr_number: u64, stack_comment: &str) -> Result<()> {
        if let Some(comment_id) = self.find_stack_comment_id(pr_number).await? {
            let full_comment = stack_comment_body(stack_comment);
            let request = CreateCommentRequest { body: &full_comment };
            let _: GitHubComment = self
                .patch_with_retry(
                    &self.repo_url(&format!("/issues/comments/{comment_id}")),
                    &request,
                )
                .await?;
            Ok(())
        } else {
            self.create_stack_comment(pr_number, stack_comment).await
        }
    }

    pub async fn create_stack_comment(&self, pr_number: u64, stack_comment: &str) -> Result<()> {
        let full_comment = stack_comment_body(stack_comment);
        let request = CreateCommentRequest { body: &full_comment };
        let _: GitHubComment = self
            .post_with_retry(&self.repo_url(&format!("/issues/{pr_number}/comments")), &request)
            .await?;
        Ok(())
    }

    pub async fn delete_stack_comment(&self, pr_number: u64) -> Result<()> {
        if let Some(comment_id) = self.find_stack_comment_id(pr_number).await? {
            delete_empty(
                &self.client,
                &self.repo_url(&format!("/issues/comments/{comment_id}")),
            )
            .await?;
        }
        Ok(())
    }

    async fn find_stack_comment_id(&self, pr_number: u64) -> Result<Option<u64>> {
        let url = self.repo_url(&format!("/issues/{pr_number}/comments?per_page=100"));
        let comments: Vec<GitHubIssueComment> = self.get_with_retry(&url).await?;
        Ok(comments
            .into_iter()
            .find(|c| c.body.as_ref().map(|b| b.contains(STACK_COMMENT_MARKER)).unwrap_or(false))
            .map(|c| c.id))
    }

    pub async fn list_issue_comments(&self, pr_number: u64) -> Result<Vec<GitHubComment>> {
        let url = self.repo_url(&format!("/issues/{pr_number}/comments?per_page=100"));
        self.get_with_retry(&url).await
    }

    pub async fn list_review_comments(&self, pr_number: u64) -> Result<Vec<GitHubReviewComment>> {
        let url = self.repo_url(&format!("/pulls/{pr_number}/comments?per_page=100"));
        self.get_with_retry(&url).await
    }

    pub async fn get_current_user(&self) -> Result<String> {
        let url = format!("{}/user", self.api_base_url);
        let user: GitHubCurrentUser = self.get_with_retry(&url).await?;
        Ok(user.login)
    }

    pub async fn combined_status_state(&self, commit_sha: &str) -> Result<Option<String>> {
        let url = self.repo_url(&format!("/commits/{commit_sha}/status"));
        let status: GitHubCommitStatus = self.get_with_retry(&url).await?;
        let overall = aggregate_ci_overall(
            status.statuses.iter().map(|s| s.state.as_str()),
            |s| matches!(s, "failure" | "error"),
            |s| matches!(s, "pending" | "queued" | "running"),
        );
        Ok(overall)
    }

    pub async fn get_check_runs_status(&self, commit_sha: &str) -> Result<Option<String>> {
        let url = self.repo_url(&format!("/commits/{commit_sha}/check-runs"));
        let response: GitHubCheckRuns = self.get_with_retry(&url).await?;
        if response.total_count == 0 {
            return Ok(None);
        }
        let mut latest_by_name: std::collections::HashMap<&str, &GitHubCheckRun> =
            std::collections::HashMap::new();
        for run in &response.check_runs {
            let entry = latest_by_name.entry(&run.name).or_insert(run);
            if run.id > entry.id {
                *entry = run;
            }
        }
        let mut has_failure = false;
        let mut has_pending = false;
        let mut all_success = true;
        for run in latest_by_name.values() {
            match run.status.as_str() {
                "completed" => {
                    if let Some(c) = run.conclusion.as_deref() {
                        match c {
                            "success" | "skipped" | "neutral" => {}
                            "failure"
                            | "timed_out"
                            | "cancelled"
                            | "action_required" => {
                                has_failure = true;
                                all_success = false;
                            }
                            _ => {
                                all_success = false;
                            }
                        }
                    }
                }
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

    pub async fn fetch_checks(&self, sha: &str) -> Result<(Option<String>, Vec<CheckRunInfo>)> {
        let url = self.repo_url(&format!("/commits/{sha}/check-runs"));
        let response: GitHubCheckRuns = self.get_with_retry(&url).await?;
        let checks: Vec<CheckRunInfo> = response
            .check_runs
            .iter()
            .map(|run| CheckRunInfo {
                name: run.name.clone(),
                status: github_check_status(&run.status),
                conclusion: github_check_conclusion(run.conclusion.as_deref()),
                url: None,
                started_at: None,
                completed_at: None,
            })
            .collect();
        let overall = aggregate_ci_overall(
            response.check_runs.iter().map(|r| r.status.as_str()),
            |s| matches!(s, "failure" | "timed_out" | "cancelled"),
            |s| matches!(s, "queued" | "in_progress" | "waiting" | "requested" | "pending"),
        );
        Ok((overall, checks))
    }

    pub async fn request_reviewers(&self, pr_number: u64, reviewers: &[String]) -> Result<()> {
        if reviewers.is_empty() {
            return Ok(());
        }
        let url = self.repo_url(&format!("/pulls/{pr_number}/requested_reviewers"));
        let request = serde_json::json!({ "reviewers": reviewers });
        let _: Value = self.post_with_retry(&url, &request).await?;
        Ok(())
    }

    pub async fn add_labels(&self, pr_number: u64, labels: &[String]) -> Result<()> {
        if labels.is_empty() {
            return Ok(());
        }
        let url = self.repo_url(&format!("/issues/{pr_number}/labels"));
        let request = serde_json::json!({ "labels": labels });
        let _: Value = self.post_with_retry(&url, &request).await?;
        Ok(())
    }

    pub async fn update_pr_branch(&self, pr_number: u64) -> Result<()> {
        let url = self.repo_url(&format!("/pulls/{pr_number}/update-branch"));
        let result: Result<Value> = self
            .put_with_retry(&url, &serde_json::json!({}))
            .await;
        match result {
            Ok(_) => Ok(()),
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("Update is not required") || msg.contains("no new commits") {
                    Ok(())
                } else {
                    Err(e)
                }
            }
        }
    }
}

fn is_retryable_error(err: &StackError) -> bool {
    match err {
        StackError::ForgeError(msg) => {
            msg.contains("rate limit")
                || msg.contains("timeout")
                || msg.contains("connection")
                || msg.contains("503")
                || msg.contains("502")
                || msg.contains("429")
        }
        _ => false,
    }
}

async fn parse_json_response<T: for<'de> Deserialize<'de>>(response: reqwest::Response) -> Result<T> {
    if response.status().is_success() {
        response
            .json::<T>()
            .await
            .map_err(|e| StackError::ForgeError(format!("JSON parse error: {e}")))
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(StackError::ForgeError(format!("Forge API error: {status} {body}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn remote_info(api_url: &str) -> RemoteInfo {
        RemoteInfo {
            forge: ForgeType::GitHub,
            owner: "owner".to_string(),
            repo: "repo".to_string(),
            api_base_url: api_url.to_string(),
        }
    }

    #[test]
    fn test_github_client_rejects_non_github_remote() {
        let remote = RemoteInfo {
            forge: ForgeType::GitLab,
            owner: "org".to_string(),
            repo: "repo".to_string(),
            api_base_url: "https://gitlab.example.com/api/v4".to_string(),
        };
        let result = GitHubClient::new(&remote);
        assert!(result.is_err());
    }

    #[test]
    fn test_github_client_with_token() {
        let remote = remote_info("");
        let result = GitHubClient::with_token(&remote, "test-token");
        assert!(result.is_ok());
    }

    #[test]
    fn test_github_state_to_pr_state_open() {
        let pr = GitHubPullRequest {
            number: 1,
            state: Some("open".to_string()),
            title: Some("Test".to_string()),
            body: None,
            draft: Some(false),
            merged: Some(false),
            mergeable: None,
            mergeable_state: None,
            head: GitHubBranchRef {
                ref_name: "feat".to_string(),
                sha: None,
                label: None,
            },
            base: GitHubBranchRef {
                ref_name: "main".to_string(),
                sha: None,
                label: None,
            },
            user: None,
            html_url: None,
            merged_at: None,
            created_at: None,
            updated_at: None,
        };
        assert_eq!(github_state_to_pr_state(&pr), PrState::Open);
    }

    #[test]
    fn test_github_state_to_pr_state_draft() {
        let pr = GitHubPullRequest {
            number: 1,
            state: Some("open".to_string()),
            title: Some("Test".to_string()),
            body: None,
            draft: Some(true),
            merged: Some(false),
            mergeable: None,
            mergeable_state: None,
            head: GitHubBranchRef {
                ref_name: "feat".to_string(),
                sha: None,
                label: None,
            },
            base: GitHubBranchRef {
                ref_name: "main".to_string(),
                sha: None,
                label: None,
            },
            user: None,
            html_url: None,
            merged_at: None,
            created_at: None,
            updated_at: None,
        };
        assert_eq!(github_state_to_pr_state(&pr), PrState::Draft);
    }

    #[test]
    fn test_github_state_to_pr_state_merged() {
        let pr = GitHubPullRequest {
            number: 1,
            state: Some("closed".to_string()),
            title: Some("Test".to_string()),
            body: None,
            draft: Some(false),
            merged: Some(true),
            mergeable: None,
            mergeable_state: None,
            head: GitHubBranchRef {
                ref_name: "feat".to_string(),
                sha: None,
                label: None,
            },
            base: GitHubBranchRef {
                ref_name: "main".to_string(),
                sha: None,
                label: None,
            },
            user: None,
            html_url: None,
            merged_at: None,
            created_at: None,
            updated_at: None,
        };
        assert_eq!(github_state_to_pr_state(&pr), PrState::Merged);
    }

    #[test]
    fn test_normalize_github_state_merged() {
        let pr = GitHubPullRequest {
            number: 1,
            state: Some("closed".to_string()),
            title: None,
            body: None,
            draft: None,
            merged: Some(true),
            mergeable: None,
            mergeable_state: None,
            head: GitHubBranchRef {
                ref_name: "".to_string(),
                sha: None,
                label: None,
            },
            base: GitHubBranchRef {
                ref_name: "".to_string(),
                sha: None,
                label: None,
            },
            user: None,
            html_url: None,
            merged_at: None,
            created_at: None,
            updated_at: None,
        };
        assert_eq!(normalize_github_state(&pr), "MERGED");
    }

    #[test]
    fn test_github_check_status() {
        assert_eq!(github_check_status("in_progress"), "in_progress");
        assert_eq!(github_check_status("queued"), "in_progress");
        assert_eq!(github_check_status("completed"), "completed");
    }

    #[test]
    fn test_github_check_conclusion() {
        assert_eq!(
            github_check_conclusion(Some("success")),
            Some("success".to_string())
        );
        assert_eq!(
            github_check_conclusion(Some("failure")),
            Some("failure".to_string())
        );
        assert_eq!(github_check_conclusion(None), None);
    }
}
