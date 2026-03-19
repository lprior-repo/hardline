use crate::error::WorkspaceError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceName(String);

const MAX_NAME_LENGTH: usize = 255;
const VALID_NAME_CHARS: &[char] = &['-', '_'];

fn validate_name_chars(name: &str) -> bool {
    name.chars()
        .all(|c| c.is_alphanumeric() || VALID_NAME_CHARS.contains(&c))
}

fn validate_name(name: &str) -> Result<(), WorkspaceError> {
    match () {
        _ if name.is_empty() => Err(WorkspaceError::InvalidWorkspaceName("empty name".into())),
        _ if name.len() > MAX_NAME_LENGTH => {
            Err(WorkspaceError::InvalidWorkspaceName("name too long".into()))
        }
        _ if !validate_name_chars(name) => Err(WorkspaceError::InvalidWorkspaceName(
            "name contains invalid characters".into(),
        )),
        _ => Ok(()),
    }
}

fn make_workspace_name(name: String) -> Result<WorkspaceName, WorkspaceError> {
    validate_name(&name).map(|_| WorkspaceName(name))
}

impl WorkspaceName {
    pub fn new(name: String) -> Result<Self, WorkspaceError> {
        make_workspace_name(name)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn default_name() -> &'static str {
        "default"
    }
}

impl WorkspaceName {
    fn new_unchecked(name: String) -> Self {
        Self(name)
    }
}

impl Default for WorkspaceName {
    fn default() -> Self {
        Self::new_unchecked(Self::default_name().into())
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
