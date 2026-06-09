use std::path::Path;

use libqalculate_rust::batch::read_batch_cases;
use libqalculate_rust::UPSTREAM_LIBQALCULATE_VERSION;

#[test]
fn crate_exposes_upstream_version() {
    assert_eq!(UPSTREAM_LIBQALCULATE_VERSION, "5.11.0");
}

#[test]
fn upstream_parser_fixture_is_readable() {
    let path = Path::new("../libqalculate/tests/parser.batch");
    if !path.exists() {
        eprintln!(
            "skipping upstream parser fixture smoke test; {} is unavailable",
            path.display()
        );
        return;
    }

    let cases = read_batch_cases(path).expect("parser.batch should be readable");
    assert!(
        cases.len() > 20,
        "expected parser.batch to provide broad smoke coverage"
    );
}
