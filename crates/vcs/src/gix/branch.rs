//! Gitoxide Branch Operations
//!
//! Pure gitoxide implementation - no CLI spawning

use crate::{
    domain::entities::Branch,
    error::{GitError, GitResult},
};

/// Maximum allowed length for a branch name.
const MAX_BRANCH_NAME_LEN: usize = 255;

/// Validate a branch name to prevent path traversal, injection, and other attacks.
///
/// # Rules
/// - Non-empty
/// - Max 255 characters
/// - Only ASCII alphanumeric, `-`, `_`, `/`
/// - Must not contain `..`, `\`, `\0`, or spaces
/// - Must not start with `-` or `.`
/// - Must not end with `/`
/// - Must not contain consecutive `//`
pub fn validate_branch_name(name: &str) -> GitResult<()> {
    reject(name, name.is_empty(), "Branch name must not be empty")?;
    reject(
        name,
        name.len() > MAX_BRANCH_NAME_LEN,
        &format!("Branch name exceeds maximum length of {MAX_BRANCH_NAME_LEN} characters"),
    )?;
    reject(
        name,
        name.starts_with('-') || name.starts_with('.'),
        "Branch name must not start with '-' or '.'",
    )?;
    reject(
        name,
        name.ends_with('/'),
        "Branch name must not end with '/'",
    )?;
    reject(
        name,
        name.contains(".."),
        "Branch name must not contain '..'",
    )?;
    reject(
        name,
        name.contains('\\'),
        "Branch name must not contain backslashes",
    )?;
    reject(
        name,
        name.contains('\0'),
        "Branch name must not contain null bytes",
    )?;
    reject(
        name,
        name.contains(' '),
        "Branch name must not contain spaces",
    )?;
    reject(
        name,
        name.contains("//"),
        "Branch name must not contain consecutive slashes",
    )?;

    for ch in name.chars() {
        if !ch.is_ascii_alphanumeric() && ch != '-' && ch != '_' && ch != '/' {
            return Err(GitError::InvalidRef {
                name: name.to_string(),
                reason: format!(
                    "Branch name contains invalid character '{ch}'; \
                     only ASCII alphanumeric, '-', '_', '/' are allowed"
                ),
            });
        }
    }

    for component in name.split('/') {
        reject(
            name,
            component.is_empty(),
            "Branch name must not contain empty path components",
        )?;
    }

    Ok(())
}

fn reject(name: &str, cond: bool, reason: &str) -> GitResult<()> {
    if cond {
        return Err(GitError::InvalidRef {
            name: name.to_string(),
            reason: reason.to_string(),
        });
    }
    Ok(())
}

/// Get the name of the current branch.
pub fn current(repo: &gix::Repository) -> GitResult<String> {
    let head_name = repo.head_name().map_err(|e| GitError::InvalidRef {
        name: "HEAD".to_string(),
        reason: e.to_string(),
    })?;

    let name = head_name
        .ok_or_else(|| GitError::InvalidRef {
            name: "HEAD".to_string(),
            reason: "Detached HEAD state".to_string(),
        })?
        .shorten()
        .to_string();

    Ok(name)
}

/// List all branches in the repository.
pub fn list(repo: &gix::Repository, all: bool) -> GitResult<Vec<Branch>> {
    let current_branch = current(repo).ok();
    let mut branches = Vec::new();

    // Get local branches
    let refs = repo.references().map_err(|e| GitError::InvalidRef {
        name: "references".to_string(),
        reason: e.to_string(),
    })?;

    let local_iter = refs.local_branches().map_err(|e| GitError::InvalidRef {
        name: "local_branches".to_string(),
        reason: e.to_string(),
    })?;

    for branch_result in local_iter {
        let reference = branch_result.map_err(|e| GitError::InvalidRef {
            name: "branch".to_string(),
            reason: e.to_string(),
        })?;
        let name = reference.name().shorten().to_string();
        let is_current = current_branch.as_ref().map(|c| c == &name).unwrap_or(false);
        branches.push(Branch::new(name, is_current, None));
    }

    // Get remote branches if requested
    if all {
        let refs = repo.references().map_err(|e| GitError::InvalidRef {
            name: "references".to_string(),
            reason: e.to_string(),
        })?;

        let remote_iter = refs.remote_branches().map_err(|e| GitError::InvalidRef {
            name: "remote_branches".to_string(),
            reason: e.to_string(),
        })?;

        for branch_result in remote_iter {
            let reference = branch_result.map_err(|e| GitError::InvalidRef {
                name: "remote_branch".to_string(),
                reason: e.to_string(),
            })?;
            let name = reference.name().shorten().to_string();
            let is_current = current_branch.as_ref().map(|c| c == &name).unwrap_or(false);
            branches.push(Branch::new(name, is_current, None));
        }
    }

    Ok(branches)
}

/// Create a new branch.
pub fn create(repo: &gix::Repository, name: &str, force: bool) -> GitResult<()> {
    validate_branch_name(name)?;

    let oid = repo.head_id().map_err(|e| GitError::InvalidRef {
        name: "HEAD".to_string(),
        reason: e.to_string(),
    })?;

    let reference_name = format!("refs/heads/{}", name);

    // Check if branch already exists
    if !force && repo.find_reference(&reference_name).is_ok() {
        return Err(GitError::InvalidRef {
            name: name.to_string(),
            reason: "Branch already exists".to_string(),
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
        format!("create branch {}", name),
    )
    .map_err(|e| GitError::InvalidRef {
        name: name.to_string(),
        reason: e.to_string(),
    })?;

    Ok(())
}

/// Delete a branch.
pub fn delete(repo: &gix::Repository, name: &str, _force: bool) -> GitResult<()> {
    // Prevent deleting the currently checked-out branch.
    if let Ok(current) = current(repo) {
        if current == name {
            return Err(GitError::InvalidRef {
                name: name.to_string(),
                reason: "Cannot delete the currently checked-out branch".to_string(),
            });
        }
    }

    let reference_name = format!("refs/heads/{}", name);
    let reference = repo
        .find_reference(&reference_name)
        .map_err(|e| GitError::InvalidRef {
            name: name.to_string(),
            reason: e.to_string(),
        })?;

    reference.delete().map_err(|e| GitError::InvalidRef {
        name: name.to_string(),
        reason: e.to_string(),
    })?;

    Ok(())
}

/// Switch to a branch (checkout).
pub fn switch(repo: &gix::Repository, name: &str, _force: bool) -> GitResult<()> {
    let reference_name = format!("refs/heads/{}", name);
    let reference = repo
        .find_reference(&reference_name)
        .map_err(|e| GitError::InvalidRef {
            name: name.to_string(),
            reason: e.to_string(),
        })?;

    // Use the id() method which returns gix::Id
    let oid = reference.id();

    // Update HEAD to point to the new branch
    repo.reference(
        "HEAD",
        oid,
        gix::refs::transaction::PreviousValue::Any,
        format!("checkout: moving to {}", name),
    )
    .map_err(|e| GitError::InvalidRef {
        name: "HEAD".to_string(),
        reason: e.to_string(),
    })?;

    // Update working directory HEAD reference
    if let Some(workdir) = repo.workdir() {
        let head_path = workdir.join(".git").join("HEAD");
        std::fs::write(&head_path, format!("ref: refs/heads/{}\n", name))?;
    }

    Ok(())
}
