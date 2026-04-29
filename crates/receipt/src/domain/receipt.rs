//! Operation receipt data types.
//!
//! Receipts are stored as JSON files under `.git/stax/ops/<op-id>.json`

use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Status of an operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OpStatus {
    InProgress,
    Success,
    Failed,
}

/// Kind of operation
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
}

impl OpKind {
    pub const fn display_name(&self) -> &'static str {
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
        }
    }
}

/// Information about a local ref that was modified
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalRefEntry {
    pub branch: String,
    pub refname: String,
    pub existed_before: bool,
    pub oid_before: Option<String>,
    pub oid_after: Option<String>,
}

/// Information about a remote ref that was modified (for submit)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteRefEntry {
    pub remote: String,
    pub branch: String,
    pub remote_refname: String,
    pub oid_before: Option<String>,
    pub oid_after: Option<String>,
}

/// Error information for failed operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpError {
    pub message: String,
    pub failed_step: Option<String>,
    pub failed_branch: Option<String>,
}

/// Plan summary for display
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlanSummary {
    pub branches_to_rebase: usize,
    pub branches_to_push: usize,
    pub description: Vec<String>,
}

/// Operation receipt - persisted to `.git/stax/ops/<op-id>.json`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpReceipt {
    pub op_id: String,
    pub kind: OpKind,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub status: OpStatus,
    pub repo_workdir: String,
    pub trunk: String,
    pub auto_stash_pop: bool,
    pub head_branch_before: String,
    pub local_refs: Vec<LocalRefEntry>,
    pub remote_refs: Vec<RemoteRefEntry>,
    pub plan_summary: PlanSummary,
    pub error: Option<OpError>,
}

impl OpReceipt {
    pub fn new(
        op_id: String,
        kind: OpKind,
        repo_workdir: String,
        trunk: String,
        head_branch_before: String,
    ) -> Self {
        let started_at = Utc::now().to_rfc3339();

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

    pub fn add_local_ref(&mut self, branch: &str, oid_before: Option<&str>) {
        self.local_refs.push(LocalRefEntry {
            branch: branch.to_string(),
            refname: format!("refs/heads/{}", branch),
            existed_before: oid_before.is_some(),
            oid_before: oid_before.map(String::from),
            oid_after: None,
        });
    }

    pub fn add_remote_ref(&mut self, remote: &str, branch: &str, oid_before: Option<&str>) {
        self.remote_refs.push(RemoteRefEntry {
            remote: remote.to_string(),
            branch: branch.to_string(),
            remote_refname: format!("refs/remotes/{}/{}", remote, branch),
            oid_before: oid_before.map(String::from),
            oid_after: None,
        });
    }

    pub fn update_local_ref_after(&mut self, branch: &str, oid_after: &str) {
        if let Some(entry) = self.local_refs.iter_mut().find(|e| e.branch == branch) {
            entry.oid_after = Some(oid_after.to_string());
        }
    }

    pub fn update_remote_ref_after(&mut self, remote: &str, branch: &str, oid_after: &str) {
        if let Some(entry) = self
            .remote_refs
            .iter_mut()
            .find(|e| e.remote == remote && e.branch == branch)
        {
            entry.oid_after = Some(oid_after.to_string());
        }
    }

    pub fn mark_success(&mut self) {
        self.status = OpStatus::Success;
        self.finished_at = Some(Utc::now().to_rfc3339());
    }

    pub fn mark_failed(
        &mut self,
        message: &str,
        failed_step: Option<&str>,
        failed_branch: Option<&str>,
    ) {
        self.status = OpStatus::Failed;
        self.finished_at = Some(Utc::now().to_rfc3339());
        self.error = Some(OpError {
            message: message.to_string(),
            failed_step: failed_step.map(String::from),
            failed_branch: failed_branch.map(String::from),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn op_status_roundtrip_serialization() {
        for status in [OpStatus::InProgress, OpStatus::Success, OpStatus::Failed] {
            let json = serde_json::to_string(&status).expect("serialize");
            let deserialized: OpStatus = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(status, deserialized);
        }
    }

    #[test]
    fn op_kind_roundtrip_serialization() {
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
        ] {
            let json = serde_json::to_string(&kind).expect("serialize");
            let deserialized: OpKind = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(kind, deserialized);
        }
    }

    #[test]
    fn op_kind_display_name_matches_expected() {
        assert_eq!(OpKind::Restack.display_name(), "restack");
        assert_eq!(OpKind::Submit.display_name(), "submit");
        assert_eq!(OpKind::Fix.display_name(), "stack fix");
    }

    #[test]
    fn op_receipt_new_sets_defaults() {
        let receipt = OpReceipt::new(
            "op-1".to_string(),
            OpKind::Restack,
            "/repo".to_string(),
            "main".to_string(),
            "feature".to_string(),
        );
        assert_eq!(receipt.op_id, "op-1");
        assert_eq!(receipt.status, OpStatus::InProgress);
        assert!(receipt.finished_at.is_none());
        assert!(receipt.local_refs.is_empty());
        assert!(receipt.remote_refs.is_empty());
        assert!(receipt.error.is_none());
        assert!(!receipt.auto_stash_pop);
    }

    #[test]
    fn op_receipt_lifecycle() {
        let mut receipt = OpReceipt::new(
            "op-2".to_string(),
            OpKind::Restack,
            "/repo".to_string(),
            "main".to_string(),
            "feature".to_string(),
        );
        receipt.add_local_ref("feature", Some("abc123"));
        assert_eq!(receipt.local_refs.len(), 1);

        receipt.update_local_ref_after("feature", "def456");
        assert_eq!(receipt.local_refs[0].oid_after, Some("def456".to_string()));

        receipt.mark_success();
        assert_eq!(receipt.status, OpStatus::Success);
        assert!(receipt.finished_at.is_some());
    }

    #[test]
    fn op_receipt_mark_failed() {
        let mut receipt = OpReceipt::new(
            "op-3".to_string(),
            OpKind::Submit,
            "/repo".to_string(),
            "main".to_string(),
            "feature".to_string(),
        );
        receipt.mark_failed("merge conflict", Some("rebase"), Some("feature"));
        assert_eq!(receipt.status, OpStatus::Failed);
        assert!(receipt.finished_at.is_some());
        let err = receipt.error.expect("should have error");
        assert_eq!(err.message, "merge conflict");
        assert_eq!(err.failed_step, Some("rebase".to_string()));
    }

    #[test]
    fn op_receipt_add_remote_ref() {
        let mut receipt = OpReceipt::new(
            "op-4".to_string(),
            OpKind::Submit,
            "/repo".to_string(),
            "main".to_string(),
            "feature".to_string(),
        );
        receipt.add_remote_ref("origin", "feature", Some("abc123"));
        assert_eq!(receipt.remote_refs.len(), 1);
        assert_eq!(receipt.remote_refs[0].remote, "origin");

        receipt.update_remote_ref_after("origin", "feature", "def456");
        assert_eq!(receipt.remote_refs[0].oid_after, Some("def456".to_string()));
    }

    #[test]
    fn op_receipt_serialize_deserialize_roundtrip() {
        let mut receipt = OpReceipt::new(
            "20251229T120500Z-abc123".to_string(),
            OpKind::Restack,
            "/tmp/repo".to_string(),
            "main".to_string(),
            "feature/foo".to_string(),
        );
        receipt.add_local_ref("feature/foo", Some("abc123"));
        receipt.update_local_ref_after("feature/foo", "def456");
        receipt.mark_success();

        let json = serde_json::to_string(&receipt).expect("serialize");
        let deserialized: OpReceipt = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.op_id, receipt.op_id);
        assert_eq!(deserialized.status, OpStatus::Success);
        assert_eq!(deserialized.local_refs.len(), 1);
    }

    #[test]
    fn local_ref_entry_fields() {
        let entry = LocalRefEntry {
            branch: "feature".to_string(),
            refname: "refs/heads/feature".to_string(),
            existed_before: true,
            oid_before: Some("abc".to_string()),
            oid_after: Some("def".to_string()),
        };
        assert_eq!(entry.branch, "feature");
        assert!(entry.existed_before);
    }

    #[test]
    fn plan_summary_default_is_zero() {
        let summary = PlanSummary::default();
        assert_eq!(summary.branches_to_rebase, 0);
        assert_eq!(summary.branches_to_push, 0);
        assert!(summary.description.is_empty());
    }
}
