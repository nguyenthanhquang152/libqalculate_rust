use libqalculate_rust::number::{Float, Number, NumberValue, Rational};
use proptest::prelude::*;

// Use i32 range to avoid i128 overflow edge cases in canonicalization.
// Exclude den=0 since Rational::new panics on zero denominator.
fn rational_strategy() -> impl Strategy<Value = Rational> {
    (any::<i32>(), any::<i32>().prop_filter("den must not be zero", |d| *d != 0))
        .prop_map(|(num, den)| Rational::new(num as i128, den as i128))
}

fn float_strategy() -> impl Strategy<Value = Float> {
    (any::<f64>(), 1..=1000u32).prop_map(|(val, prec)| Float::from_f64(val, prec))
}

fn leaf_value_strategy() -> impl Strategy<Value = NumberValue> {
    prop_oneof![
        rational_strategy().prop_map(NumberValue::Rational),
        float_strategy().prop_map(NumberValue::Float),
        Just(NumberValue::PlusInfinity),
        Just(NumberValue::MinusInfinity),
        Just(NumberValue::NaN),
    ]
}

fn interval_value_strategy() -> impl Strategy<Value = NumberValue> {
    (any::<f64>(), any::<f64>(), 1..=1000u32).prop_map(|(l, u, prec)| NumberValue::Interval {
        lower: Float::from_f64(l, prec),
        upper: Float::from_f64(u, prec),
    })
}

fn number_value_strategy() -> impl Strategy<Value = NumberValue> {
    let leaf = prop_oneof![leaf_value_strategy(), interval_value_strategy(),];
    leaf.prop_recursive(3, 16, 2, |inner| {
        (inner.clone(), inner).prop_map(|(v, u)| NumberValue::Uncertainty {
            value: Box::new(v),
            uncertainty: Box::new(u),
        })
    })
}

fn base_number_strategy() -> impl Strategy<Value = Number> {
    prop_oneof![
        rational_strategy().prop_map(Number::from_rational),
        float_strategy().prop_map(Number::from_float),
        (float_strategy(), float_strategy()).prop_map(|(l, u)| Number::new_interval(l, u)),
        (number_value_strategy(), number_value_strategy())
            .prop_map(|(v, u)| Number::new_uncertainty(v, u)),
        any::<f64>().prop_map(Number::from_f64),
    ]
}

fn number_strategy() -> impl Strategy<Value = Number> {
    prop_oneof![
        base_number_strategy(),
        (base_number_strategy(), base_number_strategy())
            .prop_map(|(r, i)| Number::new_complex(r, i)),
    ]
}

proptest! {
    #[test]
    fn test_partial_eq_reflexivity(x in number_strategy()) {
        // Reflexivity: x == x
        // Note: Float NaN is handled specifically in the implementation to return true
        prop_assert_eq!(&x, &x);
    }

    #[test]
    fn test_partial_eq_symmetry(x in number_strategy(), y in number_strategy()) {
        // Symmetry: (x == y) == (y == x)
        prop_assert_eq!(x == y, y == x);
    }

    #[test]
    fn test_predicates_correctness(x in number_strategy()) {
        let (real, imag) = x.to_canonical_real_imag();

        // is_zero correctness
        let expected_is_zero = real.is_real_zero() && imag.is_real_zero();
        prop_assert_eq!(x.is_zero(), expected_is_zero);

        // is_complex correctness
        let expected_is_complex = !imag.is_real_zero();
        prop_assert_eq!(x.is_complex(), expected_is_complex);

        // has_real_part correctness
        let expected_has_real_part = !real.is_real_zero() || imag.is_real_zero();
        prop_assert_eq!(x.has_real_part(), expected_has_real_part);

        // has_imaginary_part correctness
        prop_assert_eq!(x.has_imaginary_part(), x.is_complex());
    }
}

#[test]
fn test_partial_eq_transitivity_violation() {
    // Demonstration of transitivity violation: x == y && y == z but x != z
    // Due to conversion between exact Rational and inexact Float representations.
    let x = Number::from_rational(Rational::new(1, 3));
    let y = Number::from_float(Float::from_f64(1.0 / 3.0, 53));
    let z = Number::from_rational(Rational::new(3333333333333333, 10000000000000000));

    // Confirm that x == y
    assert_eq!(x, y);
    // Confirm that y == z
    assert_eq!(y, z);
    // Transitivity would require x == z, but since both are Rational they are compared exactly
    // and are not equal.
    assert_ne!(x, z);
}

#[test]
fn test_getter_consistency_via_uncertainty() {
    // Getter consistency between Number::precision() and Number::value().precision() for Uncertainty.
    // value precision = 0 (Rational), uncertainty precision = 100 (Float)
    let val = NumberValue::Rational(Rational::new(1, 1));
    let unc = NumberValue::Float(Float::from_f64(1.0, 100));
    let n = Number::new_uncertainty(val, unc);

    // Number::precision() is computed as max(value.precision(), uncertainty.precision()) = 100
    assert_eq!(n.precision(), 100);
    // NumberValue::precision() for Uncertainty variant also returns the max precision = 100
    assert_eq!(n.value().precision(), 100);
}

#[test]
fn test_getter_consistency_via_complex() {
    // Getter consistency in precision and approximate flags when combining Rational and Float in complex numbers.
    let real = Number::from_rational(Rational::new(3, 1)); // prec = 0, approx = false
    let imag = Number::new_complex(
        Number::from_rational(Rational::new(0, 1)),
        Number::from_float(Float::from_f64(4.0, 53)), // prec = 53, approx = true
    );
    let c = Number::new_complex(real, imag);

    // The resulting real value is Float(-4.0, 53) because real (Rational(3)) is added to imag's nested float part.
    if let NumberValue::Float(f) = c.value() {
        assert_eq!(f.prec(), 53);
        assert_eq!(c.value().precision(), 53);
    } else {
        panic!("Expected Float value");
    }

    // Now c's getters correctly compute precision = 53 and approximate = true,
    // reflecting the properties of both real and imaginary inputs.
    assert_eq!(c.precision(), 53);
    assert!(c.approximate());
}
