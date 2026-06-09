#![no_main]

use libfuzzer_sys::fuzz_target;
use libqalculate_rust::batch::parse_batch_cases;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let _ = parse_batch_cases(input);
    }
});
