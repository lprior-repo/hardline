#[test]
fn test_basic_task_id_validation() {
    assert!(TaskId::new("valid-task-001").is_ok());
    assert!(TaskId::new("invalid task").is_err());
}
