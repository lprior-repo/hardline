use crate::error::WorkspaceError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LockHolder(String);

impl LockHolder {
    pub fn new(holder: String) -> Result<Self, WorkspaceError> {
        if holder.is_empty() {
            return Err(WorkspaceError::InvalidLockHolder("empty holder".into()));
        }
        Ok(Self(holder))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for LockHolder {
    fn default() -> Self {
        Self("system".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_holder_valid() {
        let holder = LockHolder::new("agent-42".into());
        assert!(holder.is_ok());
    }

    #[test]
    fn lock_holder_empty_fails() {
        let holder = LockHolder::new("".into());
        assert!(holder.is_err());
    }
}
