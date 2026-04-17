//! Session filter value object
//!
//! Provides filter criteria for session queries.
//!
//! # Architecture
//!
//! This module is pure **calculations** tier (no I/O):
//! - `SessionFilter` - value object for filter criteria
//! - `matches()` - pure predicate function

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::domain::repository::Session;
use crate::session_state::SessionState;

// ============================================================================
// SESSION FILTER VALUE OBJECT
// ============================================================================

/// Filter criteria for session queries
///
/// A value object that encapsulates all filterable criteria.
/// All fields are optional - None means "don't filter by this criteria".
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionFilter {
    /// Filter by session status (active, paused, completed, failed, etc.)
    /// Note: Currently simplified to check if session is "active" (has branch + valid workspace)
    #[serde(default)]
    pub status: Option<SessionState>,
    /// Filter by branch name (substring match)
    #[serde(default)]
    pub branch: Option<String>,
    /// Filter by session name (substring match, case-insensitive)
    #[serde(default)]
    pub name_contains: Option<String>,
    /// Filter by workspace path prefix
    #[serde(default)]
    pub workspace_prefix: Option<PathBuf>,
    /// Only include sessions with valid workspace paths
    #[serde(default)]
    pub valid_workspace_only: bool,
    /// Only include detached sessions
    #[serde(default)]
    pub detached_only: bool,
    /// Only include sessions on a branch (not detached)
    #[serde(default)]
    pub on_branch_only: bool,
}

impl SessionFilter {
    /// Create a new empty filter (matches everything)
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by session status
    #[must_use]
    pub const fn with_status(mut self, status: SessionState) -> Self {
        self.status = Some(status);
        self
    }

    /// Filter by branch name
    #[must_use]
    pub fn with_branch(mut self, branch: impl Into<String>) -> Self {
        self.branch = Some(branch.into());
        self
    }

    /// Filter by name (case-insensitive substring)
    #[must_use]
    pub fn with_name_contains(mut self, name: impl Into<String>) -> Self {
        self.name_contains = Some(name.into());
        self
    }

    /// Only include sessions with valid workspace paths
    #[must_use]
    pub const fn with_valid_workspace_only(mut self) -> Self {
        self.valid_workspace_only = true;
        self
    }

    /// Only include detached sessions
    #[must_use]
    pub const fn with_detached_only(mut self) -> Self {
        self.detached_only = true;
        self
    }

    /// Only include sessions on a branch
    #[must_use]
    pub const fn with_on_branch_only(mut self) -> Self {
        self.on_branch_only = true;
        self
    }

    /// Check if a session matches this filter
    #[must_use]
    pub fn matches(&self, session: &Session) -> bool {
        // Note: Status filtering is simplified.
        // The session's is_active() represents "currently usable" (has branch + valid workspace).
        // For more complex status filtering, we'd need to map SessionState to this check.

        // Status filter - check if session is active based on filter
        let status_match = self.status.is_none_or(|_| session.is_active());

        // Branch filter (substring match)
        let branch_match = self.branch.as_ref().is_none_or(|branch_pattern| {
            session
                .branch
                .branch_name()
                .is_some_and(|name| name.contains(branch_pattern))
        });

        // Name contains filter (case-insensitive)
        let name_match = self.name_contains.as_ref().is_none_or(|pattern| {
            let pattern_lower = pattern.to_lowercase();
            session
                .name
                .as_str()
                .to_lowercase()
                .contains(&pattern_lower)
        });

        // Workspace prefix filter
        let workspace_match = self
            .workspace_prefix
            .as_ref()
            .is_none_or(|prefix| session.workspace_path.starts_with(prefix));

        // Valid workspace only
        let valid_workspace = !self.valid_workspace_only || session.workspace_path.exists();

        // Detached only
        let detached_match = !self.detached_only || session.branch.is_detached();

        // On branch only
        let on_branch_match = !self.on_branch_only || !session.branch.is_detached();

        status_match
            && branch_match
            && name_match
            && workspace_match
            && valid_workspace
            && detached_match
            && on_branch_match
    }
}
