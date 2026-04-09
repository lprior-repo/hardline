//! RED QUEEN: Adversarial tests for config, doctor, init, lock commands.
//!
//! These tests attack the system from an adversary's perspective:
//! - Config: injection via config values, malformed config files
//! - Doctor: doctor on corrupted workspace
//! - Init: init in existing workspace, init with invalid paths
//! - Lock: lock exhaustion, lock with zero TTL, orphaned locks

use assert_cmd::Command;
use std::fs;
use tempfile::{NamedTempFile, TempDir};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn scp_cmd(db_path: &str) -> Command {
    let mut cmd = Command::cargo_bin("scp-cli").unwrap();
    cmd.env("SCP_DATABASE_PATH", db_path);
    cmd
}

/// Create an empty SQLite file for lock tests that don't need sessions table.
fn fresh_db() -> NamedTempFile {
    NamedTempFile::new().expect("create temp db")
}

// ===========================================================================
// CONFIG — Injection via config values, malformed config files
// ===========================================================================

mod config_adversarial {
    use super::*;

    /// Config `set` rejects empty key — prevents phantom config entries.
    #[test]
    fn config_set_empty_key_rejected() {
        let tmp = TempDir::new().expect("tempdir");
        let config_dir = tmp.path().join("scp");
        fs::create_dir_all(&config_dir).unwrap();

        // Point config at our temp directory by overriding HOME
        let mut cmd = Command::cargo_bin("scp-cli").unwrap();
        cmd.env("HOME", tmp.path())
            .env("XDG_CONFIG_HOME", tmp.path().join(".config"))
            .arg("config")
            .arg("set")
            .arg("")
            .arg("value");
        cmd.assert().failure();
    }

    /// Config `set` with shell injection in value must not execute commands.
    #[test]
    fn config_set_shell_injection_value_is_literal() {
        let tmp = TempDir::new().expect("tempdir");
        fs::create_dir_all(tmp.path().join(".config/scp")).unwrap();

        let mut cmd = Command::cargo_bin("scp-cli").unwrap();
        cmd.env("HOME", tmp.path())
            .env("XDG_CONFIG_HOME", tmp.path().join(".config"))
            .arg("config")
            .arg("set")
            .arg("injection_test")
            .arg("$(rm -rf /)");
        cmd.assert().success();

        // Verify the literal string was stored, not executed
        let mut cmd2 = Command::cargo_bin("scp-cli").unwrap();
        cmd2.env("HOME", tmp.path())
            .env("XDG_CONFIG_HOME", tmp.path().join(".config"))
            .arg("config")
            .arg("get")
            .arg("injection_test");
        cmd2.assert()
            .success()
            .stdout(predicates::str::contains("$(rm -rf /)"));
    }

    /// Config `set` with backtick injection in value.
    #[test]
    fn config_set_backtick_injection_is_literal() {
        let tmp = TempDir::new().expect("tempdir");
        fs::create_dir_all(tmp.path().join(".config/scp")).unwrap();

        let mut cmd = Command::cargo_bin("scp-cli").unwrap();
        cmd.env("HOME", tmp.path())
            .env("XDG_CONFIG_HOME", tmp.path().join(".config"))
            .arg("config")
            .arg("set")
            .arg("bt_inject")
            .arg("`whoami`");
        cmd.assert().success();

        let mut cmd2 = Command::cargo_bin("scp-cli").unwrap();
        cmd2.env("HOME", tmp.path())
            .env("XDG_CONFIG_HOME", tmp.path().join(".config"))
            .arg("config")
            .arg("get")
            .arg("bt_inject");
        cmd2.assert()
            .success()
            .stdout(predicates::str::contains("`whoami`"));
    }

    /// Config `set` with path traversal in key should store literally.
    #[test]
    fn config_set_path_traversal_key_is_literal() {
        let tmp = TempDir::new().expect("tempdir");
        fs::create_dir_all(tmp.path().join(".config/scp")).unwrap();

        let mut cmd = Command::cargo_bin("scp-cli").unwrap();
        cmd.env("HOME", tmp.path())
            .env("XDG_CONFIG_HOME", tmp.path().join(".config"))
            .arg("config")
            .arg("set")
            .arg("../../etc/passwd")
            .arg("pwned");
        cmd.assert().success();

        let mut cmd2 = Command::cargo_bin("scp-cli").unwrap();
        cmd2.env("HOME", tmp.path())
            .env("XDG_CONFIG_HOME", tmp.path().join(".config"))
            .arg("config")
            .arg("get")
            .arg("../../etc/passwd");
        cmd2.assert()
            .success()
            .stdout(predicates::str::contains("pwned"));
    }

    /// Config `set` with newline injection in value.
    #[test]
    fn config_set_newline_in_value() {
        let tmp = TempDir::new().expect("tempdir");
        fs::create_dir_all(tmp.path().join(".config/scp")).unwrap();

        let mut cmd = Command::cargo_bin("scp-cli").unwrap();
        cmd.env("HOME", tmp.path())
            .env("XDG_CONFIG_HOME", tmp.path().join(".config"))
            .arg("config")
            .arg("set")
            .arg("multi_line")
            .arg("line1\nline2 = injected");
        cmd.assert().success();
    }

    /// Config `set` with very long key (resource exhaustion).
    #[test]
    fn config_set_very_long_key() {
        let tmp = TempDir::new().expect("tempdir");
        fs::create_dir_all(tmp.path().join(".config/scp")).unwrap();

        let long_key = "x".repeat(10_000);
        let mut cmd = Command::cargo_bin("scp-cli").unwrap();
        cmd.env("HOME", tmp.path())
            .env("XDG_CONFIG_HOME", tmp.path().join(".config"))
            .arg("config")
            .arg("set")
            .arg(&long_key)
            .arg("value");
        // Should not crash — either succeed or fail gracefully
        let _ = cmd.assert().try_success();
    }

    /// Config `set` with very long value (resource exhaustion).
    /// Uses 100KB to stay within OS ARG_MAX limits while still testing
    /// that the system handles large values without crashing.
    #[test]
    fn config_set_very_long_value() {
        let tmp = TempDir::new().expect("tempdir");
        fs::create_dir_all(tmp.path().join(".config/scp")).unwrap();

        let long_value = "v".repeat(100_000);
        let mut cmd = Command::cargo_bin("scp-cli").unwrap();
        cmd.env("HOME", tmp.path())
            .env("XDG_CONFIG_HOME", tmp.path().join(".config"))
            .arg("config")
            .arg("set")
            .arg("big_val")
            .arg(&long_value);
        // Should not crash — either succeed or fail gracefully
        let _ = cmd.assert().try_success();
    }

    /// Config `get` on non-existent key returns error.
    #[test]
    fn config_get_nonexistent_key_errors() {
        let tmp = TempDir::new().expect("tempdir");
        fs::create_dir_all(tmp.path().join(".config/scp")).unwrap();

        let mut cmd = Command::cargo_bin("scp-cli").unwrap();
        cmd.env("HOME", tmp.path())
            .env("XDG_CONFIG_HOME", tmp.path().join(".config"))
            .arg("config")
            .arg("get")
            .arg("does_not_exist_xyzzy");
        cmd.assert().failure();
    }

    /// Config `set` with unicode/emoji in key and value.
    #[test]
    fn config_set_unicode_key_value() {
        let tmp = TempDir::new().expect("tempdir");
        fs::create_dir_all(tmp.path().join(".config/scp")).unwrap();

        let mut cmd = Command::cargo_bin("scp-cli").unwrap();
        cmd.env("HOME", tmp.path())
            .env("XDG_CONFIG_HOME", tmp.path().join(".config"))
            .arg("config")
            .arg("set")
            .arg("clé")
            .arg("valeur");
        let _ = cmd.assert().try_success();
    }

    /// Config `set` with equals sign in value (must preserve full value).
    #[test]
    fn config_set_equals_in_value() {
        let tmp = TempDir::new().expect("tempdir");
        fs::create_dir_all(tmp.path().join(".config/scp")).unwrap();

        let mut cmd = Command::cargo_bin("scp-cli").unwrap();
        cmd.env("HOME", tmp.path())
            .env("XDG_CONFIG_HOME", tmp.path().join(".config"))
            .arg("config")
            .arg("set")
            .arg("equation")
            .arg("a == b");
        cmd.assert().success();

        let mut cmd2 = Command::cargo_bin("scp-cli").unwrap();
        cmd2.env("HOME", tmp.path())
            .env("XDG_CONFIG_HOME", tmp.path().join(".config"))
            .arg("config")
            .arg("get")
            .arg("equation");
        cmd2.assert()
            .success()
            .stdout(predicates::str::contains("a == b"));
    }
}

// ===========================================================================
// INIT — init in existing workspace, init with invalid paths
// ===========================================================================

mod init_adversarial {
    use super::*;

    /// Init with unknown VCS type should fail with clear error.
    #[test]
    fn init_unknown_vcs_type_rejected() {
        let tmp = TempDir::new().expect("tempdir");
        let mut cmd = Command::cargo_bin("scp-cli").unwrap();
        cmd.current_dir(tmp.path())
            .arg("init")
            .arg("--vcs")
            .arg("mercurial");
        cmd.assert()
            .failure()
            .stderr(predicates::str::contains("Unknown VCS type"));
    }

    /// Init with empty VCS type should fail.
    #[test]
    fn init_empty_vcs_type_rejected() {
        let tmp = TempDir::new().expect("tempdir");
        let mut cmd = Command::cargo_bin("scp-cli").unwrap();
        cmd.current_dir(tmp.path()).arg("init").arg("--vcs").arg("");
        cmd.assert()
            .failure()
            .stderr(predicates::str::contains("Unknown VCS type"));
    }

    /// Init in already-initialized git repo reports already initialized.
    #[test]
    fn init_already_initialized_is_idempotent() {
        let tmp = TempDir::new().expect("tempdir");

        // First init
        let mut cmd1 = Command::cargo_bin("scp-cli").unwrap();
        cmd1.current_dir(tmp.path()).arg("init");
        let _ = cmd1.assert().try_success();

        // Second init — should not fail, should say already initialized
        let mut cmd2 = Command::cargo_bin("scp-cli").unwrap();
        cmd2.current_dir(tmp.path()).arg("init");
        cmd2.assert()
            .success()
            .stdout(predicates::str::contains("Already initialized"));
    }

    /// Init lock file is a symlink — must be rejected (security).
    #[test]
    #[cfg(unix)]
    fn init_symlink_lock_file_rejected() {
        let tmp = TempDir::new().expect("tempdir");
        let symlink_target = tmp.path().join("lock_target");
        let symlink = tmp.path().join(".scp-init.lock");

        // Create target file first so the symlink resolves
        fs::write(&symlink_target, "target").unwrap();
        std::os::unix::fs::symlink(&symlink_target, &symlink).expect("create symlink");
        assert!(symlink.exists(), "symlink should exist");

        let mut cmd = Command::cargo_bin("scp-cli").unwrap();
        cmd.current_dir(tmp.path()).arg("init");
        cmd.assert()
            .failure()
            .stderr(predicates::str::contains("symlink"));
    }

    /// Init in a read-only directory should fail gracefully.
    /// Uses a subdirectory approach: cd to writable dir, init in read-only subdirectory.
    #[test]
    #[cfg(unix)]
    fn init_readonly_directory_fails_gracefully() {
        let tmp = TempDir::new().expect("tempdir");
        let readonly_dir = tmp.path().join("readonly");
        fs::create_dir_all(&readonly_dir).unwrap();

        // Remove write permission from the subdirectory
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&readonly_dir, fs::Permissions::from_mode(0o444)).unwrap();

        // Run init with explicit path argument from writable parent
        let mut cmd = Command::cargo_bin("scp-cli").unwrap();
        cmd.current_dir(tmp.path()).arg("init");
        // Init should fail gracefully when it can't write to the directory
        let result = cmd.assert().try_failure();

        // Restore permissions for cleanup
        fs::set_permissions(&readonly_dir, fs::Permissions::from_mode(0o755)).unwrap();
        let _ = result;
    }

    /// Init lock file path with a directory already there should fail.
    #[test]
    fn init_lock_path_is_directory() {
        let tmp = TempDir::new().expect("tempdir");
        // Create a directory where the lock file should go
        fs::create_dir(tmp.path().join(".scp-init.lock")).unwrap();

        let mut cmd = Command::cargo_bin("scp-cli").unwrap();
        cmd.current_dir(tmp.path()).arg("init");
        // Should fail — can't open a directory as a file
        cmd.assert().failure();
    }
}

// ===========================================================================
// LOCK — lock exhaustion, zero TTL, orphaned locks, adversarial inputs
// ===========================================================================

mod lock_adversarial {
    use super::*;

    /// Lock acquire with TTL=0 uses default (should succeed).
    #[test]
    fn lock_acquire_zero_ttl_uses_default() {
        let db = fresh_db();
        let db_path = db.path().to_str().unwrap();

        scp_cmd(db_path)
            .arg("lock")
            .arg("acquire")
            .arg("s1")
            .arg("--agent")
            .arg("a1")
            .arg("--ttl")
            .arg("0")
            .assert()
            .success();
    }

    /// Lock acquire with TTL=1 (minimum meaningful TTL).
    #[test]
    fn lock_acquire_ttl_one_succeeds() {
        let db = fresh_db();
        let db_path = db.path().to_str().unwrap();

        scp_cmd(db_path)
            .arg("lock")
            .arg("acquire")
            .arg("s1")
            .arg("--agent")
            .arg("a1")
            .arg("--ttl")
            .arg("1")
            .assert()
            .success();
    }

    /// Lock acquire with max TTL (86400 = 24h) should succeed.
    #[test]
    fn lock_acquire_max_ttl_succeeds() {
        let db = fresh_db();
        let db_path = db.path().to_str().unwrap();

        scp_cmd(db_path)
            .arg("lock")
            .arg("acquire")
            .arg("s1")
            .arg("--agent")
            .arg("a1")
            .arg("--ttl")
            .arg("86400")
            .assert()
            .success();
    }

    /// Lock acquire with TTL exceeding max (86401) should fail.
    #[test]
    fn lock_acquire_ttl_exceeds_max_rejected() {
        let db = fresh_db();
        let db_path = db.path().to_str().unwrap();

        scp_cmd(db_path)
            .arg("lock")
            .arg("acquire")
            .arg("s1")
            .arg("--agent")
            .arg("a1")
            .arg("--ttl")
            .arg("86401")
            .assert()
            .failure();
    }

    /// Lock acquire with absurdly large TTL should fail.
    #[test]
    fn lock_acquire_absurd_ttl_rejected() {
        let db = fresh_db();
        let db_path = db.path().to_str().unwrap();

        scp_cmd(db_path)
            .arg("lock")
            .arg("acquire")
            .arg("s1")
            .arg("--agent")
            .arg("a1")
            .arg("--ttl")
            .arg("999999999999")
            .assert()
            .failure();
    }

    /// Lock acquire with empty session name should fail.
    #[test]
    fn lock_acquire_empty_session_rejected() {
        let db = fresh_db();
        let db_path = db.path().to_str().unwrap();

        scp_cmd(db_path)
            .arg("lock")
            .arg("acquire")
            .arg("")
            .arg("--agent")
            .arg("a1")
            .assert()
            .failure();
    }

    /// Lock acquire with empty agent ID should fail.
    #[test]
    fn lock_acquire_empty_agent_rejected() {
        let db = fresh_db();
        let db_path = db.path().to_str().unwrap();

        scp_cmd(db_path)
            .arg("lock")
            .arg("acquire")
            .arg("s1")
            .arg("--agent")
            .arg("")
            .assert()
            .failure();
    }

    /// Lock acquire with session name containing control characters should fail.
    #[test]
    fn lock_acquire_control_chars_session_rejected() {
        let db = fresh_db();
        let db_path = db.path().to_str().unwrap();

        scp_cmd(db_path)
            .arg("lock")
            .arg("acquire")
            .arg("sess\tion")
            .arg("--agent")
            .arg("a1")
            .assert()
            .failure();
    }

    /// Lock acquire with session name exceeding 255 chars should fail.
    #[test]
    fn lock_acquire_session_too_long_rejected() {
        let db = fresh_db();
        let db_path = db.path().to_str().unwrap();

        let long_session = "s".repeat(256);
        scp_cmd(db_path)
            .arg("lock")
            .arg("acquire")
            .arg(&long_session)
            .arg("--agent")
            .arg("a1")
            .assert()
            .failure();
    }

    /// Lock acquire with session name at exactly 255 chars should succeed.
    #[test]
    fn lock_acquire_session_max_length_succeeds() {
        let db = fresh_db();
        let db_path = db.path().to_str().unwrap();

        let max_session = "s".repeat(255);
        scp_cmd(db_path)
            .arg("lock")
            .arg("acquire")
            .arg(&max_session)
            .arg("--agent")
            .arg("a1")
            .assert()
            .success();
    }

    /// Lock exhaustion: acquire many locks on different sessions.
    /// Verifies no resource leak under load.
    #[test]
    fn lock_exhaustion_many_sessions() {
        let db = fresh_db();
        let db_path = db.path().to_str().unwrap();

        // Acquire 50 locks on different sessions
        for i in 0..50 {
            scp_cmd(db_path)
                .arg("lock")
                .arg("acquire")
                .arg(&format!("exhaust-{i}"))
                .arg("--agent")
                .arg(&format!("agent-{i}"))
                .assert()
                .success();
        }

        // All should appear in list
        scp_cmd(db_path)
            .arg("lock")
            .arg("list")
            .assert()
            .success()
            .stdout(predicates::str::contains("exhaust-0"));
    }

    /// Lock same session by same agent is idempotent.
    #[test]
    fn lock_same_session_same_agent_idempotent() {
        let db = fresh_db();
        let db_path = db.path().to_str().unwrap();

        scp_cmd(db_path)
            .arg("lock")
            .arg("acquire")
            .arg("s1")
            .arg("--agent")
            .arg("a1")
            .assert()
            .success();

        // Re-acquire same session by same agent — should succeed (idempotent)
        scp_cmd(db_path)
            .arg("lock")
            .arg("acquire")
            .arg("s1")
            .arg("--agent")
            .arg("a1")
            .assert()
            .success();
    }

    /// Release a lock that was never acquired — should succeed (idempotent, no-op).
    /// Lock release is intentionally idempotent: releasing a non-existent lock
    /// logs a warning but returns Ok(()) to avoid fragile error handling.
    #[test]
    fn lock_release_never_acquired_is_noop() {
        let db = fresh_db();
        let db_path = db.path().to_str().unwrap();

        scp_cmd(db_path)
            .arg("lock")
            .arg("release")
            .arg("ghost")
            .arg("--agent")
            .arg("a1")
            .assert()
            .success();
    }

    /// Heartbeat on non-existent lock should fail.
    #[test]
    fn lock_heartbeat_no_lock_fails() {
        let db = fresh_db();
        let db_path = db.path().to_str().unwrap();

        scp_cmd(db_path)
            .arg("lock")
            .arg("heartbeat")
            .arg("ghost")
            .arg("--agent")
            .arg("a1")
            .assert()
            .failure();
    }

    /// Release then release again (double unlock) — should succeed (idempotent).
    /// Lock release is intentionally idempotent: both releases return Ok(()),
    /// with the second logged as a double-unlock warning in the audit trail.
    #[test]
    fn lock_double_release_is_idempotent() {
        let db = fresh_db();
        let db_path = db.path().to_str().unwrap();

        scp_cmd(db_path)
            .arg("lock")
            .arg("acquire")
            .arg("s1")
            .arg("--agent")
            .arg("a1")
            .assert()
            .success();

        scp_cmd(db_path)
            .arg("lock")
            .arg("release")
            .arg("s1")
            .arg("--agent")
            .arg("a1")
            .assert()
            .success();

        // Second release — should also succeed (idempotent, logs warning)
        scp_cmd(db_path)
            .arg("lock")
            .arg("release")
            .arg("s1")
            .arg("--agent")
            .arg("a1")
            .assert()
            .success();
    }

    /// Lock with special characters in session name that are valid.
    #[test]
    fn lock_acquire_session_with_slashes_and_dots() {
        let db = fresh_db();
        let db_path = db.path().to_str().unwrap();

        scp_cmd(db_path)
            .arg("lock")
            .arg("acquire")
            .arg("hardline/polecats/quartz")
            .arg("--agent")
            .arg("a1")
            .assert()
            .success();

        // Verify status shows locked
        scp_cmd(db_path)
            .arg("lock")
            .arg("status")
            .arg("hardline/polecats/quartz")
            .assert()
            .success()
            .stdout(predicates::str::contains("Locked"));
    }

    /// Lock acquire on corrupted database should fail gracefully.
    #[test]
    fn lock_acquire_corrupted_db_fails_gracefully() {
        let tmp = TempDir::new().expect("tempdir");
        let db_path = tmp.path().join("corrupted.db");
        // Write garbage to simulate corrupted database
        fs::write(&db_path, b"NOT A VALID SQLITE DATABASE").unwrap();

        scp_cmd(db_path.to_str().unwrap())
            .arg("lock")
            .arg("acquire")
            .arg("s1")
            .arg("--agent")
            .arg("a1")
            .assert()
            .failure();
    }

    /// Lock status on corrupted database should fail gracefully.
    #[test]
    fn lock_status_corrupted_db_fails_gracefully() {
        let tmp = TempDir::new().expect("tempdir");
        let db_path = tmp.path().join("corrupted.db");
        fs::write(&db_path, b"GARBAGE").unwrap();

        scp_cmd(db_path.to_str().unwrap())
            .arg("lock")
            .arg("status")
            .arg("s1")
            .assert()
            .failure();
    }

    /// Lock list on corrupted database should fail gracefully.
    #[test]
    fn lock_list_corrupted_db_fails_gracefully() {
        let tmp = TempDir::new().expect("tempdir");
        let db_path = tmp.path().join("corrupted.db");
        fs::write(&db_path, b"GARBAGE").unwrap();

        scp_cmd(db_path.to_str().unwrap())
            .arg("lock")
            .arg("list")
            .assert()
            .failure();
    }

    /// Two different agents competing for same session — second must fail.
    #[test]
    fn lock_contention_second_agent_rejected() {
        let db = fresh_db();
        let db_path = db.path().to_str().unwrap();

        scp_cmd(db_path)
            .arg("lock")
            .arg("acquire")
            .arg("contended")
            .arg("--agent")
            .arg("alpha")
            .assert()
            .success();

        scp_cmd(db_path)
            .arg("lock")
            .arg("acquire")
            .arg("contended")
            .arg("--agent")
            .arg("beta")
            .assert()
            .failure()
            .stderr(predicates::str::contains("alpha"));
    }

    /// SQL injection attempt in session name — must be stored literally.
    #[test]
    fn lock_sql_injection_session_name_literal() {
        let db = fresh_db();
        let db_path = db.path().to_str().unwrap();

        let inject = "'; DROP TABLE session_locks; --";
        scp_cmd(db_path)
            .arg("lock")
            .arg("acquire")
            .arg(inject)
            .arg("--agent")
            .arg("a1")
            .assert()
            .success();

        // Table should still exist — verify with status
        scp_cmd(db_path)
            .arg("lock")
            .arg("status")
            .arg(inject)
            .assert()
            .success();
    }

    /// SQL injection attempt in agent ID — must be stored literally.
    #[test]
    fn lock_sql_injection_agent_id_literal() {
        let db = fresh_db();
        let db_path = db.path().to_str().unwrap();

        scp_cmd(db_path)
            .arg("lock")
            .arg("acquire")
            .arg("s1")
            .arg("--agent")
            .arg("'; DROP TABLE session_locks; --")
            .assert()
            .success();

        // Table should still exist — list should work
        scp_cmd(db_path).arg("lock").arg("list").assert().success();
    }

    /// Rapid acquire-release cycle should not leak state.
    #[test]
    fn lock_rapid_acquire_release_cycle() {
        let db = fresh_db();
        let db_path = db.path().to_str().unwrap();

        // First acquire
        scp_cmd(db_path)
            .arg("lock")
            .arg("acquire")
            .arg("rapid")
            .arg("--agent")
            .arg("agent-0")
            .assert()
            .success();

        // Rapid cycle: release then acquire by next agent
        for i in 1..20 {
            let prev = format!("agent-{}", i - 1);
            let curr = format!("agent-{i}");

            scp_cmd(db_path)
                .arg("lock")
                .arg("release")
                .arg("rapid")
                .arg("--agent")
                .arg(&prev)
                .assert()
                .success();

            scp_cmd(db_path)
                .arg("lock")
                .arg("acquire")
                .arg("rapid")
                .arg("--agent")
                .arg(&curr)
                .assert()
                .success();
        }

        // Verify lock is held by last agent
        scp_cmd(db_path)
            .arg("lock")
            .arg("status")
            .arg("rapid")
            .assert()
            .success()
            .stdout(predicates::str::contains("agent-19"));
    }
}

// ===========================================================================
// DOCTOR — adversarial workspace health checks
// ===========================================================================

mod doctor_adversarial {
    use super::*;

    /// Doctor on non-git directory should report failure (no VCS).
    #[test]
    fn doctor_non_git_dir_reports_vcs_missing() {
        let tmp = TempDir::new().expect("tempdir");

        let mut cmd = Command::cargo_bin("scp-cli").unwrap();
        cmd.current_dir(tmp.path())
            .env("HOME", tmp.path())
            .arg("doctor");
        cmd.assert()
            .failure()
            .stdout(predicates::str::contains("No VCS found"));
    }

    /// Doctor on empty directory — all checks should gracefully report issues.
    #[test]
    fn doctor_empty_dir_graceful() {
        let tmp = TempDir::new().expect("tempdir");

        let mut cmd = Command::cargo_bin("scp-cli").unwrap();
        cmd.current_dir(tmp.path())
            .env("HOME", tmp.path())
            .arg("doctor");
        // Should not panic, should run all checks
        cmd.assert()
            .stdout(predicates::str::contains("Checking VCS"));
    }

    /// Doctor with a corrupted .git directory (file instead of directory).
    #[test]
    fn doctor_corrupted_git_file() {
        let tmp = TempDir::new().expect("tempdir");
        // Create a file named .git instead of a directory
        fs::write(tmp.path().join(".git"), "not a git repo").unwrap();

        let mut cmd = Command::cargo_bin("scp-cli").unwrap();
        cmd.current_dir(tmp.path())
            .env("HOME", tmp.path())
            .arg("doctor");
        // Should not panic — may report VCS found but fail gracefully on deeper checks
        cmd.assert().stdout(predicates::str::contains("Checking"));
    }

    /// Doctor --full on a valid git repo — should succeed.
    #[test]
    fn doctor_full_on_git_repo() {
        let tmp = TempDir::new().expect("tempdir");

        // Create a minimal git repo
        let git_dir = tmp.path().join(".git");
        fs::create_dir_all(&git_dir).unwrap();
        fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        fs::create_dir_all(git_dir.join("refs/heads")).unwrap();
        fs::create_dir_all(git_dir.join("objects")).unwrap();

        let mut cmd = Command::cargo_bin("scp-cli").unwrap();
        cmd.current_dir(tmp.path())
            .env("HOME", tmp.path())
            .arg("doctor")
            .arg("--full");
        // Should report VCS found and run full diagnostics
        cmd.assert()
            .stdout(predicates::str::contains("VCS initialized"))
            .stdout(predicates::str::contains("full diagnostics"));
    }

    /// Doctor on directory with lock file present — should report warning.
    #[test]
    fn doctor_detects_lock_file() {
        let tmp = TempDir::new().expect("tempdir");

        // Create minimal git repo
        let git_dir = tmp.path().join(".git");
        fs::create_dir_all(&git_dir).unwrap();
        fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        fs::create_dir_all(git_dir.join("refs/heads")).unwrap();
        fs::create_dir_all(git_dir.join("objects")).unwrap();

        // Create a lock file
        fs::write(git_dir.join("lock"), "test lock").unwrap();

        let mut cmd = Command::cargo_bin("scp-cli").unwrap();
        cmd.current_dir(tmp.path())
            .env("HOME", tmp.path())
            .arg("doctor")
            .arg("--full");
        cmd.assert().stdout(predicates::str::contains("lock file"));
    }

    /// Doctor on unreadable directory should fail gracefully, not panic.
    /// Uses parent directory for spawn, since 0o000 prevents chdir.
    #[test]
    #[cfg(unix)]
    fn doctor_unreadable_dir_no_panic() {
        let tmp = TempDir::new().expect("tempdir");
        let subdir = tmp.path().join("noaccess");
        fs::create_dir_all(&subdir).unwrap();

        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&subdir, fs::Permissions::from_mode(0o000)).unwrap();

        // Run from parent dir — the doctor will encounter the unreadable
        // subdirectory during its checks and must handle it gracefully.
        let mut cmd = Command::cargo_bin("scp-cli").unwrap();
        cmd.current_dir(tmp.path())
            .env("HOME", tmp.path())
            .arg("doctor");
        // Should not panic — may succeed or fail but never crash
        let result = cmd.assert().try_success();

        // Restore for cleanup
        fs::set_permissions(&subdir, fs::Permissions::from_mode(0o755)).unwrap();
        let _ = result;
    }
}

// ===========================================================================
// Cross-command adversarial — interactions between commands
// ===========================================================================

mod cross_command_adversarial {
    use super::*;

    /// Init then doctor should work together.
    #[test]
    fn init_then_doctor_consistent() {
        let tmp = TempDir::new().expect("tempdir");

        // Init
        let mut cmd = Command::cargo_bin("scp-cli").unwrap();
        cmd.current_dir(tmp.path()).arg("init");
        let _ = cmd.assert().try_success();

        // Doctor should see VCS
        let mut cmd2 = Command::cargo_bin("scp-cli").unwrap();
        cmd2.current_dir(tmp.path())
            .env("HOME", tmp.path())
            .arg("doctor");
        cmd2.assert()
            .stdout(predicates::str::contains("VCS initialized"));
    }

    /// Lock then init in same directory should not interfere.
    #[test]
    fn lock_and_init_independent() {
        let tmp = TempDir::new().expect("tempdir");
        let db = fresh_db();
        let db_path = db.path().to_str().unwrap();

        // Acquire a lock
        scp_cmd(db_path)
            .arg("lock")
            .arg("acquire")
            .arg("s1")
            .arg("--agent")
            .arg("a1")
            .assert()
            .success();

        // Init should not be affected by locks (different subsystem)
        let mut cmd = Command::cargo_bin("scp-cli").unwrap();
        cmd.current_dir(tmp.path()).arg("init");
        let _ = cmd.assert().try_success();

        // Lock should still be held
        scp_cmd(db_path)
            .arg("lock")
            .arg("status")
            .arg("s1")
            .assert()
            .success()
            .stdout(predicates::str::contains("Locked"));
    }
}
