//! Pure calculation functions for operation receipts.
//!
//! These functions operate on receipt data without any I/O.

use crate::domain::receipt::OpReceipt;

pub fn can_undo(receipt: &OpReceipt) -> bool {
    receipt.local_refs.iter().any(|r| r.oid_before.is_some())
}

pub fn can_redo(receipt: &OpReceipt) -> bool {
    receipt.local_refs.iter().any(|r| r.oid_after.is_some())
}

pub fn has_remote_changes(receipt: &OpReceipt) -> bool {
    !receipt.remote_refs.is_empty()
}

pub fn modified_branch_count(receipt: &OpReceipt) -> usize {
    receipt
        .local_refs
        .iter()
        .filter(|r| r.oid_before != r.oid_after)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::receipt::{OpKind, OpReceipt};

    #[test]
    fn test_can_undo_false_when_no_refs() {
        let receipt = OpReceipt::new(
            "test".to_string(),
            OpKind::Restack,
            "/tmp".to_string(),
            "main".to_string(),
            "feature".to_string(),
        );
        assert!(!can_undo(&receipt));
    }

    #[test]
    fn test_can_undo_true_when_ref_has_before_oid() {
        let mut receipt = OpReceipt::new(
            "test".to_string(),
            OpKind::Restack,
            "/tmp".to_string(),
            "main".to_string(),
            "feature".to_string(),
        );
        receipt.add_local_ref("feature", Some("abc123"));
        assert!(can_undo(&receipt));
    }

    #[test]
    fn test_can_undo_false_when_ref_has_no_before_oid() {
        let mut receipt = OpReceipt::new(
            "test".to_string(),
            OpKind::Restack,
            "/tmp".to_string(),
            "main".to_string(),
            "feature".to_string(),
        );
        receipt.add_local_ref("new-branch", None);
        assert!(!can_undo(&receipt));
    }

    #[test]
    fn test_can_redo_false_when_no_refs() {
        let receipt = OpReceipt::new(
            "test".to_string(),
            OpKind::Restack,
            "/tmp".to_string(),
            "main".to_string(),
            "feature".to_string(),
        );
        assert!(!can_redo(&receipt));
    }

    #[test]
    fn test_can_redo_false_when_no_after_oid() {
        let mut receipt = OpReceipt::new(
            "test".to_string(),
            OpKind::Restack,
            "/tmp".to_string(),
            "main".to_string(),
            "feature".to_string(),
        );
        receipt.add_local_ref("feature", Some("abc123"));
        assert!(!can_redo(&receipt));
    }

    #[test]
    fn test_can_redo_true_when_ref_has_after_oid() {
        let mut receipt = OpReceipt::new(
            "test".to_string(),
            OpKind::Restack,
            "/tmp".to_string(),
            "main".to_string(),
            "feature".to_string(),
        );
        receipt.add_local_ref("feature", Some("abc123"));
        receipt.update_local_ref_after("feature", "def456").unwrap();
        assert!(can_redo(&receipt));
    }

    #[test]
    fn test_has_remote_changes_false_when_empty() {
        let receipt = OpReceipt::new(
            "test".to_string(),
            OpKind::Submit,
            "/tmp".to_string(),
            "main".to_string(),
            "feature".to_string(),
        );
        assert!(!has_remote_changes(&receipt));
    }

    #[test]
    fn test_has_remote_changes_true_when_refs_exist() {
        let mut receipt = OpReceipt::new(
            "test".to_string(),
            OpKind::Submit,
            "/tmp".to_string(),
            "main".to_string(),
            "feature".to_string(),
        );
        receipt.add_remote_ref("origin", "feature", Some("abc123"));
        assert!(has_remote_changes(&receipt));
    }

    #[test]
    fn test_modified_branch_count_all_modified_when_no_after() {
        let mut receipt = OpReceipt::new(
            "test".to_string(),
            OpKind::Restack,
            "/tmp".to_string(),
            "main".to_string(),
            "feature".to_string(),
        );
        receipt.add_local_ref("feature-a", Some("abc123"));
        receipt.add_local_ref("feature-b", Some("abc123"));
        assert_eq!(modified_branch_count(&receipt), 2);
    }

    #[test]
    fn test_modified_branch_count_one_unmodified() {
        let mut receipt = OpReceipt::new(
            "test".to_string(),
            OpKind::Restack,
            "/tmp".to_string(),
            "main".to_string(),
            "feature".to_string(),
        );
        receipt.add_local_ref("feature-a", Some("abc123"));
        receipt.add_local_ref("feature-b", Some("abc123"));
        receipt.update_local_ref_after("feature-a", "abc123").unwrap();
        assert_eq!(modified_branch_count(&receipt), 1);
    }
}
