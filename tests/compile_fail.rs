//! Regression tests for things that should not compile but used to

// trybuild runs cargo, and WASI has no processes.
#[test]
#[cfg_attr(target_os = "wasi", ignore)]
fn test_compile_fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/remote_refspec.rs");
}
