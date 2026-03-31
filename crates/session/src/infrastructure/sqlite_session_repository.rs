use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::domain::entities::{BranchState, Session, SessionId, SessionState};
use crate::domain::entities::session::Created;
use crate::domain::value_objects::{BeadId, SessionName, WorkspaceId};
use crate::error::{SessionError, SessionError::*, Result};
use crate::infrastructure::repository::SessionRepository;
use scp_core::infrastructure::database::{DatabaseService, SqliteDatabaseService};

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
        let bead = row.bead.map(BeadId::parse).transpose().map_err(|e| InvalidIdentifier(e.to_string()))?;
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
                    .map_err(|e| SerializationError(format!("Invalid last_synced timestamp: {}", e)))
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
            branch,
            last_synced,
            created_at,
        ))
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
    s.replace('\'', "''")
}

#[async_trait]
impl SessionRepository for SqliteSessionRepository {
    async fn save(&self, session: &Session) -> Result<()> {
        let row = SessionRow::from(session);
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
            workspace.map(|w| format!("'{}'", w)).unwrap_or_else(|| "NULL".to_string()),
            bead.map(|b| format!("'{}'", b)).unwrap_or_else(|| "NULL".to_string()),
            escape_sql_string(&row.branch_state),
            branch_name.map(|b| format!("'{}'", b)).unwrap_or_else(|| "NULL".to_string()),
            escape_sql_string(&row.session_state),
            last_synced.map(|s| format!("'{}'", s)).unwrap_or_else(|| "NULL".to_string()),
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

        let escaped_id = escape_sql_string(id);
        let results = self
            .db
            .query(&format!("SELECT id FROM sessions WHERE id = '{}'", escaped_id))
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
                if result.is_empty() { 0 } else { result[0].len() }
            )));
        }

        let cols = &result[0];
        Ok(SessionRow {
            id: cols[0].clone(),
            name: cols[1].clone(),
            workspace: cols[2].is_empty().then_some(cols[2].clone()).filter(|s| !s.is_empty()),
            bead: cols[3].is_empty().then_some(cols[3].clone()).filter(|s| !s.is_empty()),
            branch_state: cols[4].clone(),
            branch_name: cols[5].is_empty().then_some(cols[5].clone()).filter(|s| !s.is_empty()),
            session_state: cols[6].clone(),
            last_synced: cols[7].is_empty().then_some(cols[7].clone()).filter(|s| !s.is_empty()),
            created_at: cols[8].clone(),
        })
    }
}
