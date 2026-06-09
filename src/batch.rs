use std::fmt::{self, Write as _};
use std::fs;
use std::path::Path;

/// One expression and its expected printed result from a libqalculate `.batch` file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchCase {
    /// Input expression sent to qalc.
    pub expression: String,
    /// Expected output lines. Upstream fixtures usually use one tab-prefixed line.
    pub expected: Vec<String>,
}

impl BatchCase {
    /// Create a batch case after trimming only trailing carriage returns.
    pub fn new(expression: impl Into<String>, expected: Vec<String>) -> Self {
        Self {
            expression: expression.into(),
            expected,
        }
    }
}

/// Error returned when reading or parsing libqalculate batch fixtures.
#[derive(Debug)]
pub enum BatchError {
    /// File I/O failed.
    Io(std::io::Error),
    /// A fixture contains an expected-output line before any expression line.
    ExpectedWithoutExpression {
        /// One-based line number of the invalid expected-output line.
        line: usize,
    },
    /// A fixture contains an expression without a following expected output.
    MissingExpected {
        /// Expression that did not have a tab-prefixed expected output.
        expression: String,
    },
}

impl fmt::Display for BatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "{err}"),
            Self::ExpectedWithoutExpression { line } => {
                write!(f, "expected output at line {line} has no expression")
            }
            Self::MissingExpected { expression } => {
                write!(f, "expression has no expected output: {expression:?}")
            }
        }
    }
}

impl std::error::Error for BatchError {}

impl From<std::io::Error> for BatchError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

/// Parse a libqalculate `.batch` fixture.
///
/// The upstream format is line-oriented: an expression line is followed by one or more
/// tab-prefixed expected-output lines. Blank lines separate cases and comments start with `#`.
pub fn parse_batch_cases(input: &str) -> Result<Vec<BatchCase>, BatchError> {
    let mut cases = Vec::new();
    let mut current_expression: Option<String> = None;
    let mut current_expected = Vec::new();

    for (index, raw_line) in input.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);

        if line.trim().is_empty()
            || line.trim_start().starts_with('#')
            || line.starts_with("set ")
            || line.starts_with("/set ")
        {
            flush_case(&mut cases, &mut current_expression, &mut current_expected)?;
            continue;
        }

        if let Some(expected) = line.strip_prefix('\t') {
            if current_expression.is_none() {
                return Err(BatchError::ExpectedWithoutExpression { line: line_number });
            }
            current_expected.push(expected.to_owned());
            continue;
        }

        flush_case(&mut cases, &mut current_expression, &mut current_expected)?;
        current_expression = Some(line.to_owned());
    }

    flush_case(&mut cases, &mut current_expression, &mut current_expected)?;
    Ok(cases)
}

/// Read and parse a libqalculate `.batch` fixture from disk.
pub fn read_batch_cases(path: impl AsRef<Path>) -> Result<Vec<BatchCase>, BatchError> {
    let input = fs::read_to_string(path)?;
    parse_batch_cases(&input)
}

/// Render cases back to canonical `.batch` text.
pub fn render_batch_cases(cases: &[BatchCase]) -> String {
    let mut output = String::new();
    for case in cases {
        writeln!(&mut output, "{}", case.expression).expect("write to String cannot fail");
        for expected in &case.expected {
            writeln!(&mut output, "\t{expected}").expect("write to String cannot fail");
        }
        output.push('\n');
    }
    output
}

fn flush_case(
    cases: &mut Vec<BatchCase>,
    current_expression: &mut Option<String>,
    current_expected: &mut Vec<String>,
) -> Result<(), BatchError> {
    let Some(expression) = current_expression.take() else {
        return Ok(());
    };
    if current_expected.is_empty() {
        return Err(BatchError::MissingExpected { expression });
    }
    cases.push(BatchCase::new(expression, std::mem::take(current_expected)));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{parse_batch_cases, render_batch_cases, BatchCase, BatchError};

    #[test]
    fn parses_expression_and_expected_output() {
        let cases = parse_batch_cases("1 + 1\n\t2\n").expect("fixture should parse");
        assert_eq!(cases, vec![BatchCase::new("1 + 1", vec!["2".to_owned()])]);
    }

    #[test]
    fn skips_blank_lines_and_comments() {
        let cases = parse_batch_cases("# heading\n\n2 + 2\n\t4\n\n").expect("fixture should parse");
        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].expression, "2 + 2");
    }

    #[test]
    fn renders_parseable_batch_text() {
        let cases = vec![BatchCase::new("sqrt(4)", vec!["2".to_owned()])];
        let rendered = render_batch_cases(&cases);
        assert_eq!(
            parse_batch_cases(&rendered).expect("rendered fixture parses"),
            cases
        );
    }

    #[test]
    fn reports_one_based_line_for_orphan_expected_output() {
        let error = parse_batch_cases("\n\torphan\n").expect_err("orphan output should fail");
        assert!(matches!(
            error,
            BatchError::ExpectedWithoutExpression { line: 2 }
        ));
    }

    #[test]
    fn error_display_includes_actionable_context() {
        let orphan = BatchError::ExpectedWithoutExpression { line: 9 }.to_string();
        assert!(orphan.contains("line 9"));

        let missing = BatchError::MissingExpected {
            expression: "1 +".to_owned(),
        }
        .to_string();
        assert!(missing.contains("1 +"));
    }
}
