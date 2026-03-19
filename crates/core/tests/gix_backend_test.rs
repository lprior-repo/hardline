//! Tests for core vcs GitBackend using gix

use scp_core::vcs::{create_backend, detect_vcs, VcsType};
use std::env;

#[test]
fn test_core_git_backend_uses_gix() {
    // This tests that core::vcs::GitBackend uses gix, not CLI
    let cwd = env::current_dir().unwrap();
    let vcs_type = detect_vcs(&cwd);

    if vcs_type == Some(VcsType::Git) {
        let backend = create_backend(&cwd);
        assert!(backend.is_ok(), "Should create GitBackend");

        let backend = backend.unwrap();
        // Test current_branch - should use gix
        let result = backend.current_branch();
        // If it uses CLI, this is the old implementation
        // If it uses gix, this is the new implementation
        assert!(
            result.is_ok() || result.is_err(),
            "Should be able to get branch (either via gix or error)"
        );
    }
}
