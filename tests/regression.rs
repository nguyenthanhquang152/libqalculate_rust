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
            .args(["-t", "--", expression])
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

#[test]
fn reduced_unit_conversion_regressions_match_focused_oracle_cases() {
    let cases = [
        ("5 dm3 to L", "5 L"),
        ("25 dm^3 to L", "25 L"),
        ("20 miles / 2h to km/h", "16.09344 km/h"),
        ("1.74 to ft", "5 ft + 8.503937008 in"),
        ("1.74 m to ft", "5 ft + 8.503937008 in"),
        ("1.74 m to -ft", "5.708661417 ft"),
        ("100 lbf * 60 mph to hp", "15.99999752 hp"),
        ("50 Ω * 2 A", "100 V"),
        ("50 Ω * 2 A to base", "100 kg*m^2/(A*s^3)"),
        ("50 W * 2 s", "100 J"),
        ("10 N / 5 Pa", "2 m^2"),
        ("5 m/s to s/m", "0.2 s/m"),
        ("1000 bit to b?byte", "0.1220703125 KiB"),
        ("500 megabit/s * 2 h to b?byte", "419.0951586 GiB"),
    ];

    for (expression, expected) in cases {
        let output = assert_cmd::Command::cargo_bin("qalc-rs")
            .expect("qalc-rs binary")
            .args(["-t", "--", expression])
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

#[test]
fn test_latex_html_markup_terse_and_non_terse() {
    let mut calc = libqalculate_rust::ffi::Calculator::new();
    calc.load_global_definitions();

    // 1. Non-terse LaTeX
    let res = calc
        .calculate_and_print_qalc_latex_with_settings_and_fallback_state(
            "1/2 + sqrt(2)",
            &["precision 10"],
            1000,
        )
        .unwrap();
    assert_eq!(
        res.output,
        "$\\displaystyle \\frac{1}{2} + \\sqrt{2} \\approx \\num{1.914213562}$"
    );

    // 2. Terse LaTeX
    let res_terse = calc
        .calculate_and_print_qalc_latex_terse_with_settings_and_fallback_state(
            "1/2 + sqrt(2)",
            &["precision 10"],
            1000,
        )
        .unwrap();
    assert_eq!(res_terse.output, "$\\displaystyle \\num{1.914213562}$");

    // 3. Non-terse HTML
    let res_html = calc
        .calculate_and_print_qalc_html_with_settings_and_fallback_state(
            "1/2 + sqrt(2)",
            &["precision 10"],
            1000,
        )
        .unwrap();
    assert_eq!(res_html.output, "1 / 2 + √(2) ≈ 1.914213562");

    // 4. Terse HTML
    let res_html_terse = calc
        .calculate_and_print_qalc_html_terse_with_settings_and_fallback_state(
            "1/2 + sqrt(2)",
            &["precision 10"],
            1000,
        )
        .unwrap();
    assert_eq!(res_html_terse.output, "1.914213562");
}
