//! Calculation layer for stack undo - pure functions, no I/O.

use scp_vcs::domain::entities::ops::OpReceipt;

use super::data::{build_undo_plan, UndoError, UndoPlan};

pub fn validate_undo_preconditions(receipt: &OpReceipt) -> Result<(), UndoError> {
    if !receipt.can_undo() {
        return Err(UndoError::CannotUndo(
            receipt.op_id.clone(),
            format!("{:?}", receipt.status),
        ));
    }

    Ok(())
}

pub fn compute_undo_plan(receipt: &OpReceipt) -> Result<UndoPlan, UndoError> {
    validate_undo_preconditions(receipt)?;
    Ok(build_undo_plan(receipt))
}

#[cfg(test)]
mod tests {
    use super::*;
    use scp_vcs::domain::entities::ops::OpStatus;

    fn now_iso() -> String {
        chrono::Utc::now().to_rfc3339()
    }

    fn make_receipt_success() -> OpReceipt {
        let mut receipt = OpReceipt::new(
            "op-1".to_string(),
            scp_vcs::domain::entities::ops::OpKind::Restack,
            "/tmp".to_string(),
            "main".to_string(),
            "feature".to_string(),
            now_iso(),
        );
        receipt.add_local_ref("feature-a", Some("aaa"));
        receipt.update_local_ref_after("feature-a", "bbb");
        receipt.mark_success(now_iso());
        receipt
    }

    #[test]
    fn validate_success_receipt_passes() {
        let receipt = make_receipt_success();
        assert!(validate_undo_preconditions(&receipt).is_ok());
    }

    #[test]
    fn validate_in_progress_fails() {
        let mut receipt = OpReceipt::new(
            "op-2".to_string(),
            scp_vcs::domain::entities::ops::OpKind::Restack,
            "/tmp".to_string(),
            "main".to_string(),
            "main".to_string(),
            now_iso(),
        );
        receipt.add_local_ref("feature", Some("abc"));
        let result = validate_undo_preconditions(&receipt);
        assert!(result.is_err());
    }

    #[test]
    fn validate_failed_receipt_fails() {
        let mut receipt = OpReceipt::new(
            "op-3".to_string(),
            scp_vcs::domain::entities::ops::OpKind::Restack,
            "/tmp".to_string(),
            "main".to_string(),
            "main".to_string(),
            now_iso(),
        );
        receipt.add_local_ref("feature", Some("abc"));
        receipt.mark_failed("conflict", None, None, now_iso());
        let result = validate_undo_preconditions(&receipt);
        assert!(result.is_err());
    }

    #[test]
    fn validate_undone_receipt_fails() {
        let mut receipt = make_receipt_success();
        receipt.mark_undone(now_iso());
        let result = validate_undo_preconditions(&receipt);
        assert!(result.is_err());
    }

    #[test]
    fn compute_plan_returns_restores() {
        let receipt = make_receipt_success();
        let plan = compute_undo_plan(&receipt).expect("plan");
        assert_eq!(plan.restores.len(), 1);
        assert_eq!(plan.restores[0].branch, "feature-a");
        assert_eq!(plan.restores[0].oid_before, "aaa");
        assert_eq!(plan.restores[0].oid_after, "bbb");
    }

    #[test]
    fn compute_plan_empty_refs_fails() {
        let mut receipt = OpReceipt::new(
            "op-empty".to_string(),
            scp_vcs::domain::entities::ops::OpKind::Restack,
            "/tmp".to_string(),
            "main".to_string(),
            "main".to_string(),
            now_iso(),
        );
        receipt.mark_success(now_iso());
        let result = compute_undo_plan(&receipt);
        assert!(result.is_err());
    }
}
