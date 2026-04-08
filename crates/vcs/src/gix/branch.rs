//! Gitoxide Branch Operations
//!
//! Pure gitoxide implementation - no CLI spawning
//!
//! Port of stax/src/git/repo.rs branch-related operations.

use crate::domain::entities::Branch;
use crate::error::{GitError, GitResult};
use gix::prelude::ObjectIdExt;

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
        let _ = std::fs::write(&head_path, format!("ref: refs/heads/{}\n", name));
    }

    Ok(())
}

/// Create a new branch from another local branch.
pub fn create_at(repo: &gix::Repository, name: &str, base_branch: &str) -> GitResult<()> {
    let base_ref_name = format!("refs/heads/{base_branch}");
    let reference = repo
        .find_reference(&base_ref_name)
        .map_err(|e| GitError::InvalidRef {
            name: base_branch.to_string(),
            reason: format!("Base branch not found: {e}"),
        })?;

    let oid = reference
        .try_id()
        .ok_or_else(|| GitError::InvalidRef {
            name: base_branch.to_string(),
            reason: "Reference has no target".to_string(),
        })?
        .detach();

    let reference_name = format!("refs/heads/{name}");
    repo.reference(
        reference_name,
        oid,
        gix::refs::transaction::PreviousValue::MustNotExist,
        format!("create branch {name} from {base_branch}"),
    )
    .map_err(|e| GitError::InvalidRef {
        name: name.to_string(),
        reason: format!("Failed to create branch: {e}"),
    })?;

    Ok(())
}

/// Create a new branch at a specific commit SHA.
pub fn create_at_commit(repo: &gix::Repository, name: &str, commit_sha: &str) -> GitResult<()> {
    let oid: gix::ObjectId = commit_sha
        .parse()
        .map_err(|e| GitError::InvalidRef {
            name: commit_sha.to_string(),
            reason: format!("Invalid commit SHA: {e}"),
        })?;

    // Verify the commit exists
    oid.attach(repo)
        .object()
        .map_err(|e| GitError::InvalidRef {
            name: commit_sha.to_string(),
            reason: format!("Commit not found: {e}"),
        })?;

    let reference_name = format!("refs/heads/{name}");
    repo.reference(
        reference_name,
        oid,
        gix::refs::transaction::PreviousValue::MustNotExist,
        format!("create branch {name} at {commit_sha}"),
    )
    .map_err(|e| GitError::InvalidRef {
        name: name.to_string(),
        reason: format!("Failed to create branch: {e}"),
    })?;

    Ok(())
}

/// Get the commit SHA for a branch.
pub fn branch_commit(repo: &gix::Repository, branch: &str) -> GitResult<String> {
    let ref_name = format!("refs/heads/{branch}");
    let reference = repo
        .find_reference(&ref_name)
        .map_err(|e| GitError::InvalidRef {
            name: branch.to_string(),
            reason: format!("Branch not found: {e}"),
        })?;

    let oid = reference
        .try_id()
        .ok_or_else(|| GitError::InvalidRef {
            name: branch.to_string(),
            reason: "Reference has no target".to_string(),
        })?
        .detach();

    Ok(oid.to_string())
}

/// Auto-detect trunk branch (main or master).
///
/// Checks for "main" first, then "master".
pub fn detect_trunk(repo: &gix::Repository) -> GitResult<String> {
    for name in ["main", "master"] {
        let ref_name = format!("refs/heads/{name}");
        if repo.find_reference(&ref_name).is_ok() {
            return Ok(name.to_string());
        }
    }
    Err(GitError::InvalidRef {
        name: "trunk".to_string(),
        reason: "No trunk branch (main/master) found".to_string(),
    })
}

/// Check if a branch has a remote tracking branch (origin/<branch>).
pub fn has_remote(repo: &gix::Repository, branch: &str) -> bool {
    let remote_name = format!("refs/remotes/origin/{branch}");
    repo.find_reference(&remote_name).is_ok()
}

/// Return all branch names under `refs/remotes/<remote>/`.
pub fn remote_branch_names(repo: &gix::Repository, remote: &str) -> GitResult<Vec<String>> {
    let prefix = format!("refs/remotes/{remote}/");
    crate::gix::refs::list_refs_by_prefix(repo, &prefix)
}

/// Check if a local branch exists.
pub fn local_branch_exists(repo: &gix::Repository, branch: &str) -> bool {
    let ref_name = format!("refs/heads/{branch}");
    repo.find_reference(&ref_name).is_ok()
}
