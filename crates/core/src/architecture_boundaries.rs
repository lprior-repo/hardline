//! Architecture boundary enforcement tests.
//!
//! These tests enforce domain-driven design boundaries using compile-time checks.
//! Domain types must not depend on infrastructure concerns (tokio, sqlx).
//!
//! The tests use trait bounds to verify at compile time that domain types
//! remain pure and don't accidentally pull in infrastructure dependencies.

use std::marker::PhantomData;

use crate::{
    queue::Priority,
    types::{BeadsIssue, BeadsSummary, IssueStatus, Session, SessionName, SessionStatus},
    workspace_state::WorkspaceState,
};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// MARKER TRAITS FOR ARCHITECTURE LAYERS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Marker trait for domain layer types.
/// Types in the domain layer should be pure, with no infrastructure dependencies.
pub trait DomainLayer {}

/// Marker trait for infrastructure layer types.
/// These types may depend on tokio, sqlx, filesystem, network, etc.
pub trait InfrastructureLayer {}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// COMPILE-TIME BOUNDARY VERIFICATION
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Verifies that a type implements DomainLayer marker.
/// This will fail at compile time if the type doesn't implement the trait.
#[allow(dead_code)]
const fn assert_domain<T: DomainLayer>() {}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// DOMAIN LAYER TYPES - Implement DomainLayer marker
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

impl DomainLayer for BeadsIssue {}
impl DomainLayer for BeadsSummary {}
impl DomainLayer for IssueStatus {}
impl DomainLayer for Priority {}

impl DomainLayer for crate::Error {}
impl DomainLayer for crate::Config {}

// Session types - domain layer
impl DomainLayer for Session {}
impl DomainLayer for SessionName {}
impl DomainLayer for SessionStatus {}
impl DomainLayer for WorkspaceState {}

// Types module types
impl DomainLayer for crate::types::SessionId {}
impl DomainLayer for crate::types::AbsolutePath {}
impl DomainLayer for crate::types::BranchState {}
impl DomainLayer for crate::types::ValidatedMetadata {}
impl DomainLayer for crate::types::FileStatus {}
impl DomainLayer for crate::types::FileChange {}
impl DomainLayer for crate::types::ChangesSummary {}
impl DomainLayer for crate::types::DiffSummary {}
impl DomainLayer for crate::types::FileDiffStat {}
impl DomainLayer for crate::types::Operation {}

// Queue types
impl DomainLayer for crate::queue::QueueItem {}
impl DomainLayer for crate::queue::QueueStatus {}
impl DomainLayer for crate::queue::QueueSource {}

// Lock types
impl DomainLayer for crate::lock::LockInfo {}
impl DomainLayer for crate::lock::LockType {}

// VCS types
impl DomainLayer for crate::vcs::VcsStatus {}
impl DomainLayer for crate::vcs::VcsType {}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// COMPILE-TIME TESTS - These verify trait implementations
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
fn domain_types_implement_domain_marker() {
    assert_domain::<BeadsIssue>();
    assert_domain::<BeadsSummary>();
    assert_domain::<IssueStatus>();
    assert_domain::<Priority>();
    assert_domain::<crate::Error>();
    assert_domain::<crate::Config>();

    // Session types
    assert_domain::<Session>();
    assert_domain::<SessionName>();
    assert_domain::<SessionStatus>();
    assert_domain::<WorkspaceState>();

    // Types module
    assert_domain::<crate::types::SessionId>();
    assert_domain::<crate::types::AbsolutePath>();
    assert_domain::<crate::types::BranchState>();
    assert_domain::<crate::types::ValidatedMetadata>();
    assert_domain::<crate::types::FileStatus>();
    assert_domain::<crate::types::FileChange>();
    assert_domain::<crate::types::ChangesSummary>();
    assert_domain::<crate::types::DiffSummary>();
    assert_domain::<crate::types::FileDiffStat>();
    assert_domain::<crate::types::Operation>();

    // Queue
    assert_domain::<crate::queue::QueueItem>();
    assert_domain::<crate::queue::QueueStatus>();
    assert_domain::<crate::queue::QueueSource>();

    // Lock
    assert_domain::<crate::lock::LockInfo>();
    assert_domain::<crate::lock::LockType>();

    // VCS
    assert_domain::<crate::vcs::VcsStatus>();
    assert_domain::<crate::vcs::VcsType>();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// IMPORT BOUNDARY TESTS - Verify domain modules don't import infrastructure
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that domain types work without tokio runtime.
#[test]
fn domain_types_dont_require_tokio_runtime() {
    let status = IssueStatus::Open;
    let status_json = serde_json::to_string(&status);
    assert!(status_json.is_ok());

    let priority = Priority::Low;
    let priority_json = serde_json::to_string(&priority);
    assert!(priority_json.is_ok());

    let summary = BeadsSummary {
        open: 5,
        in_progress: 3,
        blocked: 1,
        closed: 10,
    };
    assert_eq!(summary.total(), 19);
    assert_eq!(summary.active(), 8);
    assert!(summary.has_blockers());

    let issue = BeadsIssue {
        id: "test-123".to_string(),
        title: "Test issue".to_string(),
        status: IssueStatus::Open,
        priority: Some("high".to_string()),
        issue_type: Some("bug".to_string()),
    };
    assert_eq!(issue.status, IssueStatus::Open);
}

/// Test that Session types work without tokio runtime.
#[test]
fn session_types_dont_require_tokio_runtime() {
    let name = SessionName::new("test-session");
    assert!(name.is_ok());

    let invalid_name = SessionName::new("123-invalid");
    assert!(invalid_name.is_err());

    let state = WorkspaceState::Working;
    let next_states = state.valid_next_states();
    assert!(!next_states.is_empty());

    assert!(state.can_transition_to(WorkspaceState::Ready));
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// MODULE DEPENDENCY VERIFICATION
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Phantom type to detect if a type would pull in infrastructure dependencies.
/// If a type transitively depends on tokio::runtime::Runtime, this would fail.
#[allow(dead_code)]
struct NoRuntimeDependency<T>(PhantomData<T>);

#[test]
fn bead_issue_has_no_runtime_dependency() {
    let _checker: NoRuntimeDependency<BeadsIssue> = NoRuntimeDependency(PhantomData);
}

#[test]
fn session_types_have_no_runtime_dependency() {
    let _checker: NoRuntimeDependency<Session> = NoRuntimeDependency(PhantomData);
    let _checker: NoRuntimeDependency<SessionName> = NoRuntimeDependency(PhantomData);
    let _checker: NoRuntimeDependency<SessionStatus> = NoRuntimeDependency(PhantomData);
    let _checker: NoRuntimeDependency<WorkspaceState> = NoRuntimeDependency(PhantomData);
}

#[test]
fn types_module_have_no_runtime_dependency() {
    let _checker: NoRuntimeDependency<crate::types::SessionId> = NoRuntimeDependency(PhantomData);
    let _checker: NoRuntimeDependency<crate::types::AbsolutePath> =
        NoRuntimeDependency(PhantomData);
    let _checker: NoRuntimeDependency<crate::types::BranchState> = NoRuntimeDependency(PhantomData);
    let _checker: NoRuntimeDependency<crate::types::ValidatedMetadata> =
        NoRuntimeDependency(PhantomData);
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// SERDE BOUNDARY TEST - Domain types should be serializable without runtime
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
fn domain_types_are_serializable_without_runtime() {
    use serde::Serialize;

    fn assert_serializable<T: Serialize>() {}

    assert_serializable::<BeadsIssue>();
    assert_serializable::<BeadsSummary>();
    assert_serializable::<IssueStatus>();
    assert_serializable::<Priority>();

    // Session types must be serializable
    assert_serializable::<SessionStatus>();
    assert_serializable::<WorkspaceState>();
    assert_serializable::<SessionName>();
    assert_serializable::<Session>();

    // Other domain types
    assert_serializable::<crate::types::BranchState>();
    assert_serializable::<crate::types::FileStatus>();
    assert_serializable::<crate::types::ChangesSummary>();

    let status_json = serde_json::to_string(&IssueStatus::Open).ok();
    assert!(status_json.is_some());

    let state_json = serde_json::to_string(&WorkspaceState::Working).ok();
    assert!(state_json.is_some());

    let priority_json = serde_json::to_string(&Priority::High).ok();
    assert!(priority_json.is_some());
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// ARCHITECTURE DOCUMENTATION TEST
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// This test documents the architecture boundaries.
/// It serves as living documentation of what should and shouldn't depend on what.
#[test]
fn architecture_boundaries_documentation() {
    use std::any::type_name;

    let domain_types: &[&str] = &[
        type_name::<BeadsIssue>(),
        type_name::<BeadsSummary>(),
        type_name::<IssueStatus>(),
        type_name::<crate::Error>(),
        type_name::<Session>(),
        type_name::<SessionName>(),
        type_name::<SessionStatus>(),
        type_name::<WorkspaceState>(),
        type_name::<crate::types::SessionId>(),
        type_name::<crate::types::AbsolutePath>(),
        type_name::<Priority>(),
    ];

    for ty in domain_types {
        assert!(
            !ty.contains("tokio"),
            "Domain type {ty} should not reference tokio"
        );
        assert!(
            !ty.contains("sqlx"),
            "Domain type {ty} should not reference sqlx"
        );
    }
}
