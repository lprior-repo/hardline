//! Snapshot tests for VCS and config type JSON serialization.
//!
//! These tests verify that VCS types and config types serialize correctly
//! to JSON for CLI output and configuration persistence.

use scp_core::{config::Config, vcs_types::*, ConflictType, FileId, SessionId, WorkspaceId};
use std::path::PathBuf;

#[test]
fn test_workspace_id_json() {
    let id = WorkspaceId("ws-abc123".into());
    let json = serde_json::to_string(&id).unwrap();
    insta::assert_snapshot!("workspace_id", json);
}

#[test]
fn test_session_id_json() {
    let id = SessionId("session-xyz789".into());
    let json = serde_json::to_string(&id).unwrap();
    insta::assert_snapshot!("session_id", json);
}

#[test]
fn test_file_id_json() {
    let id = FileId("file-123456".into());
    let json = serde_json::to_string(&id).unwrap();
    insta::assert_snapshot!("file_id", json);
}

#[test]
fn test_vcs_commit_id_json() {
    let id = VcsCommitId("abc123def456".into());
    let json = serde_json::to_string(&id).unwrap();
    insta::assert_snapshot!("vcs_commit_id", json);
}

#[test]
fn test_vcs_branch_name_json() {
    let name = VcsBranchName("feature/new-feature".into());
    let json = serde_json::to_string(&name).unwrap();
    insta::assert_snapshot!("vcs_branch_name", json);
}

#[test]
fn test_vcs_tag_json() {
    let tag = VcsTag("v1.2.3".into());
    let json = serde_json::to_string(&tag).unwrap();
    insta::assert_snapshot!("vcs_tag", json);
}

#[test]
fn test_conflict_type_serialization() {
    let conflict_types = vec![
        (ConflictType::None, "none"),
        (ConflictType::Text, "text"),
        (ConflictType::Binary, "binary"),
        (ConflictType::Unresolved, "unresolved"),
    ];

    for (conflict_type, name) in conflict_types {
        let json = serde_json::to_string(&conflict_type).unwrap();
        insta::assert_snapshot!(format!("conflict_type_{}", name), json);
    }
}

#[test]
fn test_config_basic_json() {
    let config = Config {
        session_max_count: 10,
        session_default_branch: "main".into(),
        session_workspace_root: PathBuf::from("/home/user/isolate"),
        queue_enabled: true,
        queue_max_size: 100,
        hooks_pre_create: None,
        hooks_post_create: None,
        hooks_pre_remove: None,
        hooks_post_remove: None,
        agent_default_timeout_secs: 3600,
        output_format: "json".into(),
    };
    let json = serde_json::to_string(&config).unwrap();
    insta::assert_snapshot!("config_basic", json);
}

#[test]
fn test_config_with_hooks_json() {
    let config = Config {
        session_max_count: 5,
        session_default_branch: "develop".into(),
        session_workspace_root: PathBuf::from("/home/user/dev"),
        queue_enabled: false,
        queue_max_size: 50,
        hooks_pre_create: Some("/usr/local/bin/pre-create-hook".into()),
        hooks_post_create: Some("/usr/local/bin/post-create-hook".into()),
        hooks_pre_remove: Some("/usr/local/bin/pre-remove-hook".into()),
        hooks_post_remove: None,
        agent_default_timeout_secs: 7200,
        output_format: "yaml".into(),
    };
    let json = serde_json::to_string(&config).unwrap();
    insta::assert_snapshot!("config_with_hooks", json);
}

#[test]
fn test_branch_names_json() {
    let names = BranchNames(vec![
        "main".into(),
        "develop".into(),
        "feature/new-feature".into(),
        "bugfix/fix-issue".into(),
    ]);
    let json = serde_json::to_string(&names).unwrap();
    insta::assert_snapshot!("branch_names", json);
}

#[test]
fn test_remote_url_json() {
    let url = RemoteUrl("https://github.com/user/repo.git".into());
    let json = serde_json::to_string(&url).unwrap();
    insta::assert_snapshot!("remote_url", json);
}
