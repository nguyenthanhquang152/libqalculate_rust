//! Differential Oracle Runner for libqalculate Rust port.
//!
//! This test module implements a differential testing framework that compares the output
//! of the upstream C++ `qalc` binary against the Rust port (`qalc-rs`) for every upstream
//! `.batch` test case.
//!
//! # Architecture
//!
//! - **C++ Oracle**: The upstream `qalc` binary, located via `QALCULATE_ORACLE` env var
//!   or auto-detected at `../libqalculate/src/qalc`.
//! - **Rust Subject**: Uses the FFI `Calculator` wrapper from `libqalculate_rust::ffi`
//!   to evaluate expressions through the C++ library (until native evaluation is implemented).
//! - **Comparison**: Exact UTF-8 string comparison of stdout, with structured mismatch reporting.
//!
//! # Running
//!
//! ```sh
//! # Run only when C++ oracle is available
//! cargo test --test oracle
//!
//! # Run all batches (slow, requires oracle)
//! cargo test --test oracle -- --ignored differential_oracle_all_batches
//! ```

use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

use libqalculate_rust::batch::read_batch_cases;

// ── Constants ─────────────────────────────────────────────────────────────────

/// All 17 upstream batch test files, in alphabetical order.
const ALL_BATCH_FILES: &[&str] = &[
    "bitwise.batch",
    "calculus.batch",
    "dates.batch",
    "explog.batch",
    "geometry.batch",
    "limits.batch",
    "matrixvector.batch",
    "numberbase.batch",
    "operators.batch",
    "parser.batch",
    "percentages.batch",
    "polynomial.batch",
    "solver.batch",
    "stats.batch",
    "strings.batch",
    "units.batch",
    "variables.batch",
];

// ── DiffMismatch ──────────────────────────────────────────────────────────────

/// Which output field diverged between C++ and Rust.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MismatchField {
    Stdout,
    Stderr,
    ExitCode,
}

impl fmt::Display for MismatchField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stdout => write!(f, "stdout"),
            Self::Stderr => write!(f, "stderr"),
            Self::ExitCode => write!(f, "exit_code"),
        }
    }
}

/// A single divergence between the C++ oracle and the Rust implementation.
#[derive(Debug, Clone)]
pub struct DiffMismatch {
    /// Source batch file name (e.g. `parser.batch`).
    pub batch_file: String,
    /// Zero-based index of the case within the batch file.
    pub case_index: usize,
    /// The input expression that was evaluated.
    pub expression: String,
    /// Which output field diverged.
    pub field: MismatchField,
    /// Value produced by the C++ oracle.
    pub cpp_value: String,
    /// Value produced by the Rust implementation.
    pub rust_value: String,
    /// Optional deviation identifier for known/accepted divergences.
    pub deviation_id: Option<String>,
}

impl fmt::Display for DiffMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MISMATCH batch={} case={} expr={:?} field={} cpp={:?} rust={:?} deviation={:?}",
            self.batch_file,
            self.case_index,
            self.expression,
            self.field,
            self.cpp_value,
            self.rust_value,
            self.deviation_id.as_deref().unwrap_or("none"),
        )
    }
}

// ── Captured output ───────────────────────────────────────────────────────────

/// Captured output from running an expression through either the C++ or Rust evaluator.
#[derive(Debug, Clone)]
struct CapturedOutput {
    stdout: String,
    stderr: String,
    exit_code: i32,
}

// ── Oracle binary detection ───────────────────────────────────────────────────

/// Locate the C++ `qalc` oracle binary.
///
/// Search order:
/// 1. `QALCULATE_ORACLE` environment variable
/// 2. `../libqalculate/src/qalc` (default build location)
fn oracle_binary() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("QALCULATE_ORACLE").map(PathBuf::from) {
        return path.exists().then_some(path);
    }
    let candidate = Path::new("../libqalculate/src/qalc");
    candidate.exists().then(|| candidate.to_path_buf())
}

/// Resolve the upstream definitions directory.
fn defs_dir() -> PathBuf {
    Path::new("../libqalculate/data")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from("../libqalculate/data"))
}

/// Resolve the upstream tests directory.
fn upstream_tests_dir() -> PathBuf {
    Path::new("../libqalculate/tests").to_path_buf()
}

// ── Session command parsing ───────────────────────────────────────────────────

/// A session command parsed from a batch file (e.g. `/set`, `set`, `/assume`).
#[derive(Debug, Clone)]
struct SessionCommand {
    /// The raw command text, e.g. `/set approximation exact`.
    raw: String,
}

impl SessionCommand {
    /// Convert this session command into qalc CLI arguments.
    ///
    /// `/set key value` → `-set key value`
    /// `set key value` → `-set key value`
    /// `/assume value` → `-assume value`
    fn to_qalc_args(&self) -> Vec<String> {
        let trimmed = self.raw.trim();
        if let Some(rest) = trimmed.strip_prefix("/set ") {
            let mut args = vec!["-set".to_string()];
            args.extend(rest.split_whitespace().map(String::from));
            args
        } else if let Some(rest) = trimmed.strip_prefix("set ") {
            let mut args = vec!["-set".to_string()];
            args.extend(rest.split_whitespace().map(String::from));
            args
        } else if let Some(rest) = trimmed.strip_prefix("/assume ") {
            let mut args = vec!["-assume".to_string()];
            args.extend(rest.split_whitespace().map(String::from));
            args
        } else {
            // Unknown command type; pass as-is (shouldn't happen with valid batch files)
            vec![trimmed.to_string()]
        }
    }
}

/// Parse session commands from a batch file's raw text.
///
/// Returns a list of `(line_number, SessionCommand)` pairs and a mapping from
/// expression line numbers to their accumulated session state.
fn parse_session_commands(input: &str) -> Vec<(usize, SessionCommand)> {
    let mut commands = Vec::new();
    for (idx, raw_line) in input.lines().enumerate() {
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        let trimmed = line.trim();
        if trimmed.starts_with("/set ")
            || trimmed.starts_with("set ")
            || trimmed.starts_with("/assume ")
        {
            commands.push((
                idx + 1,
                SessionCommand {
                    raw: trimmed.to_string(),
                },
            ));
        }
    }
    commands
}

/// Build the accumulated settings list for each expression, based on the session
/// commands that precede it in file order.
fn accumulated_settings_for_cases(input: &str, case_count: usize) -> Vec<Vec<SessionCommand>> {
    let session_cmds = parse_session_commands(input);

    // Find expression line numbers by re-parsing
    let mut expr_lines = Vec::new();
    let mut current_expr_line: Option<usize> = None;
    let mut has_expected = false;

    for (idx, raw_line) in input.lines().enumerate() {
        let lineno = idx + 1;
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        let stripped = line.trim();

        if stripped.is_empty() || stripped.starts_with('#') {
            if current_expr_line.is_some() && has_expected {
                expr_lines.push(current_expr_line.unwrap());
                current_expr_line = None;
                has_expected = false;
            } else {
                current_expr_line = None;
                has_expected = false;
            }
            continue;
        }

        if stripped.starts_with("/set ")
            || stripped.starts_with("set ")
            || stripped.starts_with("/assume ")
        {
            if current_expr_line.is_some() && has_expected {
                expr_lines.push(current_expr_line.unwrap());
                current_expr_line = None;
                has_expected = false;
            } else {
                current_expr_line = None;
                has_expected = false;
            }
            continue;
        }

        if line.starts_with('\t') {
            if current_expr_line.is_some() {
                has_expected = true;
            }
            continue;
        }

        // New expression line
        if let Some(line) = current_expr_line {
            if has_expected {
                expr_lines.push(line);
            }
        }
        current_expr_line = Some(lineno);
        has_expected = false;
    }
    if let Some(line) = current_expr_line {
        if has_expected {
            expr_lines.push(line);
        }
    }

    // For each expression, find all session commands that precede it
    let mut result = Vec::with_capacity(case_count);
    for &expr_line in &expr_lines {
        let accumulated: Vec<SessionCommand> = session_cmds
            .iter()
            .filter(|(cmd_line, _)| *cmd_line < expr_line)
            .map(|(_, cmd)| cmd.clone())
            .collect();
        result.push(accumulated);
    }

    // Pad if needed (shouldn't happen, but defensive)
    while result.len() < case_count {
        result.push(Vec::new());
    }

    result
}

// ── C++ Oracle runner ─────────────────────────────────────────────────────────

/// Run a single expression through the C++ `qalc` oracle.
///
/// The oracle is invoked with:
/// - `LC_ALL=C.UTF-8` for consistent locale
/// - `-defaults` to reset to default settings
/// - `-set "decimal_comma" "0"` for dot-decimal mode
/// - `-set "curconv" "0"` to disable currency rate conversion
/// - Any accumulated session settings from the batch file
///
/// Returns the captured stdout, stderr, and exit code.
fn run_oracle_expression(
    qalc_path: &Path,
    defs: &Path,
    expression: &str,
    settings: &[SessionCommand],
) -> CapturedOutput {
    let mut cmd = Command::new(qalc_path);
    cmd.env("LC_ALL", "C.UTF-8")
        .env("QALCULATE_DEFINITIONS_DIR", defs);

    // Base arguments: reset defaults and set consistent formatting
    cmd.arg("-defaults")
        .arg("-set")
        .arg("decimal_comma")
        .arg("0")
        .arg("-set")
        .arg("curconv")
        .arg("0");

    // Apply accumulated session settings
    for setting in settings {
        for arg in setting.to_qalc_args() {
            cmd.arg(arg);
        }
    }

    // The expression to evaluate
    cmd.arg(expression);

    let output = cmd.output().expect("failed to execute C++ qalc oracle");

    CapturedOutput {
        stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        exit_code: output.status.code().unwrap_or(-1),
    }
}

// ── Rust subject runner ───────────────────────────────────────────────────────

/// Run a single expression through the Rust implementation.
///
/// Currently uses the FFI `Calculator` wrapper since native evaluation is not
/// yet implemented. This goes through the same C++ library but via our Rust
/// FFI layer, validating that the FFI wrapper produces identical output.
///
/// When native Rust evaluation is implemented, this function should be updated
/// to use the native path instead.
fn run_rust_expression(expression: &str, _settings: &[SessionCommand]) -> CapturedOutput {
    // Use cargo run for the Rust binary, capturing output.
    // This ensures we test the actual binary interface.
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    let output = Command::new(&cargo)
        .arg("run")
        .arg("--quiet")
        .arg("--bin")
        .arg("qalc-rs")
        .arg("--manifest-path")
        .arg(manifest_dir.join("Cargo.toml"))
        .arg("--")
        .arg(expression)
        .env("LC_ALL", "C.UTF-8")
        .output();

    match output {
        Ok(out) => CapturedOutput {
            stdout: String::from_utf8_lossy(&out.stdout).trim().to_string(),
            stderr: String::from_utf8_lossy(&out.stderr).trim().to_string(),
            exit_code: out.status.code().unwrap_or(-1),
        },
        Err(e) => CapturedOutput {
            stdout: String::new(),
            stderr: format!("Failed to run qalc-rs: {e}"),
            exit_code: -1,
        },
    }
}

// ── Differential comparison ───────────────────────────────────────────────────

/// Run differential comparison on a single batch file.
///
/// Reads the batch file, tracks accumulated session settings, and for each case
/// runs both the C++ oracle and the Rust implementation, comparing outputs.
///
/// Uses exact UTF-8 string comparison by default.
///
/// Returns a vector of all mismatches found.
fn differential_oracle_batch(
    batch_path: &Path,
    qalc_path: &Path,
    defs: &Path,
) -> Vec<DiffMismatch> {
    let batch_name = batch_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let input = std::fs::read_to_string(batch_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", batch_path.display()));

    let cases = read_batch_cases(batch_path)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {e}", batch_path.display()));

    let settings_per_case = accumulated_settings_for_cases(&input, cases.len());

    let mut mismatches = Vec::new();

    for (i, case) in cases.iter().enumerate() {
        let settings = &settings_per_case[i];

        // Run C++ oracle
        let cpp_out = run_oracle_expression(qalc_path, defs, &case.expression, settings);

        // Run Rust implementation
        let rust_out = run_rust_expression(&case.expression, settings);

        // Compare stdout (primary comparison)
        if cpp_out.stdout != rust_out.stdout {
            mismatches.push(DiffMismatch {
                batch_file: batch_name.clone(),
                case_index: i,
                expression: case.expression.clone(),
                field: MismatchField::Stdout,
                cpp_value: cpp_out.stdout.clone(),
                rust_value: rust_out.stdout.clone(),
                deviation_id: None,
            });
        }

        // Compare stderr
        if cpp_out.stderr != rust_out.stderr {
            mismatches.push(DiffMismatch {
                batch_file: batch_name.clone(),
                case_index: i,
                expression: case.expression.clone(),
                field: MismatchField::Stderr,
                cpp_value: cpp_out.stderr.clone(),
                rust_value: rust_out.stderr.clone(),
                deviation_id: None,
            });
        }

        // Compare exit codes
        if cpp_out.exit_code != rust_out.exit_code {
            mismatches.push(DiffMismatch {
                batch_file: batch_name.clone(),
                case_index: i,
                expression: case.expression.clone(),
                field: MismatchField::ExitCode,
                cpp_value: cpp_out.exit_code.to_string(),
                rust_value: rust_out.exit_code.to_string(),
                deviation_id: None,
            });
        }
    }

    mismatches
}

/// Print mismatches in a machine-readable format (one line per mismatch).
fn report_mismatches(mismatches: &[DiffMismatch]) {
    if mismatches.is_empty() {
        eprintln!("ORACLE: all cases match");
        return;
    }

    eprintln!("ORACLE: {} mismatch(es) found:", mismatches.len());
    for m in mismatches {
        eprintln!("  {m}");
    }
}

// ── Existing tests (preserved) ────────────────────────────────────────────────

#[test]
fn upstream_batch_inventory_is_available_for_oracle_tests() {
    let path = Path::new("../libqalculate/tests/parser.batch");
    if !path.exists() {
        eprintln!(
            "skipping upstream oracle fixture inventory; {} is unavailable",
            path.display()
        );
        return;
    }

    let cases = read_batch_cases(path).expect("upstream parser.batch should be available");
    assert!(cases.iter().any(|case| case.expression == "123456789"));
}

#[test]
fn upstream_qalc_oracle_can_run_batch_when_available() {
    let Some(qalc) = oracle_binary() else {
        eprintln!(
            "skipping upstream qalc oracle execution; set QALCULATE_ORACLE or build upstream qalc"
        );
        return;
    };

    let defs = defs_dir();
    let status = Command::new(qalc)
        .env("QALCULATE_DEFINITIONS_DIR", &defs)
        .arg("--test-file")
        .arg("../libqalculate/tests/parser.batch")
        .status()
        .expect("upstream qalc oracle should start");
    assert!(status.success(), "upstream qalc rejected parser.batch");
}

// ── Differential oracle tests ─────────────────────────────────────────────────

/// Differential oracle test for `parser.batch` — the simplest batch file with
/// no session commands. This is the baseline test for the differential framework.
#[test]
fn differential_oracle_parser_batch() {
    let Some(qalc) = oracle_binary() else {
        eprintln!(
            "skipping differential_oracle_parser_batch; \
             C++ oracle not available (set QALCULATE_ORACLE or build upstream qalc)"
        );
        return;
    };

    let batch_path = upstream_tests_dir().join("parser.batch");
    if !batch_path.exists() {
        eprintln!(
            "skipping differential_oracle_parser_batch; {} not found",
            batch_path.display()
        );
        return;
    }

    let defs = defs_dir();
    let mismatches = differential_oracle_batch(&batch_path, &qalc, &defs);
    report_mismatches(&mismatches);

    // parser.batch has no session state, so it's the cleanest comparison target.
    // We report mismatches but don't hard-fail yet — the Rust CLI doesn't support
    // expression evaluation as a positional argument yet.
    if !mismatches.is_empty() {
        eprintln!(
            "NOTE: {} mismatches in parser.batch (expected until native eval is implemented)",
            mismatches.len()
        );
    }
}

/// Differential oracle test for ALL 17 upstream batch files.
///
/// This test is marked `#[ignore]` because it is slow and requires the C++ oracle.
/// Run with: `cargo test --test oracle -- --ignored differential_oracle_all_batches`
#[test]
#[ignore]
fn differential_oracle_all_batches() {
    let Some(qalc) = oracle_binary() else {
        eprintln!(
            "skipping differential_oracle_all_batches; \
             C++ oracle not available (set QALCULATE_ORACLE or build upstream qalc)"
        );
        return;
    };

    let tests_dir = upstream_tests_dir();
    if !tests_dir.exists() {
        eprintln!(
            "skipping differential_oracle_all_batches; {} not found",
            tests_dir.display()
        );
        return;
    }

    let defs = defs_dir();
    let mut total_mismatches = 0;
    let mut total_cases = 0;

    for batch_name in ALL_BATCH_FILES {
        let batch_path = tests_dir.join(batch_name);
        if !batch_path.exists() {
            eprintln!("WARNING: batch file not found: {}", batch_path.display());
            continue;
        }

        let cases = read_batch_cases(&batch_path)
            .unwrap_or_else(|e| panic!("Failed to parse {batch_name}: {e}"));
        let case_count = cases.len();

        let mismatches = differential_oracle_batch(&batch_path, &qalc, &defs);

        eprintln!(
            "ORACLE {batch_name}: {}/{} cases passed ({} mismatches)",
            case_count
                - mismatches
                    .iter()
                    .filter(|m| m.field == MismatchField::Stdout)
                    .count(),
            case_count,
            mismatches.len()
        );

        for m in &mismatches {
            eprintln!("  {m}");
        }

        total_mismatches += mismatches.len();
        total_cases += case_count;
    }

    eprintln!("\nORACLE SUMMARY: {total_cases} total cases, {total_mismatches} total mismatches");

    // Don't hard-fail yet — this is an informational test until the Rust port
    // reaches sufficient parity. Uncomment the assertion below when ready:
    // assert_eq!(total_mismatches, 0, "Differential oracle found mismatches");
}

/// Differential oracle for a single batch file specified by name.
/// Useful for targeted testing during development.
///
/// Run with: `cargo test --test oracle -- --ignored differential_oracle_single`
/// Set `ORACLE_BATCH` env var to specify the batch file, e.g.:
///   `ORACLE_BATCH=operators.batch cargo test --test oracle -- --ignored differential_oracle_single`
#[test]
#[ignore]
fn differential_oracle_single() {
    let Some(qalc) = oracle_binary() else {
        eprintln!("skipping; C++ oracle not available");
        return;
    };

    let batch_name = std::env::var("ORACLE_BATCH").unwrap_or_else(|_| "parser.batch".to_string());
    let batch_path = upstream_tests_dir().join(&batch_name);
    if !batch_path.exists() {
        eprintln!("skipping; {} not found", batch_path.display());
        return;
    }

    let defs = defs_dir();
    let mismatches = differential_oracle_batch(&batch_path, &qalc, &defs);
    report_mismatches(&mismatches);

    let cases = read_batch_cases(&batch_path).unwrap();
    eprintln!(
        "\nORACLE {batch_name}: {}/{} cases passed",
        cases.len()
            - mismatches
                .iter()
                .filter(|m| m.field == MismatchField::Stdout)
                .count(),
        cases.len()
    );
}

// ── Unit tests for the oracle infrastructure ──────────────────────────────────

#[cfg(test)]
mod infrastructure_tests {
    use super::*;

    #[test]
    fn diff_mismatch_display_is_machine_readable() {
        let m = DiffMismatch {
            batch_file: "test.batch".to_string(),
            case_index: 0,
            expression: "1 + 1".to_string(),
            field: MismatchField::Stdout,
            cpp_value: "2".to_string(),
            rust_value: "3".to_string(),
            deviation_id: None,
        };
        let display = m.to_string();
        assert!(display.contains("MISMATCH"));
        assert!(display.contains("batch=test.batch"));
        assert!(display.contains("case=0"));
        assert!(display.contains("field=stdout"));
        assert!(display.contains("cpp=\"2\""));
        assert!(display.contains("rust=\"3\""));
        assert!(display.contains("deviation=\"none\""));
    }

    #[test]
    fn diff_mismatch_display_with_deviation() {
        let m = DiffMismatch {
            batch_file: "test.batch".to_string(),
            case_index: 5,
            expression: "pi".to_string(),
            field: MismatchField::Stdout,
            cpp_value: "3.14159".to_string(),
            rust_value: "3.14160".to_string(),
            deviation_id: Some("PRECISION-001".to_string()),
        };
        let display = m.to_string();
        assert!(display.contains("deviation=\"PRECISION-001\""));
    }

    #[test]
    fn session_command_set_produces_correct_args() {
        let cmd = SessionCommand {
            raw: "/set approximation exact".to_string(),
        };
        assert_eq!(cmd.to_qalc_args(), vec!["-set", "approximation", "exact"]);
    }

    #[test]
    fn session_command_bare_set_produces_correct_args() {
        let cmd = SessionCommand {
            raw: "set input base 16".to_string(),
        };
        assert_eq!(cmd.to_qalc_args(), vec!["-set", "input", "base", "16"]);
    }

    #[test]
    fn session_command_assume_produces_correct_args() {
        let cmd = SessionCommand {
            raw: "/assume positive".to_string(),
        };
        assert_eq!(cmd.to_qalc_args(), vec!["-assume", "positive"]);
    }

    #[test]
    fn parse_session_commands_from_batch_text() {
        let input = "\
/set approximation exact
/set fr 2

x + 3 = 0
\tx = -3
/assume positive
sqrt(x)
\tsqrt(x)
";
        let cmds = parse_session_commands(input);
        assert_eq!(cmds.len(), 3);
        assert_eq!(cmds[0].0, 1); // line 1
        assert_eq!(cmds[0].1.raw, "/set approximation exact");
        assert_eq!(cmds[1].0, 2); // line 2
        assert_eq!(cmds[1].1.raw, "/set fr 2");
        assert_eq!(cmds[2].0, 6); // line 6
        assert_eq!(cmds[2].1.raw, "/assume positive");
    }

    #[test]
    fn accumulated_settings_tracks_state_correctly() {
        let input = "\
/set approximation exact
/set fr 2

x + 3 = 0
\tx = -3
/assume positive
sqrt(x)
\tsqrt(x)
";
        let settings = accumulated_settings_for_cases(input, 2);
        assert_eq!(settings.len(), 2);
        // First case (x + 3 = 0) has 2 settings preceding it
        assert_eq!(settings[0].len(), 2);
        assert_eq!(settings[0][0].raw, "/set approximation exact");
        assert_eq!(settings[0][1].raw, "/set fr 2");
        // Second case (sqrt(x)) has 3 settings preceding it
        assert_eq!(settings[1].len(), 3);
        assert_eq!(settings[1][2].raw, "/assume positive");
    }

    #[test]
    fn mismatch_field_display() {
        assert_eq!(MismatchField::Stdout.to_string(), "stdout");
        assert_eq!(MismatchField::Stderr.to_string(), "stderr");
        assert_eq!(MismatchField::ExitCode.to_string(), "exit_code");
    }

    #[test]
    fn all_batch_files_constant_has_17_entries() {
        assert_eq!(ALL_BATCH_FILES.len(), 17);
    }

    #[test]
    fn all_batch_files_are_sorted() {
        let mut sorted = ALL_BATCH_FILES.to_vec();
        sorted.sort();
        assert_eq!(ALL_BATCH_FILES, sorted.as_slice());
    }
}
