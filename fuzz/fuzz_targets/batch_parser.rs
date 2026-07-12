#![no_main]

use libfuzzer_sys::fuzz_target;
use libqalculate_rust::batch::{batch_case_ids, parse_batch_cases, parse_batch_items};

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let _ = parse_batch_cases(input);
        let _ = parse_batch_items(input);
        let _ = batch_case_ids("fuzz.batch", input);
    }
});
