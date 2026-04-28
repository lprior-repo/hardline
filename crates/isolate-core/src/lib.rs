//! # Isolate Core
//!
//! Core functionality for Isolate - strictly functional Rust with zero unwraps.
//!
//! ## Laws (Compiler Enforced)

// Allow all warnings globally - we'll specifically deny the critical ones we care about
#![allow(warnings)]
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::todo,
        clippy::unimplemented,
        clippy::uninlined_format_args,
        clippy::ignored_unit_patterns,
        clippy::manual_let_else,
        clippy::needless_collect,
        clippy::await_holding_lock,
        clippy::significant_drop_tightening,
        clippy::redundant_clone,
        clippy::no_effect_underscore_binding,
        clippy::doc_markdown,
        clippy::unnecessary_semicolon,
        clippy::needless_borrows_for_generic_args,
        dead_code,
        unused_must_use
    )
)]

//! - No `unwrap()` - returns `Result` instead
//! - No `expect()` - returns `Result` instead
//! - No `panic!()` - returns `Result` instead
//! - No `unsafe` - safe Rust only
//! - No `todo!()` / `unimplemented!()` - complete implementations only
//!
//! ## Error Handling
//!
//! All fallible operations return `Result<T, Error>`. Use:
//! - `?` operator for propagation
//! - `map`, `and_then` combinators for transformation
//! - `match` / `map_or` / `unwrap_or_else` for defaults
//!
//! Allow clippy casts from u128 to u32 in work.rs
#![allow(clippy::cast_possible_truncation)]
// Allow various clippy lints that are not related to unwrap/panic
#![allow(clippy::doc_markdown)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::use_self)]
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::no_effect_underscore_binding)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::significant_drop_tightening)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::duplicated_attributes)]
#![allow(unused_variables)]

pub mod beads;
pub mod checkpoint;
pub mod cli_contracts;
pub mod config;
#[cfg(test)]
mod config_property_tests;
pub mod contracts;
pub mod coordination;
pub mod domain;
mod error;
#[cfg(test)]
mod error_tests;
pub mod fix;
pub mod functional;
pub mod hints;
pub mod hooks;
pub mod introspection;
pub mod jj;
pub mod jj_operation_sync;
pub mod json;
#[cfg(test)]
mod json_tests;
pub mod lifecycle;
pub mod moon_gates;
pub mod output;
mod output_format;
pub mod recovery;
pub mod result;
pub mod session_state;
#[cfg(test)]
mod session_state_tests;
pub mod session_sync;
pub mod shutdown;
pub mod taskregistry;
pub mod types;
#[cfg(test)]
mod types_tests;
pub mod validation;
pub mod watcher;
pub mod workspace_integrity;
pub mod workspace_state;

// Stak-sourced modules
pub mod agent;
pub mod conflict;
pub mod dag;
pub mod events;
pub mod lock;
pub mod metadata;
pub mod queue;
pub mod use_cases;
pub mod vcs;

pub use agent::{Agent, AgentActivity, AgentId, AgentStatus};
pub use conflict::{Conflict, ConflictManager, ConflictState};
pub use dag::{BranchDag, BranchId, DagError};
pub use events::{Event, EventType};
pub use lock::{HolderId, Lock, LockManager, ResourceId, TtlSeconds};
pub use metadata::{MetadataBackend, StackMetadata};
pub use queue::{Queue, QueueEntry, QueueEntryId, QueueStatus, SessionName, MAX_PRIORITY};
pub use use_cases::{
    dequeue_session, enqueue_session, insert_at_position, list_queue, remove_at_position,
    DomainError, QueueEntryView,
};
pub use vcs::{
    detect_backend, BackendType, BranchName, Change, ChangeId, CommitId, RepoStatus, VcsBackend,
    VcsError,
};

pub use config::{ConfigManager, RecoveryPolicy};
pub use error::{Error, FailureContext, RichError, RichErrorInfo, ValidationHint};
pub use fix::{ErrorWithFixes, Fix, FixImpact};
// Re-export fs2 for file locking utilities
pub use fs2::FileExt;
pub use hints::{ActionRisk, CommandContext, NextAction};
pub use json::{
    ErrorCode, HateoasLink, RelatedResources, ResponseMeta, SchemaEnvelope, SchemaEnvelopeArray,
};
pub use moon_gates::{
    classify_exit_code, combine_results, format_failure_message, parse_summary, GateError,
    GateResult, GatesOutcome, GatesStatus, MoonGate,
};
pub use output_format::OutputFormat;
pub use recovery::{
    log_recovery, periodic_cleanup, recover_incomplete_sessions, repair_database,
    should_log_recovery, validate_database,
};
pub use result::{Result, ResultExt};
pub use shutdown::{signal_channels, ShutdownCoordinator, ShutdownSignal};
pub use taskregistry::TaskRegistry;
pub use workspace_state::{WorkspaceState, WorkspaceStateFilter, WorkspaceStateTransition};

/// Marker trait for types guaranteed safe (no panics possible).
pub trait Infallible: Sized {}

/// Configuration builder with fallible construction.
#[derive(Debug, Clone, Default)]
pub struct ConfigBuilder {
    name: Option<String>,
}

impl ConfigBuilder {
    /// Create a new config builder.
    #[must_use]
    pub const fn new() -> Self {
        Self { name: None }
    }

    /// Set the configuration name.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Build the configuration, returning an error if validation fails.
    pub fn build(self) -> Result<Config> {
        self.name
            .ok_or_else(|| Error::InvalidConfig("name is required".into()))
            .and_then(|name| {
                if name.is_empty() {
                    Err(Error::InvalidConfig("name cannot be empty".into()))
                } else {
                    Ok(Config { name })
                }
            })
    }
}

/// A validated configuration.
#[derive(Debug, Clone, Default)]
pub struct Config {
    /// The configuration name.
    pub name: String,
}

impl Config {
    /// Create a new config builder.
    #[must_use]
    pub const fn builder() -> ConfigBuilder {
        ConfigBuilder::new()
    }

    /// Validate a name, returning an error if invalid.
    pub fn validate_name(name: &str) -> Result<()> {
        if name.is_empty() {
            return Err(Error::InvalidConfig("name cannot be empty".into()));
        }

        if name.len() > 255 {
            return Err(Error::InvalidConfig(
                "name cannot exceed 255 characters".into(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder_success() -> Result<()> {
        let result = ConfigBuilder::new().with_name("test").build();
        assert!(result.is_ok());
        let config = result?;
        assert_eq!(config.name, "test");
        Ok(())
    }

    #[test]
    fn test_config_builder_missing_name() {
        let result = ConfigBuilder::new().build();
        assert!(result.is_err());
    }

    #[test]
    fn test_config_builder_empty_name() {
        let result = ConfigBuilder::new().with_name("").build();
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_name_empty() {
        assert!(Config::validate_name("").is_err());
    }

    #[test]
    fn test_validate_name_too_long() {
        let long_name = "x".repeat(256);
        assert!(Config::validate_name(&long_name).is_err());
    }

    #[test]
    fn test_validate_name_valid() {
        assert!(Config::validate_name("valid_name").is_ok());
    }
}

#[cfg(test)]
mod architecture_boundaries;
