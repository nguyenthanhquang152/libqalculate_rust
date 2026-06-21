#![forbid(unsafe_code)]

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

/// A parsed batch case plus the one-based source line of its expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocatedBatchCase {
    /// One-based source line where the expression appears in the upstream fixture.
    pub source_line: usize,
    /// Parsed expression and expected output.
    pub case: BatchCase,
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
    /// A session command failed to parse.
    InvalidCommand {
        /// One-based line number of the invalid command.
        line: usize,
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
            Self::InvalidCommand { line } => {
                write!(f, "session command at line {line} is invalid")
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
    Ok(parse_batch_cases_with_source_lines(input)?
        .into_iter()
        .map(|located| located.case)
        .collect())
}

/// Parse a libqalculate `.batch` fixture and retain expression source lines.
pub fn parse_batch_cases_with_source_lines(
    input: &str,
) -> Result<Vec<LocatedBatchCase>, BatchError> {
    let items = parse_batch_items(input)?;
    let cases = items
        .into_iter()
        .filter_map(|item| match item {
            BatchItem::Case(case) => Some(case),
            _ => None,
        })
        .collect();
    Ok(cases)
}

/// Return `{batch_file}:{source_line}` IDs for every source case with expected output.
///
/// The manifest index tracks qalc test cases, not setup/delete lines. Some upstream fixtures
/// contain unindented setup expressions without expected output, so this scan deliberately
/// records only expression lines followed by at least one tab-prefixed expected output.
pub fn batch_case_ids(batch_file: &str, input: &str) -> Result<Vec<String>, BatchError> {
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
            if current_expression_line.is_none() {
                return Err(BatchError::ExpectedWithoutExpression { line: line_number });
            }
            has_expected = true;
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
    Ok(ids)
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

/// One parsed item in a `.batch` fixture: either a command or a test case.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BatchItem {
    /// A parsed session command.
    Command {
        /// One-based source line where the command appears.
        source_line: usize,
        /// The parsed command.
        command: crate::parser::commands::SessionCommand,
    },
    /// A parsed located batch case.
    Case(LocatedBatchCase),
}

/// Parse a libqalculate `.batch` fixture into a sequence of commands and located cases.
pub fn parse_batch_items(input: &str) -> Result<Vec<BatchItem>, BatchError> {
    let mut items = Vec::new();
    let mut current_expression: Option<(usize, String)> = None;
    let mut current_expected = Vec::new();

    for (index, raw_line) in input.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            flush_batch_items(&mut items, &mut current_expression, &mut current_expected)?;
            continue;
        }

        if is_session_command(trimmed) {
            flush_batch_items(&mut items, &mut current_expression, &mut current_expected)?;
            let cmd = crate::parser::commands::parse_command(trimmed)
                .map_err(|_| BatchError::InvalidCommand { line: line_number })?;
            items.push(BatchItem::Command {
                source_line: line_number,
                command: cmd,
            });
            continue;
        }

        if let Some(expected) = line.strip_prefix('\t') {
            if current_expression.is_none() {
                return Err(BatchError::ExpectedWithoutExpression { line: line_number });
            }
            current_expected.push(expected.to_owned());
            continue;
        }

        flush_batch_items(&mut items, &mut current_expression, &mut current_expected)?;
        current_expression = Some((line_number, line.to_owned()));
    }

    flush_batch_items(&mut items, &mut current_expression, &mut current_expected)?;
    Ok(items)
}

fn flush_batch_items(
    items: &mut Vec<BatchItem>,
    current_expression: &mut Option<(usize, String)>,
    current_expected: &mut Vec<String>,
) -> Result<(), BatchError> {
    let Some((source_line, expression)) = current_expression.take() else {
        return Ok(());
    };
    if current_expected.is_empty() {
        return Err(BatchError::MissingExpected { expression });
    }
    items.push(BatchItem::Case(LocatedBatchCase {
        source_line,
        case: BatchCase::new(expression, std::mem::take(current_expected)),
    }));
    Ok(())
}

/// Return true for upstream batch session commands that affect later cases.
pub fn is_session_command(line: &str) -> bool {
    let line = line.trim_start();
    if line
        .get(..4)
        .map_or(false, |p| p.eq_ignore_ascii_case("set "))
    {
        return true;
    }
    if line
        .get(..5)
        .map_or(false, |p| p.eq_ignore_ascii_case("/set "))
    {
        return true;
    }
    if line
        .get(..7)
        .map_or(false, |p| p.eq_ignore_ascii_case("assume "))
    {
        return true;
    }
    if line
        .get(..8)
        .map_or(false, |p| p.eq_ignore_ascii_case("/assume "))
    {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{
        parse_batch_cases, parse_batch_items, render_batch_cases, BatchCase, BatchError, BatchItem,
        LocatedBatchCase,
    };
    use crate::parser::commands::{ApproximationMode, SessionCommand, SetCommand, SetSetting};

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
    fn skips_session_assume_commands() {
        let input = "/assume positive\nsqrt(x)\n\tsqrt(x)\n/assume unknown\nx\n\tx\n";
        let cases = parse_batch_cases(input).expect("fixture should parse");
        assert_eq!(cases.len(), 2);
        assert_eq!(cases[0].expression, "sqrt(x)");
        assert_eq!(cases[1].expression, "x");
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

    #[test]
    fn reports_case_ids_for_source_expression_lines() {
        let input = "# comment\n/set approximation exact\n1 + 1\n\t2\n\n2 + 2\n\t4\n";
        let ids = super::batch_case_ids("parser.batch", input).expect("fixture should parse");

        assert_eq!(ids, vec!["parser.batch:3", "parser.batch:6"]);
    }

    #[test]
    fn case_ids_skip_setup_lines_without_expected_output() {
        let input = "values=load(tests/vectordata.csv)\nmean(values)\n\t4.5\ndelete values\n";
        let ids = super::batch_case_ids("stats.batch", input).expect("fixture should parse");

        assert_eq!(ids, vec!["stats.batch:2"]);
    }

    #[test]
    fn test_parse_batch_items_with_commands() {
        let input = "/set approximation exact\nassume positive\n1 + 1\n\t2\n";
        let items = parse_batch_items(input).expect("items should parse");
        assert_eq!(items.len(), 3);
        match &items[0] {
            BatchItem::Command {
                source_line,
                command,
            } => {
                assert_eq!(*source_line, 1);
                match command {
                    SessionCommand::Set(SetCommand { setting, .. }) => {
                        assert_eq!(
                            *setting,
                            SetSetting::Approximation(ApproximationMode::Exact)
                        );
                    }
                    _ => panic!("Expected Set command"),
                }
            }
            _ => panic!("Expected Command item"),
        }
        match &items[1] {
            BatchItem::Command {
                source_line,
                command,
            } => {
                assert_eq!(*source_line, 2);
                match command {
                    SessionCommand::Assume(c) => {
                        assert_eq!(c.kind, crate::parser::commands::AssumeKind::Positive);
                    }
                    _ => panic!("Expected Assume command"),
                }
            }
            _ => panic!("Expected Command item"),
        }
        match &items[2] {
            BatchItem::Case(LocatedBatchCase { source_line, case }) => {
                assert_eq!(*source_line, 3);
                assert_eq!(case.expression, "1 + 1");
            }
            _ => panic!("Expected Case item"),
        }
    }
}
