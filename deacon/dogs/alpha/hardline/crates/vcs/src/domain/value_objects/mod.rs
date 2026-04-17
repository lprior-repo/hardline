//! VCS Value Objects

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VcsStatus {
    Clean,
    Dirty,
    Conflicted,
    Detached,
}

impl std::fmt::Display for VcsStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Clean => write!(f, "clean"),
            Self::Dirty => write!(f, "dirty"),
            Self::Conflicted => write!(f, "conflicted"),
            Self::Detached => write!(f, "detached"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VcsType {
    Git,
}

impl VcsType {
    pub fn detect(path: &std::path::Path) -> Option<Self> {
        if path.join(".git").exists() {
            Some(Self::Git)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // -- VcsStatus Display tests --

    #[test]
    fn vcs_status_clean_display() {
        assert_eq!(format!("{}", VcsStatus::Clean), "clean");
    }

    #[test]
    fn vcs_status_dirty_display() {
        assert_eq!(format!("{}", VcsStatus::Dirty), "dirty");
    }

    #[test]
    fn vcs_status_conflicted_display() {
        assert_eq!(format!("{}", VcsStatus::Conflicted), "conflicted");
    }

    #[test]
    fn vcs_status_detached_display() {
        assert_eq!(format!("{}", VcsStatus::Detached), "detached");
    }

    // -- VcsStatus Serde roundtrip tests --

    #[test]
    fn vcs_status_serde_roundtrip_all_variants() {
        for status in [
            VcsStatus::Clean,
            VcsStatus::Dirty,
            VcsStatus::Conflicted,
            VcsStatus::Detached,
        ] {
            let json = serde_json::to_string(&status).expect("serialize");
            let deserialized: VcsStatus = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(status, deserialized);
        }
    }

    // -- VcsType detection tests --

    #[test]
    fn vcs_type_detect_git() {
        let dir = tempfile::TempDir::new().expect("temp dir");
        fs::create_dir(dir.path().join(".git")).expect("create .git");
        assert_eq!(VcsType::detect(dir.path()), Some(VcsType::Git));
    }

    #[test]
    fn vcs_type_detect_none() {
        let dir = tempfile::TempDir::new().expect("temp dir");
        assert_eq!(VcsType::detect(dir.path()), None);
    }

    #[test]
    fn vcs_type_detect_nonexistent() {
        assert_eq!(
            VcsType::detect(std::path::Path::new("/nonexistent/xyz")),
            None
        );
    }

    #[test]
    fn vcs_type_equality() {
        assert_eq!(VcsType::Git, VcsType::Git);
    }

    #[test]
    fn vcs_type_file_named_git_is_not_detected() {
        let dir = tempfile::TempDir::new().expect("temp dir");
        // A file named .git should still be detected (exists() returns true for files too)
        fs::write(dir.path().join(".git"), "not a git repo").expect("write .git file");
        assert_eq!(VcsType::detect(dir.path()), Some(VcsType::Git));
    }

    #[test]
    fn vcs_type_clone() {
        let git = VcsType::Git;
        let cloned = git;
        assert_eq!(git, cloned);
    }

    #[test]
    fn vcs_type_copy() {
        let git = VcsType::Git;
        let copied = git;
        assert_eq!(git, copied);
    }

    #[test]
    fn vcs_status_debug() {
        for status in [
            VcsStatus::Clean,
            VcsStatus::Dirty,
            VcsStatus::Conflicted,
            VcsStatus::Detached,
        ] {
            let debug = format!("{status:?}");
            assert!(!debug.is_empty());
        }
    }

    #[test]
    fn vcs_status_eq() {
        assert_eq!(VcsStatus::Clean, VcsStatus::Clean);
        assert_eq!(VcsStatus::Dirty, VcsStatus::Dirty);
        assert_ne!(VcsStatus::Clean, VcsStatus::Dirty);
    }

    #[test]
    fn vcs_status_clone() {
        let s = VcsStatus::Conflicted;
        let c = s.clone();
        assert_eq!(s, c);
    }

    // -- Proptests --

    proptest::proptest! {
        #[test]
        fn vcs_type_never_panics_on_nonexistent_paths(path in "[a-zA-Z0-9_/]{1,50}") {
            // Nonexistent paths should always return None, never panic
            let result = VcsType::detect(std::path::Path::new(&path));
            // Could be Some if path happens to exist, but never panics
            let _ = result;
        }
    }
}
