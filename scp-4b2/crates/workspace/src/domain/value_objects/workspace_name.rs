use crate::error::WorkspaceError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceName(String);

impl WorkspaceName {
    pub fn new(name: String) -> Result<Self, WorkspaceError> {
        if name.is_empty() {
            return Err(WorkspaceError::InvalidWorkspaceName("empty name".into()));
        }
        if name.len() > 255 {
            return Err(WorkspaceError::InvalidWorkspaceName("name too long".into()));
        }
        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(WorkspaceError::InvalidWorkspaceName(
                "name contains invalid characters".into(),
            ));
        }
        Ok(Self(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for WorkspaceName {
    fn default() -> Self {
        Self::new("default".into()).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_name_valid() {
        let name = WorkspaceName::new("my-workspace_123".into());
        assert!(name.is_ok());
    }

    #[test]
    fn workspace_name_empty_fails() {
        let name = WorkspaceName::new("".into());
        assert!(name.is_err());
    }

    #[test]
    fn workspace_name_with_slash_fails() {
        let name = WorkspaceName::new("my/workspace".into());
        assert!(name.is_err());
    }

    #[test]
    fn workspace_name_default_is_workspace() {
        assert_eq!(WorkspaceName::default().as_str(), "default");
    }
}
