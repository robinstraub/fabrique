#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");

    // derive_persistable
    t.pass("tests/ui/persistable/pass/*.rs");
    t.compile_fail("tests/ui/persistable/fail/*.rs");
}
