#![forbid(unsafe_code)]
mod cli;
mod listing;

use std::env;
use std::path::{Path, PathBuf};

use libqalculate_rust::batch::read_batch_cases;
use libqalculate_rust::ffi::Calculator;
use libqalculate_rust::UPSTREAM_LIBQALCULATE_VERSION;

enum EvaluationOutcome {
    Success,
    MessageError,
}

fn main() {
    let raw_args = env::args().skip(1).collect::<Vec<String>>();
    if let Some(first) = raw_args.first() {
        match first.as_str() {
            "--self-check" => {
                if let Err(error) = self_check() {
                    exit_with_error(&error);
                }
                return;
            }
            "--list-upstream-tests" => {
                if let Err(error) = list_upstream_tests() {
                    exit_with_error(&error);
                }
                return;
            }
            "--parse-batch" => {
                match raw_args.get(1) {
                    Some(path) => {
                        if let Err(error) = parse_batch(Path::new(path)) {
                            exit_with_error(&error);
                        }
                    }
                    None => exit_with_error("--parse-batch requires a file path"),
                }
                return;
            }
            _ => {}
        }
    }

    let invocation = cli::parse_args(env::args());

    // qalc-rs has no persistent configuration reader yet, so every invocation
    // starts from built-in defaults. `--defaults` is retained as typed state so
    // adding config loading later cannot silently change this contract.

    // Upstream emits diagnostics as it scans argv, before a later terminating action.
    for diagnostic in &invocation.diagnostics {
        println!("{diagnostic}");
    }

    if let Some(action) = invocation.immediate {
        match action {
            cli::ImmediateAction::Help => {
                print!("{}", cli::HELP_TEXT);
                return;
            }
            cli::ImmediateAction::Version => {
                println!("{UPSTREAM_LIBQALCULATE_VERSION}");
                return;
            }
        }
    }

    if invocation.color == cli::ColorMode::On {
        exit_with_error("forced coloring is not implemented (owner issue: #198)");
    }

    if invocation.exrates {
        if let Err(err) = validate_exchange_rates() {
            exit_with_error(&err);
        }
        // If rates validation is the only requested action
        if invocation.list.is_none()
            && invocation.expression.is_none()
            && !invocation.interactive
            && invocation.command_file.is_none()
        {
            return;
        }
    }

    if invocation.interactive {
        exit_with_error("Interactive mode is not implemented (owner #61)");
    }
    if let Some(ref file) = invocation.command_file {
        match file.mode {
            cli::CommandFileMode::Commands => {
                exit_with_error("Command file execution is not implemented (owner #62)");
            }
            cli::CommandFileMode::Test => {
                exit_with_error("Test file execution is not implemented (owner #63)");
            }
        }
    }
    if let Some(ref list_req) = invocation.list {
        let data_dir = libqalculate_rust::rates::definitions_dir();
        let unicode_enabled = invocation.unicode.unwrap_or(true);
        match listing::render_list(
            &data_dir,
            list_req,
            &invocation.definitions,
            unicode_enabled,
        ) {
            Ok(output) => {
                print!("{output}");
            }
            Err(err) => {
                exit_with_error(&err);
            }
        }
        return;
    }

    let Some(expression) = invocation.expression.as_deref() else {
        exit_with_error("Interactive mode is not implemented (owner #61)");
    };

    match evaluate_expression(&invocation, expression) {
        Ok(EvaluationOutcome::Success) => {}
        Ok(EvaluationOutcome::MessageError) => std::process::exit(1),
        Err(error) => exit_with_error(&error),
    }
}

fn self_check() -> Result<(), String> {
    let tests = upstream_batch_files()?;
    if tests.is_empty() {
        return Err("no upstream .batch fixtures found".to_owned());
    }
    println!("upstream_version={UPSTREAM_LIBQALCULATE_VERSION}");
    println!("upstream_batch_files={}", tests.len());
    Ok(())
}

fn list_upstream_tests() -> Result<(), String> {
    for path in upstream_batch_files()? {
        println!("{}", path.display());
    }
    Ok(())
}

fn parse_batch(path: &Path) -> Result<(), String> {
    let cases = read_batch_cases(path).map_err(|error| error.to_string())?;
    println!("cases={}", cases.len());
    Ok(())
}

fn validate_exchange_rates() -> Result<(), String> {
    let dir = libqalculate_rust::rates::definitions_dir();
    libqalculate_rust::rates::RatesJsonSnapshot::load_from_dir(&dir)
        .map_err(|err| format!("failed to load rates JSON: {err}"))?;
    libqalculate_rust::rates::RatesCatalog::load_from_dir(&dir)
        .map_err(|err| format!("failed to load rates catalog: {err}"))?;
    Ok(())
}

fn evaluate_expression(
    invocation: &cli::CliInvocation,
    expression: &str,
) -> Result<EvaluationOutcome, String> {
    let fallback_disabled = std::env::var("QALCULATE_DISABLE_FALLBACK").as_deref() == Ok("1");
    let report_fallback = std::env::var("QALCULATE_REPORT_FALLBACK").as_deref() == Ok("1");

    let defs = &invocation.definitions;
    if !defs.units || !defs.currencies || !defs.functions || !defs.variables || !defs.datasets {
        if fallback_disabled {
            return Err("selective definitions are unsupported for native evaluation".to_string());
        } else {
            return Err("selective definitions are incompatible with fallback".to_string());
        }
    }
    if fallback_disabled
        && !defs.global_defs
        && libqalculate_rust::ffi::native_expression_uses_global_definitions(expression)
    {
        return Err("global definitions are disabled for this native expression".to_string());
    }

    let unicode_setting = invocation
        .unicode
        .map(|enabled| format!("unicode {}", i32::from(enabled)));
    let programming_setting = invocation
        .programming_mode
        .then(|| "programming mode 1".to_string());

    let mut setting_refs = Vec::with_capacity(
        invocation.settings.len()
            + usize::from(unicode_setting.is_some())
            + usize::from(programming_setting.is_some()),
    );
    if let Some(setting) = unicode_setting.as_deref() {
        setting_refs.push(setting);
    }
    if let Some(setting) = programming_setting.as_deref() {
        setting_refs.push(setting);
    }
    setting_refs.extend(invocation.settings.iter().map(String::as_str));

    let mut calc = Calculator::new();
    if !invocation.definitions.global_defs && !fallback_disabled {
        return Err(
            "fallback evaluation requires global definitions; -nodefs is incompatible with fallback"
                .to_owned(),
        );
    }
    if invocation.definitions.global_defs && !fallback_disabled && !calc.load_global_definitions() {
        return Err("failed to load global definitions".to_owned());
    }

    let timeout = if invocation.timeout_ms == 0 {
        1000
    } else {
        invocation.timeout_ms
    };
    let result = match invocation.output_mode {
        cli::OutputMode::Text => {
            if invocation.terse {
                calc.calculate_and_print_qalc_with_settings_and_fallback_state(
                    expression,
                    &setting_refs,
                    timeout,
                )
            } else {
                calc.calculate_and_print_qalc_equation_with_settings_and_fallback_state(
                    expression,
                    &setting_refs,
                    timeout,
                )
            }
        }
        cli::OutputMode::Latex => {
            if invocation.terse {
                calc.calculate_and_print_qalc_latex_terse_with_settings_and_fallback_state(
                    expression,
                    &setting_refs,
                    timeout,
                )
            } else {
                calc.calculate_and_print_qalc_latex_with_settings_and_fallback_state(
                    expression,
                    &setting_refs,
                    timeout,
                )
            }
        }
        cli::OutputMode::Html => {
            if invocation.terse {
                calc.calculate_and_print_qalc_html_terse_with_settings_and_fallback_state(
                    expression,
                    &setting_refs,
                    timeout,
                )
            } else {
                calc.calculate_and_print_qalc_html_with_settings_and_fallback_state(
                    expression,
                    &setting_refs,
                    timeout,
                )
            }
        }
    };

    match result {
        Ok(result) => {
            let native_message_had_error = calc.last_native_message_had_error();
            if report_fallback {
                eprintln!("[qalc-rs-metadata] {}", result.fallback_state.marker());
            }
            println!("{}", result.output);
            if native_message_had_error {
                Ok(EvaluationOutcome::MessageError)
            } else {
                Ok(EvaluationOutcome::Success)
            }
        }
        Err(err) => {
            if report_fallback {
                eprintln!("[qalc-rs-metadata] {}", err.fallback_state().marker());
            }
            Err(format!("calculation failed: {err}"))
        }
    }
}

fn upstream_batch_files() -> Result<Vec<PathBuf>, String> {
    let upstream = env::var_os("LIBQALCULATE_UPSTREAM_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("../libqalculate"));
    let tests_dir = upstream.join("tests");
    let entries = std::fs::read_dir(&tests_dir)
        .map_err(|error| format!("failed to read {}: {error}", tests_dir.display()))?;
    let mut batches = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "batch"))
        .collect::<Vec<_>>();
    batches.sort();
    Ok(batches)
}

fn exit_with_error(message: &str) -> ! {
    eprintln!("error: {message}");
    std::process::exit(2);
}
