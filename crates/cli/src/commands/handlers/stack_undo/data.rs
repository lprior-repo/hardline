//! Data layer for stack undo - inert, serializable types.

use serde::{Deserialize, Serialize};

use scp_vcs::domain::entities::ops::{OpKind, OpReceipt};

#[derive(Debug, Clone)]
pub struct StackUndoOptions {
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackUndoOutput {
    pub op_id: String,
    pub kind: OpKind,
    pub branches_restored: Vec<BranchRestore>,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchRestore {
    pub branch: String,
    pub oid_before: String,
    pub oid_after: String,
}

#[derive(Debug, thiserror::Error)]
pub enum UndoError {
    #[error("No operations to undo")]
    NoOperations,
    #[error("Operation {0} cannot be undone (status: {1})")]
    CannotUndo(String, String),
    #[error("Failed to restore branch '{branch}': {reason}")]
    RestoreFailed { branch: String, reason: String },
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Receipt error: {0}")]
    ReceiptError(String),
}

impl From<UndoError> for scp_core::Error {
    fn from(err: UndoError) -> Self {
        scp_core::Error::invalid_state(err.to_string())
    }
}

pub struct UndoPlan {
    pub op_id: String,
    pub kind: OpKind,
    pub restores: Vec<BranchRestore>,
}

pub fn build_undo_plan(receipt: &OpReceipt) -> UndoPlan {
    let restores = receipt
        .local_refs
        .iter()
        .filter_map(|entry| match (&entry.oid_before, &entry.oid_after) {
            (Some(before), Some(after)) => Some(BranchRestore {
                branch: entry.branch.clone(),
                oid_before: before.clone(),
                oid_after: after.clone(),
            }),
            (Some(before), None) => Some(BranchRestore {
                branch: entry.branch.clone(),
                oid_before: before.clone(),
                oid_after: String::new(),
            }),
            _ => None,
        })
        .collect();

    UndoPlan {
        op_id: receipt.op_id.clone(),
        kind: receipt.kind.clone(),
        restores,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scp_vcs::domain::entities::ops::OpStatus;

    fn now_iso() -> String {
        chrono::Utc::now().to_rfc3339()
    }

    fn make_success_receipt() -> OpReceipt {
        let mut receipt = OpReceipt::new(
            "test-op-123".to_string(),
            OpKind::Restack,
            "/tmp/repo".to_string(),
            "main".to_string(),
            "feature".to_string(),
            now_iso(),
        );
        receipt.add_local_ref("feature-a", Some("aaa111"));
        receipt.update_local_ref_after("feature-a", "bbb222");
        receipt.add_local_ref("feature-b", Some("ccc333"));
        receipt.update_local_ref_after("feature-b", "ddd444");
        receipt.mark_success(now_iso());
        receipt
    }

    #[test]
    fn stack_undo_options_default() {
        let opts = StackUndoOptions { dry_run: false };
        assert!(!opts.dry_run);
    }

    #[test]
    fn stack_undo_output_serialization() {
        let output = StackUndoOutput {
            op_id: "test-123".to_string(),
            kind: OpKind::Restack,
            branches_restored: vec![BranchRestore {
                branch: "feature".to_string(),
                oid_before: "abc".to_string(),
                oid_after: "def".to_string(),
            }],
            dry_run: false,
        };
        let json = serde_json::to_string(&output).expect("serialize");
        let back: StackUndoOutput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.op_id, "test-123");
        assert_eq!(back.branches_restored.len(), 1);
    }

    #[test]
    fn undo_error_display() {
        let err = UndoError::NoOperations;
        assert!(err.to_string().contains("No operations"));

        let err = UndoError::CannotUndo("op-1".to_string(), "failed".to_string());
        assert!(err.to_string().contains("op-1"));

        let err = UndoError::RestoreFailed {
            branch: "feat".to_string(),
            reason: "bad ref".to_string(),
        };
        assert!(err.to_string().contains("feat"));
    }

    #[test]
    fn build_undo_plan_from_receipt() {
        let receipt = make_success_receipt();
        let plan = build_undo_plan(&receipt);
        assert_eq!(plan.op_id, "test-op-123");
        assert!(matches!(plan.kind, OpKind::Restack));
        assert_eq!(plan.restores.len(), 2);
        assert_eq!(plan.restores[0].branch, "feature-a");
        assert_eq!(plan.restores[0].oid_before, "aaa111");
        assert_eq!(plan.restores[0].oid_after, "bbb222");
    }

    #[test]
    fn build_undo_plan_skips_new_branches() {
        let mut receipt = OpReceipt::new(
            "test-op-456".to_string(),
            OpKind::Split,
            "/tmp".to_string(),
            "main".to_string(),
            "main".to_string(),
            now_iso(),
        );
        receipt.add_local_ref("existing", Some("abc"));
        receipt.update_local_ref_after("existing", "def");
        receipt.add_local_ref("new-branch", None);
        receipt.mark_success(now_iso());

        let plan = build_undo_plan(&receipt);
        assert_eq!(plan.restores.len(), 1);
        assert_eq!(plan.restores[0].branch, "existing");
    }

    #[test]
    fn branch_restore_serialization() {
        let restore = BranchRestore {
            branch: "feature".to_string(),
            oid_before: "aaa".to_string(),
            oid_after: "bbb".to_string(),
        };
        let json = serde_json::to_string(&restore).expect("serialize");
        let back: BranchRestore = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.branch, "feature");
        assert_eq!(back.oid_before, "aaa");
    }
}
