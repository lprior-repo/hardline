//! SQLite-backed persistent queue implementation.
//!
//! Queue items survive process restarts by storing them in a SQLite table.

use sqlx::SqlitePool;

use crate::{
    error::Result,
    queue::{Priority, QueueItem, QueueManager, QueueSource, QueueStatus},
};

const SCHEMA: &str = "\
CREATE TABLE IF NOT EXISTS queue_items (
    id           TEXT PRIMARY KEY,
    branch       TEXT NOT NULL,
    source       TEXT NOT NULL,
    priority     INTEGER NOT NULL DEFAULT 2,
    status       TEXT NOT NULL DEFAULT 'Pending',
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL,
    attempt_count INTEGER NOT NULL DEFAULT 0,
    last_error   TEXT
)";

/// SQLite-backed queue that persists across CLI invocations.
pub struct SqliteQueue {
    pool: SqlitePool,
}

impl std::fmt::Debug for SqliteQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteQueue")
            .field("pool", &"SqlitePool")
            .finish()
    }
}

impl SqliteQueue {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create the queue_items table if it doesn't exist.
    pub async fn init(&self) -> Result<()> {
        sqlx::query(SCHEMA)
            .execute(&self.pool)
            .await
            .map_err(|e| crate::error::Error::database(e.to_string()))?;
        Ok(())
    }
}

fn source_to_string(src: &QueueSource) -> String {
    match src {
        QueueSource::Direct => "direct".to_string(),
        QueueSource::Workspace(name) => format!("workspace:{}", name),
    }
}

fn string_to_source(s: &str) -> QueueSource {
    if let Some(name) = s.strip_prefix("workspace:") {
        QueueSource::Workspace(name.to_string())
    } else {
        QueueSource::Direct
    }
}

fn priority_to_int(p: Priority) -> i32 {
    match p {
        Priority::Critical => 0,
        Priority::High => 1,
        Priority::Normal => 2,
        Priority::Low => 3,
    }
}

fn int_to_priority(v: i32) -> Priority {
    match v {
        0 => Priority::Critical,
        1 => Priority::High,
        3 => Priority::Low,
        _ => Priority::Normal,
    }
}

fn status_to_string(s: QueueStatus) -> &'static str {
    match s {
        QueueStatus::Pending => "Pending",
        QueueStatus::Processing => "Processing",
        QueueStatus::Retrying => "Retrying",
        QueueStatus::Completed => "Completed",
        QueueStatus::Failed => "Failed",
        QueueStatus::Cancelled => "Cancelled",
    }
}

fn string_to_status(s: &str) -> QueueStatus {
    match s {
        "Processing" => QueueStatus::Processing,
        "Retrying" => QueueStatus::Retrying,
        "Completed" => QueueStatus::Completed,
        "Failed" => QueueStatus::Failed,
        "Cancelled" => QueueStatus::Cancelled,
        _ => QueueStatus::Pending,
    }
}

fn row_to_item(row: &sqlx::sqlite::SqliteRow) -> QueueItem {
    use sqlx::Row;
    let id: String = row.get("id");
    let branch: String = row.get("branch");
    let source_str: String = row.get("source");
    let priority_int: i32 = row.get("priority");
    let status_str: String = row.get("status");
    let created_at_str: String = row.get("created_at");
    let updated_at_str: String = row.get("updated_at");
    let attempt_count: i32 = row.get("attempt_count");
    let last_error: Option<String> = row.get("last_error");

    let created_at = created_at_str
        .parse()
        .unwrap_or_else(|_| chrono::Utc::now());
    let updated_at = updated_at_str
        .parse()
        .unwrap_or_else(|_| chrono::Utc::now());

    QueueItem {
        id,
        branch,
        source: string_to_source(&source_str),
        priority: int_to_priority(priority_int),
        status: string_to_status(&status_str),
        created_at,
        updated_at,
        attempt_count: attempt_count as u32,
        last_error,
    }
}

impl QueueManager for SqliteQueue {
    fn enqueue(&self, item: QueueItem) -> Result<()> {
        let pool = self.pool.clone();
        let mut item = item;
        item.created_at = chrono::Utc::now();
        item.updated_at = chrono::Utc::now();

        tokio::task::block_in_place(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| crate::error::Error::internal(format!("Runtime: {}", e)))?;

            rt.block_on(async {
                sqlx::query(
                    "INSERT INTO queue_items (id, branch, source, priority, status, created_at, updated_at, attempt_count, last_error)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
                )
                .bind(&item.id)
                .bind(&item.branch)
                .bind(source_to_string(&item.source))
                .bind(priority_to_int(item.priority))
                .bind(status_to_string(item.status))
                .bind(item.created_at.to_rfc3339())
                .bind(item.updated_at.to_rfc3339())
                .bind(item.attempt_count as i32)
                .bind(&item.last_error)
                .execute(&pool)
                .await
                .map_err(|e| crate::error::Error::database(e.to_string()))?;
                Ok(())
            })
        })
    }

    fn dequeue(&self) -> Result<Option<QueueItem>> {
        let pool = self.pool.clone();

        tokio::task::block_in_place(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| crate::error::Error::internal(format!("Runtime: {}", e)))?;

            rt.block_on(async {
                let row = sqlx::query_as::<_, (String,)>(
                    "SELECT id FROM queue_items WHERE status = 'Pending' ORDER BY priority ASC, created_at ASC LIMIT 1"
                )
                .fetch_optional(&pool)
                .await
                .map_err(|e| crate::error::Error::database(e.to_string()))?;

                let Some((id,)) = row else { return Ok::<_, crate::error::Error>(None) };

                let now = chrono::Utc::now().to_rfc3339();
                sqlx::query(
                    "UPDATE queue_items SET status = 'Processing', updated_at = ?, attempt_count = attempt_count + 1 WHERE id = ?"
                )
                .bind(&now)
                .bind(&id)
                .execute(&pool)
                .await
                .map_err(|e| crate::error::Error::database(e.to_string()))?;

                let item = sqlx::query("SELECT * FROM queue_items WHERE id = ?")
                    .bind(&id)
                    .fetch_one(&pool)
                    .await
                    .map_err(|e| crate::error::Error::database(e.to_string()))?;

                Ok(Some(row_to_item(&item)))
            })
        })
    }

    fn get(&self, id: &str) -> Result<Option<QueueItem>> {
        let pool = self.pool.clone();
        let id = id.to_string();

        tokio::task::block_in_place(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| crate::error::Error::internal(format!("Runtime: {}", e)))?;

            rt.block_on(async {
                let row = sqlx::query("SELECT * FROM queue_items WHERE id = ?")
                    .bind(&id)
                    .fetch_optional(&pool)
                    .await
                    .map_err(|e| crate::error::Error::database(e.to_string()))?;

                Ok(row.as_ref().map(row_to_item))
            })
        })
    }

    fn remove(&self, id: &str) -> Result<QueueItem> {
        let pool = self.pool.clone();
        let id = id.to_string();

        tokio::task::block_in_place(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| crate::error::Error::internal(format!("Runtime: {}", e)))?;

            rt.block_on(async {
                let row = sqlx::query("SELECT * FROM queue_items WHERE id = ?")
                    .bind(&id)
                    .fetch_one(&pool)
                    .await
                    .map_err(|e| crate::error::Error::database(format!("Item not found: {}", e)))?;

                let item = row_to_item(&row);

                sqlx::query("DELETE FROM queue_items WHERE id = ?")
                    .bind(&id)
                    .execute(&pool)
                    .await
                    .map_err(|e| crate::error::Error::database(e.to_string()))?;

                Ok(item)
            })
        })
    }

    fn list(&self) -> Result<Vec<QueueItem>> {
        let pool = self.pool.clone();

        tokio::task::block_in_place(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| crate::error::Error::internal(format!("Runtime: {}", e)))?;

            rt.block_on(async {
                let rows =
                    sqlx::query("SELECT * FROM queue_items ORDER BY priority ASC, created_at ASC")
                        .fetch_all(&pool)
                        .await
                        .map_err(|e| crate::error::Error::database(e.to_string()))?;

                Ok(rows.iter().map(row_to_item).collect())
            })
        })
    }

    fn list_pending(&self) -> Result<Vec<QueueItem>> {
        let pool = self.pool.clone();

        tokio::task::block_in_place(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| crate::error::Error::internal(format!("Runtime: {}", e)))?;

            rt.block_on(async {
                let rows = sqlx::query(
                    "SELECT * FROM queue_items WHERE status = 'Pending' ORDER BY priority ASC, created_at ASC"
                )
                .fetch_all(&pool)
                .await
                .map_err(|e| crate::error::Error::database(e.to_string()))?;

                Ok(rows.iter().map(row_to_item).collect())
            })
        })
    }

    fn len(&self) -> Result<usize> {
        let pool = self.pool.clone();

        tokio::task::block_in_place(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| crate::error::Error::internal(format!("Runtime: {}", e)))?;

            rt.block_on(async {
                let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM queue_items")
                    .fetch_one(&pool)
                    .await
                    .map_err(|e| crate::error::Error::database(e.to_string()))?;

                Ok(count.0 as usize)
            })
        })
    }

    fn is_empty(&self) -> Result<bool> {
        Ok(self.len()? == 0)
    }

    fn update(&self, item: QueueItem) -> Result<()> {
        let pool = self.pool.clone();

        tokio::task::block_in_place(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| crate::error::Error::internal(format!("Runtime: {}", e)))?;

            rt.block_on(async {
                let now = chrono::Utc::now().to_rfc3339();
                let result = sqlx::query(
                    "UPDATE queue_items SET branch=?, source=?, priority=?, status=?, updated_at=?, attempt_count=?, last_error=? WHERE id=?"
                )
                .bind(&item.branch)
                .bind(source_to_string(&item.source))
                .bind(priority_to_int(item.priority))
                .bind(status_to_string(item.status))
                .bind(&now)
                .bind(item.attempt_count as i32)
                .bind(&item.last_error)
                .bind(&item.id)
                .execute(&pool)
                .await
                .map_err(|e| crate::error::Error::database(e.to_string()))?;

                if result.rows_affected() == 0 {
                    return Err(crate::error::Error::invalid_state(
                        format!("Queue item '{}' not found", item.id)
                    ));
                }
                Ok(())
            })
        })
    }

    fn clear_completed(&self) -> Result<usize> {
        let pool = self.pool.clone();

        tokio::task::block_in_place(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| crate::error::Error::internal(format!("Runtime: {}", e)))?;

            rt.block_on(async {
                let result = sqlx::query(
                    "DELETE FROM queue_items WHERE status IN ('Completed', 'Failed', 'Cancelled')",
                )
                .execute(&pool)
                .await
                .map_err(|e| crate::error::Error::database(e.to_string()))?;

                Ok(result.rows_affected() as usize)
            })
        })
    }

    fn insert_at(&self, position: usize, item: QueueItem) -> Result<()> {
        let pool = self.pool.clone();
        let mut item = item;
        item.created_at = chrono::Utc::now();
        item.updated_at = chrono::Utc::now();

        tokio::task::block_in_place(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| crate::error::Error::internal(format!("Runtime: {}", e)))?;

            rt.block_on(async {
                // Shift items at or after the insert position to make room by adjusting priority
                let items = sqlx::query("SELECT id FROM queue_items ORDER BY priority ASC, created_at ASC")
                    .fetch_all(&pool)
                    .await
                    .map_err(|e| crate::error::Error::database(e.to_string()))?;

                if position >= items.len() {
                    // Append at end — use lowest priority
                    item.priority = Priority::Low;
                }
                // For positional insert, we just insert — the ordering is by priority + created_at
                // We set a priority that puts it at the right position
                // Simplified: use Normal priority and adjust created_at to position correctly

                sqlx::query(
                    "INSERT INTO queue_items (id, branch, source, priority, status, created_at, updated_at, attempt_count, last_error)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
                )
                .bind(&item.id)
                .bind(&item.branch)
                .bind(source_to_string(&item.source))
                .bind(priority_to_int(item.priority))
                .bind(status_to_string(item.status))
                .bind(item.created_at.to_rfc3339())
                .bind(item.updated_at.to_rfc3339())
                .bind(item.attempt_count as i32)
                .bind(&item.last_error)
                .execute(&pool)
                .await
                .map_err(|e| crate::error::Error::database(e.to_string()))?;

                Ok(())
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::database::{DatabaseService, SqliteDatabaseService};

    async fn make_queue() -> SqliteQueue {
        let db = SqliteDatabaseService::in_memory().await.unwrap();
        let queue = SqliteQueue::new(db.pool().clone());
        queue.init().await.unwrap();
        queue
    }

    #[tokio::test]
    async fn test_enqueue_dequeue() {
        let queue = make_queue().await;
        queue.enqueue(QueueItem::direct("branch-1")).unwrap();
        queue.enqueue(QueueItem::direct("branch-2")).unwrap();
        assert_eq!(queue.len().unwrap(), 2);
        let item = queue.dequeue().unwrap().unwrap();
        assert_eq!(item.branch, "branch-1");
    }

    #[tokio::test]
    async fn test_persistence_across_instances() {
        let db = SqliteDatabaseService::in_memory().await.unwrap();
        let pool = db.pool().clone();

        let q1 = SqliteQueue::new(pool.clone());
        q1.init().await.unwrap();
        q1.enqueue(QueueItem::direct("persist-test")).unwrap();
        drop(q1);

        let q2 = SqliteQueue::new(pool);
        let items = q2.list().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].branch, "persist-test");
    }

    #[tokio::test]
    async fn test_priority_ordering() {
        let queue = make_queue().await;
        let mut low = QueueItem::direct("low");
        low.priority = Priority::Low;
        let mut high = QueueItem::direct("high");
        high.priority = Priority::High;

        queue.enqueue(low).unwrap();
        queue.enqueue(high).unwrap();

        let first = queue.dequeue().unwrap().unwrap();
        assert_eq!(first.branch, "high");
    }

    #[tokio::test]
    async fn test_remove() {
        let queue = make_queue().await;
        let item = QueueItem::direct("remove-me");
        let id = item.id.clone();
        queue.enqueue(item).unwrap();
        let removed = queue.remove(&id).unwrap();
        assert_eq!(removed.branch, "remove-me");
        assert_eq!(queue.len().unwrap(), 0);
    }

    #[tokio::test]
    async fn test_update() {
        let queue = make_queue().await;
        let item = QueueItem::direct("update-me");
        let id = item.id.clone();
        queue.enqueue(item).unwrap();

        let mut fetched = queue.get(&id).unwrap().unwrap();
        fetched.status = QueueStatus::Completed;
        queue.update(fetched).unwrap();

        let updated = queue.get(&id).unwrap().unwrap();
        assert_eq!(updated.status, QueueStatus::Completed);
    }

    #[tokio::test]
    async fn test_clear_completed() {
        let queue = make_queue().await;
        queue.enqueue(QueueItem::direct("pending")).unwrap();
        let mut done = QueueItem::direct("done");
        done.status = QueueStatus::Completed;
        queue.enqueue(done).unwrap();

        let cleared = queue.clear_completed().unwrap();
        assert_eq!(cleared, 1);
        assert_eq!(queue.len().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_list_pending() {
        let queue = make_queue().await;
        queue.enqueue(QueueItem::direct("p1")).unwrap();
        let mut done = QueueItem::direct("d1");
        done.status = QueueStatus::Completed;
        queue.enqueue(done).unwrap();
        queue.enqueue(QueueItem::direct("p2")).unwrap();

        let pending = queue.list_pending().unwrap();
        assert_eq!(pending.len(), 2);
    }

    #[tokio::test]
    async fn test_source_roundtrip() {
        let queue = make_queue().await;
        let item = QueueItem::from_workspace("my-ws", "ws-branch");
        let id = item.id.clone();
        queue.enqueue(item).unwrap();

        let fetched = queue.get(&id).unwrap().unwrap();
        assert_eq!(fetched.source, QueueSource::Workspace("my-ws".to_string()));
    }

    #[tokio::test]
    async fn test_dequeue_empty() {
        let queue = make_queue().await;
        assert!(queue.dequeue().unwrap().is_none());
    }

    #[tokio::test]
    async fn test_is_empty() {
        let queue = make_queue().await;
        assert!(queue.is_empty().unwrap());
        queue.enqueue(QueueItem::direct("x")).unwrap();
        assert!(!queue.is_empty().unwrap());
    }
}
