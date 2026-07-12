use libqalculate_rust::ffi::Calculator as UpstreamCalculator;
use libqalculate_rust::messages::{CalculatorMessage, MessageStage, MessageType};
use libqalculate_rust::options::{ApproximationMode, NumberFractionFormat};
use libqalculate_rust::Calculator;
use std::path::{Path, PathBuf};

fn upstream_data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../libqalculate/data")
}

#[test]
fn native_calculator_constructs_parses_evaluates_and_prints() {
    let mut calculator = Calculator::new();

    assert_eq!(calculator.precision(), 10);

    assert_eq!(
        calculator
            .calculate_and_print("1 + 1")
            .expect("simple calculation succeeds"),
        "2"
    );

    let parsed = calculator.parse("2 * (3 + 4)").expect("expression parses");
    let evaluated = calculator.evaluate(&parsed).expect("expression evaluates");
    assert_eq!(calculator.print(&evaluated).expect("result formats"), "14");
}

#[test]
fn native_calculator_uses_the_qalc_public_precision_default() {
    let mut calculator = Calculator::new();
    calculator.set_formatting_approximation(ApproximationMode::Approximate);

    assert_eq!(
        calculator
            .calculate_and_print("1 / 3")
            .expect("default precision calculation succeeds"),
        "0.3333333333"
    );
}

#[test]
fn native_calculator_matches_the_upstream_simple_api_example() {
    let mut upstream_calculator = UpstreamCalculator::new();
    let upstream = upstream_calculator
        .calculate_and_print("1 + 1", 2_000)
        .expect("upstream Calculator::calculateAndPrint example succeeds");

    let mut calculator = Calculator::new();
    let native = calculator
        .calculate_and_print("1 + 1")
        .expect("native example succeeds");
    assert_eq!(native, upstream);
}

#[test]
fn native_calculator_exposes_options_and_structured_messages() {
    let mut calculator = Calculator::new();

    calculator.set_formatting_approximation(ApproximationMode::Approximate);
    calculator.set_fraction_format(NumberFractionFormat::Decimal);
    calculator.set_precision(8);
    assert_eq!(
        calculator
            .calculate_and_print("1 / 3")
            .expect("approximate calculation succeeds"),
        "0.33333333"
    );
    assert_eq!(
        calculator.formatting_approximation(),
        ApproximationMode::Approximate
    );
    assert_eq!(calculator.fraction_format(), NumberFractionFormat::Decimal);

    let error = calculator
        .calculate_and_print("5 := x")
        .expect_err("invalid assignment target fails");
    assert!(error.message().contains("InvalidAssignmentTarget"));
    assert_eq!(calculator.messages().len(), 1);
    assert_eq!(calculator.messages()[0].message_type(), MessageType::Error);
    assert_eq!(calculator.messages()[0].stage(), MessageStage::Parsing);

    let drained: Vec<CalculatorMessage> = calculator.take_messages();
    assert_eq!(drained.len(), 1);
    assert!(calculator.messages().is_empty());
}

#[test]
fn native_calculator_loads_and_exposes_definition_catalogs_atomically() {
    let mut calculator = Calculator::new();
    calculator
        .load_definitions_from_dir(upstream_data_dir())
        .expect("upstream definition catalogs load");

    assert!(calculator
        .definitions()
        .expect("function/variable catalog")
        .function_by_name("sin")
        .is_some());
    assert!(calculator
        .units()
        .expect("prefix/unit catalog")
        .unit_by_name("m")
        .is_some());
    let datasets = libqalculate_rust::datasets::load_dataset_catalog_from_dir(upstream_data_dir())
        .expect("public dataset catalog loads independently");
    assert!(datasets.dataset_by_name("atom").is_some());

    let parsed_unit = calculator.parse("m").expect("unit syntax parses");
    let evaluated_unit = calculator
        .evaluate(&parsed_unit)
        .expect("loaded registry resolves unit during evaluation");
    assert!(matches!(
        &evaluated_unit,
        libqalculate_rust::ast::Expression::Unit { .. }
    ));
    assert_eq!(
        calculator
            .print(&evaluated_unit)
            .expect("bare unit formats with its coefficient"),
        "1 m"
    );

    assert_eq!(
        calculator
            .convert_and_print("1 m", "cm")
            .expect("focused native unit conversion succeeds"),
        "100 cm"
    );
    assert_eq!(
        calculator
            .convert_and_print("52", "hex")
            .expect("base conversion remains available after catalog loading"),
        "34"
    );
}

#[test]
fn loaded_unit_probe_preserves_dimensionless_variable_base_conversion() {
    let mut calculator = Calculator::new();
    calculator
        .load_definitions_from_dir(upstream_data_dir())
        .expect("unit catalog loads");
    calculator
        .calculate("m := 52")
        .expect("session variable assignment succeeds");

    assert_eq!(
        calculator
            .convert_and_print("m", "hex")
            .expect("dimensionless variable converts after unit loading"),
        "34"
    );
}

#[test]
fn number_base_keywords_are_not_shadowed_by_session_variables() {
    let mut calculator = Calculator::new();
    calculator
        .load_definitions_from_dir(upstream_data_dir())
        .expect("unit catalog loads");
    calculator
        .calculate("hex := 2")
        .expect("shadowing variable assignment succeeds");

    assert_eq!(
        calculator
            .calculate_and_print("52 to hex")
            .expect("conversion keyword keeps its syntactic meaning"),
        "34"
    );
}

#[test]
fn unit_conversion_without_loaded_definitions_fails_clearly() {
    let mut calculator = Calculator::new();

    assert_eq!(
        calculator
            .convert_and_print("52", "hex")
            .expect("dimensionless base conversion does not require definitions"),
        "34"
    );
    let error = calculator
        .convert_and_print("1 m", "cm")
        .expect_err("unit conversion requires a loaded catalog");
    assert!(error.message().contains("load definition catalogs"));
}

#[test]
fn failed_unloaded_unit_conversion_does_not_mutate_the_session() {
    let mut calculator = Calculator::new();

    calculator
        .convert_and_print("x := 1 m", "cm")
        .expect_err("unloaded unit conversion fails");
    assert_eq!(
        calculator
            .calculate_and_print("x")
            .expect("failed probe does not define x"),
        "x"
    );
}

#[test]
fn conversion_target_is_not_consumed_by_an_input_comment() {
    let mut calculator = Calculator::new();

    assert_eq!(
        calculator
            .convert_and_print("52 # answer", "hex")
            .expect("commented input still converts"),
        "34"
    );
}

#[test]
fn unit_conversion_uses_session_number_formatting() {
    let mut calculator = Calculator::new();
    calculator
        .load_definitions_from_dir(upstream_data_dir())
        .expect("unit catalog loads");

    calculator.set_formatting_approximation(ApproximationMode::Approximate);
    calculator.set_precision(3);
    assert_eq!(
        calculator
            .convert_and_print("1 m", "in")
            .expect("unit conversion respects precision"),
        "39.4 in"
    );

    calculator.set_formatting_approximation(ApproximationMode::TryExact);
    calculator.set_fraction_format(NumberFractionFormat::Fractional);
    assert_eq!(
        calculator
            .convert_and_print("1 m", "in")
            .expect("unit conversion respects fraction format"),
        "5000 / 127 in"
    );
}

#[test]
fn loaded_catalog_units_outside_the_legacy_prefilter_are_converted() {
    let mut calculator = Calculator::new();
    calculator
        .load_definitions_from_dir(upstream_data_dir())
        .expect("unit catalog loads");

    assert_eq!(
        calculator
            .convert_and_print("1 mol", "mol")
            .expect("loaded SI unit is handled by the unit engine"),
        "1 mol"
    );
}

#[test]
fn unresolved_symbolic_conversion_falls_back_after_catalog_loading() {
    let mut calculator = Calculator::new();
    calculator
        .load_definitions_from_dir(upstream_data_dir())
        .expect("unit catalog loads");

    assert_eq!(
        calculator
            .calculate_and_print("x to y")
            .expect("unsupported symbolic conversion remains printable"),
        "x to y"
    );
}

#[test]
fn unsupported_unit_probe_does_not_duplicate_session_messages() {
    let mut calculator = Calculator::new();
    calculator
        .load_definitions_from_dir(upstream_data_dir())
        .expect("unit catalog loads");

    let _ = calculator.calculate_and_print("(-1)! + m");
    let factorial_warnings = calculator
        .messages()
        .iter()
        .filter(|message| message.message().contains("Factorial requires"))
        .count();
    assert_eq!(factorial_warnings, 1);
}

#[test]
fn failed_definition_reload_preserves_the_previous_catalogs() {
    let mut calculator = Calculator::new();
    calculator
        .load_definitions_from_dir(upstream_data_dir())
        .expect("initial catalog load succeeds");

    let missing = upstream_data_dir().join("does-not-exist");
    assert!(calculator.load_definitions_from_dir(missing).is_err());
    assert!(calculator
        .definitions()
        .expect("previous definitions remain available")
        .function_by_name("sin")
        .is_some());
    assert_eq!(
        calculator
            .convert_and_print("1 m", "cm")
            .expect("previous unit registry remains active"),
        "100 cm"
    );
}
