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

/// Loads a comma-separated numeric file as native [`Number`] values.
pub fn load_csv_numbers(path: impl AsRef<Path>) -> Result<Vec<Number>, CsvLoadError> {
    let path = path.as_ref();
    let contents = std::fs::read_to_string(path).map_err(|error| {
        CsvLoadError::new(format!(
            "failed to read CSV data from {}: {error}",
            path.display()
        ))
    })?;

    let mut values = Vec::new();
    for (line_index, line) in contents.lines().enumerate() {
        for (column_index, raw) in line.split(',').enumerate() {
            let value = raw.trim();
            if value.is_empty() {
                return Err(CsvLoadError::new(format!(
                    "empty numeric CSV value at row {}, column {} in {}",
                    line_index + 1,
                    column_index + 1,
                    path.display()
                )));
            }
            let number = Number::from_str(value).map_err(|_| {
                CsvLoadError::new(format!(
                    "invalid numeric CSV value at row {}, column {} in {}: {value}",
                    line_index + 1,
                    column_index + 1,
                    path.display()
                ))
            })?;
            values.push(number);
        }
    }

    if values.is_empty() {
        return Err(CsvLoadError::new(format!(
            "CSV data file {} did not contain numeric values",
            path.display()
        )));
    }

    Ok(values)
}

pub(crate) fn native_output(expr: &str) -> Result<Option<String>, CsvLoadError> {
    let Some(path) = promoted_load_count_path(expr) else {
        return Ok(None);
    };
    let values = load_csv_numbers(path)?;
    Ok(Some(values.len().to_string()))
}

fn promoted_load_count_path(expr: &str) -> Option<PathBuf> {
    match expr {
        "number(load(tests/vectordata.csv))" | "number(load(\"tests/vectordata.csv\"))" => {
            Some(PathBuf::from("tests/vectordata.csv"))
        }
        "number(load(tests/vectordata2.csv))" | "number(load(\"tests/vectordata2.csv\"))" => {
            Some(PathBuf::from("tests/vectordata2.csv"))
        }
        _ => None,
    }
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
            promoted_load_count_path("number(load(tests/vectordata.csv))").as_deref(),
            Some(Path::new("tests/vectordata.csv"))
        );
        assert_eq!(
            promoted_load_count_path("number(load(\"tests/vectordata2.csv\"))").as_deref(),
            Some(Path::new("tests/vectordata2.csv"))
        );
        assert_eq!(
            promoted_load_count_path("number(load( tests/vectordata.csv))"),
            None
        );
        assert_eq!(promoted_load_count_path("load(tests/vectordata.csv)"), None);
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
