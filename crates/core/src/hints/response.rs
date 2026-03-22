//! Hints response structures
//!
//! Complete response types for hints API

use serde::{Deserialize, Serialize};

/// Complete hints response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HintsResponse {
    /// Current system context
    pub context: SystemContext,

    /// Generated hints
    pub hints: Vec<super::types::Hint>,

    /// Suggested next actions
    pub next_actions: Vec<super::types::NextAction>,
}

/// System context summary
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SystemContext {
    /// Is scp initialized?
    pub initialized: bool,

    /// Is this a JJ repository?
    pub jj_repo: bool,

    /// Total number of sessions
    pub sessions_count: usize,

    /// Number of active sessions
    pub active_sessions: usize,

    /// Are there uncommitted changes?
    pub has_changes: bool,
}
