#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");

    // derive_persistable
    t.pass("tests/derive_persistable/pass/*.rs");
    t.compile_fail("tests/derive_persistable/fail/*.rs");
}
