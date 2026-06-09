use libqalculate_rust::batch::read_batch_cases;

#[test]
fn local_regression_fixture_is_stable() {
    let cases = read_batch_cases("tests/fixtures/regression/basic_numbers.batch")
        .expect("local regression fixture should parse");
    assert_eq!(cases.len(), 4);
    assert_eq!(cases[0].expression, "0");
    assert_eq!(cases[0].expected, ["0"]);
}
