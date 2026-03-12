use workspace::application::workspace_service::WorkspaceService;
use workspace::domain::value_objects::{WorkspaceName, WorkspacePath};

#[test]
fn test_rapid_create_delete() {
    for i in 0..1000 {
        let name = WorkspaceName::new(format!("test-ws-{}", i)).unwrap();
        let path = WorkspacePath::new(format!("/tmp/test-ws-{}", i)).unwrap();

        // Create workspace
        let mut workspace = WorkspaceService::create_workspace(name.clone(), path.clone()).unwrap();

        // Initialize
        workspace = WorkspaceService::initialize_workspace(&workspace).unwrap();

        // Lock
        workspace = WorkspaceService::lock_workspace(&workspace, "test-agent".to_string()).unwrap();

        // Unlock
        workspace = WorkspaceService::unlock_workspace(&workspace).unwrap();

        // Delete
        let _deleted = WorkspaceService::delete_workspace(&workspace).unwrap();
    }
}
