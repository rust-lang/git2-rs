//! Regression tests for things that should not compile but used to

#[test]
fn test_compile_fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/remote_refspec.rs");
}
