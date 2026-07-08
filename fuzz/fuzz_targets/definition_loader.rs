#![no_main]

use libfuzzer_sys::fuzz_target;
use libqalculate_rust::definitions::load_definition_xml_str;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let _ = load_definition_xml_str("fuzz/definition_loader.xml", input);
    }
});
