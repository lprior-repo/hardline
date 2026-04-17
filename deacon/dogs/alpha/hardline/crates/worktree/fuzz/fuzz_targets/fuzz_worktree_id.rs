#![no_main]
use libfuzzer_sys::fuzz_target;
use worktree::domain::WorktreeId;

fuzz_target!(|data: String| {
    // Test that WorktreeId::from_string never panics
    let _ = WorktreeId::from_string(&data);
});
