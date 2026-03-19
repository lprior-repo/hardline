//! Gitoxide Commit Operations

use crate::domain::entities::Commit;
use crate::error::{GitError, GitResult};
use chrono::{DateTime, TimeZone, Utc};
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
        let datetime: DateTime<Utc> = Utc
            .timestamp_opt(timestamp, 0)
            .single()
            .unwrap_or_else(Utc::now);

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
    let datetime: DateTime<Utc> = Utc
        .timestamp_opt(timestamp, 0)
        .single()
        .unwrap_or_else(Utc::now);

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
    let datetime: DateTime<Utc> = Utc
        .timestamp_opt(timestamp, 0)
        .single()
        .unwrap_or_else(Utc::now);

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
