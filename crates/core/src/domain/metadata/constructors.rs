//! StackMetadata constructors and factories

use std::collections::BTreeMap;
use std::rc::Rc;

use petgraph::graph::{DiGraph, NodeIndex};

use crate::dag::BranchId;
use crate::Error;

use super::entities::StackMetadata;

impl StackMetadata {
    /// Build a directed graph from parents mapping
    pub fn build_graph(&self) -> (DiGraph<BranchId, ()>, BTreeMap<BranchId, NodeIndex>) {
        let (graph, indices) = self.parents.keys().cloned().fold(
            (DiGraph::new(), BTreeMap::new()),
            |(mut graph, mut indices), branch| {
                let node_idx = graph.add_node(branch.clone());
                indices.insert(branch, node_idx);
                (graph, indices)
            },
        );

        let graph = self
            .parents
            .iter()
            .filter_map(|(branch, maybe_parent)| {
                maybe_parent
                    .as_ref()
                    .and_then(|parent| indices.get(parent).copied())
                    .zip(indices.get(branch).copied())
            })
            .fold(graph, |mut graph, (parent_idx, branch_idx)| {
                graph.add_edge(parent_idx, branch_idx, ());
                graph
            });

        (graph, indices)
    }

    /// Create new metadata with backend
    ///
    /// # Errors
    /// Returns an error if the backend fails to save initial metadata.
    pub fn new(backend: Rc<dyn super::MetadataBackend>) -> Result<Self, Error> {
        let trunk = BranchId::new("trunk");
        let parents = BTreeMap::from_iter([(trunk, None)]);
        let children = BTreeMap::new();

        let metadata = Self {
            parents,
            children,
            backend,
        };

        metadata.save()?;
        Ok(metadata)
    }

    /// Load metadata from backend
    ///
    /// # Errors
    /// Returns an error if the backend fails to load or if metadata is corrupted.
    pub fn load(backend: Rc<dyn super::MetadataBackend>) -> Result<Self, Error> {
        let data = backend.load()?;

        if data.is_empty() {
            return Self::new(backend);
        }

        let (parents, children) = Self::parse_metadata(&data)?;

        Ok(Self {
            parents,
            children,
            backend,
        })
    }

    /// Parse metadata from bytes
    fn parse_metadata(
        data: &[u8],
    ) -> Result<
        (
            BTreeMap<BranchId, Option<BranchId>>,
            BTreeMap<BranchId, Vec<BranchId>>,
        ),
        Error,
    > {
        let text = String::from_utf8(data.to_vec())
            .map_err(|_| Error::invalid_state("Metadata corrupted: invalid UTF-8".to_string()))?;

        text.lines().try_fold(
            (BTreeMap::new(), BTreeMap::<BranchId, Vec<BranchId>>::new()),
            |(mut parents, mut children), raw_line| {
                let line = raw_line.trim();
                if line.is_empty() || line.starts_with('#') {
                    return Ok((parents, children));
                }

                let parts: Vec<&str> = line.split('|').map(str::trim).collect();
                if parts.len() != 2 {
                    return Err(Error::invalid_state(
                        "Metadata corrupted: invalid format".to_string(),
                    ));
                }

                let branch = BranchId::new(parts[0]);
                let parent = if parts[1] != "none" {
                    Some(BranchId::new(parts[1]))
                } else {
                    None
                };

                parents.insert(branch.clone(), parent.clone());
                if let Some(parent_id) = &parent {
                    children
                        .entry(parent_id.clone())
                        .or_default()
                        .push(branch.clone());
                }

                Ok((parents, children))
            },
        )
    }
}
