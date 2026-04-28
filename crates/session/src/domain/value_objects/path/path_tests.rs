//! Tests for AbsolutePath value object.

use pretty_assertions::assert_eq;

use crate::domain::value_objects::path::{
    find_first_metacharacter, path_errors::*, AbsolutePath, PathBuf,
};

// Helper to create AbsolutePath easily in tests
fn abs_path(s: &str) -> Result<AbsolutePath, AbsolutePathError> {
    AbsolutePath::try_from(s)
}

// =========================================================================
// Happy Path Tests
// =========================================================================

#[test]
fn test_absolute_path_from_str_succeeds() {
    let result = abs_path("/usr/local/bin");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().to_path_buf(),
        PathBuf::from("/usr/local/bin")
    );
}

#[test]
fn test_absolute_path_from_pathbuf_succeeds() {
    let result: Result<AbsolutePath, _> = PathBuf::from("/var/log/nginx").try_into();
    assert!(result.is_ok());
}

#[test]
fn test_as_path_returns_correct_reference() {
    use std::path::Path;
    let path = abs_path("/home/user").unwrap();
    assert_eq!(path.as_path(), Path::new("/home/user"));
    assert!(path.as_path().is_absolute());
}

#[test]
fn test_to_path_buf_returns_equivalent_pathbuf() {
    let path = abs_path("/tmp/data").unwrap();
    assert_eq!(path.to_path_buf(), PathBuf::from("/tmp/data"));
}

#[test]
fn test_absolute_path_clone_is_equal_and_independent() {
    let original = abs_path("/etc/config").unwrap();
    let clone = original.clone();
    assert_eq!(original, clone);
}

#[test]
fn test_absolute_path_display_shows_full_path() {
    let path = abs_path("/etc/config").unwrap();
    assert_eq!(format!("{}", path), "/etc/config");
}

// =========================================================================
// Error Path Tests
// =========================================================================

#[test]
fn test_relative_path_starting_with_dot_slash_returns_error() {
    let result = abs_path("./current/dir");
    assert!(matches!(
        result,
        Err(AbsolutePathError::PathValidation(
            PathValidationError::NotAbsolute { .. }
        ))
    ));
}

#[test]
fn test_relative_path_starting_with_dot_dot_returns_error() {
    let result = abs_path("../parent/dir");
    assert!(matches!(
        result,
        Err(AbsolutePathError::PathValidation(
            PathValidationError::NotAbsolute { .. }
        ))
    ));
}

#[test]
fn test_simple_filename_returns_error() {
    let result = abs_path("just-a-file.txt");
    assert!(matches!(
        result,
        Err(AbsolutePathError::PathValidation(
            PathValidationError::NotAbsolute { .. }
        ))
    ));
}

#[test]
fn test_path_containing_dollar_sign_returns_error() {
    let result = abs_path("/path/with$VAR");
    assert!(matches!(
        result,
        Err(AbsolutePathError::ShellMetacharacter(
            ShellMetacharacterError::ContainsDollar { position: 10 }
        ))
    ));
}

#[test]
fn test_path_containing_backtick_returns_error() {
    let result = abs_path("/path/with`cmd`");
    assert!(matches!(
        result,
        Err(AbsolutePathError::ShellMetacharacter(
            ShellMetacharacterError::ContainsBacktick { .. }
        ))
    ));
}

#[test]
fn test_path_containing_semicolon_returns_error() {
    let result = abs_path("/path/with;command");
    assert!(matches!(
        result,
        Err(AbsolutePathError::ShellMetacharacter(
            ShellMetacharacterError::ContainsSemicolon { .. }
        ))
    ));
}

#[test]
fn test_path_containing_pipe_returns_error() {
    let result = abs_path("/path/with|pipe");
    assert!(matches!(
        result,
        Err(AbsolutePathError::ShellMetacharacter(
            ShellMetacharacterError::ContainsPipe { .. }
        ))
    ));
}

#[test]
fn test_path_containing_ampersand_returns_error() {
    let result = abs_path("/path/with&bg");
    assert!(matches!(
        result,
        Err(AbsolutePathError::ShellMetacharacter(
            ShellMetacharacterError::ContainsAmpersand { .. }
        ))
    ));
}

#[test]
fn test_path_with_multiple_metacharacters_returns_dollar_error() {
    // $ at position 5, ` at position 10, ; at position 14
    let result = abs_path("/path$with`and;many");
    assert!(matches!(
        result,
        Err(AbsolutePathError::ShellMetacharacter(
            ShellMetacharacterError::ContainsDollar { position: 5 }
        ))
    ));
}

// =========================================================================
// Edge Case Tests
// =========================================================================

#[test]
fn test_empty_path_returns_not_absolute_error() {
    let result = abs_path("");
    assert!(matches!(
        result,
        Err(AbsolutePathError::PathValidation(PathValidationError::NotAbsolute { input }))
            if input.is_empty()
    ));
}

#[test]
fn test_root_path_succeeds() {
    let result = abs_path("/");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().to_path_buf(), PathBuf::from("/"));
}

#[test]
fn test_path_with_multiple_slashes_succeeds() {
    let result = abs_path("/usr//local//bin");
    assert!(result.is_ok());
}

#[test]
fn test_path_with_unicode_characters_succeeds() {
    let result = abs_path("/home/用户/文档");
    assert!(result.is_ok());
}

#[test]
fn test_path_ending_with_slash_succeeds() {
    let result = abs_path("/var/log/");
    assert!(result.is_ok());
}

#[test]
fn test_path_with_at_sign_succeeds() {
    let result = abs_path("/home/user@host/path");
    assert!(result.is_ok());
}

#[test]
fn test_path_with_hyphen_underscore_succeeds() {
    let result = abs_path("/home/user_name/my-doc");
    assert!(result.is_ok());
}

// =========================================================================
// Contract Verification Tests
// =========================================================================

#[test]
fn test_invariant_is_absolute_holds_for_valid_path() {
    let path = abs_path("/home/user/data").unwrap();
    assert!(path.as_path().is_absolute());
}

#[test]
fn test_invariant_no_shell_metacharacters_holds() {
    let path = abs_path("/safe/path").unwrap();
    let path_buf = path.to_path_buf();
    let path_str = path_buf.to_string_lossy();
    assert!(find_first_metacharacter(&path_str).is_none());
}

#[test]
fn test_invariant_valid_utf8_holds() {
    let path = abs_path("/home/user/data").unwrap();
    assert!(path.as_path().to_str().is_some());
}

#[test]
fn test_error_contains_input_context_on_not_absolute() {
    let result = abs_path("./foo");
    if let Err(AbsolutePathError::PathValidation(PathValidationError::NotAbsolute { input })) =
        result
    {
        assert_eq!(input, "./foo");
    } else {
        panic!("Expected NotAbsolute error");
    }
}

#[test]
fn test_error_contains_position_on_metacharacter() {
    let result = abs_path("/foo$bar");
    if let Err(AbsolutePathError::ShellMetacharacter(ShellMetacharacterError::ContainsDollar {
        position,
    })) = result
    {
        // "/foo$bar" - $ is at position 4 (0-indexed)
        assert_eq!(position, 4);
    } else {
        panic!("Expected ContainsDollar error at position 4");
    }
}

#[test]
fn test_invariant_preserved_after_as_path() {
    let path = abs_path("/home/user/data").unwrap();
    let retrieved = path.as_path();
    assert!(retrieved.is_absolute());
}

#[test]
fn test_invariant_preserved_after_to_path_buf() {
    let path = abs_path("/home/user/data").unwrap();
    let retrieved = path.to_path_buf();
    assert!(retrieved.is_absolute());
}

#[test]
fn test_invariant_utf8_preserved_after_to_path_buf() {
    let path = abs_path("/home/user/data").unwrap();
    let retrieved = path.to_path_buf();
    assert!(retrieved.to_str().is_some());
}

// =========================================================================
// Contract Violation Tests
// =========================================================================

#[test]
fn test_violation_p1_invalid_utf8_returns_invalid_utf8_error() {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        let bytes = vec![0xFF, 0xFE];
        let os_string = std::ffi::OsString::from_vec(bytes);
        let path = PathBuf::from(os_string);
        let result: Result<AbsolutePath, _> = path.try_into();
        assert!(matches!(
            result,
            Err(AbsolutePathError::PathValidation(
                PathValidationError::InvalidUtf8 { .. }
            ))
        ));
    }
    #[cfg(not(unix))]
    {
        let _ = vec![0xFF, 0xFE];
    }
}

#[test]
fn test_violation_p2_dot_slash_relative_returns_not_absolute_error() {
    let result = abs_path("./relative/path");
    assert!(matches!(
        result,
        Err(AbsolutePathError::PathValidation(
            PathValidationError::NotAbsolute { .. }
        ))
    ));
}

#[test]
fn test_violation_p2_dot_dot_relative_returns_not_absolute_error() {
    let result = abs_path("../parent/relative");
    assert!(matches!(
        result,
        Err(AbsolutePathError::PathValidation(
            PathValidationError::NotAbsolute { .. }
        ))
    ));
}

#[test]
fn test_violation_p2_simple_filename_returns_not_absolute_error() {
    let result = abs_path("just-a-filename");
    assert!(matches!(
        result,
        Err(AbsolutePathError::PathValidation(
            PathValidationError::NotAbsolute { .. }
        ))
    ));
}

#[test]
fn test_violation_p2_empty_string_returns_not_absolute_error() {
    let result = abs_path("");
    assert!(matches!(
        result,
        Err(AbsolutePathError::PathValidation(
            PathValidationError::NotAbsolute { .. }
        ))
    ));
}

#[test]
fn test_violation_p3_dollar_metacharacter_returns_shell_error() {
    let result = abs_path("/path/with$VAR");
    assert!(matches!(
        result,
        Err(AbsolutePathError::ShellMetacharacter(
            ShellMetacharacterError::ContainsDollar { .. }
        ))
    ));
}

#[test]
fn test_violation_p3_backtick_metacharacter_returns_shell_error() {
    let result = abs_path("/path/with`cmd`");
    assert!(matches!(
        result,
        Err(AbsolutePathError::ShellMetacharacter(
            ShellMetacharacterError::ContainsBacktick { .. }
        ))
    ));
}

#[test]
fn test_violation_p3_semicolon_metacharacter_returns_shell_error() {
    let result = abs_path("/path/with;command");
    assert!(matches!(
        result,
        Err(AbsolutePathError::ShellMetacharacter(
            ShellMetacharacterError::ContainsSemicolon { .. }
        ))
    ));
}

#[test]
fn test_violation_p3_pipe_metacharacter_returns_shell_error() {
    let result = abs_path("/path/with|pipe");
    assert!(matches!(
        result,
        Err(AbsolutePathError::ShellMetacharacter(
            ShellMetacharacterError::ContainsPipe { .. }
        ))
    ));
}

#[test]
fn test_violation_p3_ampersand_metacharacter_returns_shell_error() {
    let result = abs_path("/path/with&bg");
    assert!(matches!(
        result,
        Err(AbsolutePathError::ShellMetacharacter(
            ShellMetacharacterError::ContainsAmpersand { .. }
        ))
    ));
}

#[test]
fn test_violation_p3_multiple_metacharacters_returns_dollar_error() {
    let result = abs_path("/path$with`and;many");
    assert!(matches!(
        result,
        Err(AbsolutePathError::ShellMetacharacter(
            ShellMetacharacterError::ContainsDollar { position: 5 }
        ))
    ));
}

// =========================================================================
// Given-When-Then Scenarios
// =========================================================================

#[test]
fn test_scenario_safe_absolute_path_creation() {
    let path_str = "/etc/nginx/nginx.conf";
    let result: Result<AbsolutePath, _> = path_str.try_into();
    assert!(result.is_ok());
    let absolute_path = result.unwrap();
    assert!(absolute_path.as_path().is_absolute());
}

#[test]
fn test_scenario_rejecting_shell_injection_attempt() {
    let malicious_path = "/etc/passwd;cat /etc/passwd";
    let result = abs_path(malicious_path);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        AbsolutePathError::ShellMetacharacter(_)
    ));
}

#[test]
fn test_scenario_rejecting_path_traversal_attempt() {
    let traversal_path = "../../../etc/shadow";
    let result = abs_path(traversal_path);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        AbsolutePathError::PathValidation(PathValidationError::NotAbsolute { .. })
    ));
}

#[test]
fn test_scenario_rejecting_variable_expansion_attempt() {
    let var_path = "/home/$USER/.ssh/id_rsa";
    let result = abs_path(var_path);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        AbsolutePathError::ShellMetacharacter(ShellMetacharacterError::ContainsDollar { .. })
    ));
}

// =========================================================================
// Integration Tests (Filesystem Operations)
// =========================================================================

#[test]
fn test_integration_write_and_read_file() {
    use std::io::Write;

    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path().to_path_buf();

    let file_path = temp_path.join("test_file.txt");
    let abs_file_path = AbsolutePath::try_from(file_path.as_path().to_string_lossy().as_ref())
        .expect("Should create absolute path from temp dir");

    let content = "Hello, world!";
    let mut file = std::fs::File::create(abs_file_path.as_path()).unwrap();
    file.write_all(content.as_bytes()).unwrap();
    drop(file);

    let read_content = std::fs::read_to_string(abs_file_path.as_path()).unwrap();
    assert_eq!(read_content, content);
}

#[test]
fn test_integration_create_directory() {
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path().to_path_buf();

    let test_dir = temp_path.join("test_dir");
    let abs_test_dir = AbsolutePath::try_from(test_dir.to_string_lossy().as_ref())
        .expect("Should create absolute path");

    std::fs::create_dir(abs_test_dir.as_path()).expect("Should create directory");

    assert!(abs_test_dir.as_path().is_dir());
}

#[test]
fn test_integration_file_exists_check() {
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path().to_path_buf();

    let test_file = temp_path.join("exists_test.txt");
    std::fs::write(&test_file, "content").unwrap();

    let abs_file = AbsolutePath::try_from(test_file.to_string_lossy().as_ref())
        .expect("Should create absolute path");

    assert!(abs_file.as_path().exists());
}

#[test]
fn test_integration_metadata_read() {
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path().to_path_buf();

    let test_file = temp_path.join("metadata_test.txt");
    std::fs::write(&test_file, "content").unwrap();

    let abs_file = AbsolutePath::try_from(test_file.to_string_lossy().as_ref())
        .expect("Should create absolute path");

    let metadata = std::fs::metadata(abs_file.as_path()).expect("Should read metadata");
    assert!(metadata.is_file());
}

#[test]
fn test_integration_to_path_buf_works_with_filesystem() {
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path().to_path_buf();

    let test_file = temp_path.join("hostname");
    std::fs::write(&test_file, "test-host").unwrap();

    let abs_file = AbsolutePath::try_from(test_file.to_string_lossy().as_ref())
        .expect("Should create absolute path");

    let content = std::fs::read_to_string(abs_file.to_path_buf()).expect("Should read");
    assert_eq!(content, "test-host");
}
