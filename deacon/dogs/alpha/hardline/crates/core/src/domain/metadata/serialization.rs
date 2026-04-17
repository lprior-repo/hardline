//! Metadata serialization logic

use crate::Error;

use super::entities::StackMetadata;

impl StackMetadata {
    /// Serialize metadata to bytes
    fn serialize_metadata(&self) -> Vec<u8> {
        [
            "# StackMetadata - Branch parent relationships".to_string(),
            "# Format: branch|parent".to_string(),
        ]
        .into_iter()
        .chain(self.parents.iter().map(|(branch, parent)| {
            format!(
                "{}|{}",
                branch.as_str(),
                parent
                    .as_ref()
                    .map_or_else(|| "none".to_string(), |value| value.as_str().to_string())
            )
        }))
        .collect::<Vec<_>>()
        .join("\n")
        .into_bytes()
    }

    /// Save metadata to backend
    ///
    /// # Errors
    /// Returns an error if the backend fails to save.
    pub fn save(&self) -> Result<(), Error> {
        let data = self.serialize_metadata();
        self.backend
            .save(&data)
            .map_err(|e| Error::invalid_state(format!("Metadata backend error: {e}")))
    }

    /// Check if adding parent would create a cycle
    #[must_use]
    pub fn would_create_cycle(&self, branch: &super::BranchId, parent: &super::BranchId) -> bool {
        // Can't set parent to self
        if branch == parent {
            return true;
        }

        // Can't set trunk as child of anything
        if branch.as_str() == "trunk" {
            return true;
        }

        let (graph, indices) = self.build_graph();

        indices
            .get(branch)
            .copied()
            .zip(indices.get(parent).copied())
            .is_some_and(|(branch_idx, parent_idx)| {
                petgraph::algo::has_path_connecting(&graph, branch_idx, parent_idx, None)
            })
    }
}
