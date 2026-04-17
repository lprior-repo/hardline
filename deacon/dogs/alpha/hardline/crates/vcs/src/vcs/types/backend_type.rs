//! Backend type definition
//!
//! This module provides `BackendType` - enumeration identifying Git repositories.

use serde::{Deserialize, Serialize};

/// Version control system backend type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BackendType {
    /// Git repository (contains .git directory)
    Git,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_type_eq() {
        assert_eq!(BackendType::Git, BackendType::Git);
    }

    #[test]
    fn backend_type_clone() {
        let git = BackendType::Git;
        let cloned = git;
        assert_eq!(git, cloned);
    }

    #[test]
    fn backend_type_copy() {
        let git = BackendType::Git;
        let copied = git;
        assert_eq!(git, copied);
    }

    #[test]
    fn backend_type_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(BackendType::Git);
        set.insert(BackendType::Git);
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn backend_type_debug() {
        assert_eq!(format!("{:?}", BackendType::Git), "Git");
    }

    #[test]
    fn backend_type_serde_roundtrip() {
        for bt in [BackendType::Git] {
            let json = serde_json::to_string(&bt).expect("serialize");
            let deserialized: BackendType = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(bt, deserialized);
        }
    }

    #[test]
    fn backend_type_serde_json_git_is_string_git() {
        let json = serde_json::to_string(&BackendType::Git).expect("serialize");
        assert_eq!(json, "\"Git\"");
    }
}
