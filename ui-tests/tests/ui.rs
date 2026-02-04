#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");

    // derive_persistable tests
    t.pass("tests/ui/persistable/pass/*.rs");
    t.compile_fail("tests/ui/persistable/fail/*.rs");

    // doctest macro tests
    t.pass("tests/ui/doctest/pass/*.rs");
}
