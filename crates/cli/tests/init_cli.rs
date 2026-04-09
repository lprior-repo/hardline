//! Binary-level CLI integration tests for the `scp init` command.
//!
//! Tests the actual compiled binary via `assert_cmd`, verifying:
//! 1. Happy path — initializes workspace, creates .git, prints success
//! 2. Already initialized — detects existing repo, idempotent success
//! 3. Invalid directory — read-only filesystem, permission errors
//! 4. VCS type validation — rejects unknown types
//! 5. Output format — human-readable messages
//! 6. Help flag — displays usage information
//!
//! Note: The init command does NOT currently support:
//!   - `--dry-run` mode
//!   - `--format json` for machine-parseable output
//! These are documented as limitations; tests verify current behavior.

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn scp_cli() -> Command {
    Command::cargo_bin("scp-cli").expect("failed to find scp-cli binary")
}

fn temp_dir() -> TempDir {
    TempDir::new().expect("failed to create temp dir")
}

// ============================================================================
// 1. Happy path — init succeeds in empty directory
// ============================================================================

#[test]
fn init_git_succeeds_in_empty_directory() {
    let dir = temp_dir();

    scp_cli()
        .current_dir(dir.path())
        .arg("init")
        .assert()
        .success();
}

#[test]
fn init_git_creates_git_directory() {
    let dir = temp_dir();

    scp_cli()
        .current_dir(dir.path())
        .arg("init")
        .assert()
        .success();

    assert!(
        dir.path().join(".git").exists(),
        ".git directory must exist after init"
    );
    assert!(
        dir.path().join(".git").is_dir(),
        ".git must be a directory, not a file"
    );
}

#[test]
fn init_git_prints_success_message() {
    let dir = temp_dir();

    scp_cli()
        .current_dir(dir.path())
        .arg("init")
        .assert()
        .stdout(predicate::str::contains(
            "Initializing Source Control Plane",
        ))
        .stdout(predicate::str::contains("Initialized Git"));
}

#[test]
fn init_git_explicit_vcs_flag() {
    let dir = temp_dir();

    scp_cli()
        .current_dir(dir.path())
        .arg("init")
        .arg("--vcs")
        .arg("git")
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized Git"));
}

#[test]
fn init_git_creates_valid_repository() {
    let dir = temp_dir();

    scp_cli()
        .current_dir(dir.path())
        .arg("init")
        .assert()
        .success();

    let git_config = dir.path().join(".git/config");
    let head = dir.path().join(".git/HEAD");

    assert!(head.exists(), "HEAD file must exist");
    assert!(git_config.exists(), "config file must exist");
}

#[test]
fn init_git_in_nonempty_directory_preserves_files() {
    let dir = temp_dir();
    std::fs::write(dir.path().join("README.md"), "# My Project").expect("write file");
    std::fs::create_dir(dir.path().join("src")).expect("create dir");
    std::fs::write(dir.path().join("src/main.rs"), "fn main() {}").expect("write file");

    scp_cli()
        .current_dir(dir.path())
        .arg("init")
        .assert()
        .success();

    assert_eq!(
        std::fs::read_to_string(dir.path().join("README.md")).expect("read"),
        "# My Project",
        "Existing file content must be preserved"
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("src/main.rs")).expect("read"),
        "fn main() {}",
        "Nested file content must be preserved"
    );
    assert!(dir.path().join(".git").exists());
}

#[test]
fn init_git_in_directory_with_spaces() {
    let parent = temp_dir();
    let spaced = parent.path().join("my project dir");
    std::fs::create_dir(&spaced).expect("create dir with spaces");

    scp_cli()
        .current_dir(&spaced)
        .arg("init")
        .assert()
        .success();

    assert!(spaced.join(".git").exists());
}

#[test]
fn init_git_in_deeply_nested_directory() {
    let parent = temp_dir();
    let nested = parent
        .path()
        .join("a")
        .join("b")
        .join("c")
        .join("d")
        .join("e");
    std::fs::create_dir_all(&nested).expect("create nested dirs");

    scp_cli()
        .current_dir(&nested)
        .arg("init")
        .assert()
        .success();

    assert!(nested.join(".git").exists());
}

// ============================================================================
// 2. Already initialized — idempotent re-init
// ============================================================================

#[test]
fn init_git_twice_succeeds_idempotently() {
    let dir = temp_dir();

    scp_cli()
        .current_dir(dir.path())
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized Git"));

    scp_cli()
        .current_dir(dir.path())
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("Already initialized"));
}

#[test]
fn init_git_detects_existing_repository() {
    let dir = temp_dir();

    scp_cli()
        .current_dir(dir.path())
        .arg("init")
        .assert()
        .success();

    scp_cli()
        .current_dir(dir.path())
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("Already initialized with Git"));
}

#[test]
fn init_git_preserves_files_on_reinit() {
    let dir = temp_dir();

    scp_cli()
        .current_dir(dir.path())
        .arg("init")
        .assert()
        .success();

    std::fs::write(dir.path().join("data.txt"), "important data").expect("write file");

    scp_cli()
        .current_dir(dir.path())
        .arg("init")
        .assert()
        .success();

    assert_eq!(
        std::fs::read_to_string(dir.path().join("data.txt")).expect("read"),
        "important data",
        "Files must survive re-init"
    );
}

// ============================================================================
// 3. Invalid directory — read-only filesystem
// ============================================================================

#[test]
fn init_git_fails_in_readonly_directory() {
    let parent = temp_dir();
    let readonly = parent.path().join("readonly");
    std::fs::create_dir(&readonly).expect("create dir");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&readonly, std::fs::Permissions::from_mode(0o555))
            .expect("set readonly");

        scp_cli()
            .current_dir(&readonly)
            .arg("init")
            .assert()
            .failure();

        std::fs::set_permissions(&readonly, std::fs::Permissions::from_mode(0o755))
            .expect("restore permissions");
    }
}

#[test]
fn init_git_succeeds_in_writable_directory() {
    let dir = temp_dir();

    scp_cli()
        .current_dir(dir.path())
        .arg("init")
        .assert()
        .success();
}

// ============================================================================
// 4. VCS type validation — rejects unknown types
// ============================================================================

#[test]
fn init_rejects_unknown_vcs_type_mercurial() {
    let dir = temp_dir();

    scp_cli()
        .current_dir(dir.path())
        .arg("init")
        .arg("--vcs")
        .arg("mercurial")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown VCS type"))
        .stderr(predicate::str::contains("mercurial"));
}

#[test]
fn init_rejects_unknown_vcs_type_svn() {
    let dir = temp_dir();

    scp_cli()
        .current_dir(dir.path())
        .arg("init")
        .arg("--vcs")
        .arg("svn")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown VCS type"));
}

#[test]
fn init_rejects_unknown_vcs_type_hg() {
    let dir = temp_dir();

    scp_cli()
        .current_dir(dir.path())
        .arg("init")
        .arg("--vcs")
        .arg("hg")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown VCS type"));
}

#[test]
fn init_rejects_empty_vcs_type() {
    let dir = temp_dir();

    scp_cli()
        .current_dir(dir.path())
        .arg("init")
        .arg("--vcs")
        .arg("")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown VCS type"));
}

#[test]
fn init_rejects_uppercase_git() {
    let dir = temp_dir();

    scp_cli()
        .current_dir(dir.path())
        .arg("init")
        .arg("--vcs")
        .arg("GIT")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown VCS type"));
}

#[test]
fn init_rejects_mixed_case_git() {
    let dir = temp_dir();

    scp_cli()
        .current_dir(dir.path())
        .arg("init")
        .arg("--vcs")
        .arg("Git")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown VCS type"));
}

#[test]
fn init_rejects_numeric_vcs_type() {
    let dir = temp_dir();

    scp_cli()
        .current_dir(dir.path())
        .arg("init")
        .arg("--vcs")
        .arg("123")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown VCS type"));
}

// ============================================================================
// 5. Output format — human-readable messages
// ============================================================================

#[test]
fn init_output_starts_with_initializing_message() {
    let dir = temp_dir();

    scp_cli()
        .current_dir(dir.path())
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::starts_with(
            "Initializing Source Control Plane",
        ));
}

#[test]
fn init_output_contains_path_in_success_message() {
    let dir = temp_dir();

    scp_cli()
        .current_dir(dir.path())
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized Git in"));
}

#[test]
fn init_already_initialized_output_informs_user() {
    let dir = temp_dir();

    scp_cli()
        .current_dir(dir.path())
        .arg("init")
        .assert()
        .success();

    scp_cli()
        .current_dir(dir.path())
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("Already initialized with Git"));
}

#[test]
fn init_error_output_goes_to_stderr() {
    let dir = temp_dir();

    scp_cli()
        .current_dir(dir.path())
        .arg("init")
        .arg("--vcs")
        .arg("invalid")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown VCS type"))
        .stdout(predicate::str::is_empty().not());
}

// ============================================================================
// 6. Help flag — displays usage information
// ============================================================================

#[test]
fn init_help_shows_vcs_option() {
    scp_cli()
        .arg("init")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--vcs"))
        .stdout(predicate::str::contains("VCS type"));
}

#[test]
fn init_help_shows_description() {
    scp_cli()
        .arg("init")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialize SCP"));
}

// ============================================================================
// 7. Global flags — verbose and quiet
// ============================================================================

#[test]
fn init_verbose_flag_accepted() {
    let dir = temp_dir();

    scp_cli()
        .current_dir(dir.path())
        .arg("--verbose")
        .arg("init")
        .assert()
        .success();
}

#[test]
fn init_quiet_flag_accepted() {
    let dir = temp_dir();

    scp_cli()
        .current_dir(dir.path())
        .arg("--quiet")
        .arg("init")
        .assert()
        .success();
}

#[test]
fn init_format_flag_accepted() {
    let dir = temp_dir();

    scp_cli()
        .current_dir(dir.path())
        .arg("--format")
        .arg("human")
        .arg("init")
        .assert()
        .success();
}

// ============================================================================
// 8. Multiple repos — separate directories
// ============================================================================

#[test]
fn init_separate_dirs_creates_independent_repos() {
    let parent = temp_dir();
    let dir_a = parent.path().join("project-a");
    let dir_b = parent.path().join("project-b");
    std::fs::create_dir_all(&dir_a).expect("create dir a");
    std::fs::create_dir_all(&dir_b).expect("create dir b");

    scp_cli().current_dir(&dir_a).arg("init").assert().success();

    scp_cli().current_dir(&dir_b).arg("init").assert().success();

    assert!(dir_a.join(".git").exists(), "project-a should have .git");
    assert!(dir_b.join(".git").exists(), "project-b should have .git");
}

#[test]
fn init_subdirectory_detects_parent_repo() {
    let dir = temp_dir();

    scp_cli()
        .current_dir(dir.path())
        .arg("init")
        .assert()
        .success();

    let subdir = dir.path().join("subproject");
    std::fs::create_dir(&subdir).expect("create subdir");

    scp_cli()
        .current_dir(&subdir)
        .arg("init")
        .assert()
        .success();
}
