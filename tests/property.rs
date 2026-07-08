use libqalculate_rust::batch::{parse_batch_cases, render_batch_cases, BatchCase};
use libqalculate_rust::datetime::DateTime;
use libqalculate_rust::number::Number;
use libqalculate_rust::parser::lexer::lex_line;
use libqalculate_rust::parser::operators::parse_expression;
use proptest::prelude::*;

fn safe_line() -> impl Strategy<Value = String> {
    "[ -~]{1,40}".prop_filter("line must not be blank or tab-prefixed", |line| {
        !line.trim().is_empty() && !line.starts_with('\t') && !line.trim_start().starts_with('#')
    })
}

proptest! {
    #[test]
    fn batch_render_parse_roundtrip(expression in safe_line(), expected in prop::collection::vec(safe_line(), 1..4)) {
        let cases = vec![BatchCase::new(expression, expected)];
        let rendered = render_batch_cases(&cases);
        let parsed = parse_batch_cases(&rendered).expect("rendered batch text must parse");
        prop_assert_eq!(parsed, cases);
    }

    #[test]
    fn lexer_arbitrary_utf8_input_returns_or_errors_without_panicking(input in ".*") {
        let _ = lex_line(&input);
    }

    #[test]
    fn operator_parser_arbitrary_utf8_input_returns_or_errors_without_panicking(input in ".*") {
        let _ = parse_expression(&input);
    }

    #[test]
    fn datetime_add_days_roundtrips_through_days_to(
        year in 1970i64..=2100,
        month in 1i64..=12,
        day in 1i64..=31,
        delta in -1000i64..=1000,
    ) {
        let Ok(date) = DateTime::from_ymd(year, month, day) else {
            return Ok(());
        };
        let shifted = date
            .add_days(&Number::from_i64(delta))
            .expect("bounded day shift should stay in range");
        prop_assert_eq!(date.days_to(&shifted), Number::from_i64(delta));
    }
}
