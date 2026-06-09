use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use libqalculate_rust::batch::is_session_command;

const BATCH_FILES: &[&str] = &[
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

#[test]
fn batch_manifest_case_index_matches_upstream_fixtures() {
    let manifest = fs::read_to_string("docs/batch_manifest.md")
        .expect("docs/batch_manifest.md should be readable");
    let index = extract_case_index(&manifest);
    let actual_ids = manifest_case_ids(index);
    let mut expected_ids = Vec::new();

    for batch_file in BATCH_FILES {
        let path = Path::new("../libqalculate/tests").join(batch_file);
        let input = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        expected_ids.extend(case_ids(batch_file, &input));
    }

    let expected_set = expected_ids.iter().cloned().collect::<BTreeSet<_>>();
    let actual_set = actual_ids.iter().cloned().collect::<BTreeSet<_>>();

    let missing = expected_set
        .difference(&actual_set)
        .cloned()
        .collect::<Vec<_>>();
    let extra = actual_set
        .difference(&expected_set)
        .cloned()
        .collect::<Vec<_>>();

    assert!(
        missing.is_empty() && extra.is_empty(),
        "batch manifest case index differs from upstream fixtures; missing: {missing:?}; extra: {extra:?}"
    );
    assert_eq!(
        actual_ids.len(),
        actual_set.len(),
        "batch manifest case index contains duplicate case IDs"
    );
    assert_eq!(
        expected_ids.len(),
        656,
        "unexpected upstream batch case count"
    );
    assert!(manifest.contains("Run `just manifest-check`"));
}

fn extract_case_index(manifest: &str) -> &str {
    let marker = "## Machine-Readable Case ID Index\n\n```json\n";
    let start = manifest
        .find(marker)
        .expect("manifest should contain machine-readable case index")
        + marker.len();
    let rest = &manifest[start..];
    let end = rest
        .find("\n```")
        .expect("manifest case index should close its JSON fence");
    &rest[..end]
}

fn manifest_case_ids(index: &str) -> Vec<String> {
    index
        .lines()
        .filter_map(|line| {
            let entry = line.trim().trim_end_matches(',');
            let value = entry.strip_prefix('"')?.strip_suffix('"')?;
            value.contains(':').then(|| value.to_owned())
        })
        .collect()
}

fn case_ids(batch_file: &str, input: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut current_expression_line = None;
    let mut has_expected = false;

    for (idx, raw_line) in input.lines().enumerate() {
        let line_number = idx + 1;
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') || is_session_command(trimmed) {
            flush_case_id(
                batch_file,
                &mut ids,
                &mut current_expression_line,
                has_expected,
            );
            has_expected = false;
            continue;
        }

        if line.starts_with('\t') {
            if current_expression_line.is_some() {
                has_expected = true;
            }
            continue;
        }

        flush_case_id(
            batch_file,
            &mut ids,
            &mut current_expression_line,
            has_expected,
        );
        current_expression_line = Some(line_number);
        has_expected = false;
    }

    flush_case_id(
        batch_file,
        &mut ids,
        &mut current_expression_line,
        has_expected,
    );
    ids
}

fn flush_case_id(
    batch_file: &str,
    ids: &mut Vec<String>,
    current_expression_line: &mut Option<usize>,
    has_expected: bool,
) {
    if let Some(line) = current_expression_line.take() {
        if has_expected {
            ids.push(format!("{batch_file}:{line}"));
        }
    }
}
