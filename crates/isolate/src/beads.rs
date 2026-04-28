//! Unified bead repository for managing issues in both `SQLite` and JSONL formats

use std::path::PathBuf;

use isolate_core::{Error, Result};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// Status of an issue in the beads tracker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BeadStatus {
    Open,
    InProgress,
    Blocked,
    Deferred,
    Closed,
}

impl std::fmt::Display for BeadStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Open => "open",
            Self::InProgress => "in_progress",
            Self::Blocked => "blocked",
            Self::Deferred => "deferred",
            Self::Closed => "closed",
        };
        write!(f, "{s}")
    }
}

impl std::str::FromStr for BeadStatus {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "open" | "●" | "ready" => Ok(Self::Open),
            "in_progress" | "working" | "in-progress" => Ok(Self::InProgress),
            "blocked" => Ok(Self::Blocked),
            "deferred" => Ok(Self::Deferred),
            "closed" | "completed" | "done" => Ok(Self::Closed),
            _ => Err(Error::ValidationError {
                message: format!("invalid bead status: {s}"),
                field: Some("status".to_string()),
                value: Some(s.to_string()),
                constraints: vec![
                    "valid values are: open, in_progress, blocked, deferred, closed".to_string(),
                ],
            }),
        }
    }
}

/// Metadata for a bead
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadMetadata {
    pub id: String,
    pub title: String,
    pub status: BeadStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Enable `WAL` mode on the `SQLite` connection for better crash recovery.
///
/// # Errors
///
/// Returns `Error` if the `PRAGMA` statement fails.
async fn enable_wal_mode(pool: &SqlitePool) -> Result<()> {
    sqlx::query("PRAGMA journal_mode=WAL;")
        .execute(pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("failed to enable WAL mode: {e}")))?;
    Ok(())
}

/// Unified repository for beads
pub struct BeadRepository {
    root: PathBuf,
}

impl BeadRepository {
    /// Create a new repository instance
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Get bead by ID
    pub async fn get_bead(&self, id: &str) -> Result<Option<BeadMetadata>> {
        // Railway-oriented: Try SQLite, fallback to JSONL
        match self.get_bead_sqlite(id).await {
            Ok(Some(bead)) => Ok(Some(bead)),
            _ => self.get_bead_jsonl(id).await,
        }
    }

    /// Update bead status
    pub async fn update_status(&self, id: &str, status: BeadStatus) -> Result<()> {
        if self.beads_db_path().exists() {
            self.update_status_sqlite(id, status).await?;
        } else if self.issues_jsonl_path().exists() {
            self.update_status_jsonl(id, status).await?;
        } else {
            return Err(Error::NotFound(
                "no beads database or issues file found to update".to_string(),
            ));
        }

        Ok(())
    }

    /// List all beads
    pub async fn list_beads(&self) -> Result<Vec<BeadMetadata>> {
        // Load from JSONL then supplement with SQLite using im::HashMap for functional merging
        let jsonl_beads = self.list_beads_jsonl().await.unwrap_or_default();
        let initial_map = jsonl_beads
            .into_iter()
            .fold(im::HashMap::new(), |mut acc, b| {
                acc.insert(b.id.clone(), b);
                acc
            });

        let sqlite_beads = self.list_beads_sqlite().await.unwrap_or_default();
        let final_map = sqlite_beads.into_iter().fold(initial_map, |mut acc, b| {
            acc.insert(b.id.clone(), b);
            acc
        });

        Ok(final_map.into_iter().map(|(_, v)| v).collect())
    }

    async fn list_beads_sqlite(&self) -> Result<Vec<BeadMetadata>> {
        let path = self.beads_db_path();
        if !path.exists() {
            return Ok(Vec::new());
        }

        let connection_string = format!("sqlite:{}?mode=rw", path.display());
        let pool = SqlitePool::connect(&connection_string)
            .await
            .map_err(|e| Error::DatabaseError(format!("failed to connect: {e}")))?;

        // Enable WAL mode for better crash recovery
        enable_wal_mode(&pool).await?;

        let rows: Vec<(String, String, String)> =
            sqlx::query_as("SELECT id, title, status FROM issues")
                .fetch_all(&pool)
                .await
                .map_err(|e| Error::DatabaseError(format!("failed to query: {e}")))?;

        Ok(rows
            .into_iter()
            .map(|(id, title, status_str)| BeadMetadata {
                id,
                title,
                status: status_str.parse().unwrap_or(BeadStatus::Open),
                description: None,
            })
            .collect())
    }

    async fn list_beads_jsonl(&self) -> Result<Vec<BeadMetadata>> {
        use tokio::fs;

        let path = self.issues_jsonl_path();
        if !path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(path)
            .await
            .map_err(|e| Error::IoError(format!("failed to read file: {e}")))?;

        let beads = content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|line| {
                let json: serde_json::Value = serde_json::from_str(line).ok()?;
                let id = json.get("id")?.as_str()?.to_string();
                let title = json
                    .get("title")
                    .and_then(|v| v.as_str())
                    .map_or_else(|| String::from("Unknown"), String::from);
                let status_str = json
                    .get("status")
                    .and_then(|v| v.as_str())
                    .map_or("open", |s| s);
                let description = json
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                Some(BeadMetadata {
                    id,
                    title,
                    status: status_str.parse().unwrap_or(BeadStatus::Open),
                    description,
                })
            })
            .collect();

        Ok(beads)
    }

    fn beads_db_path(&self) -> PathBuf {
        self.root.join(".beads/beads.db")
    }

    fn issues_jsonl_path(&self) -> PathBuf {
        self.root.join(".beads/issues.jsonl")
    }

    async fn get_bead_sqlite(&self, bead_id: &str) -> Result<Option<BeadMetadata>> {
        let path = self.beads_db_path();
        if !path.exists() {
            return Ok(None);
        }

        let connection_string = format!("sqlite:{}?mode=rw", path.display());
        let pool = SqlitePool::connect(&connection_string)
            .await
            .map_err(|e| Error::DatabaseError(format!("failed to connect: {e}")))?;

        // Enable WAL mode for better crash recovery
        enable_wal_mode(&pool).await?;

        let result: Option<(String, String, String)> =
            sqlx::query_as("SELECT id, title, status FROM issues WHERE id = ?1")
                .bind(bead_id)
                .fetch_optional(&pool)
                .await
                .map_err(|e| Error::DatabaseError(format!("failed to query: {e}")))?;

        Ok(result.map(|(id, title, status_str)| BeadMetadata {
            id,
            title,
            status: status_str.parse().unwrap_or(BeadStatus::Open),
            description: None,
        }))
    }

    async fn get_bead_jsonl(&self, bead_id: &str) -> Result<Option<BeadMetadata>> {
        use tokio::fs;

        let path = self.issues_jsonl_path();
        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(path)
            .await
            .map_err(|e| Error::IoError(format!("failed to read file: {e}")))?;

        let bead = content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .find_map(|line| {
                let json: serde_json::Value = serde_json::from_str(line).ok()?;
                if json.get("id").and_then(|v| v.as_str()) == Some(bead_id) {
                    let title = json
                        .get("title")
                        .and_then(|v| v.as_str())
                        .map_or_else(|| String::from("Unknown"), String::from);
                    let status_str = json
                        .get("status")
                        .and_then(|v| v.as_str())
                        .map_or("open", |s| s);
                    let description = json
                        .get("description")
                        .and_then(|v| v.as_str())
                        .map(String::from);

                    Some(BeadMetadata {
                        id: bead_id.to_string(),
                        title,
                        status: status_str.parse().unwrap_or(BeadStatus::Open),
                        description,
                    })
                } else {
                    None
                }
            });

        Ok(bead)
    }

    async fn update_status_sqlite(&self, bead_id: &str, status: BeadStatus) -> Result<()> {
        let path = self.beads_db_path();
        let connection_string = format!("sqlite:{}?mode=rw", path.display());
        let pool = SqlitePool::connect(&connection_string)
            .await
            .map_err(|e| Error::DatabaseError(format!("failed to connect: {e}")))?;

        // Enable WAL mode for better crash recovery
        enable_wal_mode(&pool).await?;

        // Atomically set closed_at when closing, or NULL when reopening
        // This satisfies the CHECK constraint: status='closed' => closed_at IS NOT NULL
        match status {
            BeadStatus::Closed => {
                sqlx::query(
                    "UPDATE issues SET status = ?1, updated_at = datetime('now'), closed_at = datetime('now') WHERE id = ?2"
                )
                .bind(status.to_string())
                .bind(bead_id)
                .execute(&pool)
                .await
                .map_err(|e| Error::DatabaseError(format!("failed to update: {e}")))?;
            }
            _ => {
                sqlx::query(
                    "UPDATE issues SET status = ?1, updated_at = datetime('now'), closed_at = NULL WHERE id = ?2"
                )
                .bind(status.to_string())
                .bind(bead_id)
                .execute(&pool)
                .await
                .map_err(|e| Error::DatabaseError(format!("failed to update: {e}")))?;
            }
        }

        Ok(())
    }

    async fn update_status_jsonl(&self, bead_id: &str, status: BeadStatus) -> Result<()> {
        use tokio::fs;

        let path = self.issues_jsonl_path();
        let content = fs::read_to_string(&path)
            .await
            .map_err(|e| Error::IoError(format!("failed to read file: {e}")))?;

        let now = chrono::Utc::now().to_rfc3339();

        let (new_content, updated) = content.lines().filter(|l| !l.trim().is_empty()).try_fold(
            (String::new(), false),
            |(mut acc, mut updated), line| {
                let mut json: serde_json::Value = serde_json::from_str(line)
                    .map_err(|e| Error::ParseError(format!("failed to parse JSON: {e}")))?;
                if json.get("id").and_then(|v| v.as_str()) == Some(bead_id) {
                    json["status"] = serde_json::json!(status.to_string());
                    json["updated_at"] = serde_json::json!(now.clone());
                    // Atomically set/unset closed_at to match status
                    match status {
                        BeadStatus::Closed => {
                            json["closed_at"] = serde_json::json!(now.clone());
                        }
                        _ => {
                            json["closed_at"] = serde_json::Value::Null;
                        }
                    }
                    updated = true;
                }
                acc.push_str(&json.to_string());
                acc.push('\n');
                Ok::<(String, bool), Error>((acc, updated))
            },
        )?;

        if updated {
            fs::write(path, new_content)
                .await
                .map_err(|e| Error::IoError(format!("failed to write file: {e}")))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bead_status_display() {
        assert_eq!(BeadStatus::Open.to_string(), "open");
        assert_eq!(BeadStatus::InProgress.to_string(), "in_progress");
        assert_eq!(BeadStatus::Blocked.to_string(), "blocked");
        assert_eq!(BeadStatus::Deferred.to_string(), "deferred");
        assert_eq!(BeadStatus::Closed.to_string(), "closed");
    }

    #[test]
    fn test_bead_status_from_str() {
        assert_eq!("open".parse().unwrap(), BeadStatus::Open);
        assert_eq!("in_progress".parse().unwrap(), BeadStatus::InProgress);
        assert_eq!("blocked".parse().unwrap(), BeadStatus::Blocked);
        assert_eq!("deferred".parse().unwrap(), BeadStatus::Deferred);
        assert_eq!("closed".parse().unwrap(), BeadStatus::Closed);
    }

    #[test]
    fn test_bead_status_from_str_aliases() {
        assert_eq!("●".parse().unwrap(), BeadStatus::Open);
        assert_eq!("ready".parse().unwrap(), BeadStatus::Open);
        assert_eq!("working".parse().unwrap(), BeadStatus::InProgress);
        assert_eq!("done".parse().unwrap(), BeadStatus::Closed);
        assert_eq!("completed".parse().unwrap(), BeadStatus::Closed);
    }

    #[test]
    fn test_bead_status_from_str_invalid() {
        assert!("invalid".parse::<BeadStatus>().is_err());
    }
}
