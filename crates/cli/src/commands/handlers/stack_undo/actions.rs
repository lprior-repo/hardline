//! Action layer for stack undo - I/O operations via git CLI and receipt persistence.
//!
//! Loads the latest transaction receipt, validates undo preconditions,
//! restores branch refs to pre-operation OIDs, deletes backup refs,
//! and marks the receipt as undone.

use std::path::Path;

use scp_core::output::Output;
use scp_vcs::domain::entities::ops::OpReceipt;
use scp_vcs::infrastructure::ops;

use super::calc;
use super::data::{StackUndoOptions, StackUndoOutput, UndoError};

pub fn run_stack_undo(
    workdir: &Path,
    git_dir: &Path,
    options: &StackUndoOptions,
) -> Result<StackUndoOutput, UndoError> {
    let receipt = load_latest(git_dir)?;

    let plan = calc::compute_undo_plan(&receipt).map_err(|e| {
        if matches!(e, UndoError::CannotUndo(_, _)) {
            UndoError::CannotUndo(receipt.op_id.clone(), format!("{:?}", receipt.status))
        } else {
            e
        }
    })?;

    if options.dry_run {
        return format_dry_run_output(&plan);
    }

    execute_restore(workdir, git_dir, &receipt, &plan)
}

fn load_latest(git_dir: &Path) -> Result<OpReceipt, UndoError> {
    ops::load_latest_receipt(git_dir)
        .map_err(|e| UndoError::ReceiptError(e.to_string()))?
        .ok_or(UndoError::NoOperations)
}

fn execute_restore(
    workdir: &Path,
    git_dir: &Path,
    receipt: &OpReceipt,
    plan: &super::data::UndoPlan,
) -> Result<StackUndoOutput, UndoError> {
    for restore in &plan.restores {
        ops::restore_branch_ref(workdir, &restore.branch, &restore.oid_before).map_err(|e| {
            UndoError::RestoreFailed {
                branch: restore.branch.clone(),
                reason: e.to_string(),
            }
        })?;
    }

    for entry in &receipt.local_refs {
        let backup_ref = ops::backup_ref_name(&receipt.op_id, &entry.branch);
        let _ = ops::delete_backup_ref(workdir, &backup_ref);
    }

    let mut updated_receipt = receipt.clone();
    let finished_at = chrono::Utc::now().to_rfc3339();
    updated_receipt.mark_undone(finished_at);
    ops::save_receipt(git_dir, &updated_receipt)
        .map_err(|e| UndoError::ReceiptError(e.to_string()))?;

    let branches_count = plan.restores.len();
    Output::success(&format!(
        "Undone {} (restored {} branch{})",
        plan.kind.display_name(),
        branches_count,
        if branches_count == 1 { "" } else { "es" }
    ));

    for restore in &plan.restores {
        Output::info(&format!("  {} -> {}", restore.branch, restore.oid_before));
    }

    Output::info("Redo with: scp stack redo");

    Ok(StackUndoOutput {
        op_id: plan.op_id.clone(),
        kind: plan.kind.clone(),
        branches_restored: plan.restores.clone(),
        dry_run: false,
    })
}

fn format_dry_run_output(plan: &super::data::UndoPlan) -> Result<StackUndoOutput, UndoError> {
    Output::info(&format!(
        "Dry-run: would undo {} ({})",
        plan.kind.display_name(),
        plan.op_id
    ));

    for restore in &plan.restores {
        Output::info(&format!(
            "  {} would reset: {} -> {}",
            restore.branch, restore.oid_after, restore.oid_before
        ));
    }

    Ok(StackUndoOutput {
        op_id: plan.op_id.clone(),
        kind: plan.kind.clone(),
        branches_restored: plan.restores.clone(),
        dry_run: true,
    })
}

pub fn run_stack_redo(
    workdir: &Path,
    git_dir: &Path,
    options: &StackUndoOptions,
) -> Result<StackUndoOutput, UndoError> {
    let receipt = load_latest(git_dir)?;

    if !receipt.can_redo() {
        return Err(UndoError::CannotUndo(
            receipt.op_id.clone(),
            format!("{:?} (not undone or no after-state)", receipt.status),
        ));
    }

    let restores: Vec<super::data::BranchRestore> = receipt
        .local_refs
        .iter()
        .filter_map(|entry| match (&entry.oid_before, &entry.oid_after) {
            (Some(before), Some(after)) => Some(super::data::BranchRestore {
                branch: entry.branch.clone(),
                oid_before: before.clone(),
                oid_after: after.clone(),
            }),
            _ => None,
        })
        .collect();

    if options.dry_run {
        Output::info(&format!(
            "Dry-run: would redo {} ({})",
            receipt.kind.display_name(),
            receipt.op_id
        ));
        for restore in &restores {
            Output::info(&format!(
                "  {} would restore: {} -> {}",
                restore.branch, restore.oid_before, restore.oid_after
            ));
        }
        return Ok(StackUndoOutput {
            op_id: receipt.op_id.clone(),
            kind: receipt.kind.clone(),
            branches_restored: restores,
            dry_run: true,
        });
    }

    for restore in &restores {
        ops::restore_branch_ref(workdir, &restore.branch, &restore.oid_after).map_err(|e| {
            UndoError::RestoreFailed {
                branch: restore.branch.clone(),
                reason: e.to_string(),
            }
        })?;
    }

    let mut updated_receipt = receipt.clone();
    let finished_at = chrono::Utc::now().to_rfc3339();
    updated_receipt.mark_success(finished_at);
    ops::save_receipt(git_dir, &updated_receipt)
        .map_err(|e| UndoError::ReceiptError(e.to_string()))?;

    Output::success(&format!(
        "Redone {} (restored {} branch{})",
        updated_receipt.kind.display_name(),
        restores.len(),
        if restores.len() == 1 { "" } else { "es" }
    ));

    Ok(StackUndoOutput {
        op_id: updated_receipt.op_id.clone(),
        kind: updated_receipt.kind.clone(),
        branches_restored: restores,
        dry_run: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use scp_vcs::domain::entities::ops::OpKind;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn setup_git_repo() -> (TempDir, PathBuf, PathBuf) {
        let temp = TempDir::new().expect("temp dir");
        let workdir = temp.path().to_path_buf();
        let git_dir = workdir.join(".git");

        std::process::Command::new("git")
            .args(["init", "--quiet", "--initial-branch=main"])
            .current_dir(&workdir)
            .output()
            .expect("git init");

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

        std::fs::write(workdir.join("file.txt"), "initial").expect("write");
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(&workdir)
            .output()
            .expect("git add");

        std::process::Command::new("git")
            .args(["commit", "--quiet", "-m", "initial"])
            .current_dir(&workdir)
            .output()
            .expect("git commit");

        (temp, workdir, git_dir)
    }

    fn now_iso() -> String {
        chrono::Utc::now().to_rfc3339()
    }

    fn create_branch_with_commit(workdir: &Path, branch: &str, filename: &str, content: &str) {
        std::process::Command::new("git")
            .args(["checkout", "--quiet", "-b", branch])
            .current_dir(workdir)
            .output()
            .expect("git checkout -b");

        std::fs::write(workdir.join(filename), content).expect("write");
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

    fn git_rev(workdir: &Path, refname: &str) -> String {
        let output = std::process::Command::new("git")
            .args(["rev-parse", refname])
            .current_dir(workdir)
            .output()
            .expect("git rev-parse");
        String::from_utf8(output.stdout)
            .expect("utf8")
            .trim()
            .to_string()
    }

    #[test]
    fn dry_run_does_not_modify_refs() {
        let (temp, workdir, git_dir) = setup_git_repo();

        create_branch_with_commit(&workdir, "feature-a", "file-a.txt", "a");
        let rev_after = git_rev(&workdir, "feature-a");

        let main_rev = git_rev(&workdir, "main");

        let mut receipt = OpReceipt::new(
            "test-dry-run".to_string(),
            OpKind::Restack,
            workdir.to_string_lossy().to_string(),
            "main".to_string(),
            "feature-a".to_string(),
            now_iso(),
        );
        receipt.add_local_ref("feature-a", Some(&main_rev));
        receipt.update_local_ref_after("feature-a", &rev_after);
        receipt.mark_success(now_iso());

        ops::save_receipt(&git_dir, &receipt).expect("save receipt");

        let result = run_stack_undo(&workdir, &git_dir, &StackUndoOptions { dry_run: true })
            .expect("dry run should succeed");

        assert!(result.dry_run);
        assert_eq!(result.branches_restored.len(), 1);

        let current_rev = git_rev(&workdir, "feature-a");
        assert_eq!(current_rev, rev_after, "Dry run should not change refs");
    }

    #[test]
    fn execute_undo_restores_refs() {
        let (temp, workdir, git_dir) = setup_git_repo();
        let main_rev = git_rev(&workdir, "main");

        create_branch_with_commit(&workdir, "feature-a", "file-a.txt", "a");
        let rev_after = git_rev(&workdir, "feature-a");

        let mut receipt = OpReceipt::new(
            "test-undo-exec".to_string(),
            OpKind::Restack,
            workdir.to_string_lossy().to_string(),
            "main".to_string(),
            "feature-a".to_string(),
            now_iso(),
        );
        receipt.add_local_ref("feature-a", Some(&main_rev));
        receipt.update_local_ref_after("feature-a", &rev_after);
        receipt.mark_success(now_iso());

        ops::save_receipt(&git_dir, &receipt).expect("save receipt");

        let result = run_stack_undo(&workdir, &git_dir, &StackUndoOptions { dry_run: false })
            .expect("undo should succeed");

        assert!(!result.dry_run);
        assert_eq!(result.branches_restored.len(), 1);

        let current_rev = git_rev(&workdir, "feature-a");
        assert_eq!(
            current_rev, main_rev,
            "Branch should be restored to original OID"
        );

        let loaded = ops::load_receipt(&git_dir, "test-undo-exec").expect("load receipt");
        assert!(
            matches!(
                loaded.status,
                scp_vcs::domain::entities::ops::OpStatus::Undone
            ),
            "Receipt should be marked as undone"
        );
    }

    #[test]
    fn no_operations_returns_error() {
        let (temp, workdir, git_dir) = setup_git_repo();

        let result = run_stack_undo(&workdir, &git_dir, &StackUndoOptions { dry_run: false });

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), UndoError::NoOperations));
    }
}
