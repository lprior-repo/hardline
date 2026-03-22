//! Output line enum and conflict analysis helper
//!
//! Top-level output types for the AI control plane.

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::output_jsonl::conflict::{ConflictAnalysis, ConflictDetail, ConflictType};

/// Top-level output line enum encompassing all possible output types.
///
/// Each variant corresponds to a different type of output line that can be
/// emitted as JSONL. The `kind()` method returns the type field name for
/// serialization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutputLine {
    Summary(crate::output_jsonl::summary::Summary),
    Session(crate::output_jsonl::session::SessionOutput),
    Issue(crate::output_jsonl::issue::Issue),
    Plan(crate::output_jsonl::plan::Plan),
    Action(crate::output_jsonl::action::Action),
    Warning(crate::output_jsonl::warning::Warning),
    Result(crate::output_jsonl::result::ResultOutput),
    ConflictDetail(ConflictDetail),
    ConflictAnalysis(ConflictAnalysis),
}

impl OutputLine {
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Summary(_) => "summary",
            Self::Session(_) => "session",
            Self::Issue(_) => "issue",
            Self::Plan(_) => "plan",
            Self::Action(_) => "action",
            Self::Warning(_) => "warning",
            Self::Result(_) => "result",
            Self::ConflictDetail(_) => "conflictdetail",
            Self::ConflictAnalysis(_) => "conflict_analysis",
        }
    }

    #[must_use]
    pub fn conflict_analysis(
        session: &str,
        merge_safe: bool,
        conflicts: Vec<ConflictDetail>,
    ) -> Self {
        let existing_conflicts = conflicts
            .iter()
            .filter(|c| c.conflict_type == ConflictType::Existing)
            .count();
        let overlapping_files = conflicts
            .iter()
            .filter(|c| c.conflict_type == ConflictType::Overlapping)
            .count();

        Self::ConflictAnalysis(ConflictAnalysis {
            type_field: "conflictdetail".to_string(),
            session: session.to_string(),
            merge_safe,
            total_conflicts: conflicts.len(),
            conflicts,
            existing_conflicts,
            overlapping_files,
            merge_base: None,
            analysis_time_ms: None,
            timestamp: Utc::now(),
        })
    }
}
