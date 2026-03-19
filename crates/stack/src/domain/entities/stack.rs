use crate::domain::value_objects::BranchName;
use crate::{Result, StackError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrInfo {
    pub number: u32,
    pub url: String,
    pub title: String,
    pub state: PrState,
    pub is_draft: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrState {
    Open,
    Merged,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackBranch {
    pub name: BranchName,
    pub parent: Option<BranchName>,
    pub children: Vec<BranchName>,
    pub needs_restack: bool,
    pub pr_info: Option<PrInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stack {
    pub branches: Vec<StackBranch>,
    pub main_branch: BranchName,
}

impl Stack {
    pub fn new(main_branch: BranchName) -> Self {
        Self {
            branches: Vec::new(),
            main_branch,
        }
    }

    pub fn add_branch(&mut self, branch: StackBranch) -> Result<()> {
        if let Some(parent) = &branch.parent {
            if !self.branches.iter().any(|b| &b.name == parent) && parent != &self.main_branch {
                return Err(StackError::OrphanedBranch(branch.name.to_string()));
            }
        }
        self.branches.push(branch);
        Ok(())
    }

    pub fn topological_order(&self) -> Vec<&StackBranch> {
        let graph = self.build_dependency_graph();
        petgraph::algo::toposort(&graph, None)
            .map(|sorted_indices| {
                sorted_indices
                    .into_iter()
                    .filter_map(|idx| graph.node_weight(idx).map(|b| *b))
                    .collect()
            })
            .unwrap_or_else(|_| self.branches.iter().collect())
    }

    fn build_dependency_graph(&self) -> petgraph::Graph<&StackBranch, ()> {
        let mut graph: petgraph::Graph<&StackBranch, ()> = petgraph::Graph::new();
        let indices: std::collections::HashMap<&BranchName, _> = self
            .branches
            .iter()
            .map(|branch| (&branch.name, graph.add_node(branch)))
            .collect();

        for branch in &self.branches {
            if let Some(parent) = &branch.parent {
                if let (Some(&child_idx), Some(&parent_idx)) =
                    (indices.get(&branch.name), indices.get(parent))
                {
                    graph.add_edge(parent_idx, child_idx, ());
                }
            }
        }
        graph
    }

    pub fn ancestors(&self, branch: &BranchName) -> Vec<BranchName> {
        self.find_branch(branch)
            .and_then(|b| self.ancestors_from_branch(b))
            .unwrap_or_else(Vec::new)
    }

    fn find_branch(&self, name: &BranchName) -> Option<&StackBranch> {
        self.branches.iter().find(|b| &b.name == name)
    }

    fn ancestors_from_branch(&self, branch: &StackBranch) -> Option<Vec<BranchName>> {
        branch.parent.as_ref().map(|parent| {
            std::iter::successors(Some(parent.clone()), |p| {
                self.find_branch(p).and_then(|b| b.parent.clone())
            })
            .take_while(|p| self.find_branch(p).is_some())
            .collect()
        })
    }

    pub fn descendants(&self, branch: &BranchName) -> Vec<BranchName> {
        self.find_branch(branch)
            .map(|b| self.flatten_children(&b.children))
            .unwrap_or_else(Vec::new)
    }

    fn flatten_children(&self, branches: &[BranchName]) -> Vec<BranchName> {
        branches
            .iter()
            .filter_map(|name| self.find_branch(name))
            .flat_map(|b| {
                let children: Vec<BranchName> = b.children.iter().cloned().collect();
                std::iter::once(b.name.clone()).chain(self.flatten_children(&children))
            })
            .collect()
    }

    pub fn current_stack(&self, branch: &BranchName) -> Vec<BranchName> {
        self.ancestors(branch)
            .into_iter()
            .rev()
            .chain(std::iter::once(branch.clone()))
            .chain(self.descendants(branch))
            .collect()
    }

    pub fn needs_restack(&self) -> Vec<BranchName> {
        self.branches
            .iter()
            .filter(|b| b.needs_restack)
            .map(|b| b.name.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_stack() -> Stack {
        let main = BranchName::new("main".to_string());
        let mut stack = Stack::new(main.clone());

        stack.branches.push(StackBranch {
            name: main.clone(),
            parent: None,
            children: vec![
                BranchName::new("feature-a".to_string()),
                BranchName::new("feature-b".to_string()),
            ],
            needs_restack: false,
            pr_info: None,
        });

        stack.branches.push(StackBranch {
            name: BranchName::new("feature-a".to_string()),
            parent: Some(main.clone()),
            children: vec![BranchName::new("feature-a-1".to_string())],
            needs_restack: false,
            pr_info: Some(PrInfo {
                number: 1,
                url: "https://github.com/test/1".to_string(),
                title: "Feature A".to_string(),
                state: PrState::Open,
                is_draft: Some(false),
            }),
        });

        stack.branches.push(StackBranch {
            name: BranchName::new("feature-a-1".to_string()),
            parent: Some(BranchName::new("feature-a".to_string())),
            children: vec![BranchName::new("feature-a-2".to_string())],
            needs_restack: true,
            pr_info: Some(PrInfo {
                number: 2,
                url: "https://github.com/test/2".to_string(),
                title: "Feature A-1".to_string(),
                state: PrState::Open,
                is_draft: Some(true),
            }),
        });

        stack.branches.push(StackBranch {
            name: BranchName::new("feature-a-2".to_string()),
            parent: Some(BranchName::new("feature-a-1".to_string())),
            children: vec![],
            needs_restack: false,
            pr_info: None,
        });

        stack.branches.push(StackBranch {
            name: BranchName::new("feature-b".to_string()),
            parent: Some(main),
            children: vec![],
            needs_restack: true,
            pr_info: Some(PrInfo {
                number: 3,
                url: "https://github.com/test/3".to_string(),
                title: "Feature B".to_string(),
                state: PrState::Merged,
                is_draft: None,
            }),
        });

        stack
    }

    #[test]
    fn test_ancestors_from_leaf() {
        let stack = create_test_stack();
        let ancestors = stack.ancestors(&BranchName::new("feature-a-2".to_string()));
        assert_eq!(ancestors.len(), 3);
    }

    #[test]
    fn test_descendants_from_trunk() {
        let stack = create_test_stack();
        let mut descendants = stack.descendants(&BranchName::new("main".to_string()));
        descendants.sort();
        assert_eq!(descendants.len(), 4);
    }

    #[test]
    fn test_current_stack_from_leaf() {
        let stack = create_test_stack();
        let current = stack.current_stack(&BranchName::new("feature-a-2".to_string()));
        assert_eq!(current.len(), 4);
    }

    #[test]
    fn test_needs_restack() {
        let stack = create_test_stack();
        let mut needs = stack.needs_restack();
        needs.sort();
        assert_eq!(needs.len(), 2);
    }
}
