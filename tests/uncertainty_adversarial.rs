use libqalculate_rust::number::{Float, Number, NumberValue, Rational};
use proptest::prelude::*;

// =========================================================================
// Category 1: Parsing Boundary & Malformed Inputs
// =========================================================================

#[test]
fn test_parsing_paren_form_variations() {
    // Standard parenthesis form
    let n1 = "1.23(4)".parse::<Number>().unwrap();
    assert_eq!(n1.to_string(), "1.230±0.040");

    // Parenthesis form with multiple decimal digits inside paren
    let n2 = "1.23(45)".parse::<Number>().unwrap();
    assert_eq!(n2.to_string(), "1.23±0.45");

    // Parenthesis form with no decimal point (integer base)
    // Note: Since unc = 4 (units digit), it gets formatted to 1 decimal place (tenths), hence 123.0±4.0
    let n3 = "123(4)".parse::<Number>().unwrap();
    assert_eq!(n3.to_string(), "123.0±4.0");

    // Parenthesis form with negative sign
    let n4 = "-1.23(4)".parse::<Number>().unwrap();
    assert_eq!(n4.to_string(), "-1.230±0.040");

    // Parenthesis form with exponent (scientific notation)
    let n5 = "1.2e-2(4)".parse::<Number>();
    if let Ok(num) = n5 {
        if let NumberValue::Uncertainty {
            value, uncertainty, ..
        } = num.value()
        {
            if let NumberValue::Float(vf) = &**value {
                assert_eq!(vf.value(), 0.012);
            }
            if let NumberValue::Float(uf) = &**uncertainty {
                assert!((uf.value() - 0.004).abs() < 1e-9);
            }
        }
    }
}

#[test]
fn test_parsing_paren_form_failures() {
    // Multiple decimal points
    assert!("1.2.3(4)".parse::<Number>().is_err());
    // Non-numeric characters inside parenthesis
    assert!("1.23(4a)".parse::<Number>().is_err());
    // Empty parenthesis
    assert!("1.23()".parse::<Number>().is_err());
    // Missing matching parenthesis
    assert!("1.23(4".parse::<Number>().is_err());
    assert!("1.234)".parse::<Number>().is_err());
}

#[test]
fn test_parsing_percentage_variations() {
    // Positive percentage uncertainty
    let n1 = "100 +/- 5%".parse::<Number>().unwrap();
    assert_eq!(n1.to_string(), "100.0±5.0%");

    // Zero percentage uncertainty
    let n2 = "100 +/- 0%".parse::<Number>().unwrap();
    assert_eq!(n2.to_string(), "100±0%");

    // Negative percentage uncertainty
    let n3 = "100 +/- -5%".parse::<Number>().unwrap();
    // Since -5% absolute is positive uncertainty, verify if it becomes positive
    if let NumberValue::Uncertainty { uncertainty, .. } = n3.value() {
        if let NumberValue::Float(uf) = &**uncertainty {
            assert_eq!(uf.value(), 5.0); // Should be positive absolute uncertainty
        }
    }
}

// =========================================================================
// Category 2: Propagation & Mathematical Operations
// =========================================================================

#[test]
fn test_uncertainty_propagation_basic() {
    // Addition
    let a = "10 +/- 3".parse::<Number>().unwrap();
    let b = "20 +/- 4".parse::<Number>().unwrap();
    let sum = a.add(&b);
    assert_eq!(sum.to_string(), "30.0±5.0"); // sqrt(3^2 + 4^2) = 5

    // Subtraction
    let diff = b.sub(&a);
    assert_eq!(diff.to_string(), "10.0±5.0"); // sqrt(3^2 + 4^2) = 5

    // Multiplication
    let c = "2 +/- 0.1".parse::<Number>().unwrap();
    let d = "3 +/- 0.2".parse::<Number>().unwrap();
    let prod = c.mul(&d);
    assert_eq!(prod.to_string(), "6.00±0.50");

    // Division (Number level, which canonicalizes real/imag and performs complex-expansion)
    let e = "6 +/- 0.5".parse::<Number>().unwrap();
    let f = "2 +/- 0.1".parse::<Number>().unwrap();
    let div_num = e.div(&f);
    if let NumberValue::Uncertainty {
        value, uncertainty, ..
    } = div_num.value()
    {
        // complex expansion computes r = ac / (c^2 + d^2) = (a * c) / (c * c)
        // val = 3.0
        // uncertainty = 0.3605551275463989
        if let NumberValue::Float(vf) = &**value {
            assert_eq!(vf.value(), 3.0);
        } else if let NumberValue::Rational(vr) = &**value {
            assert_eq!(vr.num(), 3);
        }
        if let NumberValue::Float(uf) = &**uncertainty {
            assert!((uf.value() - 0.36055512).abs() < 1e-6);
        }
    }

    // Division (NumberValue level, which performs direct division)
    let div_val = e.value().div(f.value());
    if let NumberValue::Uncertainty {
        value, uncertainty, ..
    } = div_val
    {
        // Direct division on two uncertainties:
        // val = 3
        // uncertainty = sqrt((0.5/2)^2 + (3*0.1/2)^2) = sqrt(0.085) = 0.29154759...
        if let NumberValue::Rational(vr) = &*value {
            assert_eq!(vr.num(), 3);
        }
        if let NumberValue::Float(uf) = &*uncertainty {
            assert!((uf.value() - 0.29154759).abs() < 1e-6);
        }
    }
}

#[test]
fn test_uncertainty_propagation_functions() {
    // Square root
    let a_val = NumberValue::Uncertainty {
        value: Box::new(NumberValue::Float(Float::from_f64(4.0, 53))),
        uncertainty: Box::new(NumberValue::Float(Float::from_f64(0.4, 53))),
        is_relative: false,
    };
    let sqrt_a = a_val.sqrt();
    if let NumberValue::Uncertainty {
        value, uncertainty, ..
    } = sqrt_a
    {
        if let NumberValue::Float(vf) = &*value {
            assert_eq!(vf.value(), 2.0);
        }
        if let NumberValue::Float(uf) = &*uncertainty {
            assert!((uf.value() - 0.1).abs() < 1e-9);
        }
    } else {
        panic!("Expected Uncertainty");
    }

    // Natural Logarithm
    let e_val = std::f64::consts::E;
    let b_val = NumberValue::Uncertainty {
        value: Box::new(NumberValue::Float(Float::from_f64(e_val, 53))),
        uncertainty: Box::new(NumberValue::Float(Float::from_f64(0.1 * e_val, 53))),
        is_relative: false,
    };
    let ln_b = b_val.ln();
    if let NumberValue::Uncertainty {
        value, uncertainty, ..
    } = ln_b
    {
        if let NumberValue::Float(vf) = &*value {
            assert!((vf.value() - 1.0).abs() < 1e-9);
        }
        if let NumberValue::Float(uf) = &*uncertainty {
            assert!((uf.value() - 0.1).abs() < 1e-9);
        }
    } else {
        panic!("Expected Uncertainty");
    }

    // Exponentiation with constant exponent: (4 +/- 0.2) ^ 0.5
    let c_val = NumberValue::Uncertainty {
        value: Box::new(NumberValue::Float(Float::from_f64(4.0, 53))),
        uncertainty: Box::new(NumberValue::Float(Float::from_f64(0.2, 53))),
        is_relative: false,
    };
    let exp_const = NumberValue::Float(Float::from_f64(0.5, 53));
    let pow_c = c_val.pow(&exp_const);
    if let NumberValue::Uncertainty {
        value, uncertainty, ..
    } = pow_c
    {
        if let NumberValue::Float(vf) = &*value {
            assert_eq!(vf.value(), 2.0);
        }
        if let NumberValue::Float(uf) = &*uncertainty {
            assert!((uf.value() - 0.05).abs() < 1e-9);
        }
    } else {
        panic!("Expected Uncertainty");
    }
}

// =========================================================================
// Category 3: Boundary & Extreme Values
// =========================================================================

#[test]
fn test_uncertainty_propagation_boundary_cases() {
    // Division by zero
    let a = "10 +/- 1".parse::<Number>().unwrap();
    let b = "0 +/- 0".parse::<Number>().unwrap();
    let div_zero = a.div(&b);
    assert!(div_zero.is_nan());

    let c = "0 +/- 0.5".parse::<Number>().unwrap();
    let div_zero_unc = a.div(&c);
    assert!(div_zero_unc.is_nan());

    // Square root of negative value
    let d_val = NumberValue::Uncertainty {
        value: Box::new(NumberValue::Float(Float::from_f64(-4.0, 53))),
        uncertainty: Box::new(NumberValue::Float(Float::from_f64(0.2, 53))),
        is_relative: false,
    };
    let sqrt_neg = d_val.sqrt();
    assert!(sqrt_neg.is_nan());

    // Natural log of zero and negative
    let e_val = NumberValue::Uncertainty {
        value: Box::new(NumberValue::Float(Float::from_f64(0.0, 53))),
        uncertainty: Box::new(NumberValue::Float(Float::from_f64(0.1, 53))),
        is_relative: false,
    };
    let ln_zero = e_val.ln();
    assert!(ln_zero.includes_infinity() || ln_zero.is_nan());

    let f_val = NumberValue::Uncertainty {
        value: Box::new(NumberValue::Float(Float::from_f64(-5.0, 53))),
        uncertainty: Box::new(NumberValue::Float(Float::from_f64(0.1, 53))),
        is_relative: false,
    };
    let ln_neg = f_val.ln();
    assert!(ln_neg.is_nan());

    // Operations with infinities
    let inf_val = Number::from_f64(f64::INFINITY);
    let unc_val = "5 +/- 0.5".parse::<Number>().unwrap();
    let sum_inf = inf_val.add(&unc_val);
    assert!(sum_inf.is_infinite());

    // Multiplication with zero uncertainty
    let zero_unc = "10 +/- 0".parse::<Number>().unwrap();
    let zero_unc_sum = zero_unc.add(&zero_unc);
    assert_eq!(zero_unc_sum.to_string(), "20±0");

    // Auditor specific cases:
    // 1. Exponentiating a negative base with exponent having 0 uncertainty
    let neg_base = NumberValue::Uncertainty {
        value: Box::new(NumberValue::Rational(Rational::from_i32(-2))),
        uncertainty: Box::new(NumberValue::Float(Float::from_f64(0.1, 53))),
        is_relative: false,
    };
    let zero_exp = NumberValue::Uncertainty {
        value: Box::new(NumberValue::Rational(Rational::from_i32(3))),
        uncertainty: Box::new(NumberValue::Rational(Rational::from_i32(0))),
        is_relative: false,
    };
    let res1 = neg_base.pow(&zero_exp);
    assert!(!res1.is_nan());
    if let NumberValue::Uncertainty { uncertainty, .. } = res1 {
        assert!(!uncertainty.is_nan());
    }

    // 2. Base negative constant, exponent with 0 uncertainty
    let neg_const = NumberValue::Rational(Rational::from_i32(-2));
    let res2 = neg_const.pow(&zero_exp);
    assert!(!res2.is_nan());
    if let NumberValue::Uncertainty { uncertainty, .. } = res2 {
        assert!(!uncertainty.is_nan());
    }

    // 3. sqrt(0 +/- 0)
    let zero_unc_val = NumberValue::Uncertainty {
        value: Box::new(NumberValue::Rational(Rational::from_i32(0))),
        uncertainty: Box::new(NumberValue::Rational(Rational::from_i32(0))),
        is_relative: false,
    };
    let sqrt_res = zero_unc_val.sqrt();
    assert!(!sqrt_res.is_nan());
    if let NumberValue::Uncertainty { uncertainty, .. } = sqrt_res {
        assert!(!uncertainty.is_nan());
    }

    // 4. ln(0 +/- 0)
    let ln_res = zero_unc_val.ln();
    if let NumberValue::Uncertainty {
        value, uncertainty, ..
    } = ln_res
    {
        assert!(value.is_infinite());
        assert!(!uncertainty.is_nan());
    }
}

// =========================================================================
// Category 4: Formatting Verification
// =========================================================================

#[test]
fn test_uncertainty_formatting_boundary_cases() {
    // Very small uncertainty
    let n1 = "1.00000000 +/- 0.00000001".parse::<Number>().unwrap();
    assert_eq!(n1.to_string(), "1.000000000±0.000000010");

    // Very large uncertainty
    let n2 = "1 +/- 1000000".parse::<Number>().unwrap();
    assert_eq!(n2.to_string(), "1±1000000");

    // Relative uncertainty mixed addition formatting
    let r1 = "100 +/- 5".parse::<Number>().unwrap();
    let r2 = "200 +/- 10%".parse::<Number>().unwrap();
    let r_sum = r1.add(&r2);
    assert_eq!(r_sum.to_string(), "300.0±6.9%");
}

// =========================================================================
// Category 5: Property-Based Tests
// =========================================================================

fn safe_float_strategy() -> impl Strategy<Value = f64> {
    0.1..1000.0
}

fn safe_unc_strategy() -> impl Strategy<Value = f64> {
    0.001..10.0
}

proptest! {
    #[test]
    fn prop_commutativity_of_addition(
        v1 in safe_float_strategy(),
        u1 in safe_unc_strategy(),
        v2 in safe_float_strategy(),
        u2 in safe_unc_strategy()
    ) {
        let a = Number::new_uncertainty(NumberValue::Float(Float::from_f64(v1, 53)), NumberValue::Float(Float::from_f64(u1, 53)), false);
        let b = Number::new_uncertainty(NumberValue::Float(Float::from_f64(v2, 53)), NumberValue::Float(Float::from_f64(u2, 53)), false);

        let sum1 = a.add(&b);
        let sum2 = b.add(&a);

        prop_assert_eq!(sum1, sum2);
    }

    #[test]
    fn prop_negation_identity(
        v in safe_float_strategy(),
        u in safe_unc_strategy()
    ) {
        let val = NumberValue::Uncertainty {
            value: Box::new(NumberValue::Float(Float::from_f64(v, 53))),
            uncertainty: Box::new(NumberValue::Float(Float::from_f64(u, 53))),
            is_relative: false,
        };
        let negated = val.negate();

        if let NumberValue::Uncertainty { value, uncertainty, .. } = negated {
            if let NumberValue::Float(vf) = &*value {
                prop_assert_eq!(vf.value(), -v);
            }
            if let NumberValue::Float(uf) = &*uncertainty {
                prop_assert_eq!(uf.value(), u);
            }
        } else {
            panic!("Expected Uncertainty variant");
        }
    }

    #[test]
    fn prop_constant_scaling(
        v in safe_float_strategy(),
        u in safe_unc_strategy(),
        c in safe_float_strategy()
    ) {
        let unc_val = Number::new_uncertainty(NumberValue::Float(Float::from_f64(v, 53)), NumberValue::Float(Float::from_f64(u, 53)), false);
        let scalar = Number::from_f64(c);

        let scaled = unc_val.mul(&scalar);

        if let NumberValue::Uncertainty { value, uncertainty, .. } = scaled.value() {
            if let NumberValue::Float(vf) = &**value {
                prop_assert!((vf.value() - (v * c)).abs() < 1e-9);
            }
            if let NumberValue::Float(uf) = &**uncertainty {
                prop_assert!((uf.value() - (u * c)).abs() < 1e-9);
            }
        } else {
            panic!("Expected Uncertainty variant");
        }
    }
}
