//! Safe temporary file handling examples for SCP workspace.
//!
//! This module demonstrates best practices for using the `tempfile` crate
//! to safely handle temporary files in the SCP workspace.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;

use tempfile::{tempdir, tempdir_in, NamedTempFile, TempDir};

/// Demonstrates basic temporary directory usage.
///
/// The `TempDir` is automatically cleaned up when it goes out of scope,
/// ensuring no temporary files are left behind.
#[test]
fn test_temp_directory_basic() {
    let temp_dir = tempdir().expect("failed to create temp directory");

    let file_path = temp_dir.path().join("test_file.txt");
    let mut file = File::create(&file_path).expect("failed to create temp file");

    writeln!(file, "Hello, SCP workspace!").expect("failed to write to temp file");

    assert!(file_path.exists());
    assert!(temp_dir.path().is_dir());

    drop(temp_dir);

    assert!(!file_path.exists());
}

/// Demonstrates temporary file with automatic cleanup.
///
/// `NamedTempFile` provides a temporary file that is automatically
/// cleaned up when dropped.
#[test]
fn test_named_temp_file() {
    let mut temp_file = NamedTempFile::new().expect("failed to create temp file");

    writeln!(temp_file, "Temporary content for workspace").expect("failed to write");

    let path = temp_file.path().to_path_buf();

    assert!(path.exists());

    let metadata = fs::metadata(&path).expect("failed to get metadata");
    assert!(metadata.is_file());

    drop(temp_file);

    assert!(!path.exists());
}

/// Demonstrates temporary directory in a specific location.
///
/// Use `tempdir_in` to create a temporary directory within a specific parent directory.
#[test]
fn test_temp_directory_in_location() {
    let parent = tempdir().expect("failed to create parent temp dir");
    let temp_dir = tempdir_in(parent.path()).expect("failed to create temp dir in location");

    let file_path = temp_dir.path().join("nested_temp.txt");
    File::create(&file_path).expect("failed to create nested temp file");

    assert!(file_path.exists());
    assert!(temp_dir.path().starts_with(parent.path()));

    drop(temp_dir);
    drop(parent);

    assert!(!file_path.exists());
}

/// Demonstrates persisting a temporary file.
///
/// Use `persist()` to move a temporary file to a permanent location.
/// This is useful for atomic operations where you want to write to a temp
/// file first, then atomically move it to the final location.
#[test]
fn test_persist_temp_file() {
    let mut temp_file = NamedTempFile::new().expect("failed to create temp file");

    writeln!(temp_file, "Final content").expect("failed to write");

    let final_path = temp_file.path().with_extension("txt");

    temp_file
        .persist(&final_path)
        .expect("failed to persist temp file");

    assert!(final_path.exists());

    fs::remove_file(&final_path).expect("failed to clean up");
}

/// Demonstrates atomic replacement with `persist()`.
///
/// This pattern is useful for safe configuration file updates:
/// 1. Write new config to temp file
/// 2. Atomically replace old config
#[test]
fn test_atomic_config_replace() {
    let config_path = "config_test.txt";

    let original_content = "original config";
    fs::write(config_path, original_content).expect("failed to write original config");

    let mut temp_file = NamedTempFile::new().expect("failed to create temp file");
    writeln!(temp_file, "updated config").expect("failed to write updated config");

    temp_file
        .persist(config_path)
        .expect("failed to atomically replace config");

    let new_content = fs::read_to_string(config_path).expect("failed to read new config");
    assert!(new_content.contains("updated config"));

    fs::remove_file(config_path).expect("failed to clean up");
}

/// Demonstrates using temp directory for workspace operations.
///
/// This is the recommended pattern for SCP workspace operations that
/// need temporary storage during migration or processing.
#[test]
fn test_workspace_temp_operations() {
    let workspace_dir = tempdir().expect("failed to create workspace temp dir");

    let input_dir = workspace_dir.path().join("input");
    let output_dir = workspace_dir.path().join("output");

    fs::create_dir(&input_dir).expect("failed to create input dir");
    fs::create_dir(&output_dir).expect("failed to create output dir");

    let input_file = input_dir.join("data.txt");
    fs::write(&input_file, "test data").expect("failed to write input data");

    let output_file = output_dir.join("processed.txt");
    fs::write(&output_file, "processed test data").expect("failed to write output data");

    assert!(input_dir.exists());
    assert!(output_dir.exists());
    assert!(input_file.exists());
    assert!(output_file.exists());

    drop(workspace_dir);

    assert!(!input_dir.exists());
    assert!(!output_dir.exists());
}

/// Demonstrates safe file handling with explicit cleanup on error.
///
/// Use patterns that ensure cleanup even when operations fail.
#[test]
fn test_safe_cleanup_on_error() {
    struct TempWorkspace {
        dir: TempDir,
    }

    impl TempWorkspace {
        fn new() -> io::Result<Self> {
            let dir = tempdir()?;
            Ok(Self { dir })
        }

        fn path(&self) -> &std::path::Path {
            self.dir.path()
        }
    }

    let workspace = TempWorkspace::new().expect("failed to create temp workspace");

    let file_path = workspace.path().join("data.txt");
    fs::write(&file_path, "important data").expect("failed to write data");

    assert!(file_path.exists());

    drop(workspace);

    assert!(!file_path.exists());
}

/// Demonstrates temp file with custom suffix/prefix.
///
/// While `tempfile` doesn't directly support suffixes, you can rename
/// the temp file after creation.
#[test]
fn test_temp_file_with_extension() {
    let temp_file = NamedTempFile::new().expect("failed to create temp file");

    let renamed = temp_file.path().with_extension("json");

    std::fs::rename(temp_file.path(), &renamed).expect("failed to rename temp file");

    assert!(renamed.exists());
    assert!(renamed.extension().unwrap() == "json");

    drop(renamed);
}
