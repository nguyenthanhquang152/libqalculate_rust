use libqalculate_rust::datasets::{
    load_dataset_catalog_from_dir, DatasetPropertyType, DatasetValueKind,
};
use std::path::{Path, PathBuf};

fn upstream_data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../libqalculate/data")
}

fn has_name_with_flags(
    names: &[libqalculate_rust::datasets::DefinitionName],
    name: &str,
    flags: &str,
) -> bool {
    names
        .iter()
        .any(|candidate| candidate.name() == name && candidate.flags() == flags)
}

#[test]
fn loads_element_and_planet_dataset_metadata_from_upstream_xml() {
    let catalog =
        load_dataset_catalog_from_dir(upstream_data_dir()).expect("dataset catalog loads");

    let elements = catalog.dataset_by_name("atom").expect("elements dataset");
    assert_eq!(elements.title(), Some("Elements"));
    assert_eq!(elements.default_data_file(), Some("elements.xml"));
    assert_eq!(elements.category_path(), &["Data Sets"]);
    assert_eq!(elements.object_argument().title(), Some("Element"));
    assert!(has_name_with_flags(elements.names(), "atom", "r"));
    assert!(elements
        .provenance()
        .source()
        .name()
        .ends_with("datasets.xml.in"));

    let symbol = elements.property_by_name("symbol").expect("symbol");
    assert_eq!(symbol.property_type(), DatasetPropertyType::Text);
    assert!(symbol.key());
    assert!(!symbol.hidden());
    assert_eq!(symbol.unit(), None);
    assert!(has_name_with_flags(symbol.names(), "symbol", "r"));

    let number = elements.property_by_name("number").expect("number");
    assert_eq!(number.property_type(), DatasetPropertyType::Number);
    assert!(number.key());
    assert!(has_name_with_flags(number.names(), "number", "r"));

    let mass = elements.property_by_name("weight").expect("mass alias");
    assert_eq!(mass.reference_name(), Some("mass"));
    assert_eq!(mass.title(), Some("Atomic Mass"));
    assert_eq!(mass.property_type(), DatasetPropertyType::Number);
    assert_eq!(mass.unit(), Some("u"));
    assert!(mass.approximate());

    let planets = catalog.dataset_by_name("planet").expect("planets dataset");
    assert_eq!(planets.title(), Some("Planets"));
    assert_eq!(planets.default_data_file(), Some("planets.xml"));
    assert_eq!(planets.object_argument().title(), Some("Planet"));

    let radius = planets.property_by_name("radius").expect("radius");
    assert_eq!(radius.property_type(), DatasetPropertyType::Number);
    assert_eq!(radius.unit(), Some("km"));
    assert!(radius.approximate());

    let gravity = planets.property_by_name("gravity").expect("gravity");
    assert_eq!(gravity.property_type(), DatasetPropertyType::Number);
    assert_eq!(gravity.unit(), Some("m/s^2"));
}

#[test]
fn loads_element_and_planet_objects_with_property_values_and_provenance() {
    let catalog =
        load_dataset_catalog_from_dir(upstream_data_dir()).expect("dataset catalog loads");

    let elements = catalog.dataset_by_name("atom").expect("elements dataset");
    let hydrogen = elements.object_by_key("H").expect("hydrogen by symbol");
    assert_eq!(hydrogen.value("symbol").expect("symbol").raw(), "H");
    assert_eq!(hydrogen.value("number").expect("number").raw(), "1");
    assert_eq!(hydrogen.value("name").expect("name").raw(), "Hydrogen");
    assert_eq!(
        hydrogen.value("mass").expect("mass").raw(),
        "[1.00784, 1.00811]"
    );
    assert_eq!(
        hydrogen.value("mass").expect("mass").kind(),
        DatasetValueKind::Number
    );
    assert!(hydrogen
        .provenance()
        .source()
        .name()
        .ends_with("elements.xml.in"));

    let helium = elements.object_by_key("2").expect("helium by number");
    assert_eq!(helium.value("symbol").expect("symbol").raw(), "He");
    assert_eq!(helium.value("mass").expect("mass").raw(), "4.002602(2)");

    let planets = catalog.dataset_by_name("planet").expect("planets dataset");
    let earth = planets.object_by_key("Earth").expect("earth");
    assert_eq!(earth.value("radius").expect("radius").raw(), "6371.0");
    assert_eq!(earth.value("gravity").expect("gravity").raw(), "9.80665");

    let mars = planets.object_by_key("Mars").expect("mars");
    assert_eq!(mars.value("mass").expect("mass").raw(), "6.4171E23");
    assert!(mars
        .provenance()
        .source()
        .name()
        .ends_with("planets.xml.in"));
}
