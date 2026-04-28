//! Actions layer for stack auth - I/O operations.
//!
//! Handles token storage, retrieval, and gh CLI integration.

use std::{fs, path::PathBuf, process::Command};

use scp_core::output::Output;

use crate::commands::handlers::stack_auth::{
    calc::{determine_auth_resolution_order, normalize_token, resolve_active_source},
    data::{AuthError, AuthOptions, AuthResult, AuthSource, AuthStatus},
};

const CREDENTIALS_FILENAME: &str = ".credentials";
const APP_NAME: &str = "stax";

fn config_dir() -> Result<PathBuf, AuthError> {
    directories::ProjectDirs::from("com", "scp", APP_NAME)
        .map(|dirs| dirs.config_dir().to_path_buf())
        .ok_or_else(|| AuthError::IoError("Could not determine config directory".to_string()))
}

fn credentials_path() -> Result<PathBuf, AuthError> {
    Ok(config_dir()?.join(CREDENTIALS_FILENAME))
}

fn read_env_token(var_name: &str) -> Option<String> {
    std::env::var(var_name).ok().and_then(|v| {
        let normalized = normalize_token(&v);
        if normalized.is_empty() {
            None
        } else {
            Some(normalized)
        }
    })
}

fn token_from_credentials_file() -> Result<Option<String>, AuthError> {
    let path = credentials_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&path)?;
    let normalized = normalize_token(&content);
    if normalized.is_empty() {
        Ok(None)
    } else {
        Ok(Some(normalized))
    }
}

fn token_from_gh_cli(hostname: Option<&str>) -> Result<Option<String>, AuthError> {
    let mut cmd = Command::new("gh");
    cmd.arg("auth");
    cmd.arg("token");

    if let Some(h) = hostname {
        cmd.arg("--hostname");
        cmd.arg(h);
    }

    let output = cmd
        .output()
        .map_err(|e| AuthError::GhCliError(format!("Failed to run gh: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("not logged in") || stderr.contains("auth status") {
            return Ok(None);
        }
        return Err(AuthError::GhCliError(stderr.to_string()));
    }

    let token = String::from_utf8_lossy(&output.stdout).to_string();
    let normalized = normalize_token(&token);
    if normalized.is_empty() {
        Ok(None)
    } else {
        Ok(Some(normalized))
    }
}

pub fn check_gh_cli_available() -> bool {
    token_from_gh_cli(None).is_ok()
}

pub fn get_auth_status() -> Result<AuthStatus, AuthError> {
    let stax_env_available = read_env_token("STAX_GITHUB_TOKEN").is_some();
    let credentials_file_available = token_from_credentials_file()
        .map(|t| t.is_some())
        .unwrap_or(false);
    let gh_cli_available = token_from_gh_cli(None)
        .map(|t| t.is_some())
        .unwrap_or(false);
    let github_env_available = read_env_token("GITHUB_TOKEN").is_some();

    let mut status = AuthStatus {
        stax_env_available,
        credentials_file_available,
        gh_cli_available,
        github_env_available,
        ..Default::default()
    };

    status.active_source = resolve_active_source(&status);
    Ok(status)
}

pub fn run_auth(options: &AuthOptions) -> Result<AuthResult, AuthError> {
    let token = if options.from_gh {
        token_from_gh_cli(options.gh_hostname.as_deref())?.ok_or(AuthError::GhNotAvailable)?
    } else if let Some(t) = &options.token {
        if t.trim().is_empty() {
            return Err(AuthError::EmptyToken);
        }
        normalize_token(t)
    } else {
        return Err(AuthError::EmptyToken);
    };

    let path = credentials_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, &token)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o600);
        fs::set_permissions(&path, perms)?;
    }

    Ok(AuthResult {
        source: AuthSource::CredentialsFile,
        credentials_path: path.display().to_string(),
    })
}

pub fn get_saved_token() -> Result<Option<String>, AuthError> {
    token_from_credentials_file()
}

pub fn print_auth_status(status: &AuthStatus) {
    Output::info("Auth status");
    Output::info(
        "(The saved credentials-file token is reused for GitHub, GitLab, and Gitea API calls.)",
    );

    if let Some(source) = status.active_source {
        Output::info(&format!("Active source: {}", source.display_name()));
    } else {
        Output::warn("No GitHub auth source resolved.");
    }
    Output::info("");
    Output::info("Resolution order:");

    for (auth_source, available, enabled) in determine_auth_resolution_order(status) {
        let availability = if available { "available" } else { "not found" };
        let enabled_state = if enabled { "enabled" } else { "disabled" };

        let note = match auth_source {
            AuthSource::GithubTokenEnv if !status.allow_github_token_env => {
                " (disabled by default; enable with auth.allow_github_token_env = true)"
            }
            AuthSource::GhCli if status.gh_hostname.is_some() => {
                if let Some(ref hostname) = status.gh_hostname {
                    &format!(" (hostname: {})", hostname)
                } else {
                    ""
                }
            }
            _ => "",
        };

        Output::info(&format!(
            "  {}: {} ({}){}",
            auth_source.display_name(),
            availability,
            enabled_state,
            note
        ));
    }

    if status.active_source.is_none() {
        Output::info("");
        Output::info("Run `scp auth`, `scp auth --from-gh`, or `gh auth login`.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credentials_path_creation() {
        let path = credentials_path();
        assert!(path.is_ok());
        let p = path.unwrap();
        assert!(p.to_str().unwrap().ends_with(".credentials"));
    }

    #[test]
    fn test_normalize_token_whitespace() {
        assert_eq!(normalize_token("  token  "), "token");
        assert_eq!(normalize_token("token\n\r"), "token");
    }
}
