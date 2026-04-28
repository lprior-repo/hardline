// //! Plan builder
//! Builder for `Plan` with step collection.

use chrono::{DateTime, Utc};

use crate::output_jsonl::{
    domain_types::{PlanDescription, PlanTitle},
    ActionStatus, Plan, PlanStep,
};

/// Builder for [Plan] with step collection
///
/// # Required Fields
/// - `title`: Plan title
/// - `description`: Plan description
///
/// # Optional Fields
/// - `steps`: Plan steps (can be added incrementally)
/// - `created_at`: Creation timestamp (defaults to now)
#[derive(Debug, Clone)]
pub struct PlanBuilder {
    // Required fields
    title: Option<PlanTitle>,
    description: Option<PlanDescription>,

    // Optional fields
    steps: Vec<PlanStepData>,
    created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
struct PlanStepData {
    description: String,
    status: ActionStatus,
}

impl Default for PlanBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl PlanBuilder {
    /// Create a new builder with no fields set
    #[must_use]
    pub const fn new() -> Self {
        Self {
            title: None,
            description: None,
            steps: Vec::new(),
            created_at: None,
        }
    }

    /// Set the plan title (required)
    #[must_use]
    pub fn title(mut self, title: PlanTitle) -> Self {
        self.title = Some(title);
        self
    }

    /// Set the plan description (required)
    #[must_use]
    pub fn description(mut self, description: PlanDescription) -> Self {
        self.description = Some(description);
        self
    }

    /// Add a step to the plan
    ///
    /// # Errors
    ///
    /// Returns `BuilderError::Overflow` if the step count exceeds `u32::MAX`.
    pub fn with_step(
        mut self,
        description: impl Into<String>,
        status: ActionStatus,
    ) -> Result<Self, super::errors::BuilderError> {
        let _order =
            u32::try_from(self.steps.len()).map_err(|_| super::errors::BuilderError::Overflow {
                field: "steps",
                capacity: u32::MAX as usize,
            })?;

        self.steps.push(PlanStepData {
            description: description.into(),
            status,
        });
        Ok(self)
    }

    /// Set the creation timestamp (optional)
    #[must_use]
    pub const fn created_at(mut self, created_at: DateTime<Utc>) -> Self {
        self.created_at = Some(created_at);
        self
    }

    /// Build the Plan
    ///
    /// # Errors
    ///
    /// Returns `BuilderError::MissingRequired` if any required field is not set.
    pub fn build(self) -> Result<Plan, super::errors::BuilderError> {
        let title = self
            .title
            .ok_or(super::errors::BuilderError::MissingRequired { field: "title" })?;
        let description = self
            .description
            .ok_or(super::errors::BuilderError::MissingRequired {
                field: "description",
            })?;

        let steps = self
            .steps
            .into_iter()
            .enumerate()
            .map(|(order, step)| {
                let order_u32 =
                    u32::try_from(order).map_err(|_| super::errors::BuilderError::Overflow {
                        field: "steps",
                        capacity: u32::MAX as usize,
                    })?;
                Ok(PlanStep {
                    order: order_u32,
                    description: step.description,
                    status: step.status,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Plan {
            title,
            description,
            steps,
            created_at: self.created_at.unwrap_or_else(Utc::now),
        })
    }
}
