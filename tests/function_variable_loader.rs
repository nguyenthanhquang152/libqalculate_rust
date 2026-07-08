use libqalculate_rust::definitions::{load_definition_xml_str, DefinitionDiagnosticKind};
use libqalculate_rust::definitions_catalog::{
    load_function_variable_catalog_from_dir, BuildDiagnosticKind, FunctionKind,
    FunctionVariableCatalog, VariableKind,
};
use libqalculate_rust::parser::names::{NameMatch, NameRegistry, StaticRegistry};
use std::path::{Path, PathBuf};

fn upstream_data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../libqalculate/data")
}

fn has_name_with_flags(
    names: &[libqalculate_rust::definitions_catalog::DefinitionName],
    name: &str,
    flags: &str,
) -> bool {
    names
        .iter()
        .any(|candidate| candidate.name() == name && candidate.flags() == flags)
}

#[test]
fn loads_representative_function_metadata_from_upstream_xml() {
    let catalog = load_function_variable_catalog_from_dir(upstream_data_dir())
        .expect("function/variable catalog loads");

    let vector = catalog.function_by_name("vector").expect("vector function");
    assert_eq!(vector.kind(), FunctionKind::Builtin);
    assert_eq!(vector.title(), Some("Construct Vector"));
    assert_eq!(vector.category_path(), &["Matrices & Vectors"]);
    assert!(has_name_with_flags(vector.names(), "vector", "r"));
    assert_eq!(vector.arguments().len(), 1);
    assert_eq!(vector.arguments()[0].index(), 1);
    assert_eq!(vector.arguments()[0].title(), Some("Elements"));
    assert_eq!(vector.min_args(), 1);
    assert_eq!(vector.max_args(), Some(1));
    assert!(vector
        .provenance()
        .source()
        .name()
        .ends_with("functions.xml.in"));

    let genvector = catalog
        .function_by_name("genvector")
        .expect("genvector function");
    assert_eq!(genvector.title(), Some("Generate Vector"));
    assert_eq!(genvector.arguments().len(), 6);
    assert_eq!(genvector.max_args(), Some(6));
    assert!(genvector
        .description()
        .expect("genvector description")
        .contains("variable"));
    assert!(genvector
        .examples()
        .iter()
        .any(|example| example.contains("$name(x^2, 1, 5)")));

    let mean = catalog.function_by_name("mean").expect("mean function");
    assert_eq!(mean.category_path(), &["Statistics", "Means"]);
    assert_eq!(mean.arguments().len(), 1);

    let percentile = catalog
        .function_by_name("percentile")
        .expect("percentile function");
    assert_eq!(
        percentile.category_path(),
        &["Statistics", "Descriptive Statistics"]
    );
    assert_eq!(percentile.arguments()[0].argument_type(), None);
    assert_eq!(percentile.arguments()[1].title(), Some("Percentile (%)"));
    assert_eq!(
        percentile.arguments()[2].title(),
        Some("Quantile algorithm (as in R)")
    );

    let diff = catalog.function_by_name("diff").expect("diff function");
    assert_eq!(diff.category_path(), &["Calculus"]);
    assert!(has_name_with_flags(diff.names(), "diff", "r"));
    assert!(has_name_with_flags(diff.names(), "derivative", ""));
    assert_eq!(diff.arguments().len(), 4);

    let cross = catalog.function_by_name("cross").expect("cross function");
    assert_eq!(cross.kind(), FunctionKind::User);
    assert_eq!(
        cross.expression(),
        Some("((element(\\x,2)*element(\\y,3))-(element(\\x,3)*element(\\y,2)),(element(\\x,3)*element(\\y,1))-(element(\\x,1)*element(\\y,3)),(element(\\x,1)*element(\\y,2))-(element(\\x,2)*element(\\y,1)))")
    );
    assert_eq!(cross.arguments()[0].argument_type(), Some("vector"));
    assert_eq!(cross.arguments()[0].condition(), Some("dimension(\\x)==3"));

    let neg = catalog.function_by_name("neg").expect("neg function");
    assert_eq!(neg.kind(), FunctionKind::User);
    assert_eq!(neg.expression(), Some("-\\x"));
    assert_eq!(neg.arguments()[0].handle_vector(), Some(false));

    let integrate = catalog
        .function_by_name("integrate")
        .expect("integrate function");
    assert_eq!(integrate.arguments().len(), 5);
    assert!(has_name_with_flags(integrate.names(), "∫", "au"));
}

#[test]
fn loads_representative_variable_metadata_from_upstream_xml() {
    let catalog = load_function_variable_catalog_from_dir(upstream_data_dir())
        .expect("function/variable catalog loads");

    let percent = catalog.variable_by_name("%").expect("percent variable");
    assert_eq!(percent.kind(), VariableKind::Builtin);
    assert_eq!(percent.title(), Some("Percent"));
    assert_eq!(percent.category_path(), &["Small Numbers"]);
    assert!(has_name_with_flags(percent.names(), "%", "a"));
    assert!(has_name_with_flags(percent.names(), "percent", "r"));

    let permille = catalog.variable_by_name("permille").expect("permille");
    assert_eq!(permille.kind(), VariableKind::Builtin);
    assert!(has_name_with_flags(permille.names(), "‰", "au"));

    let googol = catalog.variable_by_name("googol").expect("googol");
    assert_eq!(googol.kind(), VariableKind::Known);
    assert_eq!(googol.title(), Some("Googol"));
    assert_eq!(googol.value(), Some("10^100"));

    let planck = catalog.variable_by_name("planck").expect("planck");
    assert_eq!(planck.title(), Some("Planck Constant"));
    assert_eq!(planck.value(), Some("6.62607015E-34"));
    assert_eq!(planck.unit(), Some("J*s"));

    let gravity = catalog
        .variable_by_name("newtonian_constant")
        .expect("newtonian constant");
    assert_eq!(gravity.value(), Some("6.67430E-11"));
    assert_eq!(gravity.uncertainty(), Some("1.5E-15"));
    assert!(!gravity.uncertainty_is_relative());
    assert_eq!(gravity.precision(), None);
    assert!(!gravity.approximate());
    assert_eq!(gravity.unit(), Some("m^3*kg^(-1)*s^(-2)"));
}

#[test]
fn registers_loaded_functions_and_variables_for_name_lookup() {
    let catalog = load_function_variable_catalog_from_dir(upstream_data_dir())
        .expect("function/variable catalog loads");
    let mut registry = StaticRegistry::new();
    catalog.register_into(&mut registry);

    match registry.lookup("percentile", true) {
        Some(NameMatch::Function {
            definition,
            min_args,
            max_args,
        }) => {
            assert_eq!(definition.id(), "percentile");
            assert_eq!(min_args, 3);
            assert_eq!(max_args, Some(3));
        }
        other => panic!("expected percentile function lookup, got {other:?}"),
    }

    assert!(matches!(
        registry.lookup("integral", true),
        Some(NameMatch::Function { .. })
    ));
    assert!(matches!(
        registry.lookup("percent", false),
        Some(NameMatch::Variable { .. })
    ));
    assert!(matches!(
        registry.lookup("googol", false),
        Some(NameMatch::Variable { .. })
    ));
}

#[test]
fn builder_reports_unknown_function_and_variable_fields_without_panicking() {
    let functions_xml = r#"
<QALCULATE version="test">
  <category>
    <title>Test Functions</title>
    <builtin_function name="testfunc">
      <title>Test Function</title>
      <names>r:testfunc</names>
      <argument type="integer" index="1">
        <title>Input</title>
        <min>1</min>
        <max>9</max>
      </argument>
      <surprise>ignored but diagnosed</surprise>
    </builtin_function>
  </category>
</QALCULATE>
"#;
    let variables_xml = r#"
<QALCULATE version="test">
  <category>
    <title>Test Variables</title>
    <variable>
      <title>Test Variable</title>
      <names>r:testvar</names>
      <value unit="m" relative_uncertainty="0.1" precision="12" approximate="true">42</value>
      <mystery>ignored but diagnosed</mystery>
    </variable>
  </category>
</QALCULATE>
"#;

    let catalog = FunctionVariableCatalog::from_xml_sources([
        ("fixtures/functions.xml", functions_xml),
        ("fixtures/variables.xml", variables_xml),
    ]);

    let function = catalog.function_by_name("testfunc").expect("test function");
    assert_eq!(function.arguments()[0].argument_type(), Some("integer"));
    assert_eq!(function.arguments()[0].min(), Some("1"));
    assert_eq!(function.arguments()[0].max(), Some("9"));

    let variable = catalog.variable_by_name("testvar").expect("test variable");
    assert_eq!(variable.value(), Some("42"));
    assert_eq!(variable.unit(), Some("m"));
    assert_eq!(variable.uncertainty(), Some("0.1"));
    assert!(variable.uncertainty_is_relative());
    assert_eq!(variable.precision(), Some("12"));
    assert!(variable.approximate());

    assert!(catalog.diagnostics().iter().any(|diagnostic| {
        diagnostic.kind() == BuildDiagnosticKind::UnsupportedFunctionField
            && diagnostic.tag() == Some("surprise")
            && diagnostic.provenance().category_path() == ["Test Functions"]
    }));
    assert!(catalog.diagnostics().iter().any(|diagnostic| {
        diagnostic.kind() == BuildDiagnosticKind::UnsupportedVariableField
            && diagnostic.tag() == Some("mystery")
            && diagnostic.provenance().category_path() == ["Test Variables"]
    }));
}

#[test]
fn catalog_preserves_duplicate_name_diagnostics_without_dropping_items() {
    let xml = r#"
<QALCULATE version="test">
  <category>
    <title>Duplicate Names</title>
    <builtin_function name="dupfn">
      <title>Duplicate Function A</title>
      <names>r:dupfn</names>
    </builtin_function>
    <builtin_function name="dupfn">
      <title>Duplicate Function B</title>
      <names>r:dupfn</names>
    </builtin_function>
    <variable>
      <title>Duplicate Variable A</title>
      <names>r:dupvar</names>
      <value>1</value>
    </variable>
    <variable>
      <title>Duplicate Variable B</title>
      <names>r:dupvar</names>
      <value>2</value>
    </variable>
  </category>
</QALCULATE>
"#;
    let document = load_definition_xml_str("fixtures/duplicates.xml", xml);

    let catalog = FunctionVariableCatalog::from_documents([document]);

    assert_eq!(catalog.functions().functions().len(), 2);
    assert_eq!(catalog.variables().variables().len(), 2);
    assert!(catalog
        .source_diagnostics()
        .iter()
        .any(
            |diagnostic| diagnostic.kind() == DefinitionDiagnosticKind::DuplicateName
                && diagnostic.name() == Some("dupfn")
        ));
    assert!(catalog
        .source_diagnostics()
        .iter()
        .any(
            |diagnostic| diagnostic.kind() == DefinitionDiagnosticKind::DuplicateName
                && diagnostic.name() == Some("dupvar")
        ));
}

#[test]
fn inactive_function_and_variable_items_are_not_registered_for_lookup() {
    let xml = r#"
<QALCULATE version="test">
  <category>
    <title>Inactive Items</title>
    <builtin_function name="inactive_fn" active="false">
      <title>Inactive Function</title>
      <names>r:inactive_fn</names>
    </builtin_function>
    <variable active="false">
      <title>Inactive Variable</title>
      <names>r:inactive_var</names>
      <value>5</value>
    </variable>
  </category>
</QALCULATE>
"#;
    let catalog = FunctionVariableCatalog::from_xml_sources([("fixtures/inactive.xml", xml)]);
    let function = catalog
        .function_by_name("inactive_fn")
        .expect("inactive function is still loaded");
    let variable = catalog
        .variable_by_name("inactive_var")
        .expect("inactive variable is still loaded");
    let mut registry = StaticRegistry::new();

    assert!(!function.active());
    assert!(!variable.active());
    catalog.register_into(&mut registry);
    assert_eq!(registry.lookup("inactive_fn", true), None);
    assert_eq!(registry.lookup("inactive_var", false), None);
}
