//! Workspace aggregate tests.
//!
//! Tests for the Workspace aggregate root.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::domain::aggregates::workspace::Workspace;
    use crate::domain::aggregates::workspace_error::WorkspaceError;
    use crate::domain::identifiers::WorkspaceName;
    use crate::domain::workspace::WorkspaceState;

    fn create_test_workspace() -> Workspace {
        let name = WorkspaceName::parse("test-workspace").expect("valid name");
        let path = PathBuf::from("/tmp");
        Workspace::create(name, path).expect("workspace created")
    }

    #[test]
    fn test_create_workspace() {
        let workspace = create_test_workspace();
        assert!(workspace.is_creating());
        assert!(!workspace.is_ready());
        assert!(!workspace.is_active());
        assert_eq!(workspace.name.as_str(), "test-workspace");
    }

    #[test]
    fn test_creating_to_ready() {
        let workspace = create_test_workspace();
        let ready = workspace.mark_ready().expect("transition valid");
        assert!(ready.is_ready());
        assert!(!ready.is_creating());
    }

    #[test]
    fn test_ready_to_active() {
        let workspace = create_test_workspace();
        let ready = workspace.mark_ready().expect("transition valid");
        let active = ready.mark_active().expect("transition valid");
        assert!(active.is_active());
        assert!(!active.is_ready());
    }

    #[test]
    fn test_active_to_cleaning() {
        let workspace = create_test_workspace();
        let ready = workspace.mark_ready().expect("transition valid");
        let active = ready.mark_active().expect("transition valid");
        let cleaning = active.start_cleaning().expect("transition valid");
        assert!(cleaning.is_cleaning());
        assert!(!cleaning.is_active());
    }

    #[test]
    fn test_cleaning_to_removed() {
        let workspace = create_test_workspace();
        let ready = workspace.mark_ready().expect("transition valid");
        let active = ready.mark_active().expect("transition valid");
        let cleaning = active.start_cleaning().expect("transition valid");
        let removed = cleaning.mark_removed().expect("transition valid");
        assert!(removed.is_removed());
        assert!(removed.is_terminal());
    }

    #[test]
    fn test_ready_to_removed() {
        let workspace = create_test_workspace();
        let ready = workspace.mark_ready().expect("transition valid");
        let removed = ready.mark_removed().expect("transition valid");
        assert!(removed.is_removed());
    }

    #[test]
    fn test_creating_to_removed() {
        let workspace = create_test_workspace();
        let removed = workspace.mark_removed().expect("transition valid");
        assert!(removed.is_removed());
    }

    #[test]
    fn test_invalid_state_transition() {
        let workspace = create_test_workspace();
        let result = workspace.mark_active();
        assert!(matches!(result, Err(WorkspaceError::InvalidStateTransition { .. })));
        let removed = workspace.mark_removed().expect("transition valid");
        let result = removed.mark_ready();
        assert!(matches!(result, Err(WorkspaceError::InvalidStateTransition { .. })));
    }

    #[test]
    fn test_validate_ready() {
        let workspace = create_test_workspace();
        let result = workspace.validate_ready();
        assert!(matches!(result, Err(WorkspaceError::NotReady(_))));
        let ready = workspace.mark_ready().expect("transition valid");
        assert!(ready.validate_ready().is_ok());
    }

    #[test]
    fn test_validate_active() {
        let workspace = create_test_workspace();
        let ready = workspace.mark_ready().expect("transition valid");
        let result = ready.validate_active();
        assert!(matches!(result, Err(WorkspaceError::NotActive(_))));
        let active = ready.mark_active().expect("transition valid");
        assert!(active.validate_active().is_ok());
    }

    #[test]
    fn test_validate_can_use() {
        let workspace = create_test_workspace();
        let result = workspace.validate_can_use();
        assert!(matches!(result, Err(WorkspaceError::CannotUse(_))));
        let ready = workspace.mark_ready().expect("transition valid");
        assert!(ready.validate_can_use().is_ok());
        let active = ready.mark_active().expect("transition valid");
        assert!(active.validate_can_use().is_ok());
    }

    #[test]
    fn test_path_not_found() {
        let name = WorkspaceName::parse("test").expect("valid name");
        let path = PathBuf::from("/nonexistent/path");
        let result = Workspace::create(name, path);
        assert!(matches!(result, Err(WorkspaceError::PathNotFound(_))));
    }

    #[test]
    fn test_change_path() {
        let workspace = create_test_workspace();
        let new_path = PathBuf::from("/var/tmp");
        let changed = workspace.change_path(new_path.clone()).expect("path changed");
        assert_eq!(changed.path, new_path);
    }

    #[test]
    fn test_builder() {
        let name = WorkspaceName::parse("builder-test").expect("valid name");
        let path = PathBuf::from("/tmp");
        let workspace = Workspace::builder()
            .name(name.clone())
            .path(path)
            .build()
            .expect("builder works");
        assert_eq!(workspace.name, name);
        assert!(workspace.is_creating());
    }

    #[test]
    fn test_builder_with_state() {
        let name = WorkspaceName::parse("builder-state").expect("valid name");
        let path = PathBuf::from("/tmp");
        let workspace = Workspace::builder()
            .name(name)
            .path(path)
            .state(WorkspaceState::Ready)
            .build()
            .expect("builder works");
        assert!(workspace.is_ready());
    }
}
