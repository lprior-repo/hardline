#![no_main]
use libfuzzer_sys::fuzz_target;
use worktree::domain::WorktreeName;

fuzz_target!(|data: String| {
    // Test that WorktreeName::new never panics
    let _ = WorktreeName::new(&data);
});
