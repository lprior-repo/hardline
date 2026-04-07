use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{
    build_http_client, delete_empty, encode_query_value, forge_token, get_json, patch_json,
    post_json, stack_comment_body, aggregate_ci_overall, mergeable_bool,
    AuthStyle, CheckRunInfo, ForgeType, MergeMethod, RemoteInfo, STACK_COMMENT_MARKER,
};
use crate::domain::state::PrState;
use crate::domain::stack::PrInfo;
use crate::error::{Result, StackError};

/// HTTP client for the Gitea forge API.
#[derive(Clone)]
pub struct GiteaClient {
    client: reqwest::Client,
    api_base_url: String,
    owner: String,
    repo: String,
}

// --- Gitea API response types ---

#[derive(Debug, Deserialize)]
struct GiteaPull {
    number: u64,
    state: Option<String>,
    title: Option<String>,
    body: Option<String>,
    draft: Option<bool>,
    mergeable: Option<bool>,
    mergeable_state: Option<String>,
    merged: Option<bool>,
    head: GiteaBranchRef,
    base: GiteaBranchRef,
    user: Option<GiteaUser>,
    html_url: Option<String>,
    created_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct GiteaBranchRef {
    #[serde(rename = "ref")]
    ref_name: String,
    sha: Option<String>,
    label: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GiteaUser {
    login: String,
}

#[derive(Debug, Deserialize)]
struct GiteaComment {
    id: u64,
    body: String,
    created_at: DateTime<Utc>,
    user: GiteaUser,
}

#[derive(Debug, Deserialize)]
struct GiteaCommitStatus {
    context: Option<String>,
    status: Option<String>,
    target_url: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GiteaIssue {
    number: u64,
    title: String,
    html_url: Option<String>,
    user: Option<GiteaUser>,
    labels: Vec<GiteaLabel>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct GiteaLabel {
    name: Option<String>,
}

// --- Gitea API request types ---

#[derive(Serialize)]
struct CreatePullRequest<'a> {
    head: &'a str,
    base: &'a str,
    title: &'a str,
    body: &'a str,
    draft: bool,
}

#[derive(Serialize)]
struct UpdatePullRequest<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    base: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<&'a str>,
}

#[derive(Serialize)]
struct MergePullRequest<'a> {
    #[serde(rename = "MergeTitleField", skip_serializing_if = "Option::is_none")]
    merge_title: Option<&'a str>,
    #[serde(rename = "Do")]
    do_field: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    head_commit_id: Option<&'a str>,
}

#[derive(Serialize)]
struct CreateCommentRequest<'a> {
    body: &'a str,
}

// --- Conversion helpers ---

fn normalize_gitea_state_str(state: &str, merged: Option<bool>) -> String {
    if merged.unwrap_or(false) {
        "MERGED".to_string()
    } else {
        state.to_uppercase()
    }
}

fn normalize_gitea_state(pr: &GiteaPull) -> String {
    normalize_gitea_state_str(
        pr.state.as_deref().unwrap_or("open"),
        pr.merged,
    )
}

fn gitea_state_to_pr_state(pr: &GiteaPull) -> PrState {
    if pr.merged.unwrap_or(false) {
        return PrState::Merged;
    }
    match pr.state.as_deref().unwrap_or("open").to_lowercase().as_str() {
        "closed" => PrState::Closed,
        "open" if pr.draft.unwrap_or(false) => PrState::Draft,
        "open" => PrState::Open,
        _ => PrState::Open,
    }
}

fn pr_to_info(pr: &GiteaPull) -> PrInfo {
    PrInfo::new(
        pr.number as u32,
        pr.html_url.clone().unwrap_or_default(),
        pr.title.clone().unwrap_or_default(),
        pr.body.clone().unwrap_or_default(),
        pr.user.as_ref().map(|u| u.login.clone()).unwrap_or_default(),
        pr.draft.unwrap_or(false),
    )
    .with_state(gitea_state_to_pr_state(pr))
}

fn normalize_gitea_status(status: Option<&str>) -> String {
    match status.unwrap_or("") {
        "pending" => "in_progress".to_string(),
        _ => "completed".to_string(),
    }
}

fn normalize_gitea_conclusion(status: &str) -> String {
    match status {
        "success" => "success".to_string(),
        "failure" | "error" => "failure".to_string(),
        _ => status.to_string(),
    }
}

impl GiteaClient {
    /// Create a new Gitea client from remote info.
    ///
    /// Reads the API token from environment variables:
    /// `STAX_GITEA_TOKEN`, `GITEA_TOKEN`, or `STAX_FORGE_TOKEN`.
    pub fn new(remote: &RemoteInfo) -> Result<Self> {
        if remote.forge != ForgeType::Gitea {
            return Err(StackError::ForgeError(
                "Internal error: expected Gitea remote".to_string(),
            ));
        }

        let token = forge_token(ForgeType::Gitea).ok_or_else(|| {
            StackError::ForgeError(
                "Gitea auth not configured. Set STAX_GITEA_TOKEN, GITEA_TOKEN, or STAX_FORGE_TOKEN.".to_string(),
            )
        })?;

        Ok(Self {
            client: build_http_client(&token, AuthStyle::AuthorizationToken)?,
            api_base_url: remote.api_base_url.clone(),
            owner: remote.owner.clone(),
            repo: remote.repo.clone(),
        })
    }

    /// Create a client with an explicit token (for testing).
    pub fn with_token(remote: &RemoteInfo, token: &str) -> Result<Self> {
        Ok(Self {
            client: build_http_client(token, AuthStyle::AuthorizationToken)?,
            api_base_url: remote.api_base_url.clone(),
            owner: remote.owner.clone(),
            repo: remote.repo.clone(),
        })
    }

    fn repo_url(&self, suffix: &str) -> String {
        format!(
            "{}/repos/{}/{}/{}",
            self.api_base_url,
            encode_query_value(&self.owner),
            encode_query_value(&self.repo),
            suffix
        )
    }

    // --- Pull request operations ---

    /// Find an open PR by head branch name.
    pub async fn find_open_pr_by_head(&self, branch: &str) -> Result<Option<PrInfo>> {
        let url = format!("{}?state=open&limit=50", self.repo_url("pulls"));
        let prs: Vec<GiteaPull> = get_json(&self.client, &url).await?;
        Ok(prs
            .into_iter()
            .find(|pr| pr.head.ref_name == branch)
            .map(|pr| pr_to_info(&pr)))
    }

    /// List all open PRs keyed by head branch name.
    pub async fn list_open_prs_by_head(&self) -> Result<HashMap<String, PrInfo>> {
        let url = format!("{}?state=open&limit=50", self.repo_url("pulls"));
        let prs: Vec<GiteaPull> = get_json(&self.client, &url).await?;
        Ok(prs
            .iter()
            .map(|pr| (pr.head.ref_name.clone(), pr_to_info(pr)))
            .collect())
    }

    /// Create a new pull request.
    pub async fn create_pr(
        &self,
        head: &str,
        base: &str,
        title: &str,
        body: &str,
        is_draft: bool,
    ) -> Result<PrInfo> {
        let request = CreatePullRequest {
            head,
            base,
            title,
            body,
            draft: is_draft,
        };
        let pr: GiteaPull = post_json(&self.client, &self.repo_url("pulls"), &request).await?;
        Ok(pr_to_info(&pr))
    }

    /// Get a pull request by number.
    pub async fn get_pr(&self, number: u64) -> Result<PrInfo> {
        let pr: GiteaPull =
            get_json(&self.client, &self.repo_url(&format!("pulls/{number}"))).await?;
        Ok(pr_to_info(&pr))
    }

    /// Update the base branch of a pull request.
    pub async fn update_pr_base(&self, number: u64, new_base: &str) -> Result<()> {
        let request = UpdatePullRequest {
            base: Some(new_base),
            body: None,
        };
        let _: GiteaPull = patch_json(
            &self.client,
            &self.repo_url(&format!("pulls/{number}")),
            &request,
        )
        .await?;
        Ok(())
    }

    /// Update the body/description of a pull request.
    pub async fn update_pr_body(&self, number: u64, body: &str) -> Result<()> {
        let request = UpdatePullRequest {
            base: None,
            body: Some(body),
        };
        let _: GiteaPull = patch_json(
            &self.client,
            &self.repo_url(&format!("pulls/{number}")),
            &request,
        )
        .await?;
        Ok(())
    }

    /// Get the body/description of a pull request.
    pub async fn get_pr_body(&self, number: u64) -> Result<String> {
        let pr: GiteaPull =
            get_json(&self.client, &self.repo_url(&format!("pulls/{number}"))).await?;
        Ok(pr.body.unwrap_or_default())
    }

    /// Merge a pull request.
    pub async fn merge_pr(
        &self,
        number: u64,
        method: MergeMethod,
        commit_title: Option<&str>,
        sha: Option<&str>,
    ) -> Result<()> {
        let request = MergePullRequest {
            merge_title: commit_title,
            do_field: method.as_gitea_str(),
            head_commit_id: sha,
        };
        let _: serde_json::Value = post_json(
            &self.client,
            &self.repo_url(&format!("pulls/{number}/merge")),
            &request,
        )
        .await?;
        Ok(())
    }

    /// Check if a pull request has been merged.
    pub async fn is_pr_merged(&self, number: u64) -> Result<bool> {
        let pr: GiteaPull =
            get_json(&self.client, &self.repo_url(&format!("pulls/{number}"))).await?;
        Ok(pr.merged.unwrap_or(false))
    }

    /// Get the merge status of a pull request.
    pub async fn get_pr_merge_status(&self, number: u64) -> Result<PrMergeStatus> {
        let pr: GiteaPull =
            get_json(&self.client, &self.repo_url(&format!("pulls/{number}"))).await?;

        let mergeable_state = pr.mergeable_state.clone().unwrap_or_else(|| {
            if pr.mergeable == Some(true) {
                "clean".to_string()
            } else {
                "unknown".to_string()
            }
        });

        let ci_status = self
            .fetch_checks(pr.head.sha.as_deref().unwrap_or_default())
            .await
            .ok()
            .and_then(|(status, _)| status);

        let state = normalize_gitea_state(&pr);
        let title = pr.title.as_ref().unwrap_or(&String::new()).clone();
        let head_sha = pr.head.sha.as_ref().unwrap_or(&String::new()).clone();
        Ok(PrMergeStatus {
            number: pr.number,
            title,
            state,
            is_draft: pr.draft.unwrap_or(false),
            mergeable: pr.mergeable.or_else(|| mergeable_bool(&mergeable_state)),
            mergeable_state,
            ci_status,
            head_sha,
        })
    }

    // --- Comment operations ---

    /// Update or create a stack comment on a PR.
    pub async fn update_stack_comment(&self, number: u64, stack_comment: &str) -> Result<()> {
        if let Some(comment_id) = self.find_stack_comment_id(number).await? {
            let body = serde_json::json!({ "body": stack_comment_body(stack_comment) });
            let _: GiteaComment = patch_json(
                &self.client,
                &self.repo_url(&format!("issues/comments/{comment_id}")),
                &body,
            )
            .await?;
            Ok(())
        } else {
            self.create_stack_comment(number, stack_comment).await
        }
    }

    /// Create a stack comment on a PR.
    pub async fn create_stack_comment(&self, number: u64, stack_comment: &str) -> Result<()> {
        let request = CreateCommentRequest {
            body: &stack_comment_body(stack_comment),
        };
        let _: GiteaComment = post_json(
            &self.client,
            &self.repo_url(&format!("issues/{number}/comments")),
            &request,
        )
        .await?;
        Ok(())
    }

    /// Delete the stack comment from a PR.
    pub async fn delete_stack_comment(&self, number: u64) -> Result<()> {
        let Some(comment_id) = self.find_stack_comment_id(number).await? else {
            return Ok(());
        };
        delete_empty(
            &self.client,
            &self.repo_url(&format!("issues/comments/{comment_id}")),
        )
        .await
    }

    /// List all comments on a PR, sorted by creation time.
    pub async fn list_all_comments(&self, number: u64) -> Result<Vec<PrComment>> {
        let comments: Vec<GiteaComment> = get_json(
            &self.client,
            &self.repo_url(&format!("issues/{number}/comments?limit=50")),
        )
        .await?;

        let mut comments: Vec<PrComment> = comments
            .into_iter()
            .map(|c| PrComment {
                id: c.id,
                body: c.body,
                user: c.user.login,
                created_at: c.created_at,
            })
            .collect();
        comments.sort_by_key(|c| c.created_at);
        Ok(comments)
    }

    async fn find_stack_comment_id(&self, number: u64) -> Result<Option<u64>> {
        let comments: Vec<GiteaComment> = get_json(
            &self.client,
            &self.repo_url(&format!("issues/{number}/comments?limit=50")),
        )
        .await?;
        Ok(comments
            .into_iter()
            .find(|c| c.body.contains(STACK_COMMENT_MARKER))
            .map(|c| c.id))
    }

    // --- CI/Checks ---

    /// Fetch CI check statuses for a commit SHA.
    pub async fn fetch_checks(&self, sha: &str) -> Result<(Option<String>, Vec<CheckRunInfo>)> {
        let statuses: Vec<GiteaCommitStatus> = get_json(
            &self.client,
            &self.repo_url(&format!("commits/{sha}/statuses?limit=50")),
        )
        .await?;

        let checks: Vec<CheckRunInfo> = statuses
            .iter()
            .map(|status| CheckRunInfo {
                name: status
                    .context
                    .clone()
                    .unwrap_or_else(|| "status".to_string()),
                status: normalize_gitea_status(status.status.as_deref()),
                conclusion: status.status.as_deref().map(normalize_gitea_conclusion),
                url: status.target_url.clone(),
                started_at: status.created_at.clone(),
                completed_at: status.updated_at.clone(),
            })
            .collect();

        let overall = aggregate_ci_overall(
            statuses.iter().filter_map(|s| s.status.as_deref()),
            |s| matches!(s, "failure" | "error"),
            |s| matches!(s, "pending"),
        );

        Ok((overall, checks))
    }

    // --- User operations ---

    /// Get the currently authenticated user's login.
    pub async fn get_current_user(&self) -> Result<String> {
        let url = format!("{}/user", self.api_base_url);
        let user: GiteaUser = get_json(&self.client, &url).await?;
        Ok(user.login)
    }

    // --- Listing operations ---

    /// List open pull requests in the repository.
    pub async fn list_open_pull_requests(&self, limit: u8) -> Result<Vec<RepoPrListItem>> {
        let limit = limit.clamp(1, 50);
        let url = format!(
            "{}?state=open&sort=newest&limit={limit}",
            self.repo_url("pulls")
        );
        let prs: Vec<GiteaPull> = get_json(&self.client, &url).await?;
        Ok(prs
            .iter()
            .map(|pr| RepoPrListItem {
                number: pr.number,
                title: pr.title.as_ref().unwrap_or(&String::new()).clone(),
                url: pr.html_url.as_ref().unwrap_or(&String::new()).clone(),
                author: pr.user.as_ref().map(|u| u.login.clone()).unwrap_or_default(),
                head_branch: pr.head.ref_name.clone(),
                base_branch: pr.base.ref_name.clone(),
                state: normalize_gitea_state_str(
                    pr.state.as_deref().unwrap_or("open"),
                    pr.merged,
                ),
                is_draft: pr.draft.unwrap_or(false),
                created_at: pr.created_at.unwrap_or_default(),
            })
            .collect())
    }

    /// List open issues in the repository.
    pub async fn list_open_issues(&self, limit: u8) -> Result<Vec<RepoIssueListItem>> {
        let limit = limit.clamp(1, 50);
        let url = format!(
            "{}?state=open&type=issues&sort=updated&limit={limit}",
            self.repo_url("issues")
        );
        let issues: Vec<GiteaIssue> = get_json(&self.client, &url).await?;
        Ok(issues
            .into_iter()
            .map(|issue| RepoIssueListItem {
                number: issue.number,
                title: issue.title,
                url: issue.html_url.unwrap_or_default(),
                author: issue.user.map(|u| u.login).unwrap_or_default(),
                labels: issue.labels.into_iter().filter_map(|l| l.name).collect(),
                updated_at: issue.updated_at,
            })
            .collect())
    }
}

/// Merge status information for a pull request.
#[derive(Debug, Clone)]
pub struct PrMergeStatus {
    pub number: u64,
    pub title: String,
    pub state: String,
    pub is_draft: bool,
    pub mergeable: Option<bool>,
    pub mergeable_state: String,
    pub ci_status: Option<String>,
    pub head_sha: String,
}

/// A comment on a pull request or issue.
#[derive(Debug, Clone)]
pub struct PrComment {
    pub id: u64,
    pub body: String,
    pub user: String,
    pub created_at: DateTime<Utc>,
}

/// Summary of an open pull request for listing.
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

/// Summary of an open issue for listing.
#[derive(Debug, Clone)]
pub struct RepoIssueListItem {
    pub number: u64,
    pub title: String,
    pub url: String,
    pub author: String,
    pub labels: Vec<String>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn remote_info(api_url: &str) -> RemoteInfo {
        RemoteInfo {
            forge: ForgeType::Gitea,
            owner: "org".to_string(),
            repo: "repo".to_string(),
            api_base_url: api_url.to_string(),
        }
    }

    #[test]
    fn test_gitea_client_rejects_non_gitea_remote() {
        let remote = RemoteInfo {
            forge: ForgeType::GitHub,
            owner: "org".to_string(),
            repo: "repo".to_string(),
            api_base_url: "https://api.github.com".to_string(),
        };
        let result = GiteaClient::new(&remote);
        assert!(result.is_err());
        let err = result.err().expect("should be error");
        assert!(err.to_string().contains("expected Gitea remote"));
    }

    #[test]
    fn test_gitea_client_with_token() {
        let remote = remote_info("https://gitea.example.com/api/v1");
        let result = GiteaClient::with_token(&remote, "test-token");
        assert!(result.is_ok());
    }

    #[test]
    fn test_normalize_gitea_state_str() {
        assert_eq!(normalize_gitea_state_str("open", None), "OPEN");
        assert_eq!(normalize_gitea_state_str("closed", None), "CLOSED");
        assert_eq!(normalize_gitea_state_str("open", Some(true)), "MERGED");
        assert_eq!(normalize_gitea_state_str("closed", Some(false)), "CLOSED");
    }

    #[test]
    fn test_normalize_gitea_status() {
        assert_eq!(normalize_gitea_status(Some("pending")), "in_progress");
        assert_eq!(normalize_gitea_status(Some("success")), "completed");
        assert_eq!(normalize_gitea_status(Some("failure")), "completed");
        assert_eq!(normalize_gitea_status(None), "completed");
    }

    #[test]
    fn test_normalize_gitea_conclusion() {
        assert_eq!(normalize_gitea_conclusion("success"), "success");
        assert_eq!(normalize_gitea_conclusion("failure"), "failure");
        assert_eq!(normalize_gitea_conclusion("error"), "failure");
        assert_eq!(normalize_gitea_conclusion("pending"), "pending");
    }

    #[test]
    fn test_repo_url_encoding() {
        let remote = remote_info("https://gitea.example.com/api/v1");
        let client = GiteaClient::with_token(&remote, "token").expect("client");
        let url = client.repo_url("pulls");
        assert!(url.contains("/repos/org/repo/pulls"));
    }

    #[test]
    fn test_gitea_state_to_pr_state() {
        let mut pr = GiteaPull {
            number: 1,
            state: Some("open".to_string()),
            title: Some("test".to_string()),
            body: None,
            draft: Some(false),
            mergeable: None,
            mergeable_state: None,
            merged: Some(false),
            head: GiteaBranchRef {
                ref_name: "feat".to_string(),
                sha: None,
                label: None,
            },
            base: GiteaBranchRef {
                ref_name: "main".to_string(),
                sha: None,
                label: None,
            },
            user: None,
            html_url: None,
            created_at: None,
            updated_at: None,
        };

        assert_eq!(gitea_state_to_pr_state(&pr), PrState::Open);

        pr.draft = Some(true);
        assert_eq!(gitea_state_to_pr_state(&pr), PrState::Draft);

        pr.merged = Some(true);
        assert_eq!(gitea_state_to_pr_state(&pr), PrState::Merged);

        pr.merged = Some(false);
        pr.state = Some("closed".to_string());
        pr.draft = Some(false);
        assert_eq!(gitea_state_to_pr_state(&pr), PrState::Closed);
    }
}
