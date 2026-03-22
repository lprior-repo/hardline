//! Domain types for CLI contracts.
//!
//! This module contains semantic newtypes that make illegal states unrepresentable.
//! Following Scott Wlaschin's DDD principles:
//! - Parse at boundaries, validate once
//! - Use semantic newtypes instead of primitives
//! - Make illegal states unrepresentable with enums

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

mod agent_types;
mod config;
mod identifiers;
mod status_enums;
mod value_objects;

pub use agent_types::{AgentType, FileStatus, OutputFormat};
pub use config::{ConfigKey, ConfigValue};
pub use identifiers::{AgentId, SessionName, TaskId};
pub use status_enums::{AgentStatus, ConfigScope, SessionStatus, TaskPriority, TaskStatus};
pub use value_objects::{Limit, NonEmptyString, Priority, TimeoutSeconds};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_name_valid() {
        assert!(SessionName::try_from("valid-name").is_ok());
        assert!(SessionName::try_from("Feature_Auth").is_ok());
        assert!(SessionName::try_from("a").is_ok());
        assert!(SessionName::try_from("test123").is_ok());
    }

    #[test]
    fn test_session_name_trims_whitespace() {
        let name = SessionName::parse("  valid-name  ").expect("valid");
        assert_eq!(name.as_str(), "valid-name");
    }

    #[test]
    fn test_session_name_invalid() {
        assert!(SessionName::try_from("").is_err());
        assert!(SessionName::try_from("1invalid").is_err());
        assert!(SessionName::try_from("-invalid").is_err());
        assert!(SessionName::try_from("_invalid").is_err());
        assert!(SessionName::try_from("invalid name").is_err());
        assert!(SessionName::try_from("invalid@name").is_err());
    }

    #[test]
    fn test_session_status_transitions() {
        assert!(SessionStatus::Creating.can_transition_to(SessionStatus::Active));
        assert!(SessionStatus::Active.can_transition_to(SessionStatus::Paused));
        assert!(SessionStatus::Paused.can_transition_to(SessionStatus::Active));
        assert!(!SessionStatus::Creating.can_transition_to(SessionStatus::Paused));
        assert!(!SessionStatus::Completed.can_transition_to(SessionStatus::Active));
    }

    #[test]
    fn test_task_priority_ordering() {
        assert!(TaskPriority::P0 < TaskPriority::P1);
        assert!(TaskPriority::P1 < TaskPriority::P2);
        assert!(TaskPriority::P2 < TaskPriority::P3);
        assert!(TaskPriority::P3 < TaskPriority::P4);
    }

    #[test]
    fn test_limit_validation() {
        assert!(Limit::try_from(1).is_ok());
        assert!(Limit::try_from(1000).is_ok());
        assert!(Limit::try_from(0).is_err());
        assert!(Limit::try_from(1001).is_err());
    }

    #[test]
    fn test_timeout_validation() {
        assert!(TimeoutSeconds::try_from(1).is_ok());
        assert!(TimeoutSeconds::try_from(3600).is_ok());
        assert!(TimeoutSeconds::try_from(86400).is_ok());
        assert!(TimeoutSeconds::try_from(0).is_err());
        assert!(TimeoutSeconds::try_from(86401).is_err());
    }
}
