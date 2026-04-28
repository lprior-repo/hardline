//! Metadata serialization and parsing

use std::{collections::BTreeMap, rc::Rc};

use crate::dag::BranchId;

/// `StackMetadata` - Storage layer for branch metadata with backend delegation
#[derive(Clone)]
pub struct StackMetadata {
    /// `BranchId` -> Option<BranchId> (parent, None for trunk)
    pub(super) parents: BTreeMap<BranchId, Option<BranchId>>,
    /// `BranchId` -> Vec<BranchId> (children)
    pub(super) children: BTreeMap<BranchId, Vec<BranchId>>,
    /// Backend for persistence
    pub(super) backend: Rc<dyn super::MetadataBackend>,
}

impl std::fmt::Debug for StackMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StackMetadata")
            .field("parents", &self.parents)
            .field("children", &self.children)
            .field("backend", &"<backend trait object>")
            .finish()
    }
}
