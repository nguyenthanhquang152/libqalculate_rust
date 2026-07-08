use libqalculate_rust::definitions::{
    load_definition_xml_file, load_definition_xml_str, DefinitionActionKind,
    DefinitionDiagnosticKind, DefinitionItemKind, DefinitionSeverity,
};
use std::path::{Path, PathBuf};

fn upstream_data_file(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../libqalculate/data")
        .join(name)
}

fn path_strings(path: &[String]) -> Vec<&str> {
    path.iter().map(String::as_str).collect()
}

#[test]
fn loads_minimal_category_item_with_provenance() {
    let xml = r#"
<QALCULATE version="5.11.0">
  <category>
    <title>!units!Algebra</title>
    <variable active="false">
      <title>Answer</title>
      <names>r:answer,ultimate_answer</names>
      <value>42</value>
    </variable>
  </category>
</QALCULATE>
"#;

    let document = load_definition_xml_str("fixtures/minimal.xml", xml);

    assert_eq!(document.source().name(), "fixtures/minimal.xml");
    assert_eq!(document.version(), Some("5.11.0"));
    assert!(document.diagnostics().is_empty());
    assert_eq!(document.categories().len(), 1);
    assert_eq!(
        path_strings(document.categories()[0].path()),
        vec!["Algebra"]
    );

    let item = document
        .items()
        .first()
        .expect("minimal variable is loaded");
    assert_eq!(item.kind(), DefinitionItemKind::Variable);
    assert_eq!(path_strings(item.category_path()), vec!["Algebra"]);
    assert_eq!(item.names(), &["answer", "ultimate_answer"]);
    assert!(!item.active());
    assert!(item.active_specified());
    assert_eq!(item.provenance().source().name(), "fixtures/minimal.xml");
    assert!(item.provenance().line() >= 4);
    assert_eq!(
        item.field("value")
            .and_then(|field| field.text())
            .expect("value field is preserved"),
        "42"
    );
}

#[test]
fn malformed_xml_returns_structured_diagnostic_without_panic() {
    let document = load_definition_xml_str("fixtures/broken.xml", "<QALCULATE><category>");

    assert!(document.items().is_empty());
    let diagnostic = document
        .diagnostics()
        .first()
        .expect("malformed XML is reported");
    assert_eq!(diagnostic.kind(), DefinitionDiagnosticKind::MalformedXml);
    assert_eq!(diagnostic.severity(), DefinitionSeverity::Error);
    assert_eq!(diagnostic.source().name(), "fixtures/broken.xml");
    assert!(diagnostic.line() >= 1);
    assert!(diagnostic.message().contains("XML"));
}

#[test]
fn non_qalculate_root_returns_missing_root_diagnostic() {
    let document = load_definition_xml_str("fixtures/not-qalculate.xml", "<not_qalculate />");

    assert!(document.items().is_empty());
    let diagnostic = document
        .diagnostics()
        .first()
        .expect("missing root is reported");
    assert_eq!(diagnostic.kind(), DefinitionDiagnosticKind::MissingRoot);
    assert_eq!(diagnostic.severity(), DefinitionSeverity::Error);
    assert_eq!(diagnostic.tag(), Some("not_qalculate"));
}

#[test]
fn unknown_category_child_is_recoverable_diagnostic() {
    let xml = r#"
<QALCULATE version="5.11.0">
  <category>
    <title>Experimental</title>
    <mystery>
      <names>r:ghost</names>
    </mystery>
  </category>
</QALCULATE>
"#;

    let document = load_definition_xml_str("fixtures/unknown.xml", xml);

    assert!(document.items().is_empty());
    let diagnostic = document
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.kind() == DefinitionDiagnosticKind::UnsupportedTag)
        .expect("unknown tag is reported");
    assert_eq!(diagnostic.severity(), DefinitionSeverity::Warning);
    assert_eq!(diagnostic.tag(), Some("mystery"));
    assert_eq!(
        path_strings(diagnostic.category_path()),
        vec!["Experimental"]
    );
}

#[test]
fn duplicate_names_are_reported_without_dropping_items() {
    let xml = r#"
<QALCULATE version="5.11.0">
  <category>
    <title>Numbers</title>
    <variable><names>r:dup</names><value>1</value></variable>
    <variable><names>r:dup</names><value>2</value></variable>
  </category>
</QALCULATE>
"#;

    let document = load_definition_xml_str("fixtures/duplicates.xml", xml);

    assert_eq!(document.items().len(), 2);
    let diagnostic = document
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.kind() == DefinitionDiagnosticKind::DuplicateName)
        .expect("duplicate name is reported");
    assert_eq!(diagnostic.severity(), DefinitionSeverity::Warning);
    assert_eq!(diagnostic.name(), Some("dup"));
    assert_eq!(path_strings(diagnostic.category_path()), vec!["Numbers"]);
}

#[test]
fn translated_name_markers_are_removed_from_loaded_names() {
    let xml = r#"
<QALCULATE version="5.11.0">
  <object>
    <name>!planets!Mercury</name>
  </object>
  <dataset>
    <names>!datasets!r:number</names>
  </dataset>
</QALCULATE>
"#;

    let document = load_definition_xml_str("fixtures/translated-names.xml", xml);

    assert!(document.diagnostics().is_empty());
    assert_eq!(document.items()[0].names(), &["Mercury"]);
    assert_eq!(document.items()[1].names(), &["number"]);
}

#[test]
fn activation_and_deactivation_nodes_are_preserved() {
    let xml = r#"
<QALCULATE version="5.11.0">
  <deactivate>old_meter</deactivate>
  <activate>new_meter</activate>
</QALCULATE>
"#;

    let document = load_definition_xml_str("fixtures/actions.xml", xml);

    assert_eq!(document.actions().len(), 2);
    assert_eq!(
        document.actions()[0].kind(),
        DefinitionActionKind::Deactivate
    );
    assert_eq!(document.actions()[0].name(), "old_meter");
    assert_eq!(document.actions()[1].kind(), DefinitionActionKind::Activate);
    assert_eq!(document.actions()[1].name(), "new_meter");
    assert!(document.diagnostics().is_empty());
}

#[test]
fn parses_upstream_definition_xml_smoke_without_semantic_parity_claims() {
    let fixtures = [
        ("prefixes.xml.in", DefinitionItemKind::Prefix, false),
        ("currencies.xml.in", DefinitionItemKind::BuiltinUnit, true),
        ("units.xml.in", DefinitionItemKind::Unit, true),
        (
            "functions.xml.in",
            DefinitionItemKind::BuiltinFunction,
            true,
        ),
        (
            "variables.xml.in",
            DefinitionItemKind::BuiltinVariable,
            true,
        ),
        ("datasets.xml.in", DefinitionItemKind::Dataset, true),
        ("elements.xml.in", DefinitionItemKind::DataObject, false),
        ("planets.xml.in", DefinitionItemKind::DataObject, false),
    ];

    for (fixture, expected_kind, expects_category) in fixtures {
        let path = upstream_data_file(fixture);
        let xml = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        let document = load_definition_xml_str(format!("../libqalculate/data/{fixture}"), &xml);

        assert_eq!(
            document
                .diagnostics()
                .iter()
                .filter(|diagnostic| diagnostic.severity() == DefinitionSeverity::Error)
                .count(),
            0,
            "{fixture} should not produce loader errors: {:?}",
            document.diagnostics()
        );
        assert!(
            document
                .items()
                .iter()
                .any(|item| item.kind() == expected_kind),
            "{fixture} should load at least one {expected_kind:?}"
        );
        assert_eq!(
            !document.categories().is_empty(),
            expects_category,
            "{fixture} category expectation changed"
        );
        assert!(
            document
                .items()
                .iter()
                .any(|item| item.provenance().line() > 0),
            "{fixture} should retain source line provenance"
        );
    }
}

#[test]
fn file_loader_uses_path_as_source_name() {
    let path = upstream_data_file("prefixes.xml.in");
    let document = load_definition_xml_file(&path).expect("prefix XML file loads");

    assert_eq!(document.source().name(), path.display().to_string());
    assert!(document
        .items()
        .iter()
        .all(|item| item.provenance().source().name() == path.display().to_string()));
}

#[test]
fn file_loader_reports_missing_path_as_io_error() {
    let missing = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/does-not-exist.xml");

    let error = load_definition_xml_file(&missing).expect_err("missing XML path returns I/O error");

    assert!(error.to_string().contains("failed to read definition XML"));
    assert!(error.to_string().contains("does-not-exist.xml"));
}
