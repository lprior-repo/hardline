use crate::error::WorkspaceError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BranchName(String);

impl BranchName {
    pub fn new(name: String) -> Result<Self, WorkspaceError> {
        if name.is_empty() {
            return Err(WorkspaceError::InvalidBranchName("empty name".into()));
        }
        if name.contains('\0') {
            return Err(WorkspaceError::InvalidBranchName("null character not allowed".into()));
        }
        Ok(Self(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for BranchName {
    fn default() -> Self {
        Self::new("main".into()).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branch_name_valid() {
        let name = BranchName::new("feature/login".into());
        assert!(name.is_ok());
    }

    #[test]
    fn branch_name_empty_fails() {
        let name = BranchName::new("".into());
        assert!(name.is_err());
    }
}
