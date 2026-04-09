//! Restack operations: scope calculation and priority-based rebasing.
//!
//! Ported from stax `commands/restack.rs`. Provides pure calculation functions
//! for determining which branches need restacking and in what order.
//!
//! # Data -> Calc -> Actions
//!
//! - **Data**: `RestackScope`, `RestackStep`, `RestackPlan`
//! - **Calc**: `build_restack_plan`, `scope_branches`, `calculate_depth`, `infer_scope`
//! - **Actions**: Transactional execution lives in `transactional_engine.rs`

use std::collections::HashSet;

use crate::engine::transactional_engine::StackGraph;
use crate::error::{Result, StackError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestackScope {
    CurrentBranch,
    Upstack,
    FullStack,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestackStep {
    pub branch: String,
    pub parent: String,
    pub priority: u32,
    pub needs_restack: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestackPlan {
    pub steps: Vec<RestackStep>,
    pub scope: RestackScope,
    pub target_branch: String,
}

impl RestackPlan {
    pub fn filter_needs_restack(&self) -> Vec<&RestackStep> {
        self.steps.iter().filter(|s| s.needs_restack).collect()
    }

    pub fn has_work(&self) -> bool {
        self.steps.iter().any(|s| s.needs_restack)
    }

    pub fn needs_restack_count(&self) -> usize {
        self.steps.iter().filter(|s| s.needs_restack).count()
    }

    pub fn branch_names(&self) -> Vec<&str> {
        self.steps.iter().map(|s| s.branch.as_str()).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

pub fn calculate_depth(graph: &StackGraph, branch: &str) -> u32 {
    let mut depth = 0u32;
    let mut current = branch.to_string();
    let mut visited = HashSet::new();

    while let Some(node) = graph.branches.get(&current) {
        if !visited.insert(current.clone()) {
            break;
        }
        match &node.parent {
            Some(parent) => {
                depth = depth.saturating_add(1);
                current = parent.clone();
            }
            None => break,
        }
    }

    depth
}

pub fn scope_branches(graph: &StackGraph, branch: &str, scope: &RestackScope) -> Vec<String> {
    match scope {
        RestackScope::CurrentBranch => {
            if graph.branches.contains_key(branch) {
                vec![branch.to_string()]
            } else {
                Vec::new()
            }
        }
        RestackScope::Upstack => {
            let mut result = vec![branch.to_string()];
            result.extend(graph.descendants(branch));
            result
        }
        RestackScope::FullStack => graph.current_stack(branch),
    }
}

pub fn build_restack_plan(
    graph: &StackGraph,
    branch: &str,
    scope: RestackScope,
) -> Result<RestackPlan> {
    let branches = scope_branches(graph, branch, &scope);

    if branches.is_empty() {
        return Err(StackError::BranchNotFound(branch.to_string()));
    }

    let trunk = &graph.trunk;

    let mut steps: Vec<RestackStep> = branches
        .iter()
        .filter(|name| *name != trunk)
        .filter_map(|name| {
            let node = graph.branches.get(name)?;
            let parent = node.parent.as_ref()?;
            let depth = calculate_depth(graph, name);
            Some(RestackStep {
                branch: name.clone(),
                parent: parent.clone(),
                priority: depth,
                needs_restack: node.needs_restack,
            })
        })
        .collect();

    steps.sort_by_key(|a| a.priority);

    Ok(RestackPlan {
        steps,
        scope,
        target_branch: branch.to_string(),
    })
}

pub fn infer_scope(graph: &StackGraph, branch: &str) -> RestackScope {
    if branch == graph.trunk {
        return RestackScope::FullStack;
    }

    let descendants = graph.descendants(branch);
    let has_dirty_descendants = descendants
        .iter()
        .any(|d| graph.branches.get(d).is_some_and(|n| n.needs_restack));

    if has_dirty_descendants {
        RestackScope::Upstack
    } else {
        RestackScope::CurrentBranch
    }
}

pub fn build_all_restack_plans(graph: &StackGraph) -> Vec<RestackPlan> {
    let trunk_children = graph
        .branches
        .get(&graph.trunk)
        .map(|n| n.children.clone())
        .unwrap_or_default();

    trunk_children
        .iter()
        .filter_map(|child| {
            let plan = build_restack_plan(graph, child, RestackScope::Upstack).ok()?;
            plan.has_work().then_some(plan)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::metadata::BranchMetadata;
    use crate::engine::transactional_engine::MetadataStore;
    use std::collections::HashMap;

    struct MockStore {
        metadata: HashMap<String, BranchMetadata>,
        trunk: Option<String>,
        revisions: HashMap<String, String>,
    }

    impl MockStore {
        fn new() -> Self {
            Self {
                metadata: HashMap::new(),
                trunk: None,
                revisions: HashMap::new(),
            }
        }

        fn with_trunk(mut self, trunk: &str) -> Self {
            self.trunk = Some(trunk.to_string());
            self.revisions
                .insert(trunk.to_string(), "trunk-rev".to_string());
            self
        }

        fn add_branch(
            mut self,
            name: &str,
            parent: &str,
            parent_rev: &str,
            current_rev: &str,
        ) -> Self {
            self.metadata
                .insert(name.to_string(), BranchMetadata::new(parent, parent_rev));
            self.revisions
                .insert(name.to_string(), current_rev.to_string());
            self
        }
    }

    impl MetadataStore for MockStore {
        fn read(&self, branch: &str) -> Result<Option<BranchMetadata>> {
            Ok(self.metadata.get(branch).cloned())
        }
        fn write(&self, _branch: &str, _metadata: &BranchMetadata) -> Result<()> {
            Ok(())
        }
        fn delete(&self, _branch: &str) -> Result<()> {
            Ok(())
        }
        fn list_branches(&self) -> Result<Vec<String>> {
            Ok(self.metadata.keys().cloned().collect())
        }
        fn read_trunk(&self) -> Result<Option<String>> {
            Ok(self.trunk.clone())
        }
        fn branch_revision(&self, branch: &str) -> Result<Option<String>> {
            Ok(self.revisions.get(branch).cloned())
        }
    }

    fn create_linear_store() -> MockStore {
        MockStore::new()
            .with_trunk("main")
            .add_branch("feature-a", "main", "trunk-rev", "rev-a")
            .add_branch("feature-a-1", "feature-a", "rev-a", "rev-a1")
            .add_branch("feature-a-2", "feature-a-1", "rev-a1", "rev-a2")
    }

    fn create_dirty_store() -> MockStore {
        MockStore::new()
            .with_trunk("main")
            .add_branch("feature-a", "main", "trunk-rev", "rev-a")
            .add_branch("feature-b", "main", "trunk-rev-old", "rev-b")
            .add_branch("feature-b-1", "feature-b", "rev-b-old", "rev-b1")
    }

    fn load_graph(store: &MockStore) -> StackGraph {
        StackGraph::load(store).expect("load")
    }

    #[test]
    fn test_calculate_depth_trunk() {
        let store = create_linear_store();
        let graph = load_graph(&store);
        assert_eq!(calculate_depth(&graph, "main"), 0);
    }

    #[test]
    fn test_calculate_depth_first_level() {
        let store = create_linear_store();
        let graph = load_graph(&store);
        assert_eq!(calculate_depth(&graph, "feature-a"), 1);
    }

    #[test]
    fn test_calculate_depth_second_level() {
        let store = create_linear_store();
        let graph = load_graph(&store);
        assert_eq!(calculate_depth(&graph, "feature-a-1"), 2);
    }

    #[test]
    fn test_calculate_depth_third_level() {
        let store = create_linear_store();
        let graph = load_graph(&store);
        assert_eq!(calculate_depth(&graph, "feature-a-2"), 3);
    }

    #[test]
    fn test_calculate_depth_nonexistent() {
        let store = create_linear_store();
        let graph = load_graph(&store);
        assert_eq!(calculate_depth(&graph, "nonexistent"), 0);
    }

    #[test]
    fn test_scope_branches_current() {
        let store = create_linear_store();
        let graph = load_graph(&store);
        let branches = scope_branches(&graph, "feature-a", &RestackScope::CurrentBranch);
        assert_eq!(branches, vec!["feature-a"]);
    }

    #[test]
    fn test_scope_branches_current_nonexistent() {
        let store = create_linear_store();
        let graph = load_graph(&store);
        let branches = scope_branches(&graph, "ghost", &RestackScope::CurrentBranch);
        assert!(branches.is_empty());
    }

    #[test]
    fn test_scope_branches_upstack() {
        let store = create_linear_store();
        let graph = load_graph(&store);
        let branches = scope_branches(&graph, "feature-a", &RestackScope::Upstack);
        assert_eq!(branches, vec!["feature-a", "feature-a-1", "feature-a-2"]);
    }

    #[test]
    fn test_scope_branches_upstack_leaf() {
        let store = create_linear_store();
        let graph = load_graph(&store);
        let branches = scope_branches(&graph, "feature-a-2", &RestackScope::Upstack);
        assert_eq!(branches, vec!["feature-a-2"]);
    }

    #[test]
    fn test_scope_branches_full_stack() {
        let store = create_linear_store();
        let graph = load_graph(&store);
        let branches = scope_branches(&graph, "feature-a-1", &RestackScope::FullStack);
        assert_eq!(
            branches,
            vec!["main", "feature-a", "feature-a-1", "feature-a-2"]
        );
    }

    #[test]
    fn test_build_plan_current_branch() {
        let store = create_linear_store();
        let graph = load_graph(&store);
        let plan =
            build_restack_plan(&graph, "feature-a", RestackScope::CurrentBranch).expect("plan");

        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].branch, "feature-a");
        assert_eq!(plan.steps[0].parent, "main");
        assert_eq!(plan.steps[0].priority, 1);
    }

    #[test]
    fn test_build_plan_upstack_orders_by_priority() {
        let store = create_linear_store();
        let graph = load_graph(&store);
        let plan = build_restack_plan(&graph, "feature-a", RestackScope::Upstack).expect("plan");

        assert_eq!(plan.steps.len(), 3);
        assert_eq!(plan.steps[0].branch, "feature-a");
        assert_eq!(plan.steps[0].priority, 1);
        assert_eq!(plan.steps[1].branch, "feature-a-1");
        assert_eq!(plan.steps[1].priority, 2);
        assert_eq!(plan.steps[2].branch, "feature-a-2");
        assert_eq!(plan.steps[2].priority, 3);
    }

    #[test]
    fn test_build_plan_full_stack() {
        let store = create_linear_store();
        let graph = load_graph(&store);
        let plan =
            build_restack_plan(&graph, "feature-a-2", RestackScope::FullStack).expect("plan");

        assert_eq!(plan.steps.len(), 3);
        assert_eq!(
            plan.branch_names(),
            vec!["feature-a", "feature-a-1", "feature-a-2"]
        );
    }

    #[test]
    fn test_build_plan_excludes_trunk() {
        let store = create_linear_store();
        let graph = load_graph(&store);
        let plan =
            build_restack_plan(&graph, "feature-a-2", RestackScope::FullStack).expect("plan");

        assert!(!plan.branch_names().contains(&"main"));
    }

    #[test]
    fn test_build_plan_empty_for_nonexistent() {
        let store = create_linear_store();
        let graph = load_graph(&store);
        let result = build_restack_plan(&graph, "ghost", RestackScope::CurrentBranch);
        assert!(result.is_err());
    }

    #[test]
    fn test_plan_needs_restack_detection() {
        let store = create_dirty_store();
        let graph = load_graph(&store);
        let plan = build_restack_plan(&graph, "feature-b", RestackScope::Upstack).expect("plan");

        assert!(plan.has_work());
        assert_eq!(plan.needs_restack_count(), 2);
        let dirty: Vec<&str> = plan
            .filter_needs_restack()
            .iter()
            .map(|s| s.branch.as_str())
            .collect();
        assert!(dirty.contains(&"feature-b"));
        assert!(dirty.contains(&"feature-b-1"));
    }

    #[test]
    fn test_plan_clean_stack_has_no_work() {
        let store = create_linear_store();
        let graph = load_graph(&store);
        let plan = build_restack_plan(&graph, "feature-a", RestackScope::Upstack).expect("plan");

        assert!(!plan.has_work());
        assert_eq!(plan.needs_restack_count(), 0);
    }

    #[test]
    fn test_infer_scope_trunk_gives_full_stack() {
        let store = create_linear_store();
        let graph = load_graph(&store);
        assert_eq!(infer_scope(&graph, "main"), RestackScope::FullStack);
    }

    #[test]
    fn test_infer_scope_clean_branch_gives_current() {
        let store = create_linear_store();
        let graph = load_graph(&store);
        assert_eq!(
            infer_scope(&graph, "feature-a"),
            RestackScope::CurrentBranch
        );
    }

    #[test]
    fn test_infer_scope_dirty_descendants_gives_upstack() {
        let store = create_dirty_store();
        let graph = load_graph(&store);
        assert_eq!(infer_scope(&graph, "feature-b"), RestackScope::Upstack);
    }

    #[test]
    fn test_build_all_restack_plans() {
        let store = create_dirty_store();
        let graph = load_graph(&store);
        let plans = build_all_restack_plans(&graph);

        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].target_branch, "feature-b");
    }

    #[test]
    fn test_build_all_restack_plans_clean() {
        let store = create_linear_store();
        let graph = load_graph(&store);
        let plans = build_all_restack_plans(&graph);
        assert!(plans.is_empty());
    }

    #[test]
    fn test_plan_is_empty() {
        let plan = RestackPlan {
            steps: Vec::new(),
            scope: RestackScope::CurrentBranch,
            target_branch: "test".to_string(),
        };
        assert!(plan.is_empty());
    }

    #[test]
    fn test_plan_is_not_empty() {
        let plan = RestackPlan {
            steps: vec![RestackStep {
                branch: "a".to_string(),
                parent: "main".to_string(),
                priority: 1,
                needs_restack: false,
            }],
            scope: RestackScope::CurrentBranch,
            target_branch: "a".to_string(),
        };
        assert!(!plan.is_empty());
    }

    #[test]
    fn test_scope_branches_full_from_trunk() {
        let store = create_linear_store();
        let graph = load_graph(&store);
        let branches = scope_branches(&graph, "main", &RestackScope::FullStack);
        assert!(branches.contains(&"main".to_string()));
    }
}
