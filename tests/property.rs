use libqalculate_rust::batch::{parse_batch_cases, render_batch_cases, BatchCase};
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
}
