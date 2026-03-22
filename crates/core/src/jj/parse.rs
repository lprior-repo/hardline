//! JJ output parsing - Calculations layer
//!
//! Pure functions for parsing JJ command output.

use std::path::PathBuf;
use std::sync::OnceLock;

use crate::jj::types::{DiffSummary, Status, WorkspaceInfo};
use crate::{Error, Result};

/// Parse workspace list output
pub fn parse_workspace_list(output: &str) -> Result<Vec<WorkspaceInfo>> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let parts: Vec<&str> = line.splitn(2, ':').collect();
            if parts.len() != 2 {
                return Err(Error::InvalidIdentifier(format!(
                    "Invalid workspace list format: {line}"
                )));
            }

            let name = parts
                .first()
                .ok_or_else(|| {
                    Error::InvalidIdentifier("Missing workspace name in list output".to_string())
                })?
                .trim()
                .to_string();
            let rest = parts
                .get(1)
                .ok_or_else(|| {
                    Error::InvalidIdentifier("Missing workspace path in list output".to_string())
                })?
                .trim();

            let (path_str, is_stale) = rest
                .strip_suffix("(stale)")
                .map_or((rest, false), |path_part| (path_part.trim(), true));

            Ok(WorkspaceInfo {
                name,
                path: PathBuf::from(path_str),
                is_stale,
            })
        })
        .collect()
}

/// Parse status output
#[must_use]
pub fn parse_status(output: &str) -> Status {
    output.lines().fold(Status::default(), |mut status, line| {
        let line = line.trim();
        if line.is_empty() {
            return status;
        }

        if let Some(rest) = line.strip_prefix('M') {
            status.modified.push(PathBuf::from(rest.trim()));
        } else if let Some(rest) = line.strip_prefix('A') {
            status.added.push(PathBuf::from(rest.trim()));
        } else if let Some(rest) = line.strip_prefix('D') {
            status.deleted.push(PathBuf::from(rest.trim()));
        } else if let Some(rest) = line.strip_prefix('R') {
            if let Some((old, new)) = rest.split_once("=>") {
                status
                    .renamed
                    .push((PathBuf::from(old.trim()), PathBuf::from(new.trim())));
            }
        } else if let Some(rest) = line.strip_prefix('?') {
            status.unknown.push(PathBuf::from(rest.trim()));
        }
        status
    })
}

/// Parse diff stat output
#[must_use]
pub fn parse_diff_stat(output: &str) -> DiffSummary {
    use regex::Regex;
    static INSERTIONS_RE: OnceLock<Option<Regex>> = OnceLock::new();
    static DELETIONS_RE: OnceLock<Option<Regex>> = OnceLock::new();

    let insertions_re = INSERTIONS_RE.get_or_init(|| Regex::new(r"(\d+)\s+insertion").ok());
    let deletions_re = DELETIONS_RE.get_or_init(|| Regex::new(r"(\d+)\s+deletion").ok());

    let summary_line = output
        .lines()
        .find(|line| line.contains("insertion") || line.contains("deletion"))
        .map_or("", |s| s);

    let insertions = insertions_re
        .as_ref()
        .and_then(|re| re.captures(summary_line))
        .and_then(|caps| caps.get(1))
        .and_then(|m| m.as_str().parse().ok())
        .map_or(0, |n: usize| n);

    let deletions = deletions_re
        .as_ref()
        .and_then(|re| re.captures(summary_line))
        .and_then(|caps| caps.get(1))
        .and_then(|m| m.as_str().parse().ok())
        .map_or(0, |n: usize| n);

    DiffSummary {
        insertions,
        deletions,
    }
}
