//! Action output types
//!
//! Provides action execution reporting for the AI control plane.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::output_jsonl::domain_types::{ActionResult, ActionTarget, ActionVerb};
use crate::output_jsonl::plan::ActionStatus;

/// Action output line for reporting action execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Action {
    pub verb: ActionVerb,
    pub target: ActionTarget,
    pub status: ActionStatus,
    #[serde(skip_serializing_if = "ActionResult::is_pending")]
    #[serde(default = "ActionResult::pending")]
    pub result: ActionResult,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub timestamp: DateTime<Utc>,
}

impl Action {
    #[must_use]
    pub fn new(verb: ActionVerb, target: ActionTarget, status: ActionStatus) -> Self {
        Self {
            verb,
            target,
            status,
            result: ActionResult::Pending,
            timestamp: Utc::now(),
        }
    }

    #[must_use]
    pub fn with_result(self, result: String) -> Self {
        Self {
            result: ActionResult::Completed { result },
            ..self
        }
    }
}
