//! Data types for the export_import command handler (Tier 1).
//!
//! Inert, serializable types with no business logic.

use serde::{Deserialize, Serialize};

// ============================================================================
// Export Types
// ============================================================================

/// Options for the export command (parsed from CLI).
#[derive(Debug, Clone)]
pub struct ExportOptions {
    /// Session to export (or all if None).
    pub session: Option<String>,
    /// Output file path (stdout if None).
    pub output: Option<String>,
}

/// Exported session data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedSession {
    /// Session name.
    pub name: String,
    /// Session status.
    pub status: String,
    /// Workspace path (relative).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_path: Option<String>,
    /// Created timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    /// Additional metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Export result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
    /// Export version.
    pub version: String,
    /// Export timestamp.
    pub exported_at: String,
    /// Exported sessions.
    pub sessions: Vec<ExportedSession>,
    /// Number of sessions exported.
    pub count: usize,
}

// ============================================================================
// Import Types
// ============================================================================

/// Options for the import command (parsed from CLI).
#[derive(Debug, Clone)]
pub struct ImportOptions {
    /// Input file path.
    pub input: String,
    /// Overwrite existing sessions.
    pub force: bool,
    /// Skip existing sessions instead of erroring.
    pub skip_existing: bool,
    /// Dry-run mode.
    pub dry_run: bool,
}

/// Import result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    /// Whether import succeeded overall.
    pub success: bool,
    /// Number of sessions imported.
    pub imported: usize,
    /// Number of sessions skipped.
    pub skipped: usize,
    /// Number of sessions overwritten.
    pub overwritten: usize,
    /// Number of sessions that failed to import.
    pub failed: usize,
    /// Whether this was a dry-run.
    pub dry_run: bool,
    /// Error messages.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub errors: Vec<String>,
    /// Names of imported sessions.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub imported_sessions: Vec<String>,
    /// Names of skipped sessions.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub skipped_sessions: Vec<String>,
    /// Names of overwritten sessions.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub overwritten_sessions: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_result_serialization() {
        let result = ExportResult {
            version: "1.0".to_string(),
            exported_at: "2025-01-01T00:00:00Z".to_string(),
            count: 2,
            sessions: vec![],
        };
        let json = serde_json::to_string(&result).expect("serialize");
        assert!(json.contains("\"version\":\"1.0\""));
        assert!(json.contains("\"count\":2"));
    }

    #[test]
    fn exported_session_serialization() {
        let session = ExportedSession {
            name: "test".to_string(),
            status: "active".to_string(),
            workspace_path: Some("/path".to_string()),
            created_at: None,
            metadata: None,
        };
        let json = serde_json::to_string(&session).expect("serialize");
        assert!(json.contains("\"name\":\"test\""));
    }

    #[test]
    fn import_result_serialization() {
        let result = ImportResult {
            success: true,
            imported: 2,
            skipped: 1,
            overwritten: 0,
            failed: 0,
            dry_run: false,
            errors: vec![],
            imported_sessions: vec!["s1".to_string(), "s2".to_string()],
            skipped_sessions: vec!["s3".to_string()],
            overwritten_sessions: vec![],
        };
        let json = serde_json::to_string(&result).expect("serialize");
        assert!(json.contains("\"imported\":2"));
        assert!(json.contains("\"skipped\":1"));
    }

    #[test]
    fn export_import_roundtrip() {
        let original = ExportResult {
            version: "1.0".to_string(),
            exported_at: "2025-01-01T00:00:00Z".to_string(),
            count: 1,
            sessions: vec![ExportedSession {
                name: "test".to_string(),
                status: "active".to_string(),
                workspace_path: None,
                created_at: None,
                metadata: None,
            }],
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let parsed: ExportResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.count, original.count);
        assert_eq!(parsed.sessions.len(), original.sessions.len());
    }
}
