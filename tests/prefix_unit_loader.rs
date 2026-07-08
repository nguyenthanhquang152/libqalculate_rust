use libqalculate_rust::parser::names::{NameMatch, NameRegistry, StaticRegistry};
use libqalculate_rust::units::{
    load_prefix_unit_catalog_from_dir, BuildDiagnosticKind, PrefixKind, PrefixUnitCatalog, UnitKind,
};
use std::path::{Path, PathBuf};

fn upstream_data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../libqalculate/data")
}

fn has_name_with_flags(
    names: &[libqalculate_rust::units::DefinitionName],
    name: &str,
    flags: &str,
) -> bool {
    names
        .iter()
        .any(|candidate| candidate.name() == name && candidate.flags() == flags)
}

#[test]
fn loads_representative_prefixes_with_name_flags_and_provenance() {
    let catalog = load_prefix_unit_catalog_from_dir(upstream_data_dir()).expect("catalog loads");

    let kilo = catalog.prefix_by_name("kilo").expect("kilo prefix");
    assert_eq!(kilo.kind(), PrefixKind::Decimal);
    assert_eq!(kilo.exponent(), 3);
    assert!(has_name_with_flags(kilo.names(), "k", "ar"));
    assert!(has_name_with_flags(kilo.names(), "kilo", "r"));
    assert!(kilo
        .provenance()
        .source()
        .name()
        .ends_with("prefixes.xml.in"));

    let deci = catalog.prefix_by_name("deci").expect("deci prefix");
    assert_eq!(deci.kind(), PrefixKind::Decimal);
    assert_eq!(deci.exponent(), -1);
    assert!(has_name_with_flags(deci.names(), "d", "ar"));

    let micro = catalog.prefix_by_name("micro").expect("micro prefix");
    assert_eq!(micro.exponent(), -6);
    assert!(has_name_with_flags(micro.names(), "μ", "aur"));
    assert!(has_name_with_flags(micro.names(), "µ", "auor"));
    assert!(has_name_with_flags(micro.names(), "u", "ar"));

    let mebi = catalog.prefix_by_name("mebi").expect("mebi prefix");
    assert_eq!(mebi.kind(), PrefixKind::Binary);
    assert_eq!(mebi.exponent(), 20);
    assert!(has_name_with_flags(mebi.names(), "Mi", "ar"));
}

#[test]
fn loads_representative_units_with_metadata_and_relations() {
    let catalog = load_prefix_unit_catalog_from_dir(upstream_data_dir()).expect("catalog loads");

    let meter = catalog.unit_by_name("m").expect("meter unit");
    assert_eq!(meter.kind(), UnitKind::Base);
    assert_eq!(meter.title(), Some("Meter"));
    assert_eq!(meter.category_path(), &["Length"]);
    assert_eq!(meter.system(), Some("SI"));
    assert!(meter.use_with_prefixes());
    assert_eq!(meter.prefix_max(), Some(3));
    assert!(has_name_with_flags(meter.names(), "m", "ar"));
    assert!(has_name_with_flags(meter.names(), "meters", "p"));

    let liter = catalog.unit_by_name("L").expect("liter unit");
    assert_eq!(liter.kind(), UnitKind::Alias);
    assert_eq!(liter.title(), Some("Liter"));
    assert_eq!(liter.prefix_max(), Some(0));
    let liter_base = liter.base().expect("liter has base relation");
    assert_eq!(liter_base.unit(), "m");
    assert_eq!(liter_base.relation(), Some("0.001"));
    assert_eq!(liter_base.exponent(), 3);

    for (name, title) in [
        ("N", "Newton"),
        ("Pa", "Pascal"),
        ("Ω", "Ohm"),
        ("A", "Ampere"),
        ("V", "Volt"),
        ("bit", "Bit (Binary Digit)"),
        ("byte", "Byte (8-bit)"),
    ] {
        let unit = catalog
            .unit_by_name(name)
            .unwrap_or_else(|| panic!("{name} unit"));
        assert_eq!(unit.title(), Some(title), "{name} title");
        assert!(unit.provenance().source().name().ends_with("units.xml.in"));
    }
    assert_eq!(
        catalog
            .unit_by_name("byte")
            .expect("byte unit")
            .prefix_min(),
        Some(0)
    );

    let decimeter = catalog.unit_by_name("dm_c").expect("decimeter composite");
    assert_eq!(decimeter.kind(), UnitKind::Composite);
    assert_eq!(decimeter.parts().len(), 1);
    assert_eq!(decimeter.parts()[0].unit(), "m");
    assert_eq!(decimeter.parts()[0].prefix_exponent(), Some(-1));
}

#[test]
fn loads_currency_units_without_claiming_rate_semantics() {
    let catalog = load_prefix_unit_catalog_from_dir(upstream_data_dir()).expect("catalog loads");

    let usd = catalog.unit_by_name("USD").expect("USD currency unit");
    assert_eq!(usd.kind(), UnitKind::Builtin);
    assert_eq!(usd.title(), Some("U.S. Dollar"));
    assert_eq!(usd.category_path(), &["Currency"]);
    assert!(usd
        .countries()
        .iter()
        .any(|country| country == "United States"));
    assert!(has_name_with_flags(usd.names(), "$", "a"));
    assert!(has_name_with_flags(usd.names(), "dollars", "p"));
    assert!(usd.base().is_none(), "rate semantics belong to #49");
}

#[test]
fn registers_loaded_prefixes_and_units_for_name_lookup() {
    let catalog = load_prefix_unit_catalog_from_dir(upstream_data_dir()).expect("catalog loads");
    let mut registry = StaticRegistry::new();
    catalog.register_into(&mut registry);

    assert!(matches!(
        registry.lookup("Ω", false),
        Some(NameMatch::Unit { prefix: None, .. })
    ));
    assert!(matches!(
        registry.lookup("byte", false),
        Some(NameMatch::Unit { prefix: None, .. })
    ));

    match registry.lookup("dm", false) {
        Some(NameMatch::Unit { definition, prefix }) => {
            assert_eq!(definition.id(), "m");
            assert_eq!(prefix.expect("deci prefix").id(), "d");
        }
        other => panic!("expected deci-meter lookup, got {other:?}"),
    }

    match registry.lookup("MiB", false) {
        Some(NameMatch::Unit { definition, prefix }) => {
            assert_eq!(definition.id(), "B");
            assert_eq!(prefix.expect("mebi prefix").id(), "Mi");
        }
        other => panic!("expected mebibyte lookup, got {other:?}"),
    }
}

#[test]
fn builder_reports_unknown_prefix_and_unit_fields_without_panicking() {
    let prefix_xml = r#"
<QALCULATE version="test">
  <prefix type="decimal">
    <names>r:testprefix</names>
    <exponent>1</exponent>
    <surprise>ignored but diagnosed</surprise>
  </prefix>
</QALCULATE>
"#;
    let unit_xml = r#"
<QALCULATE version="test">
  <category>
    <title>Test Units</title>
    <unit type="base">
      <title>Test Unit</title>
      <description>Known metadata</description>
      <names>r:testunit</names>
      <countries>One, Two</countries>
      <hidden>true</hidden>
      <unexpected>ignored but diagnosed</unexpected>
    </unit>
  </category>
</QALCULATE>
"#;

    let catalog = PrefixUnitCatalog::from_xml_sources([
        ("fixtures/prefixes.xml", prefix_xml),
        ("fixtures/units.xml", unit_xml),
    ]);

    assert!(catalog.prefix_by_name("testprefix").is_some());
    let unit = catalog.unit_by_name("testunit").expect("test unit");
    assert_eq!(unit.description(), Some("Known metadata"));
    assert_eq!(unit.countries(), ["One", "Two"]);
    assert!(unit.hidden());
    assert!(catalog.diagnostics().iter().any(|diagnostic| {
        diagnostic.kind() == BuildDiagnosticKind::UnsupportedPrefixField
            && diagnostic.tag() == Some("surprise")
            && diagnostic.provenance().source().name() == "fixtures/prefixes.xml"
    }));
    assert!(catalog.diagnostics().iter().any(|diagnostic| {
        diagnostic.kind() == BuildDiagnosticKind::UnsupportedUnitField
            && diagnostic.tag() == Some("unexpected")
            && diagnostic.provenance().category_path() == ["Test Units"]
    }));
    assert!(catalog
        .diagnostics()
        .iter()
        .all(|diagnostic| diagnostic.tag() != Some("description")
            && diagnostic.tag() != Some("hidden")));
}
