#![no_main]

use libfuzzer_sys::fuzz_target;
use libqalculate_rust::parser::lexer::lex_line;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let _ = lex_line(input);
    }
});
