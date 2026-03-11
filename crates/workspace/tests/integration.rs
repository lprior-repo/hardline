use scp_workspace::{Workspace, WorkspaceName, WorkspacePath, WorkspaceService, WorkspaceState};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_workspace_operations() {
    // 1. Create workspace
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let workspace_path_str = temp_dir.path().to_string_lossy().into_owned();

    let name = WorkspaceName::new("test-workspace".into()).expect("Failed to create name");
    let path = WorkspacePath::new(workspace_path_str).expect("Failed to create path");

    let workspace =
        WorkspaceService::create_workspace(name, path).expect("Failed to create workspace");
    assert_eq!(workspace.state, WorkspaceState::Initializing);

    let active_workspace =
        WorkspaceService::initialize_workspace(&workspace).expect("Failed to initialize workspace");
    assert_eq!(active_workspace.state, WorkspaceState::Active);

    let ws_path = active_workspace.path.as_path();

    // Ensure the directory exists (tempdir creates it, but let's be sure)
    assert!(ws_path.exists());
    assert!(ws_path.is_dir());

    // 2. Add files
    let file1_path = ws_path.join("file1.txt");
    let file2_path = ws_path.join("file2.txt");

    fs::write(&file1_path, "content 1").expect("Failed to write file1");
    fs::write(&file2_path, "content 2").expect("Failed to write file2");

    assert!(file1_path.exists());
    assert!(file2_path.exists());

    // 3. List files
    let mut entries: Vec<_> = fs::read_dir(ws_path)
        .expect("Failed to read directory")
        .map(|res| res.map(|e| e.file_name().into_string().unwrap()))
        .collect::<Result<_, _>>()
        .expect("Failed to collect directory entries");

    entries.sort();
    assert_eq!(entries, vec!["file1.txt", "file2.txt"]);

    // 4. Remove files
    fs::remove_file(&file1_path).expect("Failed to remove file1");

    let remaining_entries: Vec<_> = fs::read_dir(ws_path)
        .expect("Failed to read directory")
        .map(|res| res.map(|e| e.file_name().into_string().unwrap()))
        .collect::<Result<_, _>>()
        .expect("Failed to collect directory entries");

    assert_eq!(remaining_entries, vec!["file2.txt"]);

    fs::remove_file(&file2_path).expect("Failed to remove file2");

    let empty_entries: Vec<_> = fs::read_dir(ws_path)
        .expect("Failed to read directory")
        .map(|res| res.map(|e| e.file_name().into_string().unwrap()))
        .collect::<Result<_, _>>()
        .expect("Failed to collect directory entries");

    assert!(empty_entries.is_empty());
}
