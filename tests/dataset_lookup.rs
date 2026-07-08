use libqalculate_rust::context::CalculatorContext;

fn eval_to_string(expr: &str) -> String {
    let mut context = CalculatorContext::new();
    context
        .parse_and_evaluate_to_string(expr)
        .unwrap_or_else(|error| panic!("failed to evaluate {expr:?}: {error}"))
}

fn eval_error(expr: &str) -> String {
    let mut context = CalculatorContext::new();
    context
        .parse_and_evaluate_to_string(expr)
        .expect_err("dataset lookup should fail")
}

#[test]
fn evaluates_focused_element_dataset_oracle_cases() {
    assert_eq!(eval_to_string("atom(H; mass)"), "1.008 u");
    assert_eq!(eval_to_string("atom(He; mass)"), "4.00260 u");
    assert_eq!(eval_to_string("atom(H; name)"), "\"Hydrogen\"");
    assert_eq!(eval_to_string("atom(1; symbol)"), "'H'");
    assert_eq!(eval_to_string("atom(Hydrogen; number)"), "1");
}

#[test]
fn evaluates_focused_planet_dataset_oracle_cases() {
    assert_eq!(eval_to_string("planet(Earth; radius)"), "6371.0 km");
    assert_eq!(eval_to_string("planet(Earth; gravity)"), "9.80665 m/s²");
    assert_eq!(eval_to_string("planet(Mars; mass)"), "6.4171E23 kg");
    assert_eq!(eval_to_string("planet(Pluto; mass)"), "1.3E22 kg");
}

#[test]
fn reports_unknown_dataset_object_like_upstream() {
    assert_eq!(
        eval_error("atom(Xx; mass)"),
        "Object Xx not available in data set."
    );
    assert_eq!(
        eval_error("planet(Vulcan; mass)"),
        "Object Vulcan not available in data set."
    );
}

#[test]
fn reports_unknown_dataset_property_like_upstream() {
    assert_eq!(
        eval_error("atom(H; missing)"),
        "Argument 2, Property, in atom() must be name of a data property (symbol, number, name, mass, boiling, melting, or density)."
    );
    assert_eq!(
        eval_error("planet(Earth; missing)"),
        "Argument 2, Property, in planet() must be name of a data property (name, year, speed, eccentricity, inclination, satellites, mass, density, area, gravity, temperature, or radius)."
    );
}

#[test]
fn unsupported_dataset_interval_displays_fail_closed() {
    assert_eq!(
        eval_error("atom(Li; mass)"),
        "Unsupported dataset interval display for [6.938, 6.997]."
    );
}

#[test]
fn unsupported_dataset_units_fail_closed() {
    assert_eq!(
        eval_error("atom(Fm; density)"),
        "Unsupported dataset unit g/cm^3."
    );
}
