use std::path::{Path, PathBuf};
use std::process::Command;

use libqalculate_rust::batch::read_batch_cases;

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

    let defs_dir = Path::new("../libqalculate/data")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from("../libqalculate/data"));
    let status = Command::new(qalc)
        .env("QALCULATE_DEFINITIONS_DIR", defs_dir)
        .arg("--test-file")
        .arg("../libqalculate/tests/parser.batch")
        .status()
        .expect("upstream qalc oracle should start");
    assert!(status.success(), "upstream qalc rejected parser.batch");
}

fn oracle_binary() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("QALCULATE_ORACLE").map(PathBuf::from) {
        return path.exists().then_some(path);
    }
    let candidate = Path::new("../libqalculate/src/qalc");
    candidate.exists().then(|| candidate.to_path_buf())
}
