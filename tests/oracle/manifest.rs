use std::collections::HashMap;
use std::path::Path;

const ALLOWED_PARITY_STATUSES: &[&str] = &[
    "inventory-only",
    "fallback-only",
    "native-pass",
    "approved-deviation",
    "out-of-scope",
];

#[derive(Clone, Copy)]
struct CaseTableColumns {
    case_id: usize,
    status: usize,
}

pub(super) fn load_parity_statuses() -> HashMap<String, String> {
    let manifest_path = Path::new("docs/batch_manifest.md");
    let content = std::fs::read_to_string(manifest_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", manifest_path.display()));

    let mut map = HashMap::new();
    let mut columns = None;
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            continue;
        }

        let cells = markdown_table_cells(trimmed);
        if let Some(header_columns) = case_table_columns(&cells) {
            columns = Some(header_columns);
            continue;
        }
        if !trimmed.contains(".batch:") {
            continue;
        }

        let columns = columns
            .unwrap_or_else(|| panic!("manifest case row appeared before header: {trimmed}"));
        let case_id = clean_markdown_cell(
            cells
                .get(columns.case_id)
                .unwrap_or_else(|| panic!("manifest case row is missing case_id: {trimmed}")),
        );
        let status = clean_markdown_cell(
            cells
                .get(columns.status)
                .unwrap_or_else(|| panic!("manifest case row is missing status: {trimmed}")),
        );

        if !ALLOWED_PARITY_STATUSES.contains(&status.as_str()) {
            panic!("invalid parity status {status:?} for {case_id}");
        }
        if map.insert(case_id.clone(), status).is_some() {
            panic!("duplicate parity status for {case_id}");
        }
    }

    assert!(
        !map.is_empty(),
        "docs/batch_manifest.md did not contain any case parity rows"
    );
    map
}

pub(super) fn status_for_case<'a>(statuses: &'a HashMap<String, String>, case_id: &str) -> &'a str {
    statuses
        .get(case_id)
        .unwrap_or_else(|| panic!("missing parity status for {case_id}"))
        .as_str()
}

fn case_table_columns(cells: &[String]) -> Option<CaseTableColumns> {
    let case_id = cells
        .iter()
        .position(|cell| clean_markdown_cell(cell) == "case_id")?;
    let status = cells
        .iter()
        .position(|cell| clean_markdown_cell(cell) == "Status")?;
    Some(CaseTableColumns { case_id, status })
}

fn markdown_table_cells(row: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut cell = String::new();
    let mut escaped = false;

    for ch in row.trim().chars() {
        if escaped {
            if ch == '|' {
                cell.push('|');
            } else {
                cell.push('\\');
                cell.push(ch);
            }
            escaped = false;
            continue;
        }

        match ch {
            '\\' => escaped = true,
            '|' => {
                cells.push(cell.trim().to_string());
                cell.clear();
            }
            _ => cell.push(ch),
        }
    }
    if escaped {
        cell.push('\\');
    }
    cells.push(cell.trim().to_string());

    cells.into_iter().filter(|cell| !cell.is_empty()).collect()
}

fn clean_markdown_cell(cell: &str) -> String {
    cell.trim()
        .strip_prefix('`')
        .and_then(|cell| cell.strip_suffix('`'))
        .unwrap_or_else(|| cell.trim())
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn parity_statuses_handle_escaped_markdown_pipes() {
        let statuses = load_parity_statuses();

        assert_eq!(
            status_for_case(&statuses, "bitwise.batch:40"),
            "native-pass"
        );
        assert_eq!(
            status_for_case(&statuses, "bitwise.batch:61"),
            "native-pass"
        );
    }

    #[test]
    #[should_panic(expected = "missing parity status for missing.batch:1")]
    fn parity_status_lookup_fails_closed_for_unknown_case() {
        let statuses = HashMap::new();

        let _ = status_for_case(&statuses, "missing.batch:1");
    }
}
