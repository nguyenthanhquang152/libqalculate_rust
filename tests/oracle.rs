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
//! - **Rust Subject**: Runs the `qalc-rs` binary. Inventory rows use the
//!   qalc-compatible C++ FFI fallback; rows promoted to `native-pass` run with
//!   the fallback disabled and must report `fallback=native`.
//! - **Comparison**: Exact UTF-8 string comparison of stdout, with structured mismatch reporting.
//!
//! The strict default gate started with no-session batches such as
//! `parser.batch`. A small exact-gated numberbase slice now passes accumulated
//! settings through `qalc-rs -set ...` when those rows are promoted to
//! `native-pass`; other session-dependent inventory rows remain unsupported by
//! the Rust subject path.
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

use libqalculate_rust::batch::{batch_case_ids, is_session_command, read_batch_cases};
use libqalculate_rust::ffi::FallbackState;
use libqalculate_rust::parser::lexer::{
    lex_expression, lex_line, BasePrefix, LineKind, NumberLiteralKind, Operator, TokenKind,
};

#[path = "oracle/fallback_gate.rs"]
mod oracle_fallback_gate;
#[path = "oracle/manifest.rs"]
mod oracle_manifest;

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
    FallbackState,
}

impl fmt::Display for MismatchField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stdout => write!(f, "stdout"),
            Self::Stderr => write!(f, "stderr"),
            Self::ExitCode => write!(f, "exit_code"),
            Self::FallbackState => write!(f, "fallback_state"),
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
    /// Output normalization policy used for comparison.
    pub normalization_policy: String,
    /// Whether the Rust subject used native code, C++ fallback, or rejected the case.
    pub fallback_state: String,
    /// Accumulated batch session commands active for this case.
    pub session_commands: Vec<String>,
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
        )?;
        write!(
            f,
            " normalization={} fallback={} session={:?}",
            self.normalization_policy, self.fallback_state, self.session_commands
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
    fallback_state: Option<FallbackState>,
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
    /// `/set key value` → `-set "key value"`
    /// `set key value` → `-set "key value"`
    /// `/assume value` → `-set "assumptions value"`
    fn to_qalc_args(&self) -> Vec<String> {
        let trimmed = self.raw.trim();
        if let Some(rest) = trimmed.strip_prefix("/set ") {
            vec!["-set".to_string(), rest.to_string()]
        } else if let Some(rest) = trimmed.strip_prefix("set ") {
            vec!["-set".to_string(), rest.to_string()]
        } else if let Some(rest) = trimmed.strip_prefix("/assume ") {
            vec!["-set".to_string(), format!("assumptions {rest}")]
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
        if is_session_command(trimmed) {
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

        if is_session_command(stripped) {
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
/// - `-set "decimal_comma 0"` for dot-decimal mode
/// - `-set "curconv 0"` to disable currency rate conversion
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
        .env("TZ", "UTC")
        .env("QALCULATE_DEFINITIONS_DIR", defs);

    // Base arguments: reset defaults and set consistent formatting
    cmd.arg("-defaults")
        .arg("-terse")
        .arg("-set")
        .arg("decimal_comma 0")
        .arg("-set")
        .arg("curconv 0");

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
        fallback_state: None,
    }
}

// ── Rust subject runner ───────────────────────────────────────────────────────

/// Run a single expression through the Rust implementation.
///
/// Inventory rows use the qalc-compatible FFI fallback exposed by the `qalc-rs`
/// binary. Native-pass rows run with fallback disabled through the same CLI and
/// are verified by the fallback-state oracle gate. Accumulated batch session
/// settings are passed through only for fallback-disabled native rows; inventory
/// rows with settings are reported as unsupported rather than silently ignored.
fn run_rust_expression(
    expression: &str,
    settings: &[SessionCommand],
    defs: &Path,
    disable_fallback: bool,
    report_fallback: bool,
) -> CapturedOutput {
    if !settings.is_empty() && !disable_fallback {
        return CapturedOutput {
            stdout: String::new(),
            stderr: format!(
                "qalc-rs fallback oracle does not support session settings yet: {}",
                settings
                    .iter()
                    .map(|setting| setting.raw.as_str())
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
            exit_code: -1,
            fallback_state: None,
        };
    }

    // Use cargo run for the Rust binary, capturing output.
    // This ensures we test the actual binary interface.
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    let mut cmd = Command::new(&cargo);
    cmd.arg("run")
        .arg("--quiet")
        .arg("--bin")
        .arg("qalc-rs")
        .arg("--manifest-path")
        .arg(manifest_dir.join("Cargo.toml"))
        .arg("--")
        .env("LC_ALL", "C.UTF-8")
        .env("TZ", "UTC")
        .env("QALCULATE_DEFINITIONS_DIR", defs)
        .env_remove("QALCULATE_DISABLE_FALLBACK")
        .env_remove("QALCULATE_REPORT_FALLBACK");

    for setting in settings {
        for arg in setting.to_qalc_args() {
            cmd.arg(arg);
        }
    }
    cmd.arg("--").arg(expression);

    if disable_fallback {
        cmd.env("QALCULATE_DISABLE_FALLBACK", "1");
    }
    if report_fallback {
        cmd.env("QALCULATE_REPORT_FALLBACK", "1");
    }

    let output = cmd.output();

    match output {
        Ok(out) => {
            let stderr_str = String::from_utf8_lossy(&out.stderr);
            let mut clean_stderr = Vec::new();
            let mut fallback_state = None;

            for line in stderr_str.lines() {
                if line.starts_with("[qalc-rs-metadata]") {
                    fallback_state = FallbackState::from_marker(line);
                } else {
                    clean_stderr.push(line.to_string());
                }
            }

            CapturedOutput {
                stdout: String::from_utf8_lossy(&out.stdout).trim().to_string(),
                stderr: clean_stderr.join("\n").trim().to_string(),
                exit_code: out.status.code().unwrap_or(-1),
                fallback_state,
            }
        }
        Err(e) => CapturedOutput {
            stdout: String::new(),
            stderr: format!("Failed to run qalc-rs: {e}"),
            exit_code: -1,
            fallback_state: None,
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
    let case_ids = batch_case_ids(&batch_name, &input).unwrap_or_else(|e| {
        panic!(
            "Failed to derive case IDs for {}: {e}",
            batch_path.display()
        )
    });
    let parity_map = oracle_manifest::load_parity_statuses();

    let mut mismatches = Vec::new();

    for (i, case) in cases.iter().enumerate() {
        let settings = &settings_per_case[i];
        let session_commands = settings
            .iter()
            .map(|setting| setting.raw.clone())
            .collect::<Vec<_>>();

        let case_id = case_ids
            .get(i)
            .unwrap_or_else(|| panic!("missing case ID for {} case index {i}", batch_name));
        let parity_status = oracle_manifest::status_for_case(&parity_map, case_id);

        let (disable_fallback, report_fallback) = if parity_status == "native-pass" {
            (true, true)
        } else {
            (false, true)
        };

        // Run C++ oracle
        let cpp_out = run_oracle_expression(qalc_path, defs, &case.expression, settings);

        // Run Rust implementation
        let rust_out = run_rust_expression(
            &case.expression,
            settings,
            defs,
            disable_fallback,
            report_fallback,
        );

        let fallback_state = oracle_fallback_gate::fallback_state_label(
            rust_out.fallback_state,
            !settings.is_empty(),
        );

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
                normalization_policy: "exact-utf8".to_string(),
                fallback_state: fallback_state.clone(),
                session_commands: session_commands.clone(),
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
                normalization_policy: "exact-utf8".to_string(),
                fallback_state: fallback_state.clone(),
                session_commands: session_commands.clone(),
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
                normalization_policy: "exact-utf8".to_string(),
                fallback_state: fallback_state.clone(),
                session_commands: session_commands.clone(),
            });
        }

        if let Some(mismatch) = oracle_fallback_gate::native_pass_fallback_mismatch(
            &batch_name,
            i,
            &case.expression,
            parity_status,
            &fallback_state,
            &session_commands,
        ) {
            mismatches.push(mismatch);
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
fn focused_issue17_lexer_oracle_fixture_cases() {
    let parser_path = Path::new("../libqalculate/tests/parser.batch");
    let strings_path = Path::new("../libqalculate/tests/strings.batch");
    let bitwise_path = Path::new("../libqalculate/tests/bitwise.batch");
    let operators_path = Path::new("../libqalculate/tests/operators.batch");

    for path in [parser_path, strings_path, bitwise_path, operators_path] {
        if !path.exists() {
            eprintln!(
                "skipping focused_issue17_lexer_oracle_fixture_cases; {} is unavailable",
                path.display()
            );
            return;
        }
    }

    let parser = std::fs::read_to_string(parser_path).expect("read parser.batch");
    assert!(parser.contains("123 456 789"));
    assert_eq!(
        lex_expression("123 456 789")
            .expect("lex parser.batch grouped number")
            .into_iter()
            .map(|token| token.kind)
            .collect::<Vec<_>>(),
        vec![TokenKind::Number {
            text: "123 456 789".into(),
            kind: NumberLiteralKind::Integer,
        }]
    );

    let strings = std::fs::read_to_string(strings_path).expect("read strings.batch");
    assert!(strings.contains("/set unicode 1"));
    let slash_command = lex_line("/set unicode 1").expect("lex strings.batch slash command");
    assert_eq!(slash_command.kind, LineKind::Command);
    assert_eq!(slash_command.tokens[0].kind, TokenKind::CommandPrefix);

    let bitwise = std::fs::read_to_string(bitwise_path).expect("read bitwise.batch");
    assert!(bitwise.contains("0b1011 0010 ⊻ 0b0111 0001"));
    assert_eq!(
        lex_expression("0b1011 0010 ⊻ 0b0111 0001")
            .expect("lex bitwise.batch binary xor")
            .into_iter()
            .map(|token| token.kind)
            .collect::<Vec<_>>(),
        vec![
            TokenKind::Number {
                text: "0b1011 0010".into(),
                kind: NumberLiteralKind::BasePrefixed(BasePrefix::Binary),
            },
            TokenKind::Operator(Operator::BitwiseXor),
            TokenKind::Number {
                text: "0b0111 0001".into(),
                kind: NumberLiteralKind::BasePrefixed(BasePrefix::Binary),
            },
        ]
    );

    let operators = std::fs::read_to_string(operators_path).expect("read operators.batch");
    assert!(operators.contains("5//2"));
    assert!(lex_expression("5//2")
        .expect("lex operators.batch integer division")
        .iter()
        .any(|token| token.kind == TokenKind::Operator(Operator::IntegerDivide)));
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

    assert!(
        mismatches.is_empty(),
        "differential oracle parser.batch found {} mismatch(es)",
        mismatches.len()
    );
}

/// Differential oracle test for exact operator cases promoted to `native-pass`.
#[test]
fn differential_oracle_exact_operators_batch() {
    let Some(qalc) = oracle_binary() else {
        eprintln!(
            "skipping differential_oracle_exact_operators_batch; \
             C++ oracle not available (set QALCULATE_ORACLE or build upstream qalc)"
        );
        return;
    };

    let batch_path = upstream_tests_dir().join("operators.batch");
    if !batch_path.exists() {
        eprintln!(
            "skipping differential_oracle_exact_operators_batch; {} not found",
            batch_path.display()
        );
        return;
    }

    let defs = defs_dir();
    let mismatches = differential_oracle_batch(&batch_path, &qalc, &defs);
    report_mismatches(&mismatches);

    assert!(
        mismatches.is_empty(),
        "differential oracle operators.batch found {} mismatch(es)",
        mismatches.len()
    );
}

/// Differential oracle test for `numberbase.batch`, including accumulated
/// input-base and Unicode settings on the final two rows.
#[test]
fn differential_oracle_numberbase_batch() {
    let Some(qalc) = oracle_binary() else {
        eprintln!(
            "skipping differential_oracle_numberbase_batch; \
             C++ oracle not available (set QALCULATE_ORACLE or build upstream qalc)"
        );
        return;
    };

    let batch_path = upstream_tests_dir().join("numberbase.batch");
    if !batch_path.exists() {
        eprintln!(
            "skipping differential_oracle_numberbase_batch; {} not found",
            batch_path.display()
        );
        return;
    }

    let defs = defs_dir();
    let mismatches = differential_oracle_batch(&batch_path, &qalc, &defs);
    report_mismatches(&mismatches);

    assert!(
        mismatches.is_empty(),
        "differential oracle numberbase.batch found {} mismatch(es)",
        mismatches.len()
    );
}

/// Differential oracle test for `geometry.batch`.
#[test]
fn differential_oracle_geometry_batch() {
    let Some(qalc) = oracle_binary() else {
        eprintln!(
            "skipping differential_oracle_geometry_batch; \
             C++ oracle not available (set QALCULATE_ORACLE or build upstream qalc)"
        );
        return;
    };

    let batch_path = upstream_tests_dir().join("geometry.batch");
    if !batch_path.exists() {
        eprintln!(
            "skipping differential_oracle_geometry_batch; {} not found",
            batch_path.display()
        );
        return;
    }

    let defs = defs_dir();
    let mismatches = differential_oracle_batch(&batch_path, &qalc, &defs);
    report_mismatches(&mismatches);

    assert!(
        mismatches.is_empty(),
        "differential oracle geometry.batch found {} mismatch(es)",
        mismatches.len()
    );
}

/// Focused fallback-disabled oracle evidence for no-session `numberbase.batch`
/// rows. The session-setting rows are covered separately below.
#[test]
fn focused_epic2_numberbase_no_session_oracle_cases() {
    let Some(qalc) = oracle_binary() else {
        eprintln!(
            "skipping focused_epic2_numberbase_no_session_oracle_cases; \
             C++ oracle not available (set QALCULATE_ORACLE or build upstream qalc)"
        );
        return;
    };

    let defs = defs_dir();
    let settings = Vec::new();
    let cases = [
        ("numberbase.batch:1", "52 to bin"),
        ("numberbase.batch:3", "52 to bin16"),
        ("numberbase.batch:5", "52 to oct"),
        ("numberbase.batch:7", "52 to hex"),
        ("numberbase.batch:9", "0x34"),
        ("numberbase.batch:11", "hex(34)"),
        ("numberbase.batch:13", "523<<2&250 to bin"),
        ("numberbase.batch:15", "52.345 to float"),
        (
            "numberbase.batch:17",
            "float(01000010010100010110000101001000)",
        ),
        ("numberbase.batch:19", "floatError(52.345)"),
        ("numberbase.batch:21", "1978 to roman"),
        ("numberbase.batch:23", "52 to base 32"),
        ("numberbase.batch:25", "sqrt(32) to base sqrt(2)"),
    ];

    for (case_id, expression) in cases {
        let cpp_out = run_oracle_expression(&qalc, &defs, expression, &settings);
        let rust_out = run_rust_expression(expression, &settings, &defs, true, true);
        let fallback_state =
            oracle_fallback_gate::fallback_state_label(rust_out.fallback_state, false);

        assert_eq!(
            cpp_out.stdout, rust_out.stdout,
            "{case_id} stdout mismatch for {expression:?}; fallback={fallback_state}"
        );
        assert_eq!(
            cpp_out.stderr, rust_out.stderr,
            "{case_id} stderr mismatch for {expression:?}; fallback={fallback_state}"
        );
        assert_eq!(
            cpp_out.exit_code, rust_out.exit_code,
            "{case_id} exit code mismatch for {expression:?}; fallback={fallback_state}"
        );
        assert_eq!(
            fallback_state,
            FallbackState::Native.label(),
            "{case_id} did not run natively"
        );
    }
}

/// Focused fallback-disabled oracle evidence for the remaining
/// `numberbase.batch` rows that depend on accumulated session settings.
#[test]
fn focused_epic2_numberbase_session_oracle_cases() {
    let Some(qalc) = oracle_binary() else {
        eprintln!(
            "skipping focused_epic2_numberbase_session_oracle_cases; \
             C++ oracle not available (set QALCULATE_ORACLE or build upstream qalc)"
        );
        return;
    };

    let defs = defs_dir();
    let cases: [(&str, &str, &[&str]); 2] = [
        (
            "numberbase.batch:28",
            "5p10+AEp-2*p23",
            &["set input base 16"],
        ),
        (
            "numberbase.batch:32",
            "52.34 to sexa",
            &["set input base 16", "set input base 10", "/set unicode 1"],
        ),
    ];

    for (case_id, expression, raw_settings) in cases {
        let settings = raw_settings
            .iter()
            .map(|raw| SessionCommand {
                raw: (*raw).to_string(),
            })
            .collect::<Vec<_>>();
        let cpp_out = run_oracle_expression(&qalc, &defs, expression, &settings);
        let rust_out = run_rust_expression(expression, &settings, &defs, true, true);
        let fallback_state =
            oracle_fallback_gate::fallback_state_label(rust_out.fallback_state, false);

        assert_eq!(
            cpp_out.stdout, rust_out.stdout,
            "{case_id} stdout mismatch for {expression:?}; fallback={fallback_state}"
        );
        assert_eq!(
            cpp_out.stderr, rust_out.stderr,
            "{case_id} stderr mismatch for {expression:?}; fallback={fallback_state}"
        );
        assert_eq!(
            cpp_out.exit_code, rust_out.exit_code,
            "{case_id} exit code mismatch for {expression:?}; fallback={fallback_state}"
        );
        assert_eq!(
            fallback_state,
            FallbackState::Native.label(),
            "{case_id} did not run natively"
        );
    }
}

/// Focused fallback-disabled oracle evidence for `strings.batch` rows that do
/// not depend on non-setting session expressions.
#[test]
fn focused_epic6_strings_oracle_cases() {
    let Some(qalc) = oracle_binary() else {
        eprintln!(
            "skipping focused_epic6_strings_oracle_cases; \
             C++ oracle not available (set QALCULATE_ORACLE or build upstream qalc)"
        );
        return;
    };

    let defs = defs_dir();
    let cases: [(&str, &str, &[&str]); 17] = [
        ("strings.batch:14", r#"concatenate("a", "bc", 'defg')"#, &[]),
        ("strings.batch:16", r#"concatenate("", "c", '', 'd')"#, &[]),
        ("strings.batch:18", "concatenate(1,2)", &[]),
        ("strings.batch:20", "concatenate(1*2, 5)", &[]),
        ("strings.batch:22", "dec(concatenate(4*2, 5))", &[]),
        ("strings.batch:29", r#"len("")"#, &[]),
        ("strings.batch:31", r#"len(" ")"#, &[]),
        ("strings.batch:33", "len(5)", &[]),
        ("strings.batch:35", "len(5/6)", &[]),
        ("strings.batch:37", r#"len(concatenate("a", "bc"))"#, &[]),
        ("strings.batch:41", "0xD8 to unicode", &["/set unicode 1"]),
        ("strings.batch:43", "char(0xD8)", &["/set unicode 1"]),
        (
            "strings.batch:45",
            "char([0xD8, 0x61])",
            &["/set unicode 1"],
        ),
        ("strings.batch:47", "code(Ø) to hex", &["/set unicode 1"]),
        ("strings.batch:49", "code(😀) to hex", &["/set unicode 1"]),
        (
            "strings.batch:51",
            "code(🍉, utf-8, 0) to hex",
            &["/set unicode 1"],
        ),
        ("strings.batch:53", "code(abc)", &["/set unicode 1"]),
    ];

    for (case_id, expression, raw_settings) in cases {
        let settings = raw_settings
            .iter()
            .map(|raw| SessionCommand {
                raw: (*raw).to_string(),
            })
            .collect::<Vec<_>>();
        let cpp_out = run_oracle_expression(&qalc, &defs, expression, &settings);
        let rust_out = run_rust_expression(expression, &settings, &defs, true, true);
        let fallback_state =
            oracle_fallback_gate::fallback_state_label(rust_out.fallback_state, false);

        assert_eq!(
            cpp_out.stdout, rust_out.stdout,
            "{case_id} stdout mismatch for {expression:?}; fallback={fallback_state}"
        );
        assert_eq!(
            cpp_out.stderr, rust_out.stderr,
            "{case_id} stderr mismatch for {expression:?}; fallback={fallback_state}"
        );
        assert_eq!(
            cpp_out.exit_code, rust_out.exit_code,
            "{case_id} exit code mismatch for {expression:?}; fallback={fallback_state}"
        );
        assert_eq!(
            fallback_state,
            FallbackState::Native.label(),
            "{case_id} did not run natively"
        );
    }
}

/// Focused fallback-disabled oracle evidence for Epic 2 numeric slices that are
/// not represented as simple no-session upstream batch rows.
#[test]
fn focused_epic2_native_numeric_oracle_cases() {
    let Some(qalc) = oracle_binary() else {
        eprintln!(
            "skipping focused_epic2_native_numeric_oracle_cases; \
             C++ oracle not available (set QALCULATE_ORACLE or build upstream qalc)"
        );
        return;
    };

    let defs = defs_dir();
    let settings = Vec::new();
    let cases = [
        ("complex-imaginary-unit", "i"),
        ("complex-imaginary-coefficient", "5i"),
        ("complex-addition", "(1 + 2i) + (3 + 4i)"),
        ("complex-subtraction", "(1 + 2i) - (3 + 4i)"),
        ("complex-multiplication", "(1 + 2i) * (3 + 4i)"),
        ("complex-division", "(1 + 2i) / (3 + 4i)"),
        ("complex-zero-collapse-addition", "i + (-i)"),
        ("complex-pure-imaginary-addition", "(1 + i) + (-1 + i)"),
        ("complex-pure-real-addition", "(1 + i) + (2 - i)"),
        ("complex-pure-real-multiplication", "(1 + i) * (1 - i)"),
        ("complex-pure-imaginary-division", "(1 + i) / (1 - i)"),
        ("complex-conjugate", "conj(3 + 4i)"),
        ("complex-pure-imaginary-conjugate", "conj(i)"),
        ("complex-negative-imaginary-conjugate", "conj(-i)"),
        ("complex-real-conjugate", "conj(3)"),
        ("complex-norm", "norm(3 + 4i)"),
        ("complex-pure-imaginary-norm", "norm(i)"),
        ("complex-negative-pure-imaginary-norm", "norm(-3i)"),
        ("complex-power-unit", "i^2"),
        ("complex-power-explog", "(2i - 3)^(3.2i + 3)"),
        ("complex-equality-true", "(1 + i) = (1 + i)"),
        ("complex-equality-double-equals-true", "(1 + i) == (1 + i)"),
        ("complex-equality-false", "(1 + i) = (1 - i)"),
        ("complex-inequality-true", "(1 + i) != (1 - i)"),
        ("complex-inequality-unicode-true", "(1 + i) ≠ (1 - i)"),
        ("complex-inequality-false", "(1 + i) != (1 + i)"),
        ("complex-order-less-equal-operands", "(1 + i) < (1 + i)"),
        (
            "complex-order-less-equal-equal-operands",
            "(1 + i) <= (1 + i)",
        ),
        ("complex-order-greater-equal-operands", "(1 + i) > (1 + i)"),
        (
            "complex-order-greater-equal-equal-operands",
            "(1 + i) >= (1 + i)",
        ),
        (
            "complex-order-unicode-less-equal-equal-operands",
            "(1 + i) ≤ (1 + i)",
        ),
        (
            "complex-order-unicode-greater-equal-equal-operands",
            "(1 + i) ≥ (1 + i)",
        ),
        ("float-ln-zero-special-value", "ln(0)"),
        ("float-ln-two", "ln(2)"),
        ("uncertainty-ln-propagation", "ln(5+/-0.3)"),
        ("float-sqrt-two", "sqrt(2)"),
        ("float-sqrt-exact-square", "sqrt(4)"),
        ("float-positive-infinity-literal", "infinity"),
        ("float-negative-infinity-literal", "-infinity"),
        ("float-positive-infinity-addition", "infinity + 1"),
        ("float-negative-infinity-subtraction", "-infinity - 1"),
        ("float-positive-infinity-multiplication", "infinity * 2"),
        ("float-negative-infinity-multiplication", "infinity * -2"),
        ("float-division-by-positive-infinity", "1 / infinity"),
        ("float-positive-infinity-division", "infinity / 2"),
        ("float-positive-infinity-negative-division", "infinity / -2"),
        ("float-negative-infinity-division", "-infinity / 2"),
        (
            "float-negative-infinity-negative-division",
            "-infinity / -2",
        ),
        ("float-division-by-negative-infinity", "1 / -infinity"),
        ("nonterminating-rational-qalc-format", "1/3"),
        ("fixed-power-of-ten-qalc-format", "1e10"),
        ("original-scaffold-addition", "1 + 1"),
        ("rational-integer-power-caret", "5 ^ 2"),
        ("rational-negative-integer-power", "2 ^ -3"),
        ("float-noninteger-power", "2 ^ 0.5"),
        ("negative-rational-negative-integer-power", "(-2) ^ -3"),
        ("fractional-rational-negative-integer-power", "(1/2) ^ -3"),
        ("rational-integer-power-starstar", "5 ** 3"),
        ("rational-right-associative-starstar", "4 ** 3 ** 2"),
        ("absolute-uncertainty", "2+/-0.002"),
        ("relative-uncertainty", "100+/-5%"),
        (
            "mixed-absolute-relative-uncertainty-addition",
            "100+/-5 + 200+/-10%",
        ),
        ("relative-uncertainty-addition", "100+/-5% + 200+/-10%"),
        ("relative-uncertainty-scalar-multiplication", "100+/-5% * 2"),
        ("uncertainty-addition", "20+/-3 + 10+/-4"),
        ("uncertainty-multiplication", "3+/-0.2 * 4+/-0.1"),
        ("uncertainty-division", "12+/-0.5 / 3+/-0.2"),
        ("uncertainty-power-explog", "(2+/-3)^3.2"),
        ("zero-uncertainty-as-exact", "10 +/- 0"),
    ];

    for (case_id, expression) in cases {
        let cpp_out = run_oracle_expression(&qalc, &defs, expression, &settings);
        let rust_out = run_rust_expression(expression, &settings, &defs, true, true);
        let fallback_state =
            oracle_fallback_gate::fallback_state_label(rust_out.fallback_state, false);

        assert_eq!(
            cpp_out.stdout, rust_out.stdout,
            "{case_id} stdout mismatch for {expression:?}; fallback={fallback_state}"
        );
        assert_eq!(
            cpp_out.stderr, rust_out.stderr,
            "{case_id} stderr mismatch for {expression:?}; fallback={fallback_state}"
        );
        assert_eq!(
            cpp_out.exit_code, rust_out.exit_code,
            "{case_id} exit-code mismatch for {expression:?}; fallback={fallback_state}"
        );
        assert_eq!(
            fallback_state,
            FallbackState::Native.label(),
            "{case_id} did not run natively"
        );
    }
}

type NativeOracleCase<'a> = (&'a str, &'a str, &'a [&'a str]);

fn assert_native_oracle_cases(qalc: &Path, defs: &Path, cases: &[NativeOracleCase<'_>]) {
    for (case_id, expression, raw_settings) in cases {
        let settings = raw_settings
            .iter()
            .map(|raw| SessionCommand {
                raw: (*raw).to_string(),
            })
            .collect::<Vec<_>>();
        let cpp_out = run_oracle_expression(qalc, defs, expression, &settings);
        let rust_out = run_rust_expression(expression, &settings, defs, true, true);
        let fallback_state =
            oracle_fallback_gate::fallback_state_label(rust_out.fallback_state, false);

        assert_eq!(
            cpp_out.stdout, rust_out.stdout,
            "{case_id} stdout mismatch for {expression:?}; fallback={fallback_state}"
        );
        assert_eq!(
            cpp_out.stderr, rust_out.stderr,
            "{case_id} stderr mismatch for {expression:?}; fallback={fallback_state}"
        );
        assert_eq!(
            cpp_out.exit_code, rust_out.exit_code,
            "{case_id} exit-code mismatch for {expression:?}; fallback={fallback_state}"
        );
        assert_eq!(
            fallback_state,
            FallbackState::Native.label(),
            "{case_id} did not run natively"
        );
    }
}

#[test]
fn focused_issue41_vector_matrix_literal_oracle_cases() {
    let Some(qalc) = oracle_binary() else {
        eprintln!(
            "skipping focused_issue41_vector_matrix_literal_oracle_cases; \
             C++ oracle not available (set QALCULATE_ORACLE or build upstream qalc)"
        );
        return;
    };

    let defs = defs_dir();
    let default_settings = &[][..];
    let cases: &[NativeOracleCase<'_>] = &[
        (
            "matrixvector-literal-trailing-zero",
            "(1,)",
            default_settings,
        ),
        (
            "matrixvector-literal-top-level-trailing-zero",
            "1,",
            default_settings,
        ),
        (
            "matrixvector-literal-empty-slots",
            "[,,,]",
            default_settings,
        ),
        (
            "matrixvector-literal-parenthesized-empty-slots",
            "(,,,-2)",
            default_settings,
        ),
        (
            "matrixvector-literal-semicolon-empty-slot",
            "(1;;2)",
            default_settings,
        ),
        (
            "matrixvector-literal-parenthesized-vector",
            "(1,1)",
            default_settings,
        ),
        (
            "matrixvector-literal-parenthesized-matrix",
            "((1, 2), (4, 5))",
            default_settings,
        ),
        (
            "matrixvector-literal-semicolon-matrix",
            "((1; 2; 3); (4; 5; 6))",
            default_settings,
        ),
        (
            "matrixvector-literal-bracket-nested-matrix",
            "[[1, 2], [4, 5]]",
            default_settings,
        ),
        (
            "matrixvector-literal-decimal-matrix",
            "[-0.1, 1.23, ], [.1, , -.2], [,,]",
            default_settings,
        ),
        (
            "matrixvector-access-columns-empty",
            "columns([])",
            default_settings,
        ),
        (
            "matrixvector-access-columns-vector",
            "columns([1])",
            default_settings,
        ),
        (
            "matrixvector-access-columns-nested-empty",
            "columns([[,,,]])",
            default_settings,
        ),
        (
            "matrixvector-shape-dimension-empty",
            "dimension([])",
            default_settings,
        ),
        (
            "matrixvector-shape-dimension-singleton",
            "dimension([0])",
            default_settings,
        ),
        (
            "matrixvector-shape-dimension-vector",
            "dimension([1 2 3 4])",
            default_settings,
        ),
        (
            "matrixvector-constructor-singleton-matrix",
            "matrix(1, 1, [2])",
            default_settings,
        ),
        (
            "matrixvector-constructor-row-matrix",
            "matrix(1, 3, 2)",
            default_settings,
        ),
        (
            "matrixvector-constructor-column-matrix",
            "matrix(3, 1, [1 2])",
            default_settings,
        ),
        (
            "matrixvector-constructor-vector",
            "vector(1, 2, 3)",
            default_settings,
        ),
        (
            "matrixvector-constructor-empty-vector",
            "vector()",
            default_settings,
        ),
        (
            "matrixvector-constructor-omitted-vector",
            "vector(,)",
            default_settings,
        ),
        (
            "matrixvector-constructor-zero-matrix",
            "matrix(3, 3, [])",
            default_settings,
        ),
        (
            "matrixvector-constructor-filled-matrix",
            "matrix(3, 3, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10)",
            default_settings,
        ),
        (
            "matrixvector-access-matrix2vector-singleton",
            "matrix2vector([[0]])",
            default_settings,
        ),
        (
            "matrixvector-access-matrix2vector",
            "matrix2vector([1 2; 4 5])",
            default_settings,
        ),
        (
            "matrixvector-access-matrix2vector-3x3",
            "matrix2vector([1 2 3; 4 5 6; 7 8 9])",
            default_settings,
        ),
        (
            "matrixvector-access-horzcat-row-vectors",
            "horzcat([1], [2 3], [4 5 6 7])",
            default_settings,
        ),
        (
            "matrixvector-access-horzcat-matrices",
            "horzcat([1; 2], [3 4; 5 6], [7 8 9; 10 11 12])",
            default_settings,
        ),
        (
            "matrixvector-access-vertcat-row-vectors",
            "vertcat([1 2], [3 4], [5 6])",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-dot-scalars",
            "dot((2); (3))",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-dot-two-vectors",
            "dot((1; 2); (3, 4))",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-dot-three-vectors",
            "dot((1; 2; 3); (4; 5; 6))",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-dot-operator-three-vectors",
            "(1; 2; 3).(4; 5; 6)",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-dot-operator-four-vectors",
            "(1; 2; 3, 4) . (5; 6; 7, 8)",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-cross-three-vectors",
            "cross((1; 2; 3); (4; 5; 6))",
            default_settings,
        ),
        (
            "matrixvector-access-columns",
            "columns([1 2; 4 5])",
            default_settings,
        ),
        (
            "matrixvector-access-column-scalar",
            "column([1], 1)",
            default_settings,
        ),
        (
            "matrixvector-access-column-vector",
            "column([1, 2], 1)",
            default_settings,
        ),
        (
            "matrixvector-access-column-matrix",
            "column([1 2; 3 4], 2)",
            default_settings,
        ),
        (
            "matrixvector-access-element-row",
            "element([1 2; 3 4], 1)",
            default_settings,
        ),
        (
            "matrixvector-access-element-cell",
            "element([1 2 3; 4 5 6; 1 0 9], 1, 3)",
            default_settings,
        ),
        (
            "matrixvector-access-element",
            "element([1 2 3; 4 5 6], 2, 1)",
            default_settings,
        ),
        (
            "matrixvector-access-elements-empty",
            "elements([])",
            default_settings,
        ),
        (
            "matrixvector-access-elements-vector",
            "elements([1 2])",
            default_settings,
        ),
        (
            "matrixvector-access-elements",
            "elements([1 2; 3 4])",
            default_settings,
        ),
        (
            "matrixvector-access-row-scalar",
            "row([1], 1)",
            default_settings,
        ),
        (
            "matrixvector-access-row-vector",
            "row([1 2], 1)",
            default_settings,
        ),
        (
            "matrixvector-access-row-matrix",
            "row([1 2; 3 4], 2)",
            default_settings,
        ),
        (
            "matrixvector-shape-rows-vector",
            "rows([1])",
            default_settings,
        ),
        (
            "matrixvector-shape-rows-matrix",
            "rows([1 2; 3 4])",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-vector-addition",
            "[1,2] + [3,4]",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-multiply-scalar",
            "multiply(1)",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-multiply-matrix-scalar",
            "multiply([1 2; 4 5], 2)",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-multiply-vector-scalars",
            "multiply([1 2], 3, 4)",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-multiply-vector-scalar",
            "multiply([1 2], 3)",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-multiply-matrix-scalars",
            "multiply([1 2; 4 5], 2, 3)",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-hadamard-singleton-vectors",
            "hadamard([2], [3], [4])",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-hadamard-matrices",
            "hadamard([1 2 3; 4 5 6]; [7 8 9; 10 11 12])",
            default_settings,
        ),
        (
            "matrixvector-constructor-identity-singleton",
            "identity(1)",
            default_settings,
        ),
        (
            "matrixvector-constructor-identity-3x3",
            "identity(3)",
            default_settings,
        ),
        (
            "matrixvector-constructor-identity-from-matrix",
            "identity([1 2; 4 5])",
            default_settings,
        ),
        (
            "matrixvector-access-combine-single-vector",
            "combine([1, 2])",
            default_settings,
        ),
        (
            "matrixvector-access-combine-multiple-vectors",
            "combine([1, 2], [3], [4, 5, 6])",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-magnitude-scalar",
            "magnitude(-2)",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-magnitude-singleton-vector",
            "magnitude([-2])",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-magnitude-vector",
            "magnitude([-2, 3, 4])",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-norm-singleton",
            "norm([2])",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-norm-two-vector",
            "norm([3, 4])",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-norm-three-vector",
            "norm([2, 3, 6])",
            default_settings,
        ),
        ("matrixvector-adj-2x2", "adj([1 2; 4 5])", default_settings),
        (
            "matrixvector-adj-3x3",
            "adj([1, 2, 3; 4, 5, 6; 1, 0, 9])",
            default_settings,
        ),
        (
            "matrixvector-adj-4x4",
            "adj([3 4 7 9; 5 4 -1 4; 8 7 8 5; 4 3 0 9])",
            default_settings,
        ),
        (
            "matrixvector-cofactor-2x2",
            "cofactor([1 2; 4 5], 1, 1)",
            default_settings,
        ),
        (
            "matrixvector-cofactor-3x3",
            "cofactor([1 2 3; 4 5 6; 1 0 9], 1, 2)",
            default_settings,
        ),
        (
            "matrixvector-cofactor-4x4",
            "cofactor([3 4 7 9; 5 4 -1 4; 8 7 8 5; 4 3 0 9], 4, 4)",
            default_settings,
        ),
        (
            "matrixvector-permanent-1x1",
            "permanent([1])",
            default_settings,
        ),
        (
            "matrixvector-permanent-2x2",
            "permanent([1 2; 4 5])",
            default_settings,
        ),
        (
            "matrixvector-permanent-3x3",
            "permanent([1 2 3; 4 5 6; 1 0 9])",
            default_settings,
        ),
        (
            "matrixvector-permanent-4x4",
            "permanent([3 4 7 9; 5 4 -1 4; 8 7 8 5; 4 3 0 9])",
            default_settings,
        ),
        (
            "matrixvector-access-part-singleton",
            "part([1], 1, 1, 1, 1)",
            default_settings,
        ),
        (
            "matrixvector-access-part-cell",
            "part([1 2 3; 4 5 6; 7 8 9; 10 11 12], 2, 2, 2, 2)",
            default_settings,
        ),
        (
            "matrixvector-access-part-column",
            "part([1 2 3; 4 5 6; 7 8 9; 10 11 12], 1, 3, 2, 3)",
            default_settings,
        ),
        (
            "matrixvector-access-part-block",
            "part([1 2 3; 4 5 6; 7 8 9; 10 11 12], 1, 2, 4, 3)",
            default_settings,
        ),
        (
            "matrixvector-access-slice-singleton",
            "slice([5], 1, 1)",
            default_settings,
        ),
        (
            "matrixvector-access-slice-vector-range",
            "slice([5, 6, 7, 8, 9], 2, 4)",
            default_settings,
        ),
        (
            "matrixvector-access-sort-default",
            "sort([5, 2, 0, 1, 3, -4, 0])",
            default_settings,
        ),
        (
            "matrixvector-access-sort-ascending",
            "sort([5, 2, 0, 1, 3, -4, 0], 1)",
            default_settings,
        ),
        (
            "matrixvector-access-sort-descending",
            "sort([5, 2, 0, 1, 3, -4, 0], 0)",
            default_settings,
        ),
        (
            "matrixvector-access-rank-default",
            "rank([6, 7, 1, 4])",
            default_settings,
        ),
        (
            "matrixvector-access-rank-ascending",
            "rank([-1, 2, 5, 10], 1)",
            default_settings,
        ),
        (
            "matrixvector-access-rank-descending",
            "rank([-1, 2, 5, 10], 0)",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-entrywise-identity",
            "entrywise(x, [4 10 12], x)",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-entrywise-divide",
            "entrywise(x / y, [4 10 12], x, [2 2 4], y)",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-entrywise-divide-add",
            "entrywise(x / y + z, [4 10 12], x, [2 2 4], y, [1 2 3], z)",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-pow-function",
            "pow([1 2; 3 4], 2)",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-entrywise-power-scalar",
            "[1 2; 3 4].^2",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-entrywise-power-column",
            "[2 4; 3 4].^[-1; 2]",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-entrywise-power-outer",
            "[2; 3].^[3 4]",
            default_settings,
        ),
        (
            "matrixvector-access-transpose-function",
            "transpose([1 2; 3 4])",
            default_settings,
        ),
        (
            "matrixvector-access-transpose-operator",
            "[1 2 3; 4 5 6].'",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-times-vector-scalars",
            "[1 2] times 3 times 4",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-times-vector-scalar-mixed-case",
            "[1 2] Times 3",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-vector-scale-subtract",
            "(1; 2; 3) * 2 - 2",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-matrix-scale",
            "[1 2; 4 5] * 2",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-matrix-multiply",
            "((1; 2; 3); (4; 5; 6)) * ((7; 8); (9; 10); (11; 12))",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-times-matrix-multiply",
            "[1 2; 3 4] times [5 6; 7 8]",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-elementwise-vector-multiply",
            "[1 2].*[3 4]",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-elementwise-broadcast-multiply",
            "[1; 2].*[3 4]",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-elementwise-multiply",
            "[1 2; 3 4].*[1 2; 3 4]",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-vector-divide-scalar",
            "[2 4 12] / 2",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-divide-function",
            "divide([2 4 12], 2)",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-rdivide-alias",
            "rdivide([2 4 12], 2)",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-divide-function-matrix-scalar",
            "divide([2 4; 6 12], 2)",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-rdivide-vector-denominator",
            "rdivide([2 4], [1 2])",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-elementwise-divide-scalar",
            "[2 4 12]./2",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-elementwise-divide-left-associative",
            "[2 4]./1/2",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-matrix-divide-scalar",
            "[[2, 4], [6, 12]] / 2",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-divide-function-matrix",
            "divide([2 4; 6 12], [1 2; 3 4])",
            default_settings,
        ),
        (
            "matrixvector-arithmetic-elementwise-matrix-divide",
            "[2 4; 6 12]./[1 2; 3 4]",
            default_settings,
        ),
        ("matrixvector-det-1x1", "det([[1]])", default_settings),
        ("matrixvector-det-2x2", "det([1 2; 4 5])", default_settings),
        (
            "matrixvector-det-3x3",
            "det([1 2 3; 4 5 6; 1 0 9])",
            default_settings,
        ),
        (
            "matrixvector-det-4x4",
            "det([3 4 7 9; 5 4 -1 4; 8 7 8 5; 4 3 0 9])",
            default_settings,
        ),
        (
            "matrixvector-inverse-power-2x2",
            "((1; 2); (3; 4))^-1",
            default_settings,
        ),
        (
            "matrixvector-inverse-2x2",
            "inverse([1 2; 3 5])",
            default_settings,
        ),
        (
            "matrixvector-inverse-3x3",
            "inverse([1  2  3; 4  5  6; 1  0  9])",
            default_settings,
        ),
        (
            "matrixvector-inverse-4x4",
            "inverse([1 1 1 1; 2 4 -1 4; 2 4 3 4; 4 3 0 2])",
            default_settings,
        ),
        (
            "matrixvector-genvector-two-steps",
            "genvector(x+10, 1, 2, 2)",
            default_settings,
        ),
        (
            "matrixvector-genvector-three-steps",
            "genvector(x+10, 1, 2, 3)",
            default_settings,
        ),
        (
            "matrixvector-genvector-five-steps",
            "genvector(x+10, -1, 2, 5)",
            default_settings,
        ),
        (
            "matrixvector-genvector-seven-zero-mode",
            "genvector(x+10, -1, 2, 7, x, 0)",
            default_settings,
        ),
        (
            "matrixvector-genvector-five-one-mode",
            "genvector(x+100, -3, 5, 2, x, 1)",
            default_settings,
        ),
        (
            "matrixvector-genvector-symbolic-y",
            "genvector(x+100, 1, 2, 1, y, 1)",
            default_settings,
        ),
        (
            "matrixvector-rk-2x3",
            "rk([1 2 3; 3 6 9])",
            default_settings,
        ),
        (
            "matrixvector-rk-3x3-a",
            "rk([1 2 3; 0 2 2; 1 4 5])",
            default_settings,
        ),
        (
            "matrixvector-rk-3x3-b",
            "rk([1 2 3; 0 2 2; 1 -2 -1])",
            default_settings,
        ),
        (
            "matrixvector-rk-identity",
            "rk(identity(3))",
            default_settings,
        ),
        (
            "matrixvector-rref-3x4",
            "rref([1 3 1 9; 1 1 -1 1; 3 11 5 35])",
            default_settings,
        ),
    ];

    assert_native_oracle_cases(&qalc, &defs, cases);
}

#[test]
fn focused_issue15_uncertainty_input_oracle_cases() {
    let Some(qalc) = oracle_binary() else {
        eprintln!(
            "skipping focused_issue15_uncertainty_input_oracle_cases; \
             C++ oracle not available (set QALCULATE_ORACLE or build upstream qalc)"
        );
        return;
    };

    let defs = defs_dir();
    let cases: [NativeOracleCase<'_>; 23] = [
        ("spaced-ascii-absolute-uncertainty", "2 +/- 0.002", &[]),
        (
            "spaced-ascii-absolute-uncertainty-addition",
            "2 +/- 0.002 + 3",
            &[],
        ),
        ("unicode-absolute-uncertainty", "2±0.002", &[]),
        ("unicode-absolute-uncertainty-addition", "2±0.002 + 3", &[]),
        (
            "absolute-uncertainty-constructor",
            "uncertainty(2;0.002;0)",
            &[],
        ),
        (
            "relative-uncertainty-constructor",
            "uncertainty(100;0.05;1)",
            &[],
        ),
        (
            "zero-uncertainty-constructor-collapse",
            "uncertainty(10;0;0)",
            &[],
        ),
        ("uncertainty-error-part", "errorPart(2+/-0.002)", &[]),
        (
            "relative-uncertainty-absolute-error-part",
            "errorPart(100+/-5%)",
            &[],
        ),
        (
            "uncertainty-explicit-absolute-error-part",
            "errorPart(2+/-0.002;0)",
            &[],
        ),
        (
            "uncertainty-relative-error-part-from-absolute-input",
            "errorPart(2+/-0.002;1)",
            &[],
        ),
        (
            "relative-uncertainty-explicit-absolute-error-part",
            "errorPart(100+/-5%;0)",
            &[],
        ),
        (
            "relative-uncertainty-relative-error-part",
            "errorPart(100+/-5%;1)",
            &[],
        ),
        ("uncertainty-value-part", "valuePart(2+/-0.002)", &[]),
        (
            "relative-uncertainty-value-part",
            "valuePart(100+/-5%)",
            &[],
        ),
        ("uncertainty-midpoint", "midpoint(2+/-0.002)", &[]),
        (
            "uncertainty-lower-endpoint",
            "lowerEndpoint(2+/-0.002)",
            &[],
        ),
        (
            "uncertainty-upper-endpoint",
            "upperEndpoint(2+/-0.002)",
            &[],
        ),
        (
            "uncertainty-subtraction-propagation",
            "20+/-3 - 10+/-4",
            &[],
        ),
        (
            "uncertainty-division-propagation-alt-row",
            "3+/-0.2 / 4+/-0.1",
            &[],
        ),
        (
            "concise-uncertainty-decimal",
            "1.23(4)",
            &["/set concise uncertainty 1"],
        ),
        (
            "concise-uncertainty-integer",
            "123(4)",
            &["/set concise uncertainty 1"],
        ),
        (
            "concise-uncertainty-addition",
            "1.23(4) + 2.0(3)",
            &["/set concise uncertainty 1"],
        ),
    ];

    assert_native_oracle_cases(&qalc, &defs, &cases);
}

#[test]
fn focused_epic2_float_precision_oracle_cases() {
    let Some(qalc) = oracle_binary() else {
        eprintln!(
            "skipping focused_epic2_float_precision_oracle_cases; \
             C++ oracle not available (set QALCULATE_ORACLE or build upstream qalc)"
        );
        return;
    };

    let defs = defs_dir();
    let cases: [NativeOracleCase<'_>; 38] = [
        (
            "rational-output-precision-context",
            "1/3",
            &["/set precision 128"],
        ),
        (
            "float-power-precision-context",
            "2 ^ 0.5",
            &["/set precision 128"],
        ),
        ("float-ln-two-precision-64", "ln(2)", &["/set precision 64"]),
        (
            "float-sqrt-two-precision-64",
            "sqrt(2)",
            &["/set precision 64"],
        ),
        (
            "float-sqrt-exact-square-precision-64",
            "sqrt(4)",
            &["/set precision 64"],
        ),
        (
            "float-ln-zero-precision-64",
            "ln(0)",
            &["/set precision 64"],
        ),
        (
            "float-ln-plus-sqrt-precision-64",
            "ln(2) + sqrt(2)",
            &["/set precision 64"],
        ),
        (
            "float-ln-two-precision-128",
            "ln(2)",
            &["/set precision 128"],
        ),
        (
            "float-sqrt-two-precision-128",
            "sqrt(2)",
            &["/set precision 128"],
        ),
        (
            "float-sqrt-exact-square-precision-128",
            "sqrt(4)",
            &["/set precision 128"],
        ),
        (
            "float-ln-zero-precision-128",
            "ln(0)",
            &["/set precision 128"],
        ),
        (
            "float-ln-plus-sqrt-precision-128",
            "ln(2) + sqrt(2)",
            &["/set precision 128"],
        ),
        (
            "float-addition-precision-64",
            "(2 ^ 0.5) + (3 ^ 0.5)",
            &["/set precision 64"],
        ),
        (
            "float-subtraction-precision-64",
            "(3 ^ 0.5) - (2 ^ 0.5)",
            &["/set precision 64"],
        ),
        (
            "float-multiplication-precision-64",
            "(2 ^ 0.5) * (3 ^ 0.5)",
            &["/set precision 64"],
        ),
        (
            "float-division-precision-64",
            "(3 ^ 0.5) / (2 ^ 0.5)",
            &["/set precision 64"],
        ),
        (
            "mixed-rational-float-addition-precision-64",
            "(2 ^ 0.5) + 1/3",
            &["/set precision 64"],
        ),
        (
            "float-addition-precision-context",
            "(2 ^ 0.5) + (3 ^ 0.5)",
            &["/set precision 128"],
        ),
        (
            "float-subtraction-precision-context",
            "(3 ^ 0.5) - (2 ^ 0.5)",
            &["/set precision 128"],
        ),
        (
            "float-multiplication-precision-context",
            "(2 ^ 0.5) * (3 ^ 0.5)",
            &["/set precision 128"],
        ),
        (
            "float-division-precision-context",
            "(3 ^ 0.5) / (2 ^ 0.5)",
            &["/set precision 128"],
        ),
        (
            "mixed-rational-float-addition-precision-context",
            "(2 ^ 0.5) + 1/3",
            &["/set precision 128"],
        ),
        (
            "decimal-addition-precision-64",
            "0.1 + 0.2",
            &["/set precision 64"],
        ),
        (
            "decimal-addition-precision-128",
            "0.1 + 0.2",
            &["/set precision 128"],
        ),
        (
            "scientific-addition-precision-64",
            "1.25e-20 + 2.5e-20",
            &["/set precision 64"],
        ),
        (
            "scientific-addition-precision-128",
            "1.25e-20 + 2.5e-20",
            &["/set precision 128"],
        ),
        (
            "scientific-division-precision-64",
            "2.5e3 / 4",
            &["/set precision 64"],
        ),
        (
            "scientific-division-precision-128",
            "2.5e3 / 4",
            &["/set precision 128"],
        ),
        (
            "float-less-than-precision-64",
            "(2 ^ 0.5) < (3 ^ 0.5)",
            &["/set precision 64"],
        ),
        (
            "float-equality-true-precision-64",
            "(2 ^ 0.5) = (2 ^ 0.5)",
            &["/set precision 64"],
        ),
        (
            "float-equality-false-precision-64",
            "(2 ^ 0.5) = (3 ^ 0.5)",
            &["/set precision 64"],
        ),
        (
            "mixed-rational-float-greater-than-precision-64",
            "(2 ^ 0.5) + 1/3 > 1",
            &["/set precision 64"],
        ),
        (
            "mixed-rational-float-less-than-false-precision-64",
            "(2 ^ 0.5) < 1/3",
            &["/set precision 64"],
        ),
        (
            "float-less-than-precision-context",
            "(2 ^ 0.5) < (3 ^ 0.5)",
            &["/set precision 128"],
        ),
        (
            "float-equality-true-precision-context",
            "(2 ^ 0.5) = (2 ^ 0.5)",
            &["/set precision 128"],
        ),
        (
            "float-equality-false-precision-context",
            "(2 ^ 0.5) = (3 ^ 0.5)",
            &["/set precision 128"],
        ),
        (
            "mixed-rational-float-greater-than-precision-context",
            "(2 ^ 0.5) + 1/3 > 1",
            &["/set precision 128"],
        ),
        (
            "mixed-rational-float-less-than-false-precision-context",
            "(2 ^ 0.5) < 1/3",
            &["/set precision 128"],
        ),
    ];

    assert_native_oracle_cases(&qalc, &defs, &cases);
}

#[test]
fn focused_epic2_interval_display_oracle_cases() {
    let Some(qalc) = oracle_binary() else {
        eprintln!(
            "skipping focused_epic2_interval_display_oracle_cases; \
             C++ oracle not available (set QALCULATE_ORACLE or build upstream qalc)"
        );
        return;
    };

    let defs = defs_dir();
    let cases: [NativeOracleCase<'_>; 11] = [
        (
            "interval-function-normalizes-reversed-bounds",
            "interval(5;2)",
            &["/set interval display 2"],
        ),
        (
            "interval-function-closed-endpoint-flag",
            "interval(1;3;0)",
            &["/set interval display 2"],
        ),
        (
            "interval-function-excluded-endpoint-flag-integer-bounds",
            "interval(1;3;1)",
            &["/set interval display 2"],
        ),
        (
            "interval-function-lower-infinity-endpoint",
            "interval(-infinity;5)",
            &["/set interval display 2"],
        ),
        (
            "interval-function-upper-infinity-endpoint",
            "interval(4;infinity)",
            &["/set interval display 2"],
        ),
        (
            "interval-function-lower-infinity-endpoint-with-ic2",
            "interval(-infinity;5)",
            &["/set interval display 2", "/set ic 2"],
        ),
        (
            "interval-function-upper-infinity-endpoint-with-ic2",
            "interval(4;infinity)",
            &["/set interval display 2", "/set ic 2"],
        ),
        (
            "interval-function-negative-infinity-only-endpoint",
            "interval(-infinity;-4)",
            &["/set interval display 2"],
        ),
        (
            "interval-function-negative-finite-only-endpoint",
            "interval(-3;-1)",
            &["/set interval display 2"],
        ),
        (
            "interval-function-negative-infinity-only-endpoint-with-ic2",
            "interval(-infinity;-4)",
            &["/set interval display 2", "/set ic 2"],
        ),
        (
            "interval-function-negative-finite-only-endpoint-with-ic2",
            "interval(-3;-1)",
            &["/set interval display 2", "/set ic 2"],
        ),
    ];

    assert_native_oracle_cases(&qalc, &defs, &cases);
}

#[test]
fn focused_epic2_interval_endpoint_oracle_cases() {
    let Some(qalc) = oracle_binary() else {
        eprintln!(
            "skipping focused_epic2_interval_endpoint_oracle_cases; \
             C++ oracle not available (set QALCULATE_ORACLE or build upstream qalc)"
        );
        return;
    };

    let defs = defs_dir();
    let cases: [NativeOracleCase<'_>; 8] = [
        (
            "interval-lower-endpoint-finite",
            "lowerEndpoint(interval(1;3))",
            &["/set interval display 2", "/set ic 2"],
        ),
        (
            "interval-upper-endpoint-finite",
            "upperEndpoint(interval(1;3))",
            &["/set interval display 2", "/set ic 2"],
        ),
        (
            "interval-midpoint-finite",
            "midpoint(interval(1;3))",
            &["/set interval display 2", "/set ic 2"],
        ),
        (
            "interval-lower-endpoint-finite-excluded-flag",
            "lowerEndpoint(interval(1;3;1))",
            &["/set interval display 2", "/set ic 2"],
        ),
        (
            "interval-upper-endpoint-finite-excluded-flag",
            "upperEndpoint(interval(1;3;1))",
            &["/set interval display 2", "/set ic 2"],
        ),
        (
            "interval-midpoint-finite-excluded-flag",
            "midpoint(interval(1;3;1))",
            &["/set interval display 2", "/set ic 2"],
        ),
        (
            "interval-lower-endpoint-negative-infinity",
            "lowerEndpoint(interval(-infinity;-4))",
            &["/set interval display 2", "/set ic 2"],
        ),
        (
            "interval-upper-endpoint-positive-infinity",
            "upperEndpoint(interval(4;infinity))",
            &["/set interval display 2", "/set ic 2"],
        ),
    ];

    assert_native_oracle_cases(&qalc, &defs, &cases);
}

#[test]
fn focused_epic2_interval_intersection_oracle_cases() {
    let Some(qalc) = oracle_binary() else {
        eprintln!(
            "skipping focused_epic2_interval_intersection_oracle_cases; \
             C++ oracle not available (set QALCULATE_ORACLE or build upstream qalc)"
        );
        return;
    };

    let defs = defs_dir();
    let cases: [NativeOracleCase<'_>; 1] = [(
        "interval-intersect-disjoint-intervals",
        "intersect(interval(1;2), interval(3;4))",
        &["/set interval display 2", "/set ic 2"],
    )];

    assert_native_oracle_cases(&qalc, &defs, &cases);
}

#[test]
fn focused_epic2_interval_arithmetic_oracle_cases() {
    let Some(qalc) = oracle_binary() else {
        eprintln!(
            "skipping focused_epic2_interval_arithmetic_oracle_cases; \
             C++ oracle not available (set QALCULATE_ORACLE or build upstream qalc)"
        );
        return;
    };

    let defs = defs_dir();
    let cases: [NativeOracleCase<'_>; 16] = [
        (
            "interval-addition-closed-finite-endpoint-mode",
            "interval(1;2) + interval(3;4)",
            &["/set interval display 2", "/set ic 2"],
        ),
        (
            "interval-subtraction-closed-finite-endpoint-mode",
            "interval(3;4) - interval(1;2)",
            &["/set interval display 2", "/set ic 2"],
        ),
        (
            "interval-multiplication-closed-finite-endpoint-mode",
            "interval(-2;3) * interval(-4;5)",
            &["/set interval display 2", "/set ic 2"],
        ),
        (
            "interval-division-closed-finite-endpoint-mode",
            "interval(4;6) / interval(2;3)",
            &["/set interval display 2", "/set ic 2"],
        ),
        (
            "interval-addition-lower-infinity-endpoint-mode",
            "interval(-infinity;5) + interval(2;3)",
            &["/set interval display 2", "/set ic 2"],
        ),
        (
            "interval-subtraction-lower-infinity-endpoint-mode",
            "interval(-infinity;5) - interval(2;3)",
            &["/set interval display 2", "/set ic 2"],
        ),
        (
            "interval-multiplication-lower-infinity-endpoint-mode",
            "interval(-infinity;5) * interval(2;3)",
            &["/set interval display 2", "/set ic 2"],
        ),
        (
            "interval-addition-upper-infinity-endpoint-mode",
            "interval(4;infinity) + interval(2;3)",
            &["/set interval display 2", "/set ic 2"],
        ),
        (
            "interval-subtraction-upper-infinity-endpoint-mode",
            "interval(4;infinity) - interval(2;3)",
            &["/set interval display 2", "/set ic 2"],
        ),
        (
            "interval-multiplication-upper-infinity-endpoint-mode",
            "interval(4;infinity) * interval(2;3)",
            &["/set interval display 2", "/set ic 2"],
        ),
        (
            "interval-scalar-division-upper-infinity-endpoint-mode",
            "interval(4;infinity) / 2",
            &["/set interval display 2", "/set ic 2"],
        ),
        (
            "interval-division-positive-by-negative-closed-finite-endpoint-mode",
            "interval(4;6) / interval(-3;-2)",
            &["/set interval display 2", "/set ic 2"],
        ),
        (
            "interval-division-negative-by-positive-closed-finite-endpoint-mode",
            "interval(-6;-4) / interval(2;3)",
            &["/set interval display 2", "/set ic 2"],
        ),
        (
            "interval-division-negative-by-negative-closed-finite-endpoint-mode",
            "interval(-6;-4) / interval(-3;-2)",
            &["/set interval display 2", "/set ic 2"],
        ),
        (
            "interval-scalar-division-negative-infinity-positive-scalar-endpoint-mode",
            "interval(-infinity;-4) / 2",
            &["/set interval display 2", "/set ic 2"],
        ),
        (
            "interval-scalar-division-negative-infinity-negative-scalar-endpoint-mode",
            "interval(-infinity;-4) / -2",
            &["/set interval display 2", "/set ic 2"],
        ),
    ];

    assert_native_oracle_cases(&qalc, &defs, &cases);
}

#[test]
fn focused_epic2_interval_arithmetic_requires_ic2_guard() {
    let Some(qalc) = oracle_binary() else {
        eprintln!(
            "skipping focused_epic2_interval_arithmetic_requires_ic2_guard; \
             C++ oracle not available (set QALCULATE_ORACLE or build upstream qalc)"
        );
        return;
    };

    let defs = defs_dir();
    let expression = "interval(-2;3) * interval(-4;5)";
    let infinity_expression = "interval(4;infinity) + interval(2;3)";
    let display_only = [SessionCommand {
        raw: "/set interval display 2".to_string(),
    }];
    let endpoint_mode = [
        SessionCommand {
            raw: "/set interval display 2".to_string(),
        },
        SessionCommand {
            raw: "/set ic 2".to_string(),
        },
    ];

    let cpp_display_only = run_oracle_expression(&qalc, &defs, expression, &display_only);
    let cpp_endpoint_mode = run_oracle_expression(&qalc, &defs, expression, &endpoint_mode);
    assert_ne!(
        cpp_display_only.stdout, cpp_endpoint_mode.stdout,
        "upstream default interval calculation must differ before Rust can reject the default mode"
    );

    let rust_display_only = run_rust_expression(expression, &display_only, &defs, true, true);
    let fallback_state =
        oracle_fallback_gate::fallback_state_label(rust_display_only.fallback_state, false);

    assert_eq!(
        fallback_state,
        FallbackState::Disabled.label(),
        "Rust must not claim native interval arithmetic without /set ic 2"
    );

    let rust_infinity_display_only =
        run_rust_expression(infinity_expression, &display_only, &defs, true, true);
    let infinity_fallback_state = oracle_fallback_gate::fallback_state_label(
        rust_infinity_display_only.fallback_state,
        false,
    );

    assert_eq!(
        infinity_fallback_state,
        FallbackState::Disabled.label(),
        "Rust must not claim native infinity interval arithmetic without /set ic 2"
    );
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
/// Useful as a targeted development gate. Unlike the all-batch exploratory
/// sweep, this test fails on any mismatch.
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

    assert!(
        mismatches.is_empty(),
        "differential oracle {batch_name} found {} mismatch(es)",
        mismatches.len()
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
            normalization_policy: "exact-utf8".to_string(),
            fallback_state: FallbackState::CppFallbackEnabled.label().to_string(),
            session_commands: Vec::new(),
        };
        let display = m.to_string();
        assert!(display.contains("MISMATCH"));
        assert!(display.contains("batch=test.batch"));
        assert!(display.contains("case=0"));
        assert!(display.contains("field=stdout"));
        assert!(display.contains("cpp=\"2\""));
        assert!(display.contains("rust=\"3\""));
        assert!(display.contains("deviation=\"none\""));
        assert!(display.contains("normalization=exact-utf8"));
        assert!(display.contains("fallback=cpp-fallback-enabled"));
        assert!(display.contains("session=[]"));
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
            normalization_policy: "exact-utf8".to_string(),
            fallback_state: FallbackState::Native.label().to_string(),
            session_commands: vec!["/set precision 10".to_string()],
        };
        let display = m.to_string();
        assert!(display.contains("deviation=\"PRECISION-001\""));
        assert!(display.contains("fallback=native"));
        assert!(display.contains("/set precision 10"));
    }

    #[test]
    fn session_command_set_produces_correct_args() {
        let cmd = SessionCommand {
            raw: "/set approximation exact".to_string(),
        };
        assert_eq!(cmd.to_qalc_args(), vec!["-set", "approximation exact"]);
    }

    #[test]
    fn session_command_bare_set_produces_correct_args() {
        let cmd = SessionCommand {
            raw: "set input base 16".to_string(),
        };
        assert_eq!(cmd.to_qalc_args(), vec!["-set", "input base 16"]);
    }

    #[test]
    fn session_command_assume_produces_correct_args() {
        let cmd = SessionCommand {
            raw: "/assume positive".to_string(),
        };
        assert_eq!(cmd.to_qalc_args(), vec!["-set", "assumptions positive"]);
    }

    #[test]
    fn rust_subject_reports_unsupported_session_settings() {
        let settings = vec![SessionCommand {
            raw: "set input base 16".to_string(),
        }];
        let out = run_rust_expression("5p10+AEp-2*p23", &settings, Path::new("."), false, false);

        assert_eq!(out.stdout, "");
        assert_eq!(out.exit_code, -1);
        assert!(out.stderr.contains("does not support session settings"));
        assert!(out.stderr.contains("set input base 16"));
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
        assert_eq!(MismatchField::FallbackState.to_string(), "fallback_state");
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
