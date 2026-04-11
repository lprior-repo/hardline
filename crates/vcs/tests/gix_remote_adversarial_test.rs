//! Adversarial tests for gix remote operations
//!
//! Property-based and edge-case tests targeting:
//! - `remote::fetch` — name validation, missing remotes, bare repos, all-remotes
//! - `remote::pull` — detached HEAD, rebase flag, missing remote-tracking branches
//! - `remote::push` — bare repos, detached HEAD, force/delete flags, auth error detection
//! - Error classification — GitError variants map correctly through From<GitError> for VcsError
//! - `GitBackend::push`/`pull` — trait contract invariants

use scp_vcs::domain::traits::VcsBackend;
use scp_vcs::error::{GitError, VcsError};
use scp_vcs::gix::remote;
use scp_vcs::gix::repository;
use scp_vcs::infrastructure::GitBackend;
use tempfile::TempDir;

fn init_repo_with_branch(temp: &TempDir) -> gix::Repository {
    let repo = repository::init(temp.path()).expect("init");
    let empty_tree = gix::ObjectId::from_hex(b"4b825dc642cb6eb9a060e54bf8d69288fbee4904")
        .expect("empty tree oid");
    repo.reference(
        "refs/heads/main",
        empty_tree,
        gix::refs::transaction::PreviousValue::MustNotExist,
        "init main",
    )
    .expect("create main ref");
    repo
}

// ============================================================================
// fetch — adversarial inputs
// ============================================================================

#[test]
fn fetch_nonexistent_remote_returns_error() {
    let temp = TempDir::new().expect("temp");
    let repo = init_repo_with_branch(&temp);
    let result = remote::fetch(&repo, Some("nonexistent-remote-xyz"), false, false, false);
    assert!(result.is_err(), "fetch from nonexistent remote must fail");
}

#[test]
fn fetch_empty_string_remote_defaults_to_origin() {
    let temp = TempDir::new().expect("temp");
    let repo = init_repo_with_branch(&temp);
    // None defaults to "origin" which doesn't exist → error
    let result = remote::fetch(&repo, None, false, false, false);
    assert!(
        result.is_err(),
        "fetch with None (defaults to origin) must fail when no origin"
    );
}

#[test]
fn fetch_all_remotes_on_repo_without_remotes() {
    let temp = TempDir::new().expect("temp");
    let repo = init_repo_with_branch(&temp);
    let result = remote::fetch(&repo, None, false, false, true);
    // fetch_all_remotes runs `git remote` — on a repo with no remotes,
    // it should either error or return "No remotes configured"
    match result {
        Ok(updates) => {
            assert!(!updates.is_empty(), "should return at least one message");
        }
        Err(_) => {
            // Also acceptable — git remote returns nothing, CLI fails
        }
    }
}

#[test]
fn fetch_error_is_git_error() {
    let temp = TempDir::new().expect("temp");
    let repo = init_repo_with_branch(&temp);
    let result = remote::fetch(&repo, Some("nope"), false, false, false);
    let err = result.expect_err("should fail");
    let _typed: GitError = err;
}

#[test]
fn fetch_with_prune_flag_still_errors_on_missing_remote() {
    let temp = TempDir::new().expect("temp");
    let repo = init_repo_with_branch(&temp);
    let result = remote::fetch(&repo, Some("ghost"), true, false, false);
    assert!(result.is_err());
}

#[test]
fn fetch_with_tags_flag_still_errors_on_missing_remote() {
    let temp = TempDir::new().expect("temp");
    let repo = init_repo_with_branch(&temp);
    let result = remote::fetch(&repo, Some("ghost"), false, true, false);
    assert!(result.is_err());
}

// ============================================================================
// pull — adversarial inputs
// ============================================================================

#[test]
fn pull_detached_head_returns_error() {
    let temp = TempDir::new().expect("temp");
    let repo = init_repo_with_branch(&temp);
    // Detach HEAD by writing a raw OID to HEAD file
    let empty_tree = gix::ObjectId::from_hex(b"4b825dc642cb6eb9a060e54bf8d69288fbee4904")
        .expect("empty tree oid");
    let head_path = temp.path().join(".git").join("HEAD");
    std::fs::write(&head_path, format!("{empty_tree}\n")).expect("write HEAD");

    let result = remote::pull(&repo, Some("origin"), false);
    assert!(result.is_err(), "pull with detached HEAD should fail");
}

#[test]
fn pull_rebase_flag_returns_unsupported_error() {
    let temp = TempDir::new().expect("temp");
    let repo = init_repo_with_branch(&temp);
    let result = remote::pull(&repo, None, true);
    assert!(result.is_err());
    let err = result.expect_err("should fail");
    let msg = format!("{err}");
    assert!(
        msg.contains("rebase"),
        "expected rebase in error, got: {msg}"
    );
}

#[test]
fn pull_without_remote_fails() {
    let temp = TempDir::new().expect("temp");
    let repo = init_repo_with_branch(&temp);
    let result = remote::pull(&repo, Some("nonexistent"), false);
    assert!(result.is_err());
}

#[test]
fn pull_error_propagates_through_vcs_error() {
    let temp = TempDir::new().expect("temp");
    let repo = init_repo_with_branch(&temp);
    let result = remote::pull(&repo, None, true);
    let git_err = result.expect_err("should fail");
    let vcs_err: VcsError = git_err.into();
    match vcs_err {
        VcsError::PullFailed(_) | VcsError::BranchNotFound(_) | VcsError::Unimplemented(_) => {}
        other => panic!("expected PullFailed/BranchNotFound/Unimplemented, got: {other:?}"),
    }
}

#[test]
fn pull_default_remote_is_origin() {
    let temp = TempDir::new().expect("temp");
    let repo = init_repo_with_branch(&temp);
    // Pull with None defaults to origin, which doesn't exist → error
    let result = remote::pull(&repo, None, false);
    assert!(
        result.is_err(),
        "pull with None (default origin) must fail without origin"
    );
}

// ============================================================================
// push — adversarial inputs
// ============================================================================

#[test]
fn push_bare_repo_returns_error() {
    let temp = TempDir::new().expect("temp");
    let bare_path = temp.path().join("bare.git");
    std::fs::create_dir_all(&bare_path).expect("create dir");
    // Use --bare via git CLI to create a true bare repo (no workdir)
    std::process::Command::new("git")
        .args(["init", "--bare", bare_path.to_str().expect("path")])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("git init --bare");
    let repo = gix::discover(&bare_path).expect("discover bare");
    let result = remote::push(&repo, "origin", None, false, false, false);
    assert!(
        result.is_err(),
        "push from bare repo (no workdir) must fail"
    );
}

#[test]
fn push_detached_head_returns_error() {
    let temp = TempDir::new().expect("temp");
    let repo = init_repo_with_branch(&temp);
    // Detach HEAD by writing a raw OID to HEAD file
    let empty_tree = gix::ObjectId::from_hex(b"4b825dc642cb6eb9a060e54bf8d69288fbee4904")
        .expect("empty tree oid");
    let head_path = temp.path().join(".git").join("HEAD");
    std::fs::write(&head_path, format!("{empty_tree}\n")).expect("write HEAD");

    let result = remote::push(&repo, "origin", None, false, false, false);
    assert!(result.is_err(), "push in detached HEAD state must fail");
}

#[test]
fn push_explicit_branch_still_fails_without_remote() {
    let temp = TempDir::new().expect("temp");
    let repo = init_repo_with_branch(&temp);
    let result = remote::push(&repo, "origin", Some("main"), false, false, false);
    assert!(
        result.is_err(),
        "push with explicit branch must fail when remote doesn't exist"
    );
}

#[test]
fn push_force_flag_still_fails_without_remote() {
    let temp = TempDir::new().expect("temp");
    let repo = init_repo_with_branch(&temp);
    let result = remote::push(&repo, "origin", Some("main"), true, false, false);
    assert!(result.is_err());
}

#[test]
fn push_delete_flag_still_fails_without_remote() {
    let temp = TempDir::new().expect("temp");
    let repo = init_repo_with_branch(&temp);
    let result = remote::push(&repo, "origin", Some("main"), false, false, true);
    assert!(result.is_err());
}

#[test]
fn push_tags_flag_still_fails_without_remote() {
    let temp = TempDir::new().expect("temp");
    let repo = init_repo_with_branch(&temp);
    let result = remote::push(&repo, "origin", Some("main"), false, true, false);
    assert!(result.is_err());
}

#[test]
fn push_delete_and_tags_mutually_exclusive_no_panic() {
    let temp = TempDir::new().expect("temp");
    let repo = init_repo_with_branch(&temp);
    // delete=true, tags=true — the code checks `if tags && !delete` so tags is skipped
    // but it should not panic, just fail at git push level
    let result = remote::push(&repo, "origin", Some("main"), false, true, true);
    assert!(result.is_err());
}

#[test]
fn push_error_is_git_error() {
    let temp = TempDir::new().expect("temp");
    let repo = init_repo_with_branch(&temp);
    let result = remote::push(&repo, "origin", None, false, false, false);
    let err = result.expect_err("should fail");
    let _typed: GitError = err;
}

#[test]
fn push_error_maps_to_vcs_error_push_failed_for_auth() {
    let git_err = GitError::Unauthorized("permission denied".to_string());
    let vcs_err: VcsError = git_err.into();
    assert!(matches!(vcs_err, VcsError::PushFailed(_)));
}

#[test]
fn push_error_maps_to_vcs_error_push_failed_for_network() {
    let git_err = GitError::Network("connection refused".to_string());
    let vcs_err: VcsError = git_err.into();
    // Network errors map to PullFailed per the From impl
    assert!(matches!(vcs_err, VcsError::PullFailed(_)));
}

#[test]
fn push_error_maps_to_vcs_error_for_invalid_ref() {
    let git_err = GitError::InvalidRef {
        name: "workdir".to_string(),
        reason: "no workdir".to_string(),
    };
    let vcs_err: VcsError = git_err.into();
    assert!(matches!(vcs_err, VcsError::BranchNotFound(_)));
}

// ============================================================================
// GitBackend trait contract — push/pull
// ============================================================================

#[test]
fn git_backend_push_fails_without_remote() {
    let temp = TempDir::new().expect("temp");
    init_repo_with_branch(&temp);
    let backend = GitBackend::new(temp.path().to_path_buf());
    let result = backend.push();
    assert!(result.is_err(), "GitBackend::push must fail without origin");
}

#[test]
fn git_backend_pull_fails_without_remote() {
    let temp = TempDir::new().expect("temp");
    init_repo_with_branch(&temp);
    let backend = GitBackend::new(temp.path().to_path_buf());
    let result = backend.pull();
    assert!(result.is_err(), "GitBackend::pull must fail without origin");
}

#[test]
fn git_backend_push_error_is_vcs_error() {
    let temp = TempDir::new().expect("temp");
    init_repo_with_branch(&temp);
    let backend = GitBackend::new(temp.path().to_path_buf());
    let result = backend.push();
    let _typed: VcsError = result.expect_err("should fail");
}

#[test]
fn git_backend_pull_error_is_vcs_error() {
    let temp = TempDir::new().expect("temp");
    init_repo_with_branch(&temp);
    let backend = GitBackend::new(temp.path().to_path_buf());
    let result = backend.pull();
    let _typed: VcsError = result.expect_err("should fail");
}

// ============================================================================
// Error display invariants — no panic on empty/long strings
// ============================================================================

#[test]
fn git_error_network_empty_string_no_panic() {
    let err = GitError::Network(String::new());
    let _msg = format!("{err}");
}

#[test]
fn git_error_network_long_string_no_panic() {
    let err = GitError::Network("x".repeat(100_000));
    let _msg = format!("{err}");
}

#[test]
fn git_error_unauthorized_empty_string_no_panic() {
    let err = GitError::Unauthorized(String::new());
    let _msg = format!("{err}");
}

#[test]
fn git_error_invalid_ref_empty_name_and_reason() {
    let err = GitError::InvalidRef {
        name: String::new(),
        reason: String::new(),
    };
    let _msg = format!("{err}");
}

#[test]
fn vcs_error_push_failed_empty_no_panic() {
    let err = VcsError::PushFailed(String::new());
    let _msg = format!("{err}");
}

#[test]
fn vcs_error_pull_failed_empty_no_panic() {
    let err = VcsError::PullFailed(String::new());
    let _msg = format!("{err}");
}

#[test]
fn vcs_error_push_failed_long_message() {
    let err = VcsError::PushFailed("x".repeat(100_000));
    let _msg = format!("{err}");
}

// ============================================================================
// Error classification: GitError → VcsError mapping is exhaustive
// ============================================================================

#[test]
fn git_error_network_maps_to_vcs_pull_failed() {
    let git_err = GitError::Network("timeout".to_string());
    let vcs_err: VcsError = git_err.into();
    match vcs_err {
        VcsError::PullFailed(msg) => assert_eq!(msg, "timeout"),
        other => panic!("expected PullFailed, got: {other:?}"),
    }
}

#[test]
fn git_error_unauthorized_maps_to_vcs_push_failed() {
    let git_err = GitError::Unauthorized("denied".to_string());
    let vcs_err: VcsError = git_err.into();
    match vcs_err {
        VcsError::PushFailed(msg) => assert_eq!(msg, "denied"),
        other => panic!("expected PushFailed, got: {other:?}"),
    }
}

#[test]
fn git_error_not_found_maps_to_vcs_not_initialized() {
    let git_err = GitError::NotFound("/no/repo".into());
    let vcs_err: VcsError = git_err.into();
    assert!(matches!(vcs_err, VcsError::NotInitialized));
}

#[test]
fn git_error_invalid_ref_maps_to_vcs_branch_not_found() {
    let git_err = GitError::InvalidRef {
        name: "refs/heads/ghost".to_string(),
        reason: "not found".to_string(),
    };
    let vcs_err: VcsError = git_err.into();
    match vcs_err {
        VcsError::BranchNotFound(name) => assert_eq!(name, "refs/heads/ghost"),
        other => panic!("expected BranchNotFound, got: {other:?}"),
    }
}

#[test]
fn git_error_conflict_maps_to_vcs_conflict() {
    let git_err = GitError::Conflict {
        message: "diverged".to_string(),
        conflicted_files: vec!["a.rs".into(), "b.rs".into()],
    };
    let vcs_err: VcsError = git_err.into();
    match vcs_err {
        VcsError::Conflict(msg, _) => assert_eq!(msg, "diverged"),
        other => panic!("expected Conflict, got: {other:?}"),
    }
}

#[test]
fn git_error_parse_error_maps_to_vcs_parse_error() {
    let git_err = GitError::ParseError("bad json".to_string());
    let vcs_err: VcsError = git_err.into();
    match vcs_err {
        VcsError::ParseError(msg) => assert_eq!(msg, "bad json"),
        other => panic!("expected ParseError, got: {other:?}"),
    }
}

#[test]
fn git_error_io_maps_to_vcs_io() {
    let io = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
    let git_err = GitError::Io(io);
    let vcs_err: VcsError = git_err.into();
    assert!(matches!(vcs_err, VcsError::Io(_)));
}

// ============================================================================

#[test]
fn fetch_all_remotes_no_remotes_returns_message() {
    let temp = TempDir::new().expect("temp");
    let repo = init_repo_with_branch(&temp);
    let result = remote::fetch(&repo, None, false, false, true);
    // fetch_all_remotes runs `git remote` — on a repo with no remotes,
    // it should either error or return "No remotes configured"
    match result {
        Ok(updates) => {
            // If it succeeds, should have the fallback message
            assert!(!updates.is_empty(), "should return at least one message");
        }
        Err(_) => {
            // Also acceptable — git remote returns nothing, CLI fails
        }
    }
}

// ============================================================================
// Idempotency — repeated failures are safe
// ============================================================================

#[test]
fn fetch_repeated_failures_are_consistent() {
    let temp = TempDir::new().expect("temp");
    let repo = init_repo_with_branch(&temp);

    let err1 = remote::fetch(&repo, Some("nope"), false, false, false);
    let err2 = remote::fetch(&repo, Some("nope"), false, false, false);
    assert!(err1.is_err());
    assert!(err2.is_err());
}

#[test]
fn push_repeated_failures_are_consistent() {
    let temp = TempDir::new().expect("temp");
    let repo = init_repo_with_branch(&temp);

    let err1 = remote::push(&repo, "nope", Some("main"), false, false, false);
    let err2 = remote::push(&repo, "nope", Some("main"), false, false, false);
    assert!(err1.is_err());
    assert!(err2.is_err());
}

#[test]
fn pull_repeated_failures_are_consistent() {
    let temp = TempDir::new().expect("temp");
    let repo = init_repo_with_branch(&temp);

    let err1 = remote::pull(&repo, Some("nope"), false);
    let err2 = remote::pull(&repo, Some("nope"), false);
    assert!(err1.is_err());
    assert!(err2.is_err());
}

// ============================================================================
// Unicode in remote names — no panic
// ============================================================================

#[test]
fn fetch_unicode_remote_name_no_panic() {
    let temp = TempDir::new().expect("temp");
    let repo = init_repo_with_branch(&temp);
    let result = remote::fetch(&repo, Some("日本語"), false, false, false);
    // May succeed or fail, but must not panic
    let _ = result;
}

#[test]
fn push_unicode_remote_name_no_panic() {
    let temp = TempDir::new().expect("temp");
    let repo = init_repo_with_branch(&temp);
    let result = remote::push(&repo, "日本語", Some("main"), false, false, false);
    let _ = result;
}

#[test]
fn pull_unicode_remote_name_no_panic() {
    let temp = TempDir::new().expect("temp");
    let repo = init_repo_with_branch(&temp);
    let result = remote::pull(&repo, Some("日本語"), false);
    let _ = result;
}

// ============================================================================
// Special characters in remote names — injection resistance
// ============================================================================

#[test]
fn fetch_special_chars_remote_name_no_panic() {
    let temp = TempDir::new().expect("temp");
    let repo = init_repo_with_branch(&temp);
    for name in [
        "origin; rm -rf /",
        "$(echo pwned)",
        "`whoami`",
        "origin\nevil",
    ] {
        let result = remote::fetch(&repo, Some(name), false, false, false);
        let _ = result; // Must not panic
    }
}

#[test]
fn push_special_chars_branch_name_no_panic() {
    let temp = TempDir::new().expect("temp");
    let repo = init_repo_with_branch(&temp);
    for branch in ["main; rm -rf /", "$(echo x)", "`id`"] {
        let result = remote::push(&repo, "origin", Some(branch), false, false, false);
        let _ = result; // Must not panic
    }
}

// ============================================================================
// Property-based: error classification is total (covers all GitError variants)
// ============================================================================

#[test]
fn all_git_error_variants_convert_to_vcs_error() {
    let variants: Vec<GitError> = vec![
        GitError::NotFound("/tmp".into()),
        GitError::InvalidRef {
            name: "x".into(),
            reason: "y".into(),
        },
        GitError::Conflict {
            message: "m".into(),
            conflicted_files: vec![],
        },
        GitError::Unauthorized("u".into()),
        GitError::Network("n".into()),
        GitError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "io")),
        GitError::ParseError("p".into()),
    ];

    for git_err in variants {
        let vcs_err: VcsError = git_err.into();
        let _msg = format!("{vcs_err}"); // Must not panic on display
    }
}

// ============================================================================
// GitBackend is Send + Sync
// ============================================================================

#[test]
fn git_backend_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<GitBackend>();
}
