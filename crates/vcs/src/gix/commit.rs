//! Gitoxide Commit Operations
//!
//! Port of stax/src/git/repo.rs commit-related operations.

use crate::domain::entities::Commit;
use crate::error::{GitError, GitResult};
use chrono::{TimeZone, Utc};
use gix::bstr::ByteSlice;
use gix::prelude::ObjectIdExt;

/// Get commit log
pub fn log(repo: &gix::Repository, limit: usize) -> GitResult<Vec<Commit>> {
    let head_id = repo.head_id().map_err(|e| GitError::InvalidRef {
        name: "HEAD".to_string(),
        reason: e.to_string(),
    })?;

    let mut commits = Vec::new();

    let iter = repo
        .rev_walk(Some(head_id))
        .first_parent_only()
        .all()
        .map_err(|e| GitError::InvalidRef {
            name: "rev_walk".to_string(),
            reason: e.to_string(),
        })?;

    for (i, commit_result) in iter.enumerate() {
        if i >= limit {
            break;
        }

        let commit =
            commit_result.map_err(|e: gix::revision::walk::iter::Error| GitError::InvalidRef {
                name: "commit".to_string(),
                reason: e.to_string(),
            })?;

        let commit = commit.object().map_err(|e| GitError::InvalidRef {
            name: "object".to_string(),
            reason: e.to_string(),
        })?;

        let time = commit.time().map_err(|e| GitError::InvalidRef {
            name: "time".to_string(),
            reason: e.to_string(),
        })?;
        let timestamp = time.seconds;
        let datetime =
            Utc.timestamp_opt(timestamp, 0)
                .single()
                .ok_or_else(|| GitError::InvalidRef {
                    name: "time".to_string(),
                    reason: "timestamp out of range".to_string(),
                })?;

        let parent_ids: Vec<String> = commit.parent_ids().map(|id| id.to_string()).collect();

        let message = commit.message_raw().map_err(|e| GitError::InvalidRef {
            name: "message".to_string(),
            reason: e.to_string(),
        })?;
        let message_str = String::from_utf8_lossy(message.as_bytes())
            .trim()
            .to_string();

        let author = commit.author().map_err(|e| GitError::InvalidRef {
            name: "author".to_string(),
            reason: e.to_string(),
        })?;

        commits.push(Commit::new(
            commit.id().to_string(),
            message_str,
            author.name.to_string(),
            datetime,
            parent_ids,
        ));
    }

    Ok(commits)
}

/// Find a commit by OID
pub fn find(repo: &gix::Repository, oid_str: &str) -> GitResult<Commit> {
    let oid = oid_str
        .parse::<gix::ObjectId>()
        .map_err(|e| GitError::InvalidRef {
            name: oid_str.to_string(),
            reason: e.to_string(),
        })?;

    let commit = oid
        .attach(repo)
        .object()
        .map_err(|e| GitError::InvalidRef {
            name: oid_str.to_string(),
            reason: e.to_string(),
        })?
        .peel_to_commit()
        .map_err(|e| GitError::InvalidRef {
            name: oid_str.to_string(),
            reason: e.to_string(),
        })?;

    let time = commit.time().map_err(|e| GitError::InvalidRef {
        name: "time".to_string(),
        reason: e.to_string(),
    })?;
    let timestamp = time.seconds;
    let datetime =
        Utc.timestamp_opt(timestamp, 0)
            .single()
            .ok_or_else(|| GitError::InvalidRef {
                name: "time".to_string(),
                reason: "timestamp out of range".to_string(),
            })?;

    let parent_ids: Vec<String> = commit.parent_ids().map(|id| id.to_string()).collect();

    let message = commit.message_raw().map_err(|e| GitError::InvalidRef {
        name: "message".to_string(),
        reason: e.to_string(),
    })?;
    let message_str = String::from_utf8_lossy(message.as_bytes())
        .trim()
        .to_string();

    let author = commit.author().map_err(|e| GitError::InvalidRef {
        name: "author".to_string(),
        reason: e.to_string(),
    })?;

    Ok(Commit::new(
        commit.id().to_string(),
        message_str,
        author.name.to_string(),
        datetime,
        parent_ids,
    ))
}

/// Get current commit
pub fn current(repo: &gix::Repository) -> GitResult<Commit> {
    let commit = repo.head_commit().map_err(|e| GitError::InvalidRef {
        name: "HEAD".to_string(),
        reason: e.to_string(),
    })?;

    let time = commit.time().map_err(|e| GitError::InvalidRef {
        name: "time".to_string(),
        reason: e.to_string(),
    })?;
    let timestamp = time.seconds;
    let datetime =
        Utc.timestamp_opt(timestamp, 0)
            .single()
            .ok_or_else(|| GitError::InvalidRef {
                name: "time".to_string(),
                reason: "timestamp out of range".to_string(),
            })?;

    let parent_ids: Vec<String> = commit.parent_ids().map(|id| id.to_string()).collect();

    let message = commit.message_raw().map_err(|e| GitError::InvalidRef {
        name: "message".to_string(),
        reason: e.to_string(),
    })?;
    let message_str = String::from_utf8_lossy(message.as_bytes())
        .trim()
        .to_string();

    let author = commit.author().map_err(|e| GitError::InvalidRef {
        name: "author".to_string(),
        reason: e.to_string(),
    })?;

    Ok(Commit::new(
        commit.id().to_string(),
        message_str,
        author.name.to_string(),
        datetime,
        parent_ids,
    ))
}

/// Get commits ahead/behind between two refs.
///
/// Returns `(ahead, behind)` where `ahead` is commits in `head` not in `base`,
/// and `behind` is commits in `base` not in `head`.
pub fn ahead_behind(repo: &gix::Repository, base: &str, head: &str) -> GitResult<(usize, usize)> {
    let base_oid = resolve_to_oid(repo, base)?;
    let head_oid = resolve_to_oid(repo, head)?;

    // Walk from head, counting commits not reachable from base
    let ahead = count_unique_commits(repo, head_oid, base_oid)?;

    // Walk from base, counting commits not reachable from head
    let behind = count_unique_commits(repo, base_oid, head_oid)?;

    Ok((ahead, behind))
}

/// Get commit messages between base and head (commits on head not in base).
pub fn between(repo: &gix::Repository, base: &str, head: &str) -> GitResult<Vec<String>> {
    let base_oid = resolve_to_oid(repo, base)?;
    let head_oid = resolve_to_oid(repo, head)?;

    let walk = repo
        .rev_walk(Some(head_oid))
        .first_parent_only()
        .all()
        .map_err(|e| GitError::InvalidRef {
            name: "rev_walk".to_string(),
            reason: format!("Failed to walk commits: {e}"),
        })?;

    let mut messages = Vec::new();
    for item in walk {
        let info = item.map_err(|e| GitError::InvalidRef {
            name: "walk".to_string(),
            reason: format!("Failed to read commit: {e}"),
        })?;

        // Stop at merge base
        if info.id == base_oid {
            break;
        }

        let commit_obj = info.object().map_err(|e| GitError::InvalidRef {
            name: info.id.to_string(),
            reason: format!("Failed to read commit object: {e}"),
        })?;

        let message_raw = commit_obj.message_raw().map_err(|e| GitError::InvalidRef {
            name: info.id.to_string(),
            reason: format!("Failed to read message: {e}"),
        })?;
        let summary = message_raw
            .as_bstr()
            .to_str_lossy()
            .lines()
            .next()
            .unwrap_or("")
            .to_string();

        messages.push(summary);
    }

    Ok(messages)
}

/// Get recent commits unique to a branch (not in parent).
///
/// Returns at most `limit` commits with short hash and message.
pub fn branch_commits(
    repo: &gix::Repository,
    branch: &str,
    parent: Option<&str>,
    limit: usize,
) -> GitResult<Vec<CommitInfo>> {
    let branch_oid = resolve_to_oid(repo, branch)?;

    let walk = repo
        .rev_walk(Some(branch_oid))
        .first_parent_only()
        .all()
        .map_err(|e| GitError::InvalidRef {
            name: "rev_walk".to_string(),
            reason: format!("Failed to walk commits: {e}"),
        })?;

    let parent_oid = parent.map(|p| resolve_to_oid(repo, p)).transpose()?;

    let mut commits = Vec::new();
    for item in walk.take(limit) {
        let info = item.map_err(|e| GitError::InvalidRef {
            name: "walk".to_string(),
            reason: format!("Failed to read commit: {e}"),
        })?;

        // Stop at parent
        if let Some(poid) = parent_oid {
            if info.id == poid {
                break;
            }
        }

        let commit_obj = info.object().map_err(|e| GitError::InvalidRef {
            name: info.id.to_string(),
            reason: format!("Failed to read commit object: {e}"),
        })?;

        let message_raw = commit_obj.message_raw().map_err(|e| GitError::InvalidRef {
            name: info.id.to_string(),
            reason: format!("Failed to read message: {e}"),
        })?;
        let message = message_raw
            .as_bstr()
            .to_str_lossy()
            .lines()
            .next()
            .unwrap_or("")
            .to_string();

        let short_hash = info.id.to_string();
        let short_hash = if short_hash.len() >= 10 {
            short_hash[..10].to_string()
        } else {
            short_hash
        };

        commits.push(CommitInfo {
            short_hash,
            message,
        });
    }

    Ok(commits)
}

/// Get a human-readable time since the last commit on a branch.
pub fn branch_age(repo: &gix::Repository, branch: &str) -> GitResult<String> {
    let branch_oid = resolve_to_oid(repo, branch)?;
    let commit_obj = branch_oid
        .attach(repo)
        .object()
        .map_err(|e| GitError::InvalidRef {
            name: branch.to_string(),
            reason: format!("Failed to read commit: {e}"),
        })?
        .peel_to_commit()
        .map_err(|e| GitError::InvalidRef {
            name: branch.to_string(),
            reason: format!("Not a commit: {e}"),
        })?;

    let time = commit_obj.time().map_err(|e| GitError::InvalidRef {
        name: branch.to_string(),
        reason: format!("Failed to get time: {e}"),
    })?;
    let commit_ts = time.seconds;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let diff = now - commit_ts;
    Ok(format_duration(diff))
}

// ============================================================================
// Types
// ============================================================================

/// Summary of a commit for display purposes.
#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub short_hash: String,
    pub message: String,
}

// ============================================================================
// Internal helpers
// ============================================================================

/// Resolve a branch name or ref to an OID.
fn resolve_to_oid(repo: &gix::Repository, refspec: &str) -> GitResult<gix::ObjectId> {
    // Try as local branch
    let local_ref = format!("refs/heads/{refspec}");
    if let Ok(reference) = repo.find_reference(&local_ref) {
        if let Some(oid) = reference.try_id() {
            return Ok(oid.detach());
        }
    }

    // Try as direct reference
    if let Ok(reference) = repo.find_reference(refspec) {
        if let Some(oid) = reference.try_id() {
            return Ok(oid.detach());
        }
    }

    // Try rev-parse
    let oid = repo
        .rev_parse_single(refspec)
        .map_err(|e| GitError::InvalidRef {
            name: refspec.to_string(),
            reason: format!("Cannot resolve: {e}"),
        })?
        .detach();

    Ok(oid)
}

/// Count commits reachable from `tip` but not from `base`.
fn count_unique_commits(
    repo: &gix::Repository,
    tip: gix::ObjectId,
    base: gix::ObjectId,
) -> GitResult<usize> {
    let walk = repo
        .rev_walk(Some(tip))
        .first_parent_only()
        .all()
        .map_err(|e| GitError::InvalidRef {
            name: "rev_walk".to_string(),
            reason: format!("Failed to walk commits: {e}"),
        })?;

    let mut count = 0usize;
    for item in walk {
        let info = item.map_err(|e| GitError::InvalidRef {
            name: "walk".to_string(),
            reason: format!("Failed to read commit: {e}"),
        })?;

        if info.id == base {
            break;
        }
        count += 1;
    }

    Ok(count)
}

/// Format a duration in seconds as a human-readable string.
fn format_duration(seconds: i64) -> String {
    if seconds < 60 {
        "just now".to_string()
    } else if seconds < 3600 {
        let mins = seconds / 60;
        format!("{} minute{} ago", mins, if mins == 1 { "" } else { "s" })
    } else if seconds < 86400 {
        let hours = seconds / 3600;
        format!("{} hour{} ago", hours, if hours == 1 { "" } else { "s" })
    } else {
        let days = seconds / 86400;
        format!("{} day{} ago", days, if days == 1 { "" } else { "s" })
    }
}
