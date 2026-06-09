use assert_cmd::Command as CargoCommand;
use libqalculate_rust::batch::read_batch_cases;
use std::path::{Path, PathBuf};
use std::process::Command;

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

// Optional: Test execution against qalc-rs binary once it supports --test-file
// (Disabled/conditionally enabled, or tests that it runs when support is added)
#[test]
#[ignore = "enable this once qalc-rs supports --test-file or native batch evaluation"]
fn test_qalc_rs_executes_batch_files() {
    for path in E2E_BATCH_FILES {
        let mut cmd = CargoCommand::cargo_bin("qalc-rs").expect("qalc-rs binary should build");
        cmd.arg("--test-file").arg(path).assert().success();
    }
}

fn oracle_binary() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("QALCULATE_ORACLE").map(PathBuf::from) {
        return path.exists().then_some(path);
    }
    let candidate = Path::new("../libqalculate/src/qalc");
    candidate.exists().then(|| candidate.to_path_buf())
}
