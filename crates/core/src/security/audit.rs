//! Audit logging per ADR-015.
//!
//! Provides JSONL-based audit logging for security events.
//! All access attempts (success, denied, error) are recorded.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{agent::AgentId, error::Result, error_io::IoErrorKind};

// ========================================================================
// AuditEntry
// ========================================================================

/// A single audit log entry.
///
/// Records who did what, to which resource, and the outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// When the event occurred
    pub timestamp: DateTime<Utc>,
    /// The agent that performed the action
    pub agent_id: AgentId,
    /// The action taken (e.g., "workspace.read", "queue.enqueue")
    pub action: String,
    /// The resource targeted (e.g., "workspace:my-ws", "queue")
    pub resource: String,
    /// The outcome of the action
    pub outcome: AuditOutcome,
}

// ========================================================================
// AuditOutcome
// ========================================================================

/// Outcome of an audited action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditOutcome {
    /// Action succeeded
    Success,
    /// Action was denied
    Denied(String),
    /// Action encountered an error
    Error(String),
}

// ========================================================================
// AuditFilter
// ========================================================================

/// Filter criteria for querying audit entries.
#[derive(Debug, Clone, Default)]
pub struct AuditFilter {
    /// Only entries from this agent
    pub agent_id: Option<AgentId>,
    /// Only entries matching this action prefix
    pub action_prefix: Option<String>,
    /// Only entries matching this outcome
    pub outcome: Option<AuditOutcomeFilter>,
}

/// Simplified outcome filter for queries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditOutcomeFilter {
    /// Only successful entries
    Success,
    /// Only denied entries
    Denied,
    /// Only error entries
    Error,
}

// ========================================================================
// AuditLogger
// ========================================================================

/// Audit logger that writes entries as JSONL to a file.
///
/// Each line in the log file is a JSON object (one `AuditEntry` per line).
pub struct AuditLogger {
    /// Path to the audit log file
    log_path: std::path::PathBuf,
}

impl AuditLogger {
    /// Create a new audit logger that writes to the given path.
    ///
    /// The parent directory must exist.
    #[must_use]
    pub const fn new(log_path: std::path::PathBuf) -> Self {
        Self { log_path }
    }

    /// Log an audit entry by appending it as a JSON line.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened or written.
    pub fn log(&self, entry: &AuditEntry) -> Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .map_err(IoErrorKind::Io)?;

        let line = serde_json::to_string(entry).map_err(|e| {
            IoErrorKind::Io(std::io::Error::other(format!(
                "Audit serialization failed: {e}"
            )))
        })?;

        use std::io::Write;
        writeln!(file, "{line}").map_err(IoErrorKind::Io)?;

        Ok(())
    }

    /// Read all audit entries from the log file and apply a filter.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn query(&self, filter: &AuditFilter) -> Result<Vec<AuditEntry>> {
        let contents = std::fs::read_to_string(&self.log_path).map_err(IoErrorKind::Io)?;

        let mut entries = Vec::new();

        for line in contents.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let entry: AuditEntry = serde_json::from_str(trimmed).map_err(|e| {
                IoErrorKind::Io(std::io::Error::other(format!("Audit parse failed: {e}")))
            })?;

            if matches_filter(&entry, filter) {
                entries.push(entry);
            }
        }

        Ok(entries)
    }
}

/// Check whether an audit entry matches the given filter criteria.
fn matches_filter(entry: &AuditEntry, filter: &AuditFilter) -> bool {
    if let Some(ref agent_id) = filter.agent_id {
        if entry.agent_id != *agent_id {
            return false;
        }
    }

    if let Some(ref prefix) = filter.action_prefix {
        if !entry.action.starts_with(prefix.as_str()) {
            return false;
        }
    }

    if let Some(ref outcome) = filter.outcome {
        let matches = matches!(
            (&entry.outcome, outcome),
            (AuditOutcome::Success, AuditOutcomeFilter::Success)
                | (AuditOutcome::Denied(_), AuditOutcomeFilter::Denied)
                | (AuditOutcome::Error(_), AuditOutcomeFilter::Error)
        );
        if !matches {
            return false;
        }
    }

    true
}

// ========================================================================
// Tests
// ========================================================================

#[cfg(test)]
mod tests {
    use tempfile::NamedTempFile;

    use super::*;

    fn make_entry(agent: &str, action: &str, outcome: AuditOutcome) -> AuditEntry {
        AuditEntry {
            timestamp: Utc::now(),
            agent_id: AgentId::new(agent),
            action: action.to_string(),
            resource: "workspace:test".to_string(),
            outcome,
        }
    }

    #[test]
    fn test_log_and_query_all() {
        let tmp = NamedTempFile::new().expect("temp file");
        let path = tmp.path().to_path_buf();
        let logger = AuditLogger::new(path);

        let entry = make_entry("agent-1", "workspace.read", AuditOutcome::Success);
        logger.log(&entry).expect("should log");

        let results = logger.query(&AuditFilter::default()).expect("should query");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].agent_id.as_str(), "agent-1");
    }

    #[test]
    fn test_log_multiple_entries() {
        let tmp = NamedTempFile::new().expect("temp file");
        let path = tmp.path().to_path_buf();
        let logger = AuditLogger::new(path);

        logger
            .log(&make_entry("a1", "read", AuditOutcome::Success))
            .expect("should log");
        logger
            .log(&make_entry(
                "a2",
                "write",
                AuditOutcome::Denied("no perm".into()),
            ))
            .expect("should log");
        logger
            .log(&make_entry(
                "a3",
                "delete",
                AuditOutcome::Error("io err".into()),
            ))
            .expect("should log");

        let results = logger.query(&AuditFilter::default()).expect("should query");
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_query_filter_by_agent() {
        let tmp = NamedTempFile::new().expect("temp file");
        let path = tmp.path().to_path_buf();
        let logger = AuditLogger::new(path);

        logger
            .log(&make_entry("agent-1", "read", AuditOutcome::Success))
            .expect("should log");
        logger
            .log(&make_entry("agent-2", "read", AuditOutcome::Success))
            .expect("should log");

        let filter = AuditFilter {
            agent_id: Some(AgentId::new("agent-1")),
            ..Default::default()
        };
        let results = logger.query(&filter).expect("should query");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].agent_id.as_str(), "agent-1");
    }

    #[test]
    fn test_query_filter_by_action_prefix() {
        let tmp = NamedTempFile::new().expect("temp file");
        let path = tmp.path().to_path_buf();
        let logger = AuditLogger::new(path);

        logger
            .log(&make_entry("a1", "workspace.read", AuditOutcome::Success))
            .expect("should log");
        logger
            .log(&make_entry("a1", "queue.enqueue", AuditOutcome::Success))
            .expect("should log");

        let filter = AuditFilter {
            action_prefix: Some("workspace".to_string()),
            ..Default::default()
        };
        let results = logger.query(&filter).expect("should query");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].action, "workspace.read");
    }

    #[test]
    fn test_query_filter_by_outcome() {
        let tmp = NamedTempFile::new().expect("temp file");
        let path = tmp.path().to_path_buf();
        let logger = AuditLogger::new(path);

        logger
            .log(&make_entry("a1", "read", AuditOutcome::Success))
            .expect("should log");
        logger
            .log(&make_entry(
                "a1",
                "write",
                AuditOutcome::Denied("nope".into()),
            ))
            .expect("should log");
        logger
            .log(&make_entry(
                "a1",
                "delete",
                AuditOutcome::Error("fail".into()),
            ))
            .expect("should log");

        let denied_filter = AuditFilter {
            outcome: Some(AuditOutcomeFilter::Denied),
            ..Default::default()
        };
        let results = logger.query(&denied_filter).expect("should query");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].action, "write");

        let error_filter = AuditFilter {
            outcome: Some(AuditOutcomeFilter::Error),
            ..Default::default()
        };
        let results = logger.query(&error_filter).expect("should query");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].action, "delete");
    }

    #[test]
    fn test_query_empty_log() {
        let tmp = NamedTempFile::new().expect("temp file");
        let path = tmp.path().to_path_buf();
        let logger = AuditLogger::new(path);

        let results = logger.query(&AuditFilter::default()).expect("should query");
        assert!(results.is_empty());
    }

    #[test]
    fn test_query_skips_empty_lines() {
        let tmp = NamedTempFile::new().expect("temp file");
        let path = tmp.path().to_path_buf();

        // Write a file with an empty line
        std::fs::write(
            &path,
            "\n{\"timestamp\":\"2026-01-01T00:00:00Z\",\"agent_id\":\"a1\",\"action\":\"read\",\"resource\":\"r\",\"outcome\":\"Success\"}\n\n",
        )
        .expect("should write");

        let logger = AuditLogger::new(path);
        let results = logger.query(&AuditFilter::default()).expect("should query");
        assert_eq!(results.len(), 1);
    }
}
