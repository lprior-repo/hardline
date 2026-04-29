//! Calc layer for stack auth - pure functions for auth resolution.
//!
//! No I/O. Deterministic logic only.

use super::data::{AuthOptions, AuthSource, AuthStatus};

pub const fn resolve_active_source(status: &AuthStatus) -> Option<AuthSource> {
    if status.stax_env_available {
        return Some(AuthSource::StaxGithubTokenEnv);
    }
    if status.credentials_file_available {
        return Some(AuthSource::CredentialsFile);
    }
    if status.use_gh_cli && status.gh_cli_available {
        return Some(AuthSource::GhCli);
    }
    if status.allow_github_token_env && status.github_env_available {
        return Some(AuthSource::GithubTokenEnv);
    }
    None
}

pub fn determine_auth_resolution_order(status: &AuthStatus) -> Vec<(AuthSource, bool, bool)> {
    vec![
        (
            AuthSource::StaxGithubTokenEnv,
            status.stax_env_available,
            true,
        ),
        (
            AuthSource::CredentialsFile,
            status.credentials_file_available,
            true,
        ),
        (
            AuthSource::GhCli,
            status.gh_cli_available,
            status.use_gh_cli,
        ),
        (
            AuthSource::GithubTokenEnv,
            status.github_env_available,
            status.allow_github_token_env,
        ),
    ]
}

pub fn validate_token(token: &str) -> bool {
    !token.trim().is_empty() && token.len() >= 8
}

pub fn normalize_token(token: &str) -> String {
    token.trim().to_string()
}

pub const fn should_use_gh_cli(options: &AuthOptions) -> bool {
    options.use_gh_cli && options.token.is_none() && !options.from_gh
}

pub const fn token_source_description(source: AuthSource) -> &'static str {
    match source {
        AuthSource::StaxGithubTokenEnv => "Environment variable STAX_GITHUB_TOKEN",
        AuthSource::CredentialsFile => "Stored credentials file",
        AuthSource::GhCli => "GitHub CLI (`gh auth token`)",
        AuthSource::GithubTokenEnv => "Environment variable GITHUB_TOKEN",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_active_source_priority() {
        let mut status = AuthStatus::default();

        status.stax_env_available = true;
        assert_eq!(
            resolve_active_source(&status),
            Some(AuthSource::StaxGithubTokenEnv)
        );
        status.stax_env_available = false;

        status.credentials_file_available = true;
        assert_eq!(
            resolve_active_source(&status),
            Some(AuthSource::CredentialsFile)
        );
        status.credentials_file_available = false;

        status.use_gh_cli = true;
        status.gh_cli_available = true;
        assert_eq!(resolve_active_source(&status), Some(AuthSource::GhCli));
        status.use_gh_cli = false;

        status.allow_github_token_env = true;
        status.github_env_available = true;
        assert_eq!(
            resolve_active_source(&status),
            Some(AuthSource::GithubTokenEnv)
        );
    }

    #[test]
    fn test_resolve_active_source_none_when_no_sources() {
        let status = AuthStatus::default();
        assert_eq!(resolve_active_source(&status), None);
    }

    #[test]
    fn test_validate_token() {
        assert!(!validate_token(""));
        assert!(!validate_token("   "));
        assert!(!validate_token("short"));
        assert!(validate_token("abcdefgh"));
        assert!(validate_token(" valid_token_123 "));
    }

    #[test]
    fn test_normalize_token() {
        assert_eq!(normalize_token("  token  "), "token");
        assert_eq!(normalize_token("token\n"), "token");
        assert_eq!(normalize_token("no_whitespace"), "no_whitespace");
    }

    #[test]
    fn test_should_use_gh_cli() {
        let mut options = AuthOptions::default();

        options.use_gh_cli = false;
        assert!(!should_use_gh_cli(&options));

        options.use_gh_cli = true;
        options.token = Some("token".to_string());
        assert!(!should_use_gh_cli(&options));

        options.use_gh_cli = true;
        options.token = None;
        options.from_gh = false;
        assert!(should_use_gh_cli(&options));

        options.from_gh = true;
        assert!(!should_use_gh_cli(&options));
    }

    #[test]
    fn test_determine_auth_resolution_order() {
        let mut status = AuthStatus::default();
        status.use_gh_cli = true;
        status.allow_github_token_env = true;

        let order = determine_auth_resolution_order(&status);
        assert_eq!(order.len(), 4);
        assert_eq!(order[0].0, AuthSource::StaxGithubTokenEnv);
        assert_eq!(order[1].0, AuthSource::CredentialsFile);
        assert_eq!(order[2].0, AuthSource::GhCli);
        assert_eq!(order[3].0, AuthSource::GithubTokenEnv);
    }
}
