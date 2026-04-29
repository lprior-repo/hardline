//! Data layer for stack auth - inert types.
//!
//! No business logic. Types only.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForgeType {
    GitHub,
    GitLab,
    Gitea,
}

impl ForgeType {
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::GitHub => "GitHub",
            Self::GitLab => "GitLab",
            Self::Gitea => "Gitea",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthSource {
    StaxGithubTokenEnv,
    CredentialsFile,
    GhCli,
    GithubTokenEnv,
}

impl AuthSource {
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::StaxGithubTokenEnv => "STAX_GITHUB_TOKEN",
            Self::CredentialsFile => "credentials file (~/.config/stax/.credentials)",
            Self::GhCli => "gh auth token",
            Self::GithubTokenEnv => "GITHUB_TOKEN",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AuthStatus {
    pub active_source: Option<AuthSource>,
    pub stax_env_available: bool,
    pub credentials_file_available: bool,
    pub gh_cli_available: bool,
    pub github_env_available: bool,
    pub use_gh_cli: bool,
    pub allow_github_token_env: bool,
    pub gh_hostname: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AuthOptions {
    pub token: Option<String>,
    pub from_gh: bool,
    pub use_gh_cli: bool,
    pub allow_github_token_env: bool,
    pub gh_hostname: Option<String>,
}

impl Default for AuthOptions {
    fn default() -> Self {
        Self {
            token: None,
            from_gh: false,
            use_gh_cli: true,
            allow_github_token_env: false,
            gh_hostname: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuthResult {
    pub source: AuthSource,
    pub credentials_path: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Token cannot be empty")]
    EmptyToken,
    #[error("Failed to read token from gh CLI: {0}")]
    GhCliError(String),
    #[error("Failed to write credentials file: {0}")]
    WriteError(String),
    #[error("Failed to set permissions on credentials file: {0}")]
    PermissionsError(String),
    #[error("IO error: {0}")]
    IoError(String),
    #[error("GH CLI not available or not authenticated")]
    GhNotAvailable,
    #[error("No auth source available. Run `scp auth`, `scp auth --from-gh`, or `gh auth login`.")]
    NoAuthSource,
}

impl From<std::io::Error> for AuthError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forge_type_display() {
        assert_eq!(ForgeType::GitHub.display_name(), "GitHub");
        assert_eq!(ForgeType::GitLab.display_name(), "GitLab");
        assert_eq!(ForgeType::Gitea.display_name(), "Gitea");
    }

    #[test]
    fn auth_source_display() {
        assert_eq!(
            AuthSource::StaxGithubTokenEnv.display_name(),
            "STAX_GITHUB_TOKEN"
        );
        assert_eq!(
            AuthSource::CredentialsFile.display_name(),
            "credentials file (~/.config/stax/.credentials)"
        );
        assert_eq!(AuthSource::GhCli.display_name(), "gh auth token");
        assert_eq!(AuthSource::GithubTokenEnv.display_name(), "GITHUB_TOKEN");
    }

    #[test]
    fn auth_status_default() {
        let status = AuthStatus::default();
        assert!(status.active_source.is_none());
        assert!(!status.use_gh_cli);
    }

    #[test]
    fn auth_options_default() {
        let options = AuthOptions::default();
        assert!(options.token.is_none());
        assert!(!options.from_gh);
        assert!(options.use_gh_cli);
    }

    #[test]
    fn auth_error_display() {
        let err = AuthError::EmptyToken;
        assert!(err.to_string().contains("empty"));

        let err = AuthError::NoAuthSource;
        assert!(err.to_string().contains("scp auth"));
    }

    #[test]
    fn auth_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: AuthError = io_err.into();
        assert!(err.to_string().contains("file not found"));
    }
}
