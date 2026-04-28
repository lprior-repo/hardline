//! Operation receipt data types.
//!
//! Receipts are stored as JSON files under `.git/stax/ops/<op-id>.json`

use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Status of an operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OpStatus {
    InProgress,
    Success,
    Failed,
}

/// Kind of operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    pub fn display_name(&self) -> &'static str {
        match self {
            OpKind::Restack => "restack",
            OpKind::UpstackRestack => "upstack restack",
            OpKind::SyncRestack => "sync --restack",
            OpKind::Submit => "submit",
            OpKind::Reorder => "reorder",
            OpKind::Split => "split",
            OpKind::MergeWhenReady => "merge-when-ready",
            OpKind::Detach => "detach",
            OpKind::Fix => "stack fix",
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
mod serde_roundtrip_tests {
    use super::*;

    #[test]
    fn op_status_roundtrip_via_json() {
        for status in [OpStatus::InProgress, OpStatus::Success, OpStatus::Failed] {
            let json = serde_json::to_string(&status).expect("serialize should succeed");
            let deserialized: OpStatus =
                serde_json::from_str(&json).expect("deserialize should succeed");
            assert_eq!(status, deserialized);
        }
    }

    #[test]
    fn op_status_snake_case_serialization() {
        let json_in_progress = serde_json::to_string(&OpStatus::InProgress).expect("serialize");
        assert_eq!(json_in_progress, "\"in_progress\"");
        let json_success = serde_json::to_string(&OpStatus::Success).expect("serialize");
        assert_eq!(json_success, "\"success\"");
        let json_failed = serde_json::to_string(&OpStatus::Failed).expect("serialize");
        assert_eq!(json_failed, "\"failed\"");
    }

    #[test]
    fn op_status_deserialize_snake_case() {
        let in_progress: OpStatus = serde_json::from_str("\"in_progress\"").expect("deserialize");
        assert_eq!(in_progress, OpStatus::InProgress);
        let success: OpStatus = serde_json::from_str("\"success\"").expect("deserialize");
        assert_eq!(success, OpStatus::Success);
        let failed: OpStatus = serde_json::from_str("\"failed\"").expect("deserialize");
        assert_eq!(failed, OpStatus::Failed);
    }

    #[test]
    fn op_status_deserialize_rejects_wrong_variant_name() {
        let result: std::result::Result<OpStatus, _> = serde_json::from_str("\"InProgress\"");
        assert!(result.is_err(), "camelCase variant name should be rejected");
    }

    #[test]
    fn op_kind_roundtrip_via_json() {
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
            let json = serde_json::to_string(&kind).expect("serialize should succeed");
            let deserialized: OpKind =
                serde_json::from_str(&json).expect("deserialize should succeed");
            assert_eq!(kind, deserialized);
        }
    }

    #[test]
    fn op_kind_snake_case_serialization() {
        assert_eq!(
            serde_json::to_string(&OpKind::Restack).unwrap(),
            "\"restack\""
        );
        assert_eq!(
            serde_json::to_string(&OpKind::UpstackRestack).unwrap(),
            "\"upstack_restack\""
        );
        assert_eq!(
            serde_json::to_string(&OpKind::SyncRestack).unwrap(),
            "\"sync_restack\""
        );
        assert_eq!(
            serde_json::to_string(&OpKind::Submit).unwrap(),
            "\"submit\""
        );
        assert_eq!(
            serde_json::to_string(&OpKind::Reorder).unwrap(),
            "\"reorder\""
        );
        assert_eq!(serde_json::to_string(&OpKind::Split).unwrap(), "\"split\"");
        assert_eq!(
            serde_json::to_string(&OpKind::MergeWhenReady).unwrap(),
            "\"merge_when_ready\""
        );
        assert_eq!(
            serde_json::to_string(&OpKind::Detach).unwrap(),
            "\"detach\""
        );
        assert_eq!(serde_json::to_string(&OpKind::Fix).unwrap(), "\"fix\"");
    }

    #[test]
    fn op_kind_deserialize_snake_case() {
        assert_eq!(
            serde_json::from_str::<OpKind>("\"restack\"").unwrap(),
            OpKind::Restack
        );
        assert_eq!(
            serde_json::from_str::<OpKind>("\"upstack_restack\"").unwrap(),
            OpKind::UpstackRestack
        );
        assert_eq!(
            serde_json::from_str::<OpKind>("\"sync_restack\"").unwrap(),
            OpKind::SyncRestack
        );
        assert_eq!(
            serde_json::from_str::<OpKind>("\"merge_when_ready\"").unwrap(),
            OpKind::MergeWhenReady
        );
    }

    #[test]
    fn op_kind_deserialize_rejects_wrong_variant_name() {
        let result: std::result::Result<OpKind, _> = serde_json::from_str("\"MergeWhenReady\"");
        assert!(result.is_err(), "camelCase variant name should be rejected");
    }

    #[test]
    fn local_ref_entry_roundtrip_via_json() {
        let entry = LocalRefEntry {
            branch: "feature/test".to_string(),
            refname: "refs/heads/feature/test".to_string(),
            existed_before: true,
            oid_before: Some("abc123".to_string()),
            oid_after: Some("def456".to_string()),
        };
        let json = serde_json::to_string(&entry).expect("serialize should succeed");
        let deserialized: LocalRefEntry =
            serde_json::from_str(&json).expect("deserialize should succeed");
        assert_eq!(deserialized.branch, entry.branch);
        assert_eq!(deserialized.refname, entry.refname);
        assert_eq!(deserialized.existed_before, entry.existed_before);
        assert_eq!(deserialized.oid_before, entry.oid_before);
        assert_eq!(deserialized.oid_after, entry.oid_after);
    }

    #[test]
    fn local_ref_entry_with_null_optionals_roundtrip() {
        let entry = LocalRefEntry {
            branch: "main".to_string(),
            refname: "refs/heads/main".to_string(),
            existed_before: false,
            oid_before: None,
            oid_after: None,
        };
        let json = serde_json::to_string(&entry).expect("serialize should succeed");
        let deserialized: LocalRefEntry =
            serde_json::from_str(&json).expect("deserialize should succeed");
        assert!(deserialized.oid_before.is_none());
        assert!(deserialized.oid_after.is_none());
    }

    #[test]
    fn local_ref_entry_missing_optional_fields_allowed() {
        let json = r#"{
            "branch": "feature",
            "refname": "refs/heads/feature",
            "existed_before": true,
            "oid_before": null,
            "oid_after": null
        }"#;
        let entry: LocalRefEntry = serde_json::from_str(json).expect("deserialize should succeed");
        assert_eq!(entry.branch, "feature");
    }

    #[test]
    fn remote_ref_entry_roundtrip_via_json() {
        let entry = RemoteRefEntry {
            remote: "origin".to_string(),
            branch: "feature".to_string(),
            remote_refname: "refs/remotes/origin/feature".to_string(),
            oid_before: Some("abc".to_string()),
            oid_after: Some("def".to_string()),
        };
        let json = serde_json::to_string(&entry).expect("serialize should succeed");
        let deserialized: RemoteRefEntry =
            serde_json::from_str(&json).expect("deserialize should succeed");
        assert_eq!(deserialized.remote, entry.remote);
        assert_eq!(deserialized.branch, entry.branch);
        assert_eq!(deserialized.oid_before, entry.oid_before);
    }

    #[test]
    fn remote_ref_entry_missing_optional_fields_allowed() {
        let json = r#"{
            "remote": "origin",
            "branch": "main",
            "remote_refname": "refs/remotes/origin/main",
            "oid_before": null,
            "oid_after": null
        }"#;
        let entry: RemoteRefEntry = serde_json::from_str(json).expect("deserialize should succeed");
        assert!(entry.oid_before.is_none());
    }

    #[test]
    fn op_error_roundtrip_via_json() {
        let error = OpError {
            message: "merge conflict".to_string(),
            failed_step: Some("rebase".to_string()),
            failed_branch: Some("feature".to_string()),
        };
        let json = serde_json::to_string(&error).expect("serialize should succeed");
        let deserialized: OpError =
            serde_json::from_str(&json).expect("deserialize should succeed");
        assert_eq!(deserialized.message, error.message);
        assert_eq!(deserialized.failed_step, error.failed_step);
        assert_eq!(deserialized.failed_branch, error.failed_branch);
    }

    #[test]
    fn op_error_with_null_optionals_roundtrip() {
        let error = OpError {
            message: "unknown error".to_string(),
            failed_step: None,
            failed_branch: None,
        };
        let json = serde_json::to_string(&error).expect("serialize should succeed");
        let deserialized: OpError =
            serde_json::from_str(&json).expect("deserialize should succeed");
        assert!(deserialized.failed_step.is_none());
        assert!(deserialized.failed_branch.is_none());
    }

    #[test]
    fn op_error_missing_optional_fields_allowed() {
        let json = r#"{
            "message": "error occurred",
            "failed_step": null,
            "failed_branch": null
        }"#;
        let error: OpError = serde_json::from_str(json).expect("deserialize should succeed");
        assert_eq!(error.message, "error occurred");
    }

    #[test]
    fn plan_summary_roundtrip_via_json() {
        let summary = PlanSummary {
            branches_to_rebase: 5,
            branches_to_push: 3,
            description: vec!["step 1".to_string(), "step 2".to_string()],
        };
        let json = serde_json::to_string(&summary).expect("serialize should succeed");
        let deserialized: PlanSummary =
            serde_json::from_str(&json).expect("deserialize should succeed");
        assert_eq!(deserialized.branches_to_rebase, summary.branches_to_rebase);
        assert_eq!(deserialized.branches_to_push, summary.branches_to_push);
        assert_eq!(deserialized.description, summary.description);
    }

    #[test]
    fn plan_summary_default_roundtrip() {
        let summary = PlanSummary::default();
        let json = serde_json::to_string(&summary).expect("serialize should succeed");
        let deserialized: PlanSummary =
            serde_json::from_str(&json).expect("deserialize should succeed");
        assert_eq!(deserialized.branches_to_rebase, 0);
        assert_eq!(deserialized.branches_to_push, 0);
        assert!(deserialized.description.is_empty());
    }

    #[test]
    fn op_receipt_full_roundtrip_via_json() {
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

        let json = serde_json::to_string(&receipt).expect("serialize should succeed");
        let deserialized: OpReceipt =
            serde_json::from_str(&json).expect("deserialize should succeed");
        assert_eq!(deserialized.op_id, receipt.op_id);
        assert_eq!(deserialized.kind, receipt.kind);
        assert_eq!(deserialized.status, receipt.status);
        assert_eq!(deserialized.local_refs.len(), 1);
        assert_eq!(
            deserialized.local_refs[0].oid_after,
            Some("def456".to_string())
        );
    }

    #[test]
    fn op_receipt_with_failed_status_roundtrip() {
        let mut receipt = OpReceipt::new(
            "op-failed-1".to_string(),
            OpKind::Submit,
            "/repo".to_string(),
            "main".to_string(),
            "feature".to_string(),
        );
        receipt.mark_failed("merge conflict", Some("rebase"), Some("feature"));

        let json = serde_json::to_string(&receipt).expect("serialize should succeed");
        let deserialized: OpReceipt =
            serde_json::from_str(&json).expect("deserialize should succeed");
        assert_eq!(deserialized.status, OpStatus::Failed);
        assert!(deserialized.error.is_some());
        let error = deserialized.error.unwrap();
        assert_eq!(error.message, "merge conflict");
        assert_eq!(error.failed_step, Some("rebase".to_string()));
    }

    #[test]
    fn op_receipt_missing_required_field_fails() {
        let json = r#"{
            "op_id": "op-1",
            "kind": "restack",
            "started_at": "2024-01-01T00:00:00Z",
            "status": "in_progress",
            "repo_workdir": "/repo",
            "trunk": "main",
            "auto_stash_pop": false,
            "head_branch_before": "feature"
        }"#;
        let result: std::result::Result<OpReceipt, _> = serde_json::from_str(json);
        assert!(
            result.is_err(),
            "missing local_refs, remote_refs, plan_summary should fail"
        );
    }

    #[test]
    fn op_receipt_extra_field_in_json_is_ignored() {
        let json = r#"{
            "op_id": "op-extra",
            "kind": "restack",
            "started_at": "2024-01-01T00:00:00Z",
            "finished_at": null,
            "status": "in_progress",
            "repo_workdir": "/repo",
            "trunk": "main",
            "auto_stash_pop": false,
            "head_branch_before": "feature",
            "local_refs": [],
            "remote_refs": [],
            "plan_summary": {"branches_to_rebase": 0, "branches_to_push": 0, "description": []},
            "error": null,
            "extra_field_that_should_be_ignored": "this is extra"
        }"#;
        let receipt: OpReceipt = serde_json::from_str(json).expect("deserialize should succeed");
        assert_eq!(receipt.op_id, "op-extra");
    }

    #[test]
    fn op_receipt_with_remote_refs_roundtrip() {
        let mut receipt = OpReceipt::new(
            "op-remote-1".to_string(),
            OpKind::Submit,
            "/repo".to_string(),
            "main".to_string(),
            "feature".to_string(),
        );
        receipt.add_remote_ref("origin", "feature", Some("abc123"));
        receipt.update_remote_ref_after("origin", "feature", "def456");

        let json = serde_json::to_string(&receipt).expect("serialize should succeed");
        let deserialized: OpReceipt =
            serde_json::from_str(&json).expect("deserialize should succeed");
        assert_eq!(deserialized.remote_refs.len(), 1);
        assert_eq!(deserialized.remote_refs[0].remote, "origin");
        assert_eq!(
            deserialized.remote_refs[0].oid_after,
            Some("def456".to_string())
        );
    }

    #[test]
    fn op_receipt_bytes_roundtrip() {
        let mut receipt = OpReceipt::new(
            "op-bytes-1".to_string(),
            OpKind::Fix,
            "/repo".to_string(),
            "main".to_string(),
            "bugfix".to_string(),
        );
        receipt.mark_success();

        let bytes = serde_json::to_vec(&receipt).expect("to_vec should succeed");
        let deserialized: OpReceipt =
            serde_json::from_slice(&bytes).expect("from_slice should succeed");
        assert_eq!(deserialized.op_id, receipt.op_id);
    }

    #[test]
    fn op_receipt_pretty_print_roundtrip() {
        let receipt = OpReceipt::new(
            "op-pretty-1".to_string(),
            OpKind::Reorder,
            "/repo".to_string(),
            "main".to_string(),
            "feature".to_string(),
        );

        let pretty = serde_json::to_string_pretty(&receipt).expect("pretty print should succeed");
        let deserialized: OpReceipt =
            serde_json::from_str(&pretty).expect("roundtrip should succeed");
        assert_eq!(deserialized.op_id, receipt.op_id);
    }
}
