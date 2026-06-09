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
