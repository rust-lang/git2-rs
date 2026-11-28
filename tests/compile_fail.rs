//! Regression tests for things that should not compile but used to

#[test]
fn test_compile_fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/odb_finalize_repeated.rs");
    t.compile_fail("tests/compile_fail/remote_refspec.rs");
    t.compile_fail("tests/compile_fail/diff_send.rs");
    t.compile_fail("tests/compile_fail/repo_namespace_remove.rs");
    t.compile_fail("tests/compile_fail/repo_namespace_set_bytes.rs");
    t.compile_fail("tests/compile_fail/repo_namespace_set.rs");
    t.compile_fail("tests/compile_fail/repo_workdir_set.rs");
    t.compile_fail("tests/compile_fail/opts_unsafe_setters.rs");
}
