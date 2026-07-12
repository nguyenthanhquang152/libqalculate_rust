use assert_cmd::Command;
use predicates::prelude::*;
use std::time::Duration;
use tempfile::{tempdir, TempDir};

struct IsolatedSession {
    _home: TempDir,
    state: TempDir,
    _config: TempDir,
    command: Command,
}

fn isolated_session() -> IsolatedSession {
    let home = tempdir().expect("temporary home");
    let state = tempdir().expect("temporary state directory");
    let config = tempdir().expect("temporary config directory");
    let config_dir = config.path().join("qalculate");
    std::fs::create_dir_all(&config_dir).expect("configuration directory");
    std::fs::write(config_dir.join("qalc.cfg"), "").expect("seeded preferences");

    let mut command = Command::cargo_bin("qalc-rs").expect("binary should build");
    command
        .env_remove("QALCULATE_REPORT_FALLBACK")
        .env("HOME", home.path())
        .env("XDG_STATE_HOME", state.path())
        .env("XDG_CONFIG_HOME", config.path())
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("TZ", "UTC")
        .env("QALCULATE_DEFINITIONS_DIR", "../libqalculate/data");

    IsolatedSession {
        _home: home,
        state,
        _config: config,
        command,
    }
}

#[test]
fn no_arguments_start_an_interactive_session_and_quit_cleanly() {
    let mut session = isolated_session();
    session
        .command
        .write_stdin("quit\n")
        .assert()
        .success()
        .stdout("> quit\n")
        .stderr("");
}

#[test]
fn fresh_profile_uses_the_line_repl_without_autocalc_onboarding() {
    let mut session = isolated_session();
    std::fs::remove_file(session._config.path().join("qalculate/qalc.cfg"))
        .expect("remove seeded preferences");
    session
        .command
        .write_stdin("quit\n")
        .assert()
        .success()
        .stdout("> quit\n")
        .stderr("");
}

#[test]
fn eof_at_an_empty_prompt_exits_successfully_without_a_newline() {
    let mut session = isolated_session();
    session.command.assert().success().stdout("> ").stderr("");
}

#[test]
fn interactive_flag_evaluates_an_initial_expression_before_prompting() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0", "--", "1+1"])
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .write_stdin("exit\n")
        .assert()
        .success()
        .stdout("\n  1 + 1 = 2\n\n> exit\n")
        .stderr("");
}

#[test]
fn interactive_session_preserves_native_answer_state() {
    let mut session = isolated_session();
    let output = session
        .command
        .args(["-i", "-c0"])
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .write_stdin("1+1\nans+1\nquit\n")
        .assert()
        .success()
        .stdout("> 1+1\n\n  1 + 1 = 2\n\n> ans+1\n\n  ans + 1 = 3\n\n> quit\n")
        .get_output()
        .clone();
    let stderr = String::from_utf8(output.stderr).expect("metadata should be UTF-8");
    assert_eq!(stderr.matches("fallback=native").count(), 2);
    assert!(!stderr.contains("cpp-fallback"));
}

#[test]
fn native_boolean_results_update_the_typed_answer_state() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .write_stdin("(1 + i) = (1 + i)\nans\nquit\n")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("i + 1 = i + 1 = true")
                .and(predicate::str::contains("ans = 1")),
        )
        .stderr("");
}

#[test]
fn native_data_counts_update_the_typed_answer_state() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .write_stdin("number(load(\"tests/vectordata.csv\"))\nans+1\nquit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("ans + 1 = 101"))
        .stderr("");
}

#[test]
fn cpp_fallback_results_update_the_typed_answer_state() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .write_stdin("5+5\nans+1\nquit\n")
        .assert()
        .success()
        .stdout("> 5+5\n\n  5 + 5 = 10\n\n> ans+1\n\n  ans + 1 = 11\n\n> quit\n")
        .stderr("");
}

#[test]
fn cpp_fallback_records_the_actual_complex_and_symbolic_results() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .write_stdin("sqrt(-1)\nans+1\ndiff(x^2,x)\nans+1\nquit\n")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("ans + 1 = 1 + i")
                .and(predicate::str::contains("ans + 1 = 2x + 1")),
        )
        .stderr("");
}

#[test]
fn cpp_fallback_preserves_structured_assignments_and_vectors() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .write_stdin("z:=sqrt(-1)\nz+1\n[1,2]\nx:=ans\nx\nquit\n")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("z + 1 = 1 + i").and(predicate::str::contains("x = [1  2]")),
        )
        .stderr("");
}

#[test]
fn native_to_cpp_transition_preserves_exact_answer_history() {
    let mut session = isolated_session();
    let output = session
        .command
        .args(["-i", "-c0"])
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .write_stdin("set unicode off\n1/3\ndiff(x^2,x)\nans2*3\nquit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("ans2 * 3 = 1"))
        .get_output()
        .clone();
    let stderr = String::from_utf8(output.stderr).expect("metadata should be UTF-8");
    assert_eq!(stderr.matches("fallback=native").count(), 1);
    assert_eq!(stderr.matches("fallback=cpp-fallback-enabled").count(), 2);
}

#[test]
fn assignments_update_the_session_context() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .write_stdin("myvar:=5\nmyvar+1\nans+1\nquit\n")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("myvar + 1 = 6").and(predicate::str::contains("ans + 1 = 7")),
        )
        .stderr("");
}

#[test]
fn session_defined_variables_are_visible_to_list_and_info() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .write_stdin("myvar:=5\nlist variables myvar\ninfo myvar\nquit\n")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("myvar\t")
                .and(predicate::str::contains("Variable: myvar"))
                .and(predicate::str::contains("Value: 5")),
        )
        .stderr("");
}

#[test]
fn nested_assignments_update_all_local_variable_info() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .write_stdin("x:=y:=5\ninfo y\nquit\n")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Variable: y")
                .and(predicate::str::contains("Value: 5"))
                .and(predicate::str::contains("Value: unknown").not()),
        )
        .stderr("");
}

#[test]
fn native_load_assignments_are_visible_to_local_info() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .write_stdin("x = load(\"tests/vectordata.csv\")\ninfo x\nquit\n")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Variable: x")
                .and(predicate::str::contains("Value: unknown").not()),
        )
        .stderr("");
}

#[test]
fn local_variable_searches_include_matching_global_definitions() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .write_stdin("s:=1\nlist s\nquit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("s\t").and(predicate::str::contains("sin (Sine)")))
        .stderr("");
}

#[test]
fn filtered_local_variable_searches_include_matching_global_variables() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .write_stdin("s:=1\nlist variables s\nquit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("s\t").and(predicate::str::contains("speed_of_light / c")))
        .stderr("");
}

#[test]
fn local_variable_info_uses_the_evaluated_assignment_value() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .write_stdin("x:=1+1\ninfo x\nquit\n")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Value: 2").and(predicate::str::contains("Value: 1+1").not()),
        )
        .stderr("");
}

#[test]
fn local_variable_info_uses_plain_values_after_markup_fallback_assignments() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0", "--html"])
        .write_stdin("x:=Ei(3)\ninfo x\nquit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Value: 9.933832571"))
        .stderr("");
}

#[test]
fn reverse_assignments_update_local_variable_info() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .write_stdin("rev =: 2\ninfo rev\nquit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Variable: rev").and(predicate::str::contains("Value: 2")))
        .stderr("");
}

#[test]
fn local_variable_info_prefers_an_exact_case_match() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .write_stdin("X:=1\nx:=2\ninfo x\nquit\n")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Variable: x")
                .and(predicate::str::contains("Value: 2"))
                .and(predicate::str::contains("Variable: X").not()),
        )
        .stderr("");
}

#[test]
fn assume_commands_do_not_block_later_cpp_fallback_evaluation() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .write_stdin("assume positive\nsqrt(x^2)\nassume unknown\nsqrt(x^2)\nquit\n")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("sqrt(x²) = x")
                .and(predicate::str::contains("sqrt(x²) = |x|")),
        )
        .stderr("");
}

#[test]
fn deleting_a_session_variable_removes_it_from_info() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .write_stdin("myvar:=5\ndelete myvar\ninfo myvar\nquit\n")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("No matching item found.")
                .and(predicate::str::contains("Variable: myvar").not()),
        )
        .stderr("");
}

#[test]
fn managed_answer_aliases_cannot_be_deleted_as_user_variables() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .write_stdin("set base 10 10\n1\ndelete ans\nans+1\nquit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("ans + 1 = 2"))
        .stderr(predicate::str::contains(
            "no user-defined variable with the name 'ans' exists",
        ));
}

#[test]
fn previous_result_conversion_commands_and_unknown_slash_commands_are_typed() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .write_stdin("1 m\nto cm\n/typo\nquit\n")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("ans to cm = 100 cm")
                .and(predicate::str::contains("Unknown command.\n\n")),
        )
        .stderr("");
}

#[test]
fn native_statistics_results_update_the_typed_answer_state() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .write_stdin("mean(5; 6; 4; 2; 3; 7)\nans+1\nquit\n")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("mean(5, 6, 4, 2, 3, 7) = 4.5")
                .and(predicate::str::contains("ans + 1 = 5.5")),
        )
        .stderr("");
}

#[test]
fn interactive_answer_history_rotates_without_cpp_fallback() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .write_stdin("1\n2\nans2\nquit\n")
        .assert()
        .success()
        .stdout("> 1\n\n  1 = 1\n\n> 2\n\n  2 = 2\n\n> ans2\n\n  ans2 = 1\n\n> quit\n")
        .stderr("");
}

#[test]
fn interactive_answer_retains_exact_value_behind_approximate_display() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .write_stdin("1/3\nans*3\nquit\n")
        .assert()
        .success()
        .stdout("> 1/3\n\n  1 / 3 ≈ 0.3333333333\n\n> ans*3\n\n  ans × 3 = 1\n\n> quit\n")
        .stderr("");
}

#[test]
fn fallback_disabled_native_results_remain_available_as_answers() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .write_stdin(
            "1 m to cm\nans\nans/2\n9.8 m/s^2\nans\natom(\"H\";\"weight\")\nans\n1 USD to EUR\nans\nans*2\ntimestamp(\"1970-01-01\")\nans+1\nquit\n",
        )
        .assert()
        .success()
        .stdout(
            predicate::str::contains("ans = 1 m")
                .and(predicate::str::contains("ans / 2 = m / 2"))
                .and(predicate::str::contains("ans = 9.8 m / s^2"))
                .and(predicate::str::contains("ans = 1.008 u"))
                .and(predicate::str::contains("ans ≈ €0.8585164835"))
                .and(predicate::str::contains("ans × 2 ≈ €1.717032967"))
                .and(predicate::str::contains("ans + 1 = 1")),
        )
        .stderr("");
}

#[test]
fn fallback_disabled_datetime_results_remain_typed_answers() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .write_stdin(
            "addDays(\"2024-01-01\";7)\nans\n\"2024-01-10\" - \"2024-01-01\"\nans\nstamptodate(0) to UTC\nans\nquit\n",
        )
        .assert()
        .success()
        .stdout(
            predicate::str::contains("ans = \"2024-01-08\"")
                .and(predicate::str::contains("ans = 9 d"))
                .and(predicate::str::contains(
                    "ans = \"1970-01-01T00:00:00Z\"",
                )),
        )
        .stderr("");
}

#[test]
fn interactive_settings_recalculate_and_persist() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .write_stdin(
            "set base 16\n10\nset base 10\n10\n/set unicode off\n2*3\n/set unicode on\n2*3\nquit\n",
        )
        .assert()
        .success()
        .stdout(
            "> set base 16\n> 10\n\n  10 = 0xA\n\n> set base 10\n\n  0xA = 10\n\n> 10\n\n  10 = 10\n\n> /set unicode off\n\n  10 = 10\n\n> 2*3\n\n  2 * 3 = 6\n\n> /set unicode on\n\n  6 = 6\n\n> 2*3\n\n  2 × 3 = 6\n\n> quit\n",
        )
        .stderr("");
}

#[test]
fn assumption_changes_recalculate_the_last_expression() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .write_stdin("assume positive\nsqrt(x^2)\nassume unknown\nquit\n")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("sqrt(x²) = |x|").and(predicate::str::contains("x = x").not()),
        )
        .stderr("");
}

#[test]
fn stored_answer_reformat_uses_the_evaluated_value_with_a_new_input_base() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .write_stdin("set base 10 10\n10\nset base 16 16\nquit\n")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("10 = 0xA").and(predicate::str::contains("10 = 0x10").not()),
        )
        .stderr("");
}

#[test]
fn nondecimal_input_base_results_update_the_typed_answer_state() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .write_stdin("set input base 16\nA\nans+1\nquit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("ans + 1 = 11"))
        .stderr("");
}

#[test]
fn cpp_fallback_honors_nondecimal_input_base() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .write_stdin("set input base 16\nsin(A)\nquit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("0.5440211109"))
        .stderr("");
}

#[test]
fn native_noninteger_base_conversions_update_the_typed_answer_state() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .write_stdin("set unicode on\n52.34 to sexa\nans+1\nquit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("ans + 1 = 53.34"))
        .stderr("");

    let mut float_session = isolated_session();
    float_session
        .command
        .args(["-i", "-c0"])
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .write_stdin("float(01000010010100010110000101001000)\nans+1\nquit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("ans + 1 ≈ 53.34500122"))
        .stderr("");
}

#[test]
fn unsupported_setting_is_rejected_without_poisoning_later_evaluations() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .write_stdin("set decimal comma 1\n1+1\nquit\n")
        .assert()
        .success()
        .stdout("> set decimal comma 1\n> 1+1\n\n  1 + 1 = 2\n\n> quit\n")
        .stderr(predicate::str::contains(
            "session settings are not supported by the C++ FFI fallback path: decimal comma 1",
        ));
}

#[test]
fn cpp_answer_is_reformatted_from_the_retained_value() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .write_stdin("1+1\nsin(1)\nset base 16\nans+1\nquit\n")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("sin(1) = 0.8414709848")
                .and(predicate::str::contains("0.8414709848 ≈ 0x0.D76AA4"))
                .and(predicate::str::contains("ans + 1 ≈ 0x1.D76AA48"))
                .and(predicate::str::contains("2 = 0x2").not()),
        )
        .stderr("");
}

#[test]
fn cpp_answer_reformat_reports_the_cpp_fallback_state() {
    let mut session = isolated_session();
    let output = session
        .command
        .args(["-i", "-c0"])
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .write_stdin("sin(1)\nset base 16\nquit\n")
        .assert()
        .success()
        .get_output()
        .clone();
    let stderr = String::from_utf8(output.stderr).expect("metadata should be UTF-8");
    assert_eq!(stderr.matches("fallback=cpp-fallback-enabled").count(), 2);
    assert!(!stderr.contains("fallback=native"));
}

#[test]
fn cpp_answer_reformat_rejects_settings_the_cpp_printer_cannot_apply() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .write_stdin("1/3\nset precision 20\nans\nquit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("ans = 0.3333333333"))
        .stderr(predicate::str::contains(
            "session settings are not supported by the C++ FFI fallback path: precision 20",
        ));
}

#[test]
fn cpp_answer_reformat_reads_approximation_metadata_after_printing() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .write_stdin("1/3\nset base 16\nquit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("0.3333333333 ≈ 0x0.555555"))
        .stderr("");
}

#[test]
fn native_markup_updates_the_typed_answer_state() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0", "--html"])
        .write_stdin("sin(1)\nans+1\nquit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("1.841470985"))
        .stderr("");
}

#[test]
fn native_markup_conversion_results_update_the_typed_answer_state() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .write_stdin("1/2 to latex\nans+1\nquit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("ans + 1 = 1.5"))
        .stderr("");
}

#[test]
fn native_markup_assignments_persist_in_the_session_context() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0", "--html"])
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .write_stdin("x:=1\nx+1\nquit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("<i>x</i> + 1 = 2"))
        .stderr("");
}

#[test]
fn native_markup_assignments_are_available_to_cpp_fallback() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0", "--html"])
        .write_stdin("x:=1\nEi(x)\nquit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("1.895117816"))
        .stderr("");
}

#[test]
fn nested_cpp_assignments_are_available_to_native_markup() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0", "--html"])
        .write_stdin("x:=y:=5\nx+y\nquit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("<i>x</i> + <i>y</i> = 10"))
        .stderr("");
}

#[test]
fn embedded_markup_assignments_persist_with_cpp_fallback_enabled() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0", "--html"])
        .write_stdin("(x:=5)+1\ninfo x\nx+1\nEi(x)\nquit\n")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("<i>x</i> + 1 = 6")
                .and(predicate::str::contains("Variable: x"))
                .and(predicate::str::contains("Value: 5"))
                .and(predicate::str::contains("40.18527536")),
        )
        .stderr("");
}

#[test]
fn chained_markup_assignments_persist_with_cpp_fallback_disabled() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0", "--html"])
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .write_stdin("x:=y:=5\ny+1\nquit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("<i>y</i> + 1 = 6"))
        .stderr("");
}

#[test]
fn native_markup_variables_survive_an_intervening_cpp_fallback() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0", "--html"])
        .write_stdin("x:=1\nEi(3)\nx+1\nquit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("<i>x</i> + 1 = 2"))
        .stderr("");
}

#[test]
fn native_markup_can_combine_native_variables_with_cpp_answers() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0", "--html"])
        .write_stdin("x:=1\nEi(3)\nx+ans\nquit\n")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("<i>x</i> + <i>ans</i>")
                .and(predicate::str::contains("10.933")),
        )
        .stderr("");
}

#[test]
fn native_markup_can_use_cpp_answer_history_with_native_variables() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0", "--html"])
        .write_stdin("x:=1\n2\nEi(3)\nx+ans2\nquit\n")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("<i>x</i> + <i>ans2</i>").and(predicate::str::contains("= 3")),
        )
        .stderr("");
}

#[test]
fn native_markup_can_use_cpp_assigned_variables() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0", "--html"])
        .write_stdin("x:=1\nz:=Ei(3)\nx+z\nquit\n")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("<i>x</i> + <i>z</i>").and(predicate::str::contains("10.933")),
        )
        .stderr("");
}

#[test]
fn cpp_markup_fallback_updates_the_typed_answer_state() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0", "--html"])
        .env("QALCULATE_REPORT_FALLBACK", "1")
        .write_stdin("Ei(3)\nans+1\nquit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("10.93383257"))
        .stderr(predicate::str::contains("fallback=cpp-fallback-enabled"));
}

#[test]
fn native_markup_honors_output_base_settings() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0", "--html"])
        .write_stdin("set base 16\n10\nquit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("10 = 0xA"))
        .stderr("");
}

#[test]
fn answer_reformat_uses_the_active_markup_mode() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0", "--latex"])
        .write_stdin("sin(1)\nset base 16\nquit\n")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("\\approx")
                .and(predicate::str::contains("$"))
                .and(predicate::str::contains("0x0.D76AA4")),
        )
        .stderr("");
}

#[test]
fn interactive_text_terse_cpp_fallback_matches_result_only_mode() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0", "-t"])
        .write_stdin("Ei(3)\nans+1\nquit\n")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("  9.933832571\n")
                .and(predicate::str::contains("  10.93383257\n"))
                .and(predicate::str::contains("≈").not()),
        )
        .stderr("");
}

#[test]
fn interactive_terse_answer_reformat_remains_result_only() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0", "-t"])
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .write_stdin("1+1\nset base 16\nquit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("  0x2\n").and(predicate::str::contains("2 = 0x2").not()))
        .stderr("");
}

#[test]
fn native_empty_interval_results_update_the_typed_answer_state() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .write_stdin(
            "set interval display 2\nset ic 2\nintersect(interval(1;2), interval(3;4))\nans\nquit\n",
        )
        .assert()
        .success()
        .stdout(predicate::str::contains("ans = []"))
        .stderr("");
}

#[test]
fn native_promoted_list_results_update_the_typed_answer_state() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .write_stdin("( 1; 2; 3, 4, 5, 6 ); (4; 5)\nans\nquit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "ans = [[1  2  3  4  5  6]  [4  5]]",
        ))
        .stderr("");
}

#[test]
fn calendar_rendering_retains_the_underlying_date_answer() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .write_stdin("date(2024;1;1) to calendars\nans\nquit\n")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Gregorian:")
                .and(predicate::str::contains("ans = \"2024-01-01\"")),
        )
        .stderr("");
}

#[test]
fn session_answer_respects_the_active_output_base() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .write_stdin("set base 16\n10\nans+1\nquit\n")
        .assert()
        .success()
        .stdout("> set base 16\n> 10\n\n  10 = 0xA\n\n> ans+1\n\n  ans + 1 = 0xB\n\n> quit\n")
        .stderr("");
}

#[test]
fn interactive_accepts_separate_input_and_output_bases() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .write_stdin("set base 10 16\n10\nquit\n")
        .assert()
        .success()
        .stdout("> set base 10 16\n> 10\n\n  10 = 16\n\n> quit\n")
        .stderr("");
}

#[test]
fn input_base_changes_after_cpp_answers_apply_to_later_expressions() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .write_stdin("sin(1)\nset input base 16\nA\nquit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("A = 10"))
        .stderr("");
}

#[test]
fn unsupported_native_session_calls_fall_back_to_cpp() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .write_stdin("set unicode off\n1/3\nEi(ans)\nquit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("-0.15809"))
        .stderr("");
}

#[test]
fn history_uses_xdg_state_and_clear_history_truncates_it() {
    let mut first = isolated_session();
    let history_path = first.state.path().join("qalculate/qalc.history");
    first
        .command
        .args(["-i", "-c0"])
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .write_stdin("1+1\nquit\n")
        .assert()
        .success();

    let history = std::fs::read_to_string(&history_path).expect("history should be persisted");
    assert_eq!(history, "1+1\n");

    let mut clear = isolated_session();
    clear.command.env("XDG_STATE_HOME", first.state.path());
    clear
        .command
        .args(["-i", "-c0"])
        .write_stdin("clear history\nquit\n")
        .assert()
        .success();

    assert_eq!(
        std::fs::read_to_string(history_path).expect("history file should remain readable"),
        ""
    );
}

#[test]
fn unreadable_history_path_disables_persistence_without_aborting() {
    let mut session = isolated_session();
    let state_file = tempfile::NamedTempFile::new().expect("state path sentinel");
    session.command.env("XDG_STATE_HOME", state_file.path());
    session
        .command
        .write_stdin("quit\n")
        .assert()
        .success()
        .stdout("> quit\n")
        .stderr("");
}

#[test]
fn interactive_help_list_and_info_commands_use_typed_catalogs() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .write_stdin("HeLp history\nLiSt functions sin\nInFo sin\nquit\n")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Lists the expression history.")
                .and(predicate::str::contains("sin (Sine)"))
                .and(predicate::str::contains("Function: Sine"))
                .and(predicate::str::contains("sin(Angle)")),
        )
        .stderr("");
}

#[test]
fn interactive_info_renders_variable_unit_and_prefix_details() {
    let mut session = isolated_session();
    session
        .command
        .args(["-i", "-c0"])
        .write_stdin("info pi\ninfo meter\ninfo mega\nquit\n")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Variable: Archimedes' Constant (pi)")
                .and(predicate::str::contains("Value: built-in"))
                .and(predicate::str::contains("Unit: Meter"))
                .and(predicate::str::contains("System: SI"))
                .and(predicate::str::contains("Prefix\n"))
                .and(predicate::str::contains("Value: 10^6")),
        )
        .stderr("");
}

#[test]
fn stateful_pipe_transcript_matches_the_upstream_oracle() {
    let upstream = std::path::Path::new("../libqalculate/src/qalc");
    if !upstream.exists() {
        return;
    }
    let input = "1+1\nans+1\nset base 16\n15\nset base 10\n15\n/set unicode off\n2*3\n/set unicode on\n2*3\nquit\n";

    let mut rust = isolated_session();
    let rust_output = rust
        .command
        .args(["-i", "-c0"])
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .write_stdin(input)
        .assert()
        .success()
        .get_output()
        .clone();

    let upstream_home = tempdir().expect("upstream home");
    let upstream_state = tempdir().expect("upstream state");
    let upstream_config = tempdir().expect("upstream config");
    let upstream_config_dir = upstream_config.path().join("qalculate");
    std::fs::create_dir_all(&upstream_config_dir).expect("upstream config directory");
    std::fs::write(
        upstream_config_dir.join("qalc.cfg"),
        "colorize=0\ncalculate_as_you_type=0\n",
    )
    .expect("upstream preferences");
    let mut oracle = Command::new(upstream);
    let oracle_output = oracle
        .args(["-i", "-c0"])
        .env("HOME", upstream_home.path())
        .env("XDG_STATE_HOME", upstream_state.path())
        .env("XDG_CONFIG_HOME", upstream_config.path())
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("TZ", "UTC")
        .env("QALCULATE_DEFINITIONS_DIR", "../libqalculate/data")
        .write_stdin(input)
        .assert()
        .success()
        .get_output()
        .clone();

    assert_eq!(rust_output.stdout, oracle_output.stdout);
    assert_eq!(rust_output.stderr, oracle_output.stderr);
}

#[cfg(target_os = "linux")]
#[test]
fn pty_smoke_covers_prompt_quit_and_answer_state() {
    let session = isolated_session();
    let binary = assert_cmd::cargo::cargo_bin!("qalc-rs");
    let command_line = format!("'{}' -i -c0", binary.display());
    let mut cmd = Command::new("script");
    cmd.args(["-qfec", &command_line, "/dev/null"])
        .env("HOME", session._home.path())
        .env("XDG_STATE_HOME", session.state.path())
        .env("XDG_CONFIG_HOME", session._config.path())
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("TZ", "UTC")
        .env("QALCULATE_DEFINITIONS_DIR", "../libqalculate/data")
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .timeout(Duration::from_secs(10))
        .write_stdin("1+1\nans+1\nquit\n")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("1 + 1 = 2")
                .and(predicate::str::contains("ans + 1 = 3"))
                .and(predicate::str::ends_with("> ")),
        );
}
