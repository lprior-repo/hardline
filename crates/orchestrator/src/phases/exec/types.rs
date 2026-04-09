//! Phase types and error definitions

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::cleanup::PhaseType;
use crate::metrics::ScenarioResult;

/// Errors that can occur during phase execution
#[derive(Debug, Clone, Error)]
pub enum PhaseError {
    #[error("Spec review failed: {0}")]
    SpecReviewFailed(String),

    #[error("Universe setup failed: {0}")]
    SetupFailed(String),

    #[error("Agent development failed: {0}")]
    DevelopmentFailed(String),

    #[error("Scenario validation failed: {0}")]
    ValidationFailed(String),

    #[error("Cleanup/rollback failed: {0}")]
    CleanupFailed(String),

    #[error("State persistence failed: {0}")]
    PersistenceFailed(String),

    #[error("Invalid state transition: {0}")]
    InvalidStateTransition(String),

    #[error("Parallel execution failed: {0}")]
    ParallelExecutionFailed(String),

    #[error("Dependency not met for phase: {0:?}")]
    DependencyNotMet(PhaseType),
}

/// Result of a phase execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseResult {
    pub success: bool,
    pub message: String,
    pub quality_score: Option<u32>,
    pub scenario_results: Vec<ScenarioResult>,
}

/// Decision made after validation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    Accept,
    Retry,
    Escalate,
    Fail,
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- PhaseError: construction tests ---

    #[test]
    fn test_phase_error_construction_string_variants() {
        let _ = PhaseError::SpecReviewFailed("lint error".to_string());
        let _ = PhaseError::SetupFailed("disk full".to_string());
        let _ = PhaseError::DevelopmentFailed("oom".to_string());
        let _ = PhaseError::ValidationFailed("scenario failed".to_string());
        let _ = PhaseError::CleanupFailed("rollback error".to_string());
        let _ = PhaseError::PersistenceFailed("io error".to_string());
        let _ = PhaseError::InvalidStateTransition("bad transition".to_string());
        let _ = PhaseError::ParallelExecutionFailed("dependency error".to_string());
    }

    #[test]
    fn test_phase_error_construction_phase_type_variant() {
        let _ = PhaseError::DependencyNotMet(PhaseType::Validation);
        let _ = PhaseError::DependencyNotMet(PhaseType::SpecReview);
        let _ = PhaseError::DependencyNotMet(PhaseType::UniverseSetup);
        let _ = PhaseError::DependencyNotMet(PhaseType::AgentDevelopment);
    }

    // --- PhaseError: Display formatting tests ---

    #[test]
    fn test_phase_error_display_spec_review_failed() {
        let err = PhaseError::SpecReviewFailed("lint error".to_string());
        assert_eq!(format!("{err}"), "Spec review failed: lint error");
    }

    #[test]
    fn test_phase_error_display_setup_failed() {
        let err = PhaseError::SetupFailed("disk full".to_string());
        assert_eq!(format!("{err}"), "Universe setup failed: disk full");
    }

    #[test]
    fn test_phase_error_display_development_failed() {
        let err = PhaseError::DevelopmentFailed("oom".to_string());
        assert_eq!(format!("{err}"), "Agent development failed: oom");
    }

    #[test]
    fn test_phase_error_display_validation_failed() {
        let err = PhaseError::ValidationFailed("scenario failed".to_string());
        assert_eq!(
            format!("{err}"),
            "Scenario validation failed: scenario failed"
        );
    }

    #[test]
    fn test_phase_error_display_cleanup_failed() {
        let err = PhaseError::CleanupFailed("rollback error".to_string());
        assert_eq!(format!("{err}"), "Cleanup/rollback failed: rollback error");
    }

    #[test]
    fn test_phase_error_display_persistence_failed() {
        let err = PhaseError::PersistenceFailed("io error".to_string());
        assert_eq!(format!("{err}"), "State persistence failed: io error");
    }

    #[test]
    fn test_phase_error_display_invalid_state_transition() {
        let err = PhaseError::InvalidStateTransition("bad transition".to_string());
        assert_eq!(format!("{err}"), "Invalid state transition: bad transition");
    }

    #[test]
    fn test_phase_error_display_parallel_execution_failed() {
        let err = PhaseError::ParallelExecutionFailed("dependency error".to_string());
        assert_eq!(
            format!("{err}"),
            "Parallel execution failed: dependency error"
        );
    }

    #[test]
    fn test_phase_error_display_dependency_not_met() {
        let err = PhaseError::DependencyNotMet(PhaseType::Validation);
        assert_eq!(format!("{err}"), "Dependency not met for phase: Validation");
    }

    #[test]
    fn test_phase_error_display_preserves_message_content() {
        let msg = "error with 'quotes' and \"double quotes\" and \n newlines";
        let err = PhaseError::SpecReviewFailed(msg.to_string());
        assert!(format!("{err}").contains(msg));
    }

    #[test]
    fn test_phase_error_display_empty_message() {
        let err = PhaseError::SetupFailed(String::new());
        assert_eq!(format!("{err}"), "Universe setup failed: ");
    }

    // --- PhaseError: Error trait / source() tests ---

    #[test]
    fn test_phase_error_source_is_none_for_all_string_variants() {
        use std::error::Error;
        let errors: Vec<PhaseError> = vec![
            PhaseError::SpecReviewFailed("e".to_string()),
            PhaseError::SetupFailed("e".to_string()),
            PhaseError::DevelopmentFailed("e".to_string()),
            PhaseError::ValidationFailed("e".to_string()),
            PhaseError::CleanupFailed("e".to_string()),
            PhaseError::PersistenceFailed("e".to_string()),
            PhaseError::InvalidStateTransition("e".to_string()),
            PhaseError::ParallelExecutionFailed("e".to_string()),
        ];
        for err in &errors {
            assert!(err.source().is_none(), "expected no source for {err:?}");
        }
    }

    #[test]
    fn test_phase_error_source_is_none_for_dependency_not_met() {
        use std::error::Error;
        let err = PhaseError::DependencyNotMet(PhaseType::Validation);
        assert!(err.source().is_none());
    }

    #[test]
    fn test_phase_error_implements_error_trait() {
        fn assert_error<E: std::error::Error>(_: &E) {}
        assert_error(&PhaseError::SpecReviewFailed("test".to_string()));
        assert_error(&PhaseError::DependencyNotMet(PhaseType::SpecReview));
    }

    // --- PhaseError: chain / nesting tests ---

    #[test]
    fn test_phase_error_no_chain() {
        // PhaseError variants don't use #[source], so error chains are not possible.
        // Verify that repeated source() calls on a dyn Error always return None.
        use std::error::Error;
        let err: Box<dyn Error> = Box::new(PhaseError::SetupFailed("root".to_string()));
        assert!(err.source().is_none());
    }

    #[test]
    fn test_phase_error_debug_format_includes_variant() {
        // Debug output should contain the variant name for diagnostics
        let err = PhaseError::ValidationFailed("bad".to_string());
        let debug = format!("{err:?}");
        assert!(debug.contains("ValidationFailed"));
    }

    #[test]
    fn test_phase_error_clone_preserves_display() {
        let err = PhaseError::PersistenceFailed("disk I/O".to_string());
        let cloned = err.clone();
        assert_eq!(format!("{err}"), format!("{cloned}"));
    }

    #[test]
    fn test_decision_serde_roundtrip() {
        let decisions = [
            Decision::Accept,
            Decision::Retry,
            Decision::Escalate,
            Decision::Fail,
        ];
        for decision in &decisions {
            let json = serde_json::to_string(decision).expect("serialize");
            let deserialized: Decision = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(*decision, deserialized);
        }
    }

    #[test]
    fn test_decision_uses_snake_case() {
        assert_eq!(
            serde_json::to_string(&Decision::Accept).expect("serialize"),
            "\"accept\""
        );
        assert_eq!(
            serde_json::to_string(&Decision::Retry).expect("serialize"),
            "\"retry\""
        );
        assert_eq!(
            serde_json::to_string(&Decision::Escalate).expect("serialize"),
            "\"escalate\""
        );
        assert_eq!(
            serde_json::to_string(&Decision::Fail).expect("serialize"),
            "\"fail\""
        );
    }

    #[test]
    fn test_phase_result_serde_roundtrip() {
        let result = PhaseResult {
            success: true,
            message: "All good".to_string(),
            quality_score: Some(95),
            scenario_results: vec![ScenarioResult {
                name: "s1".to_string(),
                passed: true,
                duration_secs: 1.5,
                error: None,
            }],
        };
        let json = serde_json::to_string(&result).expect("serialize");
        let deserialized: PhaseResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(result.success, deserialized.success);
        assert_eq!(result.quality_score, deserialized.quality_score);
        assert_eq!(
            result.scenario_results.len(),
            deserialized.scenario_results.len()
        );
    }

    #[test]
    fn test_decision_equality_and_inequality() {
        assert_eq!(Decision::Accept, Decision::Accept);
        assert_ne!(Decision::Accept, Decision::Fail);
    }

    #[test]
    fn test_decision_all_variants_distinct() {
        let decisions = [
            Decision::Accept,
            Decision::Retry,
            Decision::Escalate,
            Decision::Fail,
        ];
        for i in 0..decisions.len() {
            for j in (i + 1)..decisions.len() {
                assert_ne!(decisions[i], decisions[j]);
            }
        }
    }
}
