//! Plan output types
//!
//! Provides plan structure for multi-step AI operations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::output_jsonl::domain_types::{PlanDescription, PlanTitle};
use crate::output_jsonl::errors::OutputLineError;

/// Plan output line for multi-step operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Plan {
    pub title: PlanTitle,
    pub description: PlanDescription,
    pub steps: Vec<PlanStep>,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub created_at: DateTime<Utc>,
}

/// A single step in a plan.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanStep {
    pub order: u32,
    pub description: String,
    pub status: ActionStatus,
}

/// Status of an action or plan step.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Skipped,
}

impl Plan {
    /// Create a new plan output line.
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::EmptyTitle` if `title` is blank.
    /// Returns `OutputLineError::EmptyDescription` if `description` is blank.
    pub fn new(title: PlanTitle, description: PlanDescription) -> Result<Self, OutputLineError> {
        Ok(Self {
            title,
            description,
            steps: Vec::new(),
            created_at: Utc::now(),
        })
    }

    /// Append a step to this plan.
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::PlanStepOverflow` when the number of steps
    /// cannot be represented as `u32`.
    pub fn with_step(
        self,
        description: String,
        status: ActionStatus,
    ) -> Result<Self, OutputLineError> {
        let order =
            u32::try_from(self.steps.len()).map_err(|_| OutputLineError::PlanStepOverflow)?;
        Ok(Self {
            steps: self
                .steps
                .into_iter()
                .chain(std::iter::once(PlanStep {
                    order,
                    description,
                    status,
                }))
                .collect(),
            ..self
        })
    }
}
