use libqalculate_rust::number::{Float, Number, NumberValue, Rational};
use proptest::prelude::*;

// 1. Complex numbers with zero or negative real/imaginary parts
#[test]
fn test_complex_zero_and_negative_parts() {
    // real = 0, imag = 0
    let c_zero = Number::new_complex(Number::from_i32(0), Number::from_i32(0));
    assert!(c_zero.is_zero());
    assert!(c_zero.is_real_zero());
    assert!(!c_zero.is_complex());
    assert!(c_zero.has_real_part());
    assert!(!c_zero.has_imaginary_part());

    // real = -5, imag = 0
    let c_real_neg = Number::new_complex(Number::from_i32(-5), Number::from_i32(0));
    assert!(!c_real_neg.is_zero());
    assert!(!c_real_neg.is_real_zero());
    assert!(!c_real_neg.is_complex());
    assert!(c_real_neg.has_real_part());
    assert!(!c_real_neg.has_imaginary_part());
    if let NumberValue::Rational(r) = c_real_neg.value() {
        assert_eq!(r.num(), -5);
        assert_eq!(r.den(), 1);
    } else {
        panic!("Expected rational");
    }

    // real = 0, imag = -5
    let c_imag_neg = Number::new_complex(Number::from_i32(0), Number::from_i32(-5));
    assert!(!c_imag_neg.is_zero());
    assert!(c_imag_neg.is_real_zero());
    assert!(c_imag_neg.is_complex());
    assert!(!c_imag_neg.has_real_part());
    assert!(c_imag_neg.has_imaginary_part());
    let (_, imag_part) = c_imag_neg.to_canonical_real_imag();
    if let NumberValue::Rational(r) = imag_part {
        assert_eq!(r.num(), -5);
        assert_eq!(r.den(), 1);
    } else {
        panic!("Expected rational imaginary part");
    }

    // real = -3, imag = -5
    let c_both_neg = Number::new_complex(Number::from_i32(-3), Number::from_i32(-5));
    assert!(!c_both_neg.is_zero());
    assert!(!c_both_neg.is_real_zero());
    assert!(c_both_neg.is_complex());
    assert!(c_both_neg.has_real_part());
    assert!(c_both_neg.has_imaginary_part());
    let (real_part, imag_part) = c_both_neg.to_canonical_real_imag();
    if let NumberValue::Rational(r) = real_part {
        assert_eq!(r.num(), -3);
    } else {
        panic!("Expected rational real part");
    }
    if let NumberValue::Rational(r) = imag_part {
        assert_eq!(r.num(), -5);
    } else {
        panic!("Expected rational imaginary part");
    }
}

// 2. Nested complex flattening
#[test]
fn test_nested_complex_flattening() {
    // Construct nested complex: c = (a + b*i) + (c + d*i)*i
    // Mathematically: c = (a - d) + (b + c)*i
    // Let's test with a = 3, b = 2, c = 4, d = 5
    let real_nested = Number::new_complex(Number::from_i32(3), Number::from_i32(2)); // 3 + 2i
    let imag_nested = Number::new_complex(Number::from_i32(4), Number::from_i32(5)); // 4 + 5i
    let flat = Number::new_complex(real_nested, imag_nested);

    // Expected real part: 3 - 5 = -2
    // Expected imaginary part: 2 + 4 = 6
    assert!(flat.is_complex());
    let (real_part, imag_part) = flat.to_canonical_real_imag();
    assert_eq!(real_part, NumberValue::Rational(Rational::new(-2, 1)));
    assert_eq!(imag_part, NumberValue::Rational(Rational::new(6, 1)));

    // Deep nested flattening (depth 3)
    // flat_2 = flat + (1 - 1i) * i
    // Mathematically: (-2 + 6i) + (1 - i)*i = (-2 - (-1)) + (6 + 1)i = -1 + 7i
    let flat_2 = Number::new_complex(
        flat,
        Number::new_complex(Number::from_i32(1), Number::from_i32(-1)),
    );
    let (real_part_2, imag_part_2) = flat_2.to_canonical_real_imag();
    assert_eq!(real_part_2, NumberValue::Rational(Rational::new(-1, 1)));
    assert_eq!(imag_part_2, NumberValue::Rational(Rational::new(7, 1)));
}

// 3. Floating point and Rational equality under different precisions and representations
#[test]
fn test_float_and_rational_equality() {
    // Compare exact Rational 1/2 with Floats of different precisions
    let r_half = Number::from_rational(Rational::new(1, 2));
    let f_half_24 = Number::from_float(Float::from_f64(0.5, 24));
    let f_half_128 = Number::from_float(Float::from_f64(0.5, 128));

    assert_ne!(r_half, f_half_24);
    assert_ne!(r_half, f_half_128);
    assert_eq!(f_half_24, f_half_128);

    // Compare Floats with different precisions directly
    let f1 = Float::from_f64(1.2345, 12);
    let f2 = Float::from_f64(1.2345, 256);
    assert_ne!(f1, f2);

    // Point interval equality
    let interval_pt = Number::new_interval(Float::from_f64(1.5, 24), Float::from_f64(1.5, 128));
    let scalar_rational = Number::from_rational(Rational::new(3, 2));
    assert_ne!(interval_pt, scalar_rational);

    // Uncertainty with zero uncertainty equality
    let unc_zero = Number::new_uncertainty(
        NumberValue::Rational(Rational::new(2, 1)),
        NumberValue::Rational(Rational::new(0, 1)),
        false,
    );
    let scalar_rational_2 = Number::from_rational(Rational::new(2, 1));
    assert_eq!(unc_zero, scalar_rational_2);
}

// 4. i128::MIN overflow checks for gcd and canonicalization
#[test]
fn test_i128_min_self_cancels() {
    // i128::MIN / i128::MIN = 1/1 (the absolute values cancel)
    let r_min_self = Rational::new(i128::MIN, i128::MIN);
    assert_eq!(r_min_self.num(), 1);
    assert_eq!(r_min_self.den(), 1);
}

#[test]
fn test_i128_min_over_one() {
    // i128::MIN / 1 is representable (stays as i128::MIN / 1)
    let r_min_one = Rational::new(i128::MIN, 1);
    assert_eq!(r_min_one.num(), i128::MIN);
    assert_eq!(r_min_one.den(), 1);
}

#[test]
#[should_panic(expected = "Numerator exceeds i128")]
fn test_i128_min_div_neg1_panics() {
    // i128::MIN / -1 = 2^127, which overflows i128.
    // This is now caught and panics instead of silently wrapping.
    let r = Rational::new(i128::MIN, -1);
    let _ = r.num();
}

#[test]
#[should_panic(expected = "Denominator exceeds i128")]
fn test_den_i128_min_panics() {
    // 1 / i128::MIN: denominator 2^127 overflows i128.
    let r = Rational::new(1, i128::MIN);
    let _ = r.den();
}

#[test]
#[should_panic(expected = "Rational denominator must not be zero")]
fn test_den_zero_panics() {
    // Zero denominator is undefined and must panic.
    let _ = Rational::new(5, 0);
}

// 5. Property-based tests for adversarial/edge inputs using safe bounds to avoid i128 overflow
fn safe_rational_strategy() -> impl Strategy<Value = Rational> {
    (
        any::<i32>(),
        any::<i32>().prop_filter("den must not be zero", |d| *d != 0),
    )
        .prop_map(|(num, den)| Rational::new(num as i128, den as i128))
}

fn float_strategy() -> impl Strategy<Value = Float> {
    prop_oneof![
        (any::<f64>(), 1..=1000u32).prop_map(|(val, prec)| Float::from_f64(val, prec)),
        Just(Float::from_f64(f64::NAN, 53)),
        Just(Float::from_f64(f64::INFINITY, 53)),
        Just(Float::from_f64(f64::NEG_INFINITY, 53)),
    ]
}

fn adversarial_number_strategy() -> impl Strategy<Value = Number> {
    prop_oneof![
        safe_rational_strategy().prop_map(Number::from_rational),
        float_strategy().prop_map(Number::from_float),
        (float_strategy(), float_strategy()).prop_map(|(l, u)| Number::new_interval(l, u)),
        (any::<f64>()).prop_map(Number::from_f64),
    ]
}

proptest! {
    #[test]
    fn prop_test_complex_flattening_identities(r in adversarial_number_strategy(), i in adversarial_number_strategy()) {
        let c = Number::new_complex(r.clone(), i.clone());

        // Assert complex flattening results in at most one level of imaginary part
        if let Some(imag_part) = c.imaginary() {
            prop_assert!(imag_part.imaginary().is_none());
            prop_assert!(imag_part.is_imaginary());
        }
    }

    #[test]
    fn prop_test_nested_complex_identities(
        a in adversarial_number_strategy(),
        b in adversarial_number_strategy(),
        c in adversarial_number_strategy(),
        d in adversarial_number_strategy()
    ) {
        let c1 = Number::new_complex(a, b);
        let c2 = Number::new_complex(c, d);
        let c3 = Number::new_complex(c1, c2);

        // Verification that resulting structure is fully flat
        if let Some(imag_part) = c3.imaginary() {
            prop_assert!(imag_part.imaginary().is_none());
            prop_assert!(imag_part.is_imaginary());
        }
    }
}
