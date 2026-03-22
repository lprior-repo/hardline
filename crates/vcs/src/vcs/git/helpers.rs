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
