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
fn cli_applies_precision_setting_for_native_log_and_sqrt_functions() {
    for (precision, expression, expected) in [
        (
            "64",
            "ln(2)",
            "0.6931471805599453094172321214581765680755001343602552541206800095\n",
        ),
        (
            "64",
            "sqrt(2)",
            "1.414213562373095048801688724209698078569671875376948073176679738\n",
        ),
        ("64", "sqrt(4)", "2\n"),
        ("64", "ln(0)", "−∞\n"),
        (
            "128",
            "ln(2)",
            "0.69314718055994530941723212145817656807550013436025525412068000949339362196969471560586332699641868754200148102057068573368552024\n",
        ),
        (
            "128",
            "sqrt(2)",
            "1.4142135623730950488016887242096980785696718753769480731766797379907324784621070388503875343276415727350138462309122970249248361\n",
        ),
        ("128", "sqrt(4)", "2\n"),
        ("128", "ln(0)", "−∞\n"),
    ] {
        let invalid_defs = tempdir().expect("temp dir should be created");
        let mut cmd = qalc_rs();
        cmd.args(["-set", &format!("precision {precision}"), "--", expression])
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
fn cli_applies_precision_setting_for_native_real_float_arithmetic() {
    for (precision, expression, expected) in [
        (
            "64",
            "(2 ^ 0.5) + (3 ^ 0.5)",
            "3.146264369941972342329135065715570445512477129187328701232486717\n",
        ),
        (
            "64",
            "(3 ^ 0.5) - (2 ^ 0.5)",
            "0.3178372451957822447257576172961742883731333784334325548791272415\n",
        ),
        (
            "64",
            "(2 ^ 0.5) * (3 ^ 0.5)",
            "2.449489742783178098197284074705891391965947480656670128432692567\n",
        ),
        (
            "64",
            "(3 ^ 0.5) / (2 ^ 0.5)",
            "1.224744871391589049098642037352945695982973740328335064216346284\n",
        ),
        (
            "64",
            "(2 ^ 0.5) + 1/3",
            "1.747546895706428382135022057543031411903005208710281406510013071\n",
        ),
        (
            "128",
            "(2 ^ 0.5) + (3 ^ 0.5)",
            "3.1462643699419723423291350657155704455124771291873287012324867174426654953709070759315337210848901484106399876463190000548947812\n",
        ),
        (
            "128",
            "(3 ^ 0.5) - (2 ^ 0.5)",
            "0.31783724519578224472575761729617428837313337843343255487912724146120053844669299823075865242960700294061229518449440600504510904\n",
        ),
        (
            "128",
            "(2 ^ 0.5) * (3 ^ 0.5)",
            "2.4494897427831780981972840747058913919659474806566701284326925672509603774573150265398594331046402348185946012266141891248588655\n",
        ),
        (
            "128",
            "(3 ^ 0.5) / (2 ^ 0.5)",
            "1.2247448713915890490986420373529456959829737403283350642163462836254801887286575132699297165523201174092973006133070945624294327\n",
        ),
        (
            "128",
            "(2 ^ 0.5) + 1/3",
            "1.7475468957064283821350220575430314119030052087102814065100130713240658117954403721837208676609749060683471795642456303582581694\n",
        ),
    ] {
        let invalid_defs = tempdir().expect("temp dir should be created");
        let mut cmd = qalc_rs();
        cmd.args(["-set", &format!("precision {precision}"), "--", expression])
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
fn cli_applies_precision_setting_for_native_decimal_scientific_float_arithmetic() {
    for precision in ["64", "128"] {
        for (expression, expected) in [
            ("0.1 + 0.2", "0.3\n"),
            ("1.25e-20 + 2.5e-20", "0.0000000000000000000375\n"),
            ("2.5e3 / 4", "625\n"),
        ] {
            let invalid_defs = tempdir().expect("temp dir should be created");
            let mut cmd = qalc_rs();
            cmd.args(["-set", &format!("precision {precision}"), "--", expression])
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
}

#[test]
fn cli_applies_precision_setting_for_native_real_float_comparisons() {
    for precision in ["64", "128"] {
        for (expression, expected) in [
            ("(2 ^ 0.5) < (3 ^ 0.5)", "true\n"),
            ("(2 ^ 0.5) = (2 ^ 0.5)", "true\n"),
            ("(2 ^ 0.5) = (3 ^ 0.5)", "false\n"),
            ("(2 ^ 0.5) + 1/3 > 1", "true\n"),
            ("(2 ^ 0.5) < 1/3", "false\n"),
        ] {
            let invalid_defs = tempdir().expect("temp dir should be created");
            let mut cmd = qalc_rs();
            cmd.args(["-set", &format!("precision {precision}"), "--", expression])
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
}

#[test]
fn cli_rejects_native_real_float_arithmetic_without_precision_setting() {
    for expression in [
        "(2 ^ 0.5) + (3 ^ 0.5)",
        "(3 ^ 0.5) - (2 ^ 0.5)",
        "(2 ^ 0.5) * (3 ^ 0.5)",
        "(3 ^ 0.5) / (2 ^ 0.5)",
        "(2 ^ 0.5) + 1/3",
        "ln(2) + sqrt(2)",
    ] {
        let invalid_defs = tempdir().expect("temp dir should be created");
        let mut cmd = qalc_rs();
        cmd.arg(expression)
            .env("QALCULATE_DEFINITIONS_DIR", invalid_defs.path())
            .env("QALCULATE_DISABLE_FALLBACK", "1")
            .env("QALCULATE_REPORT_FALLBACK", "1")
            .assert()
            .failure()
            .stderr(predicate::str::contains(
                "[qalc-rs-metadata] fallback=disabled",
            ))
            .stderr(predicate::str::contains(format!(
                "expression '{expression}' has no native Rust implementation"
            )));
    }
}

#[test]
fn cli_applies_interval_display_setting_for_native_interval_function() {
    for (expression, expected) in [
        ("interval(5;2)", "interval(2.000000000, 5.000000000)\n"),
        ("interval(1;3;0)", "interval(1.000000000, 3.000000000)\n"),
        ("interval(1;3;1)", "interval(1.000000000, 3.000000000)\n"),
    ] {
        let invalid_defs = tempdir().expect("temp dir should be created");
        let mut cmd = qalc_rs();
        cmd.args(["-set", "interval display 2", "--", expression])
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
fn cli_applies_interval_display_setting_for_native_infinity_interval_function() {
    for (expression, expected) in [
        (
            "interval(-infinity;5)",
            "interval(\u{2212}\u{221e}, 5.000000000)\n",
        ),
        ("interval(4;infinity)", "interval(4.000000000, +\u{221e})\n"),
        (
            "interval(-infinity;-4)",
            "\u{2212}interval(4.000000000, +\u{221e})\n",
        ),
        (
            "interval(-3;-1)",
            "\u{2212}interval(1.000000000, 3.000000000)\n",
        ),
    ] {
        let invalid_defs = tempdir().expect("temp dir should be created");
        let mut cmd = qalc_rs();
        cmd.args(["-set", "interval display 2", "--", expression])
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
fn cli_allows_ic2_for_native_infinity_interval_display_function() {
    for (expression, expected) in [
        (
            "interval(-infinity;5)",
            "interval(\u{2212}\u{221e}, 5.000000000)\n",
        ),
        ("interval(4;infinity)", "interval(4.000000000, +\u{221e})\n"),
        (
            "interval(-infinity;-4)",
            "\u{2212}interval(4.000000000, +\u{221e})\n",
        ),
        (
            "interval(-3;-1)",
            "\u{2212}interval(1.000000000, 3.000000000)\n",
        ),
    ] {
        let invalid_defs = tempdir().expect("temp dir should be created");
        let mut cmd = qalc_rs();
        cmd.args([
            "-set",
            "interval display 2",
            "-set",
            "ic 2",
            "--",
            expression,
        ])
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
fn cli_runs_native_interval_endpoint_functions() {
    for (expression, expected) in [
        ("lowerEndpoint(interval(1;3))", "1.000000000\n"),
        ("upperEndpoint(interval(1;3))", "3.000000000\n"),
        ("midpoint(interval(1;3))", "2\n"),
        ("lowerEndpoint(interval(1;3;1))", "1.000000000\n"),
        ("upperEndpoint(interval(1;3;1))", "3.000000000\n"),
        ("midpoint(interval(1;3;1))", "2\n"),
        (
            "lowerEndpoint(interval(-infinity;-4))",
            "\u{2212}\u{221e}\n",
        ),
        ("upperEndpoint(interval(4;infinity))", "+\u{221e}\n"),
    ] {
        let invalid_defs = tempdir().expect("temp dir should be created");
        let mut cmd = qalc_rs();
        cmd.args([
            "-set",
            "interval display 2",
            "-set",
            "ic 2",
            "--",
            expression,
        ])
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
fn cli_runs_native_interval_non_overlap_intersection() {
    let invalid_defs = tempdir().expect("temp dir should be created");
    let mut cmd = qalc_rs();
    cmd.args([
        "-set",
        "interval display 2",
        "-set",
        "ic 2",
        "--",
        "intersect(interval(1;2), interval(3;4))",
    ])
    .env("QALCULATE_DEFINITIONS_DIR", invalid_defs.path())
    .env("QALCULATE_DISABLE_FALLBACK", "1")
    .env("QALCULATE_REPORT_FALLBACK", "1")
    .assert()
    .success()
    .stdout("[]\n")
    .stderr(predicate::str::contains(
        "[qalc-rs-metadata] fallback=native",
    ));
}

#[test]
fn cli_runs_native_interval_arithmetic_with_ic2_endpoint_mode() {
    for (expression, expected) in [
        (
            "interval(1;2) + interval(3;4)",
            "interval(4.000000000, 6.000000000)\n",
        ),
        (
            "interval(3;4) - interval(1;2)",
            "interval(1.000000000, 3.000000000)\n",
        ),
        (
            "interval(-2;3) * interval(-4;5)",
            "interval(\u{2212}12.00000000, 15.00000000)\n",
        ),
        (
            "interval(4;6) / interval(2;3)",
            "interval(1.333333333, 3.000000000)\n",
        ),
        (
            "interval(-infinity;5) + interval(2;3)",
            "interval(\u{2212}\u{221e}, 8.000000000)\n",
        ),
        (
            "interval(-infinity;5) - interval(2;3)",
            "interval(\u{2212}\u{221e}, 3.000000000)\n",
        ),
        (
            "interval(-infinity;5) * interval(2;3)",
            "interval(\u{2212}\u{221e}, 15.00000000)\n",
        ),
        (
            "interval(4;infinity) + interval(2;3)",
            "interval(6.000000000, +\u{221e})\n",
        ),
        (
            "interval(4;infinity) - interval(2;3)",
            "interval(1.000000000, +\u{221e})\n",
        ),
        (
            "interval(4;infinity) * interval(2;3)",
            "interval(8.000000000, +\u{221e})\n",
        ),
        (
            "interval(4;infinity) / 2",
            "interval(2.000000000, +\u{221e})\n",
        ),
        (
            "interval(4;6) / interval(-3;-2)",
            "\u{2212}interval(1.333333333, 3.000000000)\n",
        ),
        (
            "interval(-6;-4) / interval(2;3)",
            "\u{2212}interval(1.333333333, 3.000000000)\n",
        ),
        (
            "interval(-6;-4) / interval(-3;-2)",
            "interval(1.333333333, 3.000000000)\n",
        ),
        (
            "interval(-infinity;-4) / 2",
            "\u{2212}interval(2.000000000, +\u{221e})\n",
        ),
        (
            "interval(-infinity;-4) / -2",
            "interval(2.000000000, +\u{221e})\n",
        ),
    ] {
        let invalid_defs = tempdir().expect("temp dir should be created");
        let mut cmd = qalc_rs();
        cmd.args([
            "-set",
            "interval display 2",
            "-set",
            "ic 2",
            "--",
            expression,
        ])
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
fn cli_rejects_infinity_interval_arithmetic_without_ic2_endpoint_mode() {
    let invalid_defs = tempdir().expect("temp dir should be created");
    let mut cmd = qalc_rs();
    cmd.args([
        "-set",
        "interval display 2",
        "--",
        "interval(4;infinity) + interval(2;3)",
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
        "expression 'interval(4;infinity) + interval(2;3)' has no native Rust implementation",
    ));
}

#[test]
fn cli_rejects_interval_arithmetic_without_ic2_endpoint_mode() {
    let invalid_defs = tempdir().expect("temp dir should be created");
    let mut cmd = qalc_rs();
    cmd.args([
        "-set",
        "interval display 2",
        "--",
        "interval(-2;3) * interval(-4;5)",
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
        "expression 'interval(-2;3) * interval(-4;5)' has no native Rust implementation",
    ));
}

#[test]
fn cli_rejects_symbolic_interval_division_and_intersection_rows() {
    for expression in [
        "interval(4;6) / interval(-1;1)",
        "interval(4;6) / interval(0;2)",
        "interval(4;infinity) / interval(2;4)",
        "2 / interval(4;infinity)",
        "intersect(interval(1;4), interval(3;6))",
    ] {
        let invalid_defs = tempdir().expect("temp dir should be created");
        let mut cmd = qalc_rs();
        cmd.args([
            "-set",
            "interval display 2",
            "-set",
            "ic 2",
            "--",
            expression,
        ])
        .env("QALCULATE_DEFINITIONS_DIR", invalid_defs.path())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "[qalc-rs-metadata] fallback=disabled",
        ))
        .stderr(predicate::str::contains(format!(
            "expression '{expression}' has no native Rust implementation"
        )));
    }
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
fn cli_runs_native_unicode_uncertainty_input() {
    for (expression, expected) in [
        ("2 +/- 0.002", "2.0000±0.0020\n"),
        ("2 +/- 0.002 + 3", "5.0000±0.0020\n"),
        ("2±0.002", "2.0000±0.0020\n"),
        ("2±0.002 + 3", "5.0000±0.0020\n"),
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
fn cli_runs_native_uncertainty_api_functions() {
    for (expression, expected) in [
        ("uncertainty(2;0.002;0)", "2.0000±0.0020\n"),
        ("uncertainty(100;0.05;1)", "100.0±5.0\n"),
        ("uncertainty(10;0;0)", "10\n"),
        ("errorPart(2+/-0.002)", "0.002000000000\n"),
        ("errorPart(100+/-5%)", "5\n"),
        ("errorPart(2+/-0.002;0)", "0.002000000000\n"),
        ("errorPart(2+/-0.002;1)", "0.001000000000\n"),
        ("errorPart(100+/-5%;0)", "5\n"),
        ("errorPart(100+/-5%;1)", "0.05000000000\n"),
        ("valuePart(2+/-0.002)", "2\n"),
        ("valuePart(100+/-5%)", "100\n"),
        ("midpoint(2+/-0.002)", "2\n"),
        ("lowerEndpoint(2+/-0.002)", "1.998000000\n"),
        ("upperEndpoint(2+/-0.002)", "2.002000000\n"),
        ("20+/-3 - 10+/-4", "10.0±5.0\n"),
        ("3+/-0.2 / 4+/-0.1", "0.750±0.053\n"),
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
fn cli_applies_concise_uncertainty_setting_for_native_input() {
    for (expression, expected) in [
        ("1.23(4)", "1.230±0.040\n"),
        ("123(4)", "123.0±4.0\n"),
        ("1.23(4) + 2.0(3)", "3.23±0.30\n"),
    ] {
        let invalid_defs = tempdir().expect("temp dir should be created");
        let mut cmd = qalc_rs();
        cmd.args(["-set", "concise uncertainty 1", "--", expression])
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
fn cli_rejects_concise_uncertainty_without_setting() {
    let invalid_defs = tempdir().expect("temp dir should be created");
    let mut cmd = qalc_rs();
    cmd.arg("1.23(4)")
        .env("QALCULATE_DEFINITIONS_DIR", invalid_defs.path())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "[qalc-rs-metadata] fallback=disabled",
        ))
        .stderr(predicate::str::contains(
            "expression '1.23(4)' has no native Rust implementation",
        ));
}

#[test]
fn cli_runs_native_complex_subtraction_conjugate_and_norm() {
    for (expression, expected) in [
        ("(1 + 2i) - (3 + 4i)", "−2 − 2i\n"),
        ("i + (-i)", "0\n"),
        ("(1 + i) + (-1 + i)", "2i\n"),
        ("(1 + i) + (2 - i)", "3\n"),
        ("(1 + i) * (1 - i)", "2\n"),
        ("(1 + i) / (1 - i)", "i\n"),
        ("conj(3 + 4i)", "3 − 4i\n"),
        ("conj(i)", "−i\n"),
        ("conj(-i)", "i\n"),
        ("conj(3)", "3\n"),
        ("norm(3 + 4i)", "5\n"),
        ("norm(i)", "1\n"),
        ("norm(-3i)", "3\n"),
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
fn cli_runs_native_complex_equality_constraints() {
    for (expression, expected) in [
        ("(1 + i) = (1 + i)", "true\n"),
        ("(1 + i) == (1 + i)", "true\n"),
        ("(1 + i) = (1 - i)", "false\n"),
        ("(1 + i) != (1 - i)", "true\n"),
        ("(1 + i) ≠ (1 - i)", "true\n"),
        ("(1 + i) != (1 + i)", "false\n"),
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
fn cli_runs_native_complex_ordering_constraints() {
    for (expression, expected) in [
        ("(1 + i) < (1 + i)", "false\n"),
        ("(1 + i) <= (1 + i)", "true\n"),
        ("(1 + i) > (1 + i)", "false\n"),
        ("(1 + i) >= (1 + i)", "true\n"),
        ("(1 + i) ≤ (1 + i)", "true\n"),
        ("(1 + i) ≥ (1 + i)", "true\n"),
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
fn cli_runs_native_float_log_and_sqrt_functions() {
    for (expression, expected) in [
        ("ln(0)", "−∞\n"),
        ("ln(2)", "0.6931471806\n"),
        ("ln(5+/-0.3)", "1.609±0.060\n"),
        ("sqrt(2)", "1.414213562\n"),
        ("sqrt(4)", "2\n"),
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
fn cli_runs_native_infinity_arithmetic() {
    for (expression, expected) in [
        ("infinity", "+∞\n"),
        ("-infinity", "−∞\n"),
        ("infinity + 1", "+∞\n"),
        ("-infinity - 1", "−∞\n"),
        ("infinity * 2", "+∞\n"),
        ("infinity * -2", "−∞\n"),
        ("1 / infinity", "0\n"),
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
fn cli_runs_native_signed_infinity_division() {
    for (expression, expected) in [
        ("infinity / 2", "+∞\n"),
        ("infinity / -2", "−∞\n"),
        ("-infinity / 2", "−∞\n"),
        ("-infinity / -2", "+∞\n"),
        ("1 / -infinity", "0\n"),
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
fn cli_rejects_unsupported_uncertainty_special_function_when_fallback_disabled() {
    let invalid_defs = tempdir().expect("temp dir should be created");
    let mut cmd = qalc_rs();
    cmd.arg("Ei(3+/-0.3)")
        .env("QALCULATE_DEFINITIONS_DIR", invalid_defs.path())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "[qalc-rs-metadata] fallback=disabled",
        ))
        .stderr(predicate::str::contains(
            "expression 'Ei(3+/-0.3)' has no native Rust implementation",
        ));
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
