//! Transaction wrapper for safe history-rewriting operations.
//!
//! Provides a builder-style API for atomic multi-step git operations:
//!
//! ```ignore
//! let mut tx = Transaction::begin(OpKind::Restack, &git_dir, "/repo")?;
//! tx.plan_branch("feature/foo", Some("abc123"));
//! tx.snapshot()?;  // Creates backup refs and writes in-progress receipt
//!
//! // ... do the actual work ...
//!
//! tx.record_after("feature/foo", "def456")?;
//! tx.finish_ok()?;  // Or tx.finish_err("message", ...);
//! ```
//!
//! # Design
//!
//! The Transaction separates concerns into Data (receipt state), Calc (pure
//! validation), and Actions (I/O). The `Drop` guard ensures unfinished
//! transactions are persisted as failed.

use crate::domain::entities::ops::{OpKind, OpReceipt, PlanSummary};
use crate::error::{Result, VcsError};
use std::path::{Path, PathBuf};

/// A transaction wrapper for history-rewriting operations.
///
/// Tracks before/after state of all refs, creates backup refs on snapshot,
/// and persists receipts to disk. The `Drop` implementation marks any
/// snapshotted-but-not-finished transaction as failed.
pub struct Transaction {
    receipt: OpReceipt,
    git_dir: PathBuf,
    workdir: PathBuf,
    /// Whether `snapshot()` has been called.
    snapshotted: bool,
    /// Whether the transaction has been finished.
    finished: bool,
}

impl Transaction {
    /// Begin a new transaction.
    ///
    /// `git_dir` is the `.git` directory path (for receipt storage).
    /// `workdir` is the repository working directory.
    /// `trunk` is the trunk branch name.
    /// `head_branch` is the currently checked out branch.
    pub fn begin(
        kind: OpKind,
        git_dir: PathBuf,
        workdir: PathBuf,
        trunk: String,
        head_branch: String,
    ) -> Result<Self> {
        let op_id = crate::infrastructure::ops::generate_op_id();
        let started_at = chrono::Utc::now().to_rfc3339();

        let receipt = OpReceipt::new(
            op_id,
            kind,
            workdir.to_string_lossy().to_string(),
            trunk,
            head_branch,
            started_at,
        );

        Ok(Self {
            receipt,
            git_dir,
            workdir,
            snapshotted: false,
            finished: false,
        })
    }

    /// Get the operation ID.
    pub fn op_id(&self) -> &str {
        &self.receipt.op_id
    }

    /// Get the operation kind.
    pub fn kind(&self) -> &OpKind {
        &self.receipt.kind
    }

    /// Check if the transaction has been snapshotted.
    pub fn is_snapshotted(&self) -> bool {
        self.snapshotted
    }

    /// Plan a local branch to be modified.
    pub fn plan_branch(&mut self, branch: &str, oid_before: Option<&str>) {
        self.receipt.add_local_ref(branch, oid_before);
    }

    /// Plan multiple local branches to be modified.
    pub fn plan_branches(&mut self, branches: &[(String, Option<String>)]) {
        for (branch, oid) in branches {
            self.receipt.add_local_ref(branch, oid.as_deref());
        }
    }

    /// Plan a remote ref to be modified (for submit).
    pub fn plan_remote_branch(&mut self, remote: &str, branch: &str, oid_before: Option<&str>) {
        self.receipt.add_remote_ref(remote, branch, oid_before);
    }

    /// Set the plan summary.
    pub fn set_plan_summary(&mut self, summary: PlanSummary) {
        self.receipt.plan_summary = summary;
    }

    /// Set whether the operation should auto-stash dirty target worktrees.
    pub fn set_auto_stash_pop(&mut self, auto_stash_pop: bool) {
        self.receipt.auto_stash_pop = auto_stash_pop;
    }

    /// Create backup refs and write the in-progress receipt.
    ///
    /// This is the point of no return - after snapshotting, the `Drop` guard
    /// will persist a failure receipt if `finish_ok` or `finish_err` isn't called.
    pub fn snapshot(&mut self) -> Result<()> {
        if self.snapshotted {
            return Ok(());
        }

        // Create backup refs for all planned branches with known OIDs
        for entry in &self.receipt.local_refs {
            if let Some(oid) = &entry.oid_before {
                let ref_name =
                    crate::infrastructure::ops::backup_ref_name(&self.receipt.op_id, &entry.branch);
                // Use git update-ref to create backup ref
                let workdir = self.workdir.clone();
                create_backup_ref(&workdir, &ref_name, oid)?;
            }
        }

        // Persist the in-progress receipt
        crate::infrastructure::ops::save_receipt(&self.git_dir, &self.receipt)?;

        self.snapshotted = true;
        Ok(())
    }

    /// Record the after-OID for a branch.
    pub fn record_after(&mut self, branch: &str, oid_after: &str) {
        self.receipt.update_local_ref_after(branch, oid_after);
    }

    /// Record after-OIDs for all planned branches that have a current OID.
    ///
    /// `resolve_branch_oid` is a function that, given a branch name, returns
    /// its current OID. This keeps I/O out of the transaction itself.
    pub fn record_all_after(
        &mut self,
        resolve_branch_oid: impl Fn(&str) -> Option<String>,
    ) {
        let branches: Vec<String> = self
            .receipt
            .local_refs
            .iter()
            .map(|r| r.branch.clone())
            .collect();

        for branch in branches {
            if let Some(oid) = resolve_branch_oid(&branch) {
                self.receipt.update_local_ref_after(&branch, &oid);
            }
        }
    }

    /// Record the after-OID for a remote branch (the local OID that was pushed).
    pub fn record_remote_after(&mut self, remote: &str, branch: &str, local_oid: &str) {
        self.receipt
            .update_remote_ref_after(remote, branch, local_oid);
    }

    /// Finish the transaction successfully.
    pub fn finish_ok(mut self) -> Result<()> {
        let finished_at = chrono::Utc::now().to_rfc3339();
        self.receipt.mark_success(finished_at);
        crate::infrastructure::ops::save_receipt(&self.git_dir, &self.receipt)?;
        self.finished = true;
        Ok(())
    }

    /// Finish the transaction with an error.
    pub fn finish_err(
        mut self,
        message: &str,
        failed_step: Option<&str>,
        failed_branch: Option<&str>,
    ) -> Result<()> {
        let finished_at = chrono::Utc::now().to_rfc3339();
        self.receipt
            .mark_failed(message, failed_step, failed_branch, finished_at);
        crate::infrastructure::ops::save_receipt(&self.git_dir, &self.receipt)?;
        self.finished = true;
        Ok(())
    }
}

impl Drop for Transaction {
    fn drop(&mut self) {
        // If snapshotted but not finished, mark as failed
        if self.snapshotted && !self.finished {
            let finished_at = chrono::Utc::now().to_rfc3339();
            self.receipt.mark_failed(
                "Transaction dropped without finishing",
                None,
                None,
                finished_at,
            );
            // Best-effort persist - ignore errors in drop
            let _ = crate::infrastructure::ops::save_receipt(&self.git_dir, &self.receipt);
        }
    }
}

/// Create a backup ref via git update-ref.
///
/// This is the only place we shell out to git in the transaction module.
fn create_backup_ref(workdir: &Path, ref_name: &str, oid: &str) -> Result<()> {
    let status = std::process::Command::new("git")
        .args(["update-ref", ref_name, oid])
        .current_dir(workdir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(VcsError::Io)?;

    if !status.success() {
        return Err(VcsError::Unimplemented(format!(
            "Failed to create backup ref {ref_name} -> {oid}"
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::ops::OpStatus;
    use tempfile::TempDir;

    fn test_dirs() -> (TempDir, PathBuf, PathBuf) {
        let temp = TempDir::new().expect("temp dir");
        let git_dir = temp.path().join(".git");
        let workdir = temp.path().to_path_buf();
        std::fs::create_dir_all(&git_dir).expect("create .git");
        (temp, git_dir, workdir)
    }

    #[test]
    fn transaction_begin_creates_receipt() {
        let (_temp, git_dir, workdir) = test_dirs();
        let tx = Transaction::begin(
            OpKind::Restack,
            git_dir,
            workdir,
            "main".to_string(),
            "feature".to_string(),
        )
        .expect("begin");

        assert!(!tx.op_id().is_empty());
        assert!(matches!(tx.kind(), OpKind::Restack));
        assert!(!tx.is_snapshotted());
    }

    #[test]
    fn transaction_plan_branch() {
        let (_temp, git_dir, workdir) = test_dirs();
        let mut tx = Transaction::begin(
            OpKind::Restack,
            git_dir,
            workdir,
            "main".to_string(),
            "feature".to_string(),
        )
        .expect("begin");

        tx.plan_branch("feature/foo", Some("abc123"));
        assert_eq!(tx.receipt.local_refs.len(), 1);
        assert_eq!(tx.receipt.local_refs[0].branch, "feature/foo");
    }

    #[test]
    fn transaction_plan_multiple_branches() {
        let (_temp, git_dir, workdir) = test_dirs();
        let mut tx = Transaction::begin(
            OpKind::Restack,
            git_dir,
            workdir,
            "main".to_string(),
            "main".to_string(),
        )
        .expect("begin");

        tx.plan_branches(&[
            ("a".to_string(), Some("111".to_string())),
            ("b".to_string(), None),
        ]);
        assert_eq!(tx.receipt.local_refs.len(), 2);
    }

    #[test]
    fn transaction_set_plan_summary() {
        let (_temp, git_dir, workdir) = test_dirs();
        let mut tx = Transaction::begin(
            OpKind::Restack,
            git_dir,
            workdir,
            "main".to_string(),
            "main".to_string(),
        )
        .expect("begin");

        tx.set_plan_summary(PlanSummary {
            branches_to_rebase: 3,
            branches_to_push: 2,
            description: vec!["Rebasing upstack".to_string()],
        });

        assert_eq!(tx.receipt.plan_summary.branches_to_rebase, 3);
    }

    #[test]
    fn transaction_set_auto_stash_pop() {
        let (_temp, git_dir, workdir) = test_dirs();
        let mut tx = Transaction::begin(
            OpKind::Restack,
            git_dir,
            workdir,
            "main".to_string(),
            "main".to_string(),
        )
        .expect("begin");

        assert!(!tx.receipt.auto_stash_pop);
        tx.set_auto_stash_pop(true);
        assert!(tx.receipt.auto_stash_pop);
    }

    #[test]
    fn transaction_record_after() {
        let (_temp, git_dir, workdir) = test_dirs();
        let mut tx = Transaction::begin(
            OpKind::Restack,
            git_dir,
            workdir,
            "main".to_string(),
            "feature".to_string(),
        )
        .expect("begin");

        tx.plan_branch("feature", Some("abc"));
        tx.record_after("feature", "def");

        assert_eq!(
            tx.receipt.local_refs[0].oid_after,
            Some("def".to_string())
        );
    }

    #[test]
    fn transaction_record_all_after() {
        let (_temp, git_dir, workdir) = test_dirs();
        let mut tx = Transaction::begin(
            OpKind::Restack,
            git_dir,
            workdir,
            "main".to_string(),
            "main".to_string(),
        )
        .expect("begin");

        tx.plan_branch("a", Some("111"));
        tx.plan_branch("b", Some("222"));

        tx.record_all_after(|branch| match branch {
            "a" => Some("aaa".to_string()),
            "b" => Some("bbb".to_string()),
            _ => None,
        });

        assert_eq!(tx.receipt.local_refs[0].oid_after, Some("aaa".to_string()));
        assert_eq!(tx.receipt.local_refs[1].oid_after, Some("bbb".to_string()));
    }

    #[test]
    fn transaction_record_remote_after() {
        let (_temp, git_dir, workdir) = test_dirs();
        let mut tx = Transaction::begin(
            OpKind::Submit,
            git_dir,
            workdir,
            "main".to_string(),
            "feature".to_string(),
        )
        .expect("begin");

        tx.plan_remote_branch("origin", "feature", Some("abc"));
        tx.record_remote_after("origin", "feature", "def");

        assert_eq!(
            tx.receipt.remote_refs[0].oid_after,
            Some("def".to_string())
        );
    }

    #[test]
    fn transaction_finish_ok() {
        let (_temp, git_dir, workdir) = test_dirs();

        let op_id = {
            let mut tx = Transaction::begin(
                OpKind::Restack,
                git_dir.clone(),
                workdir,
                "main".to_string(),
                "feature".to_string(),
            )
            .expect("begin");

            let id = tx.op_id().to_string();
            // Use None for oid_before to skip backup ref creation (no real git repo)
            tx.plan_branch("feature", None);
            tx.snapshot().expect("snapshot");
            tx.record_after("feature", "def");
            tx.finish_ok().expect("finish ok");
            id
        };

        // Load receipt from disk and verify
        let receipt =
            crate::infrastructure::ops::load_receipt(&git_dir, &op_id).expect("load");
        assert!(matches!(receipt.status, OpStatus::Success));
    }

    #[test]
    fn transaction_finish_ok_receipt_is_success() {
        let (_temp, git_dir, workdir) = test_dirs();
        let op_id;

        {
            let mut tx = Transaction::begin(
                OpKind::Restack,
                git_dir.clone(),
                workdir,
                "main".to_string(),
                "feature".to_string(),
            )
            .expect("begin");

            op_id = tx.op_id().to_string();
            tx.plan_branch("feature", None);
            tx.snapshot().expect("snapshot");
            tx.finish_ok().expect("finish");
        }

        let receipt = crate::infrastructure::ops::load_receipt(&git_dir, &op_id).expect("load");
        assert!(matches!(receipt.status, OpStatus::Success));
    }

    #[test]
    fn transaction_finish_err() {
        let (_temp, git_dir, workdir) = test_dirs();
        let op_id;

        {
            let mut tx = Transaction::begin(
                OpKind::Restack,
                git_dir.clone(),
                workdir,
                "main".to_string(),
                "feature".to_string(),
            )
            .expect("begin");

            op_id = tx.op_id().to_string();
            tx.plan_branch("feature", None);
            tx.snapshot().expect("snapshot");
            tx.finish_err("Conflict", Some("rebase"), Some("feature"))
                .expect("finish err");
        }

        let receipt = crate::infrastructure::ops::load_receipt(&git_dir, &op_id).expect("load");
        assert!(matches!(receipt.status, OpStatus::Failed));
        let error = receipt.error.as_ref().expect("has error");
        assert_eq!(error.message, "Conflict");
        assert_eq!(error.failed_step, Some("rebase".to_string()));
    }

    #[test]
    fn transaction_drop_without_finish_marks_failed() {
        let (_temp, git_dir, workdir) = test_dirs();
        let op_id;

        {
            let mut tx = Transaction::begin(
                OpKind::Restack,
                git_dir.clone(),
                workdir,
                "main".to_string(),
                "feature".to_string(),
            )
            .expect("begin");

            op_id = tx.op_id().to_string();
            tx.plan_branch("feature", None);
            tx.snapshot().expect("snapshot");
            // Drop without finish
        }

        let receipt = crate::infrastructure::ops::load_receipt(&git_dir, &op_id).expect("load");
        assert!(matches!(receipt.status, OpStatus::Failed));
        assert!(receipt
            .error
            .as_ref()
            .is_some_and(|e| e.message.contains("dropped without finishing")));
    }

    #[test]
    fn transaction_snapshot_idempotent() {
        let (_temp, git_dir, workdir) = test_dirs();
        let mut tx = Transaction::begin(
            OpKind::Restack,
            git_dir,
            workdir,
            "main".to_string(),
            "main".to_string(),
        )
        .expect("begin");

        tx.plan_branch("feature", None);
        tx.snapshot().expect("first snapshot");
        tx.snapshot().expect("second snapshot - should be idempotent");
        assert!(tx.is_snapshotted());
    }

    #[test]
    fn transaction_no_drop_guard_without_snapshot() {
        let (_temp, git_dir, workdir) = test_dirs();
        let op_id;

        {
            let mut tx = Transaction::begin(
                OpKind::Restack,
                git_dir.clone(),
                workdir,
                "main".to_string(),
                "feature".to_string(),
            )
            .expect("begin");

            op_id = tx.op_id().to_string();
            tx.plan_branch("feature", Some("abc"));
            // Snapshot NOT called, so drop should NOT write a failure receipt
        }

        // No receipt should exist (no snapshot = no persistence)
        let result = crate::infrastructure::ops::load_receipt(&git_dir, &op_id);
        assert!(result.is_err());
    }

    #[test]
    fn transaction_plan_remote_branch() {
        let (_temp, git_dir, workdir) = test_dirs();
        let mut tx = Transaction::begin(
            OpKind::Submit,
            git_dir,
            workdir,
            "main".to_string(),
            "feature".to_string(),
        )
        .expect("begin");

        tx.plan_remote_branch("origin", "feature", Some("abc123"));
        assert_eq!(tx.receipt.remote_refs.len(), 1);
        assert_eq!(tx.receipt.remote_refs[0].remote, "origin");
    }
}
