#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![forbid(unsafe_code)]

use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

use crate::domain::value_objects::BranchName;
use crate::{Result, StackError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrInfo {
    pub number: u32,
    pub url: String,
    pub title: String,
    pub state: PrState,
    pub is_draft: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

pub struct Draft;
pub struct Published;
pub struct Merging;
pub struct Merged;
pub struct Conflict;
pub struct Failed;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stack<S = Draft> {
    pub branches: Vec<StackBranch>,
    pub main_branch: BranchName,
    _state: PhantomData<S>,
}

impl Stack<Draft> {
    #[must_use]
    pub fn new(main_branch: BranchName) -> Self {
        Self {
            branches: Vec::new(),
            main_branch,
            _state: PhantomData,
        }
    }

    #[must_use]
    pub fn publish(self) -> Stack<Published> {
        self.transition_impl()
    }

    #[must_use]
    pub fn fail(self) -> Stack<Failed> {
        self.transition_impl()
    }
}

impl<S> Stack<S> {
    fn transition_impl<T>(self) -> Stack<T> {
        Stack {
            branches: self.branches,
            main_branch: self.main_branch,
            _state: PhantomData,
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

    #[must_use]
    pub fn topological_order(&self) -> Vec<&StackBranch> {
        let graph = self.build_dependency_graph();
        petgraph::algo::toposort(&graph, None).map_or_else(
            |_| self.branches.iter().collect(),
            |sorted_indices| {
                sorted_indices
                    .into_iter()
                    .filter_map(|idx| graph.node_weight(idx).copied())
                    .collect()
            },
        )
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

    #[must_use]
    pub fn ancestors(&self, branch: &BranchName) -> Vec<BranchName> {
        self.find_branch(branch)
            .and_then(|b| self.ancestors_from_branch(b))
            .unwrap_or_default()
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

    #[must_use]
    pub fn descendants(&self, branch: &BranchName) -> Vec<BranchName> {
        self.find_branch(branch)
            .map(|b| self.flatten_children(&b.children))
            .unwrap_or_default()
    }

    fn flatten_children(&self, branches: &[BranchName]) -> Vec<BranchName> {
        branches
            .iter()
            .filter_map(|name| self.find_branch(name))
            .flat_map(|b| {
                let children: Vec<BranchName> = b.children.clone();
                std::iter::once(b.name.clone()).chain(self.flatten_children(&children))
            })
            .collect()
    }

    #[must_use]
    pub fn current_stack(&self, branch: &BranchName) -> Vec<BranchName> {
        self.ancestors(branch)
            .into_iter()
            .rev()
            .chain(std::iter::once(branch.clone()))
            .chain(self.descendants(branch))
            .collect()
    }

    #[must_use]
    pub fn needs_restack(&self) -> Vec<BranchName> {
        self.branches
            .iter()
            .filter(|b| b.needs_restack)
            .map(|b| b.name.clone())
            .collect()
    }
}

impl Stack<Published> {
    #[must_use]
    pub fn start_merge(self) -> Stack<Merging> {
        self.transition_impl()
    }

    #[must_use]
    pub fn fail(self) -> Stack<Failed> {
        self.transition_impl()
    }
}

impl Stack<Merging> {
    #[must_use]
    pub fn complete_merge(self) -> Stack<Merged> {
        self.transition_impl()
    }

    #[must_use]
    pub fn mark_conflict(self) -> Stack<Conflict> {
        self.transition_impl()
    }

    #[must_use]
    pub fn fail(self) -> Stack<Failed> {
        self.transition_impl()
    }
}

impl Stack<Merged> {
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        true
    }
}

impl Stack<Conflict> {
    #[must_use]
    pub fn resolve(self) -> Stack<Published> {
        self.transition_impl()
    }

    #[must_use]
    pub fn fail(self) -> Stack<Failed> {
        self.transition_impl()
    }
}

impl Stack<Failed> {
    #[must_use]
    pub fn retry(self) -> Stack<Draft> {
        self.transition_impl()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_stack() -> Stack<Draft> {
        let main = BranchName::new("main".to_string());
        let mut stack = Stack::<Draft>::new(main.clone());

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

    #[test]
    fn test_stack_state_transitions() {
        let stack = create_test_stack();
        let published: Stack<Published> = stack.publish();
        let merging: Stack<Merging> = published.start_merge();
        let merged: Stack<Merged> = merging.complete_merge();
        assert!(merged.is_terminal());
    }

    #[test]
    fn test_draft_fail_transition() {
        let stack = create_test_stack();
        let failed: Stack<Failed> = stack.fail();
        let draft: Stack<Draft> = failed.retry();
        // Can retry and go back to draft
        let _published: Stack<Published> = draft.publish();
    }

    #[test]
    fn test_conflict_transition() {
        let stack = create_test_stack();
        let published: Stack<Published> = stack.publish();
        let merging: Stack<Merging> = published.start_merge();
        let conflict: Stack<Conflict> = merging.mark_conflict();
        let resolved: Stack<Published> = conflict.resolve();
        let _merging2: Stack<Merging> = resolved.start_merge();
    }

    #[test]
    fn test_published_fail_transition() {
        let stack = create_test_stack();
        let published: Stack<Published> = stack.publish();
        let failed: Stack<Failed> = published.fail();
        let _draft: Stack<Draft> = failed.retry();
    }

    #[test]
    fn test_merging_fail_transition() {
        let stack = create_test_stack();
        let published: Stack<Published> = stack.publish();
        let merging: Stack<Merging> = published.start_merge();
        let _failed: Stack<Failed> = merging.fail();
    }

    #[test]
    fn test_conflict_fail_transition() {
        let stack = create_test_stack();
        let published: Stack<Published> = stack.publish();
        let merging: Stack<Merging> = published.start_merge();
        let conflict: Stack<Conflict> = merging.mark_conflict();
        let _failed: Stack<Failed> = conflict.fail();
    }

    #[test]
    fn test_add_branch_validates_parent() {
        let main = BranchName::new("main".to_string());
        let mut stack = Stack::<Draft>::new(main.clone());

        // Adding a branch whose parent is not in the stack and not the main branch should fail
        let result = stack.add_branch(StackBranch {
            name: BranchName::new("orphan".to_string()),
            parent: Some(BranchName::new("nonexistent".to_string())),
            children: vec![],
            needs_restack: false,
            pr_info: None,
        });

        assert!(result.is_err());
        let err = result.err().expect("should be error");
        assert!(matches!(err, StackError::OrphanedBranch(_)));
    }

    #[test]
    fn test_add_branch_with_main_as_parent() {
        let main = BranchName::new("main".to_string());
        let mut stack = Stack::<Draft>::new(main.clone());

        let result = stack.add_branch(StackBranch {
            name: BranchName::new("feature-x".to_string()),
            parent: Some(main),
            children: vec![],
            needs_restack: false,
            pr_info: None,
        });

        assert!(result.is_ok());
        assert_eq!(stack.branches.len(), 1);
    }

    #[test]
    fn test_add_branch_no_parent() {
        let main = BranchName::new("main".to_string());
        let mut stack = Stack::<Draft>::new(main);

        let result = stack.add_branch(StackBranch {
            name: BranchName::new("root-branch".to_string()),
            parent: None,
            children: vec![],
            needs_restack: false,
            pr_info: None,
        });

        assert!(result.is_ok());
        assert_eq!(stack.branches.len(), 1);
    }

    #[test]
    fn test_add_branch_with_existing_parent() {
        let main = BranchName::new("main".to_string());
        let mut stack = Stack::<Draft>::new(main.clone());

        // Add first branch
        stack
            .add_branch(StackBranch {
                name: BranchName::new("feature-a".to_string()),
                parent: Some(main),
                children: vec![],
                needs_restack: false,
                pr_info: None,
            })
            .expect("should succeed");

        // Add second branch with first as parent
        let result = stack.add_branch(StackBranch {
            name: BranchName::new("feature-a-1".to_string()),
            parent: Some(BranchName::new("feature-a".to_string())),
            children: vec![],
            needs_restack: false,
            pr_info: None,
        });

        assert!(result.is_ok());
        assert_eq!(stack.branches.len(), 2);
    }

    #[test]
    fn test_topological_order_linear_chain() {
        let main = BranchName::new("main".to_string());
        let mut stack = Stack::<Draft>::new(main.clone());

        stack.branches.push(StackBranch {
            name: BranchName::new("base-feat".to_string()),
            parent: Some(main),
            children: vec![],
            needs_restack: false,
            pr_info: None,
        });

        stack.branches.push(StackBranch {
            name: BranchName::new("mid-feat".to_string()),
            parent: Some(BranchName::new("base-feat".to_string())),
            children: vec![],
            needs_restack: false,
            pr_info: None,
        });

        let order = stack.topological_order();
        // base-feat should come before mid-feat
        let base_idx = order.iter().position(|b| b.name.as_str() == "base-feat");
        let mid_idx = order.iter().position(|b| b.name.as_str() == "mid-feat");
        assert!(base_idx < mid_idx);
    }

    #[test]
    fn test_topological_order_empty_stack() {
        let main = BranchName::new("main".to_string());
        let stack = Stack::<Draft>::new(main);
        let order = stack.topological_order();
        assert!(order.is_empty());
    }

    #[test]
    fn test_topological_order_single_branch() {
        let main = BranchName::new("main".to_string());
        let mut stack = Stack::<Draft>::new(main);

        stack.branches.push(StackBranch {
            name: BranchName::new("solo".to_string()),
            parent: None,
            children: vec![],
            needs_restack: false,
            pr_info: None,
        });

        let order = stack.topological_order();
        assert_eq!(order.len(), 1);
        assert_eq!(order[0].name.as_str(), "solo");
    }

    #[test]
    fn test_topological_order_fallback_on_cycle() {
        let main = BranchName::new("main".to_string());
        let mut stack = Stack::<Draft>::new(main);

        // Create a cycle: a -> b -> c -> a (using parent pointers that form a cycle)
        // This should fall back to returning branches in their original order
        stack.branches.push(StackBranch {
            name: BranchName::new("cycle-a".to_string()),
            parent: Some(BranchName::new("cycle-c".to_string())),
            children: vec![],
            needs_restack: false,
            pr_info: None,
        });

        stack.branches.push(StackBranch {
            name: BranchName::new("cycle-b".to_string()),
            parent: Some(BranchName::new("cycle-a".to_string())),
            children: vec![],
            needs_restack: false,
            pr_info: None,
        });

        stack.branches.push(StackBranch {
            name: BranchName::new("cycle-c".to_string()),
            parent: Some(BranchName::new("cycle-b".to_string())),
            children: vec![],
            needs_restack: false,
            pr_info: None,
        });

        // Should not panic, falls back to original order
        let order = stack.topological_order();
        assert_eq!(order.len(), 3);
    }

    #[test]
    fn test_ancestors_from_root() {
        let stack = create_test_stack();
        // main has no ancestors (no parent)
        let ancestors = stack.ancestors(&BranchName::new("main".to_string()));
        assert!(ancestors.is_empty());
    }

    #[test]
    fn test_ancestors_nonexistent_branch() {
        let stack = create_test_stack();
        let ancestors = stack.ancestors(&BranchName::new("no-such-branch".to_string()));
        assert!(ancestors.is_empty());
    }

    #[test]
    fn test_descendants_from_leaf() {
        let stack = create_test_stack();
        // feature-a-2 has no children
        let descendants = stack.descendants(&BranchName::new("feature-a-2".to_string()));
        assert!(descendants.is_empty());
    }

    #[test]
    fn test_descendants_nonexistent_branch() {
        let stack = create_test_stack();
        let descendants = stack.descendants(&BranchName::new("nope".to_string()));
        assert!(descendants.is_empty());
    }

    #[test]
    fn test_current_stack_from_mid() {
        let stack = create_test_stack();
        let current = stack.current_stack(&BranchName::new("feature-a".to_string()));
        // feature-a: ancestors=[main], self=feature-a, descendants=[feature-a-1, feature-a-2]
        assert!(current.contains(&BranchName::new("main".to_string())));
        assert!(current.contains(&BranchName::new("feature-a".to_string())));
        assert!(current.contains(&BranchName::new("feature-a-1".to_string())));
        assert!(current.contains(&BranchName::new("feature-a-2".to_string())));
        assert_eq!(current.len(), 4);
    }

    #[test]
    fn test_current_stack_nonexistent_branch() {
        let stack = create_test_stack();
        let current = stack.current_stack(&BranchName::new("nope".to_string()));
        // current_stack always includes the branch itself via iter::once,
        // even for nonexistent branches (ancestors and descendants are empty)
        assert_eq!(current.len(), 1);
        assert_eq!(current[0].as_str(), "nope");
    }

    #[test]
    fn test_needs_restack_none_needed() {
        let main = BranchName::new("main".to_string());
        let mut stack = Stack::<Draft>::new(main);

        stack.branches.push(StackBranch {
            name: BranchName::new("clean-branch".to_string()),
            parent: None,
            children: vec![],
            needs_restack: false,
            pr_info: None,
        });

        assert!(stack.needs_restack().is_empty());
    }

    #[test]
    fn test_pr_info_variants() {
        let open = PrInfo {
            number: 1,
            url: "https://github.com/test/1".to_string(),
            title: "Test".to_string(),
            state: PrState::Open,
            is_draft: Some(false),
        };
        assert!(matches!(open.state, PrState::Open));

        let merged = PrInfo {
            number: 2,
            url: "https://github.com/test/2".to_string(),
            title: "Merged PR".to_string(),
            state: PrState::Merged,
            is_draft: None,
        };
        assert!(matches!(merged.state, PrState::Merged));

        let closed = PrInfo {
            number: 3,
            url: "https://github.com/test/3".to_string(),
            title: "Closed PR".to_string(),
            state: PrState::Closed,
            is_draft: Some(true),
        };
        assert!(matches!(closed.state, PrState::Closed));
    }

    #[test]
    fn test_stack_pr_info_serde_roundtrip() {
        let pr = PrInfo {
            number: 42,
            url: "https://github.com/org/repo/pull/42".to_string(),
            title: "Test PR".to_string(),
            state: PrState::Open,
            is_draft: Some(false),
        };
        let json = serde_json::to_string(&pr).expect("serialize");
        let deserialized: PrInfo = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(pr.number, deserialized.number);
        assert_eq!(pr.url, deserialized.url);
        assert_eq!(pr.title, deserialized.title);
        assert!(matches!(pr.state, PrState::Open));
        assert!(matches!(deserialized.state, PrState::Open));
        assert_eq!(pr.is_draft, deserialized.is_draft);
    }

    #[test]
    fn test_stack_pr_info_serde_draft_none() {
        let pr = PrInfo {
            number: 1,
            url: "url".to_string(),
            title: "t".to_string(),
            state: PrState::Open,
            is_draft: None,
        };
        let json = serde_json::to_string(&pr).expect("serialize");
        let deserialized: PrInfo = serde_json::from_str(&json).expect("deserialize");
        assert!(deserialized.is_draft.is_none());
    }

    #[test]
    fn test_stack_branch_serde_roundtrip() {
        let branch = StackBranch {
            name: BranchName::new("feat".to_string()),
            parent: Some(BranchName::new("main".to_string())),
            children: vec![BranchName::new("feat-child".to_string())],
            needs_restack: true,
            pr_info: Some(PrInfo {
                number: 10,
                url: "https://github.com/test/10".to_string(),
                title: "Feat".to_string(),
                state: PrState::Open,
                is_draft: Some(true),
            }),
        };
        let json = serde_json::to_string(&branch).expect("serialize");
        let deserialized: StackBranch = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(branch.name, deserialized.name);
        assert_eq!(branch.parent, deserialized.parent);
        assert_eq!(branch.children, deserialized.children);
        assert_eq!(branch.needs_restack, deserialized.needs_restack);
        assert!(deserialized.pr_info.is_some());
    }

    #[test]
    fn test_stack_branch_serde_no_pr() {
        let branch = StackBranch {
            name: BranchName::new("no-pr".to_string()),
            parent: None,
            children: vec![],
            needs_restack: false,
            pr_info: None,
        };
        let json = serde_json::to_string(&branch).expect("serialize");
        let deserialized: StackBranch = serde_json::from_str(&json).expect("deserialize");
        assert!(deserialized.pr_info.is_none());
        assert!(!deserialized.needs_restack);
    }

    #[test]
    fn test_stack_draft_serde_roundtrip() {
        let main = BranchName::new("main".to_string());
        let stack = Stack::<Draft>::new(main);
        let json = serde_json::to_string(&stack).expect("serialize");
        let deserialized: Stack<Draft> = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(stack.main_branch, deserialized.main_branch);
        assert_eq!(stack.branches.len(), deserialized.branches.len());
    }

    #[test]
    fn test_stack_empty_ancestors_descendants_current_stack() {
        let main = BranchName::new("main".to_string());
        let stack = Stack::<Draft>::new(main);
        // Empty stack has no branches, so lookups return empty
        assert!(stack
            .ancestors(&BranchName::new("anything".to_string()))
            .is_empty());
        assert!(stack
            .descendants(&BranchName::new("anything".to_string()))
            .is_empty());
    }

    #[test]
    fn test_stack_topological_order_diamond() {
        // Diamond: main -> a, main -> b, a -> c, b -> c
        let main = BranchName::new("main".to_string());
        let mut stack = Stack::<Draft>::new(main);

        stack.branches.push(StackBranch {
            name: BranchName::new("a".to_string()),
            parent: Some(BranchName::new("main".to_string())),
            children: vec![BranchName::new("c".to_string())],
            needs_restack: false,
            pr_info: None,
        });
        stack.branches.push(StackBranch {
            name: BranchName::new("b".to_string()),
            parent: Some(BranchName::new("main".to_string())),
            children: vec![BranchName::new("c".to_string())],
            needs_restack: false,
            pr_info: None,
        });
        stack.branches.push(StackBranch {
            name: BranchName::new("c".to_string()),
            parent: Some(BranchName::new("a".to_string())),
            children: vec![],
            needs_restack: false,
            pr_info: None,
        });

        let order = stack.topological_order();
        assert_eq!(order.len(), 3);
        // main is not in branches, but a and b should come before c
        let c_idx = order
            .iter()
            .position(|b| b.name.as_str() == "c")
            .expect("c in order");
        let a_idx = order
            .iter()
            .position(|b| b.name.as_str() == "a")
            .expect("a in order");
        let b_idx = order
            .iter()
            .position(|b| b.name.as_str() == "b")
            .expect("b in order");
        assert!(a_idx < c_idx);
        assert!(b_idx < c_idx);
    }

    #[test]
    fn test_stack_ancestors_deep_chain() {
        let main = BranchName::new("main".to_string());
        let mut stack = Stack::<Draft>::new(main.clone());

        stack.branches.push(StackBranch {
            name: BranchName::new("a".to_string()),
            parent: Some(main.clone()),
            children: vec![BranchName::new("b".to_string())],
            needs_restack: false,
            pr_info: None,
        });
        stack.branches.push(StackBranch {
            name: BranchName::new("b".to_string()),
            parent: Some(BranchName::new("a".to_string())),
            children: vec![BranchName::new("c".to_string())],
            needs_restack: false,
            pr_info: None,
        });
        stack.branches.push(StackBranch {
            name: BranchName::new("c".to_string()),
            parent: Some(BranchName::new("b".to_string())),
            children: vec![],
            needs_restack: false,
            pr_info: None,
        });

        let ancestors = stack.ancestors(&BranchName::new("c".to_string()));
        assert_eq!(ancestors.len(), 2); // b, a
        assert!(ancestors.contains(&BranchName::new("a".to_string())));
        assert!(ancestors.contains(&BranchName::new("b".to_string())));
    }

    #[test]
    fn test_stack_descendants_deep_tree() {
        let main = BranchName::new("main".to_string());
        let mut stack = Stack::<Draft>::new(main.clone());

        stack.branches.push(StackBranch {
            name: BranchName::new("a".to_string()),
            parent: Some(main),
            children: vec![
                BranchName::new("b".to_string()),
                BranchName::new("c".to_string()),
            ],
            needs_restack: false,
            pr_info: None,
        });
        stack.branches.push(StackBranch {
            name: BranchName::new("b".to_string()),
            parent: Some(BranchName::new("a".to_string())),
            children: vec![BranchName::new("d".to_string())],
            needs_restack: false,
            pr_info: None,
        });
        stack.branches.push(StackBranch {
            name: BranchName::new("c".to_string()),
            parent: Some(BranchName::new("a".to_string())),
            children: vec![],
            needs_restack: false,
            pr_info: None,
        });
        stack.branches.push(StackBranch {
            name: BranchName::new("d".to_string()),
            parent: Some(BranchName::new("b".to_string())),
            children: vec![],
            needs_restack: false,
            pr_info: None,
        });

        let descendants = stack.descendants(&BranchName::new("a".to_string()));
        assert_eq!(descendants.len(), 3); // b, c, d
        assert!(descendants.contains(&BranchName::new("b".to_string())));
        assert!(descendants.contains(&BranchName::new("c".to_string())));
        assert!(descendants.contains(&BranchName::new("d".to_string())));
    }

    #[test]
    fn test_stack_add_branch_orphan_with_existing_branch_parent() {
        let main = BranchName::new("main".to_string());
        let mut stack = Stack::<Draft>::new(main.clone());

        stack
            .add_branch(StackBranch {
                name: BranchName::new("parent-br".to_string()),
                parent: Some(main),
                children: vec![],
                needs_restack: false,
                pr_info: None,
            })
            .expect("should succeed");

        // parent-br is now in the stack, can be used as parent
        let result = stack.add_branch(StackBranch {
            name: BranchName::new("child-br".to_string()),
            parent: Some(BranchName::new("parent-br".to_string())),
            children: vec![],
            needs_restack: false,
            pr_info: None,
        });
        assert!(result.is_ok());
        assert_eq!(stack.branches.len(), 2);
    }

    #[test]
    fn test_stack_add_branch_empty_name() {
        let main = BranchName::new("main".to_string());
        let mut stack = Stack::<Draft>::new(main.clone());
        let result = stack.add_branch(StackBranch {
            name: BranchName::new("".to_string()),
            parent: Some(main),
            children: vec![],
            needs_restack: false,
            pr_info: None,
        });
        // Empty name is allowed since BranchName::new accepts anything
        assert!(result.is_ok());
    }

    #[test]
    fn test_stack_merged_state_transitions_exhaustive() {
        // Merged is terminal -- only has is_terminal()
        let stack = create_test_stack();
        let published: Stack<Published> = stack.publish();
        let merging: Stack<Merging> = published.start_merge();
        let merged: Stack<Merged> = merging.complete_merge();
        assert!(merged.is_terminal());
        // Verify the data is preserved through transitions
        assert_eq!(merged.branches.len(), 5);
    }

    #[test]
    fn test_stack_new_with_clone_main_branch() {
        let main = BranchName::new("main".to_string());
        let stack = Stack::<Draft>::new(main.clone());
        assert_eq!(stack.main_branch.as_str(), "main");
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::proptest;

    proptest! {
        #[test]
        fn prop_pr_info_serde_roundtrip(num in 1u32..1_000_000u32) {
            let pr = PrInfo {
                number: num,
                url: format!("https://github.com/test/{}", num),
                title: format!("PR #{}", num),
                state: PrState::Open,
                is_draft: Some(num % 2 == 0),
            };
            let json = serde_json::to_string(&pr).expect("serialize");
            let deserialized: PrInfo = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(pr.number, deserialized.number);
            assert_eq!(pr.url, deserialized.url);
            assert_eq!(pr.title, deserialized.title);
            assert!(matches!(pr.state, PrState::Open));
            assert!(matches!(deserialized.state, PrState::Open));
            assert_eq!(pr.is_draft, deserialized.is_draft);
        }

        #[test]
        fn prop_stack_branch_needs_restack_flag(flag in proptest::bool::ANY) {
            let main = BranchName::new("main".to_string());
            let branch = StackBranch {
                name: BranchName::new("test".to_string()),
                parent: Some(main),
                children: vec![],
                needs_restack: flag,
                pr_info: None,
            };
            assert_eq!(branch.needs_restack, flag);
        }

        #[test]
        fn prop_topological_order_preserves_all_branches(names in proptest::collection::vec(proptest::string::string_regex("[a-z]{1,10}").unwrap(), 1..20)) {
            let main = BranchName::new("main".to_string());
            let mut stack = Stack::<Draft>::new(main.clone());
            let mut parent: Option<BranchName> = Some(main);

            for name in &names {
                let branch = StackBranch {
                    name: BranchName::new(name.clone()),
                    parent: parent.clone(),
                    children: vec![],
                    needs_restack: false,
                    pr_info: None,
                };
                stack.add_branch(branch).expect("should succeed");
                parent = Some(BranchName::new(name.clone()));
            }

            let order = stack.topological_order();
            assert_eq!(order.len(), names.len());
            // Verify parent always comes before child in topo order
            for window in order.windows(2) {
                let parent_branch = &window[0];
                let child_branch = &window[1];
                assert_eq!(child_branch.parent.as_ref(), Some(&parent_branch.name));
            }
        }
    }
}
