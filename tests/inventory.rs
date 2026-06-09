use std::fs;
use std::path::PathBuf;

#[test]
fn test_compatibility_inventory_validation() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let inventory_path = PathBuf::from(manifest_dir).join("docs/compatibility_inventory.md");
    let content = fs::read_to_string(&inventory_path)
        .expect("Failed to read docs/compatibility_inventory.md");

    // Split file into lines
    let lines: Vec<&str> = content.lines().collect();

    // Group lines into tables
    // Contiguous lines starting with '|' form a table
    let mut tables: Vec<Vec<Vec<String>>> = Vec::new();
    let mut current_table: Vec<Vec<String>> = Vec::new();

    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with('|') && trimmed.ends_with('|') {
            // Parse row
            let parts: Vec<String> = trimmed.split('|').map(|s| s.trim().to_string()).collect();
            if parts.len() >= 2 {
                let row_cols: Vec<String> = parts[1..parts.len() - 1].to_vec();
                current_table.push(row_cols);
            }
        } else if !current_table.is_empty() {
            tables.push(current_table);
            current_table = Vec::new();
        }
    }
    if !current_table.is_empty() {
        tables.push(current_table);
    }

    // The comprehensive inventory has many more tables than the original 4
    // (summary, headers, implementation families, batch fixtures, CLI,
    // 9 core class API tables, appendices, etc.)
    assert!(
        tables.len() >= 4,
        "Expected at least 4 tables in compatibility_inventory.md, found {}",
        tables.len()
    );

    // Validate that every table has proper structure (header + separator + data rows)
    for (t_idx, table) in tables.iter().enumerate() {
        assert!(
            table.len() >= 2,
            "Table {} must have at least header and separator rows, has {}",
            t_idx + 1,
            table.len()
        );

        // Skip tables that are purely summary/statistics (may not have status columns)
        // Only validate tables with 4+ columns that likely have a status field
        if table[0].len() < 4 {
            continue;
        }

        let data_rows = &table[2..];
        for (r_idx, row) in data_rows.iter().enumerate() {
            // Check for TBD placeholders in any column
            for (c_idx, col) in row.iter().enumerate() {
                let col_lower = col.to_lowercase();
                assert!(
                    !col_lower.contains("tbd"),
                    "Table {}, Row {}, Col {} (first col: '{}'): Column contains placeholder 'TBD': '{}'",
                    t_idx + 1,
                    r_idx + 3,
                    c_idx + 1,
                    row.first().map(|s| s.as_str()).unwrap_or("?"),
                    col
                );
            }
        }
    }
}
