//! Action layer for stack detach - I/O operations via git CLI.
//!
//! Performs the actual detach: updates metadata refs for reparented children,
//! deletes the detached branch's metadata ref, optionally deletes the git branch.

use std::path::Path;
use std::process::Command;

use scp_stack::{BranchName, Stack};
use scp_stack::engine::transactional_engine::{TransactionConfig, TransactionalStackOps};

use super::calc;
use super::data::{DetachError, StackDetachOptions, StackDetachResult};

/// Run the full stack detach operation.
///
/// # Errors
///
/// Returns `DetachError` for any failure during the detach operation.
pub fn run_stack_detach(
    workdir: &Path,
    stack: &Stack,
    options: &StackDetachOptions,
    tx_config: &TransactionConfig,
    metadata_store: &dyn MetadataStoreOps,
) -> Result<StackDetachResult, DetachError> {
    // 1. Validate preconditions
    calc::validate_detach_preconditions(stack, &options.branch, &options.trunk)?;

    // 2. Plan the detach
    let (previous_parent, children_to_reparent) =
        calc::plan_detach(stack, &options.branch)?;

    // 3. Reparent children in metadata
    for (child, new_parent) in &children_to_reparent {
        let child_meta = metadata_store
            .read_branch_meta(child.as_str())
            .map_err(|e| DetachError::MetadataError(e.to_string()))?
            .ok_or_else(|| DetachError::NotTracked(child.clone()))?;

        let new_parent_revision = metadata_store
            .branch_revision(new_parent.as_str())
            .map_err(|e| DetachError::MetadataError(e.to_string()))?
            .unwrap_or_default();

        let updated = scp_stack::domain::metadata::BranchMetadata::new(
            new_parent.as_str(),
            &new_parent_revision,
        )
        .with_pr(
            child_meta.pr_info.as_ref().map(|p| p.number).unwrap_or(0),
            &child_meta
                .pr_info
                .as_ref()
                .map(|p| p.state.clone())
                .unwrap_or_default(),
            child_meta.pr_info.as_ref().and_then(|p| p.is_draft),
        );

        metadata_store
            .write_branch_meta(child.as_str(), &updated)
            .map_err(|e| DetachError::MetadataError(e.to_string()))?;
    }

    // 4. Delete the detached branch's metadata
    metadata_store
        .delete_branch_meta(options.branch.as_str())
        .map_err(|e| DetachError::MetadataError(e.to_string()))?;

    // 5. Optionally delete the local git branch
    let branch_deleted = if options.delete_branch {
        delete_local_branch(workdir, options.branch.as_str())
    } else {
        false
    };

    // 6. Record transaction (best-effort — don't fail detach if receipt fails)
    record_detach_transaction(tx_config, options.branch.as_str());

    Ok(StackDetachResult {
        branch: options.branch.clone(),
        previous_parent,
        reparented_children: children_to_reparent
            .into_iter()
            .map(|(c, _)| c)
            .collect(),
        branch_deleted,
    })
}

/// Abstraction over metadata operations needed for detach.
///
/// This trait decouples the detach handler from the concrete metadata store,
/// making it testable without a real git repo.
pub trait MetadataStoreOps {
    /// Read branch metadata. Returns None if not tracked.
    fn read_branch_meta(
        &self,
        branch: &str,
    ) -> Result<Option<scp_stack::domain::metadata::BranchMetadata>, DetachError>;

    /// Write branch metadata.
    fn write_branch_meta(
        &self,
        branch: &str,
        metadata: &scp_stack::domain::metadata::BranchMetadata,
    ) -> Result<(), DetachError>;

    /// Delete branch metadata.
    fn delete_branch_meta(&self, branch: &str) -> Result<(), DetachError>;

    /// Get current revision of a branch. Returns None if not found.
    fn branch_revision(&self, branch: &str) -> Result<Option<String>, DetachError>;
}

/// Git-based metadata store using `git show` and `git update-ref`.
pub struct GitMetadataStore<'a> {
    workdir: &'a Path,
}

impl<'a> GitMetadataStore<'a> {
    pub fn new(workdir: &'a Path) -> Self {
        Self { workdir }
    }
}

impl MetadataStoreOps for GitMetadataStore<'_> {
    fn read_branch_meta(
        &self,
        branch: &str,
    ) -> Result<Option<scp_stack::domain::metadata::BranchMetadata>, DetachError> {
        let ref_name = format!("refs/branch-metadata/{branch}");
        let output = Command::new("git")
            .args(["show", &ref_name])
            .current_dir(self.workdir)
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let json = String::from_utf8_lossy(&out.stdout);
                let meta = scp_stack::domain::metadata::BranchMetadata::from_json(&json)
                    .map_err(|e| DetachError::MetadataError(e.to_string()))?;
                Ok(Some(meta))
            }
            _ => Ok(None),
        }
    }

    fn write_branch_meta(
        &self,
        branch: &str,
        metadata: &scp_stack::domain::metadata::BranchMetadata,
    ) -> Result<(), DetachError> {
        let json = metadata
            .to_json()
            .map_err(|e| DetachError::MetadataError(e.to_string()))?;
        let ref_name = format!("refs/branch-metadata/{branch}");

        let output = Command::new("git")
            .args(["hash-object", "-w", "--stdin"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .current_dir(self.workdir)
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                if let Some(stdin) = child.stdin.as_mut() {
                    let _ = stdin.write_all(json.as_bytes());
                }
                child.wait_with_output()
            });

        let oid = match output {
            Ok(out) if out.status.success() => {
                String::from_utf8_lossy(&out.stdout).trim().to_string()
            }
            _ => return Err(DetachError::MetadataError("hash-object failed".to_string())),
        };

        let update_output = Command::new("git")
            .args(["update-ref", &ref_name, &oid])
            .current_dir(self.workdir)
            .output();

        match update_output {
            Ok(out) if out.status.success() => Ok(()),
            Ok(out) => Err(DetachError::MetadataError(
                String::from_utf8_lossy(&out.stderr).to_string(),
            )),
            Err(e) => Err(DetachError::IoError(e.to_string())),
        }
    }

    fn delete_branch_meta(&self, branch: &str) -> Result<(), DetachError> {
        let ref_name = format!("refs/branch-metadata/{branch}");
        let output = Command::new("git")
            .args(["update-ref", "-d", &ref_name])
            .current_dir(self.workdir)
            .output();

        match output {
            Ok(out) if out.status.success() => Ok(()),
            // Already deleted is fine
            Ok(out) if String::from_utf8_lossy(&out.stderr).contains("not found") => Ok(()),
            Ok(out) => Err(DetachError::MetadataError(
                String::from_utf8_lossy(&out.stderr).to_string(),
            )),
            Err(e) => Err(DetachError::IoError(e.to_string())),
        }
    }

    fn branch_revision(&self, branch: &str) -> Result<Option<String>, DetachError> {
        let output = Command::new("git")
            .args(["rev-parse", branch])
            .current_dir(self.workdir)
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let rev = String::from_utf8_lossy(&out.stdout).trim().to_string();
                Ok(Some(rev))
            }
            _ => Ok(None),
        }
    }
}

/// Delete a local git branch.
fn delete_local_branch(workdir: &Path, branch: &str) -> bool {
    let output = Command::new("git")
        .args(["branch", "-D", branch])
        .current_dir(workdir)
        .output();

    match output {
        Ok(out) => out.status.success(),
        Err(_) => false,
    }
}

/// Best-effort transaction recording.
fn record_detach_transaction(config: &TransactionConfig, branch: &str) {
    // TransactionalStackOps needs a MetadataStore, but for receipt-only purposes
    // we create a minimal implementation that doesn't read/write real data.
    // The actual metadata mutations are already done by the caller.
    let store = ReceiptOnlyStore;
    let ops = TransactionalStackOps::new(store, config.clone());
    let _ = ops.detach(branch);
}

/// Minimal metadata store for transaction receipt recording only.
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

    /// In-memory metadata store for testing.
    struct MockStore {
        metadata: RefCell<HashMap<String, scp_stack::domain::metadata::BranchMetadata>>,
        revisions: HashMap<String, String>,
    }

    impl MockStore {
        fn new() -> Self {
            Self {
                metadata: RefCell::new(HashMap::new()),
                revisions: HashMap::new(),
            }
        }

        fn add_branch(
            self,
            name: &str,
            parent: &str,
            parent_rev: &str,
            current_rev: &str,
        ) -> Self {
            self.metadata.borrow_mut().insert(
                name.to_string(),
                scp_stack::domain::metadata::BranchMetadata::new(parent, parent_rev),
            );
            self.revisions
                .clone()
                .into_iter()
                .chain(std::iter::once((
                    name.to_string(),
                    current_rev.to_string(),
                )))
                .for_each(|(k, v)| {
                    let _ = &k;
                    let _ = &v;
                });
            self
        }

        fn with_revisions(mut self, revs: Vec<(&str, &str)>) -> Self {
            for (name, rev) in revs {
                self.revisions.insert(name.to_string(), rev.to_string());
            }
            self
        }
    }

    impl MetadataStoreOps for MockStore {
        fn read_branch_meta(
            &self,
            branch: &str,
        ) -> Result<Option<scp_stack::domain::metadata::BranchMetadata>, DetachError> {
            Ok(self.metadata.borrow().get(branch).cloned())
        }

        fn write_branch_meta(
            &self,
            branch: &str,
            metadata: &scp_stack::domain::metadata::BranchMetadata,
        ) -> Result<(), DetachError> {
            self.metadata
                .borrow_mut()
                .insert(branch.to_string(), metadata.clone());
            Ok(())
        }

        fn delete_branch_meta(&self, branch: &str) -> Result<(), DetachError> {
            self.metadata.borrow_mut().remove(branch);
            Ok(())
        }

        fn branch_revision(&self, branch: &str) -> Result<Option<String>, DetachError> {
            Ok(self.revisions.get(branch).cloned())
        }
    }

    fn make_test_stack() -> Stack {
        let mut stack = Stack::new(bn("main"));
        stack
            .add_branch(scp_stack::StackBranch {
                name: bn("feat-a"),
                parent: Some(bn("main")),
                children: vec![bn("feat-a-1")],
                needs_restack: false,
                pr_info: None,
            })
            .ok();
        stack
            .add_branch(scp_stack::StackBranch {
                name: bn("feat-a-1"),
                parent: Some(bn("feat-a")),
                children: vec![bn("feat-a-2")],
                needs_restack: false,
                pr_info: None,
            })
            .ok();
        stack
            .add_branch(scp_stack::StackBranch {
                name: bn("feat-a-2"),
                parent: Some(bn("feat-a-1")),
                children: vec![],
                needs_restack: false,
                pr_info: None,
            })
            .ok();
        stack
    }

    fn make_tx_config() -> TransactionConfig {
        TransactionConfig {
            git_dir: std::path::PathBuf::from("/tmp/test-detach/.git"),
            workdir: std::path::PathBuf::from("/tmp/test-detach"),
            trunk: "main".to_string(),
        }
    }

    #[test]
    fn detach_leaf_branch_no_children() {
        let stack = make_test_stack();
        let store = MockStore::new()
            .add_branch("feat-a", "main", "trunk-rev", "rev-a")
            .add_branch("feat-a-1", "feat-a", "rev-a", "rev-a1")
            .add_branch("feat-a-2", "feat-a-1", "rev-a1", "rev-a2")
            .with_revisions(vec![
                ("main", "trunk-rev"),
                ("feat-a", "rev-a"),
                ("feat-a-1", "rev-a1"),
                ("feat-a-2", "rev-a2"),
            ]);
        let config = make_tx_config();

        let result = run_stack_detach(
            Path::new("/tmp/test-detach"),
            &stack,
            &StackDetachOptions {
                branch: bn("feat-a-2"),
                trunk: bn("main"),
                force: true,
                delete_branch: false,
            },
            &config,
            &store,
        )
        .expect("detach should succeed");

        assert_eq!(result.branch, bn("feat-a-2"));
        assert_eq!(result.previous_parent, bn("feat-a-1"));
        assert!(result.reparented_children.is_empty());
        assert!(!result.branch_deleted);
        // Metadata should be deleted
        assert!(store.read_branch_meta("feat-a-2").unwrap().is_none());
    }

    #[test]
    fn detach_mid_branch_reparents_children() {
        let stack = make_test_stack();
        let store = MockStore::new()
            .add_branch("feat-a", "main", "trunk-rev", "rev-a")
            .add_branch("feat-a-1", "feat-a", "rev-a", "rev-a1")
            .add_branch("feat-a-2", "feat-a-1", "rev-a1", "rev-a2")
            .with_revisions(vec![
                ("main", "trunk-rev"),
                ("feat-a", "rev-a"),
                ("feat-a-1", "rev-a1"),
                ("feat-a-2", "rev-a2"),
            ]);
        let config = make_tx_config();

        let result = run_stack_detach(
            Path::new("/tmp/test-detach"),
            &stack,
            &StackDetachOptions {
                branch: bn("feat-a-1"),
                trunk: bn("main"),
                force: true,
                delete_branch: false,
            },
            &config,
            &store,
        )
        .expect("detach should succeed");

        assert_eq!(result.branch, bn("feat-a-1"));
        assert_eq!(result.previous_parent, bn("feat-a"));
        assert_eq!(result.reparented_children, vec![bn("feat-a-2")]);

        // feat-a-2 should be reparented to feat-a
        let child_meta = store
            .read_branch_meta("feat-a-2")
            .unwrap()
            .expect("metadata exists");
        assert_eq!(child_meta.parent_branch_name, "feat-a");

        // feat-a-1 metadata should be deleted
        assert!(store.read_branch_meta("feat-a-1").unwrap().is_none());
    }

    #[test]
    fn detach_first_level_reparents_to_trunk() {
        let stack = make_test_stack();
        let store = MockStore::new()
            .add_branch("feat-a", "main", "trunk-rev", "rev-a")
            .add_branch("feat-a-1", "feat-a", "rev-a", "rev-a1")
            .add_branch("feat-a-2", "feat-a-1", "rev-a1", "rev-a2")
            .with_revisions(vec![
                ("main", "trunk-rev"),
                ("feat-a", "rev-a"),
                ("feat-a-1", "rev-a1"),
                ("feat-a-2", "rev-a2"),
            ]);
        let config = make_tx_config();

        let result = run_stack_detach(
            Path::new("/tmp/test-detach"),
            &stack,
            &StackDetachOptions {
                branch: bn("feat-a"),
                trunk: bn("main"),
                force: true,
                delete_branch: false,
            },
            &config,
            &store,
        )
        .expect("detach should succeed");

        assert_eq!(result.previous_parent, bn("main"));
        assert_eq!(result.reparented_children, vec![bn("feat-a-1")]);

        // feat-a-1 reparented to main
        let child_meta = store
            .read_branch_meta("feat-a-1")
            .unwrap()
            .expect("metadata exists");
        assert_eq!(child_meta.parent_branch_name, "main");

        // feat-a metadata deleted
        assert!(store.read_branch_meta("feat-a").unwrap().is_none());
    }

    #[test]
    fn detach_rejects_trunk() {
        let stack = make_test_stack();
        let store = MockStore::new();
        let config = make_tx_config();

        let result = run_stack_detach(
            Path::new("/tmp/test-detach"),
            &stack,
            &StackDetachOptions {
                branch: bn("main"),
                trunk: bn("main"),
                force: true,
                delete_branch: false,
            },
            &config,
            &store,
        );

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            DetachError::CannotDetachTrunk(_)
        ));
    }

    #[test]
    fn detach_rejects_untracked() {
        let stack = make_test_stack();
        let store = MockStore::new();
        let config = make_tx_config();

        let result = run_stack_detach(
            Path::new("/tmp/test-detach"),
            &stack,
            &StackDetachOptions {
                branch: bn("ghost"),
                trunk: bn("main"),
                force: true,
                delete_branch: false,
            },
            &config,
            &store,
        );

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DetachError::NotTracked(_)));
    }
}
