use assert_cmd::Command;
use predicates::prelude::*;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn cli_prints_version() {
    let mut cmd = Command::cargo_bin("qalc-rs").expect("binary should build");
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("upstream libqalculate 5.11.0"));
}

#[test]
fn cli_prints_help() {
    let mut cmd = Command::cargo_bin("qalc-rs").expect("binary should build");
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--parse-batch <path>"));
}

#[test]
fn cli_self_check_finds_upstream_fixtures() {
    if !Path::new("../libqalculate/tests").exists() {
        eprintln!("skipping upstream fixture e2e test; ../libqalculate/tests is unavailable");
        return;
    }

    let mut cmd = Command::cargo_bin("qalc-rs").expect("binary should build");
    cmd.arg("--self-check")
        .assert()
        .success()
        .stdout(predicate::str::contains("upstream_batch_files="));
}

#[test]
fn cli_self_check_uses_configured_upstream_dir() {
    let upstream = fake_upstream();
    let mut cmd = Command::cargo_bin("qalc-rs").expect("binary should build");
    cmd.arg("--self-check")
        .env("LIBQALCULATE_UPSTREAM_DIR", upstream.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("upstream_batch_files=1"));
}

#[test]
fn cli_lists_only_batch_fixtures_from_configured_upstream_dir() {
    let upstream = fake_upstream();
    let mut cmd = Command::cargo_bin("qalc-rs").expect("binary should build");
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
    let mut cmd = Command::cargo_bin("qalc-rs").expect("binary should build");
    cmd.arg("--parse-batch")
        .arg("tests/fixtures/regression/basic_numbers.batch")
        .assert()
        .success()
        .stdout(predicate::str::contains("cases=4"));
}

#[test]
fn cli_rejects_unknown_arguments() {
    let mut cmd = Command::cargo_bin("qalc-rs").expect("binary should build");
    cmd.arg("--definitely-unknown")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown argument"));
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
