//! Session repository extensions
//!
//! Provides extension trait for `SessionRepository` to support filtering and querying.
//!
//! # Architecture
//!
//! This module is pure **calculations** tier (no I/O):
//! - `SessionRepositoryExt` - extension trait with query methods

use crate::domain::repository::{Session, SessionRepository};

use super::session_filter::SessionFilter;
use super::session_query_ops::{apply_query, filter_sessions};
use super::session_query_types::SessionQuery;

// ============================================================================
// REPOSITORY EXTENSIONS
// ============================================================================

/// Extension trait for SessionRepository to support filtering
pub trait SessionRepositoryExt {
    /// List sessions with a filter
    fn list_filtered(&self, filter: &SessionFilter) -> Vec<Session>;

    /// List sessions with a complete query
    fn query(&self, query: &SessionQuery) -> Vec<Session>;
}

impl<R: SessionRepository> SessionRepositoryExt for R {
    fn list_filtered(&self, filter: &SessionFilter) -> Vec<Session> {
        self.list_all()
            .map(|sessions| filter_sessions(&sessions, filter))
            .unwrap_or_default()
    }

    fn query(&self, query: &SessionQuery) -> Vec<Session> {
        self.list_all()
            .map(|sessions| apply_query(&sessions, query))
            .unwrap_or_default()
    }
}
