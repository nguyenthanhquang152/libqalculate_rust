use assert_cmd::Command as CargoCommand;
use libqalculate_rust::batch::{
    parse_batch_cases_with_source_lines, read_batch_cases, render_batch_cases,
};
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

const STATEFUL_BATCH: &str = "tests/fixtures/batch/stateful_base.batch";
const PARSER_BATCH_NATIVE_CASE_IDS: &[&str] = &[
    "parser.batch:1",
    "parser.batch:3",
    "parser.batch:5",
    "parser.batch:7",
    "parser.batch:9",
];

const E2E_BATCH_FILES: &[&str] = &[
    "tests/fixtures/e2e/tier1_feature_coverage.batch",
    "tests/fixtures/e2e/tier2_boundary_corner.batch",
    "tests/fixtures/e2e/tier3_cross_feature.batch",
    "tests/fixtures/e2e/tier4_real_world.batch",
];

#[test]
fn test_e2e_batch_files_exist_and_parse() {
    for path in E2E_BATCH_FILES {
        let cases = read_batch_cases(path)
            .unwrap_or_else(|e| panic!("Failed to parse E2E batch file {path}: {e}"));
        assert!(!cases.is_empty(), "E2E batch file {path} has no cases");

        for case in &cases {
            assert!(
                !case.expression.is_empty(),
                "Case in {path} has empty expression"
            );
            assert!(
                !case.expected.is_empty(),
                "Case in {path} has empty expected vector"
            );
            for val in &case.expected {
                assert!(!val.is_empty(), "Case in {path} has empty expected value");
            }
        }
    }
}

#[test]
fn test_qalc_rs_can_parse_e2e_batch_files() {
    for path in E2E_BATCH_FILES {
        let mut cmd = CargoCommand::cargo_bin("qalc-rs").expect("qalc-rs binary should build");
        let assert_res = cmd.arg("--parse-batch").arg(path).assert().success();
        let stdout_str = String::from_utf8(assert_res.get_output().stdout.clone())
            .expect("invalid utf-8 output");
        let cases_line = stdout_str
            .lines()
            .find(|line| line.starts_with("cases="))
            .expect("output should contain 'cases='");
        let count_str = &cases_line["cases=".len()..];
        let count: usize = count_str
            .parse()
            .expect("cases count should be a valid positive integer");
        assert!(
            count > 0,
            "batch file {path} should contain at least 1 case"
        );
    }
}

#[test]
fn test_e2e_batch_files_validated_by_cpp_oracle() {
    let Some(qalc) = oracle_binary() else {
        eprintln!("skipping E2E oracle validation; set QALCULATE_ORACLE or build upstream qalc");
        return;
    };

    for path in E2E_BATCH_FILES {
        let defs_dir = Path::new("../libqalculate/data")
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from("../libqalculate/data"));
        let status = Command::new(&qalc)
            .env("QALCULATE_DEFINITIONS_DIR", defs_dir)
            .arg("-defaults")
            .arg("-set")
            .arg("decimal comma 0")
            .arg("-set")
            .arg("curconv 0")
            .arg("--test-file")
            .arg(path)
            .status()
            .expect("upstream qalc oracle should start");
        assert!(
            status.success(),
            "upstream qalc rejected E2E batch file: {}",
            path
        );
    }
}

#[test]
fn test_qalc_rs_executes_stateful_batch_file_without_fallback() {
    let mut cmd = CargoCommand::cargo_bin("qalc-rs").expect("qalc-rs binary should build");
    cmd.arg("--test-file")
        .arg(STATEFUL_BATCH)
        .env("QALCULATE_DEFINITIONS_DIR", "../libqalculate/data")
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .assert()
        .success()
        .stdout(format!(
            "\x1B[32m\n{STATEFUL_BATCH} - 2 tests passed\n\n\x1B[0m"
        ))
        .stderr("");
}

#[test]
fn test_qalc_rs_batch_success_matches_upstream_exactly() {
    let Some(qalc) = oracle_binary() else {
        eprintln!("skipping batch-mode differential; upstream qalc is unavailable");
        return;
    };

    let mut rust = CargoCommand::cargo_bin("qalc-rs").expect("qalc-rs binary should build");
    let rust_output = rust
        .arg("--test-file")
        .arg(STATEFUL_BATCH)
        .env("QALCULATE_DEFINITIONS_DIR", "../libqalculate/data")
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .assert()
        .get_output()
        .clone();

    let home = tempdir().expect("isolated upstream home");
    let upstream_output = Command::new(qalc)
        .arg("--test-file")
        .arg(STATEFUL_BATCH)
        .env("HOME", home.path())
        .env("QALCULATE_DEFINITIONS_DIR", "../libqalculate/data")
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("TZ", "UTC")
        .output()
        .expect("upstream qalc should run");

    assert_eq!(rust_output.status.code(), upstream_output.status.code());
    assert_eq!(rust_output.stdout, upstream_output.stdout);
    assert_eq!(rust_output.stderr, upstream_output.stderr);
}

#[test]
fn test_qalc_rs_executes_upstream_parser_native_subset_without_fallback() {
    let Some(qalc) = oracle_binary() else {
        eprintln!("skipping parser.batch subset differential; upstream qalc is unavailable");
        return;
    };

    let source = std::fs::read_to_string("../libqalculate/tests/parser.batch")
        .expect("upstream parser.batch should be readable");
    let selected = parse_batch_cases_with_source_lines(&source)
        .expect("upstream parser.batch should parse")
        .into_iter()
        .filter(|case| {
            let case_id = format!("parser.batch:{}", case.source_line);
            PARSER_BATCH_NATIVE_CASE_IDS.contains(&case_id.as_str())
        })
        .collect::<Vec<_>>();
    let selected_ids = selected
        .iter()
        .map(|case| format!("parser.batch:{}", case.source_line))
        .collect::<Vec<_>>();
    assert_eq!(selected_ids, PARSER_BATCH_NATIVE_CASE_IDS);

    let fixture_dir = tempdir().expect("temporary fixture directory");
    let fixture = fixture_dir.path().join("parser-native-subset.batch");
    let cases = selected
        .into_iter()
        .map(|located| located.case)
        .collect::<Vec<_>>();
    std::fs::write(&fixture, render_batch_cases(&cases)).expect("write parser subset fixture");

    let mut rust = CargoCommand::cargo_bin("qalc-rs").expect("qalc-rs binary should build");
    let rust_output = rust
        .arg("--test-file")
        .arg(&fixture)
        .env("QALCULATE_DEFINITIONS_DIR", "../libqalculate/data")
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .assert()
        .get_output()
        .clone();

    let home = tempdir().expect("isolated upstream home");
    let upstream_output = Command::new(qalc)
        .arg("--test-file")
        .arg(&fixture)
        .env("HOME", home.path())
        .env("QALCULATE_DEFINITIONS_DIR", "../libqalculate/data")
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("TZ", "UTC")
        .output()
        .expect("upstream qalc should run");

    assert_eq!(
        rust_output.status.code(),
        upstream_output.status.code(),
        "selected cases: {PARSER_BATCH_NATIVE_CASE_IDS:?}"
    );
    assert_eq!(
        rust_output.stdout, upstream_output.stdout,
        "selected cases: {PARSER_BATCH_NATIVE_CASE_IDS:?}"
    );
    assert_eq!(
        rust_output.stderr, upstream_output.stderr,
        "selected cases: {PARSER_BATCH_NATIVE_CASE_IDS:?}"
    );
}

#[test]
fn test_qalc_rs_reports_first_batch_mismatch_and_case_line() {
    let fixture_dir = tempdir().expect("temporary fixture directory");
    let fixture = fixture_dir.path().join("mismatch.batch");
    std::fs::write(&fixture, "/set base 16\n15\n\t0xE\n").expect("write mismatch fixture");

    let mut cmd = CargoCommand::cargo_bin("qalc-rs").expect("qalc-rs binary should build");
    cmd.arg("--test-file")
        .arg(&fixture)
        .env("QALCULATE_DEFINITIONS_DIR", "../libqalculate/data")
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .assert()
        .code(1)
        .stdout(
            "\x1B[31m\nMismatch detected at line 3\n15\nexpected '0xE'\nreceived '0xF'\n\n\x1B[0m",
        )
        .stderr("");
}

#[test]
fn test_qalc_rs_reports_the_first_differing_multiline_expected_line() {
    let fixture_dir = tempdir().expect("temporary fixture directory");
    let fixture = fixture_dir.path().join("multiline-mismatch.batch");
    std::fs::write(&fixture, "1\n\t1\n\t2\n").expect("write multiline mismatch fixture");

    let mut cmd = CargoCommand::cargo_bin("qalc-rs").expect("qalc-rs binary should build");
    cmd.arg("--test-file")
        .arg(&fixture)
        .env("QALCULATE_DEFINITIONS_DIR", "../libqalculate/data")
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .assert()
        .code(1)
        .stdout(
            "\x1B[31m\nMismatch detected at line 3\n1\nexpected '1\n2'\nreceived '1'\n\n\x1B[0m",
        )
        .stderr("");
}

#[test]
fn test_qalc_rs_executes_unasserted_setup_and_cleanup() {
    let fixture_dir = tempdir().expect("temporary fixture directory");
    let fixture = fixture_dir.path().join("setup.batch");
    std::fs::write(&fixture, "x:=2\nx+1\n\t3\ndelete x\n").expect("write setup fixture");

    let mut cmd = CargoCommand::cargo_bin("qalc-rs").expect("qalc-rs binary should build");
    cmd.arg("--test-file")
        .arg(&fixture)
        .env("QALCULATE_DEFINITIONS_DIR", "../libqalculate/data")
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .assert()
        .success()
        .stdout(format!(
            "\x1B[32m\n{} - 1 tests passed\n\n\x1B[0m",
            fixture.display()
        ))
        .stderr("");
}

#[test]
fn test_qalc_rs_warns_when_batch_has_no_asserted_cases() {
    let fixture_dir = tempdir().expect("temporary fixture directory");
    let fixture = fixture_dir.path().join("empty.batch");
    std::fs::write(&fixture, "/set base 16\n").expect("write empty fixture");

    let mut cmd = CargoCommand::cargo_bin("qalc-rs").expect("qalc-rs binary should build");
    cmd.arg("--test-file")
        .arg(&fixture)
        .env("QALCULATE_DEFINITIONS_DIR", "../libqalculate/data")
        .env("QALCULATE_DISABLE_FALLBACK", "1")
        .assert()
        .success()
        .stdout(
            "\x1B[31m\nWARNING: 0 tests were run (indentation needs to be tab-based)\n\n\x1B[0m",
        )
        .stderr("");
}

fn oracle_binary() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("QALCULATE_ORACLE").map(PathBuf::from) {
        return path.exists().then_some(path);
    }
    let candidate = Path::new("../libqalculate/src/qalc");
    candidate.exists().then(|| candidate.to_path_buf())
}
