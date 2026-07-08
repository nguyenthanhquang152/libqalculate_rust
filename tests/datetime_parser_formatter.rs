use libqalculate_rust::ast::Expression;
use libqalculate_rust::datetime::{
    format_iso_datetime_with_offset, parse_datetime_literal, DateTime,
};
use libqalculate_rust::ffi::{Calculator, FallbackState};
use libqalculate_rust::number::Number;
use libqalculate_rust::parser::operators::parse_expression;

struct EnvGuard {
    key: &'static str,
    previous: Option<std::ffi::OsString>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let previous = std::env::var_os(key);
        std::env::set_var(key, value);
        Self { key, previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        if let Some(previous) = &self.previous {
            std::env::set_var(self.key, previous);
        } else {
            std::env::remove_var(self.key);
        }
    }
}

fn native_qalc_output(expression: &str) -> String {
    let _guard = EnvGuard::set("QALCULATE_DISABLE_FALLBACK", "1");
    let mut calc = Calculator::new();
    let output = calc
        .calculate_and_print_qalc_with_fallback_state(expression, 1000)
        .expect("datetime parser/formatter expression should run natively");
    assert_eq!(output.fallback_state, FallbackState::Native);
    output.output
}

#[test]
fn parses_iso_date_and_datetime_literals_with_timezones() {
    let date = parse_datetime_literal("2020-05-20").expect("valid ISO date");
    assert_eq!(date.value(), &DateTime::from_ymd(2020, 5, 20).unwrap());
    assert_eq!(date.offset_minutes(), None);

    let utc = parse_datetime_literal("2020-05-20T00:00:00Z").expect("valid UTC datetime");
    assert_eq!(
        utc.value(),
        &DateTime::from_ymd_hms(2020, 5, 20, 0, 0, Number::new()).unwrap()
    );
    assert_eq!(utc.offset_minutes(), Some(0));
    assert_eq!(
        format_iso_datetime_with_offset(utc.value(), utc.offset_minutes()),
        "2020-05-20T00:00:00Z"
    );

    let offset =
        parse_datetime_literal("2020-05-20T08:15:30+08:00").expect("valid offset datetime");
    assert_eq!(offset.offset_minutes(), Some(480));
    assert_eq!(
        format_iso_datetime_with_offset(offset.value(), offset.offset_minutes()),
        "2020-05-20T08:15:30+08:00"
    );

    let cet = parse_datetime_literal("2020-07-10T07:50CET").expect("valid CET datetime");
    assert_eq!(cet.offset_minutes(), Some(60));
    assert_eq!(
        format_iso_datetime_with_offset(cet.value(), Some(8 * 60)),
        "2020-07-10T07:50:00+08:00"
    );
}

#[test]
fn rejects_invalid_datetime_literals_without_reclassifying_plain_strings() {
    assert!(parse_datetime_literal("2020-02-30").is_err());
    assert!(parse_datetime_literal("2020-05-20T24:00:00Z").is_err());
    assert!(parse_datetime_literal("2020-05-20T12:00:00XYZ").is_err());
    assert!(parse_datetime_literal("2020-05-20T12:00:00+-8").is_err());

    let expr = parse_expression("\"2020-02-30\"").expect("invalid date stays a string literal");
    assert!(matches!(expr, Expression::Text(text) if text == "2020-02-30"));
}

#[test]
fn parser_classifies_valid_quoted_dates_as_datetime_literals() {
    let expr = parse_expression("\"2020-05-20\"").expect("parse quoted date");
    let Expression::DateTime(literal) = expr else {
        panic!("expected DateTime literal");
    };
    assert_eq!(literal.source(), "2020-05-20");
    assert_eq!(
        literal.value(),
        Some(&DateTime::from_ymd(2020, 5, 20).unwrap())
    );
}

#[test]
fn native_outputs_selected_dates_batch_parser_formatter_cases() {
    assert_eq!(native_qalc_output("10:31 + 8:30 to time"), "19:01");
    assert_eq!(native_qalc_output("10h 31min + 8h 30min to time"), "19:01");
    assert_eq!(
        native_qalc_output(r#""2020-07-10T07:50CET" to utc+8"#),
        "\"2020-07-10T14:50:00+08:00\""
    );
}
