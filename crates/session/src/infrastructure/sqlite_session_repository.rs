use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::domain::entities::session::{Created, StateInfo};
use crate::domain::entities::{BranchState, Session, SessionId, SessionState};
use crate::domain::value_objects::{AgentId, BeadId, SessionName, WorkspaceId};
use crate::error::{Result, SessionError, SessionError::*};
use crate::infrastructure::repository::SessionRepository;
use scp_core::infrastructure::database::{DatabaseService, SqliteDatabaseService};

#[derive(Debug, Clone)]
pub struct SessionRow {
    pub id: String,
    pub name: String,
    pub workspace: Option<String>,
    pub bead: Option<String>,
    pub assigned_agent: Option<String>,
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
        let assigned_agent = row
            .assigned_agent
            .map(AgentId::new)
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

        Ok(Session::from_parts(
            id,
            name,
            workspace,
            bead,
            assigned_agent,
            branch,
            last_synced,
            created_at,
        ))
    }
}

impl<S: StateInfo> From<&Session<S>> for SessionRow {
    fn from(session: &Session<S>) -> Self {
        let (branch_state, branch_name) = match &session.branch {
            BranchState::Detached => ("Detached".to_string(), None),
            BranchState::OnBranch { name } => ("OnBranch".to_string(), Some(name.clone())),
        };
        Self {
            id: session.id.as_str().to_string(),
            name: session.name.as_str().to_string(),
            workspace: session.workspace.as_ref().map(|w| w.as_str().to_string()),
            bead: session.bead.as_ref().map(|b| b.as_str().to_string()),
            assigned_agent: session.assigned_agent.as_ref().map(|a| a.as_str().to_string()),
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
    pub fn new(db: SqliteDatabaseService) -> Self {
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
                    assigned_agent TEXT,
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

        self.db
            .execute("CREATE INDEX IF NOT EXISTS idx_sessions_state ON sessions(session_state)")
            .await
            .map_err(|e| DatabaseError(e.to_string()))?;

        self.db
            .execute("CREATE INDEX IF NOT EXISTS idx_sessions_agent ON sessions(assigned_agent)")
            .await
            .map_err(|e| DatabaseError(e.to_string()))?;

        Ok(())
    }
}

fn escape_sql_string(s: &str) -> String {
    s.replace('\'', "''")
}

#[async_trait]
impl SessionRepository for SqliteSessionRepository {
    async fn save<S: StateInfo + std::marker::Sync>(&self, session: &Session<S>) -> Result<()> {
        let row = SessionRow::from(session);
        let workspace = row.workspace.as_deref().map(escape_sql_string);
        let bead = row.bead.as_deref().map(escape_sql_string);
        let assigned_agent = row.assigned_agent.as_deref().map(escape_sql_string);
        let branch_name = row.branch_name.as_deref().map(escape_sql_string);
        let last_synced = row.last_synced.as_deref().map(escape_sql_string);

        let query = format!(
            r#"INSERT INTO sessions (id, name, workspace, bead, assigned_agent, branch_state, branch_name, session_state, last_synced, created_at)
VALUES ('{}', '{}', {}, {}, {}, '{}', {}, '{}', {}, '{}')
ON CONFLICT(id) DO UPDATE SET
    name = excluded.name,
    workspace = excluded.workspace,
    bead = excluded.bead,
    assigned_agent = excluded.assigned_agent,
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
            assigned_agent
                .map(|a| format!("'{}'", a))
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

        let escaped_id = escape_sql_string(id);
        let results = self
            .db
            .query(&format!(
                "SELECT id, name, workspace, bead, assigned_agent, branch_state, branch_name, session_state, last_synced, created_at
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
        let escaped_name = escape_sql_string(name.as_str());
        let results = self
            .db
            .query(&format!(
                "SELECT id, name, workspace, bead, assigned_agent, branch_state, branch_name, session_state, last_synced, created_at
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
                "SELECT id, name, workspace, bead, assigned_agent, branch_state, branch_name, session_state, last_synced, created_at
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

    async fn find_by_state(&self, state: SessionState) -> Result<Vec<Session>> {
        let state_str = format!("{:?}", state);
        let results = self
            .db
            .query(&format!(
                "SELECT id, name, workspace, bead, assigned_agent, branch_state, branch_name, session_state, last_synced, created_at
                 FROM sessions WHERE session_state = '{}'
                 ORDER BY created_at DESC",
                escape_sql_string(&state_str)
            ))
            .await
            .map_err(|e| DatabaseError(format!("Failed to find sessions by state: {}", e)))?;

        let mut sessions = Vec::new();
        for result in results {
            let row = SessionRow {
                id: result[0].clone(),
                name: result[1].clone(),
                workspace: result[2]
                    .is_empty()
                    .then_some(result[2].clone())
                    .filter(|s| !s.is_empty()),
                bead: result[3]
                    .is_empty()
                    .then_some(result[3].clone())
                    .filter(|s| !s.is_empty()),
                assigned_agent: result[4]
                    .is_empty()
                    .then_some(result[4].clone())
                    .filter(|s| !s.is_empty()),
                branch_state: result[5].clone(),
                branch_name: result[6]
                    .is_empty()
                    .then_some(result[6].clone())
                    .filter(|s| !s.is_empty()),
                session_state: result[7].clone(),
                last_synced: result[8]
                    .is_empty()
                    .then_some(result[8].clone())
                    .filter(|s| !s.is_empty()),
                created_at: result[9].clone(),
            };
            let session = Session::try_from(row)?;
            sessions.push(session);
        }
        Ok(sessions)
    }

    async fn find_by_agent(&self, agent: &AgentId) -> Result<Vec<Session>> {
        let agent_str = agent.as_str();
        let results = self
            .db
            .query(&format!(
                "SELECT id, name, workspace, bead, assigned_agent, branch_state, branch_name, session_state, last_synced, created_at
                 FROM sessions WHERE assigned_agent = '{}'
                 ORDER BY created_at DESC",
                escape_sql_string(agent_str)
            ))
            .await
            .map_err(|e| DatabaseError(format!("Failed to find sessions by agent: {}", e)))?;

        let mut sessions = Vec::new();
        for result in results {
            let row = SessionRow {
                id: result[0].clone(),
                name: result[1].clone(),
                workspace: result[2]
                    .is_empty()
                    .then_some(result[2].clone())
                    .filter(|s| !s.is_empty()),
                bead: result[3]
                    .is_empty()
                    .then_some(result[3].clone())
                    .filter(|s| !s.is_empty()),
                assigned_agent: result[4]
                    .is_empty()
                    .then_some(result[4].clone())
                    .filter(|s| !s.is_empty()),
                branch_state: result[5].clone(),
                branch_name: result[6]
                    .is_empty()
                    .then_some(result[6].clone())
                    .filter(|s| !s.is_empty()),
                session_state: result[7].clone(),
                last_synced: result[8]
                    .is_empty()
                    .then_some(result[8].clone())
                    .filter(|s| !s.is_empty()),
                created_at: result[9].clone(),
            };
            let session = Session::try_from(row)?;
            sessions.push(session);
        }
        Ok(sessions)
    }
}

impl SqliteSessionRepository {
    fn row_to_session_row(&self, result: Vec<Vec<String>>) -> Result<SessionRow> {
        if result.len() != 1 || result[0].len() != 10 {
            return Err(RepositoryError(format!(
                "Expected single row with 10 columns, got {} rows with {} columns",
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
            workspace: if cols[2].is_empty() { None } else { Some(cols[2].clone()) },
            bead: if cols[3].is_empty() { None } else { Some(cols[3].clone()) },
            assigned_agent: if cols[4].is_empty() { None } else { Some(cols[4].clone()) },
            branch_state: cols[5].clone(),
            branch_name: if cols[6].is_empty() { None } else { Some(cols[6].clone()) },
            session_state: cols[7].clone(),
            last_synced: if cols[8].is_empty() { None } else { Some(cols[8].clone()) },
            created_at: cols[9].clone(),
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
                assigned_agent: None,
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
