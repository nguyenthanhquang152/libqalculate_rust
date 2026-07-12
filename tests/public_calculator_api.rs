use libqalculate_rust::messages::{CalculatorMessage, MessageStage, MessageType};
use libqalculate_rust::options::{
    ApproximationMode, EvaluationOptions, ParseOptions, PrintOptions,
};
use libqalculate_rust::Calculator;
use std::path::{Path, PathBuf};
use std::process::Command;

fn upstream_data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../libqalculate/data")
}

fn upstream_qalc() -> Option<PathBuf> {
    std::env::var_os("QALCULATE_ORACLE")
        .map(PathBuf::from)
        .or_else(|| Some(Path::new(env!("CARGO_MANIFEST_DIR")).join("../libqalculate/src/qalc")))
        .filter(|path| path.exists())
}

#[test]
fn native_calculator_constructs_parses_evaluates_and_prints() {
    let mut calculator = Calculator::new();

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
fn native_calculator_matches_the_upstream_simple_api_example() {
    let Some(qalc) = upstream_qalc() else {
        eprintln!("skipping public API differential; upstream qalc is not built");
        return;
    };

    let upstream = Command::new(qalc)
        .env("TZ", "UTC")
        .env("LC_ALL", "C.UTF-8")
        .env("LANG", "C.UTF-8")
        .env("QALCULATE_DEFINITIONS_DIR", upstream_data_dir())
        .args([
            "-defaults",
            "-terse",
            "-set",
            "decimal_comma 0",
            "-set",
            "curconv 0",
            "1 + 1",
        ])
        .output()
        .expect("upstream qalc starts");
    assert!(upstream.status.success());
    assert!(upstream.stderr.is_empty());

    let mut calculator = Calculator::new();
    let native = calculator
        .calculate_and_print("1 + 1")
        .expect("native example succeeds");
    assert_eq!(native, String::from_utf8_lossy(&upstream.stdout).trim());
}

#[test]
fn native_calculator_exposes_options_and_structured_messages() {
    let mut calculator = Calculator::new();

    let _: &ParseOptions = calculator.parse_options();
    let _: &EvaluationOptions = calculator.evaluation_options();
    let _: &PrintOptions = calculator.print_options();

    calculator.evaluation_options_mut().approximation = ApproximationMode::Approximate;
    calculator.set_precision(8);
    assert_eq!(
        calculator
            .calculate_and_print("1 / 3")
            .expect("approximate calculation succeeds"),
        "0.33333333"
    );

    calculator
        .apply_command("/set precision 12")
        .expect("typed session command applies");
    assert_eq!(calculator.precision(), 12);

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
    assert!(calculator
        .datasets()
        .expect("dataset catalog")
        .dataset_by_name("atom")
        .is_some());

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
    assert!(calculator
        .datasets()
        .expect("previous datasets remain available")
        .dataset_by_name("atom")
        .is_some());
    assert_eq!(
        calculator
            .convert_and_print("1 m", "cm")
            .expect("previous unit registry remains active"),
        "100 cm"
    );
}
