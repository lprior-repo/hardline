use std::env;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn get_bin_path() -> PathBuf {
    let mut path = env::current_exe().unwrap();
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.join("scp-cli")
}

#[test]
fn test_cli_end_to_end() {
    let temp_dir = TempDir::new().unwrap();
    let scp_bin = get_bin_path();

    // Check that we can run the binary and get help
    let output = Command::new(&scp_bin)
        .arg("--help")
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute scp binary");

    assert!(output.status.success(), "scp --help failed: {:?}", output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage:"));
}

// Re-export all test modules
pub mod clean_integration;
pub mod cli;
pub mod lock_integration;
pub mod red_queen_adversarial;
pub mod workspace_switch_tests;
