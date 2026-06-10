use super::*;

#[test]
fn test_constructors() {
    let n_zero = Number::new();
    assert!(n_zero.is_zero());
    assert!(n_zero.is_real_zero());
    assert!(!n_zero.is_one());
    assert!(!n_zero.is_real_one());
    assert!(!n_zero.is_complex());
    assert!(n_zero.has_real_part());
    assert!(!n_zero.has_imaginary_part());
    assert!(!n_zero.is_nan());
    assert!(!n_zero.is_infinite());

    let r = Rational::from_i32(1);
    let n_r = Number::from_rational(r);
    assert!(!n_r.is_zero());
    assert!(!n_r.is_real_zero());
    assert!(n_r.is_one());
    assert!(n_r.is_real_one());

    let f = Float::from_f64(1.0, 53);
    let n_f = Number::from_float(f);
    assert!(!n_f.is_zero());
    assert!(n_f.is_one());
    assert!(n_f.is_real_one());

    let n_i32 = Number::from_i32(42);
    if let NumberValue::Rational(rat) = n_i32.value() {
        assert_eq!(rat.num(), 42);
        assert_eq!(rat.den(), 1);
    } else {
        panic!("Expected rational");
    }

    let n_f64 = Number::from_f64(1.23);
    if let NumberValue::Float(fl) = n_f64.value() {
        assert_eq!(fl.value(), 1.23);
        assert_eq!(fl.prec(), 53);
    } else {
        panic!("Expected float");
    }
}

#[test]
fn test_special_floats() {
    let n_nan = Number::from_f64(f64::NAN);
    assert!(n_nan.is_nan());
    assert!(!n_nan.is_infinite());

    let n_inf = Number::from_f64(f64::INFINITY);
    assert!(n_inf.is_infinite());
    assert!(!n_nan.is_infinite());
    assert!(!n_inf.is_nan());

    let n_neginf = Number::from_f64(f64::NEG_INFINITY);
    assert!(n_neginf.is_infinite());
    assert!(!n_neginf.is_nan());
    assert_eq!(n_neginf.value(), &NumberValue::MinusInfinity);
}

#[test]
fn test_intervals() {
    let lower = Float::from_f64(1.0, 53);
    let upper = Float::from_f64(2.0, 53);
    let n_interval = Number::new_interval(lower, upper);
    assert!(n_interval.is_interval());
    assert!(!n_interval.is_zero());
    assert!(!n_interval.is_one());

    let infinite_interval =
        Number::new_interval(Float::from_f64(0.0, 53), Float::from_f64(f64::INFINITY, 53));
    assert!(!infinite_interval.is_infinite());
    assert!(infinite_interval.includes_infinity());

    let real = Number::from_i32(0);
    let imag = Number::new_interval(Float::from_f64(-1.0, 53), Float::from_f64(1.0, 53));
    let complex_interval = Number::new_complex(real, imag);
    assert!(complex_interval.is_interval());
}

#[test]
fn test_complex() {
    let real = Number::from_i32(3);
    let imag = Number::from_i32(4);
    let c = Number::new_complex(real, imag);

    assert!(c.is_complex());
    assert!(c.has_real_part());
    assert!(c.has_imaginary_part());
    assert!(!c.is_zero());
    assert!(!c.is_one());

    let mut pure_imag = Number::from_i32(5);
    pure_imag.is_imaginary = true;
    assert!(pure_imag.is_complex());
    assert!(!pure_imag.has_real_part());
    assert!(pure_imag.has_imaginary_part());
    assert!(pure_imag.is_real_zero());
    assert!(!pure_imag.is_real_one());
}

#[test]
fn test_float_partial_eq() {
    let f1 = Float::from_f64(f64::NAN, 53);
    let f2 = Float::from_f64(f64::NAN, 53);
    assert_eq!(f1, f2);

    let f3 = Float::from_f64(1.0, 53);
    let f4 = Float::from_f64(1.0, 53);
    assert_eq!(f3, f4);
    assert_ne!(f1, f3);
}

#[test]
fn test_default() {
    let n_default = Number::default();
    assert!(n_default.is_zero());
}

#[test]
fn test_extra_coverage() {
    // Test Float::is_zero
    let f_zero = Float::from_f64(0.0, 53);
    assert!(f_zero.is_zero());
    assert!(!f_zero.is_one());

    // Test Float::is_one
    let f_one = Float::from_f64(1.0, 53);
    assert!(f_one.is_one());
    assert!(!f_one.is_zero());

    // Test is_real_zero with Float and Interval and NaN
    let n_f_zero = Number::from_float(f_zero);
    assert!(n_f_zero.is_real_zero());

    let n_f_one = Number::from_float(f_one);
    assert!(!n_f_one.is_real_zero());

    let zero_interval = Number::new_interval(Float::from_f64(0.0, 53), Float::from_f64(0.0, 53));
    assert!(zero_interval.is_real_zero());
    assert!(zero_interval.is_zero());

    let non_zero_interval =
        Number::new_interval(Float::from_f64(0.0, 53), Float::from_f64(1.0, 53));
    assert!(!non_zero_interval.is_real_zero());
    assert!(!non_zero_interval.is_zero());

    let nan_num = Number::from_f64(f64::NAN);
    assert!(!nan_num.is_real_zero());
    assert!(!nan_num.is_real_one());

    // Test is_zero with purely imaginary Number having Float or Interval values
    let mut imag_float_zero = Number::from_float(Float::from_f64(0.0, 53));
    imag_float_zero.is_imaginary = true;
    assert!(imag_float_zero.is_zero());

    let mut imag_float_non_zero = Number::from_float(Float::from_f64(1.0, 53));
    imag_float_non_zero.is_imaginary = true;
    assert!(!imag_float_non_zero.is_zero());

    let mut imag_interval_zero =
        Number::new_interval(Float::from_f64(0.0, 53), Float::from_f64(0.0, 53));
    imag_interval_zero.is_imaginary = true;
    assert!(imag_interval_zero.is_zero());

    let mut imag_interval_non_zero =
        Number::new_interval(Float::from_f64(0.0, 53), Float::from_f64(1.0, 53));
    imag_interval_non_zero.is_imaginary = true;
    assert!(!imag_interval_non_zero.is_zero());

    let mut imag_nan = Number::from_f64(f64::NAN);
    imag_nan.is_imaginary = true;
    assert!(!imag_nan.is_zero());

    // Test is_infinite and is_nan recursive checks on imaginary part
    let inf_imag = Number::new_complex(Number::from_i32(0), Number::from_f64(f64::INFINITY));
    assert!(inf_imag.is_infinite());

    let nan_imag = Number::new_complex(Number::from_i32(0), Number::from_f64(f64::NAN));
    assert!(nan_imag.is_nan());
}

#[test]
fn test_rational_normalization() {
    // Test normal reductions
    let r1 = Rational::new(2, 2);
    assert_eq!(r1.num(), 1);
    assert_eq!(r1.den(), 1);

    let r2 = Rational::new(2, -4);
    assert_eq!(r2.num(), -1);
    assert_eq!(r2.den(), 2);

    let r3 = Rational::new(0, -5);
    assert_eq!(r3.num(), 0);
    assert_eq!(r3.den(), 1);

    let r4 = Rational::new(-10, -20);
    assert_eq!(r4.num(), 1);
    assert_eq!(r4.den(), 2);
}

#[test]
fn test_uncertainty_modeling() {
    let val = NumberValue::Rational(Rational::new(5, 1));
    let unc = NumberValue::Rational(Rational::new(1, 2));
    let n = Number::new_uncertainty(val, unc, false);

    assert!(n.approximate());
    if let NumberValue::Uncertainty {
        value, uncertainty, ..
    } = n.value()
    {
        if let NumberValue::Rational(r_val) = &**value {
            assert_eq!(r_val.num(), 5);
        } else {
            panic!("Expected Rational");
        }
        if let NumberValue::Rational(r_unc) = &**uncertainty {
            assert_eq!(r_unc.num(), 1);
            assert_eq!(r_unc.den(), 2);
        } else {
            panic!("Expected Rational");
        }
    } else {
        panic!("Expected Uncertainty variant");
    }
}

#[test]
fn test_mathematical_value_equality() {
    // Mixed exact/inexact equality is intentionally conservative in the scaffold.
    let n1 = Number::from_rational(Rational::new(1, 1));
    let mut n2 = Number::from_float(Float::from_f64(1.0, 53));
    n2.precision = 100;
    n2.approximate = true;
    assert_ne!(n1, n2);

    let r_val = Number::from_rational(Rational::new(3, 2));
    let f_val = Number::from_float(Float::from_f64(1.5, 53));
    assert_ne!(r_val, f_val);

    // Point interval equality
    let interval_pt = Number::new_interval(Float::from_f64(2.5, 53), Float::from_f64(2.5, 53));
    let scalar_val = Number::from_float(Float::from_f64(2.5, 53));
    assert_eq!(interval_pt, scalar_val);

    // Complex with zero imaginary part equals real
    let real_num = Number::from_i32(3);
    let zero_imag = Number::from_i32(0);
    let complex_num = Number::new_complex(real_num.clone(), zero_imag);
    assert_eq!(complex_num, real_num);

    // NaNs compare as equal
    let nan1 = Number::from_f64(f64::NAN);
    let nan2 = Number::from_f64(f64::NAN);
    assert_eq!(nan1, nan2);
}

#[test]
fn test_complex_flattening() {
    // (a + bi) + (c + di)i = (a - d) + (b + c)i
    // (3 + 2i) + (4 + 5i)i = (3 - 5) + (2 + 4)i = -2 + 6i
    let real = Number::new_complex(Number::from_i32(3), Number::from_i32(2));
    let imag = Number::new_complex(Number::from_i32(4), Number::from_i32(5));
    let flattened = Number::new_complex(real, imag);

    let expected_real = NumberValue::Rational(Rational::new(-2, 1));
    let expected_imag = NumberValue::Rational(Rational::new(6, 1));

    assert_eq!(flattened.value(), &expected_real);
    assert_eq!(flattened.imaginary().unwrap().value(), &expected_imag);
}

#[test]
fn test_predicates_hardening() {
    // is_real_zero
    let rat_zero = NumberValue::Rational(Rational::new(0, 1));
    let fl_zero = NumberValue::Float(Float::from_f64(0.0, 53));
    let interval_zero = NumberValue::Interval {
        lower: Float::from_f64(0.0, 53),
        upper: Float::from_f64(0.0, 53),
    };
    let unc_zero = NumberValue::Uncertainty {
        value: Box::new(rat_zero.clone()),
        uncertainty: Box::new(rat_zero.clone()),
        is_relative: false,
    };
    assert!(rat_zero.is_real_zero());
    assert!(fl_zero.is_real_zero());
    assert!(interval_zero.is_real_zero());
    assert!(unc_zero.is_real_zero());

    // is_real_one
    let rat_one = NumberValue::Rational(Rational::new(1, 1));
    let fl_one = NumberValue::Float(Float::from_f64(1.0, 53));
    let interval_one = NumberValue::Interval {
        lower: Float::from_f64(1.0, 53),
        upper: Float::from_f64(1.0, 53),
    };
    let unc_one = NumberValue::Uncertainty {
        value: Box::new(rat_one.clone()),
        uncertainty: Box::new(rat_zero.clone()),
        is_relative: false,
    };
    assert!(rat_one.is_real_one());
    assert!(fl_one.is_real_one());
    assert!(interval_one.is_real_one());
    assert!(unc_one.is_real_one());

    // is_infinite
    let inf_val = NumberValue::PlusInfinity;
    let unc_inf = NumberValue::Uncertainty {
        value: Box::new(inf_val.clone()),
        uncertainty: Box::new(rat_zero.clone()),
        is_relative: false,
    };
    assert!(inf_val.is_infinite());
    assert!(unc_inf.is_infinite());

    let interval_inf = NumberValue::Interval {
        lower: Float::from_f64(0.0, 53),
        upper: Float::from_f64(f64::INFINITY, 53),
    };
    assert!(!interval_inf.is_infinite());
    assert!(interval_inf.includes_infinity());

    // is_nan
    let nan_val = NumberValue::NaN;
    let unc_nan = NumberValue::Uncertainty {
        value: Box::new(nan_val.clone()),
        uncertainty: Box::new(rat_zero.clone()),
        is_relative: false,
    };
    assert!(nan_val.is_nan());
    assert!(unc_nan.is_nan());

    // is_complex / has_imaginary_part / has_real_part
    let c_zero = Number::new_complex(Number::from_i32(3), Number::from_i32(0));
    assert!(!c_zero.is_complex());
    assert!(!c_zero.has_imaginary_part());
    assert!(c_zero.has_real_part());

    let c_non_zero = Number::new_complex(Number::from_i32(3), Number::from_i32(4));
    assert!(c_non_zero.is_complex());
    assert!(c_non_zero.has_imaginary_part());
    assert!(c_non_zero.has_real_part());

    let pure_imag = Number::new_complex(Number::from_i32(0), Number::from_i32(4));
    assert!(pure_imag.is_complex());
    assert!(pure_imag.has_imaginary_part());
    assert!(!pure_imag.has_real_part());
}

#[test]
fn test_complex_bug() {
    let mut pure_imag = Number::from_i32(5);
    pure_imag.is_imaginary = true;
    let c = Number::new_complex(pure_imag, Number::from_i32(0));
    assert!(!c.is_zero());
    assert!(c.is_complex());
}

#[test]
fn test_float_equality_ignores_precision() {
    let f1 = Float::from_f64(1.5, 53);
    let f2 = Float::from_f64(1.5, 128);
    assert_eq!(f1, f2);
}

#[test]
fn test_gcd_and_canonicalize_limits() {
    // Test canonicalize with num = i128::MIN and den = 1 (representable)
    let r = Rational::new(i128::MIN, 1);
    assert_eq!(r.num(), i128::MIN);
    assert_eq!(r.den(), 1);
}

#[test]
fn test_rational_arithmetic_no_overflow() {
    let max = NumberValue::Rational(Rational::new(i128::MAX, 1));
    let one = NumberValue::Rational(Rational::new(1, 1));
    let sum = max.add(&one);
    assert!(!sum.is_nan());
    if let NumberValue::Rational(r) = sum {
        let expected = rug::Integer::from(i128::MAX) + 1_i32;
        assert_eq!(r.value.numer().to_string(), expected.to_string());
    } else {
        panic!("Expected rational");
    }

    let min = NumberValue::Rational(Rational::new(i128::MIN, 1));
    let negated = min.negate();
    assert!(!negated.is_nan());
    if let NumberValue::Rational(r) = negated {
        let expected = -rug::Integer::from(i128::MIN);
        assert_eq!(r.value.numer().to_string(), expected.to_string());
    } else {
        panic!("Expected rational");
    }
}

#[test]
fn test_arbitrary_precision_rationals_do_not_fall_back_to_i128_surface() {
    let beyond_i128 = (rug::Integer::from(i128::MAX) + 1_i32).to_string();

    let parsed = beyond_i128
        .parse::<Number>()
        .expect("arbitrary-size integer literal should parse as an exact rational");
    assert!(!parsed.approximate());
    assert_eq!(parsed.to_string(), beyond_i128);
    if let NumberValue::Rational(r) = parsed.value() {
        assert_eq!(r.value.numer().to_string(), beyond_i128);
        assert_eq!(r.value.denom().to_string(), "1");
    } else {
        panic!("Expected rational");
    }

    let sum = NumberValue::Rational(Rational::new(i128::MAX, 1))
        .add(&NumberValue::Rational(Rational::new(1, 1)));
    assert_eq!(sum.to_string(), beyond_i128);
    assert_eq!(
        sum.partial_cmp(&NumberValue::Rational(Rational::new(i128::MAX, 1))),
        Some(std::cmp::Ordering::Greater)
    );

    let NumberValue::Rational(parsed_rational) = parsed.value() else {
        panic!("Expected rational");
    };
    let one = Rational::new(1, 1);
    assert_eq!(
        parsed_rational
            .checked_add(&one)
            .expect("arbitrary-precision checked add should remain exact")
            .numerator_string(),
        (rug::Integer::from(i128::MAX) + 2_i32).to_string()
    );
    assert_eq!(
        parsed_rational
            .checked_sub(&one)
            .expect("arbitrary-precision checked sub should remain exact")
            .numerator_string(),
        i128::MAX.to_string()
    );
    assert_eq!(
        parsed_rational
            .checked_mul(&one)
            .expect("arbitrary-precision checked mul should remain exact")
            .numerator_string(),
        beyond_i128
    );
    assert_eq!(
        parsed_rational
            .checked_div(&one)
            .expect("arbitrary-precision checked div should remain exact")
            .numerator_string(),
        beyond_i128
    );
}

#[test]
fn evaluate_expr_handles_grouping_parentheses() {
    let result = evaluate_expr("(1 + 2) * 4").expect("grouped expression should parse");
    assert_eq!(result.to_string(), "12");
}

#[test]
fn evaluate_expr_rejects_trailing_literals_and_respects_unary_precedence() {
    assert!(evaluate_expr("1 2").is_err());
    assert!(evaluate_expr("1(2)3").is_err());
    assert!(evaluate_expr("1.23(4)5").is_err());
    assert!(evaluate_expr("1(2)%").is_err());

    let result = evaluate_expr("-2^2").expect("expression should parse");
    assert_eq!(result.to_string(), "-4");
}

#[test]
fn exact_division_by_zero_yields_nan_not_fabricated_infinity() {
    let result = evaluate_expr("1 / 0").expect("expression should parse");
    assert!(result.is_nan());
}

#[test]
fn exact_integer_powers_remain_rational_and_parse_starstar() {
    let power = evaluate_expr("2 ^ 20").expect("integer power expression should parse");
    assert_eq!(
        power.value(),
        &NumberValue::Rational(Rational::new(1_048_576, 1))
    );

    let reciprocal = evaluate_expr("2 ^ -3").expect("negative integer power should parse");
    assert_eq!(
        reciprocal.value(),
        &NumberValue::Rational(Rational::new(1, 8))
    );

    let fractional_base =
        evaluate_expr("(2 / 3) ^ -2").expect("fractional rational power should parse");
    assert_eq!(
        fractional_base.value(),
        &NumberValue::Rational(Rational::new(9, 4))
    );

    assert_eq!(evaluate_expr("5 ** 3").unwrap().to_qalc_string(), "125");
    assert_eq!(
        evaluate_expr("4 ** 3 ** 2").unwrap().to_qalc_string(),
        "262144"
    );
}

#[test]
fn exact_integer_powers_have_result_size_guard() {
    let guarded = evaluate_expr("1e5000 ^ 1000").expect("guarded power expression should parse");
    assert!(
        matches!(guarded.value(), NumberValue::Float(_)),
        "oversized exact rational power should fall back to approximate path"
    );
}

#[test]
fn decimal_and_scientific_literals_parse_without_f64_loss() {
    let decimal = "0.01"
        .parse::<Number>()
        .expect("decimal literal should parse");
    assert!(!decimal.approximate());
    assert_eq!(decimal.to_string(), "0.01");
    let NumberValue::Rational(r) = decimal.value() else {
        panic!("Expected rational decimal storage");
    };
    assert_eq!(r.value.numer().to_string(), "1");
    assert_eq!(r.value.denom().to_string(), "100");

    let sum = evaluate_expr(".123 + 1").expect("decimal expression should parse");
    assert_eq!(sum.to_string(), "1.123");

    let scientific = evaluate_expr("1e3 + 2").expect("scientific expression should parse");
    assert_eq!(scientific.to_string(), "1002");
}

#[test]
fn terminating_rationals_use_qalc_decimal_output_shape() {
    assert_eq!(evaluate_expr("1/2").unwrap().to_string(), "0.5");
    assert_eq!(evaluate_expr("2/25").unwrap().to_string(), "0.08");
    assert_eq!(evaluate_expr("10/2").unwrap().to_string(), "5");
    assert_eq!(evaluate_expr("1/3").unwrap().to_string(), "1/3");
}

#[test]
fn qalc_profile_formats_nonterminating_and_large_rationals_like_upstream() {
    assert_eq!(
        evaluate_expr("1/3").unwrap().to_qalc_string(),
        "0.3333333333"
    );
    assert_eq!(
        evaluate_expr("1e10").unwrap().to_qalc_string(),
        "10000000000"
    );
    assert_eq!(evaluate_expr("1e303").unwrap().to_qalc_string(), "1E303");
    assert_eq!(evaluate_expr("2e303").unwrap().to_qalc_string(), "2E303");
    assert_eq!(evaluate_expr("12e303").unwrap().to_qalc_string(), "1.2E304");
    assert_eq!(
        evaluate_expr("123456789012345").unwrap().to_qalc_string(),
        "1.234567890E14"
    );
    assert_eq!(
        evaluate_expr("129999999999999").unwrap().to_qalc_string(),
        "1.300000000E14"
    );
}

#[test]
fn displayed_special_values_parse_roundtrip() {
    let inf = Number::from_f64(f64::INFINITY);
    let parsed_inf = inf.to_string().parse::<Number>().unwrap();
    assert!(parsed_inf.is_infinite());
    assert_eq!(parsed_inf.value(), &NumberValue::PlusInfinity);

    let neg_inf = Number::from_f64(f64::NEG_INFINITY);
    let parsed_neg_inf = neg_inf.to_string().parse::<Number>().unwrap();
    assert!(parsed_neg_inf.is_infinite());
    assert_eq!(parsed_neg_inf.value(), &NumberValue::MinusInfinity);

    let nan = Number::from_f64(f64::NAN);
    let parsed_nan = nan.to_string().parse::<Number>().unwrap();
    assert!(parsed_nan.is_nan());
}

#[test]
fn scientific_literals_with_impractical_exponents_are_rejected() {
    assert!("1e4097".parse::<Number>().is_ok());
    assert!("1e-4097".parse::<Number>().is_ok());
    assert_eq!(
        "1e5000".parse::<Number>().unwrap().to_qalc_string(),
        "1E5000"
    );
    assert!("1e-5000".parse::<Number>().is_ok());
    assert!("1e10000".parse::<Number>().is_ok());
    assert!("1e-10000".parse::<Number>().is_ok());
    assert!("1e10001".parse::<Number>().is_err());
    assert!("1e-10001".parse::<Number>().is_err());
    assert!("1e2147483647".parse::<Number>().is_err());
    assert!("1e-2147483648".parse::<Number>().is_err());
    assert!("e1".parse::<Number>().is_err());
    assert!("+e1".parse::<Number>().is_err());
    assert!(".e1".parse::<Number>().is_err());
    assert!("- . e1".parse::<Number>().is_err());

    assert_eq!("1e303".parse::<Number>().unwrap().to_qalc_string(), "1E303");
    assert_eq!("2e303".parse::<Number>().unwrap().to_qalc_string(), "2E303");
}

#[test]
fn exact_large_rational_compare_does_not_collapse_to_f64_infinity() {
    let smaller = "1e10000".parse::<Number>().unwrap();
    let larger = "2e10000".parse::<Number>().unwrap();

    assert_eq!(
        smaller.value().compare(larger.value()),
        ComparisonResult::Greater
    );
    assert_eq!(
        larger.value().compare(smaller.value()),
        ComparisonResult::Less
    );
    assert_ne!(
        smaller.value().compare(larger.value()),
        ComparisonResult::Equal
    );

    let zero = NumberValue::Rational(Rational::new(0, 1));
    let smaller_uncertain = NumberValue::Uncertainty {
        value: Box::new(smaller.value().clone()),
        uncertainty: Box::new(zero.clone()),
        is_relative: false,
    };
    let larger_uncertain = NumberValue::Uncertainty {
        value: Box::new(larger.value().clone()),
        uncertainty: Box::new(zero),
        is_relative: false,
    };

    assert_eq!(
        smaller_uncertain.compare(&larger_uncertain),
        ComparisonResult::Greater
    );
    assert_eq!(
        larger_uncertain.compare(&smaller_uncertain),
        ComparisonResult::Less
    );
}

#[test]
fn simple_imaginary_literals_parse_natively() {
    assert_eq!(evaluate_expr("i").unwrap().to_string(), "i");
    assert_eq!(evaluate_expr("-i").unwrap().to_string(), "-i");
    assert_eq!(evaluate_expr("5i").unwrap().to_string(), "5i");
    assert_eq!(
        evaluate_expr("(1 + 2i) + (3 + 4i)").unwrap().to_string(),
        "4 + 6i"
    );
    assert_eq!(
        evaluate_expr("(1 + 2i) * (3 + 4i)").unwrap().to_string(),
        "-5 + 10i"
    );
    assert_eq!(
        evaluate_expr("(1 + 2i) / (3 + 4i)").unwrap().to_string(),
        "0.44 + 0.08i"
    );
}

#[test]
fn internal_interval_literals_parse_for_scaffold() {
    let comma_interval = evaluate_expr("[1,2]").unwrap();
    assert!(comma_interval.is_interval());
    assert_eq!(comma_interval.to_qalc_string(), "[1  2]");
    assert_eq!(evaluate_expr("[1 2]").unwrap().to_qalc_string(), "[1  2]");
    assert_eq!(evaluate_expr("[-1,2]").unwrap().to_qalc_string(), "[-1  2]");
    assert_eq!(
        evaluate_expr("[1,2] + [3,4]").unwrap().to_qalc_string(),
        "[4  6]"
    );
    assert_eq!(evaluate_expr("[2,1]").unwrap().to_qalc_string(), "[1  2]");
}

#[test]
fn test_canonicalize_no_overflow_panics() {
    // i128::MIN / -1 = 2^127 which overflows i128, but fits in rug::Rational
    let r = Rational::new(i128::MIN, -1);
    let expected = -rug::Integer::from(i128::MIN);
    assert_eq!(r.value.numer().to_string(), expected.to_string());
    assert_eq!(r.value.denom().to_string(), "1");
}

#[test]
#[should_panic]
fn test_canonicalize_den_zero_panics() {
    let _r = Rational::new(5, 0);
}

#[test]
fn test_deep_cloning_semantics() {
    // 1. Complex deep clone check (Option<Box<Number>> deep clone)
    let real = Number::from_i32(3);
    let imag = Number::from_i32(4);
    let orig_complex = Number::new_complex(real, imag);
    let cloned_complex = orig_complex.clone();

    assert_eq!(orig_complex, cloned_complex);

    let orig_imag = orig_complex.imaginary().expect("Expected imaginary part");
    let cloned_imag = cloned_complex.imaginary().expect("Expected imaginary part");

    let addr_orig_imag = orig_imag as *const Number;
    let addr_cloned_imag = cloned_imag as *const Number;
    assert_ne!(
        addr_orig_imag, addr_cloned_imag,
        "Imaginary part heap address must be different after cloning (deep clone)"
    );

    // 2. Uncertainty deep clone check (Box<NumberValue> deep clone)
    let val = NumberValue::Rational(Rational::new(5, 1));
    let unc = NumberValue::Rational(Rational::new(1, 2));
    let orig_unc = Number::new_uncertainty(val, unc, false);
    let cloned_unc = orig_unc.clone();

    assert_eq!(orig_unc, cloned_unc);

    if let (
        NumberValue::Uncertainty {
            value: orig_v,
            uncertainty: orig_u,
            ..
        },
        NumberValue::Uncertainty {
            value: cloned_v,
            uncertainty: cloned_u,
            ..
        },
    ) = (orig_unc.value(), cloned_unc.value())
    {
        let addr_orig_v = &**orig_v as *const NumberValue;
        let addr_cloned_v = &**cloned_v as *const NumberValue;
        assert_ne!(
            addr_orig_v, addr_cloned_v,
            "Uncertainty value heap address must be different after cloning"
        );

        let addr_orig_u = &**orig_u as *const NumberValue;
        let addr_cloned_u = &**cloned_u as *const NumberValue;
        assert_ne!(
            addr_orig_u, addr_cloned_u,
            "Uncertainty uncertainty heap address must be different after cloning"
        );
    } else {
        panic!("Expected Uncertainty variant");
    }
}

#[test]
fn test_numeric_variants_invariants_and_constructors() {
    // 1. Finite exact rational invariants
    let r_pos = Rational::new(6, 8);
    assert_eq!(r_pos.num(), 3);
    assert_eq!(r_pos.den(), 4);
    assert!(r_pos.num() > 0);
    assert!(r_pos.den() > 0);

    let r_neg = Rational::new(6, -8);
    assert_eq!(r_neg.num(), -3);
    assert_eq!(r_neg.den(), 4);

    let r_zero = Rational::new(0, 5);
    assert_eq!(r_zero.num(), 0);
    assert_eq!(r_zero.den(), 1);
    assert!(r_zero.is_zero());

    // 2. Arbitrary float invariants
    let f1 = Float::from_f64(3.15, 128);
    assert_eq!(f1.value(), 3.15);
    assert_eq!(f1.prec(), 128);
    assert!(!f1.is_zero());
    assert!(!f1.is_one());
    assert!(!f1.is_nan());
    assert!(!f1.is_infinite());

    let f_zero = Float::from_f64(0.0, 64);
    assert!(f_zero.is_zero());

    // 3. Complex invariants
    let c_real = Number::from_i32(1);
    let c_imag = Number::from_i32(2);
    let c = Number::new_complex(c_real, c_imag);
    assert!(c.is_complex());
    assert!(c.has_real_part());
    assert!(c.has_imaginary_part());
    assert_eq!(c.value(), &NumberValue::Rational(Rational::new(1, 1)));
    assert_eq!(
        c.imaginary().unwrap().value(),
        &NumberValue::Rational(Rational::new(2, 1))
    );

    // 4. Interval invariants
    let lower = Float::from_f64(-1.0, 53);
    let upper = Float::from_f64(1.0, 53);
    let interval = Number::new_interval(lower, upper);
    assert!(interval.is_interval());
    if let NumberValue::Interval { lower: l, upper: u } = interval.value() {
        assert_eq!(l.value(), -1.0);
        assert_eq!(u.value(), 1.0);
    } else {
        panic!("Expected Interval variant");
    }

    // 5. Uncertainty invariants
    let val_val = NumberValue::Float(Float::from_f64(10.0, 53));
    let unc_val = NumberValue::Float(Float::from_f64(0.5, 53));
    let unc = Number::new_uncertainty(val_val, unc_val, false);
    assert!(unc.approximate());
    if let NumberValue::Uncertainty {
        value: v,
        uncertainty: u,
        ..
    } = unc.value()
    {
        if let NumberValue::Float(vf) = &**v {
            assert_eq!(vf.value(), 10.0);
        } else {
            panic!("Expected Float value");
        }
        if let NumberValue::Float(uf) = &**u {
            assert_eq!(uf.value(), 0.5);
        } else {
            panic!("Expected Float uncertainty");
        }
    } else {
        panic!("Expected Uncertainty variant");
    }

    // 6. Infinity variants
    let plus_inf = Number::from_f64(f64::INFINITY);
    assert!(plus_inf.is_infinite());
    assert!(plus_inf.includes_infinity());
    assert_eq!(plus_inf.value(), &NumberValue::PlusInfinity);

    let minus_inf = Number::from_f64(f64::NEG_INFINITY);
    assert!(minus_inf.is_infinite());
    assert!(minus_inf.includes_infinity());
    assert_eq!(minus_inf.value(), &NumberValue::MinusInfinity);

    // 7. NaN variant
    let nan_val = Number::from_f64(f64::NAN);
    assert!(nan_val.is_nan());
    assert_eq!(nan_val.value(), &NumberValue::NaN);
}

#[test]
fn test_adversarial_challenger_cases() {
    // 1. Extreme Rational Values and Boundary Conditions
    let r_max = Rational::new(i128::MAX, 1);
    assert_eq!(r_max.num(), i128::MAX);
    assert_eq!(r_max.den(), 1);

    let r_min = Rational::new(i128::MIN, 1);
    assert_eq!(r_min.num(), i128::MIN);
    assert_eq!(r_min.den(), 1);

    // Rational close to limits but representable: i128::MIN + 1 / -1 should reduce to i128::MAX
    let r_neg_to_pos_limit = Rational::new(i128::MIN + 1, -1);
    assert_eq!(r_neg_to_pos_limit.num(), i128::MAX);
    assert_eq!(r_neg_to_pos_limit.den(), 1);

    // 2. Extremely large values in add_rationals and negate succeeding
    let max_rat = NumberValue::Rational(Rational::new(i128::MAX, 1));
    let two_rat = NumberValue::Rational(Rational::new(2, 1));
    assert!(!max_rat.add(&two_rat).is_nan());

    // 3. Float extremes and bounds
    let f_max = Float::from_f64(f64::MAX, 53);
    let f_min = Float::from_f64(f64::MIN, 53);
    let f_neg_zero = Float::from_f64(-0.0, 53);

    assert!(f_neg_zero.is_zero());
    assert!(!f_max.is_infinite());
    assert!(!f_min.is_infinite());

    // 4. Nested complex structures
    // Construct (1 + 2i) + (3 + 4i)i = 1 + 2i + 3i - 4 = -3 + 5i
    let real_complex = Number::new_complex(Number::from_i32(1), Number::from_i32(2));
    let imag_complex = Number::new_complex(Number::from_i32(3), Number::from_i32(4));
    let nested_complex = Number::new_complex(real_complex, imag_complex);

    assert!(nested_complex.is_complex());
    assert_eq!(
        nested_complex.value(),
        &NumberValue::Rational(Rational::new(-3, 1))
    );
    assert_eq!(
        nested_complex.imaginary().unwrap().value(),
        &NumberValue::Rational(Rational::new(5, 1))
    );
    assert!(nested_complex.imaginary().unwrap().imaginary().is_none());

    // 5. Deeply nested uncertainty structure
    let mut current_val = NumberValue::Float(Float::from_f64(1.0, 53));
    let unc_val = NumberValue::Float(Float::from_f64(0.01, 53));
    for _ in 0..100 {
        current_val = NumberValue::Uncertainty {
            value: Box::new(current_val),
            uncertainty: Box::new(unc_val.clone()),
            is_relative: false,
        };
    }
    assert!(current_val.approximate());
    assert!(!current_val.is_real_zero());
    assert!(!current_val.is_nan());
    assert!(!current_val.is_infinite());
    assert_eq!(current_val.precision(), 53);

    let negated_deep = current_val.negate();
    assert!(negated_deep.approximate());

    // Test equality between two identical deeply nested uncertainty values
    let mut current_val2 = NumberValue::Float(Float::from_f64(1.0, 53));
    for _ in 0..100 {
        current_val2 = NumberValue::Uncertainty {
            value: Box::new(current_val2),
            uncertainty: Box::new(unc_val.clone()),
            is_relative: false,
        };
    }
    assert_eq!(current_val, current_val2);
}

#[test]
fn test_native_uncertainty_behavior() {
    // 1. Parsing +/- and ±
    let n1 = "5 +/- 1".parse::<Number>().unwrap();
    assert_eq!(n1.to_string(), "5.0±1.0");

    let n2 = "5 ± 0.5".parse::<Number>().unwrap();
    assert_eq!(n2.to_string(), "5.00±0.50");

    // 2. Parsing relative percentage uncertainty
    let n3 = "100 +/- 5%".parse::<Number>().unwrap();
    assert_eq!(n3.to_string(), "100.0±5.0%");

    // 3. Parsing parenthesis form
    let n4 = "1.23(4)".parse::<Number>().unwrap();
    assert_eq!(n4.to_string(), "1.230±0.040");

    let n5 = "123.4(15)".parse::<Number>().unwrap();
    assert_eq!(n5.to_string(), "123.4±1.5");

    // 4. Mathematical operations propagation
    let a = "5 +/- 3".parse::<Number>().unwrap();
    let b = "10 +/- 4".parse::<Number>().unwrap();
    let sum = a.add(&b);
    assert_eq!(sum.to_string(), "15.0±5.0");

    let c = "3 +/- 0.1".parse::<Number>().unwrap();
    let d = "4 +/- 0.1".parse::<Number>().unwrap();
    let prod = c.mul(&d);
    assert_eq!(prod.to_string(), "12.00±0.50");

    let e = "12 +/- 5".parse::<Number>().unwrap();
    let f = "3 +/- 0".parse::<Number>().unwrap();
    let div = e.div(&f);
    assert_eq!(div.to_string(), "4.0±1.7");
}

#[test]
fn test_zero_uncertainty_formats_as_underlying_value() {
    let exact = "10 +/- 0".parse::<Number>().unwrap();
    assert_eq!(exact.to_string(), "10");

    let exact_sum = exact.add(&exact);
    assert_eq!(exact_sum.to_string(), "20");

    assert_eq!(evaluate_expr("10 +/- 0").unwrap().to_string(), "10");
    assert_eq!(evaluate_expr("10 ± 0").unwrap().to_string(), "10");
}

#[test]
fn test_exhaustive_comparison_results() {
    // 1. ComparisonResult variants existence/construction check
    let _ = ComparisonResult::Equal;
    let _ = ComparisonResult::Greater;
    let _ = ComparisonResult::Less;
    let _ = ComparisonResult::EqualOrGreater;
    let _ = ComparisonResult::EqualOrLess;
    let _ = ComparisonResult::NotEqual;
    let _ = ComparisonResult::Unknown;
    let _ = ComparisonResult::EqualLimits;
    let _ = ComparisonResult::Contains;
    let _ = ComparisonResult::Contained;
    let _ = ComparisonResult::OverlappingLess;
    let _ = ComparisonResult::OverlappingGreater;

    // 2. Comparison tests (using self = A, other = B, B relative to A)
    // - Equal: l1 == l2 && u1 == u2 and it is a point value
    let a_eq = Number::from_f64(5.0);
    let b_eq = Number::from_f64(5.0);
    assert_eq!(a_eq.compare(&b_eq), ComparisonResult::Equal);
    assert!(!a_eq.is_greater_than(&b_eq));
    assert!(!a_eq.is_less_than(&b_eq));

    // - Strictly Less: B.upper < A.lower
    let a_less = Number::new_interval(Float::from_f64(10.0, 53), Float::from_f64(12.0, 53));
    let b_less = Number::new_interval(Float::from_f64(5.0, 53), Float::from_f64(8.0, 53));
    assert_eq!(a_less.compare(&b_less), ComparisonResult::Less);
    assert!(a_less.is_greater_than(&b_less));
    assert!(!a_less.is_less_than(&b_less));

    // - Strictly Greater: B.lower > A.upper
    let a_greater = Number::new_interval(Float::from_f64(5.0, 53), Float::from_f64(8.0, 53));
    let b_greater = Number::new_interval(Float::from_f64(10.0, 53), Float::from_f64(12.0, 53));
    assert_eq!(a_greater.compare(&b_greater), ComparisonResult::Greater);
    assert!(!a_greater.is_greater_than(&b_greater));
    assert!(a_greater.is_less_than(&b_greater));

    // - EqualLimits: l1 == l2 && u1 == u2 and not a point value
    let a_lim = Number::new_interval(Float::from_f64(5.0, 53), Float::from_f64(10.0, 53));
    let b_lim = Number::new_interval(Float::from_f64(5.0, 53), Float::from_f64(10.0, 53));
    assert_eq!(a_lim.compare(&b_lim), ComparisonResult::EqualLimits);

    // - Contained: A contains B strictly (l1 <= l2 && u1 >= u2)
    let a_cont = Number::new_interval(Float::from_f64(1.0, 53), Float::from_f64(10.0, 53));
    let b_cont = Number::new_interval(Float::from_f64(3.0, 53), Float::from_f64(7.0, 53));
    assert_eq!(a_cont.compare(&b_cont), ComparisonResult::Contained);

    // - Contains: B contains A strictly (l2 <= l1 && u2 >= u1)
    let a_contains = Number::new_interval(Float::from_f64(3.0, 53), Float::from_f64(7.0, 53));
    let b_contains = Number::new_interval(Float::from_f64(1.0, 53), Float::from_f64(10.0, 53));
    assert_eq!(a_contains.compare(&b_contains), ComparisonResult::Contains);

    // - OverlappingLess: l2 < l1 && u2 < u1 && u2 >= l1 (B overlaps A on left)
    let a_ol = Number::new_interval(Float::from_f64(5.0, 53), Float::from_f64(10.0, 53));
    let b_ol = Number::new_interval(Float::from_f64(3.0, 53), Float::from_f64(7.0, 53));
    assert_eq!(a_ol.compare(&b_ol), ComparisonResult::OverlappingLess);

    // - OverlappingGreater: l1 < l2 && u1 < u2 && l2 <= u1 (B overlaps A on right)
    let a_og = Number::new_interval(Float::from_f64(3.0, 53), Float::from_f64(7.0, 53));
    let b_og = Number::new_interval(Float::from_f64(5.0, 53), Float::from_f64(10.0, 53));
    assert_eq!(a_og.compare(&b_og), ComparisonResult::OverlappingGreater);

    // - Unknown: NaN or Complex
    let a_nan = Number::from_f64(f64::NAN);
    let b_nan = Number::from_f64(5.0);
    assert_eq!(a_nan.compare(&b_nan), ComparisonResult::Unknown);

    let a_complex = Number::new_complex(Number::from_f64(1.0), Number::from_f64(2.0));
    let b_complex = Number::from_f64(5.0);
    assert_eq!(a_complex.compare(&b_complex), ComparisonResult::Unknown);
    assert_eq!(b_complex.compare(&a_complex), ComparisonResult::Unknown);

    // - Infinity comparisons
    let inf = Number::from_f64(f64::INFINITY);
    let neg_inf = Number::from_f64(f64::NEG_INFINITY);
    assert_eq!(neg_inf.compare(&inf), ComparisonResult::Greater); // other (inf) > self (neg_inf)
    assert_eq!(inf.compare(&neg_inf), ComparisonResult::Less); // other (neg_inf) < self (inf)
}

#[test]
fn test_exhaustive_interval_arithmetic() {
    // Test outward-rounded addition
    // [1.0, 2.0] + [3.0, 4.0] = [4.0, 6.0] (rounded outwards)
    let a = Number::new_interval(Float::from_f64(1.0, 53), Float::from_f64(2.0, 53));
    let b = Number::new_interval(Float::from_f64(3.0, 53), Float::from_f64(4.0, 53));
    let sum = a.add(&b);
    if let NumberValue::Interval { lower, upper } = sum.value() {
        // lower = next_after(4.0, NEG_INFINITY), upper = next_after(6.0, INFINITY)
        assert!(lower.value <= 4.0);
        assert!(upper.value >= 6.0);
    } else {
        panic!("Expected Interval");
    }

    // Test outward-rounded subtraction
    // [3.0, 4.0] - [1.0, 2.0] = [1.0, 3.0]
    let diff = b.sub(&a);
    if let NumberValue::Interval { lower, upper } = diff.value() {
        assert!(lower.value <= 1.0);
        assert!(upper.value >= 3.0);
    } else {
        panic!("Expected Interval");
    }

    // Test outward-rounded multiplication
    // [-2.0, 3.0] * [-4.0, 5.0]
    // possible products: 8, -10, -12, 15. min = -12, max = 15
    let c = Number::new_interval(Float::from_f64(-2.0, 53), Float::from_f64(3.0, 53));
    let d = Number::new_interval(Float::from_f64(-4.0, 53), Float::from_f64(5.0, 53));
    let prod = c.mul(&d);
    if let NumberValue::Interval { lower, upper } = prod.value() {
        assert!(lower.value <= -12.0);
        assert!(upper.value >= 15.0);
    } else {
        panic!("Expected Interval");
    }

    // Test outward-rounded division
    // [4.0, 6.0] / [2.0, 3.0] = [4/3, 3.0]
    let e = Number::new_interval(Float::from_f64(4.0, 53), Float::from_f64(6.0, 53));
    let f = Number::new_interval(Float::from_f64(2.0, 53), Float::from_f64(3.0, 53));
    let quot = e.div(&f);
    if let NumberValue::Interval { lower, upper } = quot.value() {
        assert!(lower.value <= 4.0 / 3.0);
        assert!(upper.value >= 3.0);
    } else {
        panic!("Expected Interval");
    }

    // Test division by zero interval
    let zero_interval = Number::new_interval(Float::from_f64(-1.0, 53), Float::from_f64(1.0, 53));
    let div_zero = e.div(&zero_interval);
    assert!(div_zero.is_nan());

    // Test mixed scalar-interval operations
    let scalar = Number::from_f64(2.0);
    let mixed_sum = a.add(&scalar);
    if let NumberValue::Interval { lower, upper } = mixed_sum.value() {
        assert!(lower.value <= 3.0);
        assert!(upper.value >= 4.0);
    } else {
        panic!("Expected Interval");
    }

    let mixed_prod = a.mul(&scalar);
    if let NumberValue::Interval { lower, upper } = mixed_prod.value() {
        assert!(lower.value <= 2.0);
        assert!(upper.value >= 4.0);
    } else {
        panic!("Expected Interval");
    }

    // Test NaN propagation in interval arithmetic
    let nan_val = Number::from_f64(f64::NAN);
    assert!(a.add(&nan_val).is_nan());
    assert!(nan_val.add(&a).is_nan());
}

#[test]
fn test_complex_arithmetic() {
    // Standard complex arithmetic: (1+2i) + (3+4i) = 4+6i
    let c1 = Number::new_complex(Number::from_i32(1), Number::from_i32(2));
    let c2 = Number::new_complex(Number::from_i32(3), Number::from_i32(4));

    let sum = c1.add(&c2);
    let (real_sum, imag_sum) = sum.to_canonical_real_imag();
    assert_eq!(real_sum, NumberValue::Rational(Rational::new(4, 1)));
    assert_eq!(imag_sum, NumberValue::Rational(Rational::new(6, 1)));

    // Subtraction: (1+2i) - (3+4i) = -2-2i
    let diff = c1.sub(&c2);
    let (real_diff, imag_diff) = diff.to_canonical_real_imag();
    assert_eq!(real_diff, NumberValue::Rational(Rational::new(-2, 1)));
    assert_eq!(imag_diff, NumberValue::Rational(Rational::new(-2, 1)));

    // Multiplication: (1+2i) * (3+4i) = (3 - 8) + (4 + 6)i = -5+10i
    let prod = c1.mul(&c2);
    let (real_prod, imag_prod) = prod.to_canonical_real_imag();
    assert_eq!(real_prod, NumberValue::Rational(Rational::new(-5, 1)));
    assert_eq!(imag_prod, NumberValue::Rational(Rational::new(10, 1)));

    // Division: (1+2i) / (3+4i) = ((1+2i)(3-4i))/(9+16) = (3 + 8 + i(6 - 4))/25 = (11 + 2i)/25 = 11/25 + 2/25 i
    let quot = c1.div(&c2);
    let (real_quot, imag_quot) = quot.to_canonical_real_imag();
    assert_eq!(real_quot, NumberValue::Rational(Rational::new(11, 25)));
    assert_eq!(imag_quot, NumberValue::Rational(Rational::new(2, 25)));
}

#[test]
fn test_new_rational_arithmetic_and_comparisons() {
    // 1. checked_add, checked_sub, checked_mul, checked_div
    let half = Rational::try_new(1, 2).unwrap();
    let third = Rational::try_new(1, 3).unwrap();

    assert_eq!(half.checked_add(&third), Rational::try_new(5, 6));
    assert_eq!(half.checked_sub(&third), Rational::try_new(1, 6));
    assert_eq!(half.checked_mul(&third), Rational::try_new(1, 6));
    assert_eq!(half.checked_div(&third), Rational::try_new(3, 2));

    // Overflows
    let max_rat = Rational::try_new(i128::MAX, 1).unwrap();
    let one_rat = Rational::try_new(1, 1).unwrap();
    assert_eq!(
        max_rat
            .checked_add(&one_rat)
            .expect("checked add should remain exact past i128")
            .numerator_string(),
        (rug::Integer::from(i128::MAX) + 1_i32).to_string()
    );

    let min_rat = Rational::try_new(i128::MIN, 1).unwrap();
    assert_eq!(
        min_rat
            .checked_sub(&one_rat)
            .expect("checked sub should remain exact past i128")
            .numerator_string(),
        (rug::Integer::from(i128::MIN) - 1_i32).to_string()
    );

    // Regression tests for premature overflow
    let max_over_2 = Rational::try_new(i128::MAX, 2).unwrap();
    assert_eq!(
        max_over_2.checked_add(&max_over_2),
        Some(Rational::new(i128::MAX, 1))
    );

    let min_plus_1_over_2 = Rational::try_new(i128::MIN + 1, 2).unwrap();
    assert_eq!(
        min_plus_1_over_2.checked_sub(&max_over_2),
        Some(Rational::new(i128::MIN + 1, 1))
    );

    // Cross-GCD multiplication/division to avoid premature overflow
    let large_a = Rational::try_new(1 << 120, 1 << 100).unwrap();
    let large_b = Rational::try_new(1 << 100, 1 << 120).unwrap();
    assert_eq!(large_a.checked_mul(&large_b), Some(Rational::new(1, 1)));
    assert_eq!(large_a.checked_div(&large_a), Some(Rational::new(1, 1)));

    // Division by zero
    let zero_rat = Rational::try_new(0, 1).unwrap();
    assert_eq!(half.checked_div(&zero_rat), None);

    // 2. Ordering comparisons
    assert!(half > third);
    assert!(Rational::try_new(-1, 2).unwrap() < Rational::try_new(-1, 3).unwrap());
    assert!(half > Rational::try_new(-1, 2).unwrap());
    assert!(zero_rat < half);
    assert!(zero_rat > Rational::try_new(-1, 2).unwrap());
    assert!(max_rat > Rational::try_new(i128::MAX - 1, 1).unwrap());

    // 3. NumberValue promotions and arithmetic
    let nv_half = NumberValue::Rational(half.clone());
    let nv_third = NumberValue::Rational(third.clone());
    let nv_float = NumberValue::Float(Float::from_f64(0.5, 53));
    let nv_float_two = NumberValue::Float(Float::from_f64(2.0, 53));

    // sub
    assert_eq!(
        nv_half.sub(&nv_third),
        NumberValue::Rational(Rational::new(1, 6))
    );
    assert!(nv_half.sub(&nv_float).is_real_zero());

    // mul
    assert_eq!(
        nv_half.mul(&nv_third),
        NumberValue::Rational(Rational::new(1, 6))
    );
    assert_eq!(
        nv_half.mul(&nv_float_two),
        NumberValue::Float(Float::from_f64(1.0, 53))
    );

    // div
    assert_eq!(
        nv_half.div(&nv_third),
        NumberValue::Rational(Rational::new(3, 2))
    );
    assert_eq!(
        nv_half.div(&nv_float),
        NumberValue::Float(Float::from_f64(1.0, 53))
    );
    assert!(nv_half
        .div(&NumberValue::Rational(zero_rat.clone()))
        .is_nan());

    // Interval promotions
    let interval = NumberValue::Interval {
        lower: Float::from_f64(0.0, 53),
        upper: Float::from_f64(2.0, 53),
    };
    // nv_half (0.5) - [0, 2] = [0.5 - 2, 0.5 - 0] = [-1.5, 0.5]
    if let NumberValue::Interval { lower, upper } = nv_half.sub(&interval) {
        assert!(lower.value() <= -1.5);
        assert!(upper.value() >= 0.5);
    } else {
        panic!("Expected interval");
    }

    // [0, 2] - nv_half (0.5) = [-0.5, 1.5]
    if let NumberValue::Interval { lower, upper } = interval.sub(&nv_half) {
        assert!(lower.value() <= -0.5);
        assert!(upper.value() >= 1.5);
    } else {
        panic!("Expected interval");
    }

    // Uncertainty promotions
    let uncertainty = NumberValue::Uncertainty {
        value: Box::new(NumberValue::Rational(Rational::new(5, 1))),
        uncertainty: Box::new(NumberValue::Rational(Rational::new(1, 2))),
        is_relative: false,
    };
    // nv_half (0.5) - (5 +/- 0.5) = -4.5 +/- 0.5
    if let NumberValue::Uncertainty {
        value,
        uncertainty: unc,
        ..
    } = nv_half.sub(&uncertainty)
    {
        assert_eq!(value.as_ref(), &NumberValue::Rational(Rational::new(-9, 2)));
        assert_eq!(unc.as_ref(), &NumberValue::Rational(Rational::new(1, 2)));
    } else {
        panic!("Expected uncertainty");
    }

    // 4. Number comparisons and promotions
    let n_half = Number::from_rational(half.clone());
    let n_third = Number::from_rational(third.clone());
    let n_float = Number::from_float(Float::from_f64(0.5, 53));

    assert!(n_half > n_third);
    assert_eq!(
        n_half.partial_cmp(&n_float),
        Some(std::cmp::Ordering::Equal)
    );
    assert!(n_half > Number::from_float(Float::from_f64(0.4, 53)));

    // Complex promotions
    let c = Number::new_complex(
        Number::from_rational(Rational::new(2, 1)),
        Number::from_rational(Rational::new(3, 1)),
    );
    // n_half (0.5) - (2 + 3i) = -1.5 - 3i
    let diff = n_half.sub(&c);
    assert_eq!(diff.value(), &NumberValue::Rational(Rational::new(-3, 2)));
    assert_eq!(
        diff.imaginary().unwrap().value(),
        &NumberValue::Rational(Rational::new(-3, 1))
    );
}

#[test]
fn test_critic_adversarial_precision_rounding_equality() {
    use rug::ops::Pow;

    // 1. Float Equality Transitivity Violation Test
    let x = Float::from_f64(1.0, 53);

    let mut y_val = rug::Float::with_val(128, 1.0);
    let base2 = rug::Float::with_val(128, 2.0);
    let diff = rug::Float::with_val(128, base2.pow(-100));
    y_val += diff;
    let y = Float { value: y_val };

    let mut z_val = rug::Float::with_val(128, 1.0);
    let base2_z = rug::Float::with_val(128, 2.0);
    let diff2 = rug::Float::with_val(128, base2_z.pow(-53));
    z_val += diff2;
    let z = Float { value: z_val };

    assert_ne!(x, y, "Transitivity check 1 (x != y)");
    assert_ne!(x, z, "Transitivity check 2 (x != z)");
    assert_ne!(y, z, "Transitivity check 3 (y != z)");

    // 2. Rational + Interval Precision Loss Test
    let lower_128 = Float {
        value: rug::Float::with_val(128, 0.0),
    };
    let upper_128 = Float {
        value: rug::Float::with_val(128, 0.0),
    };
    let interval = NumberValue::Interval {
        lower: lower_128,
        upper: upper_128,
    };

    let r_third = NumberValue::Rational(Rational::new(1, 3));

    let result = interval.add(&r_third);
    if let NumberValue::Interval { lower, upper } = result {
        assert_eq!(lower.prec(), 128);
        assert_eq!(upper.prec(), 128);

        let expected_128 = rug::Float::with_val(128, &rug::Rational::from((1, 3)));
        let actual_lower = &lower.value;

        let diff_actual_expected = rug::Float::with_val(128, actual_lower - &expected_128).abs();

        let base2_limit = rug::Float::with_val(128, 2.0);
        let limit_128 = rug::Float::with_val(128, base2_limit.pow(-120));

        // The difference must be extremely small (0 or <= limit_128) as precision is preserved at 128 bits
        assert!(
            diff_actual_expected <= limit_128,
            "Rational to Interval addition should NOT have lost precision"
        );
    } else {
        panic!("Expected Interval result");
    }

    // 3. Outward Rounding verification under Round::Down and Round::Up
    let f1_lower = Float {
        value: rug::Float::with_val(4, 0.1),
    };
    let f1_upper = Float {
        value: rug::Float::with_val(4, 0.1),
    };
    let iv1 = NumberValue::Interval {
        lower: f1_lower,
        upper: f1_upper,
    };

    let f2_lower = Float {
        value: rug::Float::with_val(4, 0.2),
    };
    let f2_upper = Float {
        value: rug::Float::with_val(4, 0.2),
    };
    let iv2 = NumberValue::Interval {
        lower: f2_lower,
        upper: f2_upper,
    };

    let sum_iv = iv1.add(&iv2);
    if let NumberValue::Interval { lower, upper } = sum_iv {
        let exact_sum = rug::Float::with_val(128, 0.3);
        let lower_f128 = rug::Float::with_val(128, &lower.value);
        let upper_f128 = rug::Float::with_val(128, &upper.value);
        assert!(
            lower_f128 <= exact_sum,
            "Lower bound must be rounded Down (<= exact sum)"
        );
        assert!(
            upper_f128 >= exact_sum,
            "Upper bound must be rounded Up (>= exact sum)"
        );
        assert!(
            lower_f128 < upper_f128,
            "Outward rounding should make interval bounds diverge"
        );
    } else {
        panic!("Expected Interval");
    }

    // 4. Boundary and extreme Float values
    let subnormal_f64 = 5e-324;
    let f_sub = Float::from_f64(subnormal_f64, 53);
    assert!(!f_sub.is_zero());
    assert!(!f_sub.is_nan());
    assert!(!f_sub.is_infinite());

    let large_float = Float {
        value: rug::Float::with_val(128, rug::Float::parse("1e10000").unwrap()),
    };
    assert_eq!(large_float.value(), f64::INFINITY);
}

#[test]
fn test_critic_float_eq_transitivity_adversarial() {
    use rug::ops::Pow;

    // 1. Float PartialEq Transitivity
    let precs = [2, 12, 24, 53, 64, 128, 256, 512];
    for &p1 in &precs {
        for &p2 in &precs {
            for &p3 in &precs {
                let values = [1.0, 0.5, 0.25, 0.0, -0.0, f64::INFINITY, f64::NEG_INFINITY];
                for &v in &values {
                    let a = Float::from_f64(v, p1);
                    let b = Float::from_f64(v, p2);
                    let c = Float::from_f64(v, p3);

                    assert_eq!(a, b, "a == b for value {} with prec {} and {}", v, p1, p2);
                    assert_eq!(b, c, "b == c for value {} with prec {} and {}", v, p2, p3);
                    assert_eq!(a, c, "a == c for value {} with prec {} and {}", v, p1, p3);
                }

                let a_nan = Float::from_f64(f64::NAN, p1);
                let b_nan = Float::from_f64(f64::NAN, p2);
                let c_nan = Float::from_f64(f64::NAN, p3);
                assert_eq!(a_nan, b_nan);
                assert_eq!(b_nan, c_nan);
                assert_eq!(a_nan, c_nan);
            }
        }
    }

    // Pairwise inequalities transitivity: A != B && B == C => A != C
    let a = Float::from_f64(1.0, 53);

    let mut b_val = rug::Float::with_val(128, 1.0);
    let diff = rug::Float::with_val(128, rug::Float::with_val(128, 2.0).pow(-100));
    b_val += diff;
    let b = Float {
        value: b_val.clone(),
    };

    let c = Float {
        value: rug::Float::with_val(256, &b_val),
    };

    assert_ne!(a, b);
    assert_eq!(b, c);
    assert_ne!(a, c);

    // Subnormals transitivity
    let sub1 = Float::from_f64(5e-324, 53);
    let sub2 = Float::from_f64(5e-324, 128);
    let sub3 = Float::from_f64(5e-324, 256);
    assert_eq!(sub1, sub2);
    assert_eq!(sub2, sub3);
    assert_eq!(sub1, sub3);
}

#[test]
fn test_critic_rational_interval_addition_precision_loss() {
    use rug::ops::AssignRound;

    let precs = [2, 12, 24, 53, 64, 128, 256, 512];
    for &prec in &precs {
        let lower = Float {
            value: rug::Float::with_val(prec, 0.0),
        };
        let upper = Float {
            value: rug::Float::with_val(prec, 0.0),
        };
        let interval = NumberValue::Interval { lower, upper };
        let r_third = NumberValue::Rational(Rational::new(1, 3));

        let res = interval.add(&r_third);
        if let NumberValue::Interval {
            lower: res_l,
            upper: res_u,
        } = res
        {
            // Verify that precision is preserved
            assert_eq!(res_l.prec(), prec);
            assert_eq!(res_u.prec(), prec);

            // Compute expected bounds under correct outward rounding (Down for lower, Up for upper)
            let r = rug::Rational::from((1, 3));
            let mut expected_lower = rug::Float::new(prec);
            expected_lower.assign_round(&r, rug::float::Round::Down);
            let mut expected_upper = rug::Float::new(prec);
            expected_upper.assign_round(&r, rug::float::Round::Up);

            // Verify that the actual bounds computed by the code match the expected outward-rounded bounds
            assert_eq!(res_l.value, expected_lower);
            assert_eq!(res_u.value, expected_upper);
        } else {
            panic!("Expected Interval");
        }
    }
}

#[test]
fn test_critic_no_regressions_float_interval_rational() {
    // 1. Outward rounding of intervals: verify round Down for lower and Up for upper under addition.
    let lower1 = Float {
        value: rug::Float::with_val(12, 0.1),
    };
    let upper1 = Float {
        value: rug::Float::with_val(12, 0.1),
    };
    let iv1 = NumberValue::Interval {
        lower: lower1,
        upper: upper1,
    };

    let lower2 = Float {
        value: rug::Float::with_val(12, 0.2),
    };
    let upper2 = Float {
        value: rug::Float::with_val(12, 0.2),
    };
    let iv2 = NumberValue::Interval {
        lower: lower2,
        upper: upper2,
    };

    let sum = iv1.add(&iv2);
    if let NumberValue::Interval {
        lower: sum_l,
        upper: sum_u,
    } = sum
    {
        let exact_sum = rug::Float::with_val(1024, 0.3);
        let sum_l_high = rug::Float::with_val(1024, &sum_l.value);
        let sum_u_high = rug::Float::with_val(1024, &sum_u.value);

        assert!(sum_l_high <= exact_sum);
        assert!(sum_u_high >= exact_sum);
        assert!(sum_l_high < sum_u_high);
    } else {
        panic!("Expected Interval");
    }

    // 2. Rational panic behaviors (overflows, zero denominator)
    let result = std::panic::catch_unwind(|| {
        let _ = Rational::new(5, 0);
    });
    assert!(result.is_err());

    let r_overflow_num = Rational::new(i128::MIN, -1);
    let result_num = std::panic::catch_unwind(move || {
        let _ = r_overflow_num.num();
    });
    assert!(result_num.is_err());

    let r_overflow_den = Rational::new(1, i128::MIN);
    let result_den = std::panic::catch_unwind(move || {
        let _ = r_overflow_den.den();
    });
    assert!(result_den.is_err());

    let r_max = NumberValue::Rational(Rational::new(i128::MAX, 1));
    let sum_large = r_max.add(&r_max);
    assert!(!sum_large.is_nan());
    if let NumberValue::Rational(sum_r) = sum_large {
        let result_sum_access = std::panic::catch_unwind(move || {
            let _ = sum_r.num();
        });
        assert!(result_sum_access.is_err());
    } else {
        panic!("Expected Rational");
    }
}

#[test]
fn test_coverage_boost() {
    // 1. NextAfter trait usage on f64
    let zero = 0.0f64;
    assert_ne!(zero.next_after(1.0), 0.0);
    assert_ne!(zero.next_after(-1.0), 0.0);
    assert!(f64::NAN.next_after(1.0).is_nan());
    assert_eq!(1.0f64.next_after(1.0), 1.0);

    // 2. is_negative_value helper function
    let v_rat_pos = NumberValue::Rational(Rational::from_i32(5));
    let v_rat_neg = NumberValue::Rational(Rational::from_i32(-5));
    let v_fl_pos = NumberValue::Float(Float::from_f64(5.0, 53));
    let v_fl_neg = NumberValue::Float(Float::from_f64(-5.0, 53));
    let v_plus_inf = NumberValue::PlusInfinity;
    let v_minus_inf = NumberValue::MinusInfinity;
    let v_nan = NumberValue::NaN;
    let v_interval_neg = NumberValue::Interval {
        lower: Float::from_f64(-10.0, 53),
        upper: Float::from_f64(-5.0, 53),
    };
    let v_interval_pos = NumberValue::Interval {
        lower: Float::from_f64(5.0, 53),
        upper: Float::from_f64(10.0, 53),
    };
    let v_interval_mixed = NumberValue::Interval {
        lower: Float::from_f64(-5.0, 53),
        upper: Float::from_f64(5.0, 53),
    };
    let v_unc = NumberValue::Uncertainty {
        value: Box::new(v_rat_neg.clone()),
        uncertainty: Box::new(v_rat_pos.clone()),
        is_relative: false,
    };

    assert_eq!(is_negative_value(&v_rat_pos), Some(false));
    assert_eq!(is_negative_value(&v_rat_neg), Some(true));
    assert_eq!(is_negative_value(&v_fl_pos), Some(false));
    assert_eq!(is_negative_value(&v_fl_neg), Some(true));
    assert_eq!(is_negative_value(&v_plus_inf), Some(false));
    assert_eq!(is_negative_value(&v_minus_inf), Some(true));
    assert_eq!(is_negative_value(&v_interval_neg), Some(true));
    assert_eq!(is_negative_value(&v_interval_pos), Some(false));
    assert_eq!(is_negative_value(&v_interval_mixed), None);
    assert_eq!(is_negative_value(&v_unc), Some(true));
    assert_eq!(is_negative_value(&v_nan), None);

    // 3. is_value_negative helper function
    assert!(!is_value_negative(&v_rat_pos));
    assert!(is_value_negative(&v_rat_neg));
    assert!(!is_value_negative(&v_fl_pos));
    assert!(is_value_negative(&v_fl_neg));
    assert!(!is_value_negative(&v_plus_inf));
    assert!(is_value_negative(&v_minus_inf));
    assert!(is_value_negative(&v_unc));
    assert!(!is_value_negative(&v_nan));

    // 4. get_sign_multiplier helper function
    assert_eq!(get_sign_multiplier(&v_rat_pos), Some(1.0));
    assert_eq!(get_sign_multiplier(&v_rat_neg), Some(-1.0));
    assert_eq!(get_sign_multiplier(&v_fl_pos), Some(1.0));
    assert_eq!(get_sign_multiplier(&v_fl_neg), Some(-1.0));
    assert_eq!(get_sign_multiplier(&v_plus_inf), Some(1.0));
    assert_eq!(get_sign_multiplier(&v_minus_inf), Some(-1.0));
    assert_eq!(get_sign_multiplier(&v_interval_pos), Some(1.0));
    assert_eq!(get_sign_multiplier(&v_interval_neg), Some(-1.0));
    assert_eq!(get_sign_multiplier(&v_interval_mixed), None);
    assert_eq!(get_sign_multiplier(&v_unc), Some(-1.0));
    assert_eq!(get_sign_multiplier(&v_nan), None);

    // 5. Number constructors and predicates coverage
    let n_default = Number::default();
    assert!(n_default.is_zero());

    let n_one = Number::from_i32(1);
    assert!(n_one.is_one());
    assert!(n_one.is_real_one());
    assert!(!n_one.is_interval());
    assert!(!n_one.is_complex());
    assert!(!n_one.is_imaginary());

    let n_interval = Number::new_interval(Float::from_f64(1.0, 53), Float::from_f64(2.0, 53));
    assert!(n_interval.is_interval());

    let n_real = NumberValue::Rational(Rational::from_i32(1));
    let n_imag = NumberValue::Rational(Rational::from_i32(2));
    let n_complex = Number::from_real_imag_values(n_real, n_imag, 53, false);
    assert!(n_complex.is_complex());
    assert!(n_complex.is_imaginary() || n_complex.has_real_part());

    // 6. norm and conjugate
    let n_comp = Number::new_complex(Number::from_i32(3), Number::from_i32(4));
    assert_eq!(n_comp.conjugate().to_string(), "3 - 4i");
    assert_eq!(n_comp.norm().to_string(), "5");

    // 7. min4 / max4
    assert_eq!(min4(1.0, 2.0, 3.0, 4.0), 1.0);
    assert_eq!(max4(1.0, 2.0, 3.0, 4.0), 4.0);

    // 8. to_interval
    let int_bounds = to_interval(&v_rat_pos);
    assert!(int_bounds.is_some());
    let int_bounds_unc = to_interval(&v_unc);
    assert!(int_bounds_unc.is_some());

    // 11. from_f64_and_prec
    let val_f64_prec = from_f64_and_prec(1.5, 64);
    if let NumberValue::Float(f) = val_f64_prec {
        assert_eq!(f.prec(), 64);
        assert_eq!(f.value(), 1.5);
    } else {
        panic!("Expected Float");
    }

    // 12. NumberValue is_zero, approximate, precision, contains_interval_or_uncertainty
    let val_zero = NumberValue::Rational(Rational::from_i32(0));
    assert!(val_zero.is_zero());
    assert!(!val_zero.approximate());
    assert_eq!(val_zero.precision(), 0);

    let val_fl = NumberValue::Float(Float::from_f64(1.23, 64));
    assert!(val_fl.approximate());
    assert_eq!(val_fl.precision(), 64);

    let val_unc = NumberValue::Uncertainty {
        value: Box::new(val_zero),
        uncertainty: Box::new(val_fl),
        is_relative: false,
    };
    assert!(val_unc.approximate());
    assert_eq!(val_unc.precision(), 64);

    // contains_interval_or_uncertainty
    let n_unc_wrapped = Number::from_rational(Rational::from_i32(1));
    assert!(!n_unc_wrapped.contains_interval_or_uncertainty());

    let val_unc_wrapped = Number::new_uncertainty(
        NumberValue::Rational(Rational::from_i32(5)),
        NumberValue::Rational(Rational::from_i32(1)),
        false,
    );
    assert!(val_unc_wrapped.contains_interval_or_uncertainty());

    // 13. NumberValue::ln & pow
    let ln_val = NumberValue::Rational(Rational::from_i32(2)).ln();
    if let NumberValue::Float(f) = ln_val {
        assert!((f.value() - 2.0f64.ln()).abs() < 1e-9);
    }
    let ln_fl = NumberValue::Float(Float::from_f64(2.0, 53)).ln();
    if let NumberValue::Float(f) = ln_fl {
        assert!((f.value() - 2.0f64.ln()).abs() < 1e-9);
    }
    let ln_interval = NumberValue::Interval {
        lower: Float::from_f64(2.0, 53),
        upper: Float::from_f64(3.0, 53),
    }
    .ln();
    if let NumberValue::Interval { lower, upper } = ln_interval {
        assert!((lower.value() - 2.0f64.ln()).abs() < 1e-9);
        assert!((upper.value() - 3.0f64.ln()).abs() < 1e-9);
    }

    let pow_val = NumberValue::Rational(Rational::from_i32(2))
        .pow(&NumberValue::Rational(Rational::from_i32(3)));
    if let NumberValue::Rational(r) = pow_val {
        assert_eq!(r.num(), 8);
    }

    // 14. Rational and Float arithmetic operations
    let rat1 = Rational::from_i32(2);
    let rat2 = Rational::from_i32(3);
    assert_eq!(rat1.add(&rat2), Some(Rational::from_i32(5)));
    assert_eq!(rat1.sub(&rat2), Some(Rational::from_i32(-1)));
    assert_eq!(rat1.mul(&rat2), Some(Rational::from_i32(6)));
    assert_eq!(rat1.div(&rat2), Some(Rational::new(2, 3)));

    let fl1 = Float::from_f64(2.0, 53);
    let fl2 = Float::from_f64(3.0, 53);
    assert_eq!(fl1.add(&fl2).value(), 5.0);
    assert_eq!(fl1.sub(&fl2).value(), -1.0);
    assert_eq!(fl1.mul(&fl2).value(), 6.0);
    assert_eq!(fl1.div(&fl2).value(), 2.0 / 3.0);

    // 15. ComparisonResult and NumberValue orderings
    let cmp_less = NumberValue::Rational(rat1).partial_cmp(&NumberValue::Rational(rat2));
    assert_eq!(cmp_less, Some(std::cmp::Ordering::Less));

    let val_less_than = NumberValue::Rational(Rational::from_i32(1));
    let val_greater_than = NumberValue::Rational(Rational::from_i32(3));
    assert!(val_less_than.is_less_than(&val_greater_than));
    assert!(val_greater_than.is_greater_than(&val_less_than));

    let n_less = Number::from_i32(1);
    let n_greater = Number::from_i32(3);
    assert!(n_less.is_less_than(&n_greater));

    // 16. Subtraction of uncertainties
    let u1 = NumberValue::Uncertainty {
        value: Box::new(NumberValue::Rational(Rational::from_i32(10))),
        uncertainty: Box::new(NumberValue::Rational(Rational::from_i32(1))),
        is_relative: false,
    };
    let u2 = NumberValue::Uncertainty {
        value: Box::new(NumberValue::Rational(Rational::from_i32(4))),
        uncertainty: Box::new(NumberValue::Rational(Rational::from_i32(2))),
        is_relative: false,
    };
    let sub_unc = u1.sub(&u2);
    if let NumberValue::Uncertainty { value, .. } = sub_unc {
        assert_eq!(*value, NumberValue::Rational(Rational::from_i32(6)));
    }

    // 17. Left-hand side plain value / right-hand side uncertainty multiplication
    let plain = NumberValue::Rational(Rational::from_i32(2));
    let unc_rhs = NumberValue::Uncertainty {
        value: Box::new(NumberValue::Rational(Rational::from_i32(5))),
        uncertainty: Box::new(NumberValue::Rational(Rational::from_i32(1))),
        is_relative: false,
    };
    let mul_unc = plain.mul(&unc_rhs);
    if let NumberValue::Uncertainty { value, .. } = mul_unc {
        assert_eq!(*value, NumberValue::Rational(Rational::from_i32(10)));
    }

    // 18. Division of two uncertainties
    let div_unc = u1.div(&u2);
    if let NumberValue::Uncertainty { value, .. } = div_unc {
        assert_eq!(*value, NumberValue::Rational(Rational::new(5, 2)));
    }

    // 19. Plain value multiplied by Interval
    let plain_mul = NumberValue::Rational(Rational::from_i32(2));
    let interval_mul = NumberValue::Interval {
        lower: Float::from_f64(1.0, 53),
        upper: Float::from_f64(3.0, 53),
    };
    let res_interval_mul = plain_mul.mul(&interval_mul);
    if let NumberValue::Interval { lower, upper } = res_interval_mul {
        assert_eq!(lower.value(), 2.0);
        assert_eq!(upper.value(), 6.0);
    }

    // 20. Division of infinity by plain negative value
    let inf = NumberValue::PlusInfinity;
    let plain_neg = NumberValue::Rational(Rational::from_i32(-2));
    let div_inf = inf.div(&plain_neg);
    assert_eq!(div_inf, NumberValue::MinusInfinity);

    // 21. Complex power fallback
    let base_comp = Number::new_complex(Number::from_i32(1), Number::from_i32(1));
    let exp_comp = Number::from_i32(2);
    let pow_comp_res = base_comp.pow(&exp_comp);
    assert!(!pow_comp_res.is_nan());

    // 22. Complex uncertainty in contains_interval_or_uncertainty
    let real_part = Number::from_i32(5);
    let imag_part_unc = Number::new_uncertainty(
        NumberValue::Rational(Rational::from_i32(2)),
        NumberValue::Rational(Rational::from_i32(1)),
        false,
    );
    let complex_unc = Number::new_complex(real_part, imag_part_unc);
    assert!(complex_unc.contains_interval_or_uncertainty());
}
