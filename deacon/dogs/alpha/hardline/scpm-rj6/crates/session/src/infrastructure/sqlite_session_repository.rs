use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::domain::entities::{BranchState, Session, SessionId, SessionName, SessionState};
use crate::domain::value_objects::{BeadId, WorkspaceId};
use crate::error::{SessionError, SessionError::*, Result};
use crate::infrastructure::repository::SessionRepository;
use scp_core::infrastructure::database::{DatabaseService, SqliteDatabaseService};

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct SessionRow {
    pub id: String,
    pub name: String,
    pub workspace: Option<String>,
    pub bead: Option<String>,
    pub branch_state: String,
    pub branch_name: Option<String>,
    pub session_state: String,
    pub created_at: String,
}

impl TryFrom<SessionRow> for Session {
    type Error = SessionError;

    fn try_from(row: SessionRow) -> Result<Self, Self::Error> {
        let id = SessionId::parse(&row.id).map_err(|e| InvalidIdentifier(e.to_string()))?;
        let name = SessionName::parse(&row.name).map_err(|e| InvalidIdentifier(e.to_string()))?;
        let workspace = row
            .workspace
            .map(WorkspaceId::parse)
            .transpose()
            .map_err(|e| InvalidIdentifier(e.to_string()))?;
        let bead = row.bead.map(BeadId::parse).transpose().map_err(|e| InvalidIdentifier(e.to_string()))?;
        let branch = match (row.branch_state.as_str(), row.branch_name) {
            ("Detached", None) => BranchState::Detached,
            ("OnBranch", Some(name)) => BranchState::OnBranch { name },
            _ => {
                return Err(RepositoryError(format!(
                    "Invalid branch state combination: {} / {:?}",
                    row.branch_state, row.branch_name
                )))
            }
        };
        let state = parse_session_state(&row.session_state)?;
        let created_at = DateTime::parse_from_rfc3339(&row.created_at)
            .map_err(|e| SerializationError(format!("Invalid created_at timestamp: {}", e)))?
            .with_timezone(&Utc);

        Ok(Self {
            id,
            name,
            workspace,
            bead,
            branch,
            state,
            created_at,
        })
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
            session_state: format!("{:?}", session.state),
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

#[async_trait]
impl SessionRepository for SqliteSessionRepository {
    async fn save(&self, session: &Session) -> Result<()> {
        let row: SessionRow = session.into();
        self.db
            .execute(
                r#"
                INSERT INTO sessions (id, name, workspace, bead, branch_state, branch_name, session_state, created_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    workspace = excluded.workspace,
                    bead = excluded.bead,
                    branch_state = excluded.branch_state,
                    branch_name = excluded.branch_name,
                    session_state = excluded.session_state,
                    created_at = excluded.created_at
                "#,
            )
            .await
            .map_err(|e| DatabaseError(format!("Failed to save session: {}", e)))?;
        Ok(())
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<Session>> {
        if id.is_empty() {
            return Err(InvalidIdentifier("SessionId cannot be empty".to_string()));
        }
        let _ = SessionId::parse(id).map_err(|e| InvalidIdentifier(e.to_string()))?;

        let results = self
            .db
            .query(
                "SELECT id, name, workspace, bead, branch_state, branch_name, session_state, created_at 
                 FROM sessions WHERE id = ?",
            )
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
        let name_str = name.as_str();
        let results = self
            .db
            .query(
                "SELECT id, name, workspace, bead, branch_state, branch_name, session_state, created_at 
                 FROM sessions WHERE name = ?",
            )
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
                "SELECT id, name, workspace, bead, branch_state, branch_name, session_state, created_at 
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

        let results = self
            .db
            .query("SELECT id FROM sessions WHERE id = ?")
            .await
            .map_err(|e| DatabaseError(format!("Failed to delete session: {}", e)))?;

        if results.is_empty() {
            return Err(NotFound(format!("Session not found: {}", id)));
        }

        self.db
            .execute("DELETE FROM sessions WHERE id = ?")
            .await
            .map_err(|e| DatabaseError(format!("Failed to delete session: {}", e)))?;
        Ok(())
    }
}

impl SqliteSessionRepository {
    fn row_to_session_row(&self, result: Vec<Vec<String>>) -> Result<SessionRow> {
        if result.len() != 1 || result[0].len() != 8 {
            return Err(RepositoryError(format!(
                "Expected single row with 8 columns, got {} rows with {} columns",
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
            created_at: cols[7].clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::Session;
    use scp_core::infrastructure::database::SqliteDatabaseService;

    async fn test_repository() -> SqliteSessionRepository {
        let db = SqliteDatabaseService::in_memory()
            .await
            .expect("failed to create in-memory database");
        let repo = SqliteSessionRepository::new(db);
        repo.init_schema().await.expect("failed to init schema");
        repo
    }

    #[tokio::test]
    async fn test_saves_valid_session_and_retrieves_by_id() {
        let repo = test_repository().await;
        let name = SessionName::parse("test-session").expect("valid");
        let session = Session::create(name).expect("created");

        repo.save(&session).await.expect("save failed");
        let found = repo.find_by_id(session.id.as_str()).await.expect("find failed");

        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.id, session.id);
        assert_eq!(found.name.as_str(), "test-session");
    }

    #[tokio::test]
    async fn test_saves_valid_session_and_retrieves_by_name() {
        let repo = test_repository().await;
        let name = SessionName::parse("my-session").expect("valid");
        let session = Session::create(name).expect("created");

        repo.save(&session).await.expect("save failed");
        let found = repo
            .find_by_name(&session.name)
            .await
            .expect("find failed");

        assert!(found.is_some());
        assert_eq!(found.unwrap().name.as_str(), "my-session");
    }

    #[tokio::test]
    async fn test_lists_all_sessions() {
        let repo = test_repository().await;
        let s1 = Session::create(SessionName::parse("session-1").unwrap()).unwrap();
        let s2 = Session::create(SessionName::parse("session-2").unwrap()).unwrap();
        let s3 = Session::create(SessionName::parse("session-3").unwrap()).unwrap();

        repo.save(&s1).await.expect("save failed");
        repo.save(&s2).await.expect("save failed");
        repo.save(&s3).await.expect("save failed");

        let list = repo.list().await.expect("list failed");
        assert_eq!(list.len(), 3);
    }

    #[tokio::test]
    async fn test_deletes_existing_session() {
        let repo = test_repository().await;
        let session = Session::create(SessionName::parse("to-delete").unwrap()).unwrap();

        repo.save(&session).await.expect("save failed");
        repo.delete(session.id.as_str()).await.expect("delete failed");

        let found = repo.find_by_id(session.id.as_str()).await.expect("find failed");
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_find_by_id_returns_not_found_for_nonexistent() {
        let repo = test_repository().await;
        let found = repo.find_by_id("nonexistent-id").await.expect("find failed");
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_delete_returns_not_found_for_nonexistent() {
        let repo = test_repository().await;
        let result = repo.delete("nonexistent").await;
        assert!(result.is_err());
        match result {
            Err(NotFound(_)) => {}
            Err(e) => panic!("Expected NotFound, got {:?}", e),
            Ok(_) => panic!("Expected error, got Ok"),
        }
    }

    #[tokio::test]
    async fn test_handles_empty_database_gracefully() {
        let repo = test_repository().await;
        let list = repo.list().await.expect("list failed");
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn test_list_after_delete_returns_correct_count() {
        let repo = test_repository().await;
        let s1 = Session::create(SessionName::parse("s1").unwrap()).unwrap();
        let s2 = Session::create(SessionName::parse("s2").unwrap()).unwrap();
        let s3 = Session::create(SessionName::parse("s3").unwrap()).unwrap();

        repo.save(&s1).await.expect("save failed");
        repo.save(&s2).await.expect("save failed");
        repo.save(&s3).await.expect("save failed");

        repo.delete(s2.id.as_str()).await.expect("delete failed");

        let list = repo.list().await.expect("list failed");
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_p3_violation_empty_session_id() {
        let repo = test_repository().await;
        let result = repo.find_by_id("").await;
        assert!(result.is_err());
        match result {
            Err(InvalidIdentifier(_)) => {}
            Err(e) => panic!("Expected InvalidIdentifier, got {:?}", e),
            Ok(_) => panic!("Expected error, got Ok"),
        }
    }

    #[tokio::test]
    async fn test_save_same_id_twice_updates() {
        let repo = test_repository().await;
        let name1 = SessionName::parse("session-123").unwrap();
        let session1 = Session::create(name1).unwrap();

        let session2 = Session {
            id: session1.id.clone(),
            name: SessionName::parse("updated-session").unwrap(),
            workspace: None,
            bead: None,
            branch: BranchState::Detached,
            state: SessionState::Completed,
            created_at: session1.created_at,
        };

        repo.save(&session1).await.expect("first save failed");
        repo.save(&session2).await.expect("second save failed");

        let found = repo.find_by_id(session1.id.as_str()).await.expect("find failed");
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.state, SessionState::Completed);
        assert_eq!(found.name.as_str(), "updated-session");
    }
}
