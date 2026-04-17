//! Error types for JSONL output operations
//!
//! All output operations return `Result<T, OutputLineError>` to ensure
//! validation happens at construction time.

use thiserror::Error;

/// Errors that can occur when creating output lines.
#[derive(Debug, Clone, Error)]
pub enum OutputLineError {
    #[error("message is required but was empty")]
    EmptyMessage,
    #[error("title is required but was empty")]
    EmptyTitle,
    #[error("description is required but was empty")]
    EmptyDescription,
    #[error("session name is required but was empty")]
    EmptySessionName,
    #[error("at least one action is required")]
    NoActions,
    #[error("plan step count exceeds u32::MAX")]
    PlanStepOverflow,
    #[error("recovery action count exceeds u32::MAX")]
    RecoveryActionOverflow,
    #[error("workspace path must be absolute")]
    RelativePath,
    #[error("invalid warning code: {0}")]
    InvalidWarningCode(String),
    #[error("invalid action verb: {0}")]
    InvalidActionVerb(String),
    #[error("invalid action target: {0}")]
    InvalidActionTarget(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_message_display() {
        let msg = format!("{}", OutputLineError::EmptyMessage);
        assert!(msg.contains("message"));
        assert!(msg.contains("empty"));
    }

    #[test]
    fn empty_title_display() {
        let msg = format!("{}", OutputLineError::EmptyTitle);
        assert!(msg.contains("title"));
        assert!(msg.contains("empty"));
    }

    #[test]
    fn empty_description_display() {
        let msg = format!("{}", OutputLineError::EmptyDescription);
        assert!(msg.contains("description"));
        assert!(msg.contains("empty"));
    }

    #[test]
    fn empty_session_name_display() {
        let msg = format!("{}", OutputLineError::EmptySessionName);
        assert!(msg.contains("session name"));
        assert!(msg.contains("empty"));
    }

    #[test]
    fn no_actions_display() {
        let msg = format!("{}", OutputLineError::NoActions);
        assert!(msg.contains("action"));
    }

    #[test]
    fn plan_step_overflow_display() {
        let msg = format!("{}", OutputLineError::PlanStepOverflow);
        assert!(msg.contains("plan"));
        assert!(msg.contains("exceeds"));
    }

    #[test]
    fn recovery_action_overflow_display() {
        let msg = format!("{}", OutputLineError::RecoveryActionOverflow);
        assert!(msg.contains("recovery"));
        assert!(msg.contains("exceeds"));
    }

    #[test]
    fn relative_path_display() {
        let msg = format!("{}", OutputLineError::RelativePath);
        assert!(msg.contains("absolute"));
    }

    #[test]
    fn invalid_warning_code_display() {
        let err = OutputLineError::InvalidWarningCode("W999".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("W999"));
        assert!(msg.contains("warning code"));
    }

    #[test]
    fn invalid_action_verb_display() {
        let err = OutputLineError::InvalidActionVerb("fly".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("fly"));
        assert!(msg.contains("verb"));
    }

    #[test]
    fn invalid_action_target_display() {
        let err = OutputLineError::InvalidActionTarget("::bad::".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("::bad::"));
        assert!(msg.contains("target"));
    }

    #[test]
    fn all_variants_are_exhaustive() {
        let _ = OutputLineError::EmptyMessage;
        let _ = OutputLineError::EmptyTitle;
        let _ = OutputLineError::EmptyDescription;
        let _ = OutputLineError::EmptySessionName;
        let _ = OutputLineError::NoActions;
        let _ = OutputLineError::PlanStepOverflow;
        let _ = OutputLineError::RecoveryActionOverflow;
        let _ = OutputLineError::RelativePath;
        let _ = OutputLineError::InvalidWarningCode(String::new());
        let _ = OutputLineError::InvalidActionVerb(String::new());
        let _ = OutputLineError::InvalidActionTarget(String::new());
    }
}
