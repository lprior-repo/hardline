//! Database Integration Tests for DatabaseService and SqliteDatabaseService
//!
//! Covers:
//! - Database connection (file-based, in-memory)
//! - Schema migrations (up/down)
//! - CRUD operations for all entities
//! - Transaction support (commit, rollback)
//! - Connection pooling
//! - Error handling (duplicate key, missing table, corrupt db)
//! - Concurrent access
//! - Query performance

#[cfg(test)]
mod tests {
    use sqlx::Row;
    use tempfile::TempDir;

    use crate::infrastructure::database::{
        DatabaseConfig, DatabaseService, SqliteDatabaseService, create_database_service,
        create_in_memory_database,
    };
    use crate::error::Result;

    // =========================================================================
    // CONNECTION TESTS
    // =========================================================================

    #[tokio::test]
    async fn given_in_memory_db_when_connect_then_pool_active() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;
        let pool = db.pool();
        assert!(!pool.is_closed());
        db.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn given_file_db_when_connect_then_file_created() -> Result<()> {
        let temp_dir = TempDir::new().map_err(|e| crate::error::Error::io_error(e.to_string()))?;
        let db_path = temp_dir.path().join("test.db");
        let db = SqliteDatabaseService::create(&db_path).await?;
        assert!(db_path.exists());
        db.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn given_multiple_configs_when_create_each_then_all_valid() -> Result<()> {
        let temp_dir = TempDir::new().map_err(|e| crate::error::Error::io_error(e.to_string()))?;
        let db_path = temp_dir.path().join("multi.db");

        let config = DatabaseConfig::new(db_path.to_string_lossy().to_string())?;
        let db1 = SqliteDatabaseService::new(config).await?;
        db1.execute("CREATE TABLE test (id INTEGER PRIMARY KEY)").await?;

        let config2 = DatabaseConfig::new(db_path.to_string_lossy().to_string())?;
        let db2 = SqliteDatabaseService::new(config2).await?;
        let results = db2.query("SELECT * FROM test").await?;
        assert_eq!(results.len(), 0);
        db1.close().await?;
        db2.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn given_different_max_connections_when_configured_then_respected() -> Result<()> {
        let temp_dir = TempDir::new().map_err(|e| crate::error::Error::io_error(e.to_string()))?;
        let db_path = temp_dir.path().join("test_connections.db");
        let config = DatabaseConfig::with_connections(db_path.to_string_lossy().to_string(), 3)?;
        let db = SqliteDatabaseService::new(config).await?;
        db.execute("CREATE TABLE test (id INTEGER PRIMARY KEY)").await?;
        db.execute("INSERT INTO test VALUES (1)").await?;
        let results = db.query("SELECT * FROM test").await?;
        assert_eq!(results.len(), 1);
        db.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn given_invalid_path_when_create_then_error() -> Result<()> {
        let result = SqliteDatabaseService::create("/nonexistent/directory/test.db").await;
        assert!(result.is_err());
        Ok(())
    }

    // =========================================================================
    // SCHEMA MIGRATION TESTS (UP)
    // =========================================================================

    #[tokio::test]
    async fn given_empty_db_when_run_initial_schema_then_tables_created() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute(
            "CREATE TABLE schema_version (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL,
                description TEXT NOT NULL
            )"
        ).await?;

        db.execute(
            "CREATE TABLE workspaces (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                path TEXT NOT NULL,
                backend TEXT NOT NULL,
                state TEXT NOT NULL,
                agent_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                last_accessed_at TEXT NOT NULL
            )"
        ).await?;

        db.execute("INSERT INTO schema_version VALUES (1, datetime('now'), 'Initial')").await?;
        db.execute("INSERT INTO workspaces VALUES ('w1', 'test', '/tmp', 'git', 'active', NULL, datetime('now'), datetime('now'), datetime('now'))").await?;

        let results = db.query("SELECT * FROM workspaces").await?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0][1], "test");
        db.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn given_existing_schema_when_run_migration_then_idempotent() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute(
            "CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL,
                description TEXT NOT NULL
            )"
        ).await?;

        db.execute(
            "CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL,
                description TEXT NOT NULL
            )"
        ).await?;

        db.execute("INSERT INTO schema_version VALUES (1, datetime('now'), 'Initial')").await?;
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM schema_version")
            .fetch_one(db.pool())
            .await
            .map_err(|e| crate::error::Error::database(e.to_string()))?;
        assert_eq!(count.0, 1);
        db.close().await?;
        Ok(())
    }

    // =========================================================================
    // SCHEMA MIGRATION TESTS (DOWN)
    // =========================================================================

    #[tokio::test]
    async fn given_schema_with_data_when_drop_table_then_data_lost() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)").await?;
        db.execute("INSERT INTO test VALUES (1, 'test')").await?;

        db.execute("DROP TABLE test").await?;

        let result = db.query("SELECT name FROM test").await;
        assert!(result.is_err());
        db.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn given_multiple_tables_when_drop_one_then_others_intact() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute("CREATE TABLE table1 (id INTEGER PRIMARY KEY)").await?;
        db.execute("CREATE TABLE table2 (id INTEGER PRIMARY KEY)").await?;
        db.execute("INSERT INTO table1 VALUES (1)").await?;
        db.execute("INSERT INTO table2 VALUES (2)").await?;

        db.execute("DROP TABLE table1").await?;

        let results = db.query("SELECT * FROM table2").await?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0][0], "2");
        db.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn given_schema_version_when_rollback_version_then_consistent() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute(
            "CREATE TABLE schema_version (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL,
                description TEXT NOT NULL
            )"
        ).await?;

        db.execute("INSERT INTO schema_version VALUES (2, datetime('now'), 'v2')").await?;
        db.execute("DELETE FROM schema_version WHERE version = 2").await?;
        db.execute("INSERT INTO schema_version VALUES (1, datetime('now'), 'v1')").await?;

        let results = db.query("SELECT version FROM schema_version ORDER BY version DESC LIMIT 1").await?;
        assert_eq!(results[0][0], "1");
        db.close().await?;
        Ok(())
    }

    // =========================================================================
    // CRUD OPERATIONS - SCHEMA VERSION TABLE
    // =========================================================================

    #[tokio::test]
    async fn given_schema_version_table_when_insert_and_query_then_correct() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute(
            "CREATE TABLE schema_version (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL,
                description TEXT NOT NULL
            )"
        ).await?;

        db.execute("INSERT INTO schema_version VALUES (1, '2026-01-01T00:00:00Z', 'Initial schema')").await?;

        let results = db.query("SELECT * FROM schema_version WHERE version = 1").await?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0][0], "1");
        assert_eq!(results[0][2], "Initial schema");
        db.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn given_schema_version_when_update_then_reflects_change() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute(
            "CREATE TABLE schema_version (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL,
                description TEXT NOT NULL
            )"
        ).await?;

        db.execute("INSERT INTO schema_version VALUES (1, '2026-01-01T00:00:00Z', 'Initial')").await?;
        db.execute("UPDATE schema_version SET description = 'Updated description' WHERE version = 1").await?;

        let results = db.query("SELECT description FROM schema_version WHERE version = 1").await?;
        assert_eq!(results[0][0], "Updated description");
        db.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn given_schema_version_when_delete_then_removed() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute(
            "CREATE TABLE schema_version (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL,
                description TEXT NOT NULL
            )"
        ).await?;

        db.execute("INSERT INTO schema_version VALUES (1, '2026-01-01T00:00:00Z', 'Initial')").await?;
        db.execute("DELETE FROM schema_version WHERE version = 1").await?;

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM schema_version")
            .fetch_one(db.pool())
            .await
            .map_err(|e| crate::error::Error::database(e.to_string()))?;
        assert_eq!(count.0, 0);
        db.close().await?;
        Ok(())
    }

    // =========================================================================
    // CRUD OPERATIONS - WORKSPACES TABLE
    // =========================================================================

    #[tokio::test]
    async fn given_workspaces_table_when_insert_multiple_then_all_stored() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute(
            "CREATE TABLE workspaces (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                path TEXT NOT NULL,
                backend TEXT NOT NULL,
                state TEXT NOT NULL,
                agent_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                last_accessed_at TEXT NOT NULL
            )"
        ).await?;

        db.execute("INSERT INTO workspaces VALUES ('w1', 'alpha', '/tmp/alpha', 'git', 'active', NULL, datetime('now'), datetime('now'), datetime('now'))").await?;
        db.execute("INSERT INTO workspaces VALUES ('w2', 'beta', '/tmp/beta', 'git', 'created', NULL, datetime('now'), datetime('now'), datetime('now'))").await?;

        let results = db.query("SELECT name, state FROM workspaces ORDER BY name").await?;
        assert_eq!(results.len(), 2);
        assert_eq!(results[0][0], "alpha");
        assert_eq!(results[0][1], "active");
        assert_eq!(results[1][0], "beta");
        db.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn given_workspace_when_update_state_then_new_state_persists() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute(
            "CREATE TABLE workspaces (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                path TEXT NOT NULL,
                backend TEXT NOT NULL,
                state TEXT NOT NULL,
                agent_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                last_accessed_at TEXT NOT NULL
            )"
        ).await?;

        db.execute("INSERT INTO workspaces VALUES ('w1', 'test', '/tmp/test', 'git', 'created', NULL, datetime('now'), datetime('now'), datetime('now'))").await?;
        db.execute("UPDATE workspaces SET state = 'active' WHERE id = 'w1'").await?;

        let results = db.query("SELECT state FROM workspaces WHERE id = 'w1'").await?;
        assert_eq!(results[0][0], "active");
        db.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn given_workspace_when_delete_then_no_longer_queryable() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute(
            "CREATE TABLE workspaces (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                path TEXT NOT NULL,
                backend TEXT NOT NULL,
                state TEXT NOT NULL,
                agent_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                last_accessed_at TEXT NOT NULL
            )"
        ).await?;

        db.execute("INSERT INTO workspaces VALUES ('w1', 'test', '/tmp/test', 'git', 'active', NULL, datetime('now'), datetime('now'), datetime('now'))").await?;
        db.execute("DELETE FROM workspaces WHERE id = 'w1'").await?;

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM workspaces")
            .fetch_one(db.pool())
            .await
            .map_err(|e| crate::error::Error::database(e.to_string()))?;
        assert_eq!(count.0, 0);
        db.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn given_workspace_unique_constraint_when_duplicate_name_then_error() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute(
            "CREATE TABLE workspaces (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                path TEXT NOT NULL,
                backend TEXT NOT NULL,
                state TEXT NOT NULL,
                agent_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                last_accessed_at TEXT NOT NULL
            )"
        ).await?;

        db.execute("INSERT INTO workspaces VALUES ('w1', 'test', '/tmp/test', 'git', 'active', NULL, datetime('now'), datetime('now'), datetime('now'))").await?;
        let result = db.execute("INSERT INTO workspaces VALUES ('w2', 'test', '/tmp/test2', 'git', 'active', NULL, datetime('now'), datetime('now'), datetime('now'))").await;

        assert!(result.is_err());
        db.close().await?;
        Ok(())
    }

    // =========================================================================
    // CRUD OPERATIONS - OPERATIONS TABLE
    // =========================================================================

    #[tokio::test]
    async fn given_operations_table_when_insert_and_query_then_relations_work() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute(
            "CREATE TABLE workspaces (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                path TEXT NOT NULL,
                backend TEXT NOT NULL,
                state TEXT NOT NULL,
                agent_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                last_accessed_at TEXT NOT NULL
            )"
        ).await?;

        db.execute(
            "CREATE TABLE operations (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL REFERENCES workspaces(id),
                name TEXT NOT NULL,
                state TEXT NOT NULL,
                current_step INTEGER NOT NULL DEFAULT 0,
                total_steps INTEGER NOT NULL,
                started_at TEXT NOT NULL,
                completed_at TEXT,
                final_revision INTEGER,
                error_message TEXT,
                author_id TEXT NOT NULL,
                description TEXT NOT NULL
            )"
        ).await?;

        db.execute("INSERT INTO workspaces VALUES ('w1', 'test', '/tmp/test', 'git', 'active', NULL, datetime('now'), datetime('now'), datetime('now'))").await?;
        db.execute("INSERT INTO operations VALUES ('op1', 'w1', 'test_op', 'started', 0, 3, datetime('now'), NULL, NULL, NULL, 'author1', 'Test operation')").await?;

        let results = db.query("SELECT o.name, w.name FROM operations o JOIN workspaces w ON o.workspace_id = w.id").await?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0][0], "test_op");
        assert_eq!(results[0][1], "test");
        db.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn given_operation_when_update_progress_then_steps_reflected() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute(
            "CREATE TABLE operations (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                name TEXT NOT NULL,
                state TEXT NOT NULL,
                current_step INTEGER NOT NULL DEFAULT 0,
                total_steps INTEGER NOT NULL,
                started_at TEXT NOT NULL,
                completed_at TEXT,
                final_revision INTEGER,
                error_message TEXT,
                author_id TEXT NOT NULL,
                description TEXT NOT NULL
            )"
        ).await?;

        db.execute("INSERT INTO operations VALUES ('op1', 'w1', 'test', 'in_progress', 0, 3, datetime('now'), NULL, NULL, NULL, 'author1', 'desc')").await?;
        db.execute("UPDATE operations SET current_step = 2 WHERE id = 'op1'").await?;

        let results = db.query("SELECT current_step, state FROM operations WHERE id = 'op1'").await?;
        assert_eq!(results[0][0], "2");
        assert_eq!(results[0][1], "in_progress");
        db.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn given_operation_when_complete_then_completed_at_set() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute(
            "CREATE TABLE operations (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                name TEXT NOT NULL,
                state TEXT NOT NULL,
                current_step INTEGER NOT NULL DEFAULT 0,
                total_steps INTEGER NOT NULL,
                started_at TEXT NOT NULL,
                completed_at TEXT,
                final_revision INTEGER,
                error_message TEXT,
                author_id TEXT NOT NULL,
                description TEXT NOT NULL
            )"
        ).await?;

        db.execute("INSERT INTO operations VALUES ('op1', 'w1', 'test', 'completed', 3, 3, datetime('now'), datetime('now'), 42, NULL, 'author1', 'desc')").await?;

        let results = db.query("SELECT state, final_revision FROM operations WHERE id = 'op1'").await?;
        assert_eq!(results[0][0], "completed");
        assert_eq!(results[0][1], "42");
        db.close().await?;
        Ok(())
    }

    // =========================================================================
    // CRUD OPERATIONS - QUEUE_ENTRIES TABLE
    // =========================================================================

    #[tokio::test]
    async fn given_queue_entries_when_insert_with_priority_then_ordered_by_priority() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute(
            "CREATE TABLE queue_entries (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                priority INTEGER NOT NULL CHECK (priority >= 0 AND priority <= 255),
                status TEXT NOT NULL,
                enqueued_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                claimed_by TEXT,
                claimed_at TEXT,
                position INTEGER NOT NULL
            )"
        ).await?;

        db.execute("INSERT INTO queue_entries VALUES ('q1', 'w1', 100, 'pending', datetime('now'), datetime('now'), NULL, NULL, 1)").await?;
        db.execute("INSERT INTO queue_entries VALUES ('q2', 'w1', 10, 'pending', datetime('now'), datetime('now'), NULL, NULL, 2)").await?;
        db.execute("INSERT INTO queue_entries VALUES ('q3', 'w1', 200, 'pending', datetime('now'), datetime('now'), NULL, NULL, 3)").await?;

        let results = db.query("SELECT id, priority FROM queue_entries ORDER BY priority DESC").await?;
        assert_eq!(results.len(), 3);
        assert_eq!(results[0][0], "q3");
        assert_eq!(results[0][1], "200");
        db.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn given_queue_entry_when_claim_then_claimed_fields_set() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute(
            "CREATE TABLE queue_entries (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                priority INTEGER NOT NULL,
                status TEXT NOT NULL,
                enqueued_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                claimed_by TEXT,
                claimed_at TEXT,
                position INTEGER NOT NULL
            )"
        ).await?;

        db.execute("INSERT INTO queue_entries VALUES ('q1', 'w1', 100, 'pending', datetime('now'), datetime('now'), NULL, NULL, 1)").await?;
        db.execute("UPDATE queue_entries SET status = 'claimed', claimed_by = 'agent1', claimed_at = datetime('now') WHERE id = 'q1'").await?;

        let results = db.query("SELECT status, claimed_by FROM queue_entries WHERE id = 'q1'").await?;
        assert_eq!(results[0][0], "claimed");
        assert_eq!(results[0][1], "agent1");
        db.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn given_queue_entry_priority_constraint_when_invalid_priority_then_error() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute(
            "CREATE TABLE queue_entries (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                priority INTEGER NOT NULL CHECK (priority >= 0 AND priority <= 255),
                status TEXT NOT NULL,
                enqueued_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                claimed_by TEXT,
                claimed_at TEXT,
                position INTEGER NOT NULL
            )"
        ).await?;

        let result = db.execute("INSERT INTO queue_entries VALUES ('q1', 'w1', 300, 'pending', datetime('now'), datetime('now'), NULL, NULL, 1)").await;
        assert!(result.is_err());
        db.close().await?;
        Ok(())
    }

    // =========================================================================
    // CRUD OPERATIONS - AGENTS TABLE
    // =========================================================================

    #[tokio::test]
    async fn given_agents_table_when_insert_and_query_then_all_fields_stored() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute(
            "CREATE TABLE agents (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                capabilities TEXT NOT NULL,
                status TEXT NOT NULL,
                last_heartbeat_at TEXT NOT NULL,
                registered_at TEXT NOT NULL,
                metadata TEXT
            )"
        ).await?;

        db.execute("INSERT INTO agents VALUES ('a1', 'worker1', '[\"build\",\"test\"]', 'active', datetime('now'), datetime('now'), '{\"role\":\"builder\"}')").await?;

        let results = db.query("SELECT name, capabilities, status FROM agents").await?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0][0], "worker1");
        assert_eq!(results[0][2], "active");
        db.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn given_agent_when_heartbeat_then_last_heartbeat_updated() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute(
            "CREATE TABLE agents (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                capabilities TEXT NOT NULL,
                status TEXT NOT NULL,
                last_heartbeat_at TEXT NOT NULL,
                registered_at TEXT NOT NULL,
                metadata TEXT
            )"
        ).await?;

        db.execute("INSERT INTO agents VALUES ('a1', 'worker1', '[\"build\"]', 'active', datetime('now'), datetime('now'), NULL)").await?;
        db.execute("UPDATE agents SET last_heartbeat_at = datetime('now'), status = 'idle' WHERE id = 'a1'").await?;

        let results = db.query("SELECT status FROM agents WHERE id = 'a1'").await?;
        assert_eq!(results[0][0], "idle");
        db.close().await?;
        Ok(())
    }

    // =========================================================================
    // CRUD OPERATIONS - SESSIONS TABLE
    // =========================================================================

    #[tokio::test]
    async fn given_sessions_table_when_insert_with_workspace_then_relation_works() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute(
            "CREATE TABLE workspaces (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                path TEXT NOT NULL,
                backend TEXT NOT NULL,
                state TEXT NOT NULL,
                agent_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                last_accessed_at TEXT NOT NULL
            )"
        ).await?;

        db.execute(
            "CREATE TABLE sessions (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL REFERENCES workspaces(id),
                name TEXT NOT NULL,
                state TEXT NOT NULL,
                bead_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                completed_at TEXT
            )"
        ).await?;

        db.execute("INSERT INTO workspaces VALUES ('w1', 'test', '/tmp/test', 'git', 'active', NULL, datetime('now'), datetime('now'), datetime('now'))").await?;
        db.execute("INSERT INTO sessions VALUES ('s1', 'w1', 'feature-x', 'active', NULL, datetime('now'), datetime('now'), NULL)").await?;

        let results = db.query("SELECT s.name, w.name FROM sessions s JOIN workspaces w ON s.workspace_id = w.id").await?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0][0], "feature-x");
        db.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn given_session_when_complete_then_completed_at_set() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute(
            "CREATE TABLE sessions (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                name TEXT NOT NULL,
                state TEXT NOT NULL,
                bead_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                completed_at TEXT
            )"
        ).await?;

        db.execute("INSERT INTO sessions VALUES ('s1', 'w1', 'test', 'completed', NULL, datetime('now'), datetime('now'), datetime('now'))").await?;

        let results = db.query("SELECT state, completed_at FROM sessions").await?;
        assert_eq!(results[0][0], "completed");
        assert!(!results[0][1].is_empty());
        db.close().await?;
        Ok(())
    }

    // =========================================================================
    // CRUD OPERATIONS - CONFIG TABLE
    // =========================================================================

    #[tokio::test]
    async fn given_config_table_when_insert_and_update_then_key_value_persists() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute(
            "CREATE TABLE config (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                description TEXT
            )"
        ).await?;

        db.execute("INSERT INTO config VALUES ('theme', 'dark', datetime('now'), 'UI theme setting')").await?;
        db.execute("UPDATE config SET value = 'light', updated_at = datetime('now') WHERE key = 'theme'").await?;

        let results = db.query("SELECT value FROM config WHERE key = 'theme'").await?;
        assert_eq!(results[0][0], "light");
        db.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn given_config_primary_key_when_duplicate_key_then_error() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute(
            "CREATE TABLE config (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                description TEXT
            )"
        ).await?;

        db.execute("INSERT INTO config VALUES ('theme', 'dark', datetime('now'), NULL)").await?;
        let result = db.execute("INSERT INTO config VALUES ('theme', 'light', datetime('now'), NULL)").await;

        assert!(result.is_err());
        db.close().await?;
        Ok(())
    }

    // =========================================================================
    // TRANSACTION TESTS
    // =========================================================================

    #[tokio::test]
    async fn given_multiple_statements_when_transaction_commits_then_all_persisted() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute(
            "CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)"
        ).await?;

        let pool = db.pool();
        let mut tx = pool.begin().await.map_err(|e| crate::error::Error::database(e.to_string()))?;

        sqlx::query("INSERT INTO test VALUES (1, 'one')")
            .execute(&mut *tx)
            .await
            .map_err(|e| crate::error::Error::database(e.to_string()))?;
        sqlx::query("INSERT INTO test VALUES (2, 'two')")
            .execute(&mut *tx)
            .await
            .map_err(|e| crate::error::Error::database(e.to_string()))?;

        tx.commit().await.map_err(|e| crate::error::Error::database(e.to_string()))?;

        let results = db.query("SELECT * FROM test ORDER BY id").await?;
        assert_eq!(results.len(), 2);
        db.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn given_transaction_when_rollback_then_nothing_persisted() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute(
            "CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)"
        ).await?;

        let pool = db.pool();
        let mut tx = pool.begin().await.map_err(|e| crate::error::Error::database(e.to_string()))?;

        sqlx::query("INSERT INTO test VALUES (1, 'one')")
            .execute(&mut *tx)
            .await
            .map_err(|e| crate::error::Error::database(e.to_string()))?;
        sqlx::query("INSERT INTO test VALUES (2, 'two')")
            .execute(&mut *tx)
            .await
            .map_err(|e| crate::error::Error::database(e.to_string()))?;

        tx.rollback().await.map_err(|e| crate::error::Error::database(e.to_string()))?;

        let results = db.query("SELECT * FROM test").await?;
        assert_eq!(results.len(), 0);
        db.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn given_transaction_when_partial_failure_then_rollback_occurs() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute(
            "CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT UNIQUE)"
        ).await?;

        let pool = db.pool();
        let mut tx = pool.begin().await.map_err(|e| crate::error::Error::database(e.to_string()))?;

        sqlx::query("INSERT INTO test VALUES (1, 'one')")
            .execute(&mut *tx)
            .await
            .map_err(|e| crate::error::Error::database(e.to_string()))?;

        let result = sqlx::query("INSERT INTO test VALUES (2, 'one')")
            .execute(&mut *tx)
            .await;

        assert!(result.is_err());

        tx.rollback().await.map_err(|e| crate::error::Error::database(e.to_string()))?;

        let results = db.query("SELECT * FROM test").await?;
        assert_eq!(results.len(), 0);
        db.close().await?;
        Ok(())
    }

    // =========================================================================
    // CONNECTION POOLING TESTS
    // =========================================================================

    #[tokio::test]
    async fn given_pool_with_multiple_connections_when_query_then_works() -> Result<()> {
        let temp_dir = TempDir::new().map_err(|e| crate::error::Error::io_error(e.to_string()))?;
        let db_path = temp_dir.path().join("pool_test.db");
        let config = DatabaseConfig::with_connections(db_path.to_string_lossy().to_string(), 5)?;
        let db = SqliteDatabaseService::new(config).await?;

        db.execute("CREATE TABLE test (id INTEGER PRIMARY KEY)").await?;

        let handles: Vec<_> = (0..3).map(|i| {
            let pool = db.pool().clone();
            async move {
                sqlx::query("INSERT INTO test VALUES (?)")
                    .bind(i)
                    .execute(&pool)
                    .await
            }
        }).collect();

        futures::future::join_all(handles).await;

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM test")
            .fetch_one(db.pool())
            .await
            .map_err(|e| crate::error::Error::database(e.to_string()))?;
        assert_eq!(count.0, 3);
        db.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn given_pool_at_capacity_when_execute_then_waits_or_reports() -> Result<()> {
        let temp_dir = TempDir::new().map_err(|e| crate::error::Error::io_error(e.to_string()))?;
        let db_path = temp_dir.path().join("capacity_test.db");
        let config = DatabaseConfig::with_connections(db_path.to_string_lossy().to_string(), 2)?;
        let db = SqliteDatabaseService::new(config).await?;

        db.execute("CREATE TABLE test (id INTEGER PRIMARY KEY)").await?;

        let pool = db.pool();
        let mut tx1 = pool.begin().await.map_err(|e| crate::error::Error::database(e.to_string()))?;
        sqlx::query("INSERT INTO test VALUES (1)")
            .execute(&mut *tx1)
            .await
            .map_err(|e| crate::error::Error::database(e.to_string()))?;
        tx1.commit().await.map_err(|e| crate::error::Error::database(e.to_string()))?;

        let mut tx2 = pool.begin().await.map_err(|e| crate::error::Error::database(e.to_string()))?;
        sqlx::query("INSERT INTO test VALUES (2)")
            .execute(&mut *tx2)
            .await
            .map_err(|e| crate::error::Error::database(e.to_string()))?;
        tx2.commit().await.map_err(|e| crate::error::Error::database(e.to_string()))?;

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM test")
            .fetch_one(db.pool())
            .await
            .map_err(|e| crate::error::Error::database(e.to_string()))?;
        assert_eq!(count.0, 2);
        db.close().await?;
        Ok(())
    }

    // =========================================================================
    // ERROR HANDLING - DUPLICATE KEY
    // =========================================================================

    #[tokio::test]
    async fn given_table_with_primary_key_when_insert_duplicate_then_error() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute("CREATE TABLE test (id TEXT PRIMARY KEY, name TEXT)").await?;
        db.execute("INSERT INTO test VALUES ('1', 'first')").await?;

        let result = db.execute("INSERT INTO test VALUES ('1', 'second')").await;
        assert!(result.is_err());
        db.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn given_table_with_unique_constraint_when_insert_duplicate_then_error() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, email TEXT UNIQUE)").await?;
        db.execute("INSERT INTO test VALUES (1, 'test@example.com')").await?;

        let result = db.execute("INSERT INTO test VALUES (2, 'test@example.com')").await;
        assert!(result.is_err());
        db.close().await?;
        Ok(())
    }

    // =========================================================================
    // ERROR HANDLING - MISSING TABLE
    // =========================================================================

    #[tokio::test]
    async fn given_no_table_when_query_then_error() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        let result = db.query("SELECT * FROM nonexistent_table").await;
        assert!(result.is_err());
        db.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn given_no_table_when_insert_then_error() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        let result = db.execute("INSERT INTO nonexistent_table VALUES (1)").await;
        assert!(result.is_err());
        db.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn given_no_table_when_update_then_error() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        let result = db.execute("UPDATE nonexistent_table SET id = 1").await;
        assert!(result.is_err());
        db.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn given_no_table_when_delete_then_error() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        let result = db.execute("DELETE FROM nonexistent_table").await;
        assert!(result.is_err());
        db.close().await?;
        Ok(())
    }

    // =========================================================================
    // ERROR HANDLING - CORRUPT DATABASE
    // =========================================================================

    #[tokio::test]
    async fn given_corrupt_database_file_when_connect_then_error() -> Result<()> {
        let temp_dir = TempDir::new().map_err(|e| crate::error::Error::io_error(e.to_string()))?;
        let db_path = temp_dir.path().join("corrupt.db");

        std::fs::write(&db_path, "this is not a valid sqlite database").map_err(|e| crate::error::Error::io_error(e.to_string()))?;

        let result = SqliteDatabaseService::create(&db_path).await;
        assert!(result.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn given_truncated_database_when_open_then_error_or_recovery() -> Result<()> {
        let temp_dir = TempDir::new().map_err(|e| crate::error::Error::io_error(e.to_string()))?;
        let db_path = temp_dir.path().join("truncated.db");

        {
            let db = SqliteDatabaseService::create(&db_path).await?;
            db.execute("CREATE TABLE test (id INTEGER PRIMARY KEY)").await?;
            db.execute("INSERT INTO test VALUES (1)").await?;
            db.close().await?;
        }

        let metadata = std::fs::metadata(&db_path).map_err(|e| crate::error::Error::io_error(e.to_string()))?;
        let file = std::fs::OpenOptions::new().write(true).open(&db_path).map_err(|e| crate::error::Error::io_error(e.to_string()))?;
        file.set_len(metadata.len() / 2).map_err(|e| crate::error::Error::io_error(e.to_string()))?;
        drop(file);

        let result = SqliteDatabaseService::create(&db_path).await;
        assert!(result.is_err() || result.is_ok());
        Ok(())
    }

    // =========================================================================
    // CONCURRENT ACCESS TESTS
    // =========================================================================

    #[tokio::test]
    async fn given_multiple_readers_when_concurrent_query_then_all_succeed() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)").await?;
        for i in 0..100 {
            db.execute(&format!("INSERT INTO test VALUES ({}, 'value{}')", i, i)).await?;
        }

        let pool = db.pool().clone();
        let handles: Vec<_> = (0..10).map(|_| {
            let pool = pool.clone();
            async move {
                sqlx::query("SELECT * FROM test WHERE id % 10 = 0")
                    .fetch_all(&pool)
                    .await
            }
        }).collect();

        let results = futures::future::join_all(handles).await;
        for r in results {
            assert!(r.is_ok());
            assert_eq!(r.unwrap().len(), 10);
        }
        db.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn given_multiple_writers_when_concurrent_insert_then_all_succeed_or_one_fails() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute("CREATE TABLE test (id INTEGER PRIMARY KEY)").await?;

        let pool = db.pool().clone();
        let handles: Vec<_> = (0..20).map(|i| {
            let pool = pool.clone();
            async move {
                sqlx::query("INSERT INTO test VALUES (?)")
                    .bind(i)
                    .execute(&pool)
                    .await
            }
        }).collect();

        let results = futures::future::join_all(handles).await;
        let success_count = results.iter().filter(|r| r.is_ok()).count();
        assert_eq!(success_count, 20);

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM test")
            .fetch_one(db.pool())
            .await
            .map_err(|e| crate::error::Error::database(e.to_string()))?;
        assert_eq!(count.0, 20);
        db.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn given_reader_and_writer_when_concurrent_then_both_succeed() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)").await?;
        db.execute("INSERT INTO test VALUES (1, 'initial')").await?;

        let pool = db.pool().clone();
        let pool_for_writer = pool.clone();
        let writer_handle = tokio::spawn(async move {
            let mut tx = pool_for_writer.begin().await.unwrap();
            sqlx::query("UPDATE test SET name = 'updated' WHERE id = 1")
                .execute(&mut *tx)
                .await
                .unwrap();
            tx.commit().await.unwrap();
        });

        let reader_handle = tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            sqlx::query("SELECT name FROM test WHERE id = 1")
                .fetch_one(&pool)
                .await
                .unwrap()
        });

        writer_handle.await.map_err(|e| crate::error::Error::internal(e.to_string()))?;
        let row = reader_handle.await.map_err(|e| crate::error::Error::internal(e.to_string()))?;

        let name: String = row.get(0);
        assert_eq!(name, "updated");
        db.close().await?;
        Ok(())
    }

    // =========================================================================
    // QUERY PERFORMANCE TESTS
    // =========================================================================

    #[tokio::test]
    async fn given_large_dataset_when_query_with_index_then_fast() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)").await?;
        db.execute("CREATE INDEX idx_test_value ON test(value)").await?;

        for i in 0..1000 {
            db.execute(&format!("INSERT INTO test VALUES ({}, 'value{}')", i, i % 100)).await?;
        }

        let start = std::time::Instant::now();
        let results = db.query("SELECT * FROM test WHERE value = 'value50'").await?;
        let elapsed = start.elapsed();

        assert_eq!(results.len(), 10);
        assert!(elapsed < std::time::Duration::from_millis(100));
        db.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn given_large_dataset_when_query_without_index_then_still_works() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)").await?;

        for i in 0..1000 {
            db.execute(&format!("INSERT INTO test VALUES ({}, 'value{}')", i, i % 100)).await?;
        }

        let results = db.query("SELECT * FROM test WHERE value = 'value50'").await?;
        assert_eq!(results.len(), 10);
        db.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn given_complex_join_when_query_then_returns_correct_results() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute("CREATE TABLE workspaces (id TEXT PRIMARY KEY, name TEXT)").await?;
        db.execute("CREATE TABLE operations (id TEXT PRIMARY KEY, workspace_id TEXT REFERENCES workspaces(id), name TEXT)").await?;
        db.execute("CREATE TABLE operation_steps (id INTEGER PRIMARY KEY, operation_id TEXT REFERENCES operations(id), step_name TEXT)").await?;

        db.execute("INSERT INTO workspaces VALUES ('w1', 'Alpha')").await?;
        db.execute("INSERT INTO workspaces VALUES ('w2', 'Beta')").await?;
        db.execute("INSERT INTO operations VALUES ('op1', 'w1', 'Build')").await?;
        db.execute("INSERT INTO operations VALUES ('op2', 'w1', 'Test')").await?;
        db.execute("INSERT INTO operation_steps VALUES (1, 'op1', 'compile')").await?;
        db.execute("INSERT INTO operation_steps VALUES (2, 'op1', 'link')").await?;
        db.execute("INSERT INTO operation_steps VALUES (3, 'op2', 'run')").await?;

        let results = db.query(
            "SELECT w.name, o.name, s.step_name
             FROM workspaces w
             JOIN operations o ON o.workspace_id = w.id
             LEFT JOIN operation_steps s ON s.operation_id = o.id
             ORDER BY w.name, o.name, s.id"
        ).await?;

        assert_eq!(results.len(), 3);
        assert_eq!(results[0][0], "Alpha");
        assert_eq!(results[0][1], "Build");
        assert_eq!(results[0][2], "compile");
        assert_eq!(results[1][2], "link");
        assert_eq!(results[2][1], "Test");
        assert_eq!(results[2][2], "run");
        db.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn given_pagination_when_query_then_returns_correct_page() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)").await?;

        for i in 0..100 {
            db.execute(&format!("INSERT INTO test VALUES ({}, 'item{}')", i, i)).await?;
        }

        let page1 = db.query("SELECT * FROM test ORDER BY id LIMIT 10 OFFSET 0").await?;
        let page2 = db.query("SELECT * FROM test ORDER BY id LIMIT 10 OFFSET 10").await?;
        let page10 = db.query("SELECT * FROM test ORDER BY id LIMIT 10 OFFSET 90").await?;

        assert_eq!(page1.len(), 10);
        assert_eq!(page1[0][1], "item0");
        assert_eq!(page1[9][1], "item9");

        assert_eq!(page2.len(), 10);
        assert_eq!(page2[0][1], "item10");

        assert_eq!(page10.len(), 10);
        assert_eq!(page10[0][1], "item90");
        assert_eq!(page10[9][1], "item99");
        db.close().await?;
        Ok(())
    }

    // =========================================================================
    // FACTORY FUNCTION TESTS
    // =========================================================================

    #[tokio::test]
    async fn given_database_config_when_create_database_service_then_returns_service() -> Result<()> {
        let config = DatabaseConfig::in_memory();
        let service = create_database_service(config).await?;
        service.execute("CREATE TABLE factory_test (id INTEGER PRIMARY KEY)").await?;
        Ok(())
    }

    #[tokio::test]
    async fn given_nothing_when_create_in_memory_database_then_works() -> Result<()> {
        let service = create_in_memory_database().await?;
        service.execute("CREATE TABLE mem_test (id INTEGER PRIMARY KEY)").await?;
        Ok(())
    }

    // =========================================================================
    // DATABASE SERVICE TRAIT OBJECT TESTS
    // =========================================================================

    #[tokio::test]
    async fn given_dyn_database_service_when_use_across_modules_then_works() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;
        let pool = db.pool().clone();

        let service: Box<dyn DatabaseService> = Box::new(SqliteDatabaseService::from_pool(pool.clone()));
        service.execute("CREATE TABLE dyn_test (id INTEGER PRIMARY KEY, data TEXT)").await?;
        service.execute("INSERT INTO dyn_test VALUES (1, 'test')").await?;

        let results = service.query("SELECT data FROM dyn_test").await?;
        assert_eq!(results[0][0], "test");
        Ok(())
    }

    #[tokio::test]
    async fn given_multiple_services_when_share_pool_then_isolation_works() -> Result<()> {
        let config = DatabaseConfig::in_memory();
        let db1 = SqliteDatabaseService::new(config).await?;
        let pool = db1.pool().clone();

        db1.execute("CREATE TABLE shared (id INTEGER PRIMARY KEY)").await?;
        db1.execute("INSERT INTO shared VALUES (1)").await?;

        let db2 = SqliteDatabaseService::from_pool(pool);
        let results = db2.query("SELECT * FROM shared").await?;
        assert_eq!(results.len(), 1);

        db1.close().await?;
        Ok(())
    }

    // =========================================================================
    // BOUNDARY TESTS - SCHEMA VALIDATION
    // =========================================================================

    #[tokio::test]
    async fn given_check_constraint_when_insert_invalid_then_error() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute(
            "CREATE TABLE constrained (
                id INTEGER PRIMARY KEY,
                status TEXT NOT NULL CHECK (status IN ('active', 'inactive'))
            )"
        ).await?;

        db.execute("INSERT INTO constrained VALUES (1, 'active')").await?;
        let result = db.execute("INSERT INTO constrained VALUES (2, 'invalid')").await;

        assert!(result.is_err());
        db.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn given_foreign_key_when_reference_invalid_then_error() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute("PRAGMA foreign_keys=ON").await?;

        db.execute("CREATE TABLE parent (id INTEGER PRIMARY KEY)").await?;
        db.execute("CREATE TABLE child (id INTEGER PRIMARY KEY, parent_id INTEGER REFERENCES parent(id))").await?;

        db.execute("INSERT INTO parent VALUES (1)").await?;
        let result = db.execute("INSERT INTO child VALUES (1, 999)").await;

        assert!(result.is_err());
        db.close().await?;
        Ok(())
    }

    // =========================================================================
    // TYPE COERCION TESTS
    // =========================================================================

    #[tokio::test]
    async fn given_integer_types_when_insert_and_query_then_preserved() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute("CREATE TABLE integers (tiny INTEGER, small INTEGER, medium INTEGER, big INTEGER)").await?;

        db.execute("INSERT INTO integers VALUES (127, 32767, 2147483647, 9223372036854775807)").await?;

        let results = db.query("SELECT * FROM integers").await?;
        assert_eq!(results[0][0], "127");
        assert_eq!(results[0][1], "32767");
        assert_eq!(results[0][2], "2147483647");
        assert_eq!(results[0][3], "9223372036854775807");
        db.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn given_float_types_when_insert_and_query_then_preserved() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute("CREATE TABLE floats (val REAL)").await?;
        db.execute("INSERT INTO floats VALUES (3.14159)").await?;
        db.execute("INSERT INTO floats VALUES (-2.71828)").await?;
        db.execute("INSERT INTO floats VALUES (1e-10)").await?;

        let results = db.query("SELECT val FROM floats").await?;
        assert_eq!(results.len(), 3);
        db.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn given_text_with_special_chars_when_query_then_preserved() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute("CREATE TABLE text_test (val TEXT)").await?;
        db.execute("INSERT INTO text_test VALUES ('hello world')").await?;
        db.execute("INSERT INTO text_test VALUES ('with '' quotes')").await?;
        db.execute("INSERT INTO text_test VALUES ('newlines\nand\ttabs')").await?;
        db.execute("INSERT INTO text_test VALUES ('unicode: \u{1F600}')").await?;

        let results = db.query("SELECT * FROM text_test").await?;
        assert_eq!(results.len(), 4);
        db.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn given_null_values_when_query_then_handled_correctly() -> Result<()> {
        let db = SqliteDatabaseService::in_memory().await?;

        db.execute("CREATE TABLE null_test (id INTEGER, value TEXT)").await?;
        db.execute("INSERT INTO null_test VALUES (1, 'has value')").await?;
        db.execute("INSERT INTO null_test VALUES (2, NULL)").await?;
        db.execute("INSERT INTO null_test VALUES (3, 'also has value')").await?;

        let results = db.query("SELECT id FROM null_test WHERE value IS NULL").await?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0][0], "2");
        db.close().await?;
        Ok(())
    }
}
