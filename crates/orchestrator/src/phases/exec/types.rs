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

    #[test]
    fn test_phase_error_variants_display() {
        let errors = [
            PhaseError::SpecReviewFailed("lint error".to_string()),
            PhaseError::SetupFailed("disk full".to_string()),
            PhaseError::DevelopmentFailed("oom".to_string()),
            PhaseError::ValidationFailed("scenario failed".to_string()),
            PhaseError::CleanupFailed("rollback error".to_string()),
            PhaseError::PersistenceFailed("io error".to_string()),
            PhaseError::InvalidStateTransition("bad transition".to_string()),
            PhaseError::ParallelExecutionFailed("dependency error".to_string()),
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
