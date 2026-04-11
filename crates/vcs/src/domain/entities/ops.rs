//! Operations domain entities - transactional operation tracking.
//!
//! Pure data types for tracking VCS operations with undo/redo support.
//! Receipts are persisted as JSON under `.git/stax/ops/<op-id>.json`.
//! Backup refs live under `refs/stax/backups/<op-id>/`.

use serde::{Deserialize, Serialize};

/// Status of an operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OpStatus {
    InProgress,
    Success,
    Failed,
    Undone,
}

/// Kind of operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OpKind {
    Restack,
    UpstackRestack,
    SyncRestack,
    Submit,
    Reorder,
    Split,
    MergeWhenReady,
    Detach,
    Fix,
    Cascade,
}

impl OpKind {
    /// Human-readable name for display.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Restack => "restack",
            Self::UpstackRestack => "upstack restack",
            Self::SyncRestack => "sync --restack",
            Self::Submit => "submit",
            Self::Reorder => "reorder",
            Self::Split => "split",
            Self::MergeWhenReady => "merge-when-ready",
            Self::Detach => "detach",
            Self::Fix => "stack fix",
            Self::Cascade => "cascade",
        }
    }
}

/// Information about a local ref that was modified.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalRefEntry {
    /// Branch name (without refs/heads/).
    pub branch: String,
    /// Full ref name (e.g., `refs/heads/feature/foo`).
    pub refname: String,
    /// Whether the ref existed before the operation.
    pub existed_before: bool,
    /// OID before the operation (`None` if didn't exist).
    pub oid_before: Option<String>,
    /// OID after the operation (filled in on success).
    pub oid_after: Option<String>,
}

/// Information about a remote ref that was modified (for submit).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteRefEntry {
    /// Remote name (e.g., "origin").
    pub remote: String,
    /// Branch name.
    pub branch: String,
    /// Full remote ref name (e.g., `refs/remotes/origin/feature/foo`).
    pub remote_refname: String,
    /// OID on remote before push (`None` if didn't exist).
    pub oid_before: Option<String>,
    /// OID pushed (the local OID that was force-pushed).
    pub oid_after: Option<String>,
}

/// Error information for failed operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpError {
    /// Human-readable error message.
    pub message: String,
    /// Which step failed (e.g., "rebase").
    pub failed_step: Option<String>,
    /// Which branch was being processed when failure occurred.
    pub failed_branch: Option<String>,
}

/// Plan summary for display purposes.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlanSummary {
    /// Number of branches to rebase.
    pub branches_to_rebase: usize,
    /// Number of branches to force-push.
    pub branches_to_push: usize,
    /// Human-readable description bullets.
    pub description: Vec<String>,
}

/// Operation receipt - persisted to `.git/stax/ops/<op-id>.json`.
///
/// Tracks before/after state of all refs modified by an operation,
/// enabling undo/redo and crash recovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpReceipt {
    /// Unique operation ID (timestamp + random suffix).
    pub op_id: String,
    /// Kind of operation.
    pub kind: OpKind,
    /// When operation started (ISO 8601).
    pub started_at: String,
    /// When operation finished (ISO 8601), `None` if still in progress.
    pub finished_at: Option<String>,
    /// Current status.
    pub status: OpStatus,
    /// Repository working directory (for verification).
    pub repo_workdir: String,
    /// Trunk branch name.
    pub trunk: String,
    /// Whether the operation auto-stashed dirty target worktrees.
    pub auto_stash_pop: bool,
    /// Branch that was checked out when operation started.
    pub head_branch_before: String,
    /// Local refs that were/will be modified.
    pub local_refs: Vec<LocalRefEntry>,
    /// Remote refs that were/will be modified (for submit).
    pub remote_refs: Vec<RemoteRefEntry>,
    /// Plan summary for display.
    pub plan_summary: PlanSummary,
    /// Error information if failed.
    pub error: Option<OpError>,
}

impl OpReceipt {
    /// Create a new receipt for an operation that's about to start.
    pub fn new(
        op_id: String,
        kind: OpKind,
        repo_workdir: String,
        trunk: String,
        head_branch_before: String,
        started_at: String,
    ) -> Self {
        Self {
            op_id,
            kind,
            started_at,
            finished_at: None,
            status: OpStatus::InProgress,
            repo_workdir,
            trunk,
            auto_stash_pop: false,
            head_branch_before,
            local_refs: Vec::new(),
            remote_refs: Vec::new(),
            plan_summary: PlanSummary::default(),
            error: None,
        }
    }

    /// Add a local ref to track.
    pub fn add_local_ref(&mut self, branch: &str, oid_before: Option<&str>) {
        self.local_refs.push(LocalRefEntry {
            branch: branch.to_string(),
            refname: format!("refs/heads/{branch}"),
            existed_before: oid_before.is_some(),
            oid_before: oid_before.map(String::from),
            oid_after: None,
        });
    }

    /// Add a remote ref to track.
    pub fn add_remote_ref(&mut self, remote: &str, branch: &str, oid_before: Option<&str>) {
        self.remote_refs.push(RemoteRefEntry {
            remote: remote.to_string(),
            branch: branch.to_string(),
            remote_refname: format!("refs/remotes/{remote}/{branch}"),
            oid_before: oid_before.map(String::from),
            oid_after: None,
        });
    }

    /// Update the after-OID for a local ref.
    pub fn update_local_ref_after(&mut self, branch: &str, oid_after: &str) {
        if let Some(entry) = self.local_refs.iter_mut().find(|e| e.branch == branch) {
            entry.oid_after = Some(oid_after.to_string());
        }
    }

    /// Update the after-OID for a remote ref.
    pub fn update_remote_ref_after(&mut self, remote: &str, branch: &str, oid_after: &str) {
        if let Some(entry) = self
            .remote_refs
            .iter_mut()
            .find(|e| e.remote == remote && e.branch == branch)
        {
            entry.oid_after = Some(oid_after.to_string());
        }
    }

    /// Mark operation as successful.
    pub fn mark_success(&mut self, finished_at: String) {
        self.status = OpStatus::Success;
        self.finished_at = Some(finished_at);
    }

    /// Mark operation as failed.
    pub fn mark_failed(
        &mut self,
        message: &str,
        failed_step: Option<&str>,
        failed_branch: Option<&str>,
        finished_at: String,
    ) {
        self.status = OpStatus::Failed;
        self.finished_at = Some(finished_at);
        self.error = Some(OpError {
            message: message.to_string(),
            failed_step: failed_step.map(String::from),
            failed_branch: failed_branch.map(String::from),
        });
    }

    /// Check if this receipt can be undone.
    pub fn can_undo(&self) -> bool {
        matches!(self.status, OpStatus::Success)
            && self.local_refs.iter().any(|r| r.oid_before.is_some())
    }

    /// Check if this receipt can be redone.
    pub fn can_redo(&self) -> bool {
        matches!(self.status, OpStatus::Undone)
            && self.local_refs.iter().any(|r| r.oid_after.is_some())
    }

    /// Mark operation as undone.
    pub fn mark_undone(&mut self, finished_at: String) {
        self.status = OpStatus::Undone;
        self.finished_at = Some(finished_at);
    }

    /// Check if this receipt has remote changes.
    pub fn has_remote_changes(&self) -> bool {
        !self.remote_refs.is_empty()
    }

    /// Count branches that were actually modified.
    pub fn modified_branch_count(&self) -> usize {
        self.local_refs
            .iter()
            .filter(|r| r.oid_before != r.oid_after)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now_iso() -> String {
        chrono::Utc::now().to_rfc3339()
    }

    fn new_receipt(kind: OpKind) -> OpReceipt {
        OpReceipt::new(
            "20251229T120500Z-abc123".to_string(),
            kind,
            "/tmp/repo".to_string(),
            "main".to_string(),
            "feature/foo".to_string(),
            now_iso(),
        )
    }

    // -- OpKind display_name --

    #[test]
    fn op_kind_display_name_all_variants() {
        assert_eq!(OpKind::Restack.display_name(), "restack");
        assert_eq!(OpKind::UpstackRestack.display_name(), "upstack restack");
        assert_eq!(OpKind::SyncRestack.display_name(), "sync --restack");
        assert_eq!(OpKind::Submit.display_name(), "submit");
        assert_eq!(OpKind::Reorder.display_name(), "reorder");
        assert_eq!(OpKind::Split.display_name(), "split");
        assert_eq!(OpKind::MergeWhenReady.display_name(), "merge-when-ready");
        assert_eq!(OpKind::Detach.display_name(), "detach");
        assert_eq!(OpKind::Fix.display_name(), "stack fix");
        assert_eq!(OpKind::Cascade.display_name(), "cascade");
    }

    // -- OpStatus serialization --

    #[test]
    fn op_status_serde_roundtrip() {
        for status in [
            OpStatus::InProgress,
            OpStatus::Success,
            OpStatus::Failed,
            OpStatus::Undone,
        ] {
            let json = serde_json::to_string(&status).expect("serialize");
            let loaded: OpStatus = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(status, loaded);
        }
    }

    #[test]
    fn op_status_serialization_values() {
        assert_eq!(
            serde_json::to_string(&OpStatus::InProgress).expect("s"),
            "\"in_progress\""
        );
        assert_eq!(
            serde_json::to_string(&OpStatus::Success).expect("s"),
            "\"success\""
        );
        assert_eq!(
            serde_json::to_string(&OpStatus::Failed).expect("s"),
            "\"failed\""
        );
        assert_eq!(
            serde_json::to_string(&OpStatus::Undone).expect("s"),
            "\"undone\""
        );
    }

    // -- OpKind serialization --

    #[test]
    fn op_kind_serde_roundtrip() {
        for kind in [
            OpKind::Restack,
            OpKind::UpstackRestack,
            OpKind::SyncRestack,
            OpKind::Submit,
            OpKind::Reorder,
            OpKind::Split,
            OpKind::MergeWhenReady,
            OpKind::Detach,
            OpKind::Fix,
            OpKind::Cascade,
        ] {
            let json = serde_json::to_string(&kind).expect("serialize");
            let loaded: OpKind = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(kind, loaded);
        }
    }

    // -- OpReceipt construction --

    #[test]
    fn receipt_new_defaults() {
        let receipt = new_receipt(OpKind::Submit);
        assert_eq!(receipt.op_id, "20251229T120500Z-abc123");
        assert!(matches!(receipt.kind, OpKind::Submit));
        assert!(matches!(receipt.status, OpStatus::InProgress));
        assert!(receipt.finished_at.is_none());
        assert!(!receipt.auto_stash_pop);
        assert!(receipt.local_refs.is_empty());
        assert!(receipt.remote_refs.is_empty());
        assert!(receipt.error.is_none());
    }

    // -- Local ref operations --

    #[test]
    fn add_local_ref_existing() {
        let mut receipt = new_receipt(OpKind::Restack);
        receipt.add_local_ref("feature/foo", Some("abc123"));

        assert_eq!(receipt.local_refs.len(), 1);
        assert_eq!(receipt.local_refs[0].branch, "feature/foo");
        assert_eq!(receipt.local_refs[0].refname, "refs/heads/feature/foo");
        assert!(receipt.local_refs[0].existed_before);
        assert_eq!(receipt.local_refs[0].oid_before, Some("abc123".to_string()));
    }

    #[test]
    fn add_local_ref_new_branch() {
        let mut receipt = new_receipt(OpKind::Restack);
        receipt.add_local_ref("new-branch", None);

        assert_eq!(receipt.local_refs.len(), 1);
        assert!(!receipt.local_refs[0].existed_before);
        assert!(receipt.local_refs[0].oid_before.is_none());
    }

    #[test]
    fn update_local_ref_after() {
        let mut receipt = new_receipt(OpKind::Restack);
        receipt.add_local_ref("feature/foo", Some("abc123"));
        receipt.update_local_ref_after("feature/foo", "def456");

        assert_eq!(receipt.local_refs[0].oid_after, Some("def456".to_string()));
    }

    #[test]
    fn update_local_ref_after_nonexistent_branch_noop() {
        let mut receipt = new_receipt(OpKind::Restack);
        receipt.update_local_ref_after("nonexistent", "abc");
        assert!(receipt.local_refs.is_empty());
    }

    // -- Remote ref operations --

    #[test]
    fn add_remote_ref() {
        let mut receipt = new_receipt(OpKind::Submit);
        receipt.add_remote_ref("origin", "feature/foo", Some("abc123"));

        assert_eq!(receipt.remote_refs.len(), 1);
        assert_eq!(receipt.remote_refs[0].remote, "origin");
        assert_eq!(receipt.remote_refs[0].branch, "feature/foo");
        assert_eq!(
            receipt.remote_refs[0].remote_refname,
            "refs/remotes/origin/feature/foo"
        );
    }

    #[test]
    fn update_remote_ref_after() {
        let mut receipt = new_receipt(OpKind::Submit);
        receipt.add_remote_ref("origin", "feature", Some("abc123"));
        receipt.update_remote_ref_after("origin", "feature", "def456");

        assert_eq!(receipt.remote_refs[0].oid_after, Some("def456".to_string()));
    }

    #[test]
    fn update_remote_ref_after_wrong_remote_noop() {
        let mut receipt = new_receipt(OpKind::Submit);
        receipt.add_remote_ref("origin", "feature", Some("abc123"));
        receipt.update_remote_ref_after("upstream", "feature", "def456");

        assert!(receipt.remote_refs[0].oid_after.is_none());
    }

    // -- Status transitions --

    #[test]
    fn mark_success() {
        let mut receipt = new_receipt(OpKind::Restack);
        let ts = now_iso();
        receipt.mark_success(ts.clone());

        assert!(matches!(receipt.status, OpStatus::Success));
        assert_eq!(receipt.finished_at, Some(ts));
    }

    #[test]
    fn mark_failed() {
        let mut receipt = new_receipt(OpKind::Restack);
        let ts = now_iso();
        receipt.mark_failed("Conflict detected", Some("rebase"), Some("feature/foo"), ts);

        assert!(matches!(receipt.status, OpStatus::Failed));
        assert!(receipt.finished_at.is_some());

        let error = receipt.error.as_ref().expect("error set");
        assert_eq!(error.message, "Conflict detected");
        assert_eq!(error.failed_step, Some("rebase".to_string()));
        assert_eq!(error.failed_branch, Some("feature/foo".to_string()));
    }

    // -- Query methods --

    #[test]
    fn can_undo_empty_refs() {
        let receipt = new_receipt(OpKind::Restack);
        assert!(!receipt.can_undo());
    }

    #[test]
    fn can_undo_in_progress_cannot() {
        let mut receipt = new_receipt(OpKind::Restack);
        receipt.add_local_ref("feature", Some("abc123"));
        assert!(!receipt.can_undo());
    }

    #[test]
    fn can_undo_with_before_oid_and_success() {
        let mut receipt = new_receipt(OpKind::Restack);
        receipt.add_local_ref("feature", Some("abc123"));
        receipt.mark_success(now_iso());
        assert!(receipt.can_undo());
    }

    #[test]
    fn can_undo_new_branch_cannot() {
        let mut receipt = new_receipt(OpKind::Restack);
        receipt.add_local_ref("new-branch", None);
        receipt.mark_success(now_iso());
        assert!(!receipt.can_undo());
    }

    #[test]
    fn can_undo_failed_cannot() {
        let mut receipt = new_receipt(OpKind::Restack);
        receipt.add_local_ref("feature", Some("abc123"));
        receipt.mark_failed("err", None, None, now_iso());
        assert!(!receipt.can_undo());
    }

    #[test]
    fn can_redo_no_after_oid() {
        let mut receipt = new_receipt(OpKind::Restack);
        receipt.add_local_ref("feature", Some("abc123"));
        assert!(!receipt.can_redo());
    }

    #[test]
    fn can_redo_with_after_oid_but_not_undone() {
        let mut receipt = new_receipt(OpKind::Restack);
        receipt.add_local_ref("feature", Some("abc123"));
        receipt.update_local_ref_after("feature", "def456");
        receipt.mark_success(now_iso());
        assert!(!receipt.can_redo());
    }

    #[test]
    fn can_redo_after_undo() {
        let mut receipt = new_receipt(OpKind::Restack);
        receipt.add_local_ref("feature", Some("abc123"));
        receipt.update_local_ref_after("feature", "def456");
        receipt.mark_success(now_iso());
        receipt.mark_undone(now_iso());
        assert!(receipt.can_redo());
    }

    #[test]
    fn mark_undone_sets_status() {
        let mut receipt = new_receipt(OpKind::Restack);
        receipt.add_local_ref("feature", Some("abc123"));
        receipt.mark_success(now_iso());
        receipt.mark_undone(now_iso());
        assert!(matches!(receipt.status, OpStatus::Undone));
        assert!(receipt.finished_at.is_some());
    }

    #[test]
    fn has_remote_changes_empty() {
        let receipt = new_receipt(OpKind::Submit);
        assert!(!receipt.has_remote_changes());
    }

    #[test]
    fn has_remote_changes_with_ref() {
        let mut receipt = new_receipt(OpKind::Submit);
        receipt.add_remote_ref("origin", "feature", Some("abc123"));
        assert!(receipt.has_remote_changes());
    }

    #[test]
    fn modified_branch_count_mixed() {
        let mut receipt = new_receipt(OpKind::Restack);
        receipt.add_local_ref("a", Some("abc"));
        receipt.add_local_ref("b", Some("abc"));
        receipt.add_local_ref("c", Some("xyz"));

        // All differ from None
        assert_eq!(receipt.modified_branch_count(), 3);

        // Same value = not modified
        receipt.update_local_ref_after("a", "abc");
        assert_eq!(receipt.modified_branch_count(), 2);

        // Different value = still modified
        receipt.update_local_ref_after("b", "def");
        assert_eq!(receipt.modified_branch_count(), 2);
    }

    // -- Serde roundtrip --

    #[test]
    fn receipt_serde_roundtrip() {
        let mut receipt = new_receipt(OpKind::Restack);
        receipt.add_local_ref("feature/foo", Some("abc123"));
        receipt.update_local_ref_after("feature/foo", "def456");
        receipt.mark_success(now_iso());

        let json = serde_json::to_string(&receipt).expect("serialize");
        let loaded: OpReceipt = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(loaded.op_id, receipt.op_id);
        assert_eq!(loaded.status, OpStatus::Success);
        assert_eq!(loaded.local_refs.len(), 1);
        assert_eq!(loaded.local_refs[0].oid_before, Some("abc123".to_string()));
        assert_eq!(loaded.local_refs[0].oid_after, Some("def456".to_string()));
    }

    // -- Clone --

    #[test]
    fn local_ref_entry_clone() {
        let entry = LocalRefEntry {
            branch: "feature".to_string(),
            refname: "refs/heads/feature".to_string(),
            existed_before: true,
            oid_before: Some("abc123".to_string()),
            oid_after: Some("def456".to_string()),
        };
        let cloned = entry.clone();
        assert_eq!(cloned.branch, "feature");
        assert_eq!(cloned.oid_before, entry.oid_before);
    }

    #[test]
    fn remote_ref_entry_clone() {
        let entry = RemoteRefEntry {
            remote: "origin".to_string(),
            branch: "feature".to_string(),
            remote_refname: "refs/remotes/origin/feature".to_string(),
            oid_before: Some("abc123".to_string()),
            oid_after: Some("def456".to_string()),
        };
        let cloned = entry.clone();
        assert_eq!(cloned.remote, "origin");
        assert_eq!(cloned.branch, "feature");
    }

    #[test]
    fn op_error_clone() {
        let error = OpError {
            message: "Test error".to_string(),
            failed_step: Some("rebase".to_string()),
            failed_branch: Some("feature".to_string()),
        };
        let cloned = error.clone();
        assert_eq!(cloned.message, "Test error");
    }

    #[test]
    fn plan_summary_default() {
        let summary = PlanSummary::default();
        assert_eq!(summary.branches_to_rebase, 0);
        assert_eq!(summary.branches_to_push, 0);
        assert!(summary.description.is_empty());
    }
}
