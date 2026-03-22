//! `BranchDag` tests




#[test]
fn test_new_returns_dag_with_trunk_branch() {
    let dag = BranchDag::new();
    assert!(dag.contains(&BranchId::new("trunk")));
    assert!(dag.is_trunk(&BranchId::new("trunk")));
    assert_eq!(dag.len(), 1);
}

#[test]
fn test_add_branch_creates_branch_with_parents() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")])
        .expect("Should add branch");
    assert!(dag.contains(&BranchId::new("feature")));
    assert_eq!(dag.len(), 2);
}

#[test]
fn test_remove_branch_removes_branch_successfully() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")])
        .expect("Should add feature");
    dag.remove_branch(BranchId::new("feature"))
        .expect("Should remove");
    assert!(!dag.contains(&BranchId::new("feature")));
    assert_eq!(dag.len(), 1);
}

#[test]
fn test_ancestors_returns_all_upstream_branches() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")])
        .expect("Should add feature");
    dag.add_branch(BranchId::new("hotfix"), vec![BranchId::new("feature")])
        .expect("Should add hotfix");

    let ancestors = dag.ancestors(&BranchId::new("hotfix")).expect("Should get ancestors");

    assert_eq!(ancestors, vec![BranchId::new("feature"), BranchId::new("trunk")]);
}

#[test]
fn test_descendants_returns_all_downstream_branches() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")])
        .expect("Should add feature");
    dag.add_branch(BranchId::new("hotfix"), vec![BranchId::new("feature")])
        .expect("Should add hotfix");

    let descendants = dag
        .descendants(&BranchId::new("trunk"))
        .expect("Should get descendants");

    assert_eq!(descendants, vec![BranchId::new("feature"), BranchId::new("hotfix")]);
}

#[test]
fn test_path_to_root_returns_chain_to_trunk() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")])
        .expect("Should add feature");
    dag.add_branch(BranchId::new("hotfix"), vec![BranchId::new("feature")])
        .expect("Should add hotfix");

    let path = dag.path_to_root(&BranchId::new("hotfix")).expect("Should get path");

    assert_eq!(
        path,
        vec![
            BranchId::new("hotfix"),
            BranchId::new("feature"),
            BranchId::new("trunk")
        ]
    );
}

#[test]
fn test_path_to_root_of_trunk_returns_empty() {
    let dag = BranchDag::new();
    let path = dag.path_to_root(&BranchId::new("trunk")).expect("Should get path");
    assert!(path.is_empty());
}

#[test]
fn test_topological_sort_returns_dependency_order() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")])
        .expect("Should add feature");
    dag.add_branch(BranchId::new("hotfix"), vec![BranchId::new("feature")])
        .expect("Should add hotfix");

    let sorted = dag.topological_sort().expect("Should sort");

    let trunk_idx = sorted.iter().position(|b| b == &BranchId::new("trunk")).unwrap();
    let feature_idx = sorted.iter().position(|b| b == &BranchId::new("feature")).unwrap();
    let hotfix_idx = sorted.iter().position(|b| b == &BranchId::new("hotfix")).unwrap();

    assert!(trunk_idx < feature_idx);
    assert!(feature_idx < hotfix_idx);
}

#[test]
fn test_contains_returns_true_for_existing_branch() {
    let dag = BranchDag::new();
    assert!(dag.contains(&BranchId::new("trunk")));
}

#[test]
fn test_is_trunk_returns_true_for_root_branch() {
    let dag = BranchDag::new();
    assert!(dag.is_trunk(&BranchId::new("trunk")));
}

#[test]
fn test_add_branch_returns_error_when_branch_already_exists() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("main"), vec![BranchId::new("trunk")])
        .expect("Should add main");

    let result = dag.add_branch(BranchId::new("main"), vec![BranchId::new("trunk")]);
    assert!(matches!(result, Err(DagError::BranchAlreadyExists(_))));
}

#[test]
fn test_add_branch_returns_error_when_parent_not_found() {
    let mut dag = BranchDag::new();
    let result = dag.add_branch(BranchId::new("feature"), vec![BranchId::new("nonexistent")]);
    assert!(matches!(result, Err(DagError::InvalidParent(_))));
}

#[test]
fn test_add_branch_returns_error_when_cycle_detected() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("a"), vec![BranchId::new("trunk")])
        .expect("Should add a");
    dag.add_branch(BranchId::new("b"), vec![BranchId::new("a")]).expect("Should add b");

    let result = dag.add_branch(BranchId::new("a"), vec![BranchId::new("b")]);
    assert!(matches!(result, Err(DagError::CycleDetected(_))));
}

#[test]
fn test_add_branch_returns_error_when_self_reference() {
    let mut dag = BranchDag::new();
    let result = dag.add_branch(BranchId::new("a"), vec![BranchId::new("a")]);
    assert!(matches!(result, Err(DagError::CycleDetected(_))));
}

#[test]
fn test_add_branch_returns_error_when_indirect_cycle() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("a"), vec![BranchId::new("trunk")])
        .expect("Should add a");
    dag.add_branch(BranchId::new("b"), vec![BranchId::new("a")]).expect("Should add b");
    dag.add_branch(BranchId::new("c"), vec![BranchId::new("b")]).expect("Should add c");

    let result = dag.add_branch(BranchId::new("a"), vec![BranchId::new("c")]);
    assert!(matches!(result, Err(DagError::CycleDetected(_))));
}

#[test]
fn test_remove_branch_returns_error_when_branch_not_found() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("main"), vec![BranchId::new("trunk")])
        .expect("Should add main");

    let result = dag.remove_branch(BranchId::new("nonexistent"));
    assert!(matches!(result, Err(DagError::BranchNotFound(_))));
}

#[test]
fn test_remove_branch_returns_error_when_has_descendants() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("main"), vec![BranchId::new("trunk")])
        .expect("Should add main");
    dag.add_branch(BranchId::new("feature"), vec![BranchId::new("main")])
        .expect("Should add feature");

    let result = dag.remove_branch(BranchId::new("main"));
    assert!(matches!(result, Err(DagError::HasDescendants(_, _))));
}

#[test]
fn test_ancestors_returns_error_when_branch_not_found() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("main"), vec![BranchId::new("trunk")])
        .expect("Should add main");

    let result = dag.ancestors(&BranchId::new("nonexistent"));
    assert!(matches!(result, Err(DagError::BranchNotFound(_))));
}

#[test]
fn test_descendants_returns_error_when_branch_not_found() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("main"), vec![BranchId::new("trunk")])
        .expect("Should add main");

    let result = dag.descendants(&BranchId::new("nonexistent"));
    assert!(matches!(result, Err(DagError::BranchNotFound(_))));
}

#[test]
fn test_ancestors_of_trunk_returns_empty() {
    let dag = BranchDag::new();
    let ancestors = dag.ancestors(&BranchId::new("trunk")).expect("Should get ancestors");
    assert!(ancestors.is_empty());
}

#[test]
fn test_descendants_of_leaf_returns_empty() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")])
        .expect("Should add feature");

    let descendants = dag.descendants(&BranchId::new("feature")).expect("Should get descendants");
    assert!(descendants.is_empty());
}

#[test]
fn test_multiple_branches_same_parent() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("feature-a"), vec![BranchId::new("trunk")])
        .expect("Should add feature-a");
    dag.add_branch(BranchId::new("feature-b"), vec![BranchId::new("trunk")])
        .expect("Should add feature-b");

    assert_eq!(dag.len(), 3);
}

#[test]
fn test_branch_with_multiple_parents() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")])
        .expect("Should add feature");
    dag.add_branch(BranchId::new("hotfix"), vec![BranchId::new("trunk")])
        .expect("Should add hotfix");
    dag.add_branch(
        BranchId::new("release"),
        vec![BranchId::new("feature"), BranchId::new("hotfix")],
    )
    .expect("Should add release");

    let parents = dag
        .parents
        .get(&BranchId::new("release"))
        .cloned()
        .unwrap_or_default();
    assert_eq!(parents.len(), 2);
}

#[test]
fn test_topological_sort_empty_dag() {
    let dag = BranchDag::new();
    let result = dag.topological_sort();
    let sorted = result.expect("Should sort");
    assert_eq!(sorted, vec![BranchId::new("trunk")]);
}

#[test]
fn test_complex_dag_with_multiple_levels() {
    let mut dag = BranchDag::new();
    dag.add_branch(BranchId::new("main"), vec![BranchId::new("trunk")])
        .expect("Should add main");
    dag.add_branch(BranchId::new("feature-a"), vec![BranchId::new("main")])
        .expect("Should add feature-a");
    dag.add_branch(BranchId::new("feature-b"), vec![BranchId::new("main")])
        .expect("Should add feature-b");
    dag.add_branch(
        BranchId::new("release"),
        vec![BranchId::new("feature-a"), BranchId::new("feature-b")],
    )
    .expect("Should add release");
    dag.add_branch(BranchId::new("hotfix"), vec![BranchId::new("release")])
        .expect("Should add hotfix");

    let ancestors = dag.ancestors(&BranchId::new("hotfix")).expect("Should get ancestors");
    assert!(ancestors.contains(&BranchId::new("release")));
    assert!(ancestors.contains(&BranchId::new("feature-a")));
    assert!(ancestors.contains(&BranchId::new("feature-b")));
    assert!(ancestors.contains(&BranchId::new("main")));
    assert!(ancestors.contains(&BranchId::new("trunk")));

    let descendants = dag.descendants(&BranchId::new("main")).expect("Should get descendants");
    assert!(descendants.contains(&BranchId::new("feature-a")));
    assert!(descendants.contains(&BranchId::new("feature-b")));
    assert!(descendants.contains(&BranchId::new("release")));
    assert!(descendants.contains(&BranchId::new("hotfix")));
}
