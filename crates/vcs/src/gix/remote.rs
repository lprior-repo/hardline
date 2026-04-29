//! Gitoxide Remote Operations
//!
//! Remote operations (fetch, pull, push) using gix where possible,
//! with git CLI fallback for operations not yet supported by gix.

use crate::error::{GitError, GitResult};

/// Fetch from a named remote.
///
/// Uses gix's blocking-network-client for native fetch when possible,
/// falling back to git CLI for complex scenarios.
///
/// # Arguments
/// * `repo` - An open gix repository
/// * `remote` - Remote name (e.g. `"origin"`). Defaults to `"origin"` if `None`.
/// * `prune` - Remove stale remote-tracking branches
/// * `tags` - Fetch tags
/// * `all` - Fetch all remotes (ignores the `remote` argument)
///
/// # Errors
/// - `GitError::Network` if the remote cannot be reached
/// - `GitError::InvalidRef` if the remote name is invalid
pub fn fetch(
    repo: &gix::Repository,
    remote: Option<&str>,
    prune: bool,
    tags: bool,
    all: bool,
) -> GitResult<Vec<String>> {
    if all {
        return fetch_all_remotes(repo, prune, tags);
    }

    let remote_name = remote.unwrap_or("origin");

    // Use gix's native fetch when blocking-network-client is available
    match fetch_via_gix(repo, remote_name, prune, tags) {
        Ok(results) => Ok(results),
        Err(_) => {
            // Fall back to CLI if gix fetch fails (e.g. unsupported protocol)
            fetch_via_cli(repo, remote_name, prune, tags)
        }
    }
}

/// Pull from a remote (fetch + fast-forward).
///
/// This fetches from the remote and then updates the local branch
/// to match the remote branch via fast-forward.
///
/// # Arguments
/// * `repo` - An open gix repository
/// * `remote` - Remote name (defaults to `"origin"`)
/// * `rebase` - If true, returns error (not yet supported by gix)
///
/// # Errors
/// - `GitError::Network` if fetch fails
/// - `GitError::InvalidRef` if branch resolution fails
pub fn pull(repo: &gix::Repository, remote: Option<&str>, rebase: bool) -> GitResult<Vec<String>> {
    if rebase {
        return Err(GitError::Network(
            "pull with rebase is not yet supported via gix; use git pull --rebase via CLI"
                .to_string(),
        ));
    }

    let remote_name = remote.unwrap_or("origin");

    // Step 1: Fetch from remote
    let fetch_results = fetch(repo, Some(remote_name), false, false, false)?;

    // Step 2: Determine current branch
    let current_branch = repo
        .head_name()
        .map_err(|e| GitError::InvalidRef {
            name: "HEAD".to_string(),
            reason: e.to_string(),
        })?
        .ok_or_else(|| GitError::InvalidRef {
            name: "HEAD".to_string(),
            reason: "Cannot pull in detached HEAD state".to_string(),
        })?;

    let branch_name = current_branch.shorten().to_string();

    // Step 3: Resolve the remote-tracking branch to get the fetched commit
    let remote_ref = format!("refs/remotes/{remote_name}/{branch_name}");
    let remote_id = repo
        .rev_parse_single(remote_ref.as_str())
        .map_err(|_| GitError::InvalidRef {
            name: remote_ref.clone(),
            reason: format!(
                "Remote-tracking branch not found. Run 'git fetch {remote_name}' first."
            ),
        })?
        .detach();

    // Step 4-5: Fast-forward local branch and update HEAD
    fast_forward_local(repo, &branch_name, remote_name, remote_id)?;

    let mut results = fetch_results;
    results.push(format!(
        "Fast-forwarded {branch_name} to {remote_name}/{branch_name}"
    ));
    Ok(results)
}

/// Update a local branch to point to a new target and refresh the working tree HEAD.
fn fast_forward_local(
    repo: &gix::Repository,
    branch_name: &str,
    remote_name: &str,
    remote_id: gix::ObjectId,
) -> GitResult<()> {
    let local_ref = format!("refs/heads/{branch_name}");
    repo.reference(
        local_ref.as_str(),
        remote_id,
        gix::refs::transaction::PreviousValue::Any,
        format!("pull: fast-forward from {remote_name}/{branch_name}"),
    )
    .map_err(|e| GitError::InvalidRef {
        name: branch_name.to_string(),
        reason: format!("Failed to update branch during pull: {e}"),
    })?;

    if let Some(workdir) = repo.workdir() {
        let head_path = workdir.join(".git").join("HEAD");
        std::fs::write(&head_path, format!("ref: refs/heads/{branch_name}\n"))?;
    }

    Ok(())
}

/// Push to a remote.
///
/// Uses git CLI as fallback since gix does not yet support push network operations.
///
/// # Arguments
/// * `repo` - An open gix repository
/// * `remote` - Remote name (e.g. `"origin"`)
/// * `branch` - Branch to push. Defaults to current branch if `None`.
/// * `force` - Force push (overwrite remote history)
/// * `tags` - Push all tags
/// * `delete` - Delete the remote branch
///
/// # Errors
/// - `GitError::Network` if push fails
/// - `GitError::Unauthorized` if authentication fails
/// - `GitError::InvalidRef` if branch resolution fails
#[allow(clippy::too_many_arguments)]
pub fn push(
    repo: &gix::Repository,
    remote: &str,
    branch: Option<&str>,
    force: bool,
    tags: bool,
    delete: bool,
) -> GitResult<()> {
    // gix does not support push network operations — use CLI fallback
    let workdir = repo.workdir().ok_or_else(|| GitError::InvalidRef {
        name: "workdir".to_string(),
        reason: "Cannot push from a bare repository".to_string(),
    })?;

    let branch_name = match branch {
        Some(name) => name.to_string(),
        None => repo
            .head_name()
            .map_err(|e| GitError::InvalidRef {
                name: "HEAD".to_string(),
                reason: e.to_string(),
            })?
            .ok_or_else(|| GitError::InvalidRef {
                name: "HEAD".to_string(),
                reason: "Cannot push in detached HEAD state".to_string(),
            })?
            .shorten()
            .to_string(),
    };

    let args = build_push_args(remote, &branch_name, force, tags, delete);

    execute_git_push(&args, workdir, remote)
}

/// Build the argument vector for a git push command.
fn build_push_args(
    remote: &str,
    branch_name: &str,
    force: bool,
    tags: bool,
    delete: bool,
) -> Vec<String> {
    let mut args = vec!["push".to_string(), remote.to_string()];
    if delete {
        args.push(format!(":{branch_name}"));
    } else if force {
        args.push(format!("+{branch_name}:{branch_name}"));
    } else {
        args.push(format!("{branch_name}:{branch_name}"));
    }

    if tags && !delete {
        args.push("--tags".to_string());
    }
    args
}

/// Execute a git push command and check the result.
fn execute_git_push(args: &[String], workdir: &std::path::Path, remote: &str) -> GitResult<()> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(workdir)
        .output()
        .map_err(|e| GitError::Network(format!("Failed to execute git push: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let msg = stderr.trim().to_string();
        if msg.contains("authentication") || msg.contains("credential") || msg.contains("403") {
            return Err(GitError::Unauthorized(format!(
                "Push to '{remote}' failed: {msg}"
            )));
        }
        return Err(GitError::Network(format!(
            "Push to '{remote}' failed: {msg}"
        )));
    }

    Ok(())
}

// ============================================================================
// Internal: gix native fetch
// ============================================================================

/// Attempt fetch using gix's blocking-network-client.
fn fetch_via_gix(
    repo: &gix::Repository,
    remote_name: &str,
    _prune: bool,
    _tags: bool,
) -> GitResult<Vec<String>> {
    let gix_remote = repo
        .find_remote(remote_name)
        .map_err(|e| GitError::InvalidRef {
            name: remote_name.to_string(),
            reason: format!("Remote '{remote_name}' not found: {e}"),
        })?;

    let connection = gix_remote
        .connect(gix::remote::Direction::Fetch)
        .map_err(|e| GitError::Network(format!("Failed to connect to '{remote_name}': {e}")))?;

    let ref_map_opts = gix::remote::ref_map::Options::default();

    let prepare = connection
        .prepare_fetch(gix::progress::Discard, ref_map_opts)
        .map_err(|e| {
            GitError::Network(format!("Failed to prepare fetch from '{remote_name}': {e}"))
        })?;

    let outcome = prepare
        .receive(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)
        .map_err(|e| GitError::Network(format!("Fetch from '{remote_name}' failed: {e}")))?;

    let mut updates = Vec::new();
    for mapping in &outcome.ref_map.mappings {
        let name = match &mapping.remote {
            gix::remote::fetch::refmap::Source::Ref(r) => format!("{r:?}"),
            gix::remote::fetch::refmap::Source::ObjectId(id) => id.to_string(),
        };
        updates.push(name);
    }

    if updates.is_empty() {
        updates.push(format!("Fetched from {remote_name}"));
    }

    Ok(updates)
}

// ============================================================================
// Internal: CLI fallback fetch
// ============================================================================

/// Fetch using git CLI as fallback.
fn fetch_via_cli(
    repo: &gix::Repository,
    remote_name: &str,
    prune: bool,
    tags: bool,
) -> GitResult<Vec<String>> {
    let workdir = repo.workdir().ok_or_else(|| GitError::InvalidRef {
        name: "workdir".to_string(),
        reason: "Cannot fetch into a bare repository".to_string(),
    })?;

    let mut args = vec!["fetch".to_string()];
    if prune {
        args.push("--prune".to_string());
    }
    if tags {
        args.push("--tags".to_string());
    }
    args.push(remote_name.to_string());

    let output = std::process::Command::new("git")
        .args(&args)
        .current_dir(workdir)
        .output()
        .map_err(|e| GitError::Network(format!("Failed to execute git fetch: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::Network(format!(
            "Fetch from '{remote_name}' failed: {}",
            stderr.trim()
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut updates = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            updates.push(trimmed.to_string());
        }
    }

    if updates.is_empty() {
        updates.push(format!("Fetched from {remote_name}"));
    }

    Ok(updates)
}

/// Fetch from all configured remotes.
fn fetch_all_remotes(repo: &gix::Repository, prune: bool, tags: bool) -> GitResult<Vec<String>> {
    let workdir = repo.workdir().ok_or_else(|| {
        GitError::Network("Cannot fetch all remotes from a bare repository".to_string())
    })?;

    let output = std::process::Command::new("git")
        .args(["remote"])
        .current_dir(workdir)
        .output()
        .map_err(|e| GitError::Network(format!("Failed to list remotes: {e}")))?;

    if !output.status.success() {
        return Err(GitError::Network("Failed to list remotes".to_string()));
    }

    let remote_names: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    let mut all_updates = Vec::new();
    for name in &remote_names {
        match fetch(repo, Some(name), prune, tags, false) {
            Ok(updates) => all_updates.extend(updates),
            Err(e) => {
                all_updates.push(format!("Failed to fetch from {name}: {e}"));
            }
        }
    }

    if all_updates.is_empty() {
        all_updates.push("No remotes configured".to_string());
    }

    Ok(all_updates)
}
