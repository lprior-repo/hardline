#![no_main]
use libfuzzer_sys::fuzz_target;
use worktree::domain::AbsolutePath;

fuzz_target!(|data: String| {
    // Test that AbsolutePath::new never panics
    let _ = AbsolutePath::new(&data);
});
