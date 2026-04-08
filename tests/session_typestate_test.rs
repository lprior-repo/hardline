//! Session typestate compile-fail tests.
//!
//! These tests prove that invalid Session state transitions are rejected
//! at compile time by the typestate pattern. Each compile_fail test file
//! demonstrates that calling a transition method on the wrong state type
//! produces a compilation error.

#[test]
fn session_typestate_invalid_transitions() {
    let t = trybuild::TestCases::new();
    t.compile_fail("compile_fail/typestate_*.rs");
}
