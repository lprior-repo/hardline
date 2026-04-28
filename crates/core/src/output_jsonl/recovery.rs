//! Recovery output types
//!
//! Provides error recovery reporting for the AI control plane.

use serde::{Deserialize, Serialize};

use crate::output_jsonl::{
    domain_types::{IssueId, RecoveryCapability, RecoveryExecution},
    errors::OutputLineError,
};

/// Recovery output line for error recovery reporting.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Recovery {
    pub issue_id: IssueId,
    pub assessment: Assessment,
    pub actions: Vec<RecoveryAction>,
}

/// Assessment of an error for recovery purposes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Assessment {
    pub severity: ErrorSeverity,
    pub capability: RecoveryCapability,
}

/// Severity of an error.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// A single recovery action.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecoveryAction {
    pub order: u32,
    pub description: String,
    pub execution: RecoveryExecution,
}

impl Recovery {
    #[must_use]
    pub const fn new(issue_id: IssueId, assessment: Assessment) -> Self {
        Self {
            issue_id,
            assessment,
            actions: Vec::new(),
        }
    }

    /// Append a recovery action.
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::RecoveryActionOverflow` when the number of
    /// actions cannot be represented as `u32`.
    pub fn with_action(
        self,
        description: String,
        command: Option<String>,
        automatic: bool,
    ) -> Result<Self, OutputLineError> {
        let order = u32::try_from(self.actions.len())
            .map_err(|_| OutputLineError::RecoveryActionOverflow)?;

        let execution = if automatic {
            let cmd = command.unwrap_or_else(|| {
                // Default command if none provided for automatic action
                "echo 'No command specified'".to_string()
            });
            RecoveryExecution::automatic(cmd)
        } else {
            RecoveryExecution::manual()
        };

        Ok(Self {
            actions: self
                .actions
                .into_iter()
                .chain(std::iter::once(RecoveryAction {
                    order,
                    description,
                    execution,
                }))
                .collect(),
            ..self
        })
    }
}

// Backward compatibility helpers
impl Assessment {
    #[must_use]
    pub const fn from_parts(
        severity: ErrorSeverity,
        recoverable: bool,
        recommended_action: String,
    ) -> Self {
        let capability = if recoverable {
            RecoveryCapability::Recoverable { recommended_action }
        } else {
            RecoveryCapability::NotRecoverable {
                reason: recommended_action,
            }
        };
        Self {
            severity,
            capability,
        }
    }

    #[must_use]
    pub const fn is_recoverable(&self) -> bool {
        matches!(self.capability, RecoveryCapability::Recoverable { .. })
    }

    #[must_use]
    pub const fn recommended_action(&self) -> Option<&str> {
        match &self.capability {
            RecoveryCapability::Recoverable { recommended_action } => {
                Some(recommended_action.as_str())
            }
            RecoveryCapability::NotRecoverable { .. } => None,
        }
    }
}
