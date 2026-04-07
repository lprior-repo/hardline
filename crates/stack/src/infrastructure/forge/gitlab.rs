use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{
    aggregate_ci_overall, build_http_client, delete_empty, encode_query_value, forge_token,
    get_json, mergeable_bool, post_json, put_json, stack_comment_body, AuthStyle, CheckRunInfo,
    ForgeType, MergeMethod, RemoteInfo, STACK_COMMENT_MARKER,
};
use crate::domain::state::PrState;
use crate::domain::stack::PrInfo;
use crate::error::{Result, StackError};

/// HTTP client for the GitLab forge API.
#[derive(Clone)]
pub struct GitLabClient {
    client: reqwest::Client,
    api_base_url: String,
    project_id: String,
}

// --- GitLab API response types ---

#[derive(Debug, Deserialize)]
struct GitLabMr {
    iid: u64,
    title: String,
    state: String,
    draft: Option<bool>,
    source_branch: String,
    target_branch: String,
    description: Option<String>,
    merge_status: Option<String>,
    detailed_merge_status: Option<String>,
    web_url: Option<String>,
    head_pipeline: Option<GitLabPipeline>,
    sha: Option<String>,
    author: Option<GitLabUser>,
    created_at: Option<DateTime<Utc>>,
    merged_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct GitLabPipeline {
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitLabUser {
    username: String,
}

#[derive(Debug, Deserialize)]
struct GitLabNote {
    id: u64,
    body: String,
    created_at: DateTime<Utc>,
    author: GitLabUser,
}

#[derive(Debug, Deserialize)]
struct GitLabCommitStatus {
    name: Option<String>,
    status: Option<String>,
    target_url: Option<String>,
    started_at: Option<String>,
    finished_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitLabIssue {
    iid: u64,
    title: String,
    web_url: Option<String>,
    author: Option<GitLabUser>,
    labels: Vec<String>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct GitLabApproval {
    user: GitLabUser,
    created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct GitLabApprovals {
    approved_by: Vec<GitLabApproval>,
}

// --- GitLab API request types ---

#[derive(Serialize)]
struct CreateMrRequest<'a> {
    source_branch: &'a str,
    target_branch: &'a str,
    title: &'a str,
    description: &'a str,
    remove_source_branch: bool,
    draft: bool,
}

#[derive(Serialize)]
struct UpdateMrRequest<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    target_branch: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
}

#[derive(Serialize)]
struct MergeMrRequest<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    merge_commit_message: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sha: Option<&'a str>,
    squash: bool,
}

#[derive(Serialize)]
struct CreateNoteRequest<'a> {
    body: &'a str,
}

// --- Conversion helpers ---

fn normalize_gitlab_state(state: &str) -> String {
    match state.to_ascii_lowercase().as_str() {
        "opened" => "OPEN".to_string(),
        "closed" => "CLOSED".to_string(),
        "merged" => "MERGED".to_string(),
        _ => state.to_ascii_uppercase(),
    }
}

fn gitlab_state_to_pr_state(mr: &GitLabMr) -> PrState {
    match mr.state.to_ascii_lowercase().as_str() {
        "merged" => PrState::Merged,
        "closed" => PrState::Closed,
        "opened" | "open" if mr.draft.unwrap_or(false) => PrState::Draft,
        "opened" | "open" => PrState::Open,
        _ => PrState::Open,
    }
}

fn mr_to_pr_info(mr: &GitLabMr) -> PrInfo {
    PrInfo::new(
        mr.iid as u32,
        mr.web_url.clone().unwrap_or_default(),
        mr.title.clone(),
        mr.description.clone().unwrap_or_default(),
        mr.author
            .as_ref()
            .map(|a| a.username.clone())
            .unwrap_or_default(),
        mr.draft.unwrap_or(false),
    )
    .with_state(gitlab_state_to_pr_state(mr))
}

fn normalize_gitlab_check_status(status: Option<&str>) -> String {
    match status.unwrap_or("") {
        "running" | "pending" | "created" => "in_progress".to_string(),
        _ => "completed".to_string(),
    }
}

fn normalize_gitlab_conclusion(status: &str) -> String {
    match status {
        "success" => "success".to_string(),
        "failed" => "failure".to_string(),
        "canceled" => "cancelled".to_string(),
        _ => status.to_string(),
    }
}

/// Build the URL-escaped project path for GitLab API.
fn encoded_project_path(remote: &RemoteInfo) -> String {
    encode_query_value(&format!("{}/{}", remote.owner, remote.repo))
}

impl GitLabClient {
    /// Create a new GitLab client from remote info.
    ///
    /// Reads the API token from environment variables:
    /// `STAX_GITLAB_TOKEN`, `GITLAB_TOKEN`, or `STAX_FORGE_TOKEN`.
    pub fn new(remote: &RemoteInfo) -> Result<Self> {
        if remote.forge != ForgeType::GitLab {
            return Err(StackError::ForgeError(
                "Internal error: expected GitLab remote".to_string(),
            ));
        }

        let token = forge_token(ForgeType::GitLab).ok_or_else(|| {
            StackError::ForgeError(
                "GitLab auth not configured. Set STAX_GITLAB_TOKEN, GITLAB_TOKEN, or STAX_FORGE_TOKEN.".to_string(),
            )
        })?;

        Ok(Self {
            client: build_http_client(&token, AuthStyle::PrivateToken)?,
            api_base_url: remote.api_base_url.clone(),
            project_id: encoded_project_path(remote),
        })
    }

    /// Create a client with an explicit token (for testing).
    pub fn with_token(remote: &RemoteInfo, token: &str) -> Result<Self> {
        Ok(Self {
            client: build_http_client(token, AuthStyle::PrivateToken)?,
            api_base_url: remote.api_base_url.clone(),
            project_id: encoded_project_path(remote),
        })
    }

    fn project_url(&self, suffix: &str) -> String {
        format!(
            "{}/projects/{}{}",
            self.api_base_url, self.project_id, suffix
        )
    }

    // --- Merge request operations ---

    /// Find an open MR by source branch name.
    pub async fn find_open_mr_by_head(&self, branch: &str) -> Result<Option<PrInfo>> {
        let url = format!(
            "{}?state=opened&source_branch={}&per_page=100",
            self.project_url("/merge_requests"),
            encode_query_value(branch)
        );
        let mrs: Vec<GitLabMr> = get_json(&self.client, &url).await?;
        Ok(mrs
            .into_iter()
            .find(|mr| mr.source_branch == branch)
            .map(|mr| mr_to_pr_info(&mr)))
    }

    /// List all open MRs keyed by source branch name.
    pub async fn list_open_mrs_by_head(&self) -> Result<HashMap<String, PrInfo>> {
        let url = format!(
            "{}?state=opened&per_page=100",
            self.project_url("/merge_requests")
        );
        let mrs: Vec<GitLabMr> = get_json(&self.client, &url).await?;
        Ok(mrs
            .iter()
            .map(|mr| (mr.source_branch.clone(), mr_to_pr_info(mr)))
            .collect())
    }

    /// Create a new merge request.
    pub async fn create_mr(
        &self,
        head: &str,
        base: &str,
        title: &str,
        body: &str,
        is_draft: bool,
    ) -> Result<PrInfo> {
        let request = CreateMrRequest {
            source_branch: head,
            target_branch: base,
            title,
            description: body,
            remove_source_branch: false,
            draft: is_draft,
        };
        let mr: GitLabMr =
            post_json(&self.client, &self.project_url("/merge_requests"), &request).await?;
        Ok(mr_to_pr_info(&mr))
    }

    /// Get a merge request by IID.
    pub async fn get_mr(&self, iid: u64) -> Result<PrInfo> {
        let mr: GitLabMr = get_json(
            &self.client,
            &self.project_url(&format!("/merge_requests/{iid}")),
        )
        .await?;
        Ok(mr_to_pr_info(&mr))
    }

    /// Update the target branch of a merge request.
    pub async fn update_mr_target(&self, iid: u64, new_target: &str) -> Result<()> {
        let request = UpdateMrRequest {
            target_branch: Some(new_target),
            description: None,
        };
        let _: GitLabMr = put_json(
            &self.client,
            &self.project_url(&format!("/merge_requests/{iid}")),
            &request,
        )
        .await?;
        Ok(())
    }

    /// Update the description of a merge request.
    pub async fn update_mr_body(&self, iid: u64, body: &str) -> Result<()> {
        let request = UpdateMrRequest {
            target_branch: None,
            description: Some(body),
        };
        let _: GitLabMr = put_json(
            &self.client,
            &self.project_url(&format!("/merge_requests/{iid}")),
            &request,
        )
        .await?;
        Ok(())
    }

    /// Get the description of a merge request.
    pub async fn get_mr_body(&self, iid: u64) -> Result<String> {
        let mr: GitLabMr = get_json(
            &self.client,
            &self.project_url(&format!("/merge_requests/{iid}")),
        )
        .await?;
        Ok(mr.description.unwrap_or_default())
    }

    /// Merge a merge request.
    pub async fn merge_mr(
        &self,
        iid: u64,
        method: MergeMethod,
        commit_title: Option<&str>,
        sha: Option<&str>,
    ) -> Result<()> {
        let request = MergeMrRequest {
            merge_commit_message: commit_title,
            sha,
            squash: matches!(method, MergeMethod::Squash),
        };
        let _: serde_json::Value = put_json(
            &self.client,
            &self.project_url(&format!("/merge_requests/{iid}/merge")),
            &request,
        )
        .await?;
        Ok(())
    }

    /// Check if a merge request has been merged.
    pub async fn is_mr_merged(&self, iid: u64) -> Result<bool> {
        let mr: GitLabMr = get_json(
            &self.client,
            &self.project_url(&format!("/merge_requests/{iid}")),
        )
        .await?;
        Ok(mr.state.eq_ignore_ascii_case("merged"))
    }

    /// Get the merge status of a merge request.
    pub async fn get_mr_merge_status(&self, iid: u64) -> Result<MrMergeStatus> {
        let mr: GitLabMr = get_json(
            &self.client,
            &self.project_url(&format!("/merge_requests/{iid}")),
        )
        .await?;

        let mergeable_state = mr
            .detailed_merge_status
            .clone()
            .or(mr.merge_status.clone())
            .unwrap_or_else(|| "unknown".to_string());

        let mergeable = mergeable_bool(&mergeable_state);

        let ci_status = mr
            .head_pipeline
            .as_ref()
            .and_then(|pipeline| pipeline.status.as_deref())
            .map(|status| {
                if matches!(status, "running" | "pending" | "created") {
                    "pending"
                } else {
                    status
                }
            });

        Ok(MrMergeStatus {
            iid: mr.iid,
            title: mr.title,
            state: normalize_gitlab_state(&mr.state),
            is_draft: mr.draft.unwrap_or(false),
            mergeable,
            mergeable_state,
            ci_status: ci_status.map(String::from),
            head_sha: mr.sha.unwrap_or_default(),
        })
    }

    // --- Note/comment operations ---

    /// Update or create a stack comment on an MR.
    pub async fn update_stack_comment(&self, iid: u64, stack_comment: &str) -> Result<()> {
        if let Some(note_id) = self.find_stack_comment_id(iid).await? {
            let body = serde_json::json!({ "body": stack_comment_body(stack_comment) });
            let _: GitLabNote = put_json(
                &self.client,
                &self.project_url(&format!("/merge_requests/{iid}/notes/{note_id}")),
                &body,
            )
            .await?;
            Ok(())
        } else {
            self.create_stack_comment(iid, stack_comment).await
        }
    }

    /// Create a stack comment on an MR.
    pub async fn create_stack_comment(&self, iid: u64, stack_comment: &str) -> Result<()> {
        let request = CreateNoteRequest {
            body: &stack_comment_body(stack_comment),
        };
        let _: GitLabNote = post_json(
            &self.client,
            &self.project_url(&format!("/merge_requests/{iid}/notes")),
            &request,
        )
        .await?;
        Ok(())
    }

    /// Delete the stack comment from an MR.
    pub async fn delete_stack_comment(&self, iid: u64) -> Result<()> {
        let Some(note_id) = self.find_stack_comment_id(iid).await? else {
            return Ok(());
        };
        delete_empty(
            &self.client,
            &self.project_url(&format!("/merge_requests/{iid}/notes/{note_id}")),
        )
        .await
    }

    /// List all notes/comments on an MR, sorted by creation time.
    pub async fn list_all_notes(&self, iid: u64) -> Result<Vec<MrNote>> {
        let notes: Vec<GitLabNote> = get_json(
            &self.client,
            &self.project_url(&format!("/merge_requests/{iid}/notes?per_page=100")),
        )
        .await?;

        let mut notes: Vec<MrNote> = notes
            .into_iter()
            .map(|note| MrNote {
                id: note.id,
                body: note.body,
                author: note.author.username,
                created_at: note.created_at,
            })
            .collect();
        notes.sort_by_key(|n| n.created_at);
        Ok(notes)
    }

    async fn find_stack_comment_id(&self, iid: u64) -> Result<Option<u64>> {
        let notes: Vec<GitLabNote> = get_json(
            &self.client,
            &self.project_url(&format!("/merge_requests/{iid}/notes?per_page=100")),
        )
        .await?;
        Ok(notes
            .into_iter()
            .find(|note| note.body.contains(STACK_COMMENT_MARKER))
            .map(|note| note.id))
    }

    // --- CI/Checks ---

    /// Fetch CI check statuses for a commit SHA.
    pub async fn fetch_checks(&self, sha: &str) -> Result<(Option<String>, Vec<CheckRunInfo>)> {
        let statuses: Vec<GitLabCommitStatus> = get_json(
            &self.client,
            &self.project_url(&format!(
                "/repository/commits/{sha}/statuses?per_page=100"
            )),
        )
        .await?;

        let checks: Vec<CheckRunInfo> = statuses
            .iter()
            .map(|status| CheckRunInfo {
                name: status
                    .name
                    .clone()
                    .unwrap_or_else(|| "pipeline".to_string()),
                status: normalize_gitlab_check_status(status.status.as_deref()),
                conclusion: status.status.as_deref().map(normalize_gitlab_conclusion),
                url: status.target_url.clone(),
                started_at: status.started_at.clone(),
                completed_at: status.finished_at.clone(),
            })
            .collect();

        let overall = aggregate_ci_overall(
            statuses.iter().filter_map(|s| s.status.as_deref()),
            |s| matches!(s, "failed" | "canceled"),
            |s| matches!(s, "running" | "pending" | "created"),
        );

        Ok((overall, checks))
    }

    // --- User operations ---

    /// Get the currently authenticated user's username.
    pub async fn get_current_user(&self) -> Result<String> {
        let url = format!("{}/user", self.api_base_url);
        let user: GitLabUser = get_json(&self.client, &url).await?;
        Ok(user.username)
    }

    // --- Listing operations ---

    /// List open merge requests in the project.
    pub async fn list_open_merge_requests(&self, limit: u8) -> Result<Vec<MrListItem>> {
        let per_page = limit.clamp(1, 100);
        let url = format!(
            "{}?state=opened&per_page={per_page}&order_by=created_at&sort=desc",
            self.project_url("/merge_requests")
        );
        let mrs: Vec<GitLabMr> = get_json(&self.client, &url).await?;
        Ok(mrs
            .iter()
            .map(|mr| MrListItem {
                iid: mr.iid,
                title: mr.title.clone(),
                url: mr.web_url.clone().unwrap_or_default(),
                author: mr
                    .author
                    .as_ref()
                    .map(|a| a.username.clone())
                    .unwrap_or_default(),
                source_branch: mr.source_branch.clone(),
                target_branch: mr.target_branch.clone(),
                state: normalize_gitlab_state(&mr.state),
                is_draft: mr.draft.unwrap_or(false),
                created_at: mr.created_at.unwrap_or_default(),
            })
            .collect())
    }

    /// List open issues in the project.
    pub async fn list_open_issues(&self, limit: u8) -> Result<Vec<IssueListItem>> {
        let per_page = limit.clamp(1, 100);
        let url = format!(
            "{}?state=opened&per_page={per_page}&order_by=updated_at&sort=desc",
            self.project_url("/issues")
        );
        let issues: Vec<GitLabIssue> = get_json(&self.client, &url).await?;
        Ok(issues
            .into_iter()
            .map(|issue| IssueListItem {
                iid: issue.iid,
                title: issue.title,
                url: issue.web_url.unwrap_or_default(),
                author: issue.author.map(|a| a.username).unwrap_or_default(),
                labels: issue.labels,
                updated_at: issue.updated_at,
            })
            .collect())
    }

    /// List open MRs for a specific user.
    pub async fn get_user_open_mrs(&self, username: &str) -> Result<Vec<UserMrInfo>> {
        let url = format!(
            "{}?state=opened&author_username={}&per_page=100",
            self.project_url("/merge_requests"),
            encode_query_value(username)
        );
        let mrs: Vec<GitLabMr> = get_json(&self.client, &url).await?;
        Ok(mrs
            .iter()
            .map(|mr| UserMrInfo {
                iid: mr.iid,
                source_branch: mr.source_branch.clone(),
                target_branch: mr.target_branch.clone(),
                state: normalize_gitlab_state(&mr.state),
                is_draft: mr.draft.unwrap_or(false),
            })
            .collect())
    }

    /// Get recently merged MRs for a user.
    pub async fn get_recent_merged_mrs(
        &self,
        hours: i64,
        username: &str,
    ) -> Result<Vec<MrActivity>> {
        let since = Utc::now() - chrono::Duration::hours(hours);
        let url = format!(
            "{}?state=merged&author_username={}&updated_after={}&per_page=30&order_by=updated_at&sort=desc",
            self.project_url("/merge_requests"),
            encode_query_value(username),
            since.to_rfc3339()
        );
        let mrs: Vec<GitLabMr> = get_json(&self.client, &url).await?;
        Ok(mrs
            .into_iter()
            .filter_map(|mr| {
                let ts = mr.merged_at.or(mr.updated_at)?;
                if ts < since {
                    return None;
                }
                Some(MrActivity {
                    iid: mr.iid,
                    title: mr.title,
                    timestamp: ts,
                    url: mr.web_url.unwrap_or_default(),
                })
            })
            .collect())
    }

    /// Get approvals received on a user's open MRs.
    pub async fn get_approvals_received(
        &self,
        hours: i64,
        username: &str,
    ) -> Result<Vec<ReviewActivity>> {
        let since = Utc::now() - chrono::Duration::hours(hours);
        let url = format!(
            "{}?state=opened&author_username={}&per_page=20",
            self.project_url("/merge_requests"),
            encode_query_value(username)
        );
        let mrs: Vec<GitLabMr> = get_json(&self.client, &url).await?;

        let mut reviews = Vec::new();
        for mr in mrs {
            let approvals_url =
                self.project_url(&format!("/merge_requests/{}/approvals", mr.iid));
            let approvals: GitLabApprovals = match get_json(&self.client, &approvals_url).await {
                Ok(a) => a,
                Err(_) => continue,
            };
            for approval in approvals.approved_by {
                if approval.user.username == username {
                    continue;
                }
                let Some(ts) = approval.created_at else {
                    continue;
                };
                if ts >= since {
                    reviews.push(ReviewActivity {
                        mr_iid: mr.iid,
                        mr_title: mr.title.clone(),
                        reviewer: approval.user.username,
                        state: "APPROVED".to_string(),
                        timestamp: ts,
                    });
                }
            }
        }
        Ok(reviews)
    }
}

// --- Data types ---

/// Merge status information for a merge request.
#[derive(Debug, Clone)]
pub struct MrMergeStatus {
    pub iid: u64,
    pub title: String,
    pub state: String,
    pub is_draft: bool,
    pub mergeable: Option<bool>,
    pub mergeable_state: String,
    pub ci_status: Option<String>,
    pub head_sha: String,
}

/// A note/comment on a merge request.
#[derive(Debug, Clone)]
pub struct MrNote {
    pub id: u64,
    pub body: String,
    pub author: String,
    pub created_at: DateTime<Utc>,
}

/// Summary of an open merge request for listing.
#[derive(Debug, Clone)]
pub struct MrListItem {
    pub iid: u64,
    pub title: String,
    pub url: String,
    pub author: String,
    pub source_branch: String,
    pub target_branch: String,
    pub state: String,
    pub is_draft: bool,
    pub created_at: DateTime<Utc>,
}

/// Summary of an open issue for listing.
#[derive(Debug, Clone)]
pub struct IssueListItem {
    pub iid: u64,
    pub title: String,
    pub url: String,
    pub author: String,
    pub labels: Vec<String>,
    pub updated_at: DateTime<Utc>,
}

/// Basic MR info for user queries.
#[derive(Debug, Clone)]
pub struct UserMrInfo {
    pub iid: u64,
    pub source_branch: String,
    pub target_branch: String,
    pub state: String,
    pub is_draft: bool,
}

/// MR activity for reporting.
#[derive(Debug, Clone)]
pub struct MrActivity {
    pub iid: u64,
    pub title: String,
    pub timestamp: DateTime<Utc>,
    pub url: String,
}

/// Review activity for reporting.
#[derive(Debug, Clone)]
pub struct ReviewActivity {
    pub mr_iid: u64,
    pub mr_title: String,
    pub reviewer: String,
    pub state: String,
    pub timestamp: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn remote_info(api_url: &str) -> RemoteInfo {
        RemoteInfo {
            forge: ForgeType::GitLab,
            owner: "group/subgroup".to_string(),
            repo: "repo".to_string(),
            api_base_url: api_url.to_string(),
        }
    }

    #[test]
    fn test_gitlab_client_rejects_non_gitlab_remote() {
        let remote = RemoteInfo {
            forge: ForgeType::GitHub,
            owner: "org".to_string(),
            repo: "repo".to_string(),
            api_base_url: "https://api.github.com".to_string(),
        };
        let result = GitLabClient::new(&remote);
        assert!(result.is_err());
        let err = result.err().expect("should be error");
        assert!(err.to_string().contains("expected GitLab remote"));
    }

    #[test]
    fn test_gitlab_client_with_token() {
        let remote = remote_info("https://gitlab.example.com/api/v4");
        let result = GitLabClient::with_token(&remote, "test-token");
        assert!(result.is_ok());
    }

    #[test]
    fn test_encoded_project_path() {
        let remote = remote_info("https://gitlab.example.com/api/v4");
        let encoded = encoded_project_path(&remote);
        assert_eq!(encoded, "group%2Fsubgroup%2Frepo");
    }

    #[test]
    fn test_project_url() {
        let remote = remote_info("https://gitlab.example.com/api/v4");
        let client = GitLabClient::with_token(&remote, "token").expect("client");
        let url = client.project_url("/merge_requests");
        assert!(url.contains("/projects/group%2Fsubgroup%2Frepo/merge_requests"));
    }

    #[test]
    fn test_normalize_gitlab_state() {
        assert_eq!(normalize_gitlab_state("opened"), "OPEN");
        assert_eq!(normalize_gitlab_state("closed"), "CLOSED");
        assert_eq!(normalize_gitlab_state("merged"), "MERGED");
        assert_eq!(normalize_gitlab_state("OPENED"), "OPEN");
        assert_eq!(normalize_gitlab_state("unknown"), "UNKNOWN");
    }

    #[test]
    fn test_normalize_gitlab_check_status() {
        assert_eq!(normalize_gitlab_check_status(Some("running")), "in_progress");
        assert_eq!(normalize_gitlab_check_status(Some("pending")), "in_progress");
        assert_eq!(normalize_gitlab_check_status(Some("created")), "in_progress");
        assert_eq!(normalize_gitlab_check_status(Some("success")), "completed");
        assert_eq!(normalize_gitlab_check_status(Some("failed")), "completed");
        assert_eq!(normalize_gitlab_check_status(None), "completed");
    }

    #[test]
    fn test_normalize_gitlab_conclusion() {
        assert_eq!(normalize_gitlab_conclusion("success"), "success");
        assert_eq!(normalize_gitlab_conclusion("failed"), "failure");
        assert_eq!(normalize_gitlab_conclusion("canceled"), "cancelled");
        assert_eq!(normalize_gitlab_conclusion("running"), "running");
    }

    #[test]
    fn test_gitlab_state_to_pr_state() {
        let mut mr = GitLabMr {
            iid: 1,
            title: "test".to_string(),
            state: "opened".to_string(),
            draft: Some(false),
            source_branch: "feat".to_string(),
            target_branch: "main".to_string(),
            description: None,
            merge_status: None,
            detailed_merge_status: None,
            web_url: None,
            head_pipeline: None,
            sha: None,
            author: None,
            created_at: None,
            merged_at: None,
            updated_at: None,
        };

        assert_eq!(gitlab_state_to_pr_state(&mr), PrState::Open);

        mr.draft = Some(true);
        assert_eq!(gitlab_state_to_pr_state(&mr), PrState::Draft);

        mr.state = "merged".to_string();
        mr.draft = Some(false);
        assert_eq!(gitlab_state_to_pr_state(&mr), PrState::Merged);

        mr.state = "closed".to_string();
        assert_eq!(gitlab_state_to_pr_state(&mr), PrState::Closed);
    }

    #[test]
    fn test_mr_to_pr_info() {
        let mr = GitLabMr {
            iid: 42,
            title: "Add feature".to_string(),
            state: "opened".to_string(),
            draft: Some(false),
            source_branch: "feature".to_string(),
            target_branch: "main".to_string(),
            description: Some("My description".to_string()),
            merge_status: Some("can_be_merged".to_string()),
            detailed_merge_status: None,
            web_url: Some("https://gitlab.example.com/repo/-/merge_requests/42".to_string()),
            head_pipeline: None,
            sha: None,
            author: Some(GitLabUser {
                username: "alice".to_string(),
            }),
            created_at: None,
            merged_at: None,
            updated_at: None,
        };

        let pr_info = mr_to_pr_info(&mr);
        assert_eq!(pr_info.pr_number, 42);
        assert_eq!(pr_info.title, "Add feature");
        assert_eq!(pr_info.description, "My description");
        assert_eq!(pr_info.author, "alice");
        assert_eq!(pr_info.state, PrState::Open);
        assert!(!pr_info.draft);
    }
}
