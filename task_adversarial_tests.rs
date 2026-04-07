// Comprehensive adversarial tests for task subcommands
// Covering: claim race conditions, yield without claim, double-done,
// invalid state transitions, special characters, long descriptions, SQL injection

use proptest::prelude::*;
use std::sync::Arc;
use tokio::time::sleep;

use hardline::commands::task_types::{Task, TaskId, Title, Assignee, Priority, TaskState};
use hardline::commands::task_validation::*;
use hardline::commands::task_store::TaskStore;
use hardline::commands::handlers::task::data::{TaskCommand, AgentId, TaskStatusOutput};
use hardline::commands::handlers::task::calculations::*;

// Test helpers
fn create_open_task(id: &str, title: &str) -> Task {
    Task::new(
        TaskId::new(id).unwrap(),
        Title::new(title),
    )
}

fn create_claimed_task(id: &str, assignee: &str) -> Task {
    let task = create_open_task(id, assignee);
    transition_to_claimed(task, assignee)
}

fn create_in_progress_task(id: &str, assignee: &str) -> Task {
    let task = create_claimed_task(id, assignee);
    transition_to_started(task)
}

fn create_closed_task(id: &str, assignee: &str) -> Task {
    let task = create_in_progress_task(id, assignee);
    transition_to_done(task)
}

// === CLAIM RACE CONDITION TESTS ===

#[tokio::test]
async fn test_claim_race_condition_two_agents() {
    let store = Arc::new(TaskStore::new());
    let task = create_open_task("race-task", "Race condition test");
    store.insert(task).unwrap();
    
    let task_id = TaskId::new("race-task").unwrap();
    let agent1 = AgentId::new("agent-1").unwrap();
    let agent2 = AgentId::new("agent-2").unwrap();
    
    // Simulate concurrent claims
    let claim1 = claim_task(store.clone(), &task_id, &agent1);
    let claim2 = claim_task(store.clone(), &task_id, &agent2);
    
    // Wait for both to complete
    let (result1, result2) = tokio::join!(claim1, claim2);
    
    // Exactly one should succeed
    let success_count = [result1.is_ok(), result2.is_ok()].iter().filter(|&&r| r).count();
    assert_eq!(success_count, 1);
}

#[tokio::test]
async fn test_claim_many_agents_contention() {
    let store = Arc::new(TaskStore::new());
    let task = create_open_task("contention-task", "Many agents contend");
    store.insert(task).unwrap();
    
    let task_id = TaskId::new("contention-task").unwrap();
    let mut handles = vec![];
    
    // Spawn 10 concurrent claim attempts
    for i in 1..=10 {
        let store_clone = store.clone();
        let task_id_clone = task_id.clone();
        let agent = AgentId::new(&format!("agent-{}", i)).unwrap();
        
        let handle = tokio::spawn(async move {
            claim_task(store_clone, &task_id_clone, &agent).await
        });
        handles.push(handle);
    }
    
    // Collect results
    let results = futures::future::join_all(handles).await;
    let success_count = results.into_iter().filter(|r| r.is_ok()).count();
    
    // Exactly one should succeed
    assert_eq!(success_count, 1);
}

// === YIELD WITHOUT CLAIM TESTS ===

#[tokio::test]
async fn test_yield_open_task() {
    let store = Arc::new(TaskStore::new());
    let task = create_open_task("yield-open", "Open task");
    store.insert(task).unwrap();
    
    let task_id = TaskId::new("yield-open").unwrap();
    let agent = AgentId::new("tester").unwrap();
    
    // Try to yield an unclaimed task
    let result = yield_task(store.clone(), &task_id, &agent).await;
    
    // Should fail
    assert!(result.is_err());
}

#[tokio::test]
async fn test_yield_different_agent_task() {
    let store = Arc::new(TaskStore::new());
    let task = create_claimed_task("yield-diff", "Task claimed by other", "other-agent");
    store.insert(task).unwrap();
    
    let task_id = TaskId::new("yield-diff").unwrap();
    let agent = AgentId::new("tester").unwrap();
    
    // Try to yield a task claimed by someone else
    let result = yield_task(store.clone(), &task_id, &agent).await;
    
    // Should fail
    assert!(result.is_err());
}

// === DOUBLE-DONE TESTS ===

#[tokio::test]
async fn test_double_done_same_agent() {
    let store = Arc::new(TaskStore::new());
    let task = create_in_progress_task("double-done", "Task to complete twice", "tester");
    store.insert(task).unwrap();
    
    let task_id = TaskId::new("double-done").unwrap();
    let agent = AgentId::new("tester").unwrap();
    
    // First done should succeed
    let result1 = complete_task(store.clone(), &task_id, &agent).await;
    assert!(result1.is_ok());
    
    // Second done should fail
    let result2 = complete_task(store.clone(), &task_id, &agent).await;
    assert!(result2.is_err());
}

#[tokio::test]
async fn test_double_done_different_agents() {
    let store = Arc::new(TaskStore::new());
    let task = create_in_progress_task("double-done-diff", "Task completed by different agents", "agent1");
    store.insert(task).unwrap();
    
    let task_id = TaskId::new("double-done-diff").unwrap();
    
    // First agent completes
    let result1 = complete_task(store.clone(), &task_id, &AgentId::new("agent1").unwrap()).await;
    assert!(result1.is_ok());
    
    // Second agent tries to complete
    let result2 = complete_task(store.clone(), &task_id, &AgentId::new("agent2").unwrap()).await;
    assert!(result2.is_err());
}

// === INVALID STATE TRANSITION TESTS ===

#[tokio::test]
async fn test_transition_from_closed_to_in_progress() {
    let store = Arc::new(TaskStore::new());
    let task = create_closed_task("closed-to-progress", "Already closed task", "tester");
    store.insert(task).unwrap();
    
    let task_id = TaskId::new("closed-to-progress").unwrap();
    let agent = AgentId::new("tester").unwrap();
    
    // Try to start a closed task
    let result = start_task(store.clone(), &task_id, &agent).await;
    
    // Should fail
    assert!(result.is_err());
}

#[tokio::test]
async fn test_transition_blocked_to_closed_without_claim() {
    let store = Arc::new(TaskStore::new());
    let mut task = create_open_task("blocked-to-closed", "Blocked task");
    task.state = TaskState::Blocked;
    store.insert(task).unwrap();
    
    let task_id = TaskId::new("blocked-to-closed").unwrap();
    let agent = AgentId::new("tester").unwrap();
    
    // Try to close a blocked task that's not claimed
    let result = complete_task(store.clone(), &task_id, &agent).await;
    
    // Should fail
    assert!(result.is_err());
}

#[test]
fn test_validate_not_closed_all_states() {
    let open_task = create_open_task("open-validate", "Open task");
    assert!(validate_not_closed(&open_task).is_ok());
    
    let progress_task = create_in_progress_task("progress-validate", "In progress task", "tester");
    assert!(validate_not_closed(&progress_task).is_ok());
    
    let mut blocked_task = create_open_task("blocked-validate", "Blocked task");
    blocked_task.state = TaskState::Blocked;
    assert!(validate_not_closed(&blocked_task).is_ok());
    
    let mut deferred_task = create_open_task("deferred-validate", "Deferred task");
    deferred_task.state = TaskState::Deferred;
    assert!(validate_not_closed(&deferred_task).is_ok());
    
    let closed_task = create_closed_task("closed-validate", "Closed task", "tester");
    assert!(validate_not_closed(&closed_task).is_err());
}

// === SPECIAL CHARACTERS TESTS ===

#[test]
fn test_task_id_special_characters() {
    // Test invalid special characters
    let invalid_ids = vec![
        "task@123", "task#001", "task$test", "task%done", "task^foo",
        "task&bar", "task*test", "task(test)", "task)test", "task+test",
        "task=test", "task{test}", "task}test", "task[test]", "task]test",
        "task\\test", "test|test", "task:test", "task;test", "task'test",
        "task\"test", "task<test", "task>test", "task/test", "task\\test"
    ];
    
    for id in invalid_ids {
        let result = TaskId::new(id);
        assert!(result.is_err(), "Task ID '{}' should be rejected", id);
    }
}

#[test]
fn test_task_id_valid_special_chars() {
    // Test valid special characters
    let valid_ids = vec!["task-001", "bead_123", "ABC-123_xyz", "a", "1-2_3"];
    
    for id in valid_ids {
        let result = TaskId::new(id);
        assert!(result.is_ok(), "Task ID '{}' should be accepted", id);
    }
}

#[test]
fn test_title_special_characters() {
    // Titles should allow special characters
    let special_titles = vec![
        "Fix bug: crash on startup!",
        "Implement [feature] for user",
        "Handle error #12345",
        "Update {config} file",
        "Test 'quotes' here",
        "Check \"quotes\" again",
        "Special <chars> & symbols"
    ];
    
    for title in special_titles {
        let title_obj = Title::new(title);
        assert_eq!(title_obj.as_str(), title);
    }
}

// === EXTREMELY LONG DESCRIPTIONS TESTS ===

#[tokio::test]
async fn test_extremely_long_description_claim() {
    let store = Arc::new(TaskStore::new());
    let mut task = create_open_task("long-desc", "Task with very long description");
    
    // Create a very long description (1MB worth)
    let long_desc = "a".repeat(1024 * 1024);
    task.description = Some(long_desc);
    
    store.insert(task).unwrap();
    
    let task_id = TaskId::new("long-desc").unwrap();
    let agent = AgentId::new("tester").unwrap();
    
    // Should handle long descriptions gracefully
    let result = claim_task(store.clone(), &task_id, &agent).await;
    assert!(result.is_ok());
}

#[test]
fn test_truncate_description_edge_cases() {
    assert_eq!(truncate_description("", 0), "");
    assert_eq!(truncate_description("hello", 10), "hello");
    assert_eq!(truncate_description("hello world", 5), "hello");
    assert_eq!(truncate_description("hello", 3), "...");
    assert_eq!(truncate_description("a".repeat(1000), 10), "aaa...");
}

#[test]
fn test_truncate_description_multi_byte() {
    let input = "Hello, world! 🌍 How are you? 🚀";
    let result = truncate_description(input, 20);
    assert!(result.len() <= 20);
    assert!(result.ends_with("...") || result == input);
}

// === SQL INJECTION TESTS ===

#[test]
fn test_sql_injection_task_id() {
    // Test various SQL injection patterns in task ID
    let sql_injection_attempts = vec![
        "'; DROP TABLE tasks; --",
        "' OR '1'='1",
        "' UNION SELECT * FROM users --",
        "' WAITFOR DELAY '0:0:10' --",
        "'; EXEC sp_executesql 'SELECT * FROM users' --",
        "" OR ""="",
        "' OR 1=1 --",
        "' OR 1=1#",
        "' OR 1=1;--",
    ];
    
    for attempt in sql_injection_attempts {
        let result = TaskId::new(attempt);
        // All should be rejected
        assert!(result.is_err(), "SQL injection '{}' should be rejected", attempt);
    }
}

// === PROPT-BASED FUZZY TESTS ===

proptest! {
    // Fuzzy test for task ID validation
    #[test]
    fn fuzzy_task_id_validation(id in "\\PC*") {
        let result = TaskId::new(&id);
        // Most strings should be invalid except those matching [a-zA-Z0-9_-]+
        let valid_pattern = regex::Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap();
        if valid_pattern.is_match(&id) {
            assert!(result.is_ok(), "Valid pattern should succeed: {}", id);
        } else {
            assert!(result.is_err(), "Invalid pattern should fail: {}", id);
        }
    }

    // Fuzzy test for agent ID validation
    #[test]
    fn fuzzy_agent_id_validation(id in "\\PC*") {
        let result = AgentId::new(id);
        // Empty or whitespace-only should fail
        if id.trim().is_empty() {
            assert!(result.is_err(), "Empty agent ID should fail");
        } else {
            assert!(result.is_ok(), "Non-empty agent ID should succeed: {}", id);
        }
    }

    // Fuzzy test for title creation
    #[test]
    fn fuzzy_title_creation(title in "\\PC*") {
        let title_obj = Title::new(title);
        assert_eq!(title_obj.as_str(), title);
    }

    // Fuzzy test for task creation with valid ID
    #[test]
    fn fuzzy_task_creation(
        valid_id in r"[a-zA-Z0-9_-]+",
        title in "\\PC*"
    ) {
        let task_id = TaskId::new(&valid_id).unwrap();
        let task = Task::new(task_id, Title::new(title));
        assert_eq!(task.id.as_str(), valid_id);
        assert_eq!(task.title.as_str(), title);
        assert!(matches!(task.state, TaskState::Open));
    }
}

// === ADDITIONAL ADVERSARIAL TESTS ===

#[tokio::test]
async fn test_claim_expiry_edge_cases() {
    let store = Arc::new(TaskStore::new());
    let task = create_open_task("expiry-test", "Claim expiry test");
    store.insert(task).unwrap();
    
    let task_id = TaskId::new("expiry-test").unwrap();
    
    // Claim with agent1
    let agent1 = AgentId::new("agent1").unwrap();
    let result1 = claim_task(store.clone(), &task_id, &agent1).await;
    assert!(result1.is_ok());
    
    // Immediately try to claim with agent2 (should fail)
    let agent2 = AgentId::new("agent2").unwrap();
    let result2 = claim_task(store.clone(), &task_id, &agent2).await;
    assert!(result2.is_err());
    
    // Yield back to open
    let result_yield = yield_task(store.clone(), &task_id, &agent1).await;
    assert!(result_yield.is_ok());
    
    // Now claim should succeed for agent2
    let result3 = claim_task(store.clone(), &task_id, &agent2).await;
    assert!(result3.is_ok());
}

#[tokio::test]
async fn test_done_without_start() {
    let store = Arc::new(TaskStore::new());
    let task = create_claimed_task("no-start", "Task claimed but not started", "tester");
    store.insert(task).unwrap();
    
    let task_id = TaskId::new("no-start").unwrap();
    let agent = AgentId::new("tester").unwrap();
    
    // Try to complete a task that's claimed but not in progress
    let result = complete_task(store.clone(), &task_id, &agent).await;
    
    // Should fail because task is not in progress
    assert!(result.is_err());
}

#[test]
fn test_priority_injection_attempts() {
    // Test various injection attempts in priority fields
    let injection_attempts = vec![
        "\"; DROP TABLE tasks; --",
        "' OR '1'='1",
        "UNION SELECT * FROM users",
        "SCRIPT>alert('xss')</SCRIPT>",
        "<img src=x onerror=alert(1)>",
        "javascript:alert(1)",
        "data:text/html,<script>alert(1)</script>",
        "file:///etc/passwd",
        "ftp://attacker.com/malicious",
        "1; DROP TABLE users;--",
    ];
    
    for attempt in injection_attempts {
        let priority = Priority::new(attempt);
        // Priority should accept any string but we can test the output
        assert_eq!(priority.as_str(), attempt);
    }
}

#[tokio::test]
async fn test_concurrent_state_transitions() {
    let store = Arc::new(TaskStore::new());
    let task = create_open_task("concurrent-transitions", "Concurrent transition test");
    store.insert(task).unwrap();
    
    let task_id = TaskId::new("concurrent-transitions").unwrap();
    let agent = AgentId::new("tester").unwrap();
    
    // Concurrent claim and start operations
    let claim = claim_task(store.clone(), &task_id, &agent);
    let start = start_task(store.clone(), &task_id, &agent);
    
    let (claim_result, start_result) = tokio::join!(claim, start);
    
    // Both should succeed
    assert!(claim_result.is_ok());
    assert!(start_result.is_ok());
    
    // Now try to yield and complete concurrently
    let yield_task = yield_task(store.clone(), &task_id, &agent);
    let complete = complete_task(store.clone(), &task_id, &agent);
    
    let (yield_result, complete_result) = tokio::join!(yield_task, complete);
    
    // Yield should succeed, complete should fail
    assert!(yield_result.is_ok());
    assert!(complete_result.is_err());
}

#[test]
fn test_task_id_unicode_injection() {
    // Test various Unicode sequences
    let unicode_attempts = vec![
        "task\\u0021",  // Unicode escape for !
        "task\\x21",     // Hex escape
        "task\\n",       // Newline
        "task\\t",       // Tab
        "task\\r\\n",    // CRLF
        "task\\u2028",   // Line separator
        "task\\u2029",   // Paragraph separator
        "task\\0",       // Null byte
        "task\\x00",     // Null byte hex
        "🚀task",        // Emoji prefix
        "task🚀",        // Emoji suffix
        "タスク",        // Non-ASCII characters
        "taskनाम",      // Mixed script
    ];
    
    for attempt in unicode_attempts {
        let result = TaskId::new(attempt);
        // These should all be rejected except basic alphanumeric with -_
        let is_valid = attempt.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_');
        if is_valid {
            assert!(result.is_ok(), "Valid Unicode task ID should succeed: {}", attempt);
        } else {
            assert!(result.is_err(), "Invalid Unicode task ID should fail: {}", attempt);
        }
    }
}

#[tokio::test]
async fn test_lock_contention_scenarios() {
    let store = Arc::new(TaskStore::new());
    let task = create_open_task("lock-contention", "Lock contention test");
    store.insert(task).unwrap();
    
    let task_id = TaskId::new("lock-contention").unwrap();
    
    // Multiple concurrent operations
    let mut handles = vec![];
    
    for i in 0..5 {
        let store_clone = store.clone();
        let task_id_clone = task_id.clone();
        let agent = AgentId::new(&format!("agent-{}", i)).unwrap();
        
        let handle = tokio::spawn(async move {
            // Each agent tries to claim then immediately yield
            let _ = claim_task(store_clone.clone(), &task_id_clone, &agent).await;
            yield_task(store_clone, &task_id_clone, &agent).await
        });
        handles.push(handle);
    }
    
    // Wait for all operations
    let results = futures::future::join_all(handles).await;
    
    // Count successful claims
    let successful_claims = results.iter().filter(|r| r.is_ok()).count();
    assert_eq!(successful_claims, 1);
}

#[test]
fn test_title_xss_vectors() {
    // Test various XSS vectors in titles
    let xss_vectors = vec![
        "<script>alert('XSS')</script>",
        "<img src='x' onerror='alert(1)'>",
        "<svg onload=alert(1)>",
        "javascript:alert(1)",
        "data:text/html,<script>alert(1)</script>",
        "<iframe src='javascript:alert(1)'>",
        "<body onload=alert(1)>",
        "<div onmouseover='alert(1)'>hover me</div>",
        "<a href='javascript:alert(1)'>click</a>",
        "<input onfocus='alert(1)' autofocus>",
    ];
    
    for xss in xss_vectors {
        let title = Title::new(xss);
        // Title should accept any string content
        assert_eq!(title.as_str(), xss);
    }
}

#[tokio::test]
async fn test_memory_pressure_long_descriptions() {
    let store = Arc::new(TaskStore::new());
    
    // Create multiple tasks with extremely long descriptions
    for i in 0..10 {
        let mut task = create_open_task(&format!("long-desc-{}", i), "Task with huge description");
        
        // Create description that's 100KB
        let long_desc = "a".repeat(100 * 1024);
        task.description = Some(long_desc);
        
        store.insert(task).unwrap();
    }
    
    // Try to claim one of them
    let task_id = TaskId::new("long-desc-0").unwrap();
    let agent = AgentId::new("tester").unwrap();
    
    let result = claim_task(store.clone(), &task_id, &agent).await;
    assert!(result.is_ok(), "Should handle long descriptions without crashing");
}

#[test]
fn test_task_state_serialization_edge_cases() {
    // Test serialization/deserialization edge cases
    let test_cases = vec![
        (TaskState::Open, r#""open""#),
        (TaskState::InProgress, r#""in_progress""#),
        (TaskState::Blocked, r#""blocked""#),
        (TaskState::Deferred, r#""deferred""#),
    ];
    
    for (state, expected_json) in test_cases {
        let json = serde_json::to_string(&state).unwrap();
        let deserialized: TaskState = serde_json::from_str(&json).unwrap();
        
        match (state, deserialized) {
            (TaskState::Open, TaskState::Open) => (),
            (TaskState::InProgress, TaskState::InProgress) => (),
            (TaskState::Blocked, TaskState::Blocked) => (),
            (TaskState::Deferred, TaskState::Deferred) => (),
            _ => panic!("Serialization roundtrip failed for {:?}", state),
        }
    }
}

// === END OF ADVERSARIAL TESTS ===
