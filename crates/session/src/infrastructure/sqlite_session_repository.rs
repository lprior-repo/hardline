use async_trait::async_trait;
use chrono::{DateTime, Utc};
use scp_core::infrastructure::database::{DatabaseService, SqliteDatabaseService};

use crate::{
    domain::{
        entities::{session::Created, BranchState, Session, SessionId, SessionParts, SessionState},
        value_objects::{BeadId, SessionName, WorkspaceId},
    },
    error::{Result, SessionError, SessionError::*},
    infrastructure::repository::SessionRepository,
};

#[derive(Debug, Clone)]
pub struct SessionRow {
    pub id: String,
    pub name: String,
    pub workspace: Option<String>,
    pub bead: Option<String>,
    pub branch_state: String,
    pub branch_name: Option<String>,
    pub session_state: String,
    pub last_synced: Option<String>,
    pub created_at: String,
}

impl TryFrom<SessionRow> for Session<Created> {
    type Error = SessionError;

    fn try_from(row: SessionRow) -> Result<Self> {
        let id = SessionId::parse(&row.id).map_err(|e| InvalidIdentifier(e.to_string()))?;
        let name = SessionName::parse(&row.name).map_err(|e| InvalidIdentifier(e.to_string()))?;
        let workspace = row
            .workspace
            .map(WorkspaceId::parse)
            .transpose()
            .map_err(|e| InvalidIdentifier(e.to_string()))?;
        let bead = row
            .bead
            .map(BeadId::parse)
            .transpose()
            .map_err(|e| InvalidIdentifier(e.to_string()))?;
        let branch_name = row.branch_name.clone();
        let branch = match (row.branch_state.as_str(), branch_name) {
            ("Detached", None) => BranchState::Detached,
            ("OnBranch", Some(name)) => BranchState::OnBranch { name },
            _ => {
                return Err(RepositoryError(format!(
                    "Invalid branch state combination: {} / {:?}",
                    row.branch_state, row.branch_name
                )))
            }
        };
        let _state = parse_session_state(&row.session_state)?;
        let last_synced = row
            .last_synced
            .as_deref()
            .map(|s| {
                DateTime::parse_from_rfc3339(s)
                    .map_err(|e| {
                        SerializationError(format!("Invalid last_synced timestamp: {}", e))
                    })
                    .map(|dt| dt.with_timezone(&Utc))
            })
            .transpose()?;
        let created_at = DateTime::parse_from_rfc3339(&row.created_at)
            .map_err(|e| SerializationError(format!("Invalid created_at timestamp: {}", e)))?
            .with_timezone(&Utc);

        Ok(Self::from_parts(SessionParts {
            id,
            name,
            workspace,
            bead,
            branch,
            last_synced,
            created_at,
        }))
    }
}

impl From<&Session> for SessionRow {
    fn from(session: &Session) -> Self {
        let (branch_state, branch_name) = match &session.branch {
            BranchState::Detached => ("Detached".to_string(), None),
            BranchState::OnBranch { name } => ("OnBranch".to_string(), Some(name.clone())),
        };
        Self {
            id: session.id.as_str().to_string(),
            name: session.name.as_str().to_string(),
            workspace: session.workspace.as_ref().map(|w| w.as_str().to_string()),
            bead: session.bead.as_ref().map(|b| b.as_str().to_string()),
            branch_state,
            branch_name,
            session_state: format!("{:?}", session.state()),
            last_synced: session.last_synced.map(|dt| dt.to_rfc3339()),
            created_at: session.created_at.to_rfc3339(),
        }
    }
}

fn parse_session_state(s: &str) -> Result<SessionState> {
    match s {
        "Created" => Ok(SessionState::Created),
        "Active" => Ok(SessionState::Active),
        "CommittingEffect" => Ok(SessionState::CommittingEffect),
        "Syncing" => Ok(SessionState::Syncing),
        "Synced" => Ok(SessionState::Synced),
        "Paused" => Ok(SessionState::Paused),
        "Completed" => Ok(SessionState::Completed),
        "Failed" => Ok(SessionState::Failed),
        _ => Err(RepositoryError(format!("Unknown session state: {}", s))),
    }
}

pub struct SqliteSessionRepository {
    db: SqliteDatabaseService,
}

impl SqliteSessionRepository {
    pub const fn new(db: SqliteDatabaseService) -> Self {
        Self { db }
    }

    pub async fn init_schema(&self) -> Result<()> {
        self.db
            .execute(
                r#"
                CREATE TABLE IF NOT EXISTS sessions (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    workspace TEXT,
                    bead TEXT,
                    branch_state TEXT NOT NULL,
                    branch_name TEXT,
                    session_state TEXT NOT NULL,
                    last_synced TEXT,
                    created_at TEXT NOT NULL
                )
                "#,
            )
            .await
            .map_err(|e| DatabaseError(e.to_string()))?;

        self.db
            .execute("CREATE INDEX IF NOT EXISTS idx_sessions_name ON sessions(name)")
            .await
            .map_err(|e| DatabaseError(e.to_string()))?;

        Ok(())
    }
}

fn escape_sql_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "''")
}

/// Validates that a string is safe to use in a single-quoted SQL literal.
/// Rejects null bytes which can cause truncation or unexpected behavior in SQL engines.
fn validate_sql_string(s: &str) -> std::result::Result<(), String> {
    if s.contains('\0') {
        return Err(format!(
            "String contains null byte which is invalid in SQL string literals: {:?}",
            s
        ));
    }
    Ok(())
}

#[async_trait]
impl SessionRepository for SqliteSessionRepository {
    async fn save(&self, session: &Session) -> Result<()> {
        let row = SessionRow::from(session);

        // Validate all fields before building SQL
        validate_sql_string(&row.id).map_err(RepositoryError)?;
        validate_sql_string(&row.name).map_err(RepositoryError)?;
        validate_sql_string(&row.branch_state).map_err(RepositoryError)?;
        validate_sql_string(&row.session_state).map_err(RepositoryError)?;
        validate_sql_string(&row.created_at).map_err(RepositoryError)?;
        if let Some(ref w) = row.workspace {
            validate_sql_string(w).map_err(RepositoryError)?;
        }
        if let Some(ref b) = row.bead {
            validate_sql_string(b).map_err(RepositoryError)?;
        }
        if let Some(ref b) = row.branch_name {
            validate_sql_string(b).map_err(RepositoryError)?;
        }
        if let Some(ref s) = row.last_synced {
            validate_sql_string(s).map_err(RepositoryError)?;
        }

        let workspace = row.workspace.as_deref().map(escape_sql_string);
        let bead = row.bead.as_deref().map(escape_sql_string);
        let branch_name = row.branch_name.as_deref().map(escape_sql_string);
        let last_synced = row.last_synced.as_deref().map(escape_sql_string);

        let query = format!(
            r#"INSERT INTO sessions (id, name, workspace, bead, branch_state, branch_name, session_state, last_synced, created_at)
VALUES ('{}', '{}', {}, {}, '{}', {}, '{}', {}, '{}')
ON CONFLICT(id) DO UPDATE SET
    name = excluded.name,
    workspace = excluded.workspace,
    bead = excluded.bead,
    branch_state = excluded.branch_state,
    branch_name = excluded.branch_name,
    session_state = excluded.session_state,
    last_synced = excluded.last_synced,
    created_at = excluded.created_at"#,
            escape_sql_string(&row.id),
            escape_sql_string(&row.name),
            workspace
                .map(|w| format!("'{}'", w))
                .unwrap_or_else(|| "NULL".to_string()),
            bead.map(|b| format!("'{}'", b))
                .unwrap_or_else(|| "NULL".to_string()),
            escape_sql_string(&row.branch_state),
            branch_name
                .map(|b| format!("'{}'", b))
                .unwrap_or_else(|| "NULL".to_string()),
            escape_sql_string(&row.session_state),
            last_synced
                .map(|s| format!("'{}'", s))
                .unwrap_or_else(|| "NULL".to_string()),
            escape_sql_string(&row.created_at),
        );

        self.db
            .execute(&query)
            .await
            .map_err(|e| DatabaseError(format!("Failed to save session: {}", e)))?;
        Ok(())
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<Session>> {
        if id.is_empty() {
            return Err(InvalidIdentifier("SessionId cannot be empty".to_string()));
        }
        let _ = SessionId::parse(id).map_err(|e| InvalidIdentifier(e.to_string()))?;
        validate_sql_string(id).map_err(RepositoryError)?;

        let escaped_id = escape_sql_string(id);
        let results = self
            .db
            .query(&format!(
                "SELECT id, name, workspace, bead, branch_state, branch_name, session_state, last_synced, created_at
                 FROM sessions WHERE id = '{}'",
                escaped_id
            ))
            .await
            .map_err(|e| DatabaseError(format!("Failed to find session: {}", e)))?;

        if results.is_empty() {
            return Ok(None);
        }

        let row = self.row_to_session_row(results)?;
        let session = Session::try_from(row)?;
        Ok(Some(session))
    }

    async fn find_by_name(&self, name: &SessionName) -> Result<Option<Session>> {
        validate_sql_string(name.as_str()).map_err(RepositoryError)?;
        let escaped_name = escape_sql_string(name.as_str());
        let results = self
            .db
            .query(&format!(
                "SELECT id, name, workspace, bead, branch_state, branch_name, session_state, last_synced, created_at
                 FROM sessions WHERE name = '{}'",
                escaped_name
            ))
            .await
            .map_err(|e| DatabaseError(format!("Failed to find session by name: {}", e)))?;

        if results.is_empty() {
            return Ok(None);
        }

        let row = self.row_to_session_row(results)?;
        let session = Session::try_from(row)?;
        Ok(Some(session))
    }

    async fn list(&self) -> Result<Vec<Session>> {
        let results = self
            .db
            .query(
                "SELECT id, name, workspace, bead, branch_state, branch_name, session_state, last_synced, created_at
                 FROM sessions ORDER BY created_at DESC",
            )
            .await
            .map_err(|e| DatabaseError(format!("Failed to list sessions: {}", e)))?;

        let mut sessions = Vec::new();
        for result in results {
            let row = self.row_to_session_row(vec![result])?;
            let session = Session::try_from(row)?;
            sessions.push(session);
        }
        Ok(sessions)
    }

    async fn delete(&self, id: &str) -> Result<()> {
        if id.is_empty() {
            return Err(InvalidIdentifier("SessionId cannot be empty".to_string()));
        }
        let _ = SessionId::parse(id).map_err(|e| InvalidIdentifier(e.to_string()))?;
        validate_sql_string(id).map_err(RepositoryError)?;

        let escaped_id = escape_sql_string(id);
        let results = self
            .db
            .query(&format!(
                "SELECT id FROM sessions WHERE id = '{}'",
                escaped_id
            ))
            .await
            .map_err(|e| DatabaseError(format!("Failed to delete session: {}", e)))?;

        if results.is_empty() {
            return Err(NotFound(format!("Session not found: {}", id)));
        }

        self.db
            .execute(&format!("DELETE FROM sessions WHERE id = '{}'", escaped_id))
            .await
            .map_err(|e| DatabaseError(format!("Failed to delete session: {}", e)))?;
        Ok(())
    }
}

impl SqliteSessionRepository {
    fn row_to_session_row(&self, result: Vec<Vec<String>>) -> Result<SessionRow> {
        if result.len() != 1 || result[0].len() != 9 {
            return Err(RepositoryError(format!(
                "Expected single row with 9 columns, got {} rows with {} columns",
                result.len(),
                if result.is_empty() {
                    0
                } else {
                    result[0].len()
                }
            )));
        }

        let cols = &result[0];
        Ok(SessionRow {
            id: cols[0].clone(),
            name: cols[1].clone(),
            workspace: cols[2]
                .is_empty()
                .then_some(cols[2].clone())
                .filter(|s| !s.is_empty()),
            bead: cols[3]
                .is_empty()
                .then_some(cols[3].clone())
                .filter(|s| !s.is_empty()),
            branch_state: cols[4].clone(),
            branch_name: cols[5]
                .is_empty()
                .then_some(cols[5].clone())
                .filter(|s| !s.is_empty()),
            session_state: cols[6].clone(),
            last_synced: cols[7]
                .is_empty()
                .then_some(cols[7].clone())
                .filter(|s| !s.is_empty()),
            created_at: cols[8].clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // escape_sql_string Tests
    // =========================================================================

    mod escape_sql_tests {
        use super::*;

        #[test]
        fn escape_sql_string_no_quotes() {
            assert_eq!(escape_sql_string("hello world"), "hello world");
        }

        #[test]
        fn escape_sql_string_single_quote_escaped() {
            assert_eq!(escape_sql_string("it's"), "it''s");
        }

        #[test]
        fn escape_sql_string_multiple_quotes() {
            assert_eq!(escape_sql_string("a'b'c"), "a''b''c");
        }

        #[test]
        fn escape_sql_string_empty() {
            assert_eq!(escape_sql_string(""), "");
        }

        #[test]
        fn escape_sql_string_only_quotes() {
            assert_eq!(escape_sql_string("'"), "''");
        }

        #[test]
        fn escape_sql_string_backslash_escaped() {
            assert_eq!(escape_sql_string(r"path\to\file"), r"path\\to\\file");
        }

        #[test]
        fn escape_sql_string_backslash_and_quote_combined() {
            assert_eq!(escape_sql_string(r"it's a \path"), r"it''s a \\path");
        }

        #[test]
        fn escape_sql_string_only_backslash() {
            assert_eq!(escape_sql_string(r"\"), r"\\");
        }

        #[test]
        fn escape_sql_string_backslash_quote_injection_attempt() {
            // The classic SQL injection pattern: \' attempts to escape the quote
            assert_eq!(escape_sql_string(r"\' OR '1'='1"), r"\\'' OR ''1''=''1");
        }
    }

    // =========================================================================
    // validate_sql_string Tests
    // =========================================================================

    mod validate_sql_tests {
        use super::*;

        #[test]
        fn validate_sql_string_accepts_normal_string() {
            assert!(validate_sql_string("hello world").is_ok());
        }

        #[test]
        fn validate_sql_string_accepts_empty_string() {
            assert!(validate_sql_string("").is_ok());
        }

        #[test]
        fn validate_sql_string_accepts_quotes() {
            // Quotes are handled by escaping; validation only rejects null bytes
            assert!(validate_sql_string("it's fine").is_ok());
        }

        #[test]
        fn validate_sql_string_rejects_null_byte() {
            assert!(validate_sql_string("hello\0world").is_err());
        }

        #[test]
        fn validate_sql_string_rejects_null_at_start() {
            assert!(validate_sql_string("\0evil").is_err());
        }

        #[test]
        fn validate_sql_string_rejects_null_at_end() {
            assert!(validate_sql_string("evil\0").is_err());
        }

        #[test]
        fn validate_sql_string_rejects_only_null() {
            assert!(validate_sql_string("\0").is_err());
        }

        #[test]
        fn validate_sql_string_rejects_embedded_null() {
            assert!(validate_sql_string("normal\0payload").is_err());
        }
    }

    // =========================================================================
    // parse_session_state Tests
    // =========================================================================

    mod parse_session_state_tests {
        use super::*;

        #[test]
        fn parse_state_created() {
            assert_eq!(
                parse_session_state("Created").expect("valid"),
                SessionState::Created
            );
        }

        #[test]
        fn parse_state_active() {
            assert_eq!(
                parse_session_state("Active").expect("valid"),
                SessionState::Active
            );
        }

        #[test]
        fn parse_state_syncing() {
            assert_eq!(
                parse_session_state("Syncing").expect("valid"),
                SessionState::Syncing
            );
        }

        #[test]
        fn parse_state_synced() {
            assert_eq!(
                parse_session_state("Synced").expect("valid"),
                SessionState::Synced
            );
        }

        #[test]
        fn parse_state_paused() {
            assert_eq!(
                parse_session_state("Paused").expect("valid"),
                SessionState::Paused
            );
        }

        #[test]
        fn parse_state_completed() {
            assert_eq!(
                parse_session_state("Completed").expect("valid"),
                SessionState::Completed
            );
        }

        #[test]
        fn parse_state_failed() {
            assert_eq!(
                parse_session_state("Failed").expect("valid"),
                SessionState::Failed
            );
        }

        #[test]
        fn parse_state_unknown_rejects() {
            let result = parse_session_state("Unknown");
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                SessionError::RepositoryError(_)
            ));
        }

        #[test]
        fn parse_state_empty_rejects() {
            let result = parse_session_state("");
            assert!(result.is_err());
        }

        #[test]
        fn parse_state_case_sensitive() {
            let result = parse_session_state("created");
            assert!(result.is_err());
        }
    }

    // =========================================================================
    // SessionRow Tests
    // =========================================================================

    mod session_row_tests {
        use super::*;

        fn make_detached_row() -> SessionRow {
            SessionRow {
                id: "session-test-1".to_string(),
                name: "test-session".to_string(),
                workspace: Some("ws-test".to_string()),
                bead: Some("bd-abc123".to_string()),
                branch_state: "Detached".to_string(),
                branch_name: None,
                session_state: "Created".to_string(),
                last_synced: None,
                created_at: "2024-01-01T00:00:00+00:00".to_string(),
            }
        }

        #[test]
        fn session_row_try_from_detached_valid() {
            let row = make_detached_row();
            let session = Session::<Created>::try_from(row).expect("valid conversion");
            assert_eq!(session.id.as_str(), "session-test-1");
            assert_eq!(session.name.as_str(), "test-session");
        }

        #[test]
        fn session_row_try_from_on_branch_valid() {
            let row = SessionRow {
                branch_state: "OnBranch".to_string(),
                branch_name: Some("feature-1".to_string()),
                ..make_detached_row()
            };
            let session = Session::<Created>::try_from(row).expect("valid");
            assert_eq!(session.branch.branch_name(), Some("feature-1"));
        }

        #[test]
        fn session_row_try_from_invalid_branch_combination_rejects() {
            let row = SessionRow {
                branch_state: "Detached".to_string(),
                branch_name: Some("should-be-none".to_string()),
                ..make_detached_row()
            };
            let result = Session::<Created>::try_from(row);
            assert!(result.is_err());
        }

        #[test]
        fn session_row_try_from_on_branch_without_name_rejects() {
            let row = SessionRow {
                branch_state: "OnBranch".to_string(),
                branch_name: None,
                ..make_detached_row()
            };
            let result = Session::<Created>::try_from(row);
            assert!(result.is_err());
        }

        #[test]
        fn session_row_try_from_invalid_id_rejects() {
            let row = SessionRow {
                id: "".to_string(),
                ..make_detached_row()
            };
            let result = Session::<Created>::try_from(row);
            assert!(result.is_err());
        }

        #[test]
        fn session_row_try_from_invalid_name_rejects() {
            let row = SessionRow {
                name: "123invalid-start".to_string(),
                ..make_detached_row()
            };
            let result = Session::<Created>::try_from(row);
            assert!(result.is_err());
        }

        #[test]
        fn session_row_from_session_roundtrip() {
            let name = SessionName::parse("roundtrip").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let row = SessionRow::from(&session);
            assert_eq!(row.id, session.id.as_str());
            assert_eq!(row.name, session.name.as_str());
            assert_eq!(row.branch_state, "Detached");
            assert!(row.branch_name.is_none());
        }

        #[test]
        fn session_row_from_session_on_branch() {
            let name = SessionName::parse("branch-test").expect("valid");
            let session = Session::<Created>::create(name).expect("created");
            let branched = session
                .transition_branch(BranchState::OnBranch { name: "dev".into() })
                .expect("branch");
            let row = SessionRow::from(&branched);
            assert_eq!(row.branch_state, "OnBranch");
            assert_eq!(row.branch_name, Some("dev".to_string()));
        }
    }
}
