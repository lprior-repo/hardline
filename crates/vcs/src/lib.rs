//! SCP VCS (Version Control System) Library
//!
//! Provides a unified VCS abstraction layer for Git.
//!
//! # Architecture (DDD)
//!
//! - `domain` - Pure domain types, entities, and traits
//! - `application` - Use cases and service orchestration
//! - `infrastructure` - Backend implementations (Git)
//!
//! # Zero Unwrap Law
//!
//! All fallible operations return `Result<T, VcsError>`. No unwrap, no panic.

#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]
#![allow(clippy::result_large_err)]

pub mod application;
pub mod domain;
pub mod error;
pub mod gix;
pub mod hooks;
pub mod infrastructure;

pub use application::{create_vcs_service, VcsService, VcsServiceImpl};
pub use application::ops::Transaction;
pub use domain::entities::{Branch, Commit, Workspace};
pub use domain::entities::ops::{
    LocalRefEntry, OpError, OpKind, OpReceipt, OpStatus, PlanSummary, RemoteRefEntry,
};
pub use domain::traits::VcsBackend;
pub use domain::value_objects::{VcsStatus, VcsType};
pub use error::{Result, VcsError};
pub use infrastructure::{GitBackend, GitCliBackend};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_vcs_type_detection_none() {
        let vcs_type = VcsType::detect(Path::new("/tmp/nonexistent"));
        assert!(vcs_type.is_none());
    }

    #[test]
    fn test_vcs_service() {
        let service = create_vcs_service();
        let _ = service.detect_vcs_type(Path::new("/"));
    }

    #[test]
    fn test_re_exports_exist() {
        // Verify that all re-exports from lib.rs are accessible
        let _ = create_vcs_service();
        let _: fn() -> VcsServiceImpl = VcsServiceImpl::new;
    }

    #[test]
    fn test_vcs_type_display_variants() {
        assert_eq!(format!("{}", VcsStatus::Clean), "clean");
        assert_eq!(format!("{}", VcsStatus::Dirty), "dirty");
        assert_eq!(format!("{}", VcsStatus::Conflicted), "conflicted");
        assert_eq!(format!("{}", VcsStatus::Detached), "detached");
    }

    #[test]
    fn test_entity_construction() {
        let _commit = Commit::new(
            "id".to_string(),
            "msg".to_string(),
            "author".to_string(),
            chrono::Utc::now(),
            vec![],
        );
        let _branch = Branch::new("main".to_string(), true, None);
        let _workspace = Workspace::new("default".to_string(), "main".to_string(), true);
    }
}
