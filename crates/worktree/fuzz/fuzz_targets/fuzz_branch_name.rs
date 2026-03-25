#![no_main]
use libfuzzer_sys::fuzz_target;
use worktree::domain::BranchName;

fuzz_target!(|data: String| {
    // Test that BranchName::new never panics
    let _ = BranchName::new(&data);
});
