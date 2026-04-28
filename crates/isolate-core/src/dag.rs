//! Branch DAG - Directed Acyclic Graph for branch management
//!
//! This module provides DAG operations for tracking branch relationships.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::error::Error;

/// Branch identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct BranchId(String);

impl BranchId {
    pub fn new(id: impl Into<String>) -> Result<Self, Error> {
        let id = id.into();
        if id.is_empty() {
            Err(Error::InvalidId("BranchId cannot be empty".into()))
        } else {
            Ok(Self(id))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for BranchId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A node in the branch DAG
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchNode {
    pub id: BranchId,
    pub parent: Option<BranchId>,
    pub children: Vec<BranchId>,
    pub commit_id: Option<String>,
}

/// Branch DAG error
#[derive(Clone)]
pub enum DagError {
    CycleDetected,
    NodeNotFound,
    InvalidParent,
}

impl std::fmt::Display for DagError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CycleDetected => write!(f, "Cycle detected in branch graph"),
            Self::NodeNotFound => write!(f, "Branch node not found"),
            Self::InvalidParent => write!(f, "Invalid parent branch"),
        }
    }
}

impl std::fmt::Debug for DagError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl std::error::Error for DagError {}

/// Branch Directed Acyclic Graph
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BranchDag {
    nodes: HashMap<BranchId, BranchNode>,
    roots: HashSet<BranchId>,
}

impl BranchDag {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn add_branch(&mut self, id: BranchId, parent: Option<BranchId>) -> Result<(), Error> {
        // Validate parent exists if specified
        if let Some(ref parent_id) = parent {
            if !self.nodes.contains_key(parent_id) {
                return Err(Error::InvalidInput(format!(
                    "Parent branch {:?} not found",
                    parent_id
                )));
            }
        }

        // Check for cycles
        if let Some(ref parent_id) = parent {
            if self.would_create_cycle(&id, parent_id) {
                return Err(Error::InvalidInput(
                    "Adding this branch would create a cycle".into(),
                ));
            }
        }

        // Add node
        let node = BranchNode {
            id: id.clone(),
            parent: parent.clone(),
            children: vec![],
            commit_id: None,
        };

        // Update parent's children
        if let Some(parent_id) = parent {
            if let Some(parent_node) = self.nodes.get_mut(&parent_id) {
                parent_node.children.push(id.clone());
            }
        } else {
            self.roots.insert(id.clone());
        }

        self.nodes.insert(id, node);
        Ok(())
    }

    pub fn remove_branch(&mut self, id: &BranchId) -> Result<(), Error> {
        let node = self.nodes.remove(id).ok_or(DagError::NodeNotFound)?;

        // Update parent
        if let Some(parent_id) = &node.parent {
            if let Some(parent_node) = self.nodes.get_mut(parent_id) {
                parent_node.children.retain(|c| c != id);
            }
        } else {
            self.roots.remove(id);
        }

        // Re-parent children to this node's parent
        for child_id in &node.children {
            if let Some(child_node) = self.nodes.get_mut(child_id) {
                child_node.parent = node.parent.clone();
                if let Some(ref new_parent) = node.parent {
                    if let Some(parent_node) = self.nodes.get_mut(new_parent) {
                        parent_node.children.push(child_id.clone());
                    }
                } else {
                    self.roots.insert(child_id.clone());
                }
            }
        }

        Ok(())
    }

    pub fn get_branch(&self, id: &BranchId) -> Option<&BranchNode> {
        self.nodes.get(id)
    }

    pub fn get_children(&self, id: &BranchId) -> Vec<&BranchId> {
        self.nodes
            .get(id)
            .map(|n| n.children.iter().collect())
            .unwrap_or_default()
    }

    pub fn get_parent(&self, id: &BranchId) -> Option<&BranchId> {
        self.nodes.get(id).and_then(|n| n.parent.as_ref())
    }

    pub fn get_roots(&self) -> Vec<&BranchId> {
        self.roots.iter().collect()
    }

    pub fn get_leaves(&self) -> Vec<&BranchId> {
        self.nodes
            .values()
            .filter(|n| n.children.is_empty())
            .map(|n| &n.id)
            .collect()
    }

    pub fn get_ancestors(&self, id: &BranchId) -> Vec<BranchId> {
        let mut ancestors = vec![];
        let mut current = self.get_parent(id);
        while let Some(parent_id) = current {
            ancestors.push(parent_id.clone());
            current = self.get_parent(parent_id);
        }
        ancestors
    }

    pub fn get_descendants(&self, id: &BranchId) -> Vec<BranchId> {
        let mut descendants = vec![];
        let mut to_visit = vec![id.clone()];

        while let Some(current) = to_visit.pop() {
            let children = self.get_children(&current);
            for child in children {
                descendants.push(child.clone());
                to_visit.push(child.clone());
            }
        }

        descendants
    }

    fn would_create_cycle(&self, new_id: &BranchId, parent_id: &BranchId) -> bool {
        // Check if adding new_id as child of parent_id would create a cycle
        // This happens if new_id is already an ancestor of parent_id
        let ancestors = self.get_ancestors(parent_id);
        ancestors.iter().any(|a| a == new_id)
    }

    pub fn topological_sort(&self) -> Result<Vec<BranchId>, Error> {
        let mut in_degree: HashMap<BranchId, usize> = HashMap::new();
        let mut result = vec![];
        let mut queue: Vec<BranchId> = self.roots.iter().cloned().collect();

        // Initialize in-degrees
        for id in self.nodes.keys() {
            in_degree.insert(id.clone(), 0);
        }

        // Calculate in-degrees from parents
        for node in self.nodes.values() {
            if let Some(parent_id) = &node.parent {
                if let Some(degree) = in_degree.get_mut(parent_id) {
                    *degree += 1;
                }
            }
        }

        // Process queue
        while let Some(current) = queue.pop() {
            result.push(current.clone());

            for child in self.get_children(&current) {
                if let Some(degree) = in_degree.get_mut(child) {
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push(child.clone());
                    }
                }
            }
        }

        // Check for cycles
        if result.len() != self.nodes.len() {
            return Err(Error::InvalidInput("Cycle detected in branch graph".into()));
        }

        Ok(result)
    }
}

impl From<DagError> for Error {
    fn from(err: DagError) -> Self {
        match err {
            DagError::CycleDetected => Error::InvalidInput("Cycle detected in branch graph".into()),
            DagError::NodeNotFound => Error::NotFound("Branch node not found".into()),
            DagError::InvalidParent => Error::InvalidInput("Invalid parent branch".into()),
        }
    }
}
