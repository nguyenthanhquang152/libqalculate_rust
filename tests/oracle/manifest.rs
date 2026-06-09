use std::collections::HashMap;
use std::path::Path;

use libqalculate_rust::batch::is_session_command;

pub(super) fn load_parity_statuses() -> HashMap<String, String> {
    let manifest_path = Path::new("docs/batch_manifest.md");
    let content = std::fs::read_to_string(manifest_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", manifest_path.display()));

    let mut map = HashMap::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') || !trimmed.contains(".batch:") {
            continue;
        }
        let cells = trimmed
            .split('|')
            .map(str::trim)
            .filter(|cell| !cell.is_empty())
            .collect::<Vec<_>>();
        if cells.len() >= 6 {
            let case_id = cells[1].replace('`', "");
            let status = cells[5].replace('`', "");
            map.insert(case_id, status);
        }
    }
    map
}

pub(super) fn case_ids(batch_file: &str, input: &str) -> Vec<String> {
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
