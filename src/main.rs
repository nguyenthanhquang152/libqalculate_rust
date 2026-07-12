#![forbid(unsafe_code)]
mod cli;
mod listing;

use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, IsTerminal};
use std::path::{Path, PathBuf};

use libqalculate_rust::batch::read_batch_cases;
use libqalculate_rust::ffi::{Calculator, FallbackState};
use libqalculate_rust::parser::commands::{parse_command, SessionCommand, SetSetting};
use libqalculate_rust::UPSTREAM_LIBQALCULATE_VERSION;

struct EvaluationOutcome {
    output: String,
    message_error: bool,
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

    let mut invocation = cli::parse_args(env::args());

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

    if let Some(ref list_req) = invocation.list {
        let data_dir = libqalculate_rust::rates::definitions_dir();
        let unicode_enabled = cli_unicode_enabled(&invocation);
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

    if let Some(command_file) = invocation
        .command_file
        .as_ref()
        .filter(|file| file.mode == cli::CommandFileMode::Test)
    {
        if command_file.path.is_empty() {
            print!("> \x1b[31m\nWARNING: 0 tests were run (indentation needs to be tab-based)\n\n\x1b[0m");
            return;
        }
        if File::open(&command_file.path).is_err() {
            println!("Could not open \"{}\".", command_file.path);
            std::process::exit(1);
        }
    }

    let mut calculator = match prepare_calculator(&invocation) {
        Ok(calculator) => calculator,
        Err(error) => exit_with_error(&error),
    };

    if let Some(command_file) = invocation.command_file.clone() {
        if command_file.mode == cli::CommandFileMode::Test {
            calculator.enable_session_mode();
            let mut session = cli::repl::ReplSessionState::default();

            let exit_code = run_test_file(
                &mut invocation,
                &mut calculator,
                &mut session,
                &command_file,
            );
            std::process::exit(exit_code);
        }

        calculator.enable_session_mode();
        let mut session = cli::repl::ReplSessionState::default();
        match run_command_file(
            &mut invocation,
            &mut calculator,
            &mut session,
            &command_file,
        ) {
            cli::repl::CommandStreamExit::Quit => return,
            cli::repl::CommandStreamExit::Eof(exit_code)
                if exit_code != 0 && !invocation.interactive =>
            {
                drop(calculator);
                std::process::exit(exit_code);
            }
            cli::repl::CommandStreamExit::Eof(_) => {}
        }

        if let Some(expression) = invocation.expression.clone() {
            run_trailing_expression(&invocation, &mut calculator, &mut session, &expression);
        }

        if invocation.interactive {
            let exit_code =
                run_interactive_session(&mut invocation, &mut calculator, &mut session, None);
            drop(calculator);
            if exit_code != 0 {
                std::process::exit(exit_code);
            }
        }
        return;
    }

    if invocation.interactive || invocation.expression.is_none() {
        calculator.enable_session_mode();
        let mut session = cli::repl::ReplSessionState::default();
        let initial_expression = invocation.expression.clone();
        let exit_code = run_interactive_session(
            &mut invocation,
            &mut calculator,
            &mut session,
            initial_expression,
        );
        drop(calculator);
        if exit_code != 0 {
            std::process::exit(exit_code);
        }
        return;
    }

    let expression = invocation
        .expression
        .as_deref()
        .expect("non-interactive invocation must contain an expression");

    match evaluate_expression(&invocation, &mut calculator, expression) {
        Ok(outcome) => {
            drop(calculator);
            println!("{}", outcome.output);
            if outcome.message_error {
                std::process::exit(1);
            }
        }
        Err(error) => {
            drop(calculator);
            exit_with_error(&error);
        }
    }
}

fn run_interactive_session(
    invocation: &mut cli::CliInvocation,
    calculator: &mut Calculator,
    session: &mut cli::repl::ReplSessionState,
    initial_expression: Option<String>,
) -> i32 {
    let echo_input = !io::stdin().is_terminal();
    let stdin = io::stdin();
    let stdout = io::stdout();
    let stderr = io::stderr();
    let mut input = stdin.lock();
    let mut output = stdout.lock();
    let mut error = stderr.lock();
    cli::repl::run(
        invocation,
        session,
        cli::repl::ReplIo::new(&mut input, &mut output, &mut error),
        echo_input,
        initial_expression,
        |invocation, request| evaluate_repl_request(invocation, calculator, request),
    )
}

fn run_command_file(
    invocation: &mut cli::CliInvocation,
    calculator: &mut Calculator,
    session: &mut cli::repl::ReplSessionState,
    command_file: &cli::CommandFile,
) -> cli::repl::CommandStreamExit {
    if command_file.path == "-" {
        let stdin = io::stdin();
        let mut input = stdin.lock();
        return run_command_input(invocation, calculator, session, &mut input);
    }

    let file = match File::open(&command_file.path) {
        Ok(file) => file,
        Err(_) => {
            println!("Could not open \"{}\".", command_file.path);
            return cli::repl::CommandStreamExit::Eof(1);
        }
    };
    let mut input = BufReader::new(file);
    run_command_input(invocation, calculator, session, &mut input)
}

fn run_command_input<R: BufRead>(
    invocation: &mut cli::CliInvocation,
    calculator: &mut Calculator,
    session: &mut cli::repl::ReplSessionState,
    input: &mut R,
) -> cli::repl::CommandStreamExit {
    let stdout = io::stdout();
    let stderr = io::stderr();
    let mut output = stdout.lock();
    let mut error = stderr.lock();
    cli::repl::run_command_stream(
        invocation,
        session,
        cli::repl::ReplIo::new(input, &mut output, &mut error),
        |invocation, request| evaluate_repl_request(invocation, calculator, request),
    )
}

fn run_trailing_expression(
    invocation: &cli::CliInvocation,
    calculator: &mut Calculator,
    session: &mut cli::repl::ReplSessionState,
    expression: &str,
) {
    match evaluate_expression(invocation, calculator, expression) {
        Ok(outcome) => {
            let assignment_renderings = calculator.session_assignment_renderings(expression);
            session.record_evaluation(expression.to_string(), &assignment_renderings);
            println!("{}", outcome.output);
        }
        Err(error) => {
            eprintln!("error: {error}");
        }
    }
}

fn evaluate_repl_request(
    invocation: &cli::CliInvocation,
    calculator: &mut Calculator,
    request: cli::repl::ReplRequest,
) -> Result<Option<cli::repl::ReplEvaluation>, String> {
    match request {
        cli::repl::ReplRequest::Evaluate(expression) => {
            evaluate_expression(invocation, calculator, &expression).map(|outcome| {
                let answer_rendering = calculator.session_answer_rendering();
                let assignment_renderings = calculator.session_assignment_renderings(&expression);
                Some(cli::repl::ReplEvaluation {
                    output: outcome.output,
                    answer_rendering,
                    assignment_renderings,
                    function_info: None,
                })
            })
        }
        cli::repl::ReplRequest::DefineVariable { name, expression } => calculator
            .define_session_variable(&name, &expression)
            .map(|rendering| {
                Some(cli::repl::ReplEvaluation {
                    output: String::new(),
                    answer_rendering: None,
                    assignment_renderings: vec![(name, rendering)],
                    function_info: None,
                })
            })
            .ok_or_else(|| "Illegal name.".to_string()),
        cli::repl::ReplRequest::DefineFunction { name, expression } => calculator
            .define_session_function(&name, &expression)
            .map(|function_info| {
                Some(cli::repl::ReplEvaluation {
                    output: String::new(),
                    answer_rendering: None,
                    assignment_renderings: Vec::new(),
                    function_info: Some(function_info),
                })
            })
            .ok_or_else(|| "Illegal name.".to_string()),
        cli::repl::ReplRequest::DeleteVariable {
            name,
            allow_managed_alias,
        } => {
            let deleted = if allow_managed_alias {
                calculator.delete_session_variable_override(&name)
            } else {
                calculator.delete_session_variable(&name)
            };
            if deleted {
                Ok(None)
            } else {
                Err("no matching user-defined variable".to_string())
            }
        }
        cli::repl::ReplRequest::DeleteFunction(name) => {
            if calculator.delete_session_function(&name) {
                Ok(None)
            } else {
                Err(format!(
                    "no user-defined variable or function with the name '{name}' exists"
                ))
            }
        }
        cli::repl::ReplRequest::ReformatLastAnswer => {
            reformat_session_answer(invocation, calculator)
        }
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

fn cli_unicode_enabled(invocation: &cli::CliInvocation) -> bool {
    let mut enabled = invocation.unicode.unwrap_or(true);
    for setting in &invocation.settings {
        let trimmed = setting.trim_start();
        let command = if trimmed.starts_with("set ") || trimmed.starts_with("/set ") {
            setting.clone()
        } else {
            format!("set {setting}")
        };
        if let Ok(SessionCommand::Set(command)) = parse_command(&command) {
            if let SetSetting::Unicode(value) = command.setting {
                enabled = value;
            }
        }
    }
    for command in &invocation.interactive_settings {
        if let SessionCommand::Set(command) = command {
            if let SetSetting::Unicode(value) = &command.setting {
                enabled = *value;
            }
        }
    }
    enabled
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
    calc: &mut Calculator,
    expression: &str,
) -> Result<EvaluationOutcome, String> {
    let fallback_disabled = std::env::var("QALCULATE_DISABLE_FALLBACK").as_deref() == Ok("1");
    let report_fallback = std::env::var("QALCULATE_REPORT_FALLBACK").as_deref() == Ok("1");

    let defs = &invocation.definitions;
    let selective_definitions_disabled =
        !defs.units || !defs.currencies || !defs.functions || !defs.variables || !defs.datasets;
    if selective_definitions_disabled
        && calc.session_expression_uses_disabled_definition_family(
            expression,
            defs.units,
            defs.currencies,
            defs.functions,
            defs.variables,
            defs.datasets,
        )
    {
        if fallback_disabled {
            return Err("selective definitions are unsupported for native evaluation".to_string());
        } else {
            return Err("selective definitions are incompatible with fallback".to_string());
        }
    }
    if !defs.global_defs && calc.session_expression_uses_global_definitions(expression) {
        return Err("global definitions are disabled for this native expression".to_string());
    }

    let settings = evaluation_settings(invocation, fallback_disabled);
    let setting_refs = settings.iter().map(String::as_str).collect::<Vec<_>>();

    let timeout = if invocation.timeout_ms == 0 {
        1000
    } else {
        invocation.timeout_ms
    };
    let result = match invocation.output_mode {
        cli::OutputMode::Text => {
            if invocation.terse {
                calc.calculate_and_print_qalc_terse_with_settings_and_fallback_state(
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
        Ok(mut result) => {
            let message_had_error =
                calc.last_native_message_had_error() || !invocation.definitions.global_defs;
            if report_fallback {
                eprintln!("[qalc-rs-metadata] {}", result.fallback_state.marker());
            }
            if !invocation.definitions.global_defs
                && !invocation.terse
                && result.fallback_state == FallbackState::Native
            {
                result.output = format!(
                    "error: Radians unit is missing. Creating one for this session.\n{}",
                    result.output
                );
            }
            Ok(EvaluationOutcome {
                output: result.output,
                message_error: message_had_error,
            })
        }
        Err(err) => {
            if report_fallback {
                eprintln!("[qalc-rs-metadata] {}", err.fallback_state().marker());
            }
            Err(format!("calculation failed: {err}"))
        }
    }
}

fn reformat_session_answer(
    invocation: &cli::CliInvocation,
    calc: &mut Calculator,
) -> Result<Option<cli::repl::ReplEvaluation>, String> {
    let fallback_disabled = std::env::var("QALCULATE_DISABLE_FALLBACK").as_deref() == Ok("1");
    let settings = evaluation_settings(invocation, fallback_disabled);
    let setting_refs = settings.iter().map(String::as_str).collect::<Vec<_>>();
    let result = match (invocation.output_mode, invocation.terse) {
        (cli::OutputMode::Text, false) => calc.reformat_session_answer_with_settings(&setting_refs),
        (cli::OutputMode::Text, true) => {
            calc.reformat_session_answer_terse_with_settings(&setting_refs)
        }
        (cli::OutputMode::Latex, false) => {
            calc.reformat_session_answer_latex_with_settings(&setting_refs)
        }
        (cli::OutputMode::Latex, true) => {
            calc.reformat_session_answer_latex_terse_with_settings(&setting_refs)
        }
        (cli::OutputMode::Html, false) => {
            calc.reformat_session_answer_html_with_settings(&setting_refs)
        }
        (cli::OutputMode::Html, true) => {
            calc.reformat_session_answer_html_terse_with_settings(&setting_refs)
        }
    }
    .map_err(|error| format!("calculation failed: {error}"))?;
    if std::env::var("QALCULATE_REPORT_FALLBACK").as_deref() == Ok("1") {
        if let Some(result) = result.as_ref() {
            eprintln!("[qalc-rs-metadata] {}", result.fallback_state.marker());
        }
    }
    Ok(result.map(|result| cli::repl::ReplEvaluation {
        output: result.output,
        answer_rendering: None,
        assignment_renderings: Vec::new(),
        function_info: None,
    }))
}

fn evaluation_settings(invocation: &cli::CliInvocation, fallback_disabled: bool) -> Vec<String> {
    let unicode_setting = invocation.unicode.and_then(|enabled| {
        (!enabled || fallback_disabled).then(|| format!("unicode {}", i32::from(enabled)))
    });
    let programming_setting = invocation
        .programming_mode
        .then(|| "programming mode 1".to_string());

    let mut settings = Vec::with_capacity(
        invocation.settings.len()
            + usize::from(unicode_setting.is_some())
            + usize::from(programming_setting.is_some()),
    );
    if let Some(setting) = unicode_setting {
        settings.push(setting);
    }
    settings.extend(invocation.settings.iter().cloned());
    settings.extend(
        invocation
            .interactive_settings
            .iter()
            .map(cli::commands::serialize_setting),
    );
    if let Some(setting) = programming_setting {
        settings.push(setting);
    }
    settings
}

fn prepare_calculator(invocation: &cli::CliInvocation) -> Result<Calculator, String> {
    let fallback_disabled = std::env::var("QALCULATE_DISABLE_FALLBACK").as_deref() == Ok("1");
    let defs = &invocation.definitions;
    let mut calc = Calculator::new();
    if defs.global_defs && defs.currencies && !fallback_disabled && !calc.load_exchange_rates() {
        return Err("failed to load exchange rates".to_owned());
    }
    if defs.global_defs
        && !fallback_disabled
        && !calc.load_global_definitions_selected(
            defs.units,
            defs.currencies,
            defs.functions,
            defs.variables,
            defs.datasets,
        )
    {
        return Err("failed to load global definitions".to_owned());
    }
    Ok(calc)
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

fn run_test_file(
    invocation: &mut cli::CliInvocation,
    calculator: &mut Calculator,
    session: &mut cli::repl::ReplSessionState,
    command_file: &cli::CommandFile,
) -> i32 {
    let input = match std::fs::read_to_string(&command_file.path) {
        Ok(input) => input,
        Err(_) => {
            println!("Could not open \"{}\".", command_file.path);
            return 1;
        }
    };

    let items = match libqalculate_rust::batch::parse_batch_items(&input) {
        Ok(items) => items,
        Err(err) => {
            eprintln!("error: {err}");
            return 1;
        }
    };

    let mut ntests = 0;

    for item in items {
        match item {
            libqalculate_rust::batch::BatchItem::Command { command, .. } => match command {
                SessionCommand::Set(cmd) => {
                    invocation
                        .interactive_settings
                        .push(SessionCommand::Set(cmd));
                }
                SessionCommand::Assume(cmd) => {
                    invocation
                        .interactive_settings
                        .push(SessionCommand::Assume(cmd));
                }
            },
            libqalculate_rust::batch::BatchItem::Unasserted { expression, .. } => {
                let _ = evaluate_batch_setup(invocation, calculator, session, &expression);
            }
            libqalculate_rust::batch::BatchItem::Case(case) => {
                let output = match evaluate_batch_expression(
                    invocation,
                    calculator,
                    session,
                    &case.case.expression,
                ) {
                    Ok(Some(eval)) => eval.output,
                    Ok(None) => String::new(),
                    Err(err) => err,
                };

                let expected_text = case.case.expected.join("\n");

                if output != expected_text {
                    print!("\x1b[31m\nMismatch detected at line {}\n{}\nexpected '{}'\nreceived '{}'\n\n\x1b[0m",
                        batch_mismatch_line(&case, &output),
                        case.case.expression,
                        expected_text,
                        output
                    );
                    return 1;
                }
                ntests += 1;
            }
        }
    }

    if ntests == 0 {
        print!(
            "\x1b[31m\nWARNING: 0 tests were run (indentation needs to be tab-based)\n\n\x1b[0m"
        );
    } else {
        print!(
            "\x1b[32m\n{} - {} tests passed\n\n\x1b[0m",
            command_file.path, ntests
        );
    }

    0
}

fn batch_mismatch_line(case: &libqalculate_rust::batch::LocatedBatchCase, output: &str) -> usize {
    let actual = output.split('\n').collect::<Vec<_>>();
    let paired = case.case.expected.len().min(actual.len());
    let mismatch_offset = case
        .case
        .expected
        .iter()
        .zip(&actual)
        .position(|(expected, actual)| expected != actual)
        .unwrap_or(paired)
        .min(case.case.expected.len().saturating_sub(1));
    case.source_line + mismatch_offset + 1
}

fn evaluate_batch_expression(
    invocation: &cli::CliInvocation,
    calculator: &mut Calculator,
    session: &mut cli::repl::ReplSessionState,
    expression: &str,
) -> Result<Option<cli::repl::ReplEvaluation>, String> {
    let result = evaluate_repl_request(
        invocation,
        calculator,
        cli::repl::ReplRequest::Evaluate(expression.to_string()),
    )?;
    if let Some(evaluation) = result.as_ref() {
        session.record_evaluation(expression.to_string(), &evaluation.assignment_renderings);
    }
    Ok(result)
}

fn evaluate_batch_setup(
    invocation: &mut cli::CliInvocation,
    calculator: &mut Calculator,
    session: &mut cli::repl::ReplSessionState,
    expression: &str,
) -> Result<(), String> {
    use cli::commands::InteractiveCommand;

    match cli::commands::parse_interactive_command(expression)? {
        InteractiveCommand::Settings(settings) => {
            invocation.interactive_settings.extend(settings);
            Ok(())
        }
        InteractiveCommand::DefineVariable { name, expression } => {
            evaluate_repl_request(
                invocation,
                calculator,
                cli::repl::ReplRequest::DefineVariable { name, expression },
            )?;
            Ok(())
        }
        InteractiveCommand::DefineFunction { name, expression } => {
            evaluate_repl_request(
                invocation,
                calculator,
                cli::repl::ReplRequest::DefineFunction { name, expression },
            )?;
            Ok(())
        }
        InteractiveCommand::Delete(name) => {
            if evaluate_repl_request(
                invocation,
                calculator,
                cli::repl::ReplRequest::DeleteVariable {
                    name: name.clone(),
                    allow_managed_alias: false,
                },
            )
            .is_err()
            {
                evaluate_repl_request(
                    invocation,
                    calculator,
                    cli::repl::ReplRequest::DeleteFunction(name),
                )?;
            }
            Ok(())
        }
        InteractiveCommand::Expression(expression) => {
            evaluate_batch_expression(invocation, calculator, session, &expression)?;
            Ok(())
        }
        InteractiveCommand::Noop => Ok(()),
        _ => {
            evaluate_batch_expression(invocation, calculator, session, expression)?;
            Ok(())
        }
    }
}
