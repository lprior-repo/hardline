//! Gitoxide Tag Operations

use crate::error::{GitError, GitResult};

/// List tags
pub fn list(repo: &gix::Repository, _pattern: Option<&str>) -> GitResult<Vec<String>> {
    let mut tags = Vec::new();

    let refs = repo.references().map_err(|e| GitError::InvalidRef {
        name: "list".to_string(),
        reason: e.to_string(),
    })?;

    let tags_iter = refs.tags().map_err(|e| GitError::InvalidRef {
        name: "list".to_string(),
        reason: e.to_string(),
    })?;

    for tag_result in tags_iter {
        let reference = tag_result.map_err(|e| GitError::InvalidRef {
            name: "list".to_string(),
            reason: e.to_string(),
        })?;
        let name = reference.name().shorten().to_string();
        tags.push(name);
    }

    Ok(tags)
}

/// Create tag - stub
pub fn create(_repo: &gix::Repository, _name: &str, _message: &str, _force: bool) -> GitResult<()> {
    Err(GitError::InvalidRef {
        name: "create".to_string(),
        reason: "Not yet implemented with gix".to_string(),
    })
}

/// Delete tag - stub
pub fn delete(_repo: &gix::Repository, _name: &str, _force: bool) -> GitResult<()> {
    Err(GitError::InvalidRef {
        name: "delete".to_string(),
        reason: "Not yet implemented with gix".to_string(),
    })
}

/// Push tag - stub
pub fn push(_repo: &gix::Repository, _remote: &str, _tag: &str) -> GitResult<()> {
    Err(GitError::InvalidRef {
        name: "push".to_string(),
        reason: "Not yet implemented with gix".to_string(),
    })
}
