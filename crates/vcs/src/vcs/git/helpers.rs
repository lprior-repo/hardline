//! Git helper functions
//!
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use crate::vcs::VcsError;

/// Parse Git version from output like "git version 2.43.0"
pub fn parse_git_version(output: &str) -> Result<(u32, u32), VcsError> {
    let output = output.trim();

    let parts: Vec<&str> = output.split_whitespace().collect();

    if parts.len() < 3 {
        return Err(VcsError::GitParseError(format!(
            "Unexpected git version format: {output}"
        )));
    }

    let version_str = parts[2];

    let version_parts: Vec<&str> = version_str.split('.').collect();

    if version_parts.len() < 2 {
        return Err(VcsError::GitParseError(format!(
            "Invalid version number: {version_str}"
        )));
    }

    let major = version_parts[0].parse::<u32>().map_err(|_| {
        VcsError::GitParseError(format!("Invalid major version: {}", version_parts[0]))
    })?;

    let minor = version_parts[1].parse::<u32>().map_err(|_| {
        VcsError::GitParseError(format!("Invalid minor version: {}", version_parts[1]))
    })?;

    Ok((major, minor))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_standard_version() {
        assert_eq!(parse_git_version("git version 2.43.0").expect("parse"), (2, 43));
    }

    #[test]
    fn parse_version_with_patch() {
        assert_eq!(parse_git_version("git version 2.38.4").expect("parse"), (2, 38));
    }

    #[test]
    fn parse_version_windows_suffix() {
        assert_eq!(parse_git_version("git version 2.43.0.windows.1").expect("parse"), (2, 43));
    }

    #[test]
    fn parse_version_apple_suffix() {
        assert_eq!(parse_git_version("git version 2.39.3 (Apple Git-146)").expect("parse"), (2, 39));
    }

    #[test]
    fn parse_version_minimal() {
        assert_eq!(parse_git_version("git version 1.0").expect("parse"), (1, 0));
    }

    #[test]
    fn parse_version_major_zero() {
        assert_eq!(parse_git_version("git version 0.99").expect("parse"), (0, 99));
    }

    #[test]
    fn parse_version_large_numbers() {
        assert_eq!(parse_git_version("git version 100.200").expect("parse"), (100, 200));
    }

    #[test]
    fn parse_error_empty_input() {
        assert!(matches!(parse_git_version(""), Err(VcsError::GitParseError(_))));
    }

    #[test]
    fn parse_error_single_word() {
        assert!(matches!(parse_git_version("git"), Err(VcsError::GitParseError(_))));
    }

    #[test]
    fn parse_error_two_words() {
        assert!(matches!(parse_git_version("git version"), Err(VcsError::GitParseError(_))));
    }

    #[test]
    fn parse_error_no_dots() {
        assert!(matches!(parse_git_version("git version abc"), Err(VcsError::GitParseError(_))));
    }

    #[test]
    fn parse_error_invalid_major() {
        assert!(matches!(parse_git_version("git version abc.0"), Err(VcsError::GitParseError(_))));
    }

    #[test]
    fn parse_error_invalid_minor() {
        assert!(matches!(parse_git_version("git version 2.xyz"), Err(VcsError::GitParseError(_))));
    }

    #[test]
    fn parse_trims_whitespace() {
        assert_eq!(parse_git_version("  git version 2.40.0  ").expect("parse"), (2, 40));
    }

    #[test]
    fn parse_extra_whitespace_between_parts() {
        assert_eq!(parse_git_version("git  version  2.41.0").expect("parse"), (2, 41));
    }
}
