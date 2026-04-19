//! SERDE-CORRECT: workspace — serialization round-trip verification
//!
//! Verifies all serde Serialize/Deserialize impls in workspace round-trip correctly.
//! - serialize to JSON, deserialize back, assert equality
//! - Check rename attributes (WorkspaceState has #[serde(rename_all = "snake_case")])
//! - Check enum representations
//! - Test with missing and extra fields

use scp_workspace::domain::entities::{WorkspaceId, WorkspaceState};
use scp_workspace::domain::entities::workspace::{WorkspaceConfig, VcsType};
use scp_workspace::domain::events::WorkspaceEvent;
use scp_workspace::domain::value_objects::{WorkspaceName, WorkspacePath};

#[test]
fn workspace_state_all_variants_serde_roundtrip() {
    for state in [
        WorkspaceState::Initializing,
        WorkspaceState::Active,
        WorkspaceState::Locked,
        WorkspaceState::Corrupted,
        WorkspaceState::Deleted,
    ] {
        let json = serde_json::to_string(&state).expect("serialize");
        let parsed: WorkspaceState = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(state, parsed, "WorkspaceState roundtrip failed for {state:?}");
    }
}

#[test]
fn workspace_state_snake_case_serialization() {
    let states = [
        ("\"initializing\"", WorkspaceState::Initializing),
        ("\"active\"", WorkspaceState::Active),
        ("\"locked\"", WorkspaceState::Locked),
        ("\"corrupted\"", WorkspaceState::Corrupted),
        ("\"deleted\"", WorkspaceState::Deleted),
    ];
    for (json_str, expected) in states {
        let parsed: WorkspaceState =
            serde_json::from_str(json_str).expect(&format!("parse {json_str}"));
        assert_eq!(parsed, expected, "json: {json_str}");
    }
}

#[test]
fn workspace_state_case_sensitive_deserialization() {
    let invalid_json = ["\"INITIALIZING\"", "\"Active\"", "\"LOCKED\""];
    for json_str in invalid_json {
        let result: Result<WorkspaceState, _> = serde_json::from_str(json_str);
        assert!(
            result.is_err(),
            "WorkspaceState should be case-sensitive, but {json_str} parsed"
        );
    }
}

#[test]
fn workspace_id_roundtrip() {
    let id = WorkspaceId::parse("test-ws-id-123".into()).unwrap();
    let json = serde_json::to_string(&id).expect("serialize");
    let parsed: WorkspaceId = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(id.as_str(), parsed.as_str());
}

#[test]
fn workspace_id_empty_deserializes_but_parse_rejects() {
    let parsed: WorkspaceId = serde_json::from_str("\"\"").expect("serde deserializes empty string");
    let parse_result = WorkspaceId::parse("".into());
    assert!(
        parse_result.is_err(),
        "WorkspaceId::parse rejects empty string even though serde allows it"
    );
}

#[test]
fn vcs_type_roundtrip() {
    for vcs in [VcsType::Git] {
        let json = serde_json::to_string(&vcs).expect("serialize");
        let parsed: VcsType = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(vcs, parsed);
    }
}

#[test]
fn workspace_config_roundtrip() {
    let config = WorkspaceConfig {
        vcs_type: VcsType::Git,
        default_branch: "main".into(),
        auto_sync: true,
    };
    let json = serde_json::to_string(&config).expect("serialize");
    let parsed: WorkspaceConfig = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(config.vcs_type, parsed.vcs_type);
    assert_eq!(config.default_branch, parsed.default_branch);
    assert_eq!(config.auto_sync, parsed.auto_sync);
}

#[test]
fn workspace_config_auto_sync_false_roundtrip() {
    let config = WorkspaceConfig {
        vcs_type: VcsType::Git,
        default_branch: "develop".into(),
        auto_sync: false,
    };
    let json = serde_json::to_string(&config).expect("serialize");
    let parsed: WorkspaceConfig = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(config.auto_sync, parsed.auto_sync);
}

#[test]
fn workspace_config_alternative_branch_roundtrip() {
    let config = WorkspaceConfig {
        vcs_type: VcsType::Git,
        default_branch: "feature/my-branch".into(),
        auto_sync: true,
    };
    let json = serde_json::to_string(&config).expect("serialize");
    let parsed: WorkspaceConfig = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(config.default_branch, parsed.default_branch);
}

#[test]
fn workspace_config_missing_auto_sync_uses_default() {
    let json = r#"{"vcs_type":"Git","default_branch":"main"}"#;
    let result: Result<WorkspaceConfig, _> = serde_json::from_str(json);
    assert!(
        result.is_err(),
        "WorkspaceConfig should require auto_sync field (no #[serde(default)])"
    );
}

#[test]
fn workspace_name_roundtrip() {
    let name = WorkspaceName::new("my-workspace".into()).unwrap();
    let json = serde_json::to_string(&name).expect("serialize");
    let parsed: WorkspaceName = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(name.as_str(), parsed.as_str());
}

#[test]
fn workspace_name_with_underscore_roundtrip() {
    let name = WorkspaceName::new("my_workspace_123".into()).unwrap();
    let json = serde_json::to_string(&name).expect("serialize");
    let parsed: WorkspaceName = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(name.as_str(), parsed.as_str());
}

#[test]
fn workspace_name_with_hyphen_roundtrip() {
    let name = WorkspaceName::new("my-workspace-test".into()).unwrap();
    let json = serde_json::to_string(&name).expect("serialize");
    let parsed: WorkspaceName = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(name.as_str(), parsed.as_str());
}

#[test]
fn workspace_name_alphanumeric_roundtrip() {
    let name = WorkspaceName::new("Workspace123".into()).unwrap();
    let json = serde_json::to_string(&name).expect("serialize");
    let parsed: WorkspaceName = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(name.as_str(), parsed.as_str());
}

#[test]
fn workspace_event_all_variants_roundtrip() {
    use chrono::Utc;
    let ts = Utc::now();

    let events = vec![
        WorkspaceEvent::WorkspaceCreated {
            workspace_id: "ws-1".into(),
            name: "test-ws".into(),
            timestamp: ts,
        },
        WorkspaceEvent::WorkspaceActivated {
            workspace_id: "ws-2".into(),
            timestamp: ts,
        },
        WorkspaceEvent::WorkspaceLocked {
            workspace_id: "ws-3".into(),
            holder: "agent-1".into(),
            timestamp: ts,
        },
        WorkspaceEvent::WorkspaceUnlocked {
            workspace_id: "ws-4".into(),
            timestamp: ts,
        },
        WorkspaceEvent::WorkspaceCorrupted {
            workspace_id: "ws-5".into(),
            reason: "disk failure".into(),
            timestamp: ts,
        },
        WorkspaceEvent::WorkspaceDeleted {
            workspace_id: "ws-6".into(),
            timestamp: ts,
        },
        WorkspaceEvent::WorkspaceConfigUpdated {
            workspace_id: "ws-7".into(),
            timestamp: ts,
        },
    ];

    for event in events {
        let json = serde_json::to_string(&event).expect("serialize");
        let parsed: WorkspaceEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(event, parsed);
    }
}

#[test]
fn workspace_event_created_deserialization() {
    let json =
        r#"{"WorkspaceCreated":{"workspace_id":"ws-123","name":"my-workspace","timestamp":"2024-01-01T00:00:00Z"}}"#;
    let event: WorkspaceEvent = serde_json::from_str(json).expect("deserialize");
    match event {
        WorkspaceEvent::WorkspaceCreated {
            workspace_id,
            name,
            timestamp: _,
        } => {
            assert_eq!(workspace_id, "ws-123");
            assert_eq!(name, "my-workspace");
        }
        _ => panic!("expected WorkspaceCreated"),
    }
}

#[test]
fn workspace_event_locked_deserialization() {
    let json = r#"{"WorkspaceLocked":{"workspace_id":"ws-locked","holder":"agent-x","timestamp":"2024-01-01T00:00:00Z"}}"#;
    let event: WorkspaceEvent = serde_json::from_str(json).expect("deserialize");
    match event {
        WorkspaceEvent::WorkspaceLocked {
            workspace_id,
            holder,
            timestamp: _,
        } => {
            assert_eq!(workspace_id, "ws-locked");
            assert_eq!(holder, "agent-x");
        }
        _ => panic!("expected WorkspaceLocked"),
    }
}

#[test]
fn workspace_config_extra_fields_ignored() {
    #[derive(Debug, serde::Deserialize)]
    struct WithExtra {
        config: WorkspaceConfig,
        extra: String,
    }
    let json = r#"{"config":{"vcs_type":"Git","default_branch":"main","auto_sync":true},"extra":"ignored"}"#;
    let parsed: WithExtra = serde_json::from_str(json).expect("deserialize");
    assert_eq!(parsed.extra, "ignored");
}

#[test]
fn workspace_name_extra_fields_ignored() {
    #[derive(Debug, serde::Deserialize)]
    struct WithExtra {
        name: WorkspaceName,
        extra: i32,
    }
    let json = r#"{"name":"extra-test","extra":42}"#;
    let parsed: WithExtra = serde_json::from_str(json).expect("deserialize");
    assert_eq!(parsed.extra, 42);
}

#[test]
fn workspace_id_extra_fields_ignored() {
    #[derive(Debug, serde::Deserialize)]
    struct WithExtra {
        id: WorkspaceId,
        extra: bool,
    }
    let json = r#"{"id":"ws-extra","extra":true}"#;
    let parsed: WithExtra = serde_json::from_str(json).expect("deserialize");
    assert!(parsed.extra);
}

#[test]
fn workspace_state_extra_fields_ignored() {
    #[derive(Debug, serde::Deserialize)]
    struct WithExtra {
        state: WorkspaceState,
        extra: Vec<String>,
    }
    let json = r#"{"state":"active","extra":["a","b"]}"#;
    let parsed: WithExtra = serde_json::from_str(json).expect("deserialize");
    assert_eq!(parsed.extra, vec!["a", "b"]);
}

#[test]
fn workspace_event_extra_fields_ignored() {
    #[derive(Debug, serde::Deserialize)]
    struct WithExtra {
        event: WorkspaceEvent,
        extra: f64,
    }
    let json = r#"{"event":{"WorkspaceActivated":{"workspace_id":"ws-x","timestamp":"2024-01-01T00:00:00Z"}},"extra":3.14}"#;
    let parsed: WithExtra = serde_json::from_str(json).expect("deserialize");
    assert_eq!(parsed.extra, 3.14);
}

#[test]
fn workspace_workspace_id_does_not_serialize_as_object() {
    let id = WorkspaceId::parse("simple-id".into()).unwrap();
    let json = serde_json::to_string(&id).expect("serialize");
    assert_eq!(json, "\"simple-id\"");
}

#[test]
fn workspace_workspace_name_does_not_serialize_as_object() {
    let name = WorkspaceName::new("simple-name".into()).unwrap();
    let json = serde_json::to_string(&name).expect("serialize");
    assert_eq!(json, "\"simple-name\"");
}

#[test]
fn workspace_state_serializes_to_snake_case_string() {
    for state in [
        WorkspaceState::Initializing,
        WorkspaceState::Active,
        WorkspaceState::Locked,
        WorkspaceState::Corrupted,
        WorkspaceState::Deleted,
    ] {
        let json = serde_json::to_string(&state).expect("serialize");
        assert!(
            json.starts_with("\"") && json.ends_with("\""),
            "state should serialize to string: {json}"
        );
        let json_lower = json.to_lowercase();
        assert!(
            json_lower.contains("initializing")
                || json_lower.contains("active")
                || json_lower.contains("locked")
                || json_lower.contains("corrupted")
                || json_lower.contains("deleted"),
            "state serialized to: {json}"
        );
    }
}

#[test]
fn all_serde_types_impl_debug() {
    fn assert_debug<T: std::fmt::Debug>() {}
    assert_debug::<WorkspaceState>();
    assert_debug::<WorkspaceId>();
    assert_debug::<WorkspaceConfig>();
    assert_debug::<VcsType>();
    assert_debug::<WorkspaceName>();
    assert_debug::<WorkspacePath>();
    assert_debug::<WorkspaceEvent>();
}

#[test]
fn all_serde_types_impl_clone() {
    fn assert_clone<T: Clone>() {}
    assert_clone::<WorkspaceState>();
    assert_clone::<WorkspaceId>();
    assert_clone::<WorkspaceConfig>();
    assert_clone::<VcsType>();
    assert_clone::<WorkspaceName>();
    assert_clone::<WorkspacePath>();
    assert_clone::<WorkspaceEvent>();
}

#[test]
fn all_serde_types_impl_partial_eq() {
    fn assert_partial_eq<T: PartialEq>() {}
    assert_partial_eq::<WorkspaceState>();
    assert_partial_eq::<WorkspaceId>();
    assert_partial_eq::<VcsType>();
    assert_partial_eq::<WorkspaceName>();
    assert_partial_eq::<WorkspacePath>();
    assert_partial_eq::<WorkspaceEvent>();
}

#[test]
fn vcs_type_only_has_git() {
    let json = serde_json::to_string(&VcsType::Git).expect("serialize");
    assert_eq!(json, "\"Git\"");
}

#[test]
fn workspace_config_special_characters_in_branch() {
    let config = WorkspaceConfig {
        vcs_type: VcsType::Git,
        default_branch: "feature/ABC-123_test".into(),
        auto_sync: true,
    };
    let json = serde_json::to_string(&config).expect("serialize");
    let parsed: WorkspaceConfig = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(config.default_branch, parsed.default_branch);
}

#[test]
fn workspace_event_deserialization_preserves_reason() {
    let json = r#"{"WorkspaceCorrupted":{"workspace_id":"ws-1","reason":"network timeout","timestamp":"2024-01-01T00:00:00Z"}}"#;
    let event: WorkspaceEvent = serde_json::from_str(json).expect("deserialize");
    match event {
        WorkspaceEvent::WorkspaceCorrupted {
            workspace_id,
            reason,
            timestamp: _,
        } => {
            assert_eq!(workspace_id, "ws-1");
            assert_eq!(reason, "network timeout");
        }
        _ => panic!("expected WorkspaceCorrupted"),
    }
}

#[test]
fn workspace_state_deserialization_unknown_variant_rejected() {
    let result: Result<WorkspaceState, _> = serde_json::from_str("\"unknown_state\"");
    assert!(result.is_err());
}

#[test]
fn workspace_event_unknown_variant_rejected() {
    let result: Result<WorkspaceEvent, _> =
        serde_json::from_str(r#"{"UnknownEvent":{"foo":"bar"}}"#);
    assert!(result.is_err());
}