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
        assert_eq!(rat.num, 42);
        assert_eq!(rat.den, 1);
    } else {
        panic!("Expected rational");
    }

    let n_f64 = Number::from_f64(1.23);
    if let NumberValue::Float(fl) = n_f64.value() {
        assert_eq!(fl.value, 1.23);
        assert_eq!(fl.prec, 53);
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
    assert_eq!(r1.num, 1);
    assert_eq!(r1.den, 1);

    let r2 = Rational::new(2, -4);
    assert_eq!(r2.num, -1);
    assert_eq!(r2.den, 2);

    let r3 = Rational::new(0, -5);
    assert_eq!(r3.num, 0);
    assert_eq!(r3.den, 1);

    let r4 = Rational::new(-10, -20);
    assert_eq!(r4.num, 1);
    assert_eq!(r4.den, 2);
}

#[test]
fn test_uncertainty_modeling() {
    let val = NumberValue::Rational(Rational::new(5, 1));
    let unc = NumberValue::Rational(Rational::new(1, 2));
    let n = Number::new_uncertainty(val, unc);

    assert!(n.approximate());
    if let NumberValue::Uncertainty { value, uncertainty } = n.value() {
        if let NumberValue::Rational(r_val) = &**value {
            assert_eq!(r_val.num, 5);
        } else {
            panic!("Expected Rational");
        }
        if let NumberValue::Rational(r_unc) = &**uncertainty {
            assert_eq!(r_unc.num, 1);
            assert_eq!(r_unc.den, 2);
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
    // Test GCD with i128::MIN
    let g = gcd(i128::MIN, i128::MIN);
    assert_eq!(g, i128::MIN.unsigned_abs());

    // Test canonicalize with self.num = i128::MIN and den = 1 (representable)
    let mut r = Rational {
        num: i128::MIN,
        den: 1,
    };
    r.canonicalize();
    assert_eq!(r.num, i128::MIN);
    assert_eq!(r.den, 1);
}

#[test]
fn test_rational_arithmetic_overflow_returns_nan() {
    let max = NumberValue::Rational(Rational::new(i128::MAX, 1));
    let one = NumberValue::Rational(Rational::new(1, 1));
    assert!(max.add(&one).is_nan());

    let min = NumberValue::Rational(Rational::new(i128::MIN, 1));
    assert!(min.negate().is_nan());
}

#[test]
#[should_panic(expected = "Rational numerator overflow")]
fn test_canonicalize_overflow_panics() {
    // i128::MIN / -1 = 2^127 which overflows i128
    let mut r = Rational {
        num: i128::MIN,
        den: -1,
    };
    r.canonicalize();
}

#[test]
#[should_panic(expected = "Rational denominator must not be zero")]
fn test_canonicalize_den_zero_panics() {
    let mut r = Rational { num: 5, den: 0 };
    r.canonicalize();
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
    let orig_unc = Number::new_uncertainty(val, unc);
    let cloned_unc = orig_unc.clone();

    assert_eq!(orig_unc, cloned_unc);

    if let (
        NumberValue::Uncertainty {
            value: orig_v,
            uncertainty: orig_u,
        },
        NumberValue::Uncertainty {
            value: cloned_v,
            uncertainty: cloned_u,
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
    let unc = Number::new_uncertainty(val_val, unc_val);
    assert!(unc.approximate());
    if let NumberValue::Uncertainty {
        value: v,
        uncertainty: u,
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

    // 2. Extremely large values in add_rationals and negate causing NaN
    let max_rat = NumberValue::Rational(Rational::new(i128::MAX, 1));
    let two_rat = NumberValue::Rational(Rational::new(2, 1));
    assert!(max_rat.add(&two_rat).is_nan());

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
        };
    }
    assert_eq!(current_val, current_val2);
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
    assert_eq!(max_rat.checked_add(&one_rat), None);

    let min_rat = Rational::try_new(i128::MIN, 1).unwrap();
    assert_eq!(min_rat.checked_sub(&one_rat), None);

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
        assert_eq!(lower.value(), -1.5);
        assert_eq!(upper.value(), 0.5);
    } else {
        panic!("Expected interval");
    }

    // [0, 2] - nv_half (0.5) = [-0.5, 1.5]
    if let NumberValue::Interval { lower, upper } = interval.sub(&nv_half) {
        assert_eq!(lower.value(), -0.5);
        assert_eq!(upper.value(), 1.5);
    } else {
        panic!("Expected interval");
    }

    // Uncertainty promotions
    let uncertainty = NumberValue::Uncertainty {
        value: Box::new(NumberValue::Rational(Rational::new(5, 1))),
        uncertainty: Box::new(NumberValue::Rational(Rational::new(1, 2))),
    };
    // nv_half (0.5) - (5 +/- 0.5) = -4.5 +/- 0.5
    if let NumberValue::Uncertainty {
        value,
        uncertainty: unc,
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
