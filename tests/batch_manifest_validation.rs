use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use libqalculate_rust::batch::batch_case_ids;

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

const REQUIRED_CASE_FIELDS: &[&str] = &[
    "case_id",
    "source_file",
    "source_line",
    "feature_tags",
    "input_kind",
    "required_assets",
    "required_settings",
    "expected_status",
    "normalization",
    "deviation_id",
    "parity_status",
    "owner",
    "last_checked_upstream_version",
];

const ALLOWED_PARITY_STATUSES: &[&str] = &[
    "inventory-only",
    "fallback-only",
    "native-pass",
    "approved-deviation",
    "out-of-scope",
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
        expected_ids.extend(
            batch_case_ids(batch_file, &input).unwrap_or_else(|error| {
                panic!("failed to parse case IDs for {batch_file}: {error}")
            }),
        );
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

#[test]
fn batch_manifest_declares_required_case_schema() {
    let manifest = fs::read_to_string("docs/batch_manifest.md")
        .expect("docs/batch_manifest.md should be readable");

    for field in REQUIRED_CASE_FIELDS {
        assert!(
            manifest.contains(&format!("`{field}`")),
            "batch manifest should document required field `{field}`"
        );
    }
    assert!(manifest.contains("Allowed `parity_status` values"));
    assert!(manifest.contains("5.11.0"));
}

#[test]
fn batch_manifest_case_rows_use_explicit_parity_statuses() {
    let manifest = fs::read_to_string("docs/batch_manifest.md")
        .expect("docs/batch_manifest.md should be readable");
    let mut invalid_rows = Vec::new();
    let mut case_rows = 0usize;

    for line in manifest.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') || !trimmed.contains(".batch:") {
            continue;
        }

        case_rows += 1;
        let cells = trimmed
            .split('|')
            .map(str::trim)
            .filter(|cell| !cell.is_empty())
            .collect::<Vec<_>>();
        let status = cells.last().copied().unwrap_or_default();
        if !ALLOWED_PARITY_STATUSES.contains(&status) {
            invalid_rows.push(trimmed.to_string());
        }
    }

    assert_eq!(case_rows, 656, "unexpected manifest case row count");
    assert!(
        invalid_rows.is_empty(),
        "case rows with invalid parity status: {invalid_rows:?}"
    );
    assert!(
        !manifest.contains("untested"),
        "batch manifest should use explicit parity statuses, not `untested`"
    );
}

#[test]
fn completed_matrixvector_manifest_is_reflected_in_compatibility_inventory() {
    let manifest = fs::read_to_string("docs/batch_manifest.md")
        .expect("docs/batch_manifest.md should be readable");
    let matrixvector_rows = manifest
        .lines()
        .filter(|line| line.contains("`matrixvector.batch:"))
        .collect::<Vec<_>>();

    assert_eq!(
        matrixvector_rows.len(),
        130,
        "unexpected matrixvector.batch manifest case count"
    );
    assert!(
        matrixvector_rows
            .iter()
            .all(|line| line.trim_end().ends_with("| native-pass |")),
        "all matrixvector.batch rows should be native-pass before the compatibility inventory marks the file complete"
    );

    let inventory = fs::read_to_string("docs/compatibility_inventory.md")
        .expect("docs/compatibility_inventory.md should be readable");
    assert!(
        inventory.contains("| Batch Test Files | 17 | 4 | 0 | 4 | 0 | 9 | 0 |"),
        "compatibility inventory summary should count matrixvector.batch as a native-pass batch file"
    );
    assert!(
        inventory.contains("| 7 | `matrixvector.batch` | 130 | 0 | — | `native-pass` |"),
        "compatibility inventory should mark matrixvector.batch native-pass when every manifest row is native-pass"
    );
}

#[test]
fn batch_manifest_retains_session_commands_and_assets() {
    let manifest = fs::read_to_string("docs/batch_manifest.md")
        .expect("docs/batch_manifest.md should be readable");

    for required in [
        "/set unicode 1",
        "set input base 16",
        "/assume positive",
        "vectordata.csv",
        "vectordata2.csv",
    ] {
        assert!(
            manifest.contains(required),
            "batch manifest should retain required setting or asset `{required}`"
        );
    }
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
