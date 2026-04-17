//! Git backend tests
//!
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

#[cfg(test)]
#[allow(clippy::expect_used)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::fs;
    use std::process::Command;

    use tempfile::TempDir;

    use crate::vcs::{
        BackendType, BranchName, CommitId, GitBackend, GitBackendConfig, RepoStatus, VcsError,
    };

    use super::helpers::parse_git_version;

    fn create_test_repo() -> (TempDir, std::path::PathBuf) {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let path = temp.path().to_path_buf();

        let output = Command::new("git")
            .args(["init"])
            .current_dir(&path)
            .output()
            .expect("Failed to run git init");

        assert!(
            output.status.success(),
            "git init failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&path)
            .output()
            .expect("Failed to configure git");

        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&path)
            .output()
            .expect("Failed to configure git");

        (temp, path)
    }

    fn create_bare_repo() -> (TempDir, std::path::PathBuf) {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let path = temp.path().join("repo.git");

        let output = Command::new("git")
            .args(["init", "--bare"])
            .arg(&path)
            .output()
            .expect("Failed to run git init --bare");

        assert!(
            output.status.success(),
            "git init --bare failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        (temp, path)
    }

    fn create_initial_commit(repo_path: &std::path::Path) -> String {
        let file = repo_path.join("README.md");
        fs::write(&file, "# Test Repository\n").expect("Failed to write file");

        Command::new("git")
            .args(["add", "."])
            .current_dir(repo_path)
            .output()
            .expect("Failed to git add");

        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to git commit");

        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to get HEAD");

        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    #[test]
    fn test_open_git_repo_returns_ok() {
        let (_temp, path) = create_test_repo();

        let result = GitBackend::open(&path);

        assert!(result.is_ok());
    }

    #[test]
    fn test_open_returns_gitbackend_with_correct_path() {
        let (_temp, path) = create_test_repo();

        let backend = GitBackend::open(&path).expect("Should open");

        let backend_path = backend.path().as_path();
        assert!(backend_path.is_absolute());
    }

    #[test]
    fn test_backend_type_returns_git() {
        let (_temp, path) = create_test_repo();
        let backend = GitBackend::open(&path).expect("Should open");

        let backend_type = backend.backend_type();

        assert_eq!(backend_type, BackendType::Git);
    }

    #[test]
    fn test_path_returns_absolute_canonical_path() {
        let (_temp, path) = create_test_repo();
        let backend = GitBackend::open(&path).expect("Should open");

        let repo_path = backend.path();

        assert!(repo_path.as_path().is_absolute());
        let path_str = repo_path.as_path().to_string_lossy();
        assert!(!path_str.contains("/./"));
        assert!(!path_str.contains("/../"));
    }

    #[test]
    fn test_open_from_subdirectory_finds_repo_root() {
        let (_temp, path) = create_test_repo();
        let subdir = path.join("src").join("lib");
        fs::create_dir_all(&subdir).expect("Failed to create subdir");

        let result = GitBackend::open(&subdir);

        assert!(result.is_ok());
    }

    #[test]
    fn test_current_branch_on_main_returns_main() {
        let (_temp, path) = create_test_repo();
        create_initial_commit(&path);

        let backend = GitBackend::open(&path).expect("Should open");

        let result = backend.current_branch();

        assert!(result.is_ok());
        let branch = result.expect("Should have branch");
        assert!(branch.is_some());
    }

    #[test]
    fn test_current_branch_name_has_no_refs_prefix() {
        let (_temp, path) = create_test_repo();
        create_initial_commit(&path);

        let backend = GitBackend::open(&path).expect("Should open");

        let branch = backend.current_branch().expect("Should work");

        if let Some(name) = branch {
            assert!(!name.as_str().starts_with("refs/heads/"));
        }
    }

    #[test]
    fn test_current_branch_on_branch_with_slash_works() {
        let (_temp, path) = create_test_repo();
        create_initial_commit(&path);

        Command::new("git")
            .args(["checkout", "-b", "feature/test-branch"])
            .current_dir(&path)
            .output()
            .expect("Failed to create branch");

        let backend = GitBackend::open(&path).expect("Should open");

        let branch = backend.current_branch().expect("Should work");

        assert!(branch.is_some());
        let name = branch.expect("Should have branch");
        assert_eq!(name.as_str(), "feature/test-branch");
    }

    #[test]
    fn test_current_branch_detached_head_returns_none() {
        let (_temp, path) = create_test_repo();
        let sha = create_initial_commit(&path);

        Command::new("git")
            .args(["checkout", &sha])
            .current_dir(&path)
            .output()
            .expect("Failed to checkout commit");

        let backend = GitBackend::open(&path).expect("Should open");

        let result = backend.current_branch();

        assert!(result.is_ok());
        assert!(result.expect("Should have result").is_none());
    }

    #[test]
    fn test_list_branches_returns_all_local_branches() {
        let (_temp, path) = create_test_repo();
        create_initial_commit(&path);

        Command::new("git")
            .args(["branch", "develop"])
            .current_dir(&path)
            .output()
            .expect("Failed to create branch");

        Command::new("git")
            .args(["branch", "feature/a"])
            .current_dir(&path)
            .output()
            .expect("Failed to create branch");

        let backend = GitBackend::open(&path).expect("Should open");

        let branches = backend.list_branches().expect("Should work");

        assert!(branches.len() >= 3);
    }

    #[test]
    fn test_list_branches_names_have_no_refs_prefix() {
        let (_temp, path) = create_test_repo();
        create_initial_commit(&path);

        let backend = GitBackend::open(&path).expect("Should open");

        let branches = backend.list_branches().expect("Should work");

        for branch in &branches {
            assert!(!branch.as_str().starts_with("refs/heads/"));
        }
    }

    #[test]
    fn test_status_clean_repo_returns_has_changes_false() {
        let (_temp, path) = create_test_repo();
        create_initial_commit(&path);

        let backend = GitBackend::open(&path).expect("Should open");

        let status = backend.status().expect("Should work");

        assert!(!status.has_changes);
    }

    #[test]
    fn test_status_modified_file_has_changes_true() {
        let (_temp, path) = create_test_repo();
        create_initial_commit(&path);

        let file = path.join("README.md");
        fs::write(&file, "# Modified content\n").expect("Failed to modify file");

        let backend = GitBackend::open(&path).expect("Should open");

        let status = backend.status().expect("Should work");

        assert!(status.has_changes);
        assert!(status.modified > 0);
    }

    #[test]
    fn test_commit_exists_valid_sha_returns_true() {
        let (_temp, path) = create_test_repo();
        let sha = create_initial_commit(&path);

        let backend = GitBackend::open(&path).expect("Should open");
        let commit_id = CommitId::new(&sha).expect("Valid commit ID");

        let result = backend.commit_exists(&commit_id);

        assert!(result.is_ok());
        assert!(result.expect("Should have result"));
    }

    #[test]
    fn test_commit_exists_nonexistent_sha_returns_false() {
        let (_temp, path) = create_test_repo();
        create_initial_commit(&path);

        let backend = GitBackend::open(&path).expect("Should open");
        let commit_id = CommitId::new("deadbeef12345678901234567890123456789012")
            .expect("Valid commit ID format");

        let result = backend.commit_exists(&commit_id);

        assert!(result.is_ok());
        assert!(!result.expect("Should have result"));
    }

    #[test]
    fn test_commit_exists_invalid_sha_returns_false() {
        let (_temp, path) = create_test_repo();
        create_initial_commit(&path);

        let backend = GitBackend::open(&path).expect("Should open");
        let commit_id = CommitId::new("not-a-valid-ref").expect("Valid string");

        let result = backend.commit_exists(&commit_id);

        assert!(result.is_ok());
        assert!(!result.expect("Should have result"));
    }

    #[test]
    fn test_is_clean_clean_repo_returns_true() {
        let (_temp, path) = create_test_repo();
        create_initial_commit(&path);

        let backend = GitBackend::open(&path).expect("Should open");

        let result = backend.is_clean();

        assert!(result.is_ok());
        assert!(result.expect("Should be clean"));
    }

    #[test]
    fn test_is_clean_with_modified_file_returns_false() {
        let (_temp, path) = create_test_repo();
        create_initial_commit(&path);

        let file = path.join("README.md");
        fs::write(&file, "# Modified\n").expect("Failed to modify");

        let backend = GitBackend::open(&path).expect("Should open");

        let result = backend.is_clean();

        assert!(result.is_ok());
        assert!(!result.expect("Should have result"));
    }

    #[test]
    fn test_verify_cli_version_returns_version_string() {
        let (_temp, path) = create_test_repo();

        let config = GitBackendConfig {
            verify_cli_version: false,
        };
        let backend = GitBackend::open_with_config(&path, &config).expect("Should open");

        let result = backend.verify_cli_version();

        assert!(result.is_ok());
        let version = result.expect("Should have version");
        assert!(!version.is_empty());
    }

    #[test]
    fn test_open_nonexistent_path_returns_path_not_found() {
        let nonexistent = "/nonexistent/path/xyz/test";

        let result = GitBackend::open(nonexistent);

        assert!(matches!(result, Err(VcsError::PathNotFound(_))));
    }

    #[test]
    fn test_open_file_path_returns_path_not_directory() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp.path().join("test.txt");
        fs::write(&file_path, "content").expect("Failed to write file");

        let result = GitBackend::open(&file_path);

        assert!(matches!(result, Err(VcsError::PathNotDirectory(_))));
    }

    #[test]
    fn test_open_non_git_directory_returns_git_open_failed() {
        let temp = TempDir::new().expect("Failed to create temp dir");

        let result = GitBackend::open(temp.path());

        assert!(matches!(result, Err(VcsError::GitOpenFailed { .. })));
    }

    #[test]
    fn test_open_bare_repo_returns_bare_repository_not_supported() {
        let (_temp, path) = create_bare_repo();

        let result = GitBackend::open(&path);

        match result {
            Err(VcsError::BareRepositoryNotSupported(p)) => {
                assert_eq!(p, path);
            }
            Err(e) => panic!("Wrong error type: {e:?}"),
            Ok(_) => panic!("Should have returned error"),
        }
    }

    #[test]
    fn test_parse_git_version_standard() {
        let output = "git version 2.43.0";
        let result = parse_git_version(output);
        assert!(result.is_ok());
        assert_eq!(result.expect("Should parse"), (2, 43));
    }

    #[test]
    fn test_parse_git_version_with_windows_suffix() {
        let output = "git version 2.43.0.windows.1";
        let result = parse_git_version(output);
        assert!(result.is_ok());
        assert_eq!(result.expect("Should parse"), (2, 43));
    }

    #[test]
    fn test_parse_git_version_invalid_format() {
        let output = "invalid output";
        let result = parse_git_version(output);
        assert!(matches!(result, Err(VcsError::GitParseError(_))));
    }

    #[test]
    fn test_gitbackend_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<GitBackend>();
    }

    #[test]
    fn test_git_backend_config_default() {
        let config = GitBackendConfig::default();
        assert!(config.verify_cli_version);
    }

    #[test]
    fn test_open_with_config_skip_version_check() {
        let (_temp, path) = create_test_repo();

        let config = GitBackendConfig {
            verify_cli_version: false,
        };

        let result = GitBackend::open_with_config(&path, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_snapshot_commit_message() {
        let (_temp, path) = create_test_repo();

        fs::write(path.join("file.txt"), "content\n").expect("Failed to write file");
        Command::new("git")
            .args(["add", "."])
            .current_dir(&path)
            .output()
            .expect("Failed to git add");

        Command::new("git")
            .args(["commit", "-m", "Add initial file"])
            .current_dir(&path)
            .output()
            .expect("Failed to git commit");

        let output = Command::new("git")
            .args(["log", "--format=%B", "-1"])
            .current_dir(&path)
            .output()
            .expect("Failed to git log");

        let message = String::from_utf8_lossy(&output.stdout).to_string();

        insta::assert_snapshot!("commit_message_single", message);
    }

    #[test]
    fn test_snapshot_commit_message_multiline() {
        let (_temp, path) = create_test_repo();

        fs::write(path.join("file.txt"), "content\n").expect("Failed to write file");
        Command::new("git")
            .args(["add", "."])
            .current_dir(&path)
            .output()
            .expect("Failed to git add");

        Command::new("git")
            .args([
                "commit",
                "-m",
                "Add feature\n\nThis is the body of the commit.\nWith multiple lines.",
            ])
            .current_dir(&path)
            .output()
            .expect("Failed to git commit");

        let output = Command::new("git")
            .args(["log", "--format=%B", "-1"])
            .current_dir(&path)
            .output()
            .expect("Failed to git log");

        let message = String::from_utf8_lossy(&output.stdout).to_string();

        insta::assert_snapshot!("commit_message_multiline", message);
    }

    #[test]
    fn test_snapshot_diff_single_file() {
        let (_temp, path) = create_test_repo();

        fs::write(path.join("file.txt"), "content\n").expect("Failed to write file");
        Command::new("git")
            .args(["add", "."])
            .current_dir(&path)
            .output()
            .expect("Failed to git add");

        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(&path)
            .output()
            .expect("Failed to git commit");

        fs::write(path.join("file.txt"), "modified content\n").expect("Failed to modify file");

        let output = Command::new("git")
            .args(["diff"])
            .current_dir(&path)
            .output()
            .expect("Failed to git diff");

        let diff = String::from_utf8_lossy(&output.stdout).to_string();

        insta::assert_snapshot!("diff_single_file", diff);
    }

    #[test]
    fn test_snapshot_diff_binary_file() {
        let (_temp, path) = create_test_repo();

        fs::write(path.join("file.txt"), "content\n").expect("Failed to write file");
        Command::new("git")
            .args(["add", "."])
            .current_dir(&path)
            .output()
            .expect("Failed to git add");

        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(&path)
            .output()
            .expect("Failed to git commit");

        fs::write(path.join("binary.bin"), b"\x00\x01\x02\x03").expect("Failed to write binary");

        let output = Command::new("git")
            .args(["diff", "--binary"])
            .current_dir(&path)
            .output()
            .expect("Failed to git diff");

        let diff = String::from_utf8_lossy(&output.stdout).to_string();

        insta::assert_snapshot!("diff_binary_file", diff);
    }

    #[test]
    fn test_snapshot_tree_output() {
        let (_temp, path) = create_test_repo();

        fs::write(path.join("file.txt"), "content\n").expect("Failed to write file");
        fs::create_dir(path.join("subdir")).expect("Failed to create dir");
        fs::write(path.join("subdir", "nested.txt"), "nested\n").expect("Failed to write nested");

        Command::new("git")
            .args(["add", "."])
            .current_dir(&path)
            .output()
            .expect("Failed to git add");

        Command::new("git")
            .args(["commit", "-m", "Add files and dirs"])
            .current_dir(&path)
            .output()
            .expect("Failed to git commit");

        let output = Command::new("git")
            .args(["ls-tree", "-R", "HEAD"])
            .current_dir(&path)
            .output()
            .expect("Failed to git ls-tree");

        let tree = String::from_utf8_lossy(&output.stdout).to_string();

        insta::assert_snapshot!("tree_output", tree);
    }

    #[test]
    fn test_snapshot_diff_with_renamed_file() {
        let (_temp, path) = create_test_repo();

        fs::write(path.join("old.txt"), "content\n").expect("Failed to write file");
        Command::new("git")
            .args(["add", "."])
            .current_dir(&path)
            .output()
            .expect("Failed to git add");

        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(&path)
            .output()
            .expect("Failed to git commit");

        Command::new("git")
            .args(["mv", "old.txt", "new.txt"])
            .current_dir(&path)
            .output()
            .expect("Failed to git mv");

        let output = Command::new("git")
            .args(["diff", "--cached"])
            .current_dir(&path)
            .output()
            .expect("Failed to git diff");

        let diff = String::from_utf8_lossy(&output.stdout).to_string();

        insta::assert_snapshot!("diff_renamed_file", diff);
    }
}
