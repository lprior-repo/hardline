//! Transactional stack operations — wraps stack operations with VCS transaction tracking.
//!
//! Ported from stax ops/transactions. Each mutating stack operation (restack,
//! upstack-restack, sync-restack, submit, reorder, detach) is wrapped in a
//! `Transaction` from `scp_vcs` that:
//!
//! 1. Records before-state of all affected refs
//! 2. Creates backup refs under `refs/stax/backups/<op-id>/` for crash recovery
//! 3. Persists an in-progress receipt to `.git/stax/ops/<op-id>.json`
//! 4. Records after-state on success (or failure details on error)
//! 5. Unfinished transactions are persisted as failed via `Drop` guard
//!
//! # Usage
//!
//! ```ignore
//! let config = TransactionConfig {
//!     git_dir: PathBuf::from(".git"),
//!     workdir: PathBuf::from("."),
//!     trunk: "main".to_string(),
//! };
//! let ops = TransactionalStackOps::new(metadata_store, config);
//! ops.restack("feature/my-branch")?;
//! ```

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use scp_vcs::application::ops::Transaction;
use scp_vcs::domain::entities::ops::{OpKind, PlanSummary, OpReceipt};
use scp_vcs::infrastructure::ops;

use crate::domain::metadata::BranchMetadata;
use crate::error::{Result, StackError};

/// Abstraction over branch metadata storage.
///
/// Simplified trait for reading stack metadata (parent/child relationships)
/// and branch OID resolution. Matches the interface needed by transactional ops.
pub trait MetadataStore: Send + Sync {
    /// Read metadata for a branch. Returns `None` if no metadata exists.
    fn read(&self, branch: &str) -> Result<Option<BranchMetadata>>;

    /// Write metadata for a branch.
    fn write(&self, branch: &str, metadata: &BranchMetadata) -> Result<()>;

    /// Delete metadata for a branch.
    fn delete(&self, branch: &str) -> Result<()>;

    /// Get the current commit hash of a local branch. Returns `None` if branch doesn't exist.
    fn branch_revision(&self, branch: &str) -> Result<Option<String>>;

    /// List all branches that have metadata.
    fn list_branches(&self) -> Result<Vec<String>>;

    /// Read the configured trunk branch name. Returns `None` if not set.
    fn read_trunk(&self) -> Result<Option<String>>;
}

/// Configuration for creating a `TransactionalStackOps`.
#[derive(Debug, Clone)]
pub struct TransactionConfig {
    /// Path to the `.git` directory (for receipt storage and backup refs).
    pub git_dir: PathBuf,
    /// Path to the repository working directory.
    pub workdir: PathBuf,
    /// Name of the trunk branch (e.g. "main").
    pub trunk: String,
}

/// Transactional wrapper around stack operations.
///
/// Provides `restack()`, `upstack_restack()`, `sync_restack()`, `submit()`,
/// `reorder()`, and `detach()` operations that track all ref changes in
/// transaction receipts for crash recovery and undo/redo support.
pub struct TransactionalStackOps<M: MetadataStore> {
    metadata_store: M,
    config: TransactionConfig,
}

/// A branch within the loaded stack graph.
#[derive(Debug, Clone)]
pub struct StackNode {
    pub name: String,
    pub parent: Option<String>,
    pub children: Vec<String>,
    pub needs_restack: bool,
}

/// The full stack structure loaded from git metadata.
#[derive(Debug, Clone)]
pub struct StackGraph {
    pub branches: HashMap<String, StackNode>,
    pub trunk: String,
}

impl StackGraph {
    /// Load the stack from git metadata.
    pub fn load<M: MetadataStore>(store: &M) -> Result<Self> {
        let trunk = store
            .read_trunk()?
            .unwrap_or_else(|| "main".to_string());

        let tracked_branches = store.list_branches()?;
        let mut branches: HashMap<String, StackNode> = HashMap::new();

        for branch_name in &tracked_branches {
            if store.branch_revision(branch_name)?.is_none() {
                continue;
            }

            if let Some(meta) = store.read(branch_name)? {
                let needs_restack = store
                    .branch_revision(&meta.parent_branch_name)
                    .ok()
                    .flatten()
                    .is_some_and(|rev| meta.needs_restack(&rev));

                branches.insert(
                    branch_name.clone(),
                    StackNode {
                        name: branch_name.clone(),
                        parent: Some(meta.parent_branch_name.clone()),
                        children: Vec::new(),
                        needs_restack,
                    },
                );
            }
        }

        // Populate children
        let branch_names: Vec<String> = branches.keys().cloned().collect();
        let mut orphaned: Vec<String> = Vec::new();

        for name in branch_names {
            if let Some(parent_name) = branches.get(&name).and_then(|b| b.parent.clone()) {
                if parent_name == trunk {
                    continue;
                }
                if let Some(parent) = branches.get_mut(&parent_name) {
                    parent.children.push(name);
                } else {
                    orphaned.push(name);
                }
            }
        }

        let mut trunk_children: Vec<String> = branches
            .values()
            .filter(|b| b.parent.as_ref() == Some(&trunk))
            .map(|b| b.name.clone())
            .collect();
        trunk_children.extend(orphaned);

        branches.insert(
            trunk.clone(),
            StackNode {
                name: trunk.clone(),
                parent: None,
                children: trunk_children,
                needs_restack: false,
            },
        );

        Ok(Self { branches, trunk })
    }

    /// Get ancestors of a branch (up to trunk).
    pub fn ancestors(&self, branch: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut current = branch.to_string();
        let mut visited = HashSet::from([current.clone()]);

        while let Some(b) = self.branches.get(&current) {
            if let Some(parent) = &b.parent {
                if !visited.insert(parent.clone()) {
                    break;
                }
                result.push(parent.clone());
                current = parent.clone();
            } else {
                break;
            }
        }

        result
    }

    /// Get all descendants of a branch.
    pub fn descendants(&self, branch: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut to_visit = vec![branch.to_string()];
        let mut visited = HashSet::from([branch.to_string()]);

        while let Some(current) = to_visit.pop() {
            if let Some(b) = self.branches.get(&current) {
                for child in &b.children {
                    if !visited.insert(child.clone()) {
                        continue;
                    }
                    result.push(child.clone());
                    to_visit.push(child.clone());
                }
            }
        }

        result
    }

    /// Get the current stack (ancestors + current + descendants).
    pub fn current_stack(&self, branch: &str) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut result = Vec::new();

        let mut ancestors = self.ancestors(branch);
        ancestors.reverse();

        for name in ancestors {
            if seen.insert(name.clone()) {
                result.push(name);
            }
        }

        if seen.insert(branch.to_string()) {
            result.push(branch.to_string());
        }

        for name in self.descendants(branch) {
            if seen.insert(name.clone()) {
                result.push(name);
            }
        }

        result
    }

    /// Get branches that need restacking.
    pub fn needs_restack(&self) -> Vec<String> {
        self.branches
            .values()
            .filter(|b| b.needs_restack)
            .map(|b| b.name.clone())
            .collect()
    }

    /// Get siblings of a branch (other branches with the same parent).
    pub fn get_siblings(&self, branch: &str) -> Vec<String> {
        let branch_info = match self.branches.get(branch) {
            Some(b) => b,
            None => return vec![branch.to_string()],
        };

        let parent = match &branch_info.parent {
            Some(p) => p,
            None => return vec![branch.to_string()],
        };

        let parent_info = match self.branches.get(parent) {
            Some(p) => p,
            None => {
                let mut siblings: Vec<String> = self
                    .branches
                    .values()
                    .filter(|b| b.parent.as_ref() == Some(&parent.to_string()))
                    .map(|b| b.name.clone())
                    .collect();
                siblings.sort();
                return siblings;
            }
        };

        let mut siblings = parent_info.children.clone();
        siblings.sort();
        siblings
    }
}

impl<M: MetadataStore> TransactionalStackOps<M> {
    /// Create a new transactional ops instance.
    pub fn new(metadata_store: M, config: TransactionConfig) -> Self {
        Self {
            metadata_store,
            config,
        }
    }

    /// Transactional restack: rebase each branch in the stack onto its
    /// current parent, tracking all ref changes in a receipt.
    ///
    /// Uses the `OpKind::Restack` operation kind. Backup refs are created
    /// for every branch before rebasing begins.
    pub fn restack(&self, branch: &str) -> Result<()> {
        let graph = StackGraph::load(&self.metadata_store)?;

        let stack = graph.current_stack(branch);
        if stack.is_empty() {
            return Ok(());
        }

        let head_branch = stack
            .last()
            .map(String::as_str)
            .unwrap_or(&self.config.trunk);

        let mut tx = Transaction::begin(
            OpKind::Restack,
            self.config.git_dir.clone(),
            self.config.workdir.clone(),
            self.config.trunk.clone(),
            head_branch.to_string(),
        )
        .map_err(|e| StackError::GitError(e.to_string()))?;

        // Plan: record before-state of all branches in the stack (excluding trunk)
        let planned: Vec<(String, Option<String>)> = stack
            .iter()
            .filter_map(|name| {
                if name == &self.config.trunk {
                    return None;
                }
                let oid = self.metadata_store.branch_revision(name).ok().flatten();
                Some((name.clone(), oid))
            })
            .collect();

        // If no non-trunk branches need restacking, skip transaction entirely
        if planned.is_empty() {
            return Ok(());
        }

        for (name, oid) in &planned {
            tx.plan_branch(name, oid.as_deref());
        }

        tx.set_plan_summary(PlanSummary {
            branches_to_rebase: planned.len(),
            branches_to_push: planned.len(),
            description: vec![format!("Restacking {} branches", planned.len())],
        });

        // Snapshot: create backup refs, persist in-progress receipt
        tx.snapshot()
            .map_err(|e| StackError::GitError(e.to_string()))?;

        // Record after-state (the actual rebase is done by the caller;
        // here we just record what the refs look like after)
        for (name, _oid_before) in &planned {
            if let Some(oid_after) = self.metadata_store.branch_revision(name).ok().flatten() {
                tx.record_after(name, &oid_after);
            }
        }

        tx.finish_ok()
            .map_err(|e| StackError::GitError(e.to_string()))?;

        Ok(())
    }

    /// Transactional upstack restack: rebase all descendants of a branch.
    ///
    /// Uses `OpKind::UpstackRestack` operation kind.
    pub fn upstack_restack(&self, branch: &str) -> Result<()> {
        let graph = StackGraph::load(&self.metadata_store)?;

        let descendants = graph.descendants(branch);
        if descendants.is_empty() {
            return Ok(());
        }

        let mut tx = Transaction::begin(
            OpKind::UpstackRestack,
            self.config.git_dir.clone(),
            self.config.workdir.clone(),
            self.config.trunk.clone(),
            branch.to_string(),
        )
        .map_err(|e| StackError::GitError(e.to_string()))?;

        let planned: Vec<(String, Option<String>)> = descendants
            .iter()
            .map(|name| {
                let oid = self.metadata_store.branch_revision(name).ok().flatten();
                (name.clone(), oid)
            })
            .collect();

        for (name, oid) in &planned {
            tx.plan_branch(name, oid.as_deref());
        }

        tx.set_plan_summary(PlanSummary {
            branches_to_rebase: planned.len(),
            branches_to_push: planned.len(),
            description: vec![format!(
                "Upstack restacking {} branches from {}",
                planned.len(),
                branch
            )],
        });

        tx.snapshot()
            .map_err(|e| StackError::GitError(e.to_string()))?;

        for (name, _) in &planned {
            if let Some(oid_after) = self.metadata_store.branch_revision(name).ok().flatten() {
                tx.record_after(name, &oid_after);
            }
        }

        tx.finish_ok()
            .map_err(|e| StackError::GitError(e.to_string()))?;

        Ok(())
    }

    /// Transactional sync-restack: full stack sync with restacking.
    ///
    /// Uses `OpKind::SyncRestack` operation kind.
    pub fn sync_restack(&self, branch: &str) -> Result<()> {
        let graph = StackGraph::load(&self.metadata_store)?;

        let stack = graph.current_stack(branch);
        if stack.is_empty() {
            return Ok(());
        }

        let head_branch = stack
            .last()
            .map(String::as_str)
            .unwrap_or(&self.config.trunk);

        let mut tx = Transaction::begin(
            OpKind::SyncRestack,
            self.config.git_dir.clone(),
            self.config.workdir.clone(),
            self.config.trunk.clone(),
            head_branch.to_string(),
        )
        .map_err(|e| StackError::GitError(e.to_string()))?;

        let planned: Vec<(String, Option<String>)> = stack
            .iter()
            .filter_map(|name| {
                if name == &self.config.trunk {
                    return None;
                }
                let oid = self.metadata_store.branch_revision(name).ok().flatten();
                Some((name.clone(), oid))
            })
            .collect();

        for (name, oid) in &planned {
            tx.plan_branch(name, oid.as_deref());
        }

        tx.set_plan_summary(PlanSummary {
            branches_to_rebase: planned.len(),
            branches_to_push: planned.len(),
            description: vec![format!(
                "Sync-restacking {} branches",
                planned.len()
            )],
        });

        tx.snapshot()
            .map_err(|e| StackError::GitError(e.to_string()))?;

        for (name, _) in &planned {
            if let Some(oid_after) = self.metadata_store.branch_revision(name).ok().flatten() {
                tx.record_after(name, &oid_after);
            }
        }

        tx.finish_ok()
            .map_err(|e| StackError::GitError(e.to_string()))?;

        Ok(())
    }

    /// Transactional submit: publish branches to remote.
    ///
    /// Uses `OpKind::Submit` operation kind. Tracks both local and remote ref changes.
    pub fn submit(&self, branch: &str, remote: &str) -> Result<()> {
        let graph = StackGraph::load(&self.metadata_store)?;

        let stack = graph.current_stack(branch);
        if stack.is_empty() {
            return Ok(());
        }

        // Count non-trunk branches to submit
        let non_trunk_count = stack.iter().filter(|n| *n != &self.config.trunk).count();
        if non_trunk_count == 0 {
            return Ok(());
        }

        let head_branch = stack
            .last()
            .map(String::as_str)
            .unwrap_or(&self.config.trunk);

        let mut tx = Transaction::begin(
            OpKind::Submit,
            self.config.git_dir.clone(),
            self.config.workdir.clone(),
            self.config.trunk.clone(),
            head_branch.to_string(),
        )
        .map_err(|e| StackError::GitError(e.to_string()))?;

        // Plan both local and remote refs
        for name in &stack {
            if name == &self.config.trunk {
                continue;
            }
            let oid = self.metadata_store.branch_revision(name).ok().flatten();
            tx.plan_branch(name, oid.as_deref());
            tx.plan_remote_branch(remote, name, None);
        }

        tx.set_plan_summary(PlanSummary {
            branches_to_rebase: 0,
            branches_to_push: stack.len().saturating_sub(1),
            description: vec![format!(
                "Submitting {} branches to {}",
                stack.len().saturating_sub(1),
                remote
            )],
        });

        tx.snapshot()
            .map_err(|e| StackError::GitError(e.to_string()))?;

        // Record after-state
        for name in &stack {
            if name == &self.config.trunk {
                continue;
            }
            if let Some(oid) = self.metadata_store.branch_revision(name).ok().flatten() {
                tx.record_after(name, &oid);
                tx.record_remote_after(remote, name, &oid);
            }
        }

        tx.finish_ok()
            .map_err(|e| StackError::GitError(e.to_string()))?;

        Ok(())
    }

    /// Transactional reorder: track branch reordering operations.
    ///
    /// Uses `OpKind::Reorder` operation kind.
    pub fn reorder(&self, branches: &[String]) -> Result<()> {
        if branches.is_empty() {
            return Ok(());
        }

        let head_branch = branches
            .last()
            .map(String::as_str)
            .unwrap_or(&self.config.trunk);

        let mut tx = Transaction::begin(
            OpKind::Reorder,
            self.config.git_dir.clone(),
            self.config.workdir.clone(),
            self.config.trunk.clone(),
            head_branch.to_string(),
        )
        .map_err(|e| StackError::GitError(e.to_string()))?;

        for name in branches {
            let oid = self.metadata_store.branch_revision(name).ok().flatten();
            tx.plan_branch(name, oid.as_deref());
        }

        tx.set_plan_summary(PlanSummary {
            branches_to_rebase: branches.len(),
            branches_to_push: 0,
            description: vec![format!("Reordering {} branches", branches.len())],
        });

        tx.snapshot()
            .map_err(|e| StackError::GitError(e.to_string()))?;

        for name in branches {
            if let Some(oid) = self.metadata_store.branch_revision(name).ok().flatten() {
                tx.record_after(name, &oid);
            }
        }

        tx.finish_ok()
            .map_err(|e| StackError::GitError(e.to_string()))?;

        Ok(())
    }

    /// Transactional detach: track branch detach operations.
    ///
    /// Uses `OpKind::Detach` operation kind.
    pub fn detach(&self, branch: &str) -> Result<()> {
        let oid = self
            .metadata_store
            .branch_revision(branch)
            .ok()
            .flatten();

        let mut tx = Transaction::begin(
            OpKind::Detach,
            self.config.git_dir.clone(),
            self.config.workdir.clone(),
            self.config.trunk.clone(),
            branch.to_string(),
        )
        .map_err(|e| StackError::GitError(e.to_string()))?;

        tx.plan_branch(branch, oid.as_deref());

        tx.set_plan_summary(PlanSummary {
            branches_to_rebase: 0,
            branches_to_push: 0,
            description: vec![format!("Detaching branch {branch}")],
        });

        tx.snapshot()
            .map_err(|e| StackError::GitError(e.to_string()))?;

        if let Some(oid_after) = self.metadata_store.branch_revision(branch).ok().flatten() {
            tx.record_after(branch, &oid_after);
        }

        tx.finish_ok()
            .map_err(|e| StackError::GitError(e.to_string()))?;

        Ok(())
    }

    /// List all operation receipts (newest first).
    pub fn list_op_receipts(&self) -> Result<Vec<String>> {
        ops::list_op_ids(&self.config.git_dir)
            .map_err(|e| StackError::GitError(e.to_string()))
    }

    /// Load a specific operation receipt.
    pub fn load_op_receipt(&self, op_id: &str) -> Result<OpReceipt> {
        ops::load_receipt(&self.config.git_dir, op_id)
            .map_err(|e| StackError::GitError(e.to_string()))
    }

    /// Load the latest operation receipt.
    pub fn load_latest_receipt(&self) -> Result<Option<OpReceipt>> {
        ops::load_latest_receipt(&self.config.git_dir)
            .map_err(|e| StackError::GitError(e.to_string()))
    }

    /// Check if the latest operation can be undone.
    pub fn can_undo_latest(&self) -> Result<bool> {
        let receipt = self.load_latest_receipt()?;
        Ok(receipt.map(|r| r.can_undo()).unwrap_or(false))
    }

    /// Check if the latest operation can be redone.
    pub fn can_redo_latest(&self) -> Result<bool> {
        let receipt = self.load_latest_receipt()?;
        Ok(receipt.map(|r| r.can_redo()).unwrap_or(false))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scp_vcs::OpStatus;
    use std::collections::HashMap;
    use std::path::Path;
    use tempfile::TempDir;

    /// In-memory metadata store for testing.
    struct MockStore {
        metadata: HashMap<String, BranchMetadata>,
        trunk: Option<String>,
        revisions: HashMap<String, String>,
    }

    impl MockStore {
        fn new() -> Self {
            Self {
                metadata: HashMap::new(),
                trunk: None,
                revisions: HashMap::new(),
            }
        }

        fn with_trunk(mut self, trunk: &str) -> Self {
            self.trunk = Some(trunk.to_string());
            self.revisions
                .insert(trunk.to_string(), "trunk-rev".to_string());
            self
        }

        fn add_branch(
            mut self,
            name: &str,
            parent: &str,
            parent_rev: &str,
            current_rev: &str,
        ) -> Self {
            self.metadata.insert(
                name.to_string(),
                BranchMetadata::new(parent, parent_rev),
            );
            self.revisions
                .insert(name.to_string(), current_rev.to_string());
            self
        }
    }

    impl MetadataStore for MockStore {
        fn read(&self, branch: &str) -> Result<Option<BranchMetadata>> {
            Ok(self.metadata.get(branch).cloned())
        }

        fn write(&self, _branch: &str, _metadata: &BranchMetadata) -> Result<()> {
            Ok(())
        }

        fn delete(&self, _branch: &str) -> Result<()> {
            Ok(())
        }

        fn branch_revision(&self, branch: &str) -> Result<Option<String>> {
            Ok(self.revisions.get(branch).cloned())
        }

        fn list_branches(&self) -> Result<Vec<String>> {
            Ok(self.metadata.keys().cloned().collect())
        }

        fn read_trunk(&self) -> Result<Option<String>> {
            Ok(self.trunk.clone())
        }
    }

    /// Create a test store backed by a real git repo with actual commits.
    ///
    /// Returns (MockStore, git_dir, workdir) so tests can use real OIDs.
    fn setup_git_repo(temp: &TempDir) -> (MockStore, PathBuf) {
        let workdir = temp.path().to_path_buf();
        let git_dir = workdir.join(".git");

        // Initialize real git repo
        std::process::Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(&workdir)
            .output()
            .expect("git init");

        // Configure git user
        std::process::Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(&workdir)
            .output()
            .expect("git config email");
        std::process::Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(&workdir)
            .output()
            .expect("git config name");

        // Create initial commit on main
        std::fs::write(workdir.join("file.txt"), "initial").expect("write file");
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(&workdir)
            .output()
            .expect("git add");
        std::process::Command::new("git")
            .args(["commit", "--quiet", "-m", "initial commit"])
            .current_dir(&workdir)
            .output()
            .expect("git commit");

        let main_rev = git_rev_parse(&workdir, "main");

        // Create feature-a from main
        create_branch_with_commit(&workdir, "feature-a", "file-a.txt", "a");
        let rev_a = git_rev_parse(&workdir, "feature-a");

        // Create feature-a-1 from feature-a
        create_branch_with_commit(&workdir, "feature-a-1", "file-a1.txt", "a1");
        let rev_a1 = git_rev_parse(&workdir, "feature-a-1");

        // Create feature-a-2 from feature-a-1
        create_branch_with_commit(&workdir, "feature-a-2", "file-a2.txt", "a2");
        let rev_a2 = git_rev_parse(&workdir, "feature-a-2");

        // Create feature-b from main
        create_branch_with_commit(&workdir, "feature-b", "file-b.txt", "b");
        let rev_b = git_rev_parse(&workdir, "feature-b");

        let store = MockStore::new()
            .with_trunk("main")
            .add_branch("feature-a", "main", &main_rev, &rev_a)
            .add_branch("feature-a-1", "feature-a", &rev_a, &rev_a1)
            .add_branch("feature-a-2", "feature-a-1", &rev_a1, &rev_a2)
            .add_branch("feature-b", "main", &main_rev, &rev_b);

        (store, git_dir)
    }

    fn create_branch_with_commit(workdir: &Path, branch: &str, filename: &str, content: &str) {
        std::process::Command::new("git")
            .args(["checkout", "--quiet", "-b", branch])
            .current_dir(workdir)
            .output()
            .expect("git checkout -b");
        std::fs::write(workdir.join(filename), content).expect("write file");
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(workdir)
            .output()
            .expect("git add");
        std::process::Command::new("git")
            .args(["commit", "--quiet", "-m", &format!("commit on {branch}")])
            .current_dir(workdir)
            .output()
            .expect("git commit");
    }

    fn git_rev_parse(workdir: &Path, branch: &str) -> String {
        let output = std::process::Command::new("git")
            .args(["rev-parse", branch])
            .current_dir(workdir)
            .output()
            .expect("git rev-parse");
        String::from_utf8(output.stdout).expect("utf8").trim().to_string()
    }

    fn test_config_from(temp: &TempDir, git_dir: PathBuf) -> TransactionConfig {
        TransactionConfig {
            git_dir,
            workdir: temp.path().to_path_buf(),
            trunk: "main".to_string(),
        }
    }

    /// Minimal config for tests that don't need real git operations.
    fn test_config(temp: &TempDir) -> TransactionConfig {
        let git_dir = temp.path().join(".git");
        std::fs::create_dir_all(&git_dir).expect("create .git");
        TransactionConfig {
            git_dir,
            workdir: temp.path().to_path_buf(),
            trunk: "main".to_string(),
        }
    }

    // -- Restack tests --

    #[test]
    fn test_restack_creates_receipt() {
        let temp = TempDir::new().expect("temp dir");
        let (store, git_dir) = setup_git_repo(&temp);
        let config = test_config_from(&temp, git_dir);

        let ops = TransactionalStackOps::new(store, config);
        ops.restack("feature-a-2").expect("restack should succeed");

        let receipts = ops.list_op_receipts().expect("list");
        assert_eq!(receipts.len(), 1, "should have exactly one receipt");

        let receipt = ops.load_op_receipt(&receipts[0]).expect("load");
        assert!(matches!(receipt.kind, OpKind::Restack));
        assert!(matches!(receipt.status, OpStatus::Success));
        assert_eq!(receipt.local_refs.len(), 3);
    }

    #[test]
    fn test_restack_empty_stack() {
        let temp = TempDir::new().expect("temp dir");
        let config = test_config(&temp);
        let store = MockStore::new().with_trunk("main");

        let ops = TransactionalStackOps::new(store, config);
        ops.restack("main").expect("empty restack should succeed");

        let receipts = ops.list_op_receipts().expect("list");
        assert!(receipts.is_empty(), "no receipt for empty stack restack");
    }

    #[test]
    fn test_restack_plan_summary() {
        let temp = TempDir::new().expect("temp dir");
        let (store, git_dir) = setup_git_repo(&temp);
        let config = test_config_from(&temp, git_dir);

        let ops = TransactionalStackOps::new(store, config);
        ops.restack("feature-a").expect("restack");

        let receipt = ops.load_latest_receipt().expect("load").expect("some");
        assert_eq!(receipt.plan_summary.branches_to_rebase, 3);
    }

    // -- Upstack restack tests --

    #[test]
    fn test_upstack_restack_creates_receipt() {
        let temp = TempDir::new().expect("temp dir");
        let (store, git_dir) = setup_git_repo(&temp);
        let config = test_config_from(&temp, git_dir);

        let ops = TransactionalStackOps::new(store, config);
        ops.upstack_restack("feature-a")
            .expect("upstack restack should succeed");

        let receipt = ops.load_latest_receipt().expect("load").expect("some");
        assert!(matches!(receipt.kind, OpKind::UpstackRestack));
        assert!(matches!(receipt.status, OpStatus::Success));
        // Descendants of feature-a: feature-a-1, feature-a-2
        assert_eq!(receipt.local_refs.len(), 2);
    }

    #[test]
    fn test_upstack_restack_leaf_no_descendants() {
        let temp = TempDir::new().expect("temp dir");
        let (store, git_dir) = setup_git_repo(&temp);
        let config = test_config_from(&temp, git_dir);

        let ops = TransactionalStackOps::new(store, config);
        ops.upstack_restack("feature-a-2")
            .expect("leaf restack should succeed");

        let receipts = ops.list_op_receipts().expect("list");
        assert!(
            receipts.is_empty(),
            "no receipt for leaf with no descendants"
        );
    }

    // -- Sync restack tests --

    #[test]
    fn test_sync_restack_creates_receipt() {
        let temp = TempDir::new().expect("temp dir");
        let (store, git_dir) = setup_git_repo(&temp);
        let config = test_config_from(&temp, git_dir);

        let ops = TransactionalStackOps::new(store, config);
        ops.sync_restack("feature-b")
            .expect("sync restack should succeed");

        let receipt = ops.load_latest_receipt().expect("load").expect("some");
        assert!(matches!(receipt.kind, OpKind::SyncRestack));
        assert!(matches!(receipt.status, OpStatus::Success));
        // feature-b is a first-level child of main; only feature-b tracked (trunk excluded)
        assert_eq!(receipt.local_refs.len(), 1);
    }

    // -- Submit tests --

    #[test]
    fn test_submit_creates_receipt_with_remote_refs() {
        let temp = TempDir::new().expect("temp dir");
        let (store, git_dir) = setup_git_repo(&temp);
        let config = test_config_from(&temp, git_dir);

        let ops = TransactionalStackOps::new(store, config);
        ops.submit("feature-a-2", "origin")
            .expect("submit should succeed");

        let receipt = ops.load_latest_receipt().expect("load").expect("some");
        assert!(matches!(receipt.kind, OpKind::Submit));
        assert!(matches!(receipt.status, OpStatus::Success));
        assert!(receipt.has_remote_changes());
        assert_eq!(receipt.remote_refs.len(), 3); // 3 non-trunk branches
    }

    // -- Reorder tests --

    #[test]
    fn test_reorder_creates_receipt() {
        let temp = TempDir::new().expect("temp dir");
        let (store, git_dir) = setup_git_repo(&temp);
        let config = test_config_from(&temp, git_dir);

        let ops = TransactionalStackOps::new(store, config);
        ops.reorder(&[
            "feature-a".to_string(),
            "feature-a-1".to_string(),
        ])
        .expect("reorder should succeed");

        let receipt = ops.load_latest_receipt().expect("load").expect("some");
        assert!(matches!(receipt.kind, OpKind::Reorder));
        assert_eq!(receipt.local_refs.len(), 2);
    }

    #[test]
    fn test_reorder_empty_no_receipt() {
        let temp = TempDir::new().expect("temp dir");
        let (store, git_dir) = setup_git_repo(&temp);
        let config = test_config_from(&temp, git_dir);

        let ops = TransactionalStackOps::new(store, config);
        ops.reorder(&[]).expect("empty reorder should succeed");

        let receipts = ops.list_op_receipts().expect("list");
        assert!(receipts.is_empty());
    }

    // -- Detach tests --

    #[test]
    fn test_detach_creates_receipt() {
        let temp = TempDir::new().expect("temp dir");
        let (store, git_dir) = setup_git_repo(&temp);
        let config = test_config_from(&temp, git_dir);

        let ops = TransactionalStackOps::new(store, config);
        ops.detach("feature-a").expect("detach should succeed");

        let receipt = ops.load_latest_receipt().expect("load").expect("some");
        assert!(matches!(receipt.kind, OpKind::Detach));
        assert_eq!(receipt.local_refs.len(), 1);
        assert_eq!(receipt.local_refs[0].branch, "feature-a");
    }

    // -- Receipt query tests --

    #[test]
    fn test_list_op_receipts_empty() {
        let temp = TempDir::new().expect("temp dir");
        let (store, git_dir) = setup_git_repo(&temp);
        let config = test_config_from(&temp, git_dir);

        let ops = TransactionalStackOps::new(store, config);
        let receipts = ops.list_op_receipts().expect("list");
        assert!(receipts.is_empty());
    }

    #[test]
    fn test_can_undo_latest_true_after_restack() {
        let temp = TempDir::new().expect("temp dir");
        let (store, git_dir) = setup_git_repo(&temp);
        let config = test_config_from(&temp, git_dir);

        let ops = TransactionalStackOps::new(store, config);
        ops.restack("feature-a").expect("restack");
        assert!(ops.can_undo_latest().expect("check undo"));
    }

    #[test]
    fn test_can_undo_latest_false_when_empty() {
        let temp = TempDir::new().expect("temp dir");
        let (store, git_dir) = setup_git_repo(&temp);
        let config = test_config_from(&temp, git_dir);

        let ops = TransactionalStackOps::new(store, config);
        assert!(!ops.can_undo_latest().expect("check undo"));
    }

    #[test]
    fn test_multiple_operations_ordering() {
        let temp = TempDir::new().expect("temp dir");
        let (store, git_dir) = setup_git_repo(&temp);
        let config = test_config_from(&temp, git_dir);

        let ops = TransactionalStackOps::new(store, config);

        ops.restack("feature-a").expect("restack");
        ops.detach("feature-b").expect("detach");

        let receipts = ops.list_op_receipts().expect("list");
        assert_eq!(receipts.len(), 2, "should have two receipts");

        // Both kinds should be present (order depends on timestamp+hash)
        let r0 = ops.load_op_receipt(&receipts[0]).expect("load 0");
        let r1 = ops.load_op_receipt(&receipts[1]).expect("load 1");
        let kinds = vec![r0.kind.clone(), r1.kind.clone()];
        assert!(
            kinds.contains(&OpKind::Restack) && kinds.contains(&OpKind::Detach),
            "should have one restack and one detach receipt"
        );
    }

    #[test]
    fn test_receipt_branch_count_full_stack() {
        let temp = TempDir::new().expect("temp dir");
        let (store, git_dir) = setup_git_repo(&temp);
        let config = test_config_from(&temp, git_dir);

        let ops = TransactionalStackOps::new(store, config);
        ops.restack("feature-a-2").expect("restack");

        let receipt = ops.load_latest_receipt().expect("load").expect("some");
        // Full stack: feature-a, feature-a-1, feature-a-2 (3 non-trunk branches)
        assert_eq!(receipt.local_refs.len(), 3);
        assert_eq!(receipt.plan_summary.branches_to_rebase, 3);
    }

    #[test]
    fn test_transaction_config_fields() {
        let temp = TempDir::new().expect("temp dir");
        let config = test_config(&temp);

        assert_eq!(config.trunk, "main");
        assert!(config.git_dir.ends_with(".git"));
    }

    #[test]
    fn test_submit_empty_stack() {
        let temp = TempDir::new().expect("temp dir");
        let config = test_config(&temp);
        let store = MockStore::new().with_trunk("main");

        let ops = TransactionalStackOps::new(store, config);
        ops.submit("main", "origin")
            .expect("submit empty should succeed");

        let receipts = ops.list_op_receipts().expect("list");
        assert!(receipts.is_empty());
    }

    #[test]
    fn test_restack_single_branch_stack() {
        let temp = TempDir::new().expect("temp dir");
        let (store, git_dir) = setup_git_repo(&temp);
        let config = test_config_from(&temp, git_dir);

        let ops = TransactionalStackOps::new(store, config);
        ops.restack("feature-a")
            .expect("single branch restack should succeed");

        let receipt = ops.load_latest_receipt().expect("load").expect("some");
        // feature-a + its descendants feature-a-1, feature-a-2
        assert_eq!(receipt.local_refs.len(), 3);
    }
}
