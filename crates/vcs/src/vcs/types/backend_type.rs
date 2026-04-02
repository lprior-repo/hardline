//! Backend type definition
//!
//! This module provides `BackendType` - enumeration distinguishing Git vs JJ repositories.

use serde::{Deserialize, Serialize};

/// Version control system backend type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BackendType {
    /// Git repository (contains .git directory)
    Git,
    /// Jujutsu repository (contains .jj directory)
    Jj,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_type_eq() {
        assert_eq!(BackendType::Git, BackendType::Git);
        assert_eq!(BackendType::Jj, BackendType::Jj);
        assert_ne!(BackendType::Git, BackendType::Jj);
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
        set.insert(BackendType::Jj);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn backend_type_debug() {
        assert_eq!(format!("{:?}", BackendType::Git), "Git");
        assert_eq!(format!("{:?}", BackendType::Jj), "Jj");
    }

    #[test]
    fn backend_type_serde_roundtrip() {
        for bt in [BackendType::Git, BackendType::Jj] {
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

    #[test]
    fn backend_type_serde_json_jj_is_string_jj() {
        let json = serde_json::to_string(&BackendType::Jj).expect("serialize");
        assert_eq!(json, "\"Jj\"");
    }
}
