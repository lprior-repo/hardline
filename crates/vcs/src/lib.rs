//! SCP VCS (Version Control System) Library
//!
//! Provides a unified VCS abstraction layer supporting both Jujutsu (JJ) and Git.
//!
//! # Architecture (DDD)
//!
//! - `domain` - Pure domain types, entities, and traits
//! - `application` - Use cases and service orchestration
//! - `infrastructure` - Backend implementations (Git, JJ)
//!
//! # Zero Unwrap Law
//!
//! All fallible operations return `Result<T, VcsError>`. No unwrap, no panic.

#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

pub mod application;
pub mod domain;
pub mod error;
pub mod infrastructure;

pub use application::{create_vcs_service, VcsService, VcsServiceImpl};
pub use domain::entities::{Branch, Commit, Workspace};
pub use domain::traits::VcsBackend;
pub use domain::value_objects::{VcsStatus, VcsType};
pub use error::{Result, VcsError};
pub use infrastructure::{GitBackend, JjBackend};

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
}
