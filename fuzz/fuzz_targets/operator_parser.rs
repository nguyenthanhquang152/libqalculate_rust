#![no_main]

use libfuzzer_sys::fuzz_target;
use libqalculate_rust::parser::operators::parse_expression;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let _ = parse_expression(input);
    }
});
