use assert_cmd::Command;
use libqalculate_rust::UPSTREAM_LIBQALCULATE_VERSION;
use predicates::prelude::*;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

#[test]
fn cli_prints_version() {
    let mut cmd = qalc_rs_raw();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(format!("{UPSTREAM_LIBQALCULATE_VERSION}\n"));
}

#[test]
fn cli_prints_help() {
    let mut cmd = qalc_rs_raw();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(format!("{}\n", include_str!("../src/cli/help.txt")));
}

#[test]
fn cli_self_check_finds_upstream_fixtures() {
    if !Path::new("../libqalculate/tests").exists() {
        eprintln!("skipping upstream fixture e2e test; ../libqalculate/tests is unavailable");
        return;
    }

    let mut cmd = qalc_rs_raw();
    cmd.arg("--self-check")
        .assert()
        .success()
        .stdout(predicate::str::contains("upstream_batch_files="));
}

#[test]
fn cli_self_check_uses_configured_upstream_dir() {
    let upstream = fake_upstream();
    let mut cmd = qalc_rs_raw();
    cmd.arg("--self-check")
        .env("LIBQALCULATE_UPSTREAM_DIR", upstream.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("upstream_batch_files=1"));
}

#[test]
fn cli_lists_only_batch_fixtures_from_configured_upstream_dir() {
    let upstream = fake_upstream();
    let mut cmd = qalc_rs_raw();
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
    let mut cmd = qalc_rs_raw();
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
fn cli_formats_native_message_functions_with_fallback_disabled() {
    let mut warning = qalc_rs();
    warning
        .arg("--")
        .arg(r#"warning("first")"#)
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .assert()
        .success()
        .stdout("warning: first\n0\n")
        .stderr(predicate::str::contains(
            "[qalc-rs-metadata] fallback=native",
        ));

    let mut info = qalc_rs();
    info.arg("--")
        .arg(r#"message("second")"#)
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .assert()
        .success()
        .stdout("second\n0\n")
        .stderr(predicate::str::contains(
            "[qalc-rs-metadata] fallback=native",
        ));

    let mut error = qalc_rs();
    error
        .arg("--")
        .arg(r#"error("third")"#)
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .assert()
        .code(1)
        .stdout("error: third\n0\n")
        .stderr(predicate::str::contains(
            "[qalc-rs-metadata] fallback=native",
        ));
}

#[test]
fn cli_formats_cpp_fallback_messages_before_non_terse_equation() {
    let mut warning = qalc_rs_raw();
    warning
        .args(["--", r#"warning("first")"#])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .assert()
        .success()
        .stdout("warning: first\nwarning(\"first\") = 0\n")
        .stderr(predicate::str::contains(
            "[qalc-rs-metadata] fallback=cpp-fallback-enabled",
        ));
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
fn cli_applies_input_base_only_to_integer_and_word_operator_expressions() {
    for (expression, expected) in [
        ("10+1", "17\n"),
        ("A xor B", "1\n"),
        ("A and B", "1\n"),
        ("A or B", "1\n"),
        ("A mod 3", "1\n"),
        ("A div 3", "3\n"),
        ("A rem 3", "1\n"),
    ] {
        let mut cmd = qalc_rs_raw();
        cmd.args(["-t", "-set", "input base 16", "--", expression])
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
fn cli_two_part_base_uses_upstream_output_then_input_order() {
    let invalid_defs = tempdir().expect("temp dir should be created");
    let mut cmd = qalc_rs_raw();
    cmd.args(["-t", "-b", "10 16", "10"])
        .env("QALCULATE_DEFINITIONS_DIR", invalid_defs.path())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .assert()
        .success()
        .stdout("16\n")
        .stderr("");
}

#[test]
fn cli_applies_unicode_on_setting_for_native_sexagesimal_output() {
    let invalid_defs = tempdir().expect("temp dir should be created");
    let mut cmd = qalc_rs();
    cmd.args(["-set", "/set unicode 1", "--", "52.34 to sexa"])
        .env("QALCULATE_DEFINITIONS_DIR", invalid_defs.path())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .assert()
        .success()
        .stdout("52°20′24″\n")
        .stderr(predicate::str::contains(
            "[qalc-rs-metadata] fallback=native",
        ));
}

#[test]
fn cli_applies_unicode_off_setting_for_native_sexagesimal_output() {
    let invalid_defs = tempdir().expect("temp dir should be created");
    let mut cmd = qalc_rs();
    cmd.args(["-set", "/set unicode 0", "--", "52.34 to sexa"])
        .env("QALCULATE_DEFINITIONS_DIR", invalid_defs.path())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .assert()
        .success()
        .stdout("52o20'24\"\n")
        .stderr(predicate::str::contains(
            "[qalc-rs-metadata] fallback=native",
        ));
}

#[test]
fn cli_applies_u8_flags_for_native_sexagesimal_output() {
    let invalid_defs = tempdir().expect("temp dir should be created");
    let mut unicode_cmd = qalc_rs();
    unicode_cmd
        .args(["-u8", "--", "52.34 to sexa"])
        .env("QALCULATE_DEFINITIONS_DIR", invalid_defs.path())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .assert()
        .success()
        .stdout("52°20′24″\n")
        .stderr(predicate::str::contains(
            "[qalc-rs-metadata] fallback=native",
        ));

    let mut ascii_cmd = qalc_rs();
    ascii_cmd
        .args(["+u8", "--", "52.34 to sexa"])
        .env("QALCULATE_DEFINITIONS_DIR", invalid_defs.path())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .assert()
        .success()
        .stdout("52o20'24\"\n")
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
fn cli_applies_print_option_settings_for_native_number_formatting() {
    for (setting, expression, expected) in [
        ("exp 0", "10000000000000", "10000000000000\n"),
        ("exp -3", "10000", "10E3\n"),
        ("edisp 2", "12345678901234", "1.234567890 × 10^13\n"),
        ("edisp 2", "10000000000000", "10^13\n"),
        ("max decimals 2", "2/3", "0.67\n"),
        ("max decimals 2", "12345678901234", "1.23E13\n"),
        ("min decimals 4", "1.2", "1.2000\n"),
        ("min decimals 2", "10000000000000", "1.00E13\n"),
    ] {
        let invalid_defs = tempdir().expect("temp dir should be created");
        let mut cmd = qalc_rs();
        cmd.args(["-set", setting, "--", expression])
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
fn cli_rejects_print_option_setting_for_unvetted_native_number_formatting() {
    let invalid_defs = tempdir().expect("temp dir should be created");
    let mut cmd = qalc_rs();
    cmd.args(["-set", "exp 3", "--", "i"])
        .env("QALCULATE_DEFINITIONS_DIR", invalid_defs.path())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "[qalc-rs-metadata] fallback=disabled",
        ))
        .stderr(predicate::str::contains(
            "expression 'i' has no native Rust implementation",
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
    let mut cmd = qalc_rs_raw();
    cmd.arg("--definitely-unknown").assert().failure();
}

#[test]
fn cli_formats_native_latex_markup_with_fallback_disabled() {
    let mut cmd = qalc_rs_raw();
    cmd.args(["--latex", "-set", "precision 10", "--", "1/2 + sqrt(2)"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .assert()
        .success()
        .stdout("$\\displaystyle \\frac{1}{2} + \\sqrt{2} \\approx \\num{1.914213562}$\n")
        .stderr(predicate::str::contains(
            "[qalc-rs-metadata] fallback=native",
        ));
}

#[test]
fn cli_formats_native_html_markup_with_fallback_disabled() {
    let mut cmd = qalc_rs_raw();
    cmd.args(["--html", "-set", "precision 10", "--", "1/2 + sqrt(2)"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .assert()
        .success()
        .stdout("1 / 2 + √(2) ≈ 1.914213562\n")
        .stderr(predicate::str::contains(
            "[qalc-rs-metadata] fallback=native",
        ));
}

#[test]
fn cli_native_markup_preserves_evaluation_messages() {
    let mut cmd = qalc_rs_raw();
    cmd.args(["--html", "--", "acosh(0.5)"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .assert()
        .success()
        .stdout("warning: acosh: argument must be >= 1\nacosh(0.5) ≈ nan\n")
        .stderr(predicate::str::contains(
            "[qalc-rs-metadata] fallback=native",
        ));

    let mut terse = qalc_rs_raw();
    terse
        .args(["-t", "--html", "--", "acosh(0.5)"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .assert()
        .success()
        .stdout("nan\n")
        .stderr("");
}

#[test]
fn cli_formats_native_html_symbolic_comparison_markup() {
    let mut cmd = qalc_rs_raw();
    cmd.args(["--html", "-set", "assumptions unknown", "--", "x<y"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .assert()
        .success()
        .stdout("(<i>x</i> &lt; <i>y</i>) = (<i>x</i> &lt; <i>y</i>)\n")
        .stderr(predicate::str::contains(
            "[qalc-rs-metadata] fallback=native",
        ));
}

#[test]
fn cli_supports_to_latex_conversion_markup_with_fallback_disabled() {
    let mut cmd = qalc_rs_raw();
    cmd.args(["-set", "precision 10", "--", "1/2 + sqrt(2) to latex"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .assert()
        .success()
        .stdout("$\\displaystyle \\frac{1}{2} + \\sqrt{2} \\approx \\num{1.914213562}$\n")
        .stderr(predicate::str::contains(
            "[qalc-rs-metadata] fallback=native",
        ));
}

#[test]
fn cli_supports_to_html_conversion_markup_with_fallback_disabled() {
    let mut cmd = qalc_rs_raw();
    cmd.args(["-set", "assumptions unknown", "--", "(x<y) to html"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .assert()
        .success()
        .stdout("(<i>x</i> &lt; <i>y</i>) = (<i>x</i> &lt; <i>y</i>)\n")
        .stderr(predicate::str::contains(
            "[qalc-rs-metadata] fallback=native",
        ));
}

fn qalc_rs() -> Command {
    let mut cmd = Command::cargo_bin("qalc-rs").expect("binary should build");
    cmd.env_remove("QALCULATE_DISABLE_FALLBACK")
        .env_remove("QALCULATE_REPORT_FALLBACK");
    cmd.arg("-t");
    cmd
}

fn qalc_rs_raw() -> Command {
    let mut cmd = Command::cargo_bin("qalc-rs").expect("binary should build");
    cmd.env_remove("QALCULATE_DISABLE_FALLBACK")
        .env_remove("QALCULATE_REPORT_FALLBACK");
    cmd
}

#[test]
fn cli_help_and_version_aliases_match_upstream() {
    let help = format!("{}\n", include_str!("../src/cli/help.txt"));
    let version = format!("{UPSTREAM_LIBQALCULATE_VERSION}\n");
    let cases = [
        ("-h", help.clone()),
        ("-help", help.clone()),
        ("--help", help),
        ("-v", version.clone()),
        ("-V", version.clone()),
        ("-version", version.clone()),
        ("--version", version),
    ];

    for (alias, expected) in cases {
        let mut cmd = qalc_rs_raw();
        cmd.arg(alias).assert().success().stdout(expected);
    }
}

#[test]
fn cli_default_and_terse_output_match_upstream() {
    let cases = [("1 + 1", false, "1 + 1 = 2\n"), ("1 + 1", true, "2\n")];

    for (expr, is_terse, expected) in cases {
        let mut cmd = if is_terse { qalc_rs() } else { qalc_rs_raw() };
        cmd.arg(expr)
            .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
            .assert()
            .success()
            .stdout(expected);
    }

    let mut cpp_fallback = qalc_rs_raw();
    cpp_fallback
        .args(["--", "2^0.5"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .assert()
        .success()
        .stdout("2^0.5 ≈ 1.414213562\n");
}

#[test]
fn cli_nonterse_relation_matches_upstream_exactness() {
    let mut exact = qalc_rs_raw();
    exact
        .arg("1/2")
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .assert()
        .success()
        .stdout("1 / 2 = 0.5\n");

    for (expression, expected) in [
        ("1/3", "1 / 3 ≈ 0.3333333333\n"),
        ("sqrt(2)", "sqrt(2) ≈ 1.414213562\n"),
    ] {
        let mut cmd = qalc_rs_raw();
        cmd.args(["-set", "precision 10", "--", expression])
            .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
            .env("QALCULATE_DISABLE_FALLBACK", "1")
            .assert()
            .success()
            .stdout(expected);
    }

    for unicode_args in [
        vec!["+u8", "--", "1/3"],
        vec!["-set", "unicode 0", "--", "1/3"],
    ] {
        let mut ascii = qalc_rs_raw();
        ascii
            .args(unicode_args)
            .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
            .env("QALCULATE_DISABLE_FALLBACK", "1")
            .assert()
            .success()
            .stdout("1 / 3 = approx. 0.3333333333\n");
    }
}

#[test]
fn cli_unknown_option_before_separator_matches_upstream() {
    let cases = [
        (
            vec!["--definitely-unknown", "--", "1+1"],
            "Unrecognized option: --definitely-unknown.\n1 + 1 = 2\n",
        ),
        (
            vec!["-foo", "-bar", "--", "1+1"],
            "Unrecognized option: -foo.\nUnrecognized option: -bar.\n1 + 1 = 2\n",
        ),
    ];

    for (args, expected) in cases {
        let mut cmd = qalc_rs_raw();
        cmd.args(args)
            .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
            .assert()
            .success()
            .stdout(expected);
    }

    let mut version_after_diagnostic = qalc_rs_raw();
    version_after_diagnostic
        .args(["--definitely-unknown", "-v", "--", "1+1"])
        .assert()
        .success()
        .stdout(format!(
            "Unrecognized option: --definitely-unknown.\n{UPSTREAM_LIBQALCULATE_VERSION}\n"
        ));
}

#[test]
fn cli_evaluation_flag_matrix_matches_upstream() {
    let cases = [
        (vec!["-t", "-b", "16", "255"], "0xFF\n"),
        (vec!["-t", "-s", "base 16", "255"], "0xFF\n"),
        (
            vec!["-t", "-p", "16", "255"],
            "0000 0010 0101 0101 = 01125 = 597 = 0x255\n",
        ),
        (
            vec!["-t", "-p", "10", "52"],
            "0011 0100 = 064 = 52 = 0x34\n",
        ),
        (
            vec!["-t", "-p", "10", "--", "-128"],
            "1000 0000 = −0200 = −128 = −0x80\n",
        ),
        (vec!["-t", "-b", "2", "5"], "0000 0101\n"),
        (vec!["-t", "-b", "2", "--", "-5"], "1111 1011\n"),
        (vec!["-t", "-b", "2", "2+3"], "0000 0101\n"),
        (vec!["-t", "-p", "10", "2+3"], "0000 0101 = 05 = 5 = 0x5\n"),
        (vec!["-t", "-b", "bin", "--", "10"], "0000 1010\n"),
        (vec!["-t", "-b", "oct", "--", "10"], "012\n"),
        (vec!["-t", "-b", "dec", "--", "10"], "10\n"),
        (vec!["-t", "-b", "hex", "--", "10"], "0xA\n"),
        (
            vec!["-t", "-p", "bin", "--", "10"],
            "0000 0010 = 02 = 2 = 0x2\n",
        ),
        (
            vec!["-t", "-p", "--", "52"],
            "Illegal base.\n0000 0010 = 02 = 2 = 0x2\n",
        ),
        (
            vec!["-t", "-p", "10", "--", "2^3"],
            "0000 0001 = 01 = 1 = 0x1\n",
        ),
        (vec!["-t", "-s", "xor^ 1", "--", "2^3"], "1\n"),
        (
            vec![
                "-t",
                "-b",
                "16",
                "--",
                "340282366920938463463374607431768211456",
            ],
            "0x100000000000000000000000000000000\n",
        ),
        (
            vec!["-t", "-p", "16", "--", "A&F"],
            "0000 1010 = 012 = 10 = 0xA\n",
        ),
        (
            vec!["-t", "-p", "16", "--", "1<<8"],
            "0000 0001 0000 0000 = 0400 = 256 = 0x100\n",
        ),
        (vec!["-t", "-set", "output base 32", "--", "52"], "1K\n"),
        (
            vec![
                "-t",
                "-set",
                "input base 32",
                "-set",
                "output base 10",
                "--",
                "1K",
            ],
            "52\n",
        ),
        (vec!["-t", "-b", "10", "--", "1/3"], "0.3333333333\n"),
        (vec!["-t", "-b", "10 10", "--", "1/3"], "0.3333333333\n"),
        (vec!["-t", "+p", "--", "1/3"], "0.3333333333\n"),
        (
            vec!["-t", "-b", "10", "-set", "precision 10", "--", "sqrt(2)"],
            "1.414213562\n",
        ),
        (
            vec!["-t", "-p", "16", "--", "A+1"],
            "0000 1011 = 013 = 11 = 0xB\n",
        ),
        (vec!["-t", "-b", "16 16", "--", "A+1"], "0xB\n"),
        (vec!["-t", "-b", "8", "--", "-5"], "−05\n"),
        (vec!["-t", "+u8", "-b", "8", "--", "-5"], "-05\n"),
        (vec!["-t", "+p", "255"], "255\n"),
        (
            vec!["-t", "-p", "16", "+p", "255"],
            "0000 0000 1111 1111 = 0377 = 255 = 0xFF\n",
        ),
        (vec!["-t", "-u8", "52.34 to sexa"], "52°20′24″\n"),
        (vec!["-t", "+u8", "52.34 to sexa"], "52o20'24\"\n"),
        (vec!["-t", "-u8", "--", "1+1"], "2\n"),
        (vec!["-t", "+u8", "--", "-123"], "-123\n"),
        (
            vec!["-t", "-s", "unicode 1", "+u8", "--", "52.34 to sexa"],
            "52°20′24″\n",
        ),
        (
            vec!["-t", "+u8", "-s", "unicode 1", "--", "52.34 to sexa"],
            "52°20′24″\n",
        ),
        (vec!["-t", "--latex", "1/2"], "$0.5$\n"),
        (vec!["-t", "--html", "1/2"], "0.5\n"),
        (vec!["-t", "--", "-1"], "−1\n"),
    ];

    for (args, expected) in cases {
        let mut cmd = qalc_rs_raw();
        cmd.args(args)
            .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
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
fn cli_evaluation_settings_work_when_cpp_fallback_is_available() {
    let cases = [
        (vec!["-t", "-b", "16", "255"], "0xFF\n"),
        (vec!["-t", "-b", "hex", "255"], "0xFF\n"),
        (vec!["-t", "-s", "base 16", "255"], "0xFF\n"),
        (
            vec!["-t", "-p", "10", "52"],
            "0011 0100 = 064 = 52 = 0x34\n",
        ),
        (vec!["-t", "-p", "bin", "10"], "0000 0010 = 02 = 2 = 0x2\n"),
        (vec!["-t", "+u8", "-b", "8", "--", "-5"], "-05\n"),
    ];

    for (args, expected) in cases {
        let mut cmd = qalc_rs_raw();
        cmd.args(args)
            .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
            .env_remove("QALCULATE_DISABLE_FALLBACK")
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
fn cli_explicit_unicode_on_keeps_cpp_fallback_available() {
    let mut cmd = qalc_rs_raw();
    cmd.args(["-t", "-u8", "--", "cross([1,0,0];[0,1 m^2,0])"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .assert()
        .success()
        .stdout("[(0 m²)  0  (1 m²)]\n")
        .stderr(predicate::str::contains(
            "[qalc-rs-metadata] fallback=cpp-fallback-enabled",
        ));
}

#[test]
fn cli_explicit_ascii_keeps_cpp_fallback_available() {
    let mut cmd = qalc_rs_raw();
    cmd.args(["-t", "+u8", "--", "cross([1,0,0];[0,1 m^2,0])"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .assert()
        .success()
        .stdout("[(0 m^2)  0  (1 m^2)]\n")
        .stderr(predicate::str::contains(
            "[qalc-rs-metadata] fallback=cpp-fallback-enabled",
        ));
}

#[test]
fn cli_markup_modes_use_cpp_fallback_for_unsupported_native_expressions() {
    let expression = "cross([1,0,0];[0,1,0])";
    let cases = [
        (
            vec!["-t", "--latex", "--", expression],
            "$\\displaystyle \\begin{bmatrix}0 & 0 & 1\\end{bmatrix}$\n",
        ),
        (
            vec!["--latex", "--", expression],
            "$\\displaystyle \\operatorname{cross}(\\begin{bmatrix}1 & 0 & 0\\end{bmatrix}, \\begin{bmatrix}0 & 1 & 0\\end{bmatrix}) = \\begin{bmatrix}0 & 0 & 1\\end{bmatrix}$\n",
        ),
        (
            vec!["-t", "--html", "--", expression],
            "[0&nbsp; 0&nbsp; 1]\n",
        ),
        (
            vec!["--html", "--", expression],
            "cross([1&nbsp; 0&nbsp; 0], [0&nbsp; 1&nbsp; 0]) = [0&nbsp; 0&nbsp; 1]\n",
        ),
        (
            vec!["--latex", "--", "Ei(3)"],
            "$\\displaystyle \\operatorname{Ei}(3) \\approx \\num{9.933832571}$\n",
        ),
        (vec!["--html", "--", "Ei(3)"], "Ei(3) ≈ 9.933832571\n"),
        (
            vec!["-t", "--latex", "--", "1 m + 1 cm"],
            "$\\displaystyle \\qty{1.01}{m}$\n",
        ),
        (
            vec!["--latex", "--", "1 m + 1 cm"],
            "$\\displaystyle \\qty{1}{m} + \\qty{1}{cm} = \\qty{1.01}{m}$\n",
        ),
        (vec!["-t", "--html", "--", "1 m + 1 cm"], "1.01 m\n"),
        (
            vec!["--html", "--", "1 m + 1 cm"],
            "1 m + 1 cm = 1.01 m\n",
        ),
        (
            vec!["--latex", "--", "Ei(3) meter"],
            "$\\displaystyle \\qty[parse-numbers=false]{\\operatorname{Ei}(3)}{m} \\approx \\qty{9.933832571}{m}$\n",
        ),
        (
            vec!["--html", "--", "Ei(3) meter"],
            "Ei(3) × m ≈ 9.933832571 m\n",
        ),
        (
            vec!["+u8", "--html", "--", "Ei(3)"],
            "Ei(3) = approx. 9.933832571\n",
        ),
        (
            vec!["-t", "--html", "--", "Ei(3)*x"],
            "9.933832571<i>x</i>\n",
        ),
        (
            vec!["-t", "--html", "--", "Ei(3)*'foo'"],
            "9.933832571 <i>\"foo\"</i>\n",
        ),
        (
            vec!["--html", "--", "Ei(3)*x"],
            "Ei(3) × <i>x</i> ≈ 9.933832571<i>x</i>\n",
        ),
        (
            vec!["--latex", "--", "sum(1/x; 1; 3; x)"],
            "$\\displaystyle \\sum_{x=1}^{3}\\left(\\frac{1}{x}\\right) = \\frac{11}{6} = 1 + \\frac{5}{6} \\approx \\num{1.833333333}$\n",
        ),
        (
            vec!["-t", "--latex", "--", "sum(1/x; 1; 3; x)"],
            "$\\displaystyle \\num{1.833333333}$\n",
        ),
        (
            vec!["+u8", "--latex", "--", "sum(1/x; 1; 3; x)"],
            "$\\displaystyle \\sum_{x=1}^{3}\\left(\\frac{1}{x}\\right) = \\frac{11}{6} = 1 + \\frac{5}{6} \\approx \\num{1.833333333}$\n",
        ),
        (
            vec!["--latex", "--", "Ei(3;4)"],
            "$\\displaystyle \\operatorname{Ei}(\\begin{bmatrix}3 & 4\\end{bmatrix}) = \\begin{bmatrix}\\operatorname{Ei}(3) & \\operatorname{Ei}(4)\\end{bmatrix} \\approx \\begin{bmatrix}\\num[parse-numbers=true]{9.933832571} & \\num[parse-numbers=true]{19.63087447}\\end{bmatrix}$\n",
        ),
        (
            vec![
                "--latex",
                "--",
                "cross([1,0,0];[0,1,0])=[0,0,1]",
            ],
            "$\\displaystyle \\left(\\operatorname{cross}(\\begin{bmatrix}1 & 0 & 0\\end{bmatrix}, \\begin{bmatrix}0 & 1 & 0\\end{bmatrix}) = \\begin{bmatrix}0 & 0 & 1\\end{bmatrix}\\right) = true$\n",
        ),
        (
            vec!["--latex", "--", "Ei(x) where x=3"],
            "$\\displaystyle \\operatorname{Ei}(x) = \\operatorname{Ei}(3) \\approx \\num{9.933832571}$\n",
        ),
        (
            vec!["--html", "--", "Ei(x) where x=3"],
            "Ei(x) = Ei(3) ≈ 9.933832571\n",
        ),
        (
            vec!["--latex", "--", "Ei(3) meter to cm"],
            "$\\displaystyle \\qty[parse-numbers=false]{\\operatorname{Ei}(3)}{m} \\approx \\qty{993.3832571}{cm}$\n",
        ),
        (vec!["--latex", "--", "# comment"], "$0 = 0$\n"),
        (vec!["-t", "--html", "--", "# comment"], "0\n"),
        (
            vec!["--latex", "--", "Ei(3) to fraction"],
            "$\\displaystyle \\operatorname{Ei}(3) \\approx \\num{9.933832571}$\n",
        ),
        (
            vec!["--html", "--", "Ei(30) to sci"],
            "Ei(30) ≈ 3.689732094<small>E</small>11\n",
        ),
        (
            vec!["-t", "--html", "--", "cross([1,0,0];[0,1,0]) to hex"],
            "[0x0&nbsp; 0x0&nbsp; 0x1]\n",
        ),
        (
            vec!["--latex", "--", "Ei(3) to bases"],
            "$\\displaystyle \\operatorname{Ei}(3) \\approx \\text{1001.11101111000011111010011011000} = \\text{011.736076466} = \\num{9.933832571} = \\text{0x9.EF0FA6C}$\n",
        ),
        (
            vec!["--html", "--", "factorial(20) to factors"],
            "factorial(20) = 2432902008176640000 = 2<sup>18</sup> × 3<sup>8</sup> × 5<sup>4</sup> × 7<sup>2</sup> × 11 × 13 × 17 × 19\n",
        ),
        (
            vec!["--html", "--", "1/(x^2-1) to partial fraction"],
            "1 / (<i>x</i><sup>2</sup> − 1) = 1 / (2<i>x</i> − 2) − 1 / (2<i>x</i> + 2)\n",
        ),
        (vec!["--html", "--", "10 to base 3"], "10 = 101\n"),
    ];

    for (args, expected) in cases {
        let mut cmd = qalc_rs_raw();
        cmd.args(args)
            .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
            .env("QALCULATE_REPORT_FALLBACK", "1")
            .assert()
            .success()
            .stdout(expected)
            .stderr(predicate::str::contains(
                "[qalc-rs-metadata] fallback=cpp-fallback-enabled",
            ));
    }
}

#[test]
fn cli_markup_fallback_preserves_qalc_terse_message_suppression() {
    let errors = concat!(
        "error: Argument 1, Vector 1, in cross() must be a vector that fulfills the condition: ",
        "\"dimension(Vector 1)==3\".\n",
        "error: Argument 2, Vector 2, in cross() must be a vector that fulfills the condition: ",
        "\"dimension(Vector 2)==3\".\n",
    );

    let mut terse = qalc_rs_raw();
    terse
        .args(["-t", "--html", "--", "cross(1;2)"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .assert()
        .failure()
        .code(1)
        .stdout("cross(1, 2)\n")
        .stderr(predicate::str::contains(
            "[qalc-rs-metadata] fallback=cpp-fallback-enabled",
        ));

    let mut equation = qalc_rs_raw();
    equation
        .args(["--html", "--", "cross(1;2)"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .assert()
        .failure()
        .code(1)
        .stdout(format!("{errors}cross(1, 2) = cross(1, 2)\n"))
        .stderr(predicate::str::contains(
            "[qalc-rs-metadata] fallback=cpp-fallback-enabled",
        ));
}

#[test]
fn cli_markup_special_calendar_conversion_uses_complete_cpp_output() {
    let mut command = qalc_rs_raw();
    command
        .args(["-t", "--html", "--", "today to calendars"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .assert()
        .success()
        .stdout(predicate::str::starts_with(
            "Calendar\t\t\t\tDay, Month, Year\nGregorian:",
        ))
        .stderr(predicate::str::contains(
            "[qalc-rs-metadata] fallback=cpp-fallback-enabled",
        ));
}

#[test]
fn cli_unsupported_markup_expression_fails_closed_without_cpp_fallback() {
    let mut cmd = qalc_rs_raw();
    cmd.args(["-t", "--latex", "--", "cross([1,0,0];[0,1,0])"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .assert()
        .failure()
        .code(2)
        .stdout("")
        .stderr(predicate::str::contains(
            "[qalc-rs-metadata] fallback=disabled",
        ));
}

#[test]
fn cli_programming_flag_wins_over_programming_mode_set_replay() {
    for args in [
        ["-t", "-s", "programming mode 0", "-p", "16", "255"],
        ["-t", "-p", "16", "-s", "programming mode 0", "255"],
    ] {
        let mut cmd = qalc_rs_raw();
        cmd.args(args)
            .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
            .assert()
            .success()
            .stdout("Unrecognized option.\n\n0000 0010 0101 0101 = 01125 = 597 = 0x255\n");
    }
}

#[test]
fn cli_treats_try_exact_as_the_default_approximation_mode() {
    for fallback_disabled in [false, true] {
        let mut cmd = qalc_rs_raw();
        cmd.args(["-t", "-set", "approximation try exact", "--", "1+1"])
            .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
            .env("QALCULATE_REPORT_FALLBACK", "1");
        if fallback_disabled {
            cmd.env("QALCULATE_DISABLE_FALLBACK", "1");
        }
        cmd.assert()
            .success()
            .stdout("2\n")
            .stderr(predicate::str::contains(
                "[qalc-rs-metadata] fallback=native",
            ));
    }
}

#[test]
fn cli_fails_closed_when_requested_output_base_cannot_be_formatted_natively() {
    let mut cmd = qalc_rs_raw();
    cmd.args(["-t", "-b", "16", "--", "1/3"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .assert()
        .failure()
        .stdout("")
        .stderr(
            predicate::str::contains("[qalc-rs-metadata] fallback=disabled").and(
                predicate::str::contains(
                    "C++ FFI fallback is disabled, and expression '1/3' has no native Rust implementation",
                ),
            ),
        );
}

#[test]
fn cli_fails_closed_instead_of_ignoring_settings_on_cpp_fallback() {
    let cases = [
        (
            vec!["-t", "-set", "angle unit deg", "--", "sin(90)"],
            "angle unit deg",
        ),
        (
            vec!["-t", "-set", "approximation exact", "--", "sqrt(2)"],
            "approximation exact",
        ),
    ];

    for (args, setting) in cases {
        let mut cmd = qalc_rs_raw();
        cmd.args(args)
            .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
            .env_remove("QALCULATE_DISABLE_FALLBACK")
            .assert()
            .failure()
            .code(2)
            .stdout("")
            .stderr(predicate::str::contains(format!(
                "session settings are not supported by the C++ FFI fallback path: {setting}"
            )));
    }
}

#[test]
fn cli_cpp_fallback_applies_supported_output_base_settings() {
    let mut cmd = qalc_rs_raw();
    cmd.args(["-t", "-b", "16", "--", "1/3"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .env_remove("QALCULATE_DISABLE_FALLBACK")
        .assert()
        .success()
        .stdout("0x0.55555555\n")
        .stderr(predicate::str::contains(
            "[qalc-rs-metadata] fallback=cpp-fallback-enabled",
        ));
}

#[test]
fn cli_applies_fraction_format_before_returning_native_output() {
    for fallback_disabled in [false, true] {
        let mut cmd = qalc_rs_raw();
        cmd.args(["-t", "-set", "fr 2", "--", "1/3"])
            .env("QALCULATE_DEFINITIONS_DIR", definitions_dir());
        if fallback_disabled {
            cmd.env("QALCULATE_DISABLE_FALLBACK", "1");
        } else {
            cmd.env_remove("QALCULATE_DISABLE_FALLBACK");
        }
        cmd.assert().success().stdout("1/3\n");

        let mut equation = qalc_rs_raw();
        equation
            .args(["-set", "fr 2", "--", "1/3"])
            .env("QALCULATE_DEFINITIONS_DIR", definitions_dir());
        if fallback_disabled {
            equation.env("QALCULATE_DISABLE_FALLBACK", "1");
        } else {
            equation.env_remove("QALCULATE_DISABLE_FALLBACK");
        }
        equation.assert().success().stdout("1 / 3 = 1/3\n");
    }
}

#[test]
fn cli_rejects_unevaluated_terse_markup_results() {
    for output_flag in ["--latex", "--html"] {
        let mut cmd = qalc_rs_raw();
        cmd.args(["-t", output_flag, "--", "sqrt(1;2)"])
            .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
            .env("QALCULATE_DISABLE_FALLBACK", "1")
            .assert()
            .failure()
            .code(2)
            .stdout("")
            .stderr(predicate::str::contains("Expected 1 argument(s), got 2"));
    }
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

#[test]
fn cli_definition_and_listing_flags_are_effective() {
    let mut cmd = qalc_rs_raw();
    cmd.args(["--list-functions", "sinc"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .assert()
        .success()
        .stdout("sinc (Cardinal Sine (Sinc Function))\t\n\nFor more information about a specific function, variable, unit, or prefix, please use the info command (in interactive mode).\n\n");

    let mut cmd2 = qalc_rs_raw();
    cmd2.args(["--list-units", "mWC"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .assert()
        .success()
        .stdout("mWC / mwg / mH\u{2082}O (Meter of Water)\t\n\nFor more information about a specific function, variable, unit, or prefix, please use the info command (in interactive mode).\n\n");

    let mut cmd3 = qalc_rs_raw();
    cmd3.args(["--list-variables", "Archimedes"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .assert()
        .success()
        .stdout("pi / \u{03c0} (Archimedes' Constant (pi))\t\n\nFor more information about a specific function, variable, unit, or prefix, please use the info command (in interactive mode).\n\n");

    let mut cmd4 = qalc_rs_raw();
    cmd4.args(["--list-prefixes", "mega"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .assert()
        .success()
        .stdout("\t\tmega / M\t\n\nFor more information about a specific function, variable, unit, or prefix, please use the info command (in interactive mode).\n\n");

    let mut cmd5 = qalc_rs_raw();
    cmd5.args(["-nodatasets", "--list-functions", "atom"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .assert()
        .success()
        .stdout("No matching item found.\n\n");

    let mut unicode_setting_off = qalc_rs_raw();
    unicode_setting_off
        .args(["-s", "unicode 0", "-u8", "--list-variables", "Archimedes"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .assert()
        .success()
        .stdout("pi (Archimedes' Constant (pi))\t\n\nFor more information about a specific function, variable, unit, or prefix, please use the info command (in interactive mode).\n\n");

    let mut unicode_setting_on = qalc_rs_raw();
    unicode_setting_on
        .args(["-s", "unicode 1", "+u8", "--list-variables", "Archimedes"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .assert()
        .success()
        .stdout("pi / \u{03c0} (Archimedes' Constant (pi))\t\n\nFor more information about a specific function, variable, unit, or prefix, please use the info command (in interactive mode).\n\n");

    let mut slash_unicode_setting_off = qalc_rs_raw();
    slash_unicode_setting_off
        .args(["-s", "/set unicode 0", "--list-variables", "Archimedes"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .assert()
        .success()
        .stdout("pi (Archimedes' Constant (pi))\t\n\nFor more information about a specific function, variable, unit, or prefix, please use the info command (in interactive mode).\n\n");

    let mut interactive_list = qalc_rs_raw();
    interactive_list
        .args(["-i", "--list-functions", "sinc"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .assert()
        .success()
        .stdout("sinc (Cardinal Sine (Sinc Function))\t\n\nFor more information about a specific function, variable, unit, or prefix, please use the info command (in interactive mode).\n\n");

    for args in [
        vec![
            "-f",
            "/tmp/does-not-exist-pr200",
            "--list-functions",
            "sinc",
        ],
        vec![
            "--list-functions",
            "sinc",
            "--test-file",
            "/tmp/does-not-exist-pr200",
        ],
    ] {
        let mut list_before_file_workflow = qalc_rs_raw();
        list_before_file_workflow
            .args(args)
            .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
            .assert()
            .success()
            .stdout("sinc (Cardinal Sine (Sinc Function))\t\n\nFor more information about a specific function, variable, unit, or prefix, please use the info command (in interactive mode).\n\n");
    }
}

#[test]
fn cli_usd_currency_searches() {
    let mut cmd = qalc_rs_raw();
    cmd.args(["--list-units", "USD"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .assert()
        .success()
        .stdout(predicate::str::contains("cent / ¢ (Cent (USD))"))
        .stdout(predicate::str::contains("dollar"))
        .stdout(predicate::str::contains("USD"));

    let temp = tempdir().expect("temp dir should be created");
    std::fs::copy(
        Path::new(definitions_dir()).join("currencies.xml.in"),
        temp.path().join("currencies.xml.in"),
    )
    .expect("currency fixture should be copied");
    let mut cmd2 = qalc_rs_raw();
    cmd2.args(["-nounits", "--list-units", "USD"])
        .env("QALCULATE_DEFINITIONS_DIR", temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("dollar"))
        .stdout(predicate::str::contains("USD"));

    let mut cmd3 = qalc_rs_raw();
    cmd3.args(["-nocurrencies", "--list-units", "USD"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .assert()
        .success()
        .stdout("No matching item found.\n\n");

    let mut country = qalc_rs_raw();
    country
        .args(["-l", "United"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .assert()
        .success()
        .stdout(predicate::str::contains("United Arab Emirates Dirham"))
        .stdout(predicate::str::contains("British Pound"))
        .stdout(predicate::str::contains("U.S. Dollar"));

    let mut obsolete = qalc_rs_raw();
    obsolete
        .args(["--list-units", "BEF"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .assert()
        .success()
        .stdout("BEF (Belgian Franc (obsolete))\t\n\nFor more information about a specific function, variable, unit, or prefix, please use the info command (in interactive mode).\n\n");
}

#[test]
fn cli_unfiltered_prefix_list_uses_loaded_catalog() {
    let mut cmd = qalc_rs_raw();
    cmd.arg("--list-prefixes")
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .assert()
        .success()
        .stdout(predicate::str::contains("kilo / k"))
        .stdout(predicate::str::contains(
            "For more information about a specific function",
        ));
}

#[test]
fn cli_unfiltered_all_list_does_not_load_global_catalogs() {
    let temp = tempdir().expect("temp dir should be created");
    let mut cmd = qalc_rs_raw();
    cmd.arg("--list")
        .env("QALCULATE_DEFINITIONS_DIR", temp.path())
        .assert()
        .success()
        .stdout("\nNo local variables, functions or units have been defined.\n\nFor more information about a specific function, variable, unit, or prefix, please use the info command (in interactive mode).\n\n");
}

#[test]
fn cli_list_missing_file_fails() {
    let temp = tempdir().expect("temp dir should be created");
    let mut cmd = qalc_rs_raw();
    cmd.args(["--list-prefixes", "mega"])
        .env("QALCULATE_DEFINITIONS_DIR", temp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("failed to load"));
}

#[test]
fn cli_list_disable_selection_does_not_load_file() {
    let temp = tempdir().expect("temp dir should be created");
    let mut cmd = qalc_rs_raw();
    cmd.args(["-nounits", "--list-prefixes", "mega"])
        .env("QALCULATE_DEFINITIONS_DIR", temp.path())
        .assert()
        .success()
        .stdout("No matching item found.\n\n");
}

#[test]
fn cli_defaults_ignores_persistent_config_files() {
    let temp_home = tempdir().expect("temporary home should be created");
    let temp_xdg = tempdir().expect("temporary config root should be created");
    let qalc_config_dir = temp_xdg.path().join("qalculate");
    std::fs::create_dir_all(&qalc_config_dir).expect("config directory should be created");
    std::fs::write(qalc_config_dir.join("qalc.cfg"), "base 16\n")
        .expect("poison config should be written");
    let mut cmd = qalc_rs_raw();
    cmd.args(["--defaults", "1+1"])
        .env("HOME", temp_home.path())
        .env("XDG_CONFIG_HOME", temp_xdg.path())
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .assert()
        .success()
        .stdout("1 + 1 = 2\n");
}

#[test]
fn cli_rates_success_and_failure() {
    let mut cmd = qalc_rs_raw();
    cmd.args(["-e"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .assert()
        .success()
        .stdout("")
        .stderr("");

    let temp = tempdir().expect("temp dir should be created");
    let mut cmd2 = qalc_rs_raw();
    cmd2.args(["-e"])
        .env("QALCULATE_DEFINITIONS_DIR", temp.path())
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("error: failed to load rates JSON"));
}

#[test]
fn cli_color_on_off_cases() {
    let mut cmd_off = qalc_rs_raw();
    cmd_off
        .args(["-c0", "1+1"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .assert()
        .success()
        .stdout("1 + 1 = 2\n");

    let mut cmd_on = qalc_rs_raw();
    cmd_on
        .args(["-c1", "1+1"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains(
            "forced coloring is not implemented (owner issue: #198)",
        ));
}

#[test]
fn cli_nodefs_flag_evaluation() {
    let mut cmd = qalc_rs();
    cmd.args(["-nodefs", r#"message("hello")"#])
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .assert()
        .code(1)
        .stdout("hello\n0\n");

    for args in [
        vec!["-nodefs", "-t", "--", "1+1"],
        vec!["-nodefs", "-t", "--html", "--", "1+1"],
    ] {
        let mut terse = qalc_rs_raw();
        terse
            .args(args)
            .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
            .assert()
            .code(1)
            .stdout("2\n");
    }

    let mut markup_equation = qalc_rs_raw();
    markup_equation
        .args(["-nodefs", "--html", "--", "1+1"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .assert()
        .code(1)
        .stdout("error: Radians unit is missing. Creating one for this session.\n1 + 1 = 2\n");

    let mut definition_free = qalc_rs_raw();
    definition_free
        .args(["-nodefs", "1+1"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .assert()
        .failure()
        .code(1)
        .stdout("error: Radians unit is missing. Creating one for this session.\n1 + 1 = 2\n");

    let mut definition_backed = qalc_rs_raw();
    definition_backed
        .args(["-nodefs", "-t", "--", "1 USD to EUR"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .assert()
        .failure()
        .code(2)
        .stdout("")
        .stderr(predicate::str::contains(
            "global definitions are disabled for this native expression",
        ));
}

#[test]
fn cli_cpp_fallback_initializes_exchange_rates_before_currency_definitions() {
    for (expression, expected) in [
        ("1 USD", "€0.8585164835\n"),
        ("1 EUR to USD", "$1.164800000\n"),
    ] {
        let home = tempdir().expect("isolated qalc home should be created");
        let mut cmd = qalc_rs_raw();
        cmd.args(["-t", "--", expression])
            .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
            .env("QALCULATE_REPORT_FALLBACK", "1")
            .env("HOME", home.path())
            .env("XDG_CONFIG_HOME", home.path())
            .env("LC_ALL", "C")
            .env("LANG", "C")
            .assert()
            .success()
            .stdout(expected)
            .stderr(predicate::str::contains(
                "[qalc-rs-metadata] fallback=cpp-fallback-enabled",
            ));
    }
}

#[test]
fn cli_selective_definition_fallback_rejection() {
    for (flag, expression) in [
        ("-nounits", "1 m to cm"),
        ("-nocurrencies", "1 USD to EUR"),
        ("-nofunctions", "cross([1, 0, 0]; [0, 1, 0])"),
        ("-novariables", "c"),
        ("-nodatasets", "atom(H; mass)"),
    ] {
        let mut disabled_family = qalc_rs_raw();
        disabled_family
            .args([flag, "-t", "--", expression])
            .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
            .env("QALCULATE_DISABLE_FALLBACK", "1")
            .assert()
            .failure()
            .code(2)
            .stderr(predicate::str::contains(
                "selective definitions are unsupported for native evaluation",
            ));
    }

    for (flag, expression, expected) in [
        ("-nounits", r#"message("hello")"#, "hello\n0\n"),
        ("-nocurrencies", "1 m to cm", "100 cm\n"),
        ("-nofunctions", "1 m to cm", "100 cm\n"),
        ("-novariables", "sqrt(4)", "2\n"),
        ("-nodatasets", "sqrt(4)", "2\n"),
    ] {
        let mut unrelated_family = qalc_rs_raw();
        unrelated_family
            .args([flag, "-t", "--", expression])
            .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
            .env("QALCULATE_DISABLE_FALLBACK", "1")
            .env("QALCULATE_REPORT_FALLBACK", "1")
            .assert()
            .success()
            .stdout(expected)
            .stderr(predicate::str::contains(
                "[qalc-rs-metadata] fallback=native",
            ));
    }

    for flag in [
        "-nounits",
        "-nocurrencies",
        "-nofunctions",
        "-novariables",
        "-nodatasets",
    ] {
        let mut default_path = qalc_rs_raw();
        default_path
            .args([flag, "1+1"])
            .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
            .assert()
            .success()
            .stdout("1 + 1 = 2\n");

        let mut definition_free = qalc_rs_raw();
        definition_free
            .args(["-t", flag, "--", "1+1"])
            .env("QALCULATE_DISABLE_FALLBACK", "1")
            .env("QALCULATE_REPORT_FALLBACK", "1")
            .assert()
            .success()
            .stdout("2\n")
            .stderr(predicate::str::contains(
                "[qalc-rs-metadata] fallback=native",
            ));
    }
}

#[test]
fn cli_selective_loading_skips_disabled_xml_catalogs() {
    let definitions = tempdir().expect("temporary definitions directory");
    for file in [
        "prefixes.xml",
        "currencies.xml",
        "units.xml",
        "datasets.xml",
        "variables.xml",
    ] {
        std::fs::copy(
            Path::new(definitions_dir()).join(file),
            definitions.path().join(file),
        )
        .unwrap_or_else(|error| panic!("failed to copy {file}: {error}"));
    }

    let mut cmd = qalc_rs_raw();
    cmd.args(["-nofunctions", "-t", "--", "1+1"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions.path())
        .assert()
        .success()
        .stdout("2\n");
}

#[test]
fn cli_disabled_unit_gate_uses_full_catalog_aliases() {
    for fallback_disabled in [false, true] {
        let mut cmd = qalc_rs_raw();
        cmd.args(["-nounits", "-t", "--", "1 acre to hectare"])
            .env("QALCULATE_DEFINITIONS_DIR", definitions_dir());
        if fallback_disabled {
            cmd.env("QALCULATE_DISABLE_FALLBACK", "1");
        }
        cmd.assert()
            .failure()
            .code(2)
            .stderr(predicate::str::contains(if fallback_disabled {
                "selective definitions are unsupported for native evaluation"
            } else {
                "selective definitions are incompatible with fallback"
            }));
    }
}

#[test]
fn cli_test_file_without_path_reports_upstream_diagnostic_before_handoff() {
    let mut cmd = qalc_rs_raw();
    cmd.arg("--test-file")
        .assert()
        .success()
        .stdout(
            "No file specified.\n> \x1B[31m\nWARNING: 0 tests were run (indentation needs to be tab-based)\n\n\x1B[0m",
        )
        .stderr("");
}

#[test]
fn cli_runs_terse_stdin_command_stream_without_prompts() {
    let state = tempdir().expect("temporary state directory");
    let mut cmd = qalc_rs_raw();
    cmd.args(["-c0", "-t", "-f", "-"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .env("XDG_STATE_HOME", state.path())
        .write_stdin("# setup\n\n1+1\n1+1\n")
        .assert()
        .success()
        .stdout("2\n2\n")
        .stderr("");
    assert!(!state.path().join("qalculate/qalc.history").exists());
}

#[test]
fn cli_stdin_command_stream_matches_upstream_without_global_definitions() {
    let upstream = Path::new("../libqalculate/src/qalc");
    if !upstream.exists() {
        return;
    }
    let input = "1+1\nans+1\nset base 16\n15\nset base 10\n15\n";

    let mut rust = qalc_rs_raw();
    let rust_output = rust
        .args(["-c0", "-nodefs", "-t", "-f", "-"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .write_stdin(input)
        .assert()
        .success()
        .get_output()
        .clone();

    let upstream_home = tempdir().expect("upstream home");
    let mut oracle = Command::new(upstream);
    let oracle_output = oracle
        .args(["-c0", "-nodefs", "-t", "-f", "-"])
        .env("HOME", upstream_home.path())
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("TZ", "UTC")
        .write_stdin(input)
        .assert()
        .success()
        .get_output()
        .clone();

    assert_eq!(rust_output.stdout, oracle_output.stdout);
    assert_eq!(rust_output.stderr, oracle_output.stderr);
}

#[test]
fn cli_non_terse_stdin_command_stream_prints_equations_without_repl_spacing() {
    let mut cmd = qalc_rs_raw();
    cmd.args(["-c0", "-f", "-"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .write_stdin("1+1\n1+1\n")
        .assert()
        .success()
        .stdout("1 + 1 = 2\n1 + 1 = 2\n")
        .stderr("");
}

#[test]
fn cli_stdin_command_stream_reports_invalid_commands_and_continues() {
    let mut cmd = qalc_rs_raw();
    cmd.args(["-c0", "-t", "-f", "-"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .write_stdin("set base nope\n1+1\n")
        .assert()
        .success()
        .stdout("Illegal base.\n2\n")
        .stderr("");
}

#[test]
fn cli_command_stream_accepts_bare_base_command() {
    let mut cmd = qalc_rs_raw();
    cmd.args(["-c0", "-nodefs", "-t", "-f", "-"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .write_stdin("base 16\n15\n")
        .assert()
        .success()
        .stdout("0xF\n")
        .stderr("");
}

#[test]
fn cli_command_stream_keeps_command_variables_cpp_owned() {
    let mut cmd = qalc_rs_raw();
    cmd.args(["-c0", "-nodefs", "-t", "-f", "-"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .write_stdin("set base 10\n1+1\nvariable x 1/3\nx+1\n")
        .assert()
        .success()
        .stdout("2\n1.333333333\n")
        .stderr(
            "[qalc-rs-metadata] fallback=native\n\
             [qalc-rs-metadata] fallback=cpp-fallback-enabled\n",
        );
}

#[test]
fn cli_command_stream_cpp_local_shadows_disabled_global_definition() {
    let mut cmd = qalc_rs_raw();
    cmd.args(["-c0", "-t", "-nounits", "-f", "-"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .write_stdin("variable m 1±0.1\nm\n")
        .assert()
        .success()
        .stdout("1.00±0.10\n")
        .stderr("");
}

#[test]
fn cli_command_stream_finds_currencies_explicitly() {
    let mut cmd = qalc_rs_raw();
    cmd.args(["-c0", "-t", "-f", "-"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .write_stdin("find currencies\n")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("cent / ¢ (Cent (USD))")
                .and(predicate::str::contains("dollar"))
                .and(predicate::str::contains("USD"))
                .and(predicate::str::contains("metre / meter").not())
                .and(predicate::str::contains("No matching item found.").not()),
        )
        .stderr("");
}

#[test]
fn cli_quoted_command_definition_matches_upstream_semantics() {
    let upstream = Path::new("../libqalculate/src/qalc");
    if !upstream.exists() {
        return;
    }
    let input = "variable label \"green apples\"\nlabel\n";

    let mut rust = qalc_rs_raw();
    let rust_output = rust
        .args(["-c0", "-t", "-f", "-"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .write_stdin(input)
        .assert()
        .success()
        .get_output()
        .clone();

    let upstream_home = tempdir().expect("upstream home");
    let mut oracle = Command::new(upstream);
    let oracle_output = oracle
        .args(["-c0", "-t", "-set", "save definitions off", "-f", "-"])
        .env("HOME", upstream_home.path())
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("TZ", "UTC")
        .write_stdin(input)
        .assert()
        .success()
        .get_output()
        .clone();

    assert_eq!(rust_output.stdout, oracle_output.stdout);
    assert_eq!(rust_output.stderr, oracle_output.stderr);
}

#[test]
fn cli_command_stream_definitions_are_silent_stateful_and_preserve_ans() {
    let mut cmd = qalc_rs_raw();
    cmd.args(["-c0", "-nodefs", "-t", "-f", "-"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .write_stdin("1+1\nvariable ans 5\nans\nvariable rate 5\nans\nrate+1\nfunction twice 2*\\x\nans\ntwice(3)\nvariable zero\nzero\nfunction empty\nempty(3)\nset input base 16\nvariable based 15\nbased\nvariable malformed 5\nvariable malformed \"\nmalformed\nvariable quoted \"2024-01-01\"\nquoted\n")
        .assert()
        .success()
        .stdout("2\n5\n5\n6\n5\n6\n0\n0\n15\n0\n2022\n")
        .stderr("");
}

#[test]
fn cli_command_stream_deletes_a_command_defined_ans_override() {
    let mut cmd = qalc_rs_raw();
    cmd.args(["-c0", "-nodefs", "-t", "-f", "-"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .write_stdin("variable ans 5\ndelete ans\nans\n")
        .assert()
        .success()
        .stdout("undefined\n")
        .stderr("");
}

#[test]
fn cli_command_stream_lists_inspects_and_deletes_user_functions() {
    let mut cmd = qalc_rs_raw();
    cmd.args(["-c0", "-nodefs", "-t", "-f", "-"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .write_stdin(
            "function Twice 2*\\x\nlist\nfind functions twice\ninfo twice\nTwice(3)\ndelete twice()\ninfo Twice\n",
        )
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Twice\t")
                .count(2)
                .and(predicate::str::contains("\nFunction\n\nTwice(argument)\n"))
                .and(predicate::str::contains("Expression: 2*\\x"))
                .and(predicate::str::contains("\n6\n"))
                .and(predicate::str::ends_with("No matching item found.\n\n")),
        )
        .stderr("");
}

#[test]
fn cli_command_stream_keeps_settings_after_reformat_failure() {
    let mut cmd = qalc_rs_raw();
    cmd.args(["-c0", "-t", "-f", "-"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .write_stdin("1/3\nset precision 20\n1/3\n")
        .assert()
        .success()
        .stdout(
            predicate::str::starts_with("0.3333333333\n")
                .and(predicate::str::ends_with("0.33333333333333333333\n")),
        )
        .stderr(predicate::str::contains(
            "session settings are not supported by the C++ FFI fallback path: precision 20",
        ));
}

#[test]
fn cli_command_stream_only_treats_hash_as_an_expression_comment() {
    let mut cmd = qalc_rs_raw();
    cmd.args(["-c0", "-nodefs", "-t", "-f", "-"])
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .write_stdin("set base 16 # invalid command suffix\n1+1 # expression comment\nquit # not a control command\n2+2\n")
        .assert()
        .success()
        .stdout("Illegal base.\n2\n0\n4\n")
        .stderr("");
}

#[test]
fn cli_command_file_runs_before_the_trailing_expression() {
    let command_dir = tempdir().expect("temporary command-file directory");
    let command_path = command_dir.path().join("commands.qalc");
    std::fs::write(&command_path, "set base 16\n15\n").expect("command file");

    for flag in ["-f", "-file", "--file"] {
        let mut cmd = qalc_rs_raw();
        cmd.args(["-c0", "-t", flag])
            .arg(&command_path)
            .arg("10")
            .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
            .env("QALCULATE_DISABLE_FALLBACK", "1")
            .assert()
            .success()
            .stdout("0xF\n0xA\n")
            .stderr("");
    }
}

#[test]
fn cli_command_stream_quit_skips_remaining_input_and_trailing_expression() {
    let command_dir = tempdir().expect("temporary command-file directory");
    let command_path = command_dir.path().join("commands.qalc");
    std::fs::write(&command_path, "1+1\nquit\n1+1\n").expect("command file");

    let mut cmd = qalc_rs_raw();
    cmd.args(["-c0", "-t", "-f"])
        .arg(&command_path)
        .arg("1+1")
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .assert()
        .success()
        .stdout("2\n")
        .stderr("");
}

#[test]
fn cli_command_file_trailing_errors_keep_upstream_success_status() {
    let command_dir = tempdir().expect("temporary command-file directory");
    let command_path = command_dir.path().join("commands.qalc");
    std::fs::write(&command_path, "1+1\n").expect("command file");

    let mut cmd = qalc_rs_raw();
    cmd.args(["-c0", "-t", "-f"])
        .arg(&command_path)
        .arg("foo(ans)")
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .assert()
        .success()
        .stdout("2\n")
        .stderr(predicate::str::contains(
            "expression 'foo(ans)' has no native Rust implementation",
        ));
}

#[test]
fn cli_interactive_mode_continues_after_a_trailing_expression_error() {
    let command_dir = tempdir().expect("temporary command-file directory");
    let command_path = command_dir.path().join("commands.qalc");
    std::fs::write(&command_path, "1+1\n").expect("command file");

    let mut cmd = qalc_rs_raw();
    cmd.args(["-i", "-c0", "-t", "-f"])
        .arg(&command_path)
        .arg("foo(ans)")
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .write_stdin("ans+1\nquit\n")
        .assert()
        .success()
        .stdout(
            predicate::str::starts_with("2\n> ans+1\n")
                .and(predicate::str::contains("  3\n"))
                .and(predicate::str::ends_with("> quit\n")),
        )
        .stderr(predicate::str::contains(
            "expression 'foo(ans)' has no native Rust implementation",
        ));
}

#[test]
fn cli_interactive_info_retains_variables_from_the_trailing_expression() {
    let command_dir = tempdir().expect("temporary command-file directory");
    let command_path = command_dir.path().join("commands.qalc");
    std::fs::write(&command_path, "1+1\n").expect("command file");

    let mut cmd = qalc_rs_raw();
    cmd.args(["-i", "-c0", "-t", "-f"])
        .arg(&command_path)
        .arg("my_var:=10")
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .write_stdin("info my_var\nquit\n")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Variable: my_var").and(predicate::str::contains("Value: 10")),
        )
        .stderr("");
}

#[test]
fn cli_interactive_info_retains_variables_defined_by_a_command_file() {
    let command_dir = tempdir().expect("temporary command-file directory");
    let command_path = command_dir.path().join("commands.qalc");
    std::fs::write(&command_path, "x:=5\n").expect("command file");

    let mut cmd = qalc_rs_raw();
    cmd.args(["-i", "-c0", "-t", "-f"])
        .arg(&command_path)
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .write_stdin("x+1\ninfo x\nquit\n")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("  6\n")
                .and(predicate::str::contains("Variable: x"))
                .and(predicate::str::contains("Value: 5")),
        )
        .stderr("");
}

#[test]
fn cli_command_file_without_path_reports_once_and_enters_repl() {
    let mut cmd = qalc_rs_raw();
    cmd.arg("-f")
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .assert()
        .success()
        .stdout("No file specified.\n> ")
        .stderr("");
}

#[test]
fn cli_missing_command_file_matches_upstream_exit_status() {
    let command_dir = tempdir().expect("temporary command-file directory");
    let missing = command_dir.path().join("missing.qalc");
    let mut cmd = qalc_rs_raw();
    cmd.args(["-c0", "-t", "-f"])
        .arg(&missing)
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .assert()
        .code(1)
        .stdout(format!("Could not open \"{}\".\n", missing.display()))
        .stderr("");
}

#[test]
fn cli_interactive_mode_continues_after_a_missing_command_file() {
    let command_dir = tempdir().expect("temporary command-file directory");
    let missing = command_dir.path().join("missing.qalc");
    let diagnostic = format!("Could not open \"{}\".\n", missing.display());
    let mut cmd = qalc_rs_raw();
    cmd.args(["-i", "-c0", "-t", "-f"])
        .arg(&missing)
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .write_stdin("1+1\nquit\n")
        .assert()
        .success()
        .stdout(
            predicate::str::starts_with(diagnostic)
                .and(predicate::str::contains("> 1+1\n"))
                .and(predicate::str::contains("  2\n"))
                .and(predicate::str::ends_with("> quit\n")),
        )
        .stderr("");
}

#[test]
fn test_batch_workflow_stops_parsing_after_test_file() {
    for args in [
        vec!["--test-file", "dummy_test.batch"],
        vec![
            "--test-file",
            "dummy_test.batch",
            "-i",
            "-v",
            "--help",
            "1+1",
        ],
    ] {
        let mut cmd = qalc_rs_raw();
        cmd.args(args)
            .assert()
            .code(1)
            .stdout("Could not open \"dummy_test.batch\".\n")
            .stderr("");
    }
}

fn docs_upstream_qalc() -> PathBuf {
    let upstream = std::env::var_os("QALCULATE_ORACLE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("../libqalculate/src/qalc"));
    assert!(
        upstream.exists(),
        "docs example parity requires an upstream qalc binary at {}",
        upstream.display()
    );
    upstream
}

fn assert_docs_cli_example_matches_upstream(args: &[&str]) {
    let upstream = docs_upstream_qalc();

    let rust_home = tempdir().expect("Rust CLI home");
    let mut rust = qalc_rs_raw();
    let rust_output = rust
        .args(args)
        .env("HOME", rust_home.path())
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("TZ", "UTC")
        .assert()
        .success()
        .get_output()
        .clone();
    assert_eq!(rust_output.stderr, b"[qalc-rs-metadata] fallback=native\n");

    let upstream_home = tempdir().expect("upstream home");
    let mut oracle = Command::new(&upstream);
    let oracle_output = oracle
        .args(args)
        .env("HOME", upstream_home.path())
        .env("QALCULATE_DEFINITIONS_DIR", definitions_dir())
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("TZ", "UTC")
        .assert()
        .success()
        .get_output()
        .clone();

    assert_eq!(rust_output.stdout, oracle_output.stdout);
    assert!(oracle_output.stderr.is_empty());
}

#[test]
fn docs_example_readme_cli_arithmetic_matches_upstream() {
    assert_docs_cli_example_matches_upstream(&["-c0", "--", "5+2"]);
}

#[test]
fn docs_example_readme_help_matches_upstream() {
    let upstream = docs_upstream_qalc();

    let rust_home = tempdir().expect("Rust CLI home");
    let mut rust = qalc_rs_raw();
    let rust_output = rust
        .arg("--help")
        .env("HOME", rust_home.path())
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("TZ", "UTC")
        .assert()
        .success()
        .get_output()
        .clone();

    let upstream_home = tempdir().expect("upstream home");
    let mut oracle = Command::new(&upstream);
    let oracle_output = oracle
        .arg("--help")
        .env("HOME", upstream_home.path())
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("TZ", "UTC")
        .assert()
        .success()
        .get_output()
        .clone();

    assert_eq!(rust_output.stdout, oracle_output.stdout);
    assert_eq!(rust_output.stderr, oracle_output.stderr);
}

#[test]
fn docs_example_man_set_base_16_matches_upstream() {
    assert_docs_cli_example_matches_upstream(&["-c0", "-t", "-s", "base 16", "--", "52"]);
}

#[test]
fn docs_example_readme_number_base_matches_upstream() {
    assert_docs_cli_example_matches_upstream(&["-c0", "-t", "--", "52 to bin"]);
}
