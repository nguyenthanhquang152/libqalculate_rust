#![forbid(unsafe_code)]

use std::env;
use std::path::{Path, PathBuf};

use libqalculate_rust::batch::read_batch_cases;
use libqalculate_rust::ffi::Calculator;
use libqalculate_rust::UPSTREAM_LIBQALCULATE_VERSION;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OutputMode {
    Text,
    Latex,
    Html,
}

#[derive(Debug, PartialEq, Eq)]
struct LeadingModes {
    output_mode: OutputMode,
    settings: Vec<String>,
}

fn main() {
    let mut raw_args = env::args().skip(1).collect::<Vec<_>>();
    let leading_modes = take_leading_modes(&mut raw_args);
    let mut args = raw_args.into_iter();
    let result = match args.next().as_deref() {
        Some("--version") | Some("-V") => {
            println!(
                "qalc-rs {} (upstream libqalculate {})",
                env!("CARGO_PKG_VERSION"),
                UPSTREAM_LIBQALCULATE_VERSION
            );
            Ok(())
        }
        Some("--self-check") => self_check(),
        Some("--list-upstream-tests") => list_upstream_tests(),
        Some("--parse-batch") => match args.next() {
            Some(path) => parse_batch(Path::new(&path)),
            None => Err("--parse-batch requires a file path".to_owned()),
        },
        Some("-set") => evaluate_expression_with_leading_setting(
            args,
            leading_modes.output_mode,
            leading_modes.settings,
        ),
        Some("--help") | Some("-h") | None => {
            print_help();
            Ok(())
        }
        Some("--") => match args.next() {
            Some(expression) => evaluate_expression_with_settings(
                join_expression(expression, args),
                leading_modes.settings,
                leading_modes.output_mode,
            ),
            None => Err("-- requires an expression".to_owned()),
        },
        Some(other) if other.starts_with("--") => Err(format!("unknown argument: {other}")),
        Some(expression) => evaluate_expression_with_settings(
            join_expression(expression.to_owned(), args),
            leading_modes.settings,
            leading_modes.output_mode,
        ),
    };

    if let Err(error) = result {
        exit_with_error(&error);
    }
}

fn print_help() {
    println!("qalc-rs quality scaffold");
    println!("  --version              Print scaffold and upstream versions");
    println!("  --self-check           Verify upstream fixture inventory is readable");
    println!("  --list-upstream-tests  List upstream .batch fixtures");
    println!("  --parse-batch <path>   Parse a libqalculate .batch fixture");
    println!("  -set <setting>         Limited native-evidence qalc setting support");
    println!("  -u8                   Enable Unicode output signs");
    println!("  +u8                   Disable Unicode output signs");
    println!("  --latex                Format expression output as LaTeX markup");
    println!("  --html                 Format expression output as HTML markup");
    println!("  <expression>           Evaluate through the C++ fallback bridge");
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

fn join_expression(first: String, rest: impl Iterator<Item = String>) -> String {
    let mut expression = first;
    for part in rest {
        expression.push(' ');
        expression.push_str(&part);
    }
    expression
}

fn take_leading_modes(args: &mut Vec<String>) -> LeadingModes {
    let mut output_mode = OutputMode::Text;
    let mut settings = Vec::new();
    loop {
        let Some(first) = args.first() else {
            return LeadingModes {
                output_mode,
                settings,
            };
        };
        match first.as_str() {
            "--latex" | "-latex" => {
                output_mode = OutputMode::Latex;
                args.remove(0);
            }
            "--html" | "-html" => {
                output_mode = OutputMode::Html;
                args.remove(0);
            }
            "-u8" => {
                settings.push("unicode 1".to_string());
                args.remove(0);
            }
            "+u8" => {
                settings.push("unicode 0".to_string());
                args.remove(0);
            }
            _ => {
                return LeadingModes {
                    output_mode,
                    settings,
                };
            }
        }
    }
}

fn evaluate_expression_with_leading_setting(
    mut args: impl Iterator<Item = String>,
    output_mode: OutputMode,
    mut settings: Vec<String>,
) -> Result<(), String> {
    loop {
        let Some(setting) = args.next() else {
            return Err("-set requires a setting".to_owned());
        };
        settings.push(setting);

        match args.next() {
            Some(flag) if flag == "-set" => continue,
            Some(separator) if separator == "--" => {
                let Some(expression) = args.next() else {
                    return Err("-- requires an expression".to_owned());
                };
                return evaluate_expression_with_settings(
                    join_expression(expression, args),
                    settings,
                    output_mode,
                );
            }
            Some(expression) => {
                return evaluate_expression_with_settings(
                    join_expression(expression, args),
                    settings,
                    output_mode,
                );
            }
            None => return Err("-set requires an expression".to_owned()),
        }
    }
}

fn evaluate_expression_with_settings(
    expression: String,
    settings: Vec<String>,
    output_mode: OutputMode,
) -> Result<(), String> {
    let fallback_disabled = std::env::var("QALCULATE_DISABLE_FALLBACK").as_deref() == Ok("1");
    let report_fallback = std::env::var("QALCULATE_REPORT_FALLBACK").as_deref() == Ok("1");

    if output_mode == OutputMode::Text && !fallback_disabled && !settings.is_empty() {
        return Err("session settings require QALCULATE_DISABLE_FALLBACK=1".to_owned());
    }

    let mut calc = Calculator::new();
    if !fallback_disabled && !calc.load_global_definitions() {
        return Err("failed to load global definitions".to_owned());
    }

    let setting_refs = settings.iter().map(String::as_str).collect::<Vec<_>>();
    let result = match output_mode {
        OutputMode::Text if setting_refs.is_empty() => {
            calc.calculate_and_print_qalc_with_fallback_state(&expression, 1000)
        }
        OutputMode::Text => calc.calculate_and_print_qalc_with_settings_and_fallback_state(
            &expression,
            &setting_refs,
            1000,
        ),
        OutputMode::Latex => calc.calculate_and_print_qalc_latex_with_settings_and_fallback_state(
            &expression,
            &setting_refs,
            1000,
        ),
        OutputMode::Html => calc.calculate_and_print_qalc_html_with_settings_and_fallback_state(
            &expression,
            &setting_refs,
            1000,
        ),
    };

    match result {
        Ok(result) => {
            let native_message_had_error = calc.last_native_message_had_error();
            if report_fallback {
                eprintln!("[qalc-rs-metadata] {}", result.fallback_state.marker());
            }
            println!("{}", result.output);
            if native_message_had_error {
                std::process::exit(1);
            }
            Ok(())
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
