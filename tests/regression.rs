use libqalculate_rust::batch::read_batch_cases;
use libqalculate_rust::context::CalculatorContext;
use std::path::{Path, PathBuf};

fn definitions_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../libqalculate/data")
}

#[test]
fn local_regression_fixture_is_stable() {
    let cases = read_batch_cases("tests/fixtures/regression/basic_numbers.batch")
        .expect("local regression fixture should parse");
    assert_eq!(cases.len(), 4);
    assert_eq!(cases[0].expression, "0");
    assert_eq!(cases[0].expected, ["0"]);
}

#[test]
fn reduced_dataset_lookup_regressions_match_focused_oracle_cases() {
    let mut context = CalculatorContext::new();
    let cases = [
        ("atom(H; mass)", "1.008 u"),
        ("atom(He; mass)", "4.00260 u"),
        ("atom(H; name)", "\"Hydrogen\""),
        ("atom(1; symbol)", "'H'"),
        ("atom(Hydrogen; number)", "1"),
        ("planet(Earth; radius)", "6371.0 km"),
        ("planet(Earth; gravity)", "9.80665 m/s²"),
        ("planet(Mars; mass)", "6.4171E23 kg"),
        ("planet(Pluto; mass)", "1.3E22 kg"),
    ];

    for (expression, expected) in cases {
        context.clear_messages();
        let actual = context
            .parse_and_evaluate_to_string(expression)
            .unwrap_or_else(|error| panic!("{expression:?} failed: {error}"));
        assert_eq!(actual, expected, "{expression}");
    }
}

#[test]
fn reduced_currency_conversion_regressions_match_focused_oracle_cases() {
    let cases = [
        ("1 EUR to USD", "$1.164800000"),
        ("1 USD to EUR", "€0.8585164835"),
        ("10 USD to EUR", "€8.585164835"),
        ("1 EUR to JPY", "¥184.9300000"),
        ("1 BTC to EUR", "€66025.70000"),
        ("0 EUR to USD", "$0"),
    ];

    for (expression, expected) in cases {
        let output = assert_cmd::Command::cargo_bin("qalc-rs")
            .expect("qalc-rs binary")
            .args(["--", expression])
            .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
            .env("QALCULATE_DISABLE_FALLBACK", "1")
            .env("QALCULATE_REPORT_FALLBACK", "1")
            .assert()
            .success()
            .get_output()
            .clone();
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            expected,
            "{expression}"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("fallback=native"),
            "{expression} should run natively"
        );
    }
}
