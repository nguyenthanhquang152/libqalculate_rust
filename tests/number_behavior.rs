use libqalculate_rust::number::{evaluate_expr, Float, Number, NumberValue, Rational};

#[test]
fn rational_addition_canonicalizes_result() {
    let one_third = Number::from_rational(Rational::new(1, 3));
    let one_sixth = Number::from_rational(Rational::new(1, 6));

    if let NumberValue::Rational(r1) = one_third.value() {
        assert_eq!(r1.num(), 1);
        assert_eq!(r1.den(), 3);
    } else {
        panic!("Expected rational");
    }

    let result_val = one_third.value().add(one_sixth.value());
    if let NumberValue::Rational(res) = &result_val {
        assert_eq!(res.num(), 1);
        assert_eq!(res.den(), 2);
    } else {
        panic!("Expected rational result");
    }
}

#[test]
fn rational_from_str_exposes_lossless_arbitrary_precision_surface() {
    let rational = "340282366920938463463374607431768211456 / 6"
        .parse::<Rational>()
        .expect("arbitrary-precision rational should parse");

    assert_eq!(
        rational.numerator_string(),
        "170141183460469231731687303715884105728"
    );
    assert_eq!(rational.denominator_string(), "3");
    assert_eq!(
        Number::from_rational(rational).to_string(),
        "170141183460469231731687303715884105728/3"
    );

    assert!("1/0".parse::<Rational>().is_err());
    assert!("e1".parse::<Rational>().is_err());
}

#[test]
fn interval_literal_parsing_normalizes_reversed_bounds() {
    let number = "[5, 1]"
        .parse::<Number>()
        .expect("finite reversed interval bounds should parse");
    let NumberValue::Interval { lower, upper } = number.value() else {
        panic!("expected interval number");
    };

    assert_eq!(lower.value(), 1.0);
    assert_eq!(upper.value(), 5.0);
}

#[test]
fn interval_literal_parsing_collapses_equal_bounds_to_scalar() {
    let number = "[2, 2]"
        .parse::<Number>()
        .expect("equal finite interval bounds should parse");

    assert!(!number.is_interval());
    assert!(!number.approximate());
    let NumberValue::Rational(value) = number.value() else {
        panic!("expected equal exact interval bounds to collapse to a rational scalar");
    };
    assert_eq!(value.num(), 2);
    assert_eq!(value.den(), 1);
    assert_eq!(number.to_qalc_string(), "2");
}

#[test]
fn interval_function_is_numeric_primary_in_native_expression() {
    let result = evaluate_expr("interval(1;2) + interval(3;4)")
        .expect("interval(lower; upper) should parse as a numeric primary");

    assert_eq!(result.value().to_interval_bounds(), Some((4.0, 6.0)));
}

#[test]
fn interval_function_accepts_optional_excluded_endpoint_flag_for_integer_bounds() {
    for expression in ["interval(1;3;0)", "interval(1;3;1)"] {
        let result = evaluate_expr(expression)
            .expect("optional interval exclude flag should parse for integer bounds");

        assert_eq!(
            result.value().to_interval_bounds(),
            Some((1.0, 3.0)),
            "expression: {expression}"
        );
    }
}

#[test]
fn interval_function_rejects_optional_excluded_endpoint_flag_for_decimal_bounds() {
    for expression in ["interval(1.1;3.3;0)", "interval(1.1;3.3;1)"] {
        assert!(
            evaluate_expr(expression).is_err(),
            "decimal optional-bound expression should stay unsupported until outward rounding is modeled: {expression}"
        );
    }
}

#[test]
fn interval_function_rejects_unprobed_optional_excluded_endpoint_integer_bounds() {
    for expression in ["interval(2;5;1)", "interval(1;1;1)"] {
        assert!(
            evaluate_expr(expression).is_err(),
            "optional interval exclude flag should stay limited to oracle-backed integer rows: {expression}"
        );
    }
}

#[test]
fn interval_function_accepts_infinity_endpoints() {
    let lower_unbounded =
        evaluate_expr("interval(-infinity;5)").expect("lower infinity endpoint should parse");
    assert_eq!(
        lower_unbounded.value().to_interval_bounds(),
        Some((f64::NEG_INFINITY, 5.0))
    );

    let upper_unbounded =
        evaluate_expr("interval(4;infinity)").expect("upper infinity endpoint should parse");
    assert_eq!(
        upper_unbounded.value().to_interval_bounds(),
        Some((4.0, f64::INFINITY))
    );
}

#[test]
fn interval_arithmetic_propagates_infinity_endpoints() {
    for (expression, expected_bounds) in [
        (
            "interval(-infinity;5) + interval(2;3)",
            (f64::NEG_INFINITY, 8.0),
        ),
        (
            "interval(-infinity;5) - interval(2;3)",
            (f64::NEG_INFINITY, 3.0),
        ),
        (
            "interval(-infinity;5) * interval(2;3)",
            (f64::NEG_INFINITY, 15.0),
        ),
        ("interval(4;infinity) + interval(2;3)", (6.0, f64::INFINITY)),
        ("interval(4;infinity) - interval(2;3)", (1.0, f64::INFINITY)),
        ("interval(4;infinity) * interval(2;3)", (8.0, f64::INFINITY)),
        ("interval(4;infinity) / 2", (2.0, f64::INFINITY)),
    ] {
        let result = evaluate_expr(expression).expect("interval expression should evaluate");
        assert_eq!(
            result.value().to_interval_bounds(),
            Some(expected_bounds),
            "expression: {expression}"
        );
    }
}

#[test]
fn nested_complex_numbers_are_flattened() {
    let real_part = Number::new_complex(Number::from_i32(1), Number::from_i32(2));
    let imag_part = Number::new_complex(Number::from_i32(3), Number::from_i32(4));

    let complex_result = Number::new_complex(real_part, imag_part);

    assert!(complex_result.is_complex());
    let (r_val, i_val) = complex_result.to_canonical_real_imag();

    if let NumberValue::Rational(r) = r_val {
        assert_eq!(r.num(), -3);
        assert_eq!(r.den(), 1);
    } else {
        panic!("Expected rational real part");
    }

    if let NumberValue::Rational(i) = i_val {
        assert_eq!(i.num(), 5);
        assert_eq!(i.den(), 1);
    } else {
        panic!("Expected rational imaginary part");
    }
}

#[test]
fn float_addition_preserves_max_precision() {
    let f1 = NumberValue::Float(Float::from_f64(1.5, 24));
    let f2 = NumberValue::Float(Float::from_f64(2.5, 64));

    let sum = f1.add(&f2);
    if let NumberValue::Float(res) = sum {
        assert_eq!(res.value(), 4.0);
        assert_eq!(res.prec(), 64);
    } else {
        panic!("Expected float result");
    }
}

#[test]
fn uncertainty_addition_combines_value_and_uncertainty() {
    let v1 = NumberValue::Rational(Rational::new(5, 1));
    let u1 = NumberValue::Rational(Rational::new(1, 2));
    let unc1 = NumberValue::Uncertainty {
        value: Box::new(v1),
        uncertainty: Box::new(u1),
        is_relative: false,
    };

    let v2 = NumberValue::Rational(Rational::new(10, 1));
    let u2 = NumberValue::Rational(Rational::new(3, 2));
    let unc2 = NumberValue::Uncertainty {
        value: Box::new(v2),
        uncertainty: Box::new(u2),
        is_relative: false,
    };

    let sum = unc1.add(&unc2);
    if let NumberValue::Uncertainty {
        value, uncertainty, ..
    } = sum
    {
        if let NumberValue::Rational(r_val) = &*value {
            assert_eq!(r_val.num(), 15);
            assert_eq!(r_val.den(), 1);
        } else {
            panic!("Expected rational value");
        }

        if let NumberValue::Float(f_unc) = &*uncertainty {
            assert!((f_unc.value() - 1.58113883).abs() < 1e-6);
        } else {
            panic!("Expected float uncertainty");
        }
    } else {
        panic!("Expected uncertainty result");
    }
}
