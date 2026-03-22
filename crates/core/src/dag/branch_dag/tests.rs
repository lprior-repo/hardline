// `BranchDag` tests

use crate::dag::BranchDag;
use crate::dag::BranchId;

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
