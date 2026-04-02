//! JSON module tests

use serde::{Deserialize, Serialize};

use crate::error_io::IoErrorKind;
use crate::fix::Fix;
use crate::json::envelope::SchemaEnvelope;
use crate::json::envelope_array::SchemaEnvelopeArray;
use crate::json::error_code::ErrorCode;
use crate::json::error_types::{ErrorDetail, JsonError};
use crate::json::hateoas::{HateoasLink, RelatedResources};
use crate::json::helpers::error_with_available_sessions;
use crate::json::meta::ResponseMeta;
use crate::json::schemas;
use crate::json::serializable::JsonSerializable;
use crate::json::JsonSuccess;

// ═══════════════════════════════════════════════════════════════════════════
// JSON ERROR TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_json_error_basic() {
    let err = JsonError::new("TEST_ERROR", "Test error message");
    assert_eq!(err.error.code, "TEST_ERROR");
    assert_eq!(err.error.message, "Test error message");
    assert!(err.error.details.is_none());
    assert!(err.error.suggestion.is_none());
}

#[test]
fn test_json_error_with_details() {
    let details = serde_json::json!({"key": "value"});
    let err = JsonError::new("TEST_ERROR", "Test").with_details(details.clone());

    assert!(err.error.details.is_some());
    assert_eq!(err.error.details, Some(details));
}

#[test]
fn test_json_error_with_suggestion() {
    let err = JsonError::new("TEST_ERROR", "Test").with_suggestion("Try this instead");

    assert_eq!(err.error.suggestion, Some("Try this instead".to_string()));
}

#[test]
fn test_error_code_as_str() {
    assert_eq!(ErrorCode::SessionNotFound.as_str(), "SESSION_NOT_FOUND");
    assert_eq!(ErrorCode::JjNotInstalled.as_str(), "JJ_NOT_INSTALLED");
    assert_eq!(ErrorCode::HookFailed.as_str(), "HOOK_FAILED");
}

#[test]
fn test_error_code_to_string() {
    let code: String = ErrorCode::SessionNotFound.into();
    assert_eq!(code, "SESSION_NOT_FOUND");
}

#[test]
fn test_json_error_serialization() -> crate::error::Result<()> {
    let err = JsonError::new("TEST_ERROR", "Test message");
    let json = err.to_json()?;

    assert!(json.contains("\"code\""));
    assert!(json.contains("\"message\""));
    assert!(json.contains("TEST_ERROR"));
    assert!(json.contains("Test message"));

    Ok(())
}

#[test]
fn test_error_with_available_sessions() {
    let available = vec!["session1".to_string(), "session2".to_string()];
    let err = error_with_available_sessions(
        ErrorCode::SessionNotFound,
        "Session 'foo' not found",
        "foo",
        &available,
    );

    assert_eq!(err.error.code, "SESSION_NOT_FOUND");
    assert!(err.error.details.is_some());
    assert!(err.error.suggestion.is_some());
}

#[test]
fn test_json_serializable_trait() -> crate::error::Result<()> {
    #[derive(Serialize)]
    struct TestStruct {
        field: String,
    }

    let test = TestStruct {
        field: "value".to_string(),
    };

    let json = test.to_json()?;
    assert!(json.contains("\"field\""));
    assert!(json.contains("\"value\""));

    Ok(())
}

#[test]
fn test_json_success_wrapper() -> crate::error::Result<()> {
    #[derive(Serialize, Deserialize)]
    struct TestData {
        name: String,
        count: usize,
    }

    let data = TestData {
        name: "test".to_string(),
        count: 42,
    };

    let success = JsonSuccess {
        success: true,
        data,
    };
    let json = success.to_json()?;

    assert!(json.contains("\"name\""));
    assert!(json.contains("\"test\""));
    assert!(json.contains("\"count\""));
    assert!(json.contains("42"));

    Ok(())
}

#[test]
fn test_error_detail_skip_none() -> crate::error::Result<()> {
    let err = JsonError::new("TEST", "message");
    let json = err.to_json()?;

    // Should not contain "details" or "suggestion" fields when they're None
    assert!(!json.contains("\"details\""));
    assert!(!json.contains("\"suggestion\""));

    Ok(())
}

#[test]
fn test_error_detail_from_validation_error() {
    let err = crate::error::Error::validation_error("invalid session name");
    let detail = ErrorDetail::from_error(&err);

    assert!(detail.code.contains("VALIDATION") || detail.code.contains("INVALID"));
    assert!(detail.message.contains("Validation"));
}

#[test]
fn test_error_detail_from_io_error() {
    let err = crate::error::Error::io_error("file not found");
    let detail = ErrorDetail::from_error(&err);

    assert!(detail.code.contains("UNKNOWN") || detail.message.contains("IO"));
    assert!(detail.message.contains("file not found"));
    assert_eq!(detail.exit_code, 3);
}

#[test]
fn test_error_detail_from_not_found_error() {
    let err = crate::error::Error::not_found("session not found");
    let detail = ErrorDetail::from_error(&err);

    assert!(detail.code.contains("NOT_FOUND") || detail.code.contains("SESSION"));
    assert!(detail.message.contains("Not found"));
    assert_eq!(detail.exit_code, 2);
}

#[test]
fn test_error_detail_includes_suggestion() {
    let err = crate::error::Error::not_found("session not found");
    let detail = ErrorDetail::from_error(&err);

    // Should have suggestion populated
    assert!(detail.suggestion.is_some());
}

// ═══════════════════════════════════════════════════════════════════════════
// HATEOAS LINK TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_hateoas_link_self() {
    let link = HateoasLink::self_link("scp status test");
    assert_eq!(link.rel, "self");
    assert_eq!(link.href, "scp status test");
    assert_eq!(link.method, Some("GET".to_string()));
    assert!(link.title.is_none());
}

#[test]
fn test_hateoas_link_related() {
    let link = HateoasLink::related("parent", "scp list");
    assert_eq!(link.rel, "parent");
    assert_eq!(link.href, "scp list");
    assert_eq!(link.method, Some("GET".to_string()));
}

#[test]
fn test_hateoas_link_action() {
    let link = HateoasLink::action("remove", "scp remove test", "Delete session");
    assert_eq!(link.rel, "remove");
    assert_eq!(link.href, "scp remove test");
    assert_eq!(link.method, Some("POST".to_string()));
    assert_eq!(link.title, Some("Delete session".to_string()));
}

#[test]
fn test_hateoas_link_with_title() {
    let link = HateoasLink::self_link("scp status").with_title("Get current status");
    assert_eq!(link.title, Some("Get current status".to_string()));
}

#[test]
fn test_hateoas_link_serialization() -> crate::error::Result<()> {
    let link = HateoasLink::action("sync", "scp sync test", "Sync session");
    let json = serde_json::to_string(&link)
        .map_err(|e| crate::error::Error::from(IoErrorKind::JsonParse(e)))?;

    assert!(json.contains("\"rel\":\"sync\""));
    assert!(json.contains("\"href\":\"scp sync test\""));
    assert!(json.contains("\"method\":\"POST\""));
    assert!(json.contains("\"title\":\"Sync session\""));
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// RELATED RESOURCES TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_related_resources_empty() {
    let related = RelatedResources::default();
    assert!(related.is_empty());
}

#[test]
fn test_related_resources_with_sessions() {
    let related = RelatedResources {
        sessions: vec!["session-1".to_string(), "session-2".to_string()],
        ..Default::default()
    };
    assert!(!related.is_empty());
    assert_eq!(related.sessions.len(), 2);
}

#[test]
fn test_related_resources_with_parent() {
    let related = RelatedResources {
        parent: Some("main".to_string()),
        ..Default::default()
    };
    assert!(!related.is_empty());
}

#[test]
fn test_related_resources_serialization() -> crate::error::Result<()> {
    let related = RelatedResources {
        sessions: vec!["s1".to_string()],
        beads: vec!["scp-1234".to_string()],
        commits: vec!["abc123".to_string()],
        ..Default::default()
    };
    let json = serde_json::to_string(&related)
        .map_err(|e| crate::error::Error::from(IoErrorKind::JsonParse(e)))?;

    assert!(json.contains("\"sessions\":[\"s1\"]"));
    assert!(json.contains("\"beads\":[\"scp-1234\"]"));
    assert!(json.contains("\"commits\":[\"abc123\"]"));
    // Empty fields should be omitted
    assert!(!json.contains("\"workspaces\""));
    assert!(!json.contains("\"parent\""));
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// RESPONSE META TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_response_meta_new() {
    let meta = ResponseMeta::new("status");
    assert_eq!(meta.command, "status");
    assert!(!meta.timestamp.is_empty());
    assert!(meta.duration_ms.is_none());
    assert!(meta.dry_run.is_none());
    assert!(meta.reversible.is_none());
    assert!(meta.undo_command.is_none());
}

#[test]
fn test_response_meta_with_duration() {
    let meta = ResponseMeta::new("add").with_duration(150);
    assert_eq!(meta.duration_ms, Some(150));
}

#[test]
fn test_response_meta_as_dry_run() {
    let meta = ResponseMeta::new("remove").as_dry_run();
    assert_eq!(meta.dry_run, Some(true));
}

#[test]
fn test_response_meta_with_undo() {
    let meta = ResponseMeta::new("remove test").with_undo("scp undo");
    assert_eq!(meta.reversible, Some(true));
    assert_eq!(meta.undo_command, Some("scp undo".to_string()));
}

#[test]
fn test_response_meta_with_agent() {
    let meta = ResponseMeta::new("work").with_agent("agent-001");
    assert_eq!(meta.agent_id, Some("agent-001".to_string()));
}

#[test]
fn test_response_meta_with_request_id() {
    let meta = ResponseMeta::new("status").with_request_id("req-123");
    assert_eq!(meta.request_id, Some("req-123".to_string()));
}

#[test]
fn test_response_meta_serialization() -> crate::error::Result<()> {
    let meta = ResponseMeta::new("add test")
        .with_duration(50)
        .with_undo("scp undo")
        .with_agent("agent-x");
    let json = serde_json::to_string(&meta)
        .map_err(|e| crate::error::Error::from(IoErrorKind::JsonParse(e)))?;

    assert!(json.contains("\"command\":\"add test\""));
    assert!(json.contains("\"duration_ms\":50"));
    assert!(json.contains("\"reversible\":true"));
    assert!(json.contains("\"undo_command\":\"scp undo\""));
    assert!(json.contains("\"agent_id\":\"agent-x\""));
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// SCHEMA ENVELOPE WITH HATEOAS TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_schema_envelope_with_links() {
    #[derive(Serialize, Deserialize)]
    struct TestData {
        name: String,
    }

    let data = TestData {
        name: "test".to_string(),
    };
    let envelope = SchemaEnvelope::new("test-response", "single", data)
        .add_link(HateoasLink::self_link("scp status test"))
        .add_link(HateoasLink::related("list", "scp list"));

    assert_eq!(envelope.links.len(), 2);
    assert_eq!(
        envelope.links.first().map(|l| &l.rel),
        Some(&"self".to_string())
    );
    assert_eq!(
        envelope.links.get(1).map(|l| &l.rel),
        Some(&"list".to_string())
    );
}

#[test]
fn test_schema_envelope_with_related() {
    #[derive(Serialize, Deserialize)]
    struct TestData {
        id: String,
    }

    let data = TestData {
        id: "abc".to_string(),
    };
    let related = RelatedResources {
        sessions: vec!["s1".to_string()],
        beads: vec!["scp-001".to_string()],
        ..Default::default()
    };
    let envelope = SchemaEnvelope::new("test-response", "single", data).with_related(related);

    assert!(envelope.related.is_some());
    if let Some(rel) = envelope.related.as_ref() {
        assert_eq!(rel.sessions.len(), 1);
        assert_eq!(rel.beads.len(), 1);
    }
}

#[test]
fn test_schema_envelope_with_meta() {
    #[derive(Serialize, Deserialize)]
    struct TestData {
        value: i32,
    }

    let data = TestData { value: 42 };
    let meta = ResponseMeta::new("test").with_duration(100);
    let envelope = SchemaEnvelope::new("test-response", "single", data).with_meta(meta);

    assert!(envelope.meta.is_some());
    if let Some(m) = envelope.meta {
        assert_eq!(m.command, "test");
        assert_eq!(m.duration_ms, Some(100));
    }
}

#[test]
fn test_schema_envelope_as_error() {
    #[derive(Serialize, Deserialize)]
    struct TestData {
        error: String,
    }

    let data = TestData {
        error: "failed".to_string(),
    };
    let envelope = SchemaEnvelope::new("error-response", "single", data).as_error();

    assert!(!envelope.success);
}

#[test]
fn test_schema_envelope_with_fixes() {
    #[derive(Serialize, Deserialize)]
    struct TestData {
        status: String,
    }

    let data = TestData {
        status: "error".to_string(),
    };
    let fixes = vec![Fix::safe("Try again", vec!["scp retry".to_string()])];
    let envelope = SchemaEnvelope::new("error-response", "single", data).with_fixes(fixes);

    assert_eq!(envelope.fixes.len(), 1);
}

#[test]
fn test_schema_envelope_full_serialization() -> crate::error::Result<()> {
    #[derive(Serialize, Deserialize)]
    struct TestData {
        name: String,
    }

    let data = TestData {
        name: "test-session".to_string(),
    };
    let envelope = SchemaEnvelope::new("session-response", "single", data)
        .add_link(HateoasLink::self_link("scp status test-session"))
        .add_link(HateoasLink::related("list", "scp list"));

    let json = serde_json::to_string_pretty(&envelope)
        .map_err(|e| crate::error::Error::from(IoErrorKind::JsonParse(e)))?;

    assert!(json.contains("$schema"));
    assert!(json.contains("test-session"));
    assert!(json.contains("_links"));
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// SCHEMA ENVELOPE ARRAY TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_schema_envelope_array_new() {
    let envelope = SchemaEnvelopeArray::new("list-response", vec!["a", "b", "c"]);
    assert_eq!(envelope.data.len(), 3);
    assert_eq!(envelope.schema_type, "array");
    assert!(envelope.success);
}

#[test]
fn test_schema_envelope_array_with_meta() {
    let data = vec![1, 2, 3];
    let meta = ResponseMeta::new("list").with_duration(50);
    let envelope = SchemaEnvelopeArray::new("list-response", data).with_meta(meta);

    assert!(envelope.meta.is_some());
}

// ═══════════════════════════════════════════════════════════════════════════
// SCHEMA URI TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_schema_uri() {
    let uri = schemas::uri("status-response");
    assert!(uri.starts_with("scp://"));
    assert!(uri.contains("status-response"));
    assert!(uri.ends_with("/v1"));
}

#[test]
fn test_all_valid_schemas() {
    let schemas_list = schemas::all_valid_schemas();
    assert!(!schemas_list.is_empty());
    assert!(schemas_list.contains(&schemas::STATUS_RESPONSE));
}

#[test]
fn test_is_valid_schema() {
    assert!(schemas::is_valid_schema("status-response"));
    assert!(!schemas::is_valid_schema("invalid-schema"));
}

// ═══════════════════════════════════════════════════════════════════════════
// JSON ERROR FROM ERROR TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_json_error_from_session_not_found() {
    let err = crate::error::Error::session("test");
    let json_error = JsonError::from(&err);

    assert!(!json_error.success);
    assert!(json_error.error.code.contains("SESSION"));
}

#[test]
fn test_json_error_from_workspace_not_found() {
    let err = crate::error::Error::workspace_not_found("test");
    let json_error = JsonError::from(&err);

    assert!(!json_error.success);
    assert!(
        json_error.error.code.contains("WORKSPACE") || json_error.error.code.contains("NOT_FOUND")
    );
}

#[test]
fn test_json_error_from_validation_error() {
    let err = crate::error::Error::validation_error("Invalid input");
    let json_error = JsonError::from(&err);

    assert!(!json_error.success);
    assert_eq!(json_error.error.exit_code, 1);
}

#[test]
fn test_json_error_from_io_error() {
    let err = crate::error::Error::io_error("disk full");
    let json_error = JsonError::from(&err);

    assert!(!json_error.success);
    assert_eq!(json_error.error.exit_code, 3);
}

#[test]
fn test_json_error_from_jj_not_found() {
    let err = crate::error::Error::jj_command_error("test", "jj not found", true);
    let json_error = JsonError::from(&err);

    assert!(!json_error.success);
    assert!(json_error.error.code.contains("JJ"));
    assert!(json_error.error.suggestion.is_some());
}

// ═══════════════════════════════════════════════════════════════════════════
// CONTEXT_MAP INTEGRATION TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_error_detail_includes_context_map() {
    let err = crate::error::Error::session_locked("my-session", "agent-001");
    let detail = ErrorDetail::from_error(&err);

    assert_eq!(detail.code, "SESSION_LOCKED");
    assert!(detail.details.is_some());
    let details = detail.details.expect("should have details");
    assert_eq!(details["session"], "my-session");
    assert_eq!(details["holder"], "agent-001");
}

#[test]
fn test_json_error_from_error_includes_context_map() {
    let err = crate::error::Error::workspace_not_found("my-workspace");
    let json_error = JsonError::from(&err);

    assert!(!json_error.success);
    assert!(json_error.error.details.is_some());
    let details = json_error.error.details.expect("should have details");
    assert_eq!(details["resource_type"], "workspace");
    assert_eq!(details["workspace_name"], "my-workspace");
}

#[test]
fn test_error_detail_context_map_jj_command() {
    let err = crate::error::Error::jj_command_error("log", "fatal error", false);
    let detail = ErrorDetail::from_error(&err);

    assert_eq!(detail.code, "JJ_COMMAND_ERROR");
    assert!(detail.details.is_some());
    let details = detail.details.expect("should have details");
    assert_eq!(details["operation"], "log");
    assert_eq!(details["source"], "fatal error");
    assert_eq!(details["is_not_found"], false);
}

// ═══════════════════════════════════════════════════════════════════════════
// JSON HELPER FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_output_json_success() {
    let data = serde_json::json!({
        "message": "success",
        "value": 42
    });
    let result = crate::json::output_json_success(&data);
    assert!(result.is_ok());
}

#[test]
fn test_output_json_success_with_struct() {
    #[derive(serde::Serialize)]
    struct TestData {
        name: String,
        count: usize,
    }
    let data = TestData {
        name: "test".to_string(),
        count: 42,
    };
    let result = crate::json::output_json_success(&data);
    assert!(result.is_ok());
}

#[test]
fn test_output_json_parse_error() {
    let exit_code = crate::json::output_json_parse_error("unknown argument --bogus");
    assert_eq!(exit_code, 2);
}

#[test]
fn test_semantic_exit_code_validation() {
    let err = crate::error::Error::validation_error("bad input");
    assert_eq!(crate::json::semantic_exit_code(&err), 1);
}

#[test]
fn test_semantic_exit_code_not_found() {
    let err = crate::error::Error::not_found("missing");
    assert_eq!(crate::json::semantic_exit_code(&err), 2);
}

#[test]
fn test_semantic_exit_code_io() {
    let err = crate::error::Error::io_error("disk full");
    assert_eq!(crate::json::semantic_exit_code(&err), 3);
}

#[test]
fn test_semantic_exit_code_jj() {
    let err = crate::error::Error::jj_command_error("status", "error", false);
    assert_eq!(crate::json::semantic_exit_code(&err), 4);
}

#[test]
fn test_semantic_exit_code_lock() {
    // Error::Lock (from coordination::locks) maps to exit code 5
    use crate::coordination::locks::errors::LockErrorKind;
    let err = crate::error::Error::from(LockErrorKind::SessionLocked {
        session: "s".to_string(),
        holder: "h".to_string(),
    });
    assert_eq!(crate::json::semantic_exit_code(&err), 5);
}

// ═══════════════════════════════════════════════════════════════════════════
// VCS ERROR CLASSIFICATION TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_vcs_not_initialized_maps_correctly() {
    use crate::error_vcs::VcsErrorKind;
    let err = crate::error::Error::from(VcsErrorKind::NotInitialized);
    let (code, msg, suggestion) = crate::json::map_error_to_parts(&err);

    assert_eq!(code, ErrorCode::NotJjRepository);
    assert_eq!(msg, "VCS not initialized");
    assert_eq!(
        suggestion,
        Some("Run 'scp init' to initialize VCS".to_string())
    );
}

#[test]
fn test_vcs_conflict_maps_to_unknown_with_suggestion() {
    use crate::error_vcs::VcsErrorKind;
    let err = crate::error::Error::from(VcsErrorKind::Conflict(
        "my-repo".to_string(),
        "merge conflict in file.rs".to_string(),
    ));
    let (code, msg, suggestion) = crate::json::map_error_to_parts(&err);

    assert_eq!(code, ErrorCode::Unknown);
    assert!(msg.contains("conflict"));
    assert_eq!(
        suggestion,
        Some("Resolve conflicts before continuing".to_string())
    );
}

#[test]
fn test_vcs_push_failed_maps_to_unknown() {
    use crate::error_vcs::VcsErrorKind;
    let err =
        crate::error::Error::from(VcsErrorKind::PushFailed("network error".to_string()));
    let (code, msg, suggestion) = crate::json::map_error_to_parts(&err);

    assert_eq!(code, ErrorCode::Unknown);
    assert!(msg.contains("push") || msg.contains("Failed"));
    // PushFailed has no built-in suggestion
    assert!(suggestion.is_none());
}

#[test]
fn test_vcs_pull_failed_maps_to_unknown() {
    use crate::error_vcs::VcsErrorKind;
    let err =
        crate::error::Error::from(VcsErrorKind::PullFailed("connection refused".to_string()));
    let (code, msg, _suggestion) = crate::json::map_error_to_parts(&err);

    assert_eq!(code, ErrorCode::Unknown);
    assert!(msg.contains("pull") || msg.contains("Failed"));
}

#[test]
fn test_vcs_rebase_failed_maps_to_unknown() {
    use crate::error_vcs::VcsErrorKind;
    let err = crate::error::Error::from(VcsErrorKind::RebaseFailed(
        "conflict during rebase".to_string(),
    ));
    let (code, msg, _suggestion) = crate::json::map_error_to_parts(&err);

    assert_eq!(code, ErrorCode::Unknown);
    assert!(msg.contains("rebase"));
}

#[test]
fn test_vcs_branch_not_found_maps_to_spawn_bead_not_found() {
    use crate::error_vcs::VcsErrorKind;
    let err =
        crate::error::Error::from(VcsErrorKind::BranchNotFound("feature-xyz".to_string()));
    let (code, msg, suggestion) = crate::json::map_error_to_parts(&err);

    assert_eq!(code, ErrorCode::SpawnBeadNotFound);
    assert!(msg.contains("Branch not found"));
    assert!(msg.contains("feature-xyz"));
    assert!(suggestion.is_none());
}

#[test]
fn test_vcs_branch_exists_maps_to_unknown() {
    use crate::error_vcs::VcsErrorKind;
    let err =
        crate::error::Error::from(VcsErrorKind::BranchExists("main".to_string()));
    let (code, msg, _suggestion) = crate::json::map_error_to_parts(&err);

    assert_eq!(code, ErrorCode::Unknown);
    assert!(msg.contains("Branch already exists"));
}

#[test]
fn test_vcs_commit_not_found_maps_to_unknown() {
    use crate::error_vcs::VcsErrorKind;
    let err =
        crate::error::Error::from(VcsErrorKind::CommitNotFound("abc123".to_string()));
    let (code, msg, _suggestion) = crate::json::map_error_to_parts(&err);

    assert_eq!(code, ErrorCode::Unknown);
    assert!(msg.contains("Commit not found"));
}

#[test]
fn test_vcs_working_copy_dirty_maps_with_suggestion() {
    use crate::error_vcs::VcsErrorKind;
    let err = crate::error::Error::from(VcsErrorKind::WorkingCopyDirty);
    let (code, msg, suggestion) = crate::json::map_error_to_parts(&err);

    assert_eq!(code, ErrorCode::Unknown);
    assert!(msg.contains("uncommitted"));
    assert_eq!(
        suggestion,
        Some("Commit or stash your changes before continuing".to_string())
    );
}

#[test]
fn test_vcs_commit_failed_maps_to_unknown() {
    use crate::error_vcs::VcsErrorKind;
    let err =
        crate::error::Error::from(VcsErrorKind::CommitFailed("pre-commit hook failed".to_string()));
    let (code, msg, _suggestion) = crate::json::map_error_to_parts(&err);

    assert_eq!(code, ErrorCode::Unknown);
    assert!(msg.contains("commit") && msg.contains("Failed"));
}

#[test]
fn test_vcs_checkout_failed_maps_to_unknown() {
    use crate::error_vcs::VcsErrorKind;
    let err = crate::error::Error::from(VcsErrorKind::CheckoutFailed(
        "uncommitted changes".to_string(),
    ));
    let (code, msg, _suggestion) = crate::json::map_error_to_parts(&err);

    assert_eq!(code, ErrorCode::Unknown);
    assert!(msg.contains("checkout") && msg.contains("Failed"));
}

#[test]
fn test_vcs_diff_failed_maps_to_unknown() {
    use crate::error_vcs::VcsErrorKind;
    let err =
        crate::error::Error::from(VcsErrorKind::DiffFailed("binary file".to_string()));
    let (code, msg, _suggestion) = crate::json::map_error_to_parts(&err);

    assert_eq!(code, ErrorCode::Unknown);
    assert!(msg.contains("diff"));
}

#[test]
fn test_vcs_merge_no_commit_id_maps_to_unknown() {
    use crate::error_vcs::VcsErrorKind;
    let err = crate::error::Error::from(VcsErrorKind::MergeNoCommitId);
    let (code, msg, _suggestion) = crate::json::map_error_to_parts(&err);

    assert_eq!(code, ErrorCode::Unknown);
    assert!(msg.contains("Merge"));
}

#[test]
fn test_vcs_init_failed_maps_to_unknown_with_suggestion() {
    use crate::error_vcs::VcsErrorKind;
    let err = crate::error::Error::from(VcsErrorKind::InitFailed {
        vcs_type: "jj".to_string(),
        directory: "/tmp/test".to_string(),
        reason: "jj not found".to_string(),
    });
    let (code, msg, suggestion) = crate::json::map_error_to_parts(&err);

    assert_eq!(code, ErrorCode::Unknown);
    assert!(msg.contains("initialize") || msg.contains("Failed"));
    // InitFailed with "not found" reason should produce a suggestion via classify_init_failure_suggestion
    assert!(suggestion.is_some());
    let sug = suggestion.expect("should have suggestion");
    assert!(sug.contains("installed") || sug.contains("PATH"));
}

// ───────────────────────────────────────────────────────────────────────────
// VCS exit code classification tests
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn test_vcs_exit_code_not_initialized() {
    use crate::error_vcs::VcsErrorKind;
    let err = crate::error::Error::from(VcsErrorKind::NotInitialized);
    assert_eq!(crate::json::classify_exit_code(&err), 1);
}

#[test]
fn test_vcs_exit_code_branch_not_found() {
    use crate::error_vcs::VcsErrorKind;
    let err =
        crate::error::Error::from(VcsErrorKind::BranchNotFound("missing".to_string()));
    assert_eq!(crate::json::classify_exit_code(&err), 2);
}

#[test]
fn test_vcs_exit_code_commit_not_found() {
    use crate::error_vcs::VcsErrorKind;
    let err =
        crate::error::Error::from(VcsErrorKind::CommitNotFound("deadbeef".to_string()));
    assert_eq!(crate::json::classify_exit_code(&err), 2);
}

#[test]
fn test_vcs_exit_code_working_copy_dirty() {
    use crate::error_vcs::VcsErrorKind;
    let err = crate::error::Error::from(VcsErrorKind::WorkingCopyDirty);
    assert_eq!(crate::json::classify_exit_code(&err), 1);
}

#[test]
fn test_vcs_exit_code_push_failed_uses_vcs_exit_code() {
    use crate::error_vcs::VcsErrorKind;
    let err =
        crate::error::Error::from(VcsErrorKind::PushFailed("network error".to_string()));
    // PushFailed exit code is 32, which falls through to the default arm
    assert_eq!(crate::json::classify_exit_code(&err), 32);
}

#[test]
fn test_vcs_exit_code_conflict_uses_vcs_exit_code() {
    use crate::error_vcs::VcsErrorKind;
    let err = crate::error::Error::from(VcsErrorKind::Conflict(
        "repo".to_string(),
        "details".to_string(),
    ));
    // Conflict exit code is 31, which falls through to the default arm
    assert_eq!(crate::json::classify_exit_code(&err), 31);
}
