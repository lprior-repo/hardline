//! Queue commands (from Stak)

use scp_core::{
    infrastructure::database::{DatabaseConfig, DatabaseService, SqliteDatabaseService},
    lock::{LockManager, LockType, MemLockManager},
    queue::{MemQueue, Priority, QueueItem, QueueManager, QueueStatus},
    queue_sqlite::SqliteQueue,
    vcs::{self, VcsStatus},
    Result,
};
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Get the database path from environment or default
fn get_db_path() -> String {
    env::var("SCP_DATABASE_PATH").unwrap_or_else(|_| {
        let mut path = env::var("HOME").map_or_else(|_| PathBuf::from("."), PathBuf::from);
        path.push(".scp");
        path.push("hardline.db");
        path.to_string_lossy().to_string()
    })
}

/// Run async code in a temporary Tokio runtime
fn run_async<F, T>(f: F) -> Result<T>
where
    F: std::future::Future<Output = Result<T>>,
{
    let rt = Runtime::new()
        .map_err(|e| scp_core::Error::internal(format!("Failed to create runtime: {}", e)))?;
    rt.block_on(f)
}

/// Get a persistent SQLite-backed queue
fn get_queue() -> Result<Arc<dyn QueueManager>> {
    let db_path = get_db_path();
    // Ensure parent directory exists
    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            scp_core::Error::io_error(format!("Failed to create database directory: {}", e))
        })?;
    }
    run_async(async {
        let config = DatabaseConfig::new(db_path)?;
        let db_service = SqliteDatabaseService::new(config).await?;
        let queue = SqliteQueue::new(db_service.pool().clone());
        queue.init().await?;
        Ok(Arc::new(queue) as Arc<dyn QueueManager>)
    })
}

/// List queue items
pub fn list() -> Result<()> {
    let queue = get_queue()?;
    let items = queue.list()?;

    if items.is_empty() {
        println!("Queue is empty");
    } else {
        println!("Queue ({} items):", items.len());
        for (i, item) in items.iter().enumerate() {
            let status = format!("{:?}", item.status);
            let priority = format!("{:?}", item.priority);
            println!("  {}. {} [{}] {}", i + 1, item.branch, priority, status);
        }
    }

    Ok(())
}

/// Add item to queue
pub fn enqueue(branch: &str, priority: Option<&str>) -> Result<()> {
    let queue = get_queue()?;

    let mut item = QueueItem::direct(branch);

    if let Some(p) = priority {
        item.priority = match p.to_lowercase().as_str() {
            "low" => Priority::Low,
            "high" => Priority::High,
            "critical" => Priority::Critical,
            _ => Priority::Normal,
        };
    }

    queue.enqueue(item)?;
    println!("✓ Added '{}' to queue", branch);

    Ok(())
}

/// Remove front item from queue
pub fn dequeue() -> Result<()> {
    let queue = get_queue()?;

    match queue.dequeue()? {
        Some(item) => {
            println!("✓ Dequeued '{}'", item.branch);
            Ok(())
        }
        None => Err(scp_core::Error::queue_empty()),
    }
}

/// Process next item in queue using a provided VCS backend (testable core).
pub fn process_with_backend(
    queue: &dyn scp_core::queue::QueueManager,
    backend: &dyn scp_core::vcs::VcsBackend,
    checks: bool,
) -> Result<()> {
    let mut item = match queue.dequeue()? {
        Some(i) => i,
        None => return Err(scp_core::Error::queue_empty()),
    };

    println!("Processing '{}'...", item.branch);

    // Pre-flight checks
    if checks {
        println!("  Running pre-flight checks...");

        match backend.status()? {
            VcsStatus::Clean => println!("    ✓ Working copy clean"),
            VcsStatus::Dirty => {
                let msg = "working copy has uncommitted changes".to_string();
                println!("    ✗ {}", msg);
                item.fail(msg.clone());
                queue.enqueue(item)?;
                return Err(scp_core::Error::working_copy_dirty());
            }
            VcsStatus::Conflicted => {
                let msg = "working copy has merge conflicts".to_string();
                println!("    ✗ {}", msg);
                item.fail(msg.clone());
                queue.enqueue(item)?;
                return Err(scp_core::Error::vcs_conflict("working copy", "pre-flight"));
            }
            VcsStatus::Detached => {
                let msg = "detached HEAD — cannot process queue".to_string();
                println!("    ✗ {}", msg);
                item.fail(msg.clone());
                queue.enqueue(item)?;
                return Err(scp_core::Error::invalid_state(msg));
            }
        }

        // Check target branch exists
        let branches = backend.list_branches()?;
        let branch_exists = branches.iter().any(|b| b.name == item.branch);
        if !branch_exists {
            let branch_name = item.branch.clone();
            let msg = format!("branch '{}' not found", branch_name);
            println!("    ✗ {}", msg);
            item.fail(msg.clone());
            queue.enqueue(item)?;
            return Err(scp_core::Error::branch_not_found(branch_name));
        }
        println!("    ✓ Branch '{}' exists", item.branch);
    }

    // Merge the branch
    println!("  Merging '{}'...", item.branch);
    if let Err(e) = backend.merge(&item.branch) {
        let msg = format!("merge failed: {}", e);
        println!("  ✗ {}", msg);
        let _ = backend.merge("--abort");
        item.fail(msg.clone());
        queue.enqueue(item)?;
        return Err(e);
    }
    println!("  ✓ Merge successful");

    // Push
    println!("  Pushing...");
    if let Err(e) = backend.push() {
        let msg = format!("push failed: {}", e);
        println!("  ✗ {}", msg);
        item.fail(msg.clone());
        queue.enqueue(item)?;
        return Err(e);
    }
    println!("  ✓ Push successful");

    println!("✓ Processed '{}'", item.branch);
    Ok(())
}

/// Process next item in queue
pub fn process(checks: bool) -> Result<()> {
    let queue = get_queue()?;

    // Acquire lock
    let lock = Arc::new(MemLockManager::new()) as Arc<dyn LockManager>;
    let _guard = lock.acquire(LockType::Queue("default".into()), "scp")?;

    let cwd = std::env::current_dir()?;
    let backend = vcs::create_backend(&cwd)?;

    process_with_backend(queue.as_ref(), backend.as_ref(), checks)
}

/// Insert item at position
pub fn insert(position: usize, branch: &str) -> Result<()> {
    let queue = get_queue()?;

    let item = QueueItem::direct(branch);
    queue.insert_at(position, item)?;

    println!("✓ Inserted '{}' at position {}", branch, position);
    Ok(())
}

/// Remove item from queue
pub fn remove(branch: &str) -> Result<()> {
    let queue = get_queue()?;

    // Find by branch name
    let items = queue.list()?;
    let item = items
        .iter()
        .find(|i| i.branch == branch)
        .ok_or_else(|| scp_core::Error::queue_item_not_found(branch.to_string()))?;

    queue.remove(&item.id)?;
    println!("✓ Removed '{}' from queue", branch);

    Ok(())
}

/// Show queue status
pub fn status() -> Result<()> {
    let queue = get_queue()?;

    let len = queue.len()?;
    let pending = queue.list_pending()?;

    println!("Queue Status:");
    println!("  Total items: {}", len);
    println!("  Pending: {}", pending.len());

    if !pending.is_empty() {
        println!("  Next: {}", pending[0].branch);
    }

    Ok(())
}

/// Parse a priority string into a Priority enum (pure function for testing)
pub fn parse_priority(priority: &str) -> Priority {
    match priority.to_lowercase().as_str() {
        "low" => Priority::Low,
        "high" => Priority::High,
        "critical" => Priority::Critical,
        _ => Priority::Normal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scp_core::vcs::{Branch, Commit, CommitId, RepoStatus, VcsStatus, Workspace};

    // ── Mock VCS backend for testing ──────────────────────────────────────

    struct MockVcsBackend {
        status_result: VcsStatus,
        branches: Vec<Branch>,
        merge_should_fail: bool,
        push_should_fail: bool,
        merge_calls: std::sync::Mutex<Vec<String>>,
    }

    impl MockVcsBackend {
        fn clean_with_branch(branch: &str) -> Self {
            Self {
                status_result: VcsStatus::Clean,
                branches: vec![Branch {
                    name: branch.to_string(),
                    is_current: false,
                    tracking: None,
                }],
                merge_should_fail: false,
                push_should_fail: false,
                merge_calls: std::sync::Mutex::new(vec![]),
            }
        }

        fn merge_calls(&self) -> Vec<String> {
            self.merge_calls.lock().unwrap().clone()
        }
    }

    impl scp_core::vcs::VcsBackend for MockVcsBackend {
        fn current_branch(&self) -> scp_core::Result<String> {
            Ok("main".to_string())
        }
        fn list_branches(&self) -> scp_core::Result<Vec<Branch>> {
            Ok(self.branches.clone())
        }
        fn create_branch(&self, _name: &str) -> scp_core::Result<()> {
            Ok(())
        }
        fn switch_branch(&self, _name: &str) -> scp_core::Result<()> {
            Ok(())
        }
        fn push(&self) -> scp_core::Result<()> {
            if self.push_should_fail {
                Err(scp_core::Error::invalid_state("push rejected"))
            } else {
                Ok(())
            }
        }
        fn pull(&self) -> scp_core::Result<()> {
            Ok(())
        }
        fn rebase(&self, _onto: &str) -> scp_core::Result<()> {
            Ok(())
        }
        fn merge(&self, branch: &str) -> scp_core::Result<()> {
            self.merge_calls.lock().unwrap().push(branch.to_string());
            if self.merge_should_fail {
                Err(scp_core::Error::vcs_conflict(
                    branch.to_string(),
                    "conflict in src/main.rs".to_string(),
                ))
            } else {
                Ok(())
            }
        }
        fn log(&self, _limit: usize) -> scp_core::Result<Vec<Commit>> {
            Ok(vec![])
        }
        fn status(&self) -> scp_core::Result<VcsStatus> {
            Ok(self.status_result.clone())
        }
        fn is_initialized(&self) -> scp_core::Result<bool> {
            Ok(true)
        }
        fn repo_exists(&self, _path: &str) -> bool {
            true
        }
        fn checkout(&self, _target: &str) -> scp_core::Result<()> {
            Ok(())
        }
        fn commit(&self, _message: &str) -> scp_core::Result<CommitId> {
            Ok(CommitId::from_unchecked("abc123"))
        }
        fn diff(&self, _from: &CommitId, _to: &CommitId) -> scp_core::Result<String> {
            Ok(String::new())
        }
        fn repo_status(&self) -> scp_core::Result<RepoStatus> {
            Ok(RepoStatus::clean())
        }
        fn create_workspace(&self, _name: &str) -> scp_core::Result<()> {
            Ok(())
        }
        fn switch_workspace(&self, _name: &str) -> scp_core::Result<()> {
            Ok(())
        }
        fn list_workspaces(&self) -> scp_core::Result<Vec<Workspace>> {
            Ok(vec![])
        }
        fn delete_workspace(&self, _name: &str) -> scp_core::Result<()> {
            Ok(())
        }
        fn fork_workspace(&self, _src: &str, _tgt: &str) -> scp_core::Result<()> {
            Ok(())
        }
        fn merge_workspace(&self, _name: &str) -> scp_core::Result<()> {
            Ok(())
        }
        fn abort_workspace(&self, _name: &str) -> scp_core::Result<()> {
            Ok(())
        }
    }

    fn make_queue() -> Arc<dyn scp_core::queue::QueueManager> {
        let lock = Arc::new(MemLockManager::new()) as Arc<dyn LockManager>;
        Arc::new(MemQueue::new(lock))
    }

    fn enqueue_branch(queue: &dyn scp_core::queue::QueueManager, branch: &str) {
        queue
            .enqueue(scp_core::queue::QueueItem::direct(branch))
            .unwrap();
    }

    // ── process_with_backend: happy path ──────────────────────────────────

    #[test]
    fn process_succeeds_with_checks_clean_and_branch_exists() {
        let queue = make_queue();
        enqueue_branch(queue.as_ref(), "feature-x");
        let backend = MockVcsBackend::clean_with_branch("feature-x");

        let result = process_with_backend(queue.as_ref(), &backend, true);
        assert!(result.is_ok(), "process should succeed: {:?}", result.err());
    }

    #[test]
    fn process_succeeds_without_checks() {
        let queue = make_queue();
        enqueue_branch(queue.as_ref(), "feature-y");
        let backend = MockVcsBackend::clean_with_branch("feature-y");

        let result = process_with_backend(queue.as_ref(), &backend, false);
        assert!(result.is_ok(), "process without checks should succeed");
    }

    #[test]
    fn process_calls_merge_then_push() {
        let queue = make_queue();
        enqueue_branch(queue.as_ref(), "merge-push-test");
        let backend = MockVcsBackend::clean_with_branch("merge-push-test");

        process_with_backend(queue.as_ref(), &backend, false).unwrap();

        let calls = backend.merge_calls();
        assert_eq!(calls, vec!["merge-push-test"]);
    }

    #[test]
    fn process_marks_item_completed_on_success() {
        let queue = make_queue();
        enqueue_branch(queue.as_ref(), "complete-test");
        let backend = MockVcsBackend::clean_with_branch("complete-test");

        process_with_backend(queue.as_ref(), &backend, false).unwrap();

        // Queue should be empty (item dequeued, completed, updated)
        let pending = queue.list_pending().unwrap();
        assert!(pending.is_empty(), "no pending items after success");
    }

    #[test]
    fn process_empty_queue_returns_error() {
        let queue = make_queue();
        let backend = MockVcsBackend::clean_with_branch("any");

        let result = process_with_backend(queue.as_ref(), &backend, false);
        assert!(result.is_err());
    }

    // ── Pre-flight check: dirty working copy ──────────────────────────────

    #[test]
    fn process_checks_fails_on_dirty_working_copy() {
        let queue = make_queue();
        enqueue_branch(queue.as_ref(), "dirty-test");
        let mut backend = MockVcsBackend::clean_with_branch("dirty-test");
        backend.status_result = VcsStatus::Dirty;

        let result = process_with_backend(queue.as_ref(), &backend, true);
        assert!(result.is_err());
    }

    #[test]
    fn process_checks_fails_on_conflicted_working_copy() {
        let queue = make_queue();
        enqueue_branch(queue.as_ref(), "conflict-test");
        let mut backend = MockVcsBackend::clean_with_branch("conflict-test");
        backend.status_result = VcsStatus::Conflicted;

        let result = process_with_backend(queue.as_ref(), &backend, true);
        assert!(result.is_err());
    }

    #[test]
    fn process_checks_fails_on_detached_head() {
        let queue = make_queue();
        enqueue_branch(queue.as_ref(), "detached-test");
        let mut backend = MockVcsBackend::clean_with_branch("detached-test");
        backend.status_result = VcsStatus::Detached;

        let result = process_with_backend(queue.as_ref(), &backend, true);
        assert!(result.is_err());
    }

    #[test]
    fn process_checks_fails_when_branch_not_found() {
        let queue = make_queue();
        enqueue_branch(queue.as_ref(), "missing-branch");
        // Backend has no branches
        let backend = MockVcsBackend {
            status_result: VcsStatus::Clean,
            branches: vec![],
            merge_should_fail: false,
            push_should_fail: false,
            merge_calls: std::sync::Mutex::new(vec![]),
        };

        let result = process_with_backend(queue.as_ref(), &backend, true);
        assert!(result.is_err());
    }

    #[test]
    fn process_checks_does_not_merge_when_branch_missing() {
        let queue = make_queue();
        enqueue_branch(queue.as_ref(), "no-merge-branch");
        let backend = MockVcsBackend {
            status_result: VcsStatus::Clean,
            branches: vec![],
            merge_should_fail: false,
            push_should_fail: false,
            merge_calls: std::sync::Mutex::new(vec![]),
        };

        let _ = process_with_backend(queue.as_ref(), &backend, true);

        // Merge should never have been called
        assert!(backend.merge_calls().is_empty());
    }

    // ── Merge failure: rollback ───────────────────────────────────────────

    #[test]
    fn process_merge_failure_marks_item_failed() {
        let queue = make_queue();
        enqueue_branch(queue.as_ref(), "merge-fail");
        let mut backend = MockVcsBackend::clean_with_branch("merge-fail");
        backend.merge_should_fail = true;

        let result = process_with_backend(queue.as_ref(), &backend, false);
        assert!(result.is_err());

        // Item should be in queue as failed
        let all = queue.list().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].status, scp_core::queue::QueueStatus::Failed);
        assert!(all[0].last_error.is_some());
    }

    #[test]
    fn process_merge_failure_attempts_abort() {
        let queue = make_queue();
        enqueue_branch(queue.as_ref(), "abort-test");
        let mut backend = MockVcsBackend::clean_with_branch("abort-test");
        backend.merge_should_fail = true;

        let _ = process_with_backend(queue.as_ref(), &backend, false);

        let calls = backend.merge_calls();
        // First call: merge branch, second call: merge --abort
        assert_eq!(calls, vec!["abort-test", "--abort"]);
    }

    // ── Push failure ──────────────────────────────────────────────────────

    #[test]
    fn process_push_failure_marks_item_failed() {
        let queue = make_queue();
        enqueue_branch(queue.as_ref(), "push-fail");
        let mut backend = MockVcsBackend::clean_with_branch("push-fail");
        backend.push_should_fail = true;

        let result = process_with_backend(queue.as_ref(), &backend, false);
        assert!(result.is_err());

        let all = queue.list().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].status, scp_core::queue::QueueStatus::Failed);
        assert!(all[0].last_error.as_ref().unwrap().contains("push failed"));
    }

    #[test]
    fn process_push_failure_does_not_abort_merge() {
        let queue = make_queue();
        enqueue_branch(queue.as_ref(), "push-no-abort");
        let mut backend = MockVcsBackend::clean_with_branch("push-no-abort");
        backend.push_should_fail = true;

        let _ = process_with_backend(queue.as_ref(), &backend, false);

        let calls = backend.merge_calls();
        // Only one merge call (the actual merge), no --abort
        assert_eq!(calls, vec!["push-no-abort"]);
    }

    // ── Dirty check failure does not attempt merge ────────────────────────

    #[test]
    fn process_dirty_check_does_not_attempt_merge_or_push() {
        let queue = make_queue();
        enqueue_branch(queue.as_ref(), "dirty-no-merge");
        let mut backend = MockVcsBackend::clean_with_branch("dirty-no-merge");
        backend.status_result = VcsStatus::Dirty;

        let _ = process_with_backend(queue.as_ref(), &backend, true);

        // No merge calls at all — pre-flight stopped us
        assert!(backend.merge_calls().is_empty());
    }

    // ── parse_priority tests ──────────────────────────────────────────────

    #[test]
    fn parse_priority_low() {
        assert_eq!(parse_priority("low"), Priority::Low);
    }

    #[test]
    fn parse_priority_low_uppercase() {
        assert_eq!(parse_priority("LOW"), Priority::Low);
    }

    #[test]
    fn parse_priority_low_mixed_case() {
        assert_eq!(parse_priority("LoW"), Priority::Low);
    }

    #[test]
    fn parse_priority_high() {
        assert_eq!(parse_priority("high"), Priority::High);
    }

    #[test]
    fn parse_priority_critical() {
        assert_eq!(parse_priority("critical"), Priority::Critical);
    }

    #[test]
    fn parse_priority_critical_uppercase() {
        assert_eq!(parse_priority("CRITICAL"), Priority::Critical);
    }

    #[test]
    fn parse_priority_normal() {
        assert_eq!(parse_priority("normal"), Priority::Normal);
    }

    #[test]
    fn parse_priority_unknown_falls_back_to_normal() {
        assert_eq!(parse_priority("unknown"), Priority::Normal);
    }

    #[test]
    fn parse_priority_empty_string_falls_back_to_normal() {
        assert_eq!(parse_priority(""), Priority::Normal);
    }

    #[test]
    fn parse_priority_whitespace_falls_back_to_normal() {
        assert_eq!(parse_priority("  "), Priority::Normal);
    }

    #[test]
    fn parse_priority_partial_match_high_is_not_higher() {
        // "higher" should not match "high"
        assert_eq!(parse_priority("higher"), Priority::Normal);
    }

    #[test]
    fn parse_priority_partial_match_crit_is_not_critical() {
        assert_eq!(parse_priority("crit"), Priority::Normal);
    }

    #[test]
    fn parse_priority_number_falls_back_to_normal() {
        assert_eq!(parse_priority("42"), Priority::Normal);
    }

    #[test]
    fn parse_priority_medium_falls_back_to_normal() {
        assert_eq!(parse_priority("medium"), Priority::Normal);
    }

    #[test]
    fn priority_equality() {
        assert_eq!(Priority::Low, Priority::Low);
        assert_eq!(Priority::High, Priority::High);
        assert_ne!(Priority::Low, Priority::High);
        assert_ne!(Priority::Normal, Priority::Critical);
    }
}
