//! Tests for the domain module.

use chrono::Utc;

use super::{DomainError, IssueId, IssueState, Labels, Title};

#[test]
fn test_issue_id_valid() {
    assert!(IssueId::new("valid-id-123").is_ok());
    assert!(IssueId::new("valid_id_456").is_ok());
}

#[test]
fn test_issue_id_invalid() {
    assert!(matches!(IssueId::new(""), Err(DomainError::EmptyId)));
    assert!(IssueId::new("invalid id").is_err());
    assert!(IssueId::new("invalid.id").is_err());
}

#[test]
fn test_title_valid() {
    assert!(Title::new("Valid Title").is_ok());
    assert!(Title::new("  Trimmed  ").is_ok());
}

#[test]
fn test_title_invalid() {
    assert!(matches!(Title::new(""), Err(DomainError::EmptyTitle)));
    assert!(Title::new("  ").is_err()); // Trimmed to empty
}

#[test]
fn test_issue_state_closed_has_timestamp() {
    let state = IssueState::Closed {
        closed_at: Utc::now(),
    };
    assert!(state.is_closed());
    assert!(state.closed_at().is_some());
}

#[test]
fn test_issue_state_open_no_timestamp() {
    let state = IssueState::Open;
    assert!(!state.is_closed());
    assert!(state.closed_at().is_none());
    assert!(state.is_active());
}

#[test]
fn test_labels_validation() {
    assert!(Labels::new(vec!["label1".to_string(), "label2".to_string()]).is_ok());

    // Test exceeding max count
    let too_many_labels: Vec<String> = (0..=Labels::MAX_COUNT)
        .map(|i| format!("label{i}"))
        .collect();
    assert!(Labels::new(too_many_labels).is_err());
}
