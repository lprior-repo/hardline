//! Serde Roundtrip Validation Tests for scp-session
//!
//! This file tests that Serialize/Deserialize implementations correctly
//! validate data during deserialization, ensuring that invalid data
//! cannot be deserialized.

use scp_session::domain::entities::session::{
    BranchState, SessionId, SessionState,
};
use scp_session::domain::events::{
    deserialize_event, serialize_event, SessionEvent,
};
use scp_session::domain::value_objects::session::{
    BeadId as VoBeadId, SessionName, WorkspaceId as VoWorkspaceId,
};

#[test]
fn serde_session_state_roundtrip_all_variants() {
    let variants = [
        (SessionState::Created, r#""created""#),
        (SessionState::Active, r#""active""#),
        (SessionState::Syncing, r#""syncing""#),
        (SessionState::Synced, r#""synced""#),
        (SessionState::Paused, r#""paused""#),
        (SessionState::Completed, r#""completed""#),
        (SessionState::Failed, r#""failed""#),
    ];
    for (state, expected_json) in variants {
        let json = serde_json::to_string(&state).expect("serialize");
        assert_eq!(json, expected_json, "SessionState {:?} serializes correctly", state);
        let parsed: SessionState = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(state, parsed, "SessionState roundtrip preserves value");
    }
}

#[test]
fn serde_branch_state_roundtrip_detached() {
    let state = BranchState::Detached;
    let json = serde_json::to_string(&state).expect("serialize");
    let parsed: BranchState = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(state, parsed);
    assert_eq!(parsed.branch_name(), None);
}

#[test]
fn serde_branch_state_roundtrip_on_branch() {
    let state = BranchState::OnBranch { name: "main".into() };
    let json = serde_json::to_string(&state).expect("serialize");
    let parsed: BranchState = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(state, parsed);
    assert_eq!(parsed.branch_name(), Some("main"));
}

#[test]
fn serde_branch_state_roundtrip_with_special_chars() {
    let state = BranchState::OnBranch { name: "feature/test".into() };
    let json = serde_json::to_string(&state).expect("serialize");
    let parsed: BranchState = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(state, parsed);
    assert_eq!(parsed.branch_name(), Some("feature/test"));
}

#[test]
fn serde_session_id_roundtrip() {
    let id = SessionId::parse("session-abc123").expect("valid");
    let json = serde_json::to_string(&id).expect("serialize");
    let parsed: SessionId = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(id, parsed);
    assert_eq!(id.as_str(), parsed.as_str());
}

#[test]
fn serde_session_name_roundtrip() {
    let name = SessionName::parse("test-session").expect("valid");
    let json = serde_json::to_string(&name).expect("serialize");
    let parsed: SessionName = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(name, parsed);
    assert_eq!(name.as_str(), parsed.as_str());
}

#[test]
fn serde_workspace_id_roundtrip() {
    let id = VoWorkspaceId::parse("ws-test-123").expect("valid");
    let json = serde_json::to_string(&id).expect("serialize");
    let parsed: VoWorkspaceId = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(id, parsed);
    assert_eq!(id.as_str(), parsed.as_str());
}

#[test]
fn serde_bead_id_roundtrip() {
    let id = VoBeadId::parse("bd-abc123def").expect("valid");
    let json = serde_json::to_string(&id).expect("serialize");
    let parsed: VoBeadId = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(id, parsed);
    assert_eq!(id.as_str(), parsed.as_str());
}

#[test]
fn serde_session_event_roundtrip_all_variants() {
    let events = [
        SessionEvent::Activated,
        SessionEvent::Syncing,
        SessionEvent::Synced,
        SessionEvent::Paused,
        SessionEvent::Completed,
        SessionEvent::Failed,
    ];
    for event in events {
        let json = serialize_event(&event).expect("serialize");
        let parsed = deserialize_event(&json).expect("deserialize");
        assert_eq!(event, parsed, "SessionEvent roundtrip failed for {:?}", event);
    }
}

#[test]
fn serde_session_state_rejects_invalid_variant() {
    let json = r#""invalid_state""#;
    let result: Result<SessionState, _> = serde_json::from_str(json);
    assert!(result.is_err(), "Invalid enum variant should be rejected");
}

#[test]
fn serde_branch_state_repr_untagged() {
    let state = BranchState::Detached;
    let json = serde_json::to_string(&state).expect("serialize");
    assert_eq!(json, r#""Detached""#, "Detached should serialize to 'Detached'");
    let parsed: BranchState = serde_json::from_str(&json).expect("deserialize");
    assert!(matches!(parsed, BranchState::Detached));
}

#[test]
fn serde_branch_state_on_branch_serialization() {
    let state = BranchState::OnBranch { name: "main".into() };
    let json = serde_json::to_string(&state).expect("serialize");
    assert_eq!(json, r#"{"OnBranch":{"name":"main"}}"#);
    let parsed: BranchState = serde_json::from_str(&json).expect("deserialize");
    assert!(matches!(parsed, BranchState::OnBranch { name } if name == "main"));
}

#[test]
fn session_event_serialize_returns_valid_json() {
    for event in [
        SessionEvent::Activated,
        SessionEvent::Syncing,
        SessionEvent::Synced,
        SessionEvent::Paused,
        SessionEvent::Completed,
        SessionEvent::Failed,
    ] {
        let json = serialize_event(&event).expect("serialize");
        assert!(json.starts_with('"'));
        assert!(json.ends_with('"'));
    }
}
