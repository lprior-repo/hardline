//! Action layer for stack split - I/O operations via git CLI and metadata store.
//!
//! Performs the actual split: creates new branches at the split point,
//! updates metadata for reparented children, deletes the source metadata,
//! and records a transaction receipt.

use std::path::Path;
use std::process::Command;

use scp_stack::engine::transactional_engine::{TransactionConfig, TransactionalStackOps};
use scp_stack::BranchName;

use super::calc;
use super::data::{SplitError, SplitPlan, StackSplitOptions, StackSplitResult};

/// Run the full stack split operation.
///
/// # Errors
///
/// Returns `SplitError` for any failure during the split operation.
pub fn run_stack_split(
    workdir: &Path,
    options: &StackSplitOptions,
    tx_config: &TransactionConfig,
    metadata_store: &dyn SplitMetadataStoreOps,
) -> Result<StackSplitResult, SplitError> {
    let stack = load_stack_from_metadata(metadata_store)?;

    calc::validate_split_preconditions(&stack, &options.branch, &options.trunk)?;

    let source_revision = metadata_store
        .branch_revision(options.branch.as_str())
        .map_err(|e| SplitError::MetadataError(e.to_string()))?
        .ok_or_else(|| SplitError::NotTracked(options.branch.clone()))?;

    let (lower_name, upper_name) = calc::resolve_branch_names(
        &options.branch,
        options.lower_name.as_ref(),
        options.upper_name.as_ref(),
    );

    ensure_branches_dont_exist(workdir, &[&lower_name, &upper_name])?;

    let parent_revision = get_parent_revision(metadata_store, &options.branch)?;

    let plan = calc::plan_split(
        &stack,
        &options.branch,
        &lower_name,
        &upper_name,
        &options.at_commit,
        &source_revision,
        &parent_revision,
    )?;

    execute_split(workdir, &plan)?;

    write_split_metadata(metadata_store, &plan)?;

    for child in &plan.children_to_reparent {
        reparent_child(metadata_store, child, &plan.upper_branch)?;
    }

    metadata_store
        .delete_branch_meta(options.branch.as_str())
        .map_err(|e| SplitError::MetadataError(e.to_string()))?;

    record_split_transaction(tx_config, options.branch.as_str(), &plan);

    Ok(StackSplitResult {
        source_branch: options.branch.clone(),
        lower_branch: plan.lower_branch,
        upper_branch: plan.upper_branch,
        split_commit: plan.split_commit,
        reparented_children: plan.children_to_reparent,
    })
}

/// Abstraction over metadata operations needed for split.
pub trait SplitMetadataStoreOps {
    fn read_branch_meta(
        &self,
        branch: &str,
    ) -> Result<Option<scp_stack::domain::metadata::BranchMetadata>, SplitError>;

    fn write_branch_meta(
        &self,
        branch: &str,
        metadata: &scp_stack::domain::metadata::BranchMetadata,
    ) -> Result<(), SplitError>;

    fn delete_branch_meta(&self, branch: &str) -> Result<(), SplitError>;

    fn branch_revision(&self, branch: &str) -> Result<Option<String>, SplitError>;

    fn list_tracked_branches(&self) -> Result<Vec<String>, SplitError>;

    fn read_trunk(&self) -> Result<Option<String>, SplitError>;
}

fn load_stack_from_metadata(
    store: &dyn SplitMetadataStoreOps,
) -> Result<scp_stack::Stack, SplitError> {
    let trunk = store.read_trunk()?.unwrap_or_else(|| "main".to_string());

    let mut stack = scp_stack::Stack::new(BranchName::new(&trunk));

    for branch_name in store.list_tracked_branches()? {
        if let Some(meta) = store.read_branch_meta(&branch_name)? {
            stack
                .add_branch(scp_stack::StackBranch {
                    name: BranchName::new(&branch_name),
                    parent: Some(BranchName::new(&meta.parent_branch_name)),
                    children: Vec::new(),
                    needs_restack: false,
                    pr_info: None,
                })
                .ok();
        }
    }

    let branch_names: Vec<String> = stack
        .branches
        .iter()
        .map(|b| b.name.as_str().to_string())
        .collect();
    for name in &branch_names {
        let parent = stack
            .branches
            .iter()
            .find(|b| b.name.as_str() == name)
            .and_then(|b| b.parent.clone());
        if let Some(p) = parent {
            if let Some(parent_branch) = stack.branches.iter_mut().find(|b| b.name == p) {
                parent_branch.children.push(BranchName::new(name));
            }
        }
    }

    Ok(stack)
}

fn get_parent_revision(
    store: &dyn SplitMetadataStoreOps,
    branch: &BranchName,
) -> Result<String, SplitError> {
    let meta = store
        .read_branch_meta(branch.as_str())?
        .ok_or_else(|| SplitError::NotTracked(branch.clone()))?;

    store
        .branch_revision(&meta.parent_branch_name)?
        .ok_or_else(|| {
            SplitError::MetadataError(format!(
                "parent '{}' has no revision",
                meta.parent_branch_name
            ))
        })
}

fn ensure_branches_dont_exist(workdir: &Path, names: &[&BranchName]) -> Result<(), SplitError> {
    for name in names {
        let output = Command::new("git")
            .args(["rev-parse", "--verify", name.as_str()])
            .current_dir(workdir)
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                return Err(SplitError::BranchAlreadyExists((*name).clone()));
            }
        }
    }
    Ok(())
}

fn execute_split(workdir: &Path, plan: &SplitPlan) -> Result<(), SplitError> {
    let lower_output = Command::new("git")
        .args(["branch", plan.lower_branch.as_str(), &plan.split_commit])
        .current_dir(workdir)
        .output();

    match lower_output {
        Ok(out) if out.status.success() => {}
        Ok(out) => {
            return Err(SplitError::GitError(format!(
                "failed to create lower branch '{}': {}",
                plan.lower_branch,
                String::from_utf8_lossy(&out.stderr)
            )));
        }
        Err(e) => return Err(SplitError::IoError(e.to_string())),
    }

    let upper_output = Command::new("git")
        .args(["branch", plan.upper_branch.as_str(), &plan.source_tip])
        .current_dir(workdir)
        .output();

    match upper_output {
        Ok(out) if out.status.success() => {}
        Ok(out) => {
            return Err(SplitError::GitError(format!(
                "failed to create upper branch '{}': {}",
                plan.upper_branch,
                String::from_utf8_lossy(&out.stderr)
            )));
        }
        Err(e) => return Err(SplitError::IoError(e.to_string())),
    }

    Ok(())
}

fn write_split_metadata(
    store: &dyn SplitMetadataStoreOps,
    plan: &SplitPlan,
) -> Result<(), SplitError> {
    let lower_meta = scp_stack::domain::metadata::BranchMetadata::new(
        plan.lower_parent.as_str(),
        &plan.lower_parent_revision,
    );
    store.write_branch_meta(plan.lower_branch.as_str(), &lower_meta)?;

    let upper_meta = scp_stack::domain::metadata::BranchMetadata::new(
        plan.lower_branch.as_str(),
        &plan.split_commit,
    );
    store.write_branch_meta(plan.upper_branch.as_str(), &upper_meta)?;

    Ok(())
}

fn reparent_child(
    store: &dyn SplitMetadataStoreOps,
    child: &BranchName,
    new_parent: &BranchName,
) -> Result<(), SplitError> {
    let child_meta = store
        .read_branch_meta(child.as_str())?
        .ok_or_else(|| SplitError::NotTracked(child.clone()))?;

    let new_parent_revision = store
        .branch_revision(new_parent.as_str())?
        .unwrap_or_default();

    let updated =
        scp_stack::domain::metadata::BranchMetadata::new(new_parent.as_str(), &new_parent_revision)
            .with_pr(
                child_meta.pr_info.as_ref().map(|p| p.number).unwrap_or(0),
                &child_meta
                    .pr_info
                    .as_ref()
                    .map(|p| p.state.clone())
                    .unwrap_or_default(),
                child_meta.pr_info.as_ref().and_then(|p| p.is_draft),
            );

    store.write_branch_meta(child.as_str(), &updated)
}

fn record_split_transaction(config: &TransactionConfig, source: &str, plan: &SplitPlan) {
    let store = ReceiptOnlyStore;
    let ops = TransactionalStackOps::new(store, config.clone());
    let targets = vec![
        plan.lower_branch.as_str().to_string(),
        plan.upper_branch.as_str().to_string(),
    ];
    let _ = ops.split(source, &targets);
}

struct ReceiptOnlyStore;

impl scp_stack::engine::transactional_engine::MetadataStore for ReceiptOnlyStore {
    fn read(
        &self,
        _: &str,
    ) -> scp_stack::error::Result<Option<scp_stack::domain::metadata::BranchMetadata>> {
        Ok(None)
    }

    fn write(
        &self,
        _: &str,
        _: &scp_stack::domain::metadata::BranchMetadata,
    ) -> scp_stack::error::Result<()> {
        Ok(())
    }

    fn delete(&self, _: &str) -> scp_stack::error::Result<()> {
        Ok(())
    }

    fn branch_revision(&self, _: &str) -> scp_stack::error::Result<Option<String>> {
        Ok(None)
    }

    fn list_branches(&self) -> scp_stack::error::Result<Vec<String>> {
        Ok(vec![])
    }

    fn read_trunk(&self) -> scp_stack::error::Result<Option<String>> {
        Ok(Some("main".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    fn bn(name: &str) -> BranchName {
        BranchName::new(name.to_string())
    }

    struct MockStore {
        metadata: RefCell<HashMap<String, scp_stack::domain::metadata::BranchMetadata>>,
        revisions: HashMap<String, String>,
        trunk: Option<String>,
    }

    impl MockStore {
        fn new() -> Self {
            Self {
                metadata: RefCell::new(HashMap::new()),
                revisions: HashMap::new(),
                trunk: Some("main".to_string()),
            }
        }

        fn add_branch(self, name: &str, parent: &str, parent_rev: &str, current_rev: &str) -> Self {
            self.metadata.borrow_mut().insert(
                name.to_string(),
                scp_stack::domain::metadata::BranchMetadata::new(parent, parent_rev),
            );
            self.revisions.len();
            let _ = (parent_rev, current_rev);
            self
        }

        fn with_revisions(mut self, revs: Vec<(&str, &str)>) -> Self {
            for (name, rev) in revs {
                self.revisions.insert(name.to_string(), rev.to_string());
            }
            self
        }
    }

    impl SplitMetadataStoreOps for MockStore {
        fn read_branch_meta(
            &self,
            branch: &str,
        ) -> Result<Option<scp_stack::domain::metadata::BranchMetadata>, SplitError> {
            Ok(self.metadata.borrow().get(branch).cloned())
        }

        fn write_branch_meta(
            &self,
            branch: &str,
            metadata: &scp_stack::domain::metadata::BranchMetadata,
        ) -> Result<(), SplitError> {
            self.metadata
                .borrow_mut()
                .insert(branch.to_string(), metadata.clone());
            Ok(())
        }

        fn delete_branch_meta(&self, branch: &str) -> Result<(), SplitError> {
            self.metadata.borrow_mut().remove(branch);
            Ok(())
        }

        fn branch_revision(&self, branch: &str) -> Result<Option<String>, SplitError> {
            Ok(self.revisions.get(branch).cloned())
        }

        fn list_tracked_branches(&self) -> Result<Vec<String>, SplitError> {
            Ok(self.metadata.borrow().keys().cloned().collect())
        }

        fn read_trunk(&self) -> Result<Option<String>, SplitError> {
            Ok(self.trunk.clone())
        }
    }

    fn make_test_store() -> MockStore {
        MockStore::new()
            .add_branch("feat-a", "main", "trunk-rev", "rev-a")
            .add_branch("feat-a-1", "feat-a", "rev-a", "rev-a1")
            .add_branch("feat-a-2", "feat-a-1", "rev-a1", "rev-a2")
            .with_revisions(vec![
                ("main", "trunk-rev"),
                ("feat-a", "rev-a"),
                ("feat-a-1", "rev-a1"),
                ("feat-a-2", "rev-a2"),
            ])
    }

    fn make_tx_config() -> TransactionConfig {
        TransactionConfig {
            git_dir: std::path::PathBuf::from("/tmp/test-split/.git"),
            workdir: std::path::PathBuf::from("/tmp/test-split"),
            trunk: "main".to_string(),
        }
    }

    #[test]
    fn metadata_written_for_both_branches() {
        let store = make_test_store();

        let plan = SplitPlan {
            lower_branch: bn("feat-a-lower"),
            upper_branch: bn("feat-a-upper"),
            lower_parent: bn("main"),
            lower_parent_revision: "trunk-rev".to_string(),
            split_commit: "split-rev".to_string(),
            source_tip: "rev-a".to_string(),
            children_to_reparent: vec![bn("feat-a-1")],
        };

        write_split_metadata(&store, &plan).expect("write metadata");

        let lower_meta = store
            .read_branch_meta("feat-a-lower")
            .unwrap()
            .expect("lower meta");
        assert_eq!(lower_meta.parent_branch_name, "main");

        let upper_meta = store
            .read_branch_meta("feat-a-upper")
            .unwrap()
            .expect("upper meta");
        assert_eq!(upper_meta.parent_branch_name, "feat-a-lower");
    }

    #[test]
    fn reparent_child_updates_parent() {
        let store = make_test_store();

        reparent_child(&store, &bn("feat-a-1"), &bn("feat-a-upper")).expect("reparent");

        let child_meta = store
            .read_branch_meta("feat-a-1")
            .unwrap()
            .expect("meta exists");
        assert_eq!(child_meta.parent_branch_name, "feat-a-upper");
    }

    #[test]
    fn source_metadata_deleted() {
        let store = make_test_store();
        store.delete_branch_meta("feat-a").expect("delete source");
        assert!(store.read_branch_meta("feat-a").unwrap().is_none());
    }

    #[test]
    fn load_stack_builds_graph() {
        let store = make_test_store();
        let stack = load_stack_from_metadata(&store).expect("load stack");

        assert!(stack.branches.iter().any(|b| b.name.as_str() == "feat-a"));
        assert!(stack.branches.iter().any(|b| b.name.as_str() == "feat-a-1"));
    }
}
