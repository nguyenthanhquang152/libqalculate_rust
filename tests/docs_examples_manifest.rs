use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const MANIFEST: &str = "docs/examples_manifest.md";
const REQUIRED_IDS: &[&str] = &[
    "README-CLI-001",
    "README-CLI-002",
    "MAN-CLI-SET-001",
    "README-NUMBASE-001",
    "CALCULATOR-API-001",
];

fn manifest_rows(manifest: &str) -> Vec<Vec<String>> {
    manifest
        .lines()
        .filter(|line| line.starts_with("| `"))
        .map(|line| {
            line.trim_matches('|')
                .split('|')
                .map(|column| column.trim().trim_matches('`').to_string())
                .collect::<Vec<_>>()
        })
        .collect()
}

fn source_anchor(source: &str) -> (&str, usize) {
    let (path, line) = source
        .rsplit_once(':')
        .expect("source anchor must use path:line syntax");
    let line = line.parse().expect("source line must be numeric");
    assert!(line > 0, "source line must be one-based");
    (path, line)
}

fn repository_path(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(path)
}

fn assert_owner_links(manifest: &str, owner: &str, id: &str) {
    let mut remainder = owner;
    while let Some(start) = remainder.find("[#") {
        let link = &remainder[start..];
        let end = link
            .find(']')
            .unwrap_or_else(|| panic!("owner link is closed for {id}"));
        let label = &link[..=end];
        assert!(
            manifest.contains(&format!(
                "{label}: https://github.com/nguyenthanhquang152/libqalculate_rust/issues/"
            )),
            "owner link definition is missing for {id}: {label}"
        );
        remainder = &link[end + 1..];
    }
}

#[test]
fn docs_example_manifest_has_traceable_sources_owners_and_statuses() {
    let testing_strategy = fs::read_to_string("docs/testing_strategy.md")
        .expect("testing strategy must remain available");
    assert!(
        testing_strategy.contains("docs/examples_manifest.md"),
        "testing strategy must link the docs example manifest"
    );

    let manifest = fs::read_to_string(MANIFEST).expect("docs example manifest must exist");
    let rows = manifest_rows(&manifest);
    assert!(!rows.is_empty(), "docs example manifest must contain rows");

    let mut ids = HashSet::new();
    for row in &rows {
        assert_eq!(
            row.len(),
            8,
            "manifest row must have eight columns: {row:?}"
        );
        let [id, source, context, _rust_invocation, _requirements, owner, status, evidence] =
            row.as_slice()
        else {
            unreachable!("column count was checked above")
        };

        assert!(ids.insert(id.as_str()), "duplicate manifest id {id}");
        assert!(
            owner.contains('#'),
            "manifest owner must link an issue: {id}"
        );
        assert_owner_links(&manifest, owner, id);
        assert!(
            matches!(status.as_str(), "native-pass" | "pending" | "out-of-scope"),
            "unsupported manifest status {status} for {id}"
        );

        let (source_path, source_line) = source_anchor(source);
        assert!(
            source_path.starts_with("../libqalculate/"),
            "source must point at the adjacent upstream checkout: {id}"
        );
        let source_path = repository_path(source_path);
        if source_path.exists() {
            let source_text =
                fs::read_to_string(&source_path).expect("upstream source is readable");
            let anchored_line = source_text
                .lines()
                .nth(source_line - 1)
                .unwrap_or_else(|| panic!("source line is in range for {id}"));
            assert!(
                anchored_line.contains(context),
                "upstream context for {id} drifted: {anchored_line:?}"
            );
        }

        match status.as_str() {
            "native-pass" => {
                let (evidence_path, symbol) = evidence
                    .split_once("::")
                    .expect("native-pass evidence must use path::symbol");
                let evidence_path = repository_path(evidence_path);
                let evidence_text = fs::read_to_string(&evidence_path)
                    .unwrap_or_else(|_| panic!("evidence path exists for {id}: {evidence_path:?}"));
                assert!(
                    evidence_text.contains(symbol),
                    "evidence symbol for {id} is missing from {evidence_path:?}"
                );
            }
            "pending" => assert_eq!(evidence, "pending", "pending row evidence drifted: {id}"),
            "out-of-scope" => assert!(
                evidence.starts_with("docs/deviations.md::"),
                "out-of-scope rows require an approved rationale: {id}"
            ),
            _ => unreachable!("status allowlist was checked above"),
        }
    }

    for required in REQUIRED_IDS {
        assert!(
            ids.contains(required),
            "missing required example id {required}"
        );
    }
}
