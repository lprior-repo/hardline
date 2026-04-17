//! Phase types and error definitions

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::cleanup::PhaseType;
use crate::metrics::ScenarioResult;
use crate::parallel::ParallelError;
use crate::persistence::StoreError;
use crate::state::{IterationError, TransitionError};

/// Errors that can occur during phase execution
#[derive(Debug, Error)]
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
    PersistenceFailed(#[from] StoreError),

    #[error("Invalid state transition: {0}")]
    InvalidStateTransition(#[from] TransitionError),

    #[error("Iteration error: {0}")]
    IterationError(#[from] IterationError),

    #[error("Parallel execution failed: {0}")]
    ParallelExecutionFailed(#[from] ParallelError),

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

    #[test]
    fn test_phase_error_variants_display() {
        let errors: Vec<PhaseError> = vec![
            PhaseError::SpecReviewFailed("lint error".to_string()),
            PhaseError::SetupFailed("disk full".to_string()),
            PhaseError::DevelopmentFailed("oom".to_string()),
            PhaseError::ValidationFailed("scenario failed".to_string()),
            PhaseError::CleanupFailed("rollback error".to_string()),
            StoreError::NotFound("missing".to_string()).into(),
            TransitionError::InvalidTransition {
                from: crate::state::PipelineState::Pending,
                to: crate::state::PipelineState::Accepted,
            }
            .into(),
            ParallelError::InvalidPhaseConfiguration("bad config".to_string()).into(),
            PhaseError::DependencyNotMet(PhaseType::Validation),
        ];
        for err in &errors {
            let msg = format!("{err}");
            assert!(!msg.is_empty());
        }
    }

    #[test]
    fn test_phase_error_implements_error() {
        use std::error::Error;
        let err = PhaseError::SpecReviewFailed("test".to_string());
        assert!(err.source().is_none());

        // Typed errors preserve source chain
        let err: PhaseError = StoreError::NotFound("x".to_string()).into();
        assert!(err.source().is_some());
    }

    #[test]
    fn test_from_store_error() {
        let store_err = StoreError::NotFound("pipeline-42".to_string());
        let phase_err: PhaseError = store_err.into();
        assert!(format!("{phase_err}").contains("pipeline-42"));
    }

    #[test]
    fn test_from_transition_error() {
        let trans_err = TransitionError::InvalidTransition {
            from: crate::state::PipelineState::Pending,
            to: crate::state::PipelineState::Accepted,
        };
        let phase_err: PhaseError = trans_err.into();
        assert!(format!("{phase_err}").contains("Invalid"));
    }

    #[test]
    fn test_from_parallel_error() {
        let par_err = ParallelError::DependencyNotMet(PhaseType::Validation);
        let phase_err: PhaseError = par_err.into();
        assert!(format!("{phase_err}").contains("Validation"));
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
