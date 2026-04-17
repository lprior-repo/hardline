//! Issue aggregate root - tests.
//!
//! This module contains unit tests for the `Issue` aggregate root.

use crate::beads::domain::DomainError;
use crate::beads::issue::Issue;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_valid_id_and_title_when_new_then_id_is_set() {
        let issue = Issue::new("test-1", "Test Issue").unwrap();
        assert_eq!(issue.id.as_str(), "test-1");
    }

    #[test]
    fn given_valid_id_and_title_when_new_then_title_is_set() {
        let issue = Issue::new("test-1", "Test Issue").unwrap();
        assert_eq!(issue.title.as_str(), "Test Issue");
    }

    #[test]
    fn given_valid_id_and_title_when_new_then_state_is_active() {
        let issue = Issue::new("test-1", "Test Issue").unwrap();
        assert!(issue.is_active());
    }

    #[test]
    fn given_valid_id_and_title_when_new_then_state_is_not_closed() {
        let issue = Issue::new("test-1", "Test Issue").unwrap();
        assert!(!issue.is_closed());
    }

    #[test]
    fn given_open_issue_when_close_then_state_is_closed() {
        let mut issue = Issue::new("test-1", "Test Issue").unwrap();
        issue.close();
        assert!(issue.is_closed());
    }

    #[test]
    fn given_open_issue_when_close_then_closed_at_is_set() {
        let mut issue = Issue::new("test-1", "Test Issue").unwrap();
        issue.close();
        assert!(issue.closed_at().is_some());
    }

    #[test]
    fn given_closed_issue_when_reopen_then_state_is_not_closed() {
        let mut issue = Issue::new("test-1", "Test Issue").unwrap();
        issue.close();
        issue.reopen().unwrap();
        assert!(!issue.is_closed());
    }

    #[test]
    fn given_closed_issue_when_reopen_then_state_is_active() {
        let mut issue = Issue::new("test-1", "Test Issue").unwrap();
        issue.close();
        issue.reopen().unwrap();
        assert!(issue.is_active());
    }

    #[test]
    fn given_issue_when_set_blocked_by_then_is_blocked_is_true() {
        let mut issue = Issue::new("test-1", "Test Issue").unwrap();
        issue.set_blocked_by(vec!["blocker-1".to_string()]).unwrap();
        assert!(issue.is_blocked());
    }

    #[test]
    fn given_empty_id_when_new_then_returns_empty_id_error() {
        let result = Issue::new("", "Test Issue");
        assert!(matches!(result, Err(DomainError::EmptyId)));
    }

    #[test]
    fn given_empty_title_when_new_then_returns_empty_title_error() {
        let result = Issue::new("test-1", "");
        assert!(matches!(result, Err(DomainError::EmptyTitle)));
    }
}
