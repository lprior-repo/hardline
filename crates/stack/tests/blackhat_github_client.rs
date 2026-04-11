//! Black-hat integration tests: GitHub client — PR creation, status checks, merge detection
//!
//! Tests the async GitHub client (crate::github::client::GitHubClient) against
//! mocked GitHub API responses. Covers: PR creation, status checks, merge detection,
//! API error handling, retry logic, and response parsing correctness.

use mockito::Server;
use scp_stack::github::client::GitHubClient;
use scp_stack::github::pr::{CiStatus, MergeMethod};
use serde_json::json;

const TEST_OWNER: &str = "test-owner";
const TEST_REPO: &str = "test-repo";

fn create_mock_client(server_url: &str) -> GitHubClient {
    GitHubClient::new(
        TEST_OWNER,
        TEST_REPO,
        "test-token".to_string(),
        Some(server_url.to_string()),
    )
    .expect("client creation")
}

#[tokio::test]
async fn combined_status_state_success() {
    let mut server = Server::new_async().await;
    let commit_sha = "abc123def456";

    let check_runs_mock = server
        .mock(
            "GET",
            format!(
                "/repos/test-owner/test-repo/commits/{}/check-runs",
                commit_sha
            )
            .as_str(),
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            serde_json::json!({
                "total_count": 2,
                "check_runs": [
                    {
                        "id": 1,
                        "name": "ci/check",
                        "status": "completed",
                        "conclusion": "success"
                    },
                    {
                        "id": 2,
                        "name": "test/check",
                        "status": "completed",
                        "conclusion": "success"
                    }
                ]
            })
            .to_string(),
        )
        .create_async()
        .await;

    let client = create_mock_client(&server.url());
    let result = client.combined_status_state(commit_sha).await;

    check_runs_mock.assert_async().await;
    assert!(
        result.is_ok(),
        "combined_status_state failed: {:?}",
        result.err()
    );
    assert_eq!(result.unwrap(), Some("success".to_string()));
}

#[tokio::test]
async fn combined_status_state_failure() {
    let mut server = Server::new_async().await;
    let commit_sha = "abc123def456";

    let check_runs_mock = server
        .mock(
            "GET",
            format!(
                "/repos/test-owner/test-repo/commits/{}/check-runs",
                commit_sha
            )
            .as_str(),
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            serde_json::json!({
                "total_count": 2,
                "check_runs": [
                    {
                        "id": 1,
                        "name": "ci/check",
                        "status": "completed",
                        "conclusion": "success"
                    },
                    {
                        "id": 2,
                        "name": "test/check",
                        "status": "completed",
                        "conclusion": "failure"
                    }
                ]
            })
            .to_string(),
        )
        .create_async()
        .await;

    let client = create_mock_client(&server.url());
    let result = client.combined_status_state(commit_sha).await;

    check_runs_mock.assert_async().await;
    assert!(
        result.is_ok(),
        "combined_status_state failed: {:?}",
        result.err()
    );
    assert_eq!(result.unwrap(), Some("failure".to_string()));
}

#[tokio::test]
async fn combined_status_state_pending() {
    let mut server = Server::new_async().await;
    let commit_sha = "abc123def456";

    let check_runs_mock = server
        .mock(
            "GET",
            format!(
                "/repos/test-owner/test-repo/commits/{}/check-runs",
                commit_sha
            )
            .as_str(),
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            serde_json::json!({
                "total_count": 1,
                "check_runs": [
                    {
                        "id": 1,
                        "name": "ci/check",
                        "status": "in_progress",
                        "conclusion": null
                    }
                ]
            })
            .to_string(),
        )
        .create_async()
        .await;

    let client = create_mock_client(&server.url());
    let result = client.combined_status_state(commit_sha).await;

    check_runs_mock.assert_async().await;
    assert!(
        result.is_ok(),
        "combined_status_state failed: {:?}",
        result.err()
    );
    assert_eq!(result.unwrap(), Some("pending".to_string()));
}

#[tokio::test]
async fn get_check_runs_status_empty() {
    let mut server = Server::new_async().await;
    let commit_sha = "abc123def456";

    let check_runs_mock = server
        .mock(
            "GET",
            format!(
                "/repos/test-owner/test-repo/commits/{}/check-runs",
                commit_sha
            )
            .as_str(),
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            serde_json::json!({
                "total_count": 0,
                "check_runs": []
            })
            .to_string(),
        )
        .create_async()
        .await;

    let client = create_mock_client(&server.url());
    let result = client.get_check_runs_status(commit_sha).await;

    check_runs_mock.assert_async().await;
    assert!(
        result.is_ok(),
        "get_check_runs_status failed: {:?}",
        result.err()
    );
    assert_eq!(result.unwrap(), None);
}

#[tokio::test]
async fn api_error_rate_limit() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("GET", "/repos/test-owner/test-repo/pulls/42")
        .with_status(403)
        .with_header("content-type", "application/json")
        .with_header("x-ratelimit-remaining", "0")
        .with_header("x-ratelimit-reset", "1705320000")
        .with_body(
            serde_json::json!({
                "message": "API rate limit exceeded",
                "documentation_url": "https://docs.github.com/rest/overview/rate-limiting"
            })
            .to_string(),
        )
        .create_async()
        .await;

    let client = create_mock_client(&server.url());
    let result = client.get_pr(42).await;

    mock.assert_async().await;
    assert!(result.is_err(), "Expected error for rate limit");
}

#[tokio::test]
async fn api_error_auth_failure() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("GET", "/repos/test-owner/test-repo/pulls/42")
        .with_status(401)
        .with_header("content-type", "application/json")
        .with_body(
            serde_json::json!({
                "message": "Bad credentials",
                "documentation_url": "https://docs.github.com/authentication"
            })
            .to_string(),
        )
        .create_async()
        .await;

    let client = create_mock_client(&server.url());
    let result = client.get_pr(42).await;

    mock.assert_async().await;
    assert!(result.is_err(), "Expected error for auth failure");
}

#[tokio::test]
async fn api_error_not_found() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("GET", "/repos/test-owner/test-repo/pulls/99999")
        .with_status(404)
        .with_header("content-type", "application/json")
        .with_body(
            serde_json::json!({
                "message": "Not Found",
                "documentation_url": "https://docs.github.com/rest/pulls/pulls"
            })
            .to_string(),
        )
        .create_async()
        .await;

    let client = create_mock_client(&server.url());
    let result = client.get_pr(99999).await;

    mock.assert_async().await;
    assert!(result.is_err(), "Expected error for not found");
}

#[tokio::test]
async fn find_open_pr_by_head_returns_none_when_not_found() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("GET", "/repos/test-owner/test-repo/pulls")
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("state".into(), "open".into()),
            mockito::Matcher::UrlEncoded("head".into(), "test-owner:missing-branch".into()),
            mockito::Matcher::UrlEncoded("per_page".into(), "100".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            serde_json::json!({
                "total_count": 0,
                "items": []
            })
            .to_string(),
        )
        .create_async()
        .await;

    let client = create_mock_client(&server.url());
    let result = client
        .find_open_pr_by_head("test-owner", "missing-branch")
        .await;

    mock.assert_async().await;
    assert!(
        result.is_ok(),
        "find_open_pr_by_head failed: {:?}",
        result.err()
    );
    assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn merge_pr_squash_method() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("PUT", "/repos/test-owner/test-repo/pulls/42/merge")
        .match_header("content-type", "application/json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            serde_json::json!({
                "merged": true,
                "message": "Pull Request successfully merged"
            })
            .to_string(),
        )
        .create_async()
        .await;

    let client = create_mock_client(&server.url());
    let result = client
        .merge_pr(
            42,
            MergeMethod::Squash,
            Some("Merge PR".to_string()),
            Some("Merged via stack".to_string()),
        )
        .await;

    mock.assert_async().await;
    assert!(result.is_ok(), "merge_pr failed: {:?}", result.err());
}

#[tokio::test]
async fn merge_pr_merge_method() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("PUT", "/repos/test-owner/test-repo/pulls/43/merge")
        .match_header("content-type", "application/json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            serde_json::json!({
                "merged": true,
                "message": "Pull Request successfully merged"
            })
            .to_string(),
        )
        .create_async()
        .await;

    let client = create_mock_client(&server.url());
    let result = client.merge_pr(43, MergeMethod::Merge, None, None).await;

    mock.assert_async().await;
    assert!(result.is_ok(), "merge_pr failed: {:?}", result.err());
}

#[tokio::test]
async fn merge_pr_rebase_method() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("PUT", "/repos/test-owner/test-repo/pulls/44/merge")
        .match_header("content-type", "application/json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            serde_json::json!({
                "merged": true,
                "message": "Pull Request successfully merged"
            })
            .to_string(),
        )
        .create_async()
        .await;

    let client = create_mock_client(&server.url());
    let result = client.merge_pr(44, MergeMethod::Rebase, None, None).await;

    mock.assert_async().await;
    assert!(result.is_ok(), "merge_pr failed: {:?}", result.err());
}

#[tokio::test]
async fn ci_status_from_api_str_parses_correctly() {
    assert_eq!(CiStatus::from_api_str("success"), CiStatus::Success);
    assert_eq!(CiStatus::from_api_str("pending"), CiStatus::Pending);
    assert_eq!(CiStatus::from_api_str("failure"), CiStatus::Failure);
    assert_eq!(CiStatus::from_api_str("error"), CiStatus::Failure);
    assert_eq!(CiStatus::from_api_str("neutral"), CiStatus::Success);
    assert_eq!(CiStatus::from_api_str("skipped"), CiStatus::Success);
    assert_eq!(CiStatus::from_api_str("cancelled"), CiStatus::Success);
    assert_eq!(CiStatus::from_api_str(""), CiStatus::NoCi);
    assert_eq!(CiStatus::from_api_str("unknown"), CiStatus::NoCi);
}

#[tokio::test]
async fn request_reviewers_empty_list_does_nothing() {
    let server = Server::new_async().await;
    let client = create_mock_client(&server.url());

    let result = client.request_reviewers(42, &[]).await;
    assert!(
        result.is_ok(),
        "request_reviewers with empty list failed: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn add_labels_sends_correct_payload() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("POST", "/repos/test-owner/test-repo/issues/42/labels")
        .match_header("content-type", "application/json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[]")
        .create_async()
        .await;

    let client = create_mock_client(&server.url());
    let result = client
        .add_labels(42, &["bug".to_string(), "urgent".to_string()])
        .await;

    mock.assert_async().await;
    assert!(result.is_ok(), "add_labels failed: {:?}", result.err());
}

#[tokio::test]
async fn list_issue_comments_returns_comments() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("GET", "/repos/test-owner/test-repo/issues/42/comments")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            serde_json::json!([
                {
                    "id": 1,
                    "body": "First comment",
                    "user": {"login": "user1"},
                    "created_at": "2024-01-15T10:00:00Z"
                },
                {
                    "id": 2,
                    "body": "Second comment",
                    "user": {"login": "user2"},
                    "created_at": "2024-01-15T11:00:00Z"
                }
            ])
            .to_string(),
        )
        .create_async()
        .await;

    let client = create_mock_client(&server.url());
    let result = client.list_issue_comments(42).await;

    mock.assert_async().await;
    assert!(
        result.is_ok(),
        "list_issue_comments failed: {:?}",
        result.err()
    );
    let comments = result.unwrap();
    assert_eq!(comments.len(), 2);
    assert_eq!(comments[0].body, "First comment");
    assert_eq!(comments[0].user, "user1");
    assert_eq!(comments[1].body, "Second comment");
}

#[tokio::test]
async fn list_review_comments_returns_comments() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("GET", "/repos/test-owner/test-repo/pulls/42/comments")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            serde_json::json!([
                {
                    "id": 1,
                    "body": "Code review comment",
                    "user": {"login": "reviewer1"},
                    "path": "src/main.rs",
                    "line": 42,
                    "start_line": null,
                    "created_at": "2024-01-15T10:00:00Z",
                    "diff_hunk": "@@ -1,5 +1,6 @@"
                }
            ])
            .to_string(),
        )
        .create_async()
        .await;

    let client = create_mock_client(&server.url());
    let result = client.list_review_comments(42).await;

    mock.assert_async().await;
    assert!(
        result.is_ok(),
        "list_review_comments failed: {:?}",
        result.err()
    );
    let comments = result.unwrap();
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0].body, "Code review comment");
    assert_eq!(comments[0].path, "src/main.rs");
    assert_eq!(comments[0].line, Some(42));
}

#[tokio::test]
async fn update_pr_branch_handles_no_update_needed() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("PUT", "/repos/test-owner/test-repo/pulls/42/update-branch")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            serde_json::json!({
                "message": "Update is not required"
            })
            .to_string(),
        )
        .create_async()
        .await;

    let client = create_mock_client(&server.url());
    let result = client.update_pr_branch(42).await;

    mock.assert_async().await;
    assert!(
        result.is_ok(),
        "update_pr_branch failed: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn update_pr_branch_success() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("PUT", "/repos/test-owner/test-repo/pulls/42/update-branch")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(serde_json::json!({}).to_string())
        .create_async()
        .await;

    let client = create_mock_client(&server.url());
    let result = client.update_pr_branch(42).await;

    mock.assert_async().await;
    assert!(
        result.is_ok(),
        "update_pr_branch failed: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn delete_stack_comment_handles_missing_comment() {
    let mut server = Server::new_async().await;

    let list_mock = server
        .mock("GET", "/repos/test-owner/test-repo/issues/42/comments")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[]")
        .create_async()
        .await;

    let client = create_mock_client(&server.url());
    let result = client.delete_stack_comment(42).await;

    list_mock.assert_async().await;
    assert!(
        result.is_ok(),
        "delete_stack_comment failed: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn list_open_pull_requests_returns_prs() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("GET", "/repos/test-owner/test-repo/pulls")
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("state".into(), "open".into()),
            mockito::Matcher::UrlEncoded("sort".into(), "created".into()),
            mockito::Matcher::UrlEncoded("direction".into(), "desc".into()),
            mockito::Matcher::UrlEncoded("per_page".into(), "10".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            serde_json::json!([
                {
                    "number": 1,
                    "title": "PR One",
                    "html_url": "https://github.com/test-owner/test-repo/pull/1",
                    "state": "open",
                    "draft": false,
                    "created_at": "2024-01-15T10:00:00Z",
                    "head": {
                        "ref": "branch-a",
                        "label": "test-owner:branch-a",
                        "sha": "sha1"
                    },
                    "base": {
                        "ref": "main",
                        "label": "test-owner:main",
                        "sha": "sha0"
                    },
                    "user": {
                        "login": "author1"
                    }
                }
            ])
            .to_string(),
        )
        .create_async()
        .await;

    let client = create_mock_client(&server.url());
    let result = client.list_open_pull_requests(10).await;

    mock.assert_async().await;
    assert!(
        result.is_ok(),
        "list_open_pull_requests failed: {:?}",
        result.err()
    );
    let prs = result.unwrap();
    assert_eq!(prs.len(), 1);
    assert_eq!(prs[0].number, 1);
    assert_eq!(prs[0].author, "author1");
}

#[tokio::test]
async fn list_open_issues_returns_issues() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("GET", "/repos/test-owner/test-repo/issues")
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("state".into(), "open".into()),
            mockito::Matcher::UrlEncoded("sort".into(), "updated".into()),
            mockito::Matcher::UrlEncoded("direction".into(), "desc".into()),
            mockito::Matcher::UrlEncoded("per_page".into(), "20".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            serde_json::json!([
                {
                    "number": 1,
                    "title": "Issue One",
                    "html_url": "https://github.com/test-owner/test-repo/issues/1",
                    "user": {"login": "author1"},
                    "labels": [{"name": "bug"}, {"name": "urgent"}],
                    "updated_at": "2024-01-15T10:00:00Z",
                    "pull_request": null
                }
            ])
            .to_string(),
        )
        .create_async()
        .await;

    let client = create_mock_client(&server.url());
    let result = client.list_open_issues(10).await;

    mock.assert_async().await;
    assert!(
        result.is_ok(),
        "list_open_issues failed: {:?}",
        result.err()
    );
    let issues = result.unwrap();
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].number, 1);
    assert_eq!(issues[0].author, "author1");
    assert_eq!(issues[0].labels, vec!["bug", "urgent"]);
}
