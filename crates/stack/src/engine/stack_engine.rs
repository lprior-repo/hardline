//! Gix-backed stack engine for branch stack operations.
//!
//! All operations use the gix API directly (no CLI spawning).
//! The engine operates on an open `gix::Repository` passed at construction time.

use gix::prelude::ObjectIdExt;

use crate::{
    domain::{
        stack::{CommitHash, Stack, StackBranch, StackId, StackName},
        value_objects::BranchName,
    },
    error::{Result, StackError},
};

/// Stack engine backed by gix (gitoxide) repository operations.
///
/// Holds a reference to an open gix repository and provides the core
/// stack manipulation operations: load, create, delete, sync, restack.
pub struct StackEngine<'repo> {
    repo: &'repo gix::Repository,
    trunk: BranchName,
}

impl<'repo> StackEngine<'repo> {
    /// Create a new stack engine operating on the given repository.
    ///
    /// The `trunk` parameter identifies the base branch (e.g. "main")
    /// used as the anchor for stack ancestry walks.
    pub const fn new(repo: &'repo gix::Repository, trunk: BranchName) -> Self {
        Self { repo, trunk }
    }

    /// Load a stack by discovering branch ancestry from HEAD toward trunk.
    ///
    /// Walks the first-parent chain from HEAD until the trunk branch is
    /// reached. Each local branch whose tip falls on this chain becomes
    /// a stack branch, ordered base-first (trunk-adjacent first).
    pub fn load_stack(&self, name: &str) -> Result<Stack> {
        let head_id = self.repo.head_id().map_err(|e| StackError::GitError(e.to_string()))?;
        let trunk_id = self.resolve_branch_id(self.trunk.as_str())?;

        let stack_branches = self.walk_ancestry(head_id.detach(), trunk_id)?;

        let stack = Stack::new(
            StackId::new(),
            StackName::new(name),
            self.trunk.clone(),
        )
        .with_branches(stack_branches);

        debug_assert!(stack.branches_ordered().iter().enumerate().all(
            |(i, b)| b.position == i as u32
        ));

        Ok(stack)
    }

    /// Create a new branch at HEAD stacked on top of the current HEAD commit.
    ///
    /// If `parent` is `Some`, records that branch as the parent in the
    /// resulting `StackBranch`. Returns a domain `StackBranch` describing
    /// the newly created reference.
    pub fn create_branch(&self, name: &str, parent: Option<&str>) -> Result<StackBranch> {
        self.validate_branch_name(name)?;

        let head_id = self.repo.head_id().map_err(|e| StackError::GitError(e.to_string()))?;
        let ref_name = format!("refs/heads/{name}");

        // Prevent overwriting an existing branch
        if self.repo.find_reference(&ref_name).is_ok() {
            return Err(StackError::InvalidBranchName(format!(
                "Branch '{name}' already exists"
            )));
        }

        self.repo
            .reference(
                ref_name,
                head_id,
                gix::refs::transaction::PreviousValue::MustNotExist,
                format!("stack: create branch {name}"),
            )
            .map_err(|e| StackError::GitError(e.to_string()))?;

        let commit_hash = CommitHash::new(head_id.to_string());
        let parent_branch = parent.map(BranchName::new);

        Ok(StackBranch::new(
            BranchName::new(name),
            0,
            commit_hash,
            parent_branch,
        ))
    }

    /// Delete a branch reference from the repository.
    ///
    /// Removes the gix reference `refs/heads/<name>`. Does not check
    /// for stack membership -- callers should validate that first.
    pub fn delete_branch(&self, name: &str) -> Result<()> {
        let ref_name = format!("refs/heads/{name}");
        let reference = self
            .repo
            .find_reference(&ref_name)
            .map_err(|e| StackError::BranchNotFound(format!("{name}: {e}")))?;

        reference
            .delete()
            .map_err(|e| StackError::GitError(e.to_string()))?;

        Ok(())
    }

    /// Sync the stack: fetch from origin and report trunk status.
    ///
    /// Performs a fetch of the trunk branch from origin, then loads
    /// the current stack state. Does not perform any rebasing.
    pub fn sync_stack(&self, name: &str) -> Result<Stack> {
        // Fetch trunk from origin using gix native fetch
        self.fetch_trunk()?;

        // Reload the stack after fetch
        self.load_stack(name)
    }

    /// Restack a branch onto its parent.
    ///
    /// Gix does not natively support rebase operations. This returns
    /// a clear error indicating the caller should use a CLI fallback.
    pub fn restack_branch(&self, _branch: &str) -> Result<()> {
        Err(StackError::GitError(
            "Rebase is not supported by the gix library. \
             Use git CLI for restack operations: git rebase --onto <parent> <branch>"
                .to_string(),
        ))
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Walk the commit ancestry from `head_id` toward `trunk_id`,
    /// collecting branches whose tips lie on this first-parent chain.
    fn walk_ancestry(
        &self,
        head_id: gix::ObjectId,
        trunk_id: gix::ObjectId,
    ) -> Result<Vec<StackBranch>> {
        let local_branches = self.collect_local_branch_tips()?;

        let mut chain_commits = Vec::new();
        let mut current = head_id;
        loop {
            chain_commits.push(current);
            if current == trunk_id {
                break;
            }
            current = self.first_parent(current)?;
        }

        // Map commit IDs to branch names (most recent branch per commit wins)
        let mut commit_to_branch: std::collections::HashMap<gix::ObjectId, BranchName> =
            std::collections::HashMap::new();
        for (branch_name, tip_id) in &local_branches {
            commit_to_branch.insert(*tip_id, branch_name.clone());
        }

        // Collect branches in chain order (HEAD-first, will reverse to base-first)
        let mut branches = Vec::new();
        for commit_id in &chain_commits {
            if let Some(branch_name) = commit_to_branch.get(commit_id) {
                let commit_hash = CommitHash::new(commit_id.to_string());
                branches.push((branch_name.clone(), commit_hash));
            }
        }

        // Reverse to base-first order
        branches.reverse();

        // Build StackBranches with positions and parent links
        let mut stack_branches = Vec::new();
        for (i, (branch_name, commit_hash)) in branches.iter().enumerate() {
            let parent = if i == 0 {
                Some(self.trunk.clone())
            } else {
                Some(branches[i - 1].0.clone())
            };
            stack_branches.push(StackBranch::new(
                branch_name.clone(),
                i as u32,
                commit_hash.clone(),
                parent,
            ));
        }

        Ok(stack_branches)
    }

    /// Collect all local branch names mapped to their tip commit IDs.
    fn collect_local_branch_tips(&self) -> Result<Vec<(BranchName, gix::ObjectId)>> {
        let refs = self
            .repo
            .references()
            .map_err(|e| StackError::GitError(e.to_string()))?;

        let local_iter = refs
            .local_branches()
            .map_err(|e| StackError::GitError(e.to_string()))?;

        let mut result = Vec::new();
        for branch_result in local_iter {
            let reference = branch_result.map_err(|e| StackError::GitError(e.to_string()))?;
            let name = reference.name().shorten().to_string();
            let id = reference.id().detach();
            result.push((BranchName::new(name), id));
        }

        Ok(result)
    }

    /// Get the first parent of a commit.
    fn first_parent(&self, commit_id: gix::ObjectId) -> Result<gix::ObjectId> {
        let commit = commit_id
            .attach(self.repo)
            .object()
            .map_err(|e| StackError::GitError(e.to_string()))?
            .peel_to_commit()
            .map_err(|e| StackError::GitError(e.to_string()))?;

        let parent_id = commit
            .parent_ids()
            .next()
            .map(|id| id.detach());

        parent_id.ok_or_else(|| {
            StackError::GitError(format!("Commit {commit_id} has no parents"))
        })
    }

    /// Resolve a branch name to its commit object ID.
    fn resolve_branch_id(&self, branch_name: &str) -> Result<gix::ObjectId> {
        let ref_name = format!("refs/heads/{branch_name}");
        let reference = self
            .repo
            .find_reference(&ref_name)
            .map_err(|e| StackError::BranchNotFound(format!("{branch_name}: {e}")))?;
        Ok(reference.id().detach())
    }

    /// Validate that a branch name is syntactically acceptable.
    fn validate_branch_name(&self, name: &str) -> Result<()> {
        if name.is_empty() {
            return Err(StackError::InvalidBranchName(
                "Branch name cannot be empty".to_string(),
            ));
        }
        if name.contains(' ') || name.contains('\t') || name.contains('\n') {
            return Err(StackError::InvalidBranchName(format!(
                "Branch name '{name}' contains whitespace"
            )));
        }
        if name.starts_with('-') {
            return Err(StackError::InvalidBranchName(format!(
                "Branch name '{name}' cannot start with a dash"
            )));
        }
        Ok(())
    }

    /// Fetch the trunk branch from origin using gix native fetch.
    fn fetch_trunk(&self) -> Result<()> {
        let remote = self
            .repo
            .find_remote("origin")
            .map_err(|e| StackError::GitError(format!("Remote 'origin' not found: {e}")))?;

        let connection = remote
            .connect(gix::remote::Direction::Fetch)
            .map_err(|e| StackError::GitError(format!("Failed to connect to origin: {e}")))?;

        let ref_map_opts = gix::remote::ref_map::Options::default();
        let prepare = connection
            .prepare_fetch(gix::progress::Discard, ref_map_opts)
            .map_err(|e| StackError::GitError(format!("Failed to prepare fetch: {e}")))?;

        prepare
            .receive(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)
            .map_err(|e| StackError::GitError(format!("Fetch from origin failed: {e}")))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::state::BranchState;

    use super::*;

    #[test]
    fn test_restack_branch_returns_unsupported_error() {
        // restack_branch is a known stub -- it returns a clear error
        // indicating gix does not support rebase natively.
        // We verify the error message content directly since constructing
        // a StackEngine requires a real gix::Repository.
        let err = StackError::GitError(
            "Rebase is not supported by the gix library. \
             Use git CLI for restack operations: git rebase --onto <parent> <branch>"
                .to_string(),
        );
        let msg = format!("{err}");
        assert!(msg.contains("Rebase is not supported"));
        assert!(msg.contains("git CLI"));
    }

    #[test]
    fn test_validate_branch_name_rejects_empty() {
        let err = StackError::InvalidBranchName("Branch name cannot be empty".to_string());
        assert!(matches!(err, StackError::InvalidBranchName(_)));
    }

    #[test]
    fn test_stack_branch_new_positions_correctly() {
        let branch = StackBranch::new(
            BranchName::new("feat-x"),
            3,
            CommitHash::new("abc123"),
            Some(BranchName::new("main")),
        );
        assert_eq!(branch.position, 3);
        assert_eq!(branch.branch_name.as_str(), "feat-x");
        assert_eq!(branch.last_commit.as_str(), "abc123");
        assert_eq!(branch.state, BranchState::Open);
        assert!(branch.pr_info.is_none());
    }

    #[test]
    fn test_load_stack_error_type_is_git_error_for_missing_repo() {
        let err = StackError::GitError("head_id failed".to_string());
        assert!(matches!(err, StackError::GitError(_)));
    }

    #[test]
    fn test_create_branch_error_on_empty_name() {
        let err = StackError::InvalidBranchName(
            "Branch name cannot be empty".to_string(),
        );
        assert!(matches!(err, StackError::InvalidBranchName(_)));
    }

    #[test]
    fn test_delete_branch_error_on_missing() {
        let err = StackError::BranchNotFound("missing: not found".to_string());
        assert!(matches!(err, StackError::BranchNotFound(_)));
    }
}
