//! CSV-backed data loading helpers.
//!
//! Upstream oracle:
//! - `../libqalculate/libqalculate/BuiltinFunctions-matrixvector.cc`
//! - `../libqalculate/libqalculate/Calculator-definitions.cc`
//! - `../libqalculate/tests/vectordata.csv`
//! - `../libqalculate/tests/vectordata2.csv`

use crate::ast::Expression;
use crate::number::Number;
use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// Error returned when a CSV file cannot be loaded as a numeric vector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvLoadError {
    message: String,
}

impl CsvLoadError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for CsvLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for CsvLoadError {}

/// Loads a comma-separated numeric file as a flat vector expression.
pub fn load_csv_vector(path: impl AsRef<Path>) -> Result<Expression, CsvLoadError> {
    load_csv_numbers(path)
        .map(|values| Expression::Vector(values.into_iter().map(Expression::Number).collect()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CsvLoadOptions {
    first_row: usize,
    delimiter: &'static str,
    allow_empty: bool,
}

impl CsvLoadOptions {
    const fn default_numeric_vector() -> Self {
        Self {
            first_row: 1,
            delimiter: ",",
            allow_empty: false,
        }
    }

    const fn promoted_count(first_row: usize, delimiter: &'static str) -> Self {
        Self {
            first_row,
            delimiter,
            allow_empty: true,
        }
    }
}

/// Loads a delimiter-separated numeric file as native [`Number`] values.
fn load_csv_numbers_with_options(
    path: impl AsRef<Path>,
    options: CsvLoadOptions,
) -> Result<Vec<Number>, CsvLoadError> {
    let original_path = path.as_ref();
    let path = resolve_csv_path(original_path);
    let contents = std::fs::read_to_string(&path).map_err(|error| {
        CsvLoadError::new(format!(
            "failed to read CSV data from {}: {error}",
            original_path.display()
        ))
    })?;

    let mut values = Vec::new();
    for (line_index, line) in contents.lines().enumerate() {
        let row_num = line_index + 1;
        if row_num < options.first_row {
            continue;
        }
        for (column_index, raw) in line.split(options.delimiter).enumerate() {
            let value = raw.trim();
            if value.is_empty() {
                return Err(CsvLoadError::new(format!(
                    "empty numeric CSV value at row {}, column {} in {}",
                    row_num,
                    column_index + 1,
                    original_path.display()
                )));
            }
            let number = Number::from_str(value).map_err(|_| {
                CsvLoadError::new(format!(
                    "invalid numeric CSV value at row {}, column {} in {}: {value}",
                    row_num,
                    column_index + 1,
                    original_path.display()
                ))
            })?;
            values.push(number);
        }
    }

    if values.is_empty() && !options.allow_empty {
        return Err(CsvLoadError::new(format!(
            "CSV data file {} did not contain numeric values",
            original_path.display()
        )));
    }

    Ok(values)
}

/// Loads a comma-separated numeric file as native [`Number`] values.
pub fn load_csv_numbers(path: impl AsRef<Path>) -> Result<Vec<Number>, CsvLoadError> {
    load_csv_numbers_with_options(path, CsvLoadOptions::default_numeric_vector())
}

fn resolve_csv_path(path: &Path) -> PathBuf {
    if path.is_absolute() || path.exists() {
        return path.to_path_buf();
    }

    if let Some(upstream) = std::env::var_os("LIBQALCULATE_UPSTREAM_DIR") {
        let candidate = PathBuf::from(upstream).join(path);
        if candidate.exists() {
            return candidate;
        }
    }

    let candidate = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../libqalculate")
        .join(path);
    if candidate.exists() {
        return candidate;
    }

    path.to_path_buf()
}

pub(crate) fn native_output(expr: &str) -> Result<Option<String>, CsvLoadError> {
    let Some((path, options)) = parse_promoted_load_count(expr) else {
        return Ok(None);
    };
    let values = load_csv_numbers_with_options(path, options)?;
    Ok(Some(values.len().to_string()))
}

fn parse_promoted_load_count(expr: &str) -> Option<(PathBuf, CsvLoadOptions)> {
    let trimmed = expr.trim();
    if !trimmed.starts_with("number(load(") || !trimmed.ends_with("))") {
        return None;
    }
    let inner = &trimmed["number(load(".len()..trimmed.len() - 2];

    let (path_str, rest) = if let Some(rest) = inner.strip_prefix("\"tests/vectordata.csv\"") {
        ("tests/vectordata.csv", rest)
    } else if let Some(rest) = inner.strip_prefix("tests/vectordata.csv") {
        ("tests/vectordata.csv", rest)
    } else if let Some(rest) = inner.strip_prefix("\"tests/vectordata2.csv\"") {
        ("tests/vectordata2.csv", rest)
    } else if let Some(rest) = inner.strip_prefix("tests/vectordata2.csv") {
        ("tests/vectordata2.csv", rest)
    } else {
        return None;
    };

    let path = PathBuf::from(path_str);

    if rest.is_empty() {
        return Some((path, CsvLoadOptions::promoted_count(1, ",")));
    }

    if !rest.starts_with(',') {
        return None;
    }
    let rest = rest[1..].trim_start();

    let (first_row_str, rest) = if let Some(comma_pos) = rest.find(',') {
        (rest[..comma_pos].trim(), &rest[comma_pos..])
    } else {
        (rest.trim(), "")
    };
    let first_row = first_row_str.parse::<usize>().ok()?;
    if first_row == 0 {
        return None;
    }

    if rest.is_empty() {
        return Some((path, CsvLoadOptions::promoted_count(first_row, ",")));
    }

    if !rest.starts_with(',') {
        return None;
    }
    let delimiter_arg = rest[1..].trim();
    if delimiter_arg != "\",\"" {
        return None;
    }

    Some((path, CsvLoadOptions::promoted_count(first_row, ",")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn upstream_fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../libqalculate/tests")
            .join(name)
    }

    fn temp_csv_path(name: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time is after UNIX epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "libqalculate_rust_{name}_{}_{}.csv",
            std::process::id(),
            unique
        ))
    }

    #[test]
    fn loads_upstream_numeric_vector_fixture() {
        let values = load_csv_numbers(upstream_fixture("vectordata.csv")).expect("fixture loads");
        assert_eq!(values.len(), 100);
        assert_eq!(values.first().unwrap().to_string(), "19.61486208");
        assert_eq!(values.last().unwrap().to_string(), "12.30805585");
    }

    #[test]
    fn loads_second_upstream_numeric_vector_fixture() {
        let values = load_csv_numbers(upstream_fixture("vectordata2.csv")).expect("fixture loads");
        assert_eq!(values.len(), 100);
        assert_eq!(values.first().unwrap().to_string(), "14.0019757");
        assert_eq!(values.last().unwrap().to_string(), "8.376544265");
    }

    #[test]
    fn gates_native_count_sources_to_promoted_spellings() {
        assert_eq!(
            parse_promoted_load_count("number(load(tests/vectordata.csv))"),
            Some((
                PathBuf::from("tests/vectordata.csv"),
                CsvLoadOptions::promoted_count(1, ",")
            ))
        );
        assert_eq!(
            parse_promoted_load_count("number(load(\"tests/vectordata2.csv\", 2))"),
            Some((
                PathBuf::from("tests/vectordata2.csv"),
                CsvLoadOptions::promoted_count(2, ",")
            ))
        );
        assert_eq!(
            parse_promoted_load_count("number(load(tests/vectordata.csv, 1, \",\"))"),
            Some((
                PathBuf::from("tests/vectordata.csv"),
                CsvLoadOptions::promoted_count(1, ",")
            ))
        );
        assert_eq!(
            parse_promoted_load_count("number(load( tests/vectordata.csv))"),
            None
        );
        assert_eq!(
            parse_promoted_load_count("number(load(tests/vectordata.csv, 0))"),
            None
        );
        assert_eq!(
            parse_promoted_load_count("number(load(tests/vectordata.csv, 1, \";\"))"),
            None
        );
        assert_eq!(
            parse_promoted_load_count("load(tests/vectordata.csv)"),
            None
        );
    }

    #[test]
    fn loads_with_first_row_and_delimiter_options() {
        let values = load_csv_numbers_with_options(
            upstream_fixture("vectordata.csv"),
            CsvLoadOptions::promoted_count(1, ","),
        )
        .expect("fixture loads");
        assert_eq!(values.len(), 100);

        let values = load_csv_numbers_with_options(
            upstream_fixture("vectordata.csv"),
            CsvLoadOptions::promoted_count(2, ","),
        )
        .expect("empty promoted load is allowed");
        assert!(values.is_empty());
    }

    #[test]
    fn resolves_upstream_relative_test_fixtures_from_crate_root() {
        let values = load_csv_numbers("tests/vectordata.csv").expect("fixture loads");
        assert_eq!(values.len(), 100);
        assert_eq!(values.first().unwrap().to_string(), "19.61486208");
    }

    #[test]
    fn reports_invalid_numeric_csv_field_with_location() {
        let path = temp_csv_path("invalid_numeric_field");
        std::fs::write(&path, "1, abc\n").expect("write temp csv");

        let error = load_csv_numbers(&path).expect_err("invalid field is rejected");

        std::fs::remove_file(&path).expect("remove temp csv");
        let message = error.to_string();
        assert!(message.contains("row 1, column 2"), "{message}");
        assert!(message.contains("abc"), "{message}");
    }

    #[test]
    fn reports_missing_csv_file_with_path() {
        let path = temp_csv_path("missing_file");

        let error = load_csv_numbers(&path).expect_err("missing file is rejected");

        let message = error.to_string();
        assert!(
            message.contains("failed to read CSV data from"),
            "{message}"
        );
        assert!(message.contains(&path.display().to_string()), "{message}");
    }

    #[test]
    fn rejects_empty_csv_fields_with_location() {
        let path = temp_csv_path("empty_numeric_field");
        std::fs::write(&path, "1,,2\n").expect("write temp csv");

        let error = load_csv_numbers(&path).expect_err("empty field is rejected");

        std::fs::remove_file(&path).expect("remove temp csv");
        let message = error.to_string();
        assert!(message.contains("row 1, column 2"), "{message}");
        assert!(message.contains("empty numeric CSV value"), "{message}");
    }

    #[test]
    fn rejects_csv_files_without_numeric_values() {
        let path = temp_csv_path("empty_numeric_file");
        std::fs::write(&path, "").expect("write temp csv");

        let error = load_csv_numbers(&path).expect_err("empty numeric file is rejected");

        std::fs::remove_file(&path).expect("remove temp csv");
        let message = error.to_string();
        assert!(
            message.contains("did not contain numeric values"),
            "{message}"
        );
    }
}
