//! Gitoxide Tag Operations

use crate::error::{GitError, GitResult};

const TAG_REF_PREFIX: &str = "refs/tags/";

fn validate_tag_name(name: &str) -> GitResult<()> {
    if name.is_empty() {
        return Err(GitError::InvalidRef {
            name: "create".to_string(),
            reason: "Tag name cannot be empty".to_string(),
        });
    }
    if name.starts_with("ref:") {
        return Err(GitError::InvalidRef {
            name: "create".to_string(),
            reason: "Tag name cannot start with 'ref:'".to_string(),
        });
    }
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '/')
    {
        return Err(GitError::InvalidRef {
            name: "create".to_string(),
            reason: "Tag name contains invalid characters".to_string(),
        });
    }
    Ok(())
}

pub fn list(repo: &gix::Repository, _pattern: Option<&str>) -> GitResult<Vec<String>> {
    let refs = repo.references().map_err(|e| GitError::InvalidRef {
        name: "list".to_string(),
        reason: e.to_string(),
    })?;

    let tags_iter = refs.tags().map_err(|e| GitError::InvalidRef {
        name: "list".to_string(),
        reason: e.to_string(),
    })?;

    let tags: GitResult<Vec<String>> = tags_iter
        .map(|tag_result| {
            tag_result
                .map_err(|e| GitError::InvalidRef {
                    name: "list".to_string(),
                    reason: e.to_string(),
                })
                .map(|reference| reference.name().shorten().to_string())
        })
        .collect();

    tags
}

pub fn create(repo: &gix::Repository, name: &str, _message: &str, force: bool) -> GitResult<()> {
    validate_tag_name(name)?;

    let oid = repo.head_id().map_err(|e| GitError::InvalidRef {
        name: "HEAD".to_string(),
        reason: e.to_string(),
    })?;

    let reference_name = format!("{}{}", TAG_REF_PREFIX, name);

    if !force && repo.find_reference(&reference_name).is_ok() {
        return Err(GitError::Conflict {
            message: format!("Tag '{}' already exists", name),
            conflicted_files: vec![],
        });
    }

    let constraint = if force {
        gix::refs::transaction::PreviousValue::Any
    } else {
        gix::refs::transaction::PreviousValue::MustNotExist
    };

    repo.reference(
        reference_name,
        oid,
        constraint,
        format!("create tag {}", name),
    )
    .map_err(|e| GitError::InvalidRef {
        name: name.to_string(),
        reason: e.to_string(),
    })?;

    Ok(())
}

pub fn delete(repo: &gix::Repository, name: &str, force: bool) -> GitResult<()> {
    let reference_name = format!("{}{}", TAG_REF_PREFIX, name);

    let reference = if force {
        repo.find_reference(&reference_name).ok()
    } else {
        Some(
            repo.find_reference(&reference_name)
                .map_err(|_| GitError::NotFound(std::path::PathBuf::from(&reference_name)))?,
        )
    };

    if let Some(ref reference) = reference {
        reference.delete().map_err(|e| GitError::InvalidRef {
            name: name.to_string(),
            reason: e.to_string(),
        })?;
    }

    Ok(())
}

pub fn push(repo: &gix::Repository, remote: &str, tag: &str) -> GitResult<()> {
    let workdir = crate::gix::cli::require_workdir(repo, "tag push")?;

    let output = crate::gix::cli::run_git(workdir, &["push", remote, tag])?;

    if !output.success {
        return Err(crate::gix::cli::cli_error(&output, "tag push"));
    }

    Ok(())
}
