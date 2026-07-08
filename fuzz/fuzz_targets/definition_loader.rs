#![no_main]

use libfuzzer_sys::fuzz_target;
use libqalculate_rust::definitions::load_definition_xml_str;
use libqalculate_rust::definitions_catalog::FunctionVariableCatalog;
use libqalculate_rust::units::PrefixUnitCatalog;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let document = load_definition_xml_str("fuzz/definition_loader.xml", input);
        let _ = PrefixUnitCatalog::from_documents([document.clone()]);
        let _ = FunctionVariableCatalog::from_documents([document]);
    }
});
