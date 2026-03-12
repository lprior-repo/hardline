use crate::domain::value_objects::BranchName;
use crate::error::StackError;
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

    pub fn add_branch(&mut self, branch: StackBranch) -> Result<(), StackError> {
        if let Some(parent) = &branch.parent {
            if !self.branches.iter().any(|b| &b.name == parent) && parent != &self.main_branch {
                return Err(StackError::OrphanedBranch(branch.name.to_string()));
            }
        }
        self.branches.push(branch);
        Ok(())
    }

    pub fn topological_order(&self) -> Vec<&StackBranch> {
        let mut graph: petgraph::Graph<&StackBranch, ()> = petgraph::Graph::new();
        let mut indices: std::collections::HashMap<&BranchName, _> =
            std::collections::HashMap::new();

        for branch in &self.branches {
            let idx = graph.add_node(branch);
            indices.insert(&branch.name, idx);
        }

        for branch in &self.branches {
            if let Some(parent) = &branch.parent {
                if let Some(&child_idx) = indices.get(&branch.name) {
                    if let Some(&parent_idx) = indices.get(parent) {
                        graph.add_edge(parent_idx, child_idx, ());
                    }
                }
            }
        }

        petgraph::algo::toposort(&graph, None)
            .unwrap_or_default()
            .iter()
            .map(|&idx| *graph.node_weight(idx).unwrap())
            .collect()
    }

    pub fn ancestors(&self, branch: &BranchName) -> Vec<BranchName> {
        let mut result = Vec::new();
        let mut current = branch.clone();

        while let Some(b) = self.branches.iter().find(|b| &b.name == &current) {
            if let Some(parent) = &b.parent {
                result.push(parent.clone());
                current = parent.clone();
            } else {
                break;
            }
        }

        result
    }

    pub fn descendants(&self, branch: &BranchName) -> Vec<BranchName> {
        let mut result = Vec::new();
        let mut to_visit = vec![branch.clone()];

        while let Some(current) = to_visit.pop() {
            if let Some(b) = self.branches.iter().find(|b| &b.name == &current) {
                for child in &b.children {
                    result.push(child.clone());
                    to_visit.push(child.clone());
                }
            }
        }

        result
    }

    pub fn current_stack(&self, branch: &BranchName) -> Vec<BranchName> {
        let mut ancestors = self.ancestors(branch);
        ancestors.reverse();
        let mut result = ancestors;
        result.push(branch.clone());
        result.extend(self.descendants(branch));
        result
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
