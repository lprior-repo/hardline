// //! Conflict detail builder
//!
//! Builder for `ConflictDetail` with fluent API.

use crate::output_jsonl::{
    ConflictDetail, ConflictType as OutputConflictType,
    ResolutionStrategy as OutputResolutionStrategy,
};

/// Builder for `ConflictDetail` with fluent API
///
/// # Required Fields
/// - `file`: Conflicted file path
///
/// # Optional Fields
/// - `conflict_type`: Type of conflict (defaults to Overlapping)
/// - `recommended`: Recommended resolution strategy
#[derive(Debug, Clone)]
pub struct ConflictDetailBuilder {
    // Required fields
    file: Option<String>,

    // Optional fields
    conflict_type: Option<ConflictType>,
    workspace_additions: Option<u32>,
    workspace_deletions: Option<u32>,
    main_additions: Option<u32>,
    main_deletions: Option<u32>,
    recommended: Option<ResolutionStrategy>,
}

/// Conflict type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictType {
    Overlapping,
    Existing,
    DeleteModify,
    RenameModify,
    Binary,
}

/// Resolution strategy enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionStrategy {
    AcceptOurs,
    AcceptTheirs,
    JjResolve,
    ManualMerge,
    Rebase,
    Abort,
    Skip,
}

impl Default for ConflictDetailBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ConflictDetailBuilder {
    /// Create a new builder with no fields set
    #[must_use]
    pub const fn new() -> Self {
        Self {
            file: None,
            conflict_type: None,
            workspace_additions: None,
            workspace_deletions: None,
            main_additions: None,
            main_deletions: None,
            recommended: None,
        }
    }

    /// Set the conflicted file (required)
    #[must_use]
    pub fn file(mut self, file: String) -> Self {
        self.file = Some(file);
        self
    }

    /// Set the conflict type (optional)
    #[must_use]
    pub const fn conflict_type(mut self, conflict_type: ConflictType) -> Self {
        self.conflict_type = Some(conflict_type);
        self
    }

    /// Set workspace additions count (optional)
    #[must_use]
    pub const fn workspace_additions(mut self, count: u32) -> Self {
        self.workspace_additions = Some(count);
        self
    }

    /// Set workspace deletions count (optional)
    #[must_use]
    pub const fn workspace_deletions(mut self, count: u32) -> Self {
        self.workspace_deletions = Some(count);
        self
    }

    /// Set main additions count (optional)
    #[must_use]
    pub const fn main_additions(mut self, count: u32) -> Self {
        self.main_additions = Some(count);
        self
    }

    /// Set main deletions count (optional)
    #[must_use]
    pub const fn main_deletions(mut self, count: u32) -> Self {
        self.main_deletions = Some(count);
        self
    }

    /// Set the recommended resolution strategy (optional)
    #[must_use]
    pub const fn recommended(mut self, strategy: ResolutionStrategy) -> Self {
        self.recommended = Some(strategy);
        self
    }

    /// Build the `ConflictDetail`
    ///
    /// # Errors
    ///
    /// Returns `BuilderError::MissingRequired` if any required field is not set.
    pub fn build(self) -> Result<ConflictDetail, super::errors::BuilderError> {
        let file = self
            .file
            .ok_or(super::errors::BuilderError::MissingRequired { field: "file" })?;

        let conflict_type = self.conflict_type.unwrap_or(ConflictType::Overlapping);
        let recommended = self.recommended.unwrap_or(ResolutionStrategy::JjResolve);

        Ok(ConflictDetail {
            file,
            conflict_type: convert_conflict_type(conflict_type),
            workspace_additions: self.workspace_additions,
            workspace_deletions: self.workspace_deletions,
            main_additions: self.main_additions,
            main_deletions: self.main_deletions,
            resolutions: vec![],
            recommended: convert_resolution_strategy(recommended),
        })
    }
}

const fn convert_conflict_type(ty: ConflictType) -> OutputConflictType {
    match ty {
        ConflictType::Overlapping => OutputConflictType::Overlapping,
        ConflictType::Existing => OutputConflictType::Existing,
        ConflictType::DeleteModify => OutputConflictType::DeleteModify,
        ConflictType::RenameModify => OutputConflictType::RenameModify,
        ConflictType::Binary => OutputConflictType::Binary,
    }
}

const fn convert_resolution_strategy(strategy: ResolutionStrategy) -> OutputResolutionStrategy {
    match strategy {
        ResolutionStrategy::AcceptOurs => OutputResolutionStrategy::AcceptOurs,
        ResolutionStrategy::AcceptTheirs => OutputResolutionStrategy::AcceptTheirs,
        ResolutionStrategy::JjResolve => OutputResolutionStrategy::JjResolve,
        ResolutionStrategy::ManualMerge => OutputResolutionStrategy::ManualMerge,
        ResolutionStrategy::Rebase => OutputResolutionStrategy::Rebase,
        ResolutionStrategy::Abort => OutputResolutionStrategy::Abort,
        ResolutionStrategy::Skip => OutputResolutionStrategy::Skip,
    }
}
