use assert_cmd::Command;
use predicates::prelude::*;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn cli_prints_version() {
    let mut cmd = qalc_rs();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("upstream libqalculate 5.11.0"));
}

#[test]
fn cli_prints_help() {
    let mut cmd = qalc_rs();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--parse-batch <path>"))
        .stdout(predicate::str::contains(
            "Limited native-evidence qalc setting support",
        ));
}

#[test]
fn cli_self_check_finds_upstream_fixtures() {
    if !Path::new("../libqalculate/tests").exists() {
        eprintln!("skipping upstream fixture e2e test; ../libqalculate/tests is unavailable");
        return;
    }

    let mut cmd = qalc_rs();
    cmd.arg("--self-check")
        .assert()
        .success()
        .stdout(predicate::str::contains("upstream_batch_files="));
}

#[test]
fn cli_self_check_uses_configured_upstream_dir() {
    let upstream = fake_upstream();
    let mut cmd = qalc_rs();
    cmd.arg("--self-check")
        .env("LIBQALCULATE_UPSTREAM_DIR", upstream.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("upstream_batch_files=1"));
}

#[test]
fn cli_lists_only_batch_fixtures_from_configured_upstream_dir() {
    let upstream = fake_upstream();
    let mut cmd = qalc_rs();
    let output = cmd
        .arg("--list-upstream-tests")
        .env("LIBQALCULATE_UPSTREAM_DIR", upstream.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    assert!(output.contains("smoke.batch"));
    assert!(!output.contains("notes.txt"));
}

#[test]
fn cli_parse_batch_reports_case_count() {
    let mut cmd = qalc_rs();
    cmd.arg("--parse-batch")
        .arg("tests/fixtures/regression/basic_numbers.batch")
        .assert()
        .success()
        .stdout(predicate::str::contains("cases=4"));
}

#[test]
fn cli_evaluates_positional_expression_via_fallback() {
    let mut cmd = qalc_rs();
    cmd.arg("1 + 1")
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .assert()
        .success()
        .stdout("2\n");
}

#[test]
fn cli_evaluates_negative_expression_after_separator() {
    let mut cmd = qalc_rs();
    cmd.args(["--", "-0"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .assert()
        .success()
        .stdout("0\n");
}

#[test]
fn cli_evaluates_negative_expression_without_separator() {
    let mut cmd = qalc_rs();
    cmd.arg("-1")
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .assert()
        .success()
        .stdout("\u{2212}1\n");
}

#[test]
fn cli_evaluates_negative_decimal_without_separator() {
    let mut cmd = qalc_rs();
    cmd.arg("-.5")
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .assert()
        .success()
        .stdout("\u{2212}0.5\n");
}

#[test]
fn cli_reports_definition_load_failure_for_expressions() {
    let invalid_defs = tempdir().expect("temp dir should be created");
    let mut cmd = qalc_rs();
    cmd.arg("1 + 1")
        .env("QALCULATE_DEFINITIONS_DIR", invalid_defs.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "failed to load global definitions",
        ));
}

#[test]
fn cli_native_scaffold_does_not_require_definitions_when_fallback_disabled() {
    let invalid_defs = tempdir().expect("temp dir should be created");
    let mut cmd = qalc_rs();
    cmd.arg("1 + 2")
        .env("QALCULATE_DEFINITIONS_DIR", invalid_defs.path())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .assert()
        .success()
        .stdout("3\n")
        .stderr(predicate::str::contains(
            "[qalc-rs-metadata] fallback=native",
        ));
}

#[test]
fn cli_applies_limited_set_for_native_numberbase_evidence() {
    let invalid_defs = tempdir().expect("temp dir should be created");
    let mut cmd = qalc_rs();
    cmd.args(["-set", "input base 16", "--", "5p10+AEp-2*p23"])
        .env("QALCULATE_DEFINITIONS_DIR", invalid_defs.path())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .assert()
        .success()
        .stdout("364909568\n")
        .stderr(predicate::str::contains(
            "[qalc-rs-metadata] fallback=native",
        ));
}

#[test]
fn cli_applies_precision_setting_for_native_rational_output() {
    let invalid_defs = tempdir().expect("temp dir should be created");
    let mut cmd = qalc_rs();
    cmd.args(["-set", "precision 128", "--", "1/3"])
        .env("QALCULATE_DEFINITIONS_DIR", invalid_defs.path())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .assert()
        .success()
        .stdout(
            "0.33333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333\n",
        )
        .stderr(predicate::str::contains(
            "[qalc-rs-metadata] fallback=native",
        ));
}

#[test]
fn cli_applies_precision_setting_for_native_float_power() {
    let invalid_defs = tempdir().expect("temp dir should be created");
    let mut cmd = qalc_rs();
    cmd.args(["-set", "precision 128", "--", "2 ^ 0.5"])
        .env("QALCULATE_DEFINITIONS_DIR", invalid_defs.path())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .assert()
        .success()
        .stdout(
            "1.4142135623730950488016887242096980785696718753769480731766797379907324784621070388503875343276415727350138462309122970249248361\n",
        )
        .stderr(predicate::str::contains(
            "[qalc-rs-metadata] fallback=native",
        ));
}

#[test]
fn cli_applies_interval_display_setting_for_native_interval_function() {
    let invalid_defs = tempdir().expect("temp dir should be created");
    let mut cmd = qalc_rs();
    cmd.args(["-set", "interval display 2", "--", "interval(5;2)"])
        .env("QALCULATE_DEFINITIONS_DIR", invalid_defs.path())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .assert()
        .success()
        .stdout("interval(2.000000000, 5.000000000)\n")
        .stderr(predicate::str::contains(
            "[qalc-rs-metadata] fallback=native",
        ));
}

#[test]
fn cli_requires_interval_display_setting_for_native_interval_function() {
    let invalid_defs = tempdir().expect("temp dir should be created");
    let mut cmd = qalc_rs();
    cmd.arg("interval(5;2)")
        .env("QALCULATE_DEFINITIONS_DIR", invalid_defs.path())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "[qalc-rs-metadata] fallback=disabled",
        ))
        .stderr(predicate::str::contains(
            "expression 'interval(5;2)' has no native Rust implementation",
        ));
}

#[test]
fn cli_rejects_interval_display_setting_for_unrelated_native_evidence() {
    let invalid_defs = tempdir().expect("temp dir should be created");
    let mut cmd = qalc_rs();
    cmd.args(["-set", "interval display 2", "--", "1 + 2"])
        .env("QALCULATE_DEFINITIONS_DIR", invalid_defs.path())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "[qalc-rs-metadata] fallback=disabled",
        ))
        .stderr(predicate::str::contains(
            "expression '1 + 2' has no native Rust implementation",
        ));
}

#[test]
fn cli_rejects_interval_display_setting_for_numberbase_evidence() {
    let invalid_defs = tempdir().expect("temp dir should be created");
    let mut cmd = qalc_rs();
    cmd.args([
        "-set",
        "interval display 2",
        "-set",
        "input base 16",
        "--",
        "5p10+AEp-2*p23",
    ])
    .env("QALCULATE_DEFINITIONS_DIR", invalid_defs.path())
    .env("QALCULATE_DISABLE_FALLBACK", "1")
    .env("QALCULATE_REPORT_FALLBACK", "1")
    .assert()
    .failure()
    .stderr(predicate::str::contains(
        "[qalc-rs-metadata] fallback=disabled",
    ))
    .stderr(predicate::str::contains(
        "expression '5p10+AEp-2*p23' has no native Rust implementation",
    ));
}

#[test]
fn cli_runs_native_uncertainty_power() {
    let invalid_defs = tempdir().expect("temp dir should be created");
    let mut cmd = qalc_rs();
    cmd.arg("(2+/-3)^3.2")
        .env("QALCULATE_DEFINITIONS_DIR", invalid_defs.path())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .assert()
        .success()
        .stdout("9.18958684±44.11001683\n")
        .stderr(predicate::str::contains(
            "[qalc-rs-metadata] fallback=native",
        ));
}

#[test]
fn cli_runs_native_complex_subtraction_conjugate_and_norm() {
    for (expression, expected) in [
        ("(1 + 2i) - (3 + 4i)", "−2 − 2i\n"),
        ("conj(3 + 4i)", "3 − 4i\n"),
        ("norm(3 + 4i)", "5\n"),
    ] {
        let invalid_defs = tempdir().expect("temp dir should be created");
        let mut cmd = qalc_rs();
        cmd.arg(expression)
            .env("QALCULATE_DEFINITIONS_DIR", invalid_defs.path())
            .env("QALCULATE_DISABLE_FALLBACK", "1")
            .env("QALCULATE_REPORT_FALLBACK", "1")
            .assert()
            .success()
            .stdout(expected)
            .stderr(predicate::str::contains(
                "[qalc-rs-metadata] fallback=native",
            ));
    }
}

#[test]
fn cli_runs_native_complex_powers() {
    for (expression, expected) in [
        ("i^2", "−1\n"),
        ("(2i - 3)^(3.2i + 3)", "0.009212545193 − 0.009517560625i\n"),
    ] {
        let invalid_defs = tempdir().expect("temp dir should be created");
        let mut cmd = qalc_rs();
        cmd.arg(expression)
            .env("QALCULATE_DEFINITIONS_DIR", invalid_defs.path())
            .env("QALCULATE_DISABLE_FALLBACK", "1")
            .env("QALCULATE_REPORT_FALLBACK", "1")
            .assert()
            .success()
            .stdout(expected)
            .stderr(predicate::str::contains(
                "[qalc-rs-metadata] fallback=native",
            ));
    }
}

#[test]
fn cli_rejects_unknown_arguments() {
    let mut cmd = qalc_rs();
    cmd.arg("--definitely-unknown")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown argument"));
}

fn qalc_rs() -> Command {
    let mut cmd = Command::cargo_bin("qalc-rs").expect("binary should build");
    cmd.env_remove("QALCULATE_DISABLE_FALLBACK")
        .env_remove("QALCULATE_REPORT_FALLBACK");
    cmd
}

fn fake_upstream() -> tempfile::TempDir {
    let dir = tempdir().expect("temp dir should be created");
    let tests = dir.path().join("tests");
    std::fs::create_dir(&tests).expect("tests directory should be created");
    std::fs::write(tests.join("smoke.batch"), "1\n\t1\n").expect("batch fixture should be written");
    std::fs::write(tests.join("notes.txt"), "not a batch\n")
        .expect("non-batch file should be written");
    dir
}

fn definitions_dir() -> &'static str {
    "../libqalculate/data"
}
