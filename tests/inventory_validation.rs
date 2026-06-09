//! Automated validation of `docs/compatibility_inventory.md`.
//!
//! This integration test reads the compatibility inventory and verifies that
//! it contains entries for every expected upstream asset and core class,
//! each with a valid status classification.

use std::fs;
use std::path::Path;

/// Valid status classifications for inventory entries.
const VALID_STATUSES: &[&str] = &[
    "unstarted",
    "scaffold",
    "native-pass",
    "fallback-only",
    "approved-deviation",
    "out-of-scope",
];

/// All 9 definition data files that must appear in the inventory.
const DEFINITION_DATA_FILES: &[&str] = &[
    "currencies",
    "datasets",
    "elements",
    "functions",
    "planets",
    "prefixes",
    "units",
    "variables",
    "rates",
];

/// All 17 upstream batch test files that must appear in the inventory.
const BATCH_FILES: &[&str] = &[
    "bitwise",
    "calculus",
    "dates",
    "explog",
    "geometry",
    "limits",
    "matrixvector",
    "numberbase",
    "operators",
    "parser",
    "percentages",
    "polynomial",
    "solver",
    "stats",
    "strings",
    "units",
    "variables",
];

/// All 9 core class API sections that must appear in the inventory.
const CORE_CLASSES: &[&str] = &[
    "Calculator",
    "MathStructure",
    "Number",
    "ExpressionItem",
    "Variable",
    "Function",
    "DataSet",
    "Unit",
    "Prefix",
];

const SUPPORTING_UPSTREAM_FILES: &[&str] = &[
    "../libqalculate/src/qalc.cc",
    "../libqalculate/src/test.cc",
    "../libqalculate/src/unittest.cc",
    "../libqalculate/libqalculate/Makefile.am",
    "../libqalculate/src/Makefile.am",
    "../libqalculate/tests/Makefile.am",
    "../libqalculate/data/Makefile.am",
];

const CSV_ASSETS: &[&str] = &["vectordata.csv", "vectordata2.csv"];

#[test]
fn inventory_file_exists_and_nonempty() {
    let path = Path::new("docs/compatibility_inventory.md");
    assert!(
        path.exists(),
        "docs/compatibility_inventory.md does not exist"
    );

    let content = fs::read_to_string(path).expect("Failed to read compatibility_inventory.md");
    assert!(
        !content.trim().is_empty(),
        "docs/compatibility_inventory.md is empty"
    );
}

#[test]
fn inventory_contains_all_definition_data_files() {
    let content = fs::read_to_string("docs/compatibility_inventory.md")
        .expect("Failed to read compatibility_inventory.md");

    let mut missing = Vec::new();
    for name in DEFINITION_DATA_FILES {
        // Data files appear as either "name.xml.in" or "name.json" in the inventory
        let has_xml = content.contains(&format!("{}.xml.in", name));
        let has_json = content.contains(&format!("{}.json", name));
        if !has_xml && !has_json {
            missing.push(*name);
        }
    }

    assert!(
        missing.is_empty(),
        "Missing definition data file entries in inventory: {:?}",
        missing
    );
}

#[test]
fn inventory_contains_every_current_upstream_header() {
    let content = fs::read_to_string("docs/compatibility_inventory.md")
        .expect("Failed to read compatibility_inventory.md");
    let headers = upstream_filenames("../libqalculate/libqalculate", "h");

    assert_eq!(headers.len(), 22, "unexpected upstream public header count");
    assert_all_present(&content, &headers, "header");
}

#[test]
fn inventory_contains_every_current_upstream_implementation_file() {
    let content = fs::read_to_string("docs/compatibility_inventory.md")
        .expect("Failed to read compatibility_inventory.md");
    let implementations = upstream_filenames("../libqalculate/libqalculate", "cc");

    assert_eq!(
        implementations.len(),
        41,
        "unexpected upstream implementation file count"
    );
    assert_all_present(&content, &implementations, "implementation file");
    assert!(
        !content.contains("BuiltinFunctions-unit.cc"),
        "inventory contains stale implementation file BuiltinFunctions-unit.cc"
    );
}

#[test]
fn inventory_contains_supporting_cli_test_and_makefile_sources() {
    let content = fs::read_to_string("docs/compatibility_inventory.md")
        .expect("Failed to read compatibility_inventory.md");

    assert_all_present(
        &content,
        SUPPORTING_UPSTREAM_FILES,
        "supporting upstream file",
    );
}

#[test]
fn inventory_contains_all_batch_files() {
    let content = fs::read_to_string("docs/compatibility_inventory.md")
        .expect("Failed to read compatibility_inventory.md");

    let mut missing = Vec::new();
    for name in BATCH_FILES {
        // Batch files appear as "name.batch" in the inventory
        let pattern = format!("{}.batch", name);
        if !content.contains(&pattern) {
            missing.push(*name);
        }
    }

    assert!(
        missing.is_empty(),
        "Missing batch file entries in inventory: {:?}",
        missing
    );
}

#[test]
fn inventory_contains_all_csv_assets() {
    let content = fs::read_to_string("docs/compatibility_inventory.md")
        .expect("Failed to read compatibility_inventory.md");

    assert_all_present(&content, CSV_ASSETS, "CSV asset");
}

#[test]
fn inventory_contains_all_core_class_sections() {
    let content = fs::read_to_string("docs/compatibility_inventory.md")
        .expect("Failed to read compatibility_inventory.md");

    let mut missing = Vec::new();
    for class_name in CORE_CLASSES {
        // Core classes appear in header references like "Calculator.h" or
        // in descriptions/purpose columns. We check for the class name
        // appearing with its canonical .h header form.
        let header_pattern = format!("{}.h", class_name);
        // Also check for the class name itself in case it appears in a
        // purpose or module column without the .h suffix
        if !content.contains(&header_pattern) && !content.contains(class_name) {
            missing.push(*class_name);
        }
    }

    assert!(
        missing.is_empty(),
        "Missing core class API sections in inventory: {:?}",
        missing
    );
}

#[test]
fn inventory_entries_have_valid_status() {
    let content = fs::read_to_string("docs/compatibility_inventory.md")
        .expect("Failed to read compatibility_inventory.md");

    // Check that numbered inventory entry rows (e.g. "| 1 |", "| 22 |")
    // each contain a valid status classification. Summary tables, header
    // rows, and metadata rows are intentionally skipped.
    let mut entries_without_status = Vec::new();
    let mut total_data_rows = 0;

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip non-table lines
        if !trimmed.starts_with('|') {
            continue;
        }

        // Skip separator rows (e.g., "| :--- | :--- |")
        let cells: Vec<&str> = trimmed.split('|').collect();
        let non_empty_cells: Vec<&str> = cells
            .iter()
            .map(|c| c.trim())
            .filter(|c| !c.is_empty())
            .collect();
        if non_empty_cells
            .iter()
            .all(|c| c.chars().all(|ch| ch == '-' || ch == ':' || ch == ' '))
        {
            continue;
        }

        // Only check rows whose first non-empty cell is a number — these are
        // the numbered inventory entries that must carry a status value.
        // Summary tables (e.g. "| Category | Total | ...") and header rows
        // are not numbered and are skipped.
        let first_cell = non_empty_cells.first().copied().unwrap_or("");
        if first_cell.parse::<usize>().is_err() {
            continue;
        }

        total_data_rows += 1;

        // Check if the row contains a valid backtick-quoted status
        let has_valid_status = VALID_STATUSES.iter().any(|status| {
            let backtick_pattern = format!("`{}`", status);
            trimmed.contains(&backtick_pattern)
        });

        if !has_valid_status {
            entries_without_status.push(trimmed.to_string());
        }
    }

    assert!(
        total_data_rows > 0,
        "No numbered data rows found in compatibility inventory tables"
    );

    assert!(
        entries_without_status.is_empty(),
        "Found {} inventory entries without a valid status classification \
         (expected one of {:?}):\n{}",
        entries_without_status.len(),
        VALID_STATUSES,
        entries_without_status
            .iter()
            .map(|e| format!("  - {}", e))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

fn upstream_filenames(dir: &str, extension: &str) -> Vec<String> {
    let mut files = fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("failed to read {dir}: {error}"))
        .map(|entry| {
            entry
                .unwrap_or_else(|error| panic!("failed to read entry in {dir}: {error}"))
                .path()
        })
        .filter(|path| path.extension().is_some_and(|actual| actual == extension))
        .map(|path| {
            path.file_name()
                .expect("upstream path should have a filename")
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    files.sort();
    files
}

fn assert_all_present(content: &str, expected: &[impl AsRef<str>], label: &str) {
    let missing = expected
        .iter()
        .map(AsRef::as_ref)
        .filter(|name| !content.contains(*name))
        .collect::<Vec<_>>();

    assert!(
        missing.is_empty(),
        "missing {label} entries in compatibility inventory: {missing:?}"
    );
}
