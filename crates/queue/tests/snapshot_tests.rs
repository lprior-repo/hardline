//! Snapshot tests for queue type JSON serialization.
//!
//! These tests verify that queue types serialize correctly to JSON
//! for persistence and API responses.

use chrono::{DateTime, Utc};
use scp_queue::domain::{
    identifiers::{QueueEntryId, SessionName},
    queue::{entry::QueueEntry, status::QueueStatus, Queue},
};

fn create_test_timestamp() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2024-01-15T10:30:00Z")
        .unwrap()
        .with_timezone(&Utc)
}

#[test]
fn test_queue_entry_pending_json() {
    let entry = QueueEntry::with_status(
        QueueEntryId::new("qe-abc123").unwrap(),
        SessionName::new("feature-test").unwrap(),
        1,
        create_test_timestamp(),
        QueueStatus::Pending,
    )
    .unwrap();
    let json = serde_json::to_string(&entry).unwrap();
    insta::assert_snapshot!("queue_entry_pending", json);
}

#[test]
fn test_queue_entry_claimed_json() {
    let entry = QueueEntry::with_status(
        QueueEntryId::new("qe-def456").unwrap(),
        SessionName::new("bugfix-issue").unwrap(),
        2,
        create_test_timestamp(),
        QueueStatus::Claimed,
    )
    .unwrap();
    let json = serde_json::to_string(&entry).unwrap();
    insta::assert_snapshot!("queue_entry_claimed", json);
}

#[test]
fn test_queue_entry_rebasing_json() {
    let entry = QueueEntry::with_status(
        QueueEntryId::new("qe-ghi789").unwrap(),
        SessionName::new("release-v1").unwrap(),
        0,
        create_test_timestamp(),
        QueueStatus::Rebasing,
    )
    .unwrap();
    let json = serde_json::to_string(&entry).unwrap();
    insta::assert_snapshot!("queue_entry_rebasing", json);
}

#[test]
fn test_queue_entry_testing_json() {
    let entry = QueueEntry::with_status(
        QueueEntryId::new("qe-jkl012").unwrap(),
        SessionName::new("feature-auth").unwrap(),
        3,
        create_test_timestamp(),
        QueueStatus::Testing,
    )
    .unwrap();
    let json = serde_json::to_string(&entry).unwrap();
    insta::assert_snapshot!("queue_entry_testing", json);
}

#[test]
fn test_queue_entry_ready_to_merge_json() {
    let entry = QueueEntry::with_status(
        QueueEntryId::new("qe-mno345").unwrap(),
        SessionName::new("hotfix-security").unwrap(),
        0,
        create_test_timestamp(),
        QueueStatus::ReadyToMerge,
    )
    .unwrap();
    let json = serde_json::to_string(&entry).unwrap();
    insta::assert_snapshot!("queue_entry_ready_to_merge", json);
}

#[test]
fn test_queue_entry_merged_json() {
    let entry = QueueEntry::with_status(
        QueueEntryId::new("qe-pqr678").unwrap(),
        SessionName::new("feature-complete").unwrap(),
        5,
        create_test_timestamp(),
        QueueStatus::Merged,
    )
    .unwrap();
    let json = serde_json::to_string(&entry).unwrap();
    insta::assert_snapshot!("queue_entry_merged", json);
}

#[test]
fn test_queue_entry_failed_retryable_json() {
    let entry = QueueEntry::with_status(
        QueueEntryId::new("qe-stu901").unwrap(),
        SessionName::new("feature-failing").unwrap(),
        10,
        create_test_timestamp(),
        QueueStatus::FailedRetryable,
    )
    .unwrap();
    let json = serde_json::to_string(&entry).unwrap();
    insta::assert_snapshot!("queue_entry_failed_retryable", json);
}

#[test]
fn test_queue_entry_cancelled_json() {
    let entry = QueueEntry::with_status(
        QueueEntryId::new("qe-vwx234").unwrap(),
        SessionName::new("feature-cancelled").unwrap(),
        7,
        create_test_timestamp(),
        QueueStatus::Cancelled,
    )
    .unwrap();
    let json = serde_json::to_string(&entry).unwrap();
    insta::assert_snapshot!("queue_entry_cancelled", json);
}

#[test]
fn test_queue_status_serialization() {
    let statuses = vec![
        (QueueStatus::Pending, "pending"),
        (QueueStatus::Claimed, "claimed"),
        (QueueStatus::Rebasing, "rebasing"),
        (QueueStatus::Testing, "testing"),
        (QueueStatus::ReadyToMerge, "ready_to_merge"),
        (QueueStatus::Merging, "merging"),
        (QueueStatus::Merged, "merged"),
        (QueueStatus::FailedRetryable, "failed_retryable"),
        (QueueStatus::FailedTerminal, "failed_terminal"),
        (QueueStatus::Cancelled, "cancelled"),
    ];

    for (status, name) in statuses {
        let json = serde_json::to_string(&status).unwrap();
        insta::assert_snapshot!(format!("queue_status_{}", name), json);
    }
}

#[test]
fn test_queue_empty_json() {
    let queue = Queue::new();
    let json = serde_json::to_string(&queue).unwrap();
    insta::assert_snapshot!("queue_empty", json);
}

#[test]
fn test_queue_with_entries_json() {
    let entries = vec![
        QueueEntry::with_status(
            QueueEntryId::new("qe-001").unwrap(),
            SessionName::new("high-priority").unwrap(),
            0,
            create_test_timestamp(),
            QueueStatus::Pending,
        )
        .unwrap(),
        QueueEntry::with_status(
            QueueEntryId::new("qe-002").unwrap(),
            SessionName::new("medium-priority").unwrap(),
            5,
            create_test_timestamp(),
            QueueStatus::Claimed,
        )
        .unwrap(),
        QueueEntry::with_status(
            QueueEntryId::new("qe-003").unwrap(),
            SessionName::new("low-priority").unwrap(),
            10,
            create_test_timestamp(),
            QueueStatus::Pending,
        )
        .unwrap(),
    ];
    let queue = Queue::from_entries(entries);
    let json = serde_json::to_string(&queue).unwrap();
    insta::assert_snapshot!("queue_with_entries", json);
}
