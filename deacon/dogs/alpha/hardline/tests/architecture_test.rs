//! Architecture boundary compile-fail tests
//! These tests prove that domain crates cannot import infrastructure.

#[test]
fn architecture_boundaries() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}
