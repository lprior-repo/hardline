pub mod gitea;
pub mod gitlab;

use std::fmt;
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::{Result, StackError};

/// Supported forge (code-hosting platform) types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ForgeType {
    GitHub,
    GitLab,
    Gitea,
}

impl fmt::Display for ForgeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GitHub => write!(f, "GitHub"),
            Self::GitLab => write!(f, "GitLab"),
            Self::Gitea => write!(f, "Gitea"),
        }
    }
}

/// Authentication header style used by different forges.
#[derive(Clone, Copy)]
pub enum AuthStyle {
    /// `Authorization: token <value>` (GitHub, Gitea).
    AuthorizationToken,
    /// `PRIVATE-TOKEN: <value>` (GitLab).
    PrivateToken,
}

/// Remote repository information resolved from a git remote URL.
#[derive(Debug, Clone)]
pub struct RemoteInfo {
    pub forge: ForgeType,
    pub owner: String,
    pub repo: String,
    pub api_base_url: String,
}

/// Merge strategy for pull/merge requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeMethod {
    Merge,
    Squash,
    Rebase,
}

impl MergeMethod {
    /// Returns the Gitea/API string representation.
    pub fn as_gitea_str(self) -> &'static str {
        match self {
            Self::Merge => "merge",
            Self::Squash => "squash",
            Self::Rebase => "rebase",
        }
    }
}

impl fmt::Display for MergeMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Merge => write!(f, "merge"),
            Self::Squash => write!(f, "squash"),
            Self::Rebase => write!(f, "rebase"),
        }
    }
}

/// CI check information for a commit.
#[derive(Debug, Clone)]
pub struct CheckRunInfo {
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub url: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

/// HTML comment marker embedded in stack comments for identification.
pub const STACK_COMMENT_MARKER: &str = "<!-- stax-stack-comment -->";

/// Wrap a stack comment with the identification marker.
pub fn stack_comment_body(stack_comment: &str) -> String {
    format!("{STACK_COMMENT_MARKER}\n{stack_comment}")
}

/// Build standard HTTP headers for forge API calls.
fn base_headers(token: &str, auth_style: AuthStyle) -> Result<reqwest::header::HeaderMap> {
    use reqwest::header::{HeaderValue, ACCEPT, AUTHORIZATION, USER_AGENT};

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("stax"));
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

    match auth_style {
        AuthStyle::AuthorizationToken => {
            let value = HeaderValue::from_str(&format!("token {token}"))
                .map_err(|e| StackError::ForgeError(format!("Invalid auth header: {e}")))?;
            headers.insert(AUTHORIZATION, value);
        }
        AuthStyle::PrivateToken => {
            let value = HeaderValue::from_str(token).map_err(|e| {
                StackError::ForgeError(format!("Invalid private token header: {e}"))
            })?;
            headers.insert("PRIVATE-TOKEN", value);
        }
    }

    Ok(headers)
}

/// Build an HTTP client with standard forge headers and timeouts.
pub fn build_http_client(token: &str, auth_style: AuthStyle) -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .default_headers(base_headers(token, auth_style)?)
        .connect_timeout(Duration::from_secs(10))
        .read_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| StackError::ForgeError(format!("Failed to build HTTP client: {e}")))
}

/// Send a GET request and parse the JSON response.
pub async fn get_json<T: DeserializeOwned>(client: &reqwest::Client, url: &str) -> Result<T> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| StackError::ForgeError(format!("GET {url}: {e}")))?;
    parse_json_response(response).await
}

/// Send a POST request with a JSON body and parse the response.
pub async fn post_json<T: DeserializeOwned, B: Serialize>(
    client: &reqwest::Client,
    url: &str,
    body: &B,
) -> Result<T> {
    let response = client
        .post(url)
        .json(body)
        .send()
        .await
        .map_err(|e| StackError::ForgeError(format!("POST {url}: {e}")))?;
    parse_json_response(response).await
}

/// Send a PUT request with a JSON body and parse the response.
pub async fn put_json<T: DeserializeOwned, B: Serialize>(
    client: &reqwest::Client,
    url: &str,
    body: &B,
) -> Result<T> {
    let response = client
        .put(url)
        .json(body)
        .send()
        .await
        .map_err(|e| StackError::ForgeError(format!("PUT {url}: {e}")))?;
    parse_json_response(response).await
}

/// Send a PATCH request with a JSON body and parse the response.
pub async fn patch_json<T: DeserializeOwned, B: Serialize>(
    client: &reqwest::Client,
    url: &str,
    body: &B,
) -> Result<T> {
    let response = client
        .patch(url)
        .json(body)
        .send()
        .await
        .map_err(|e| StackError::ForgeError(format!("PATCH {url}: {e}")))?;
    parse_json_response(response).await
}

/// Send a DELETE request; succeeds on 2xx or 404.
pub async fn delete_empty(client: &reqwest::Client, url: &str) -> Result<()> {
    let response = client
        .delete(url)
        .send()
        .await
        .map_err(|e| StackError::ForgeError(format!("DELETE {url}: {e}")))?;

    if response.status().is_success() || response.status().as_u16() == 404 {
        return Ok(());
    }

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    Err(StackError::ForgeError(format!(
        "DELETE {url}: {status} {body}"
    )))
}

/// Parse a JSON response, returning an error on non-2xx status codes.
async fn parse_json_response<T: DeserializeOwned>(response: reqwest::Response) -> Result<T> {
    if response.status().is_success() {
        response
            .json::<T>()
            .await
            .map_err(|e| StackError::ForgeError(format!("JSON parse error: {e}")))
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(StackError::ForgeError(format!(
            "Forge API error: {status} {body}"
        )))
    }
}

/// Aggregate individual CI statuses into one overall result.
/// Failure always takes priority over pending.
pub fn aggregate_ci_overall<'a>(
    statuses: impl Iterator<Item = &'a str>,
    is_failure: impl Fn(&str) -> bool,
    is_pending: impl Fn(&str) -> bool,
) -> Option<String> {
    let mut has_any = false;
    let mut has_failure = false;
    let mut has_pending = false;

    for status in statuses {
        has_any = true;
        if is_failure(status) {
            has_failure = true;
        } else if is_pending(status) {
            has_pending = true;
        }
    }

    if has_failure {
        Some("failure".to_string())
    } else if has_pending {
        Some("pending".to_string())
    } else if has_any {
        Some("success".to_string())
    } else {
        None
    }
}

/// Convert a mergeable state string to an optional boolean.
pub fn mergeable_bool(mergeable_state: &str) -> Option<bool> {
    match mergeable_state {
        "checking" | "unchecked" | "preparing" | "unknown" => None,
        "mergeable" | "can_be_merged" | "clean" => Some(true),
        _ => Some(false),
    }
}

/// Resolve a forge token from environment variables.
pub fn forge_token(forge: ForgeType) -> Option<String> {
    match forge {
        ForgeType::GitHub => {
            read_env_token("STAX_GITHUB_TOKEN").or_else(|| read_env_token("GITHUB_TOKEN"))
        }
        ForgeType::GitLab => read_env_token("STAX_GITLAB_TOKEN")
            .or_else(|| read_env_token("GITLAB_TOKEN"))
            .or_else(|| read_env_token("STAX_FORGE_TOKEN")),
        ForgeType::Gitea => read_env_token("STAX_GITEA_TOKEN")
            .or_else(|| read_env_token("GITEA_TOKEN"))
            .or_else(|| read_env_token("STAX_FORGE_TOKEN")),
    }
}

fn read_env_token(var_name: &str) -> Option<String> {
    std::env::var(var_name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Percent-encode a value for use in a URL query parameter (RFC 3986).
pub fn encode_query_value(value: &str) -> String {
    use std::fmt::Write;
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => {
                let _ = write!(encoded, "%{byte:02X}");
            }
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forge_type_display() {
        assert_eq!(ForgeType::GitHub.to_string(), "GitHub");
        assert_eq!(ForgeType::GitLab.to_string(), "GitLab");
        assert_eq!(ForgeType::Gitea.to_string(), "Gitea");
    }

    #[test]
    fn test_forge_type_equality() {
        assert_eq!(ForgeType::GitHub, ForgeType::GitHub);
        assert_ne!(ForgeType::GitHub, ForgeType::GitLab);
        assert_ne!(ForgeType::GitLab, ForgeType::Gitea);
    }

    #[test]
    fn test_merge_method_display() {
        assert_eq!(MergeMethod::Merge.to_string(), "merge");
        assert_eq!(MergeMethod::Squash.to_string(), "squash");
        assert_eq!(MergeMethod::Rebase.to_string(), "rebase");
    }

    #[test]
    fn test_merge_method_gitea_str() {
        assert_eq!(MergeMethod::Merge.as_gitea_str(), "merge");
        assert_eq!(MergeMethod::Squash.as_gitea_str(), "squash");
        assert_eq!(MergeMethod::Rebase.as_gitea_str(), "rebase");
    }

    #[test]
    fn test_merge_method_equality() {
        assert_eq!(MergeMethod::Merge, MergeMethod::Merge);
        assert_ne!(MergeMethod::Merge, MergeMethod::Squash);
    }

    #[test]
    fn test_stack_comment_body() {
        let body = stack_comment_body("hello world");
        assert!(body.contains(STACK_COMMENT_MARKER));
        assert!(body.contains("hello world"));
        assert!(body.starts_with(STACK_COMMENT_MARKER));
    }

    #[test]
    fn test_aggregate_ci_failure_priority() {
        let statuses = ["pending", "failed"];
        let result = aggregate_ci_overall(
            statuses.iter().copied(),
            |s| matches!(s, "failed" | "error"),
            |s| matches!(s, "pending" | "running"),
        );
        assert_eq!(result.as_deref(), Some("failure"));
    }

    #[test]
    fn test_aggregate_ci_all_success() {
        let statuses = ["success", "success"];
        let result = aggregate_ci_overall(
            statuses.iter().copied(),
            |s| matches!(s, "failed" | "error"),
            |s| matches!(s, "pending" | "running"),
        );
        assert_eq!(result.as_deref(), Some("success"));
    }

    #[test]
    fn test_aggregate_ci_pending_only() {
        let statuses = ["success", "running"];
        let result = aggregate_ci_overall(
            statuses.iter().copied(),
            |s| matches!(s, "failed" | "error"),
            |s| matches!(s, "pending" | "running"),
        );
        assert_eq!(result.as_deref(), Some("pending"));
    }

    #[test]
    fn test_aggregate_ci_empty() {
        let statuses: [&str; 0] = [];
        let result = aggregate_ci_overall(
            statuses.iter().copied(),
            |s| matches!(s, "failed"),
            |s| matches!(s, "pending"),
        );
        assert_eq!(result, None);
    }

    #[test]
    fn test_mergeable_bool() {
        assert_eq!(mergeable_bool("mergeable"), Some(true));
        assert_eq!(mergeable_bool("can_be_merged"), Some(true));
        assert_eq!(mergeable_bool("clean"), Some(true));
        assert_eq!(mergeable_bool("checking"), None);
        assert_eq!(mergeable_bool("unchecked"), None);
        assert_eq!(mergeable_bool("unknown"), None);
        assert_eq!(mergeable_bool("dirty"), Some(false));
        assert_eq!(mergeable_bool("conflicting"), Some(false));
    }

    #[test]
    fn test_encode_query_value_ascii() {
        assert_eq!(encode_query_value("hello"), "hello");
        assert_eq!(encode_query_value("a-b_c.d~z"), "a-b_c.d~z");
    }

    #[test]
    fn test_encode_query_value_special_chars() {
        assert_eq!(encode_query_value("hello world"), "hello%20world");
        assert_eq!(encode_query_value("a/b"), "a%2Fb");
        assert_eq!(encode_query_value("foo@bar"), "foo%40bar");
    }

    #[test]
    fn test_encode_query_value_group_subgroup() {
        let encoded = encode_query_value("group/subgroup/repo");
        assert_eq!(encoded, "group%2Fsubgroup%2Frepo");
    }

    #[test]
    fn test_encode_query_value_empty() {
        assert_eq!(encode_query_value(""), "");
    }

    #[test]
    fn test_forge_token_env_not_set() {
        // This test just verifies the function doesn't panic when env vars aren't set
        let _ = forge_token(ForgeType::GitHub);
        let _ = forge_token(ForgeType::GitLab);
        let _ = forge_token(ForgeType::Gitea);
    }

    #[test]
    fn test_remote_info_fields() {
        let info = RemoteInfo {
            forge: ForgeType::Gitea,
            owner: "org".to_string(),
            repo: "repo".to_string(),
            api_base_url: "https://gitea.example.com/api/v1".to_string(),
        };
        assert_eq!(info.forge, ForgeType::Gitea);
        assert_eq!(info.owner, "org");
        assert_eq!(info.repo, "repo");
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::proptest;

    proptest! {
        #[test]
        fn prop_encode_query_value_roundtrip(s in "[a-zA-Z0-9\\-_.~]{0,50}") {
            // Unreserved characters should not be encoded
            assert_eq!(encode_query_value(&s), s);
        }

        #[test]
        fn prop_mergeable_bool_never_panics(s in ".*") {
            let _ = mergeable_bool(&s);
        }

        #[test]
        fn prop_aggregate_ci_overall_never_panics(statuses in proptest::collection::vec("(success|failure|pending|running)", 0..10)) {
            let _ = aggregate_ci_overall(
                statuses.iter().map(|s| s.as_str()),
                |s| s == "failure",
                |s| s == "pending" || s == "running",
            );
        }
    }
}
