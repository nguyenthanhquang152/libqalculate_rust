use libqalculate_rust::number::{Float, Number, NumberValue, ComparisonResult};
use proptest::prelude::*;

#[test]
fn test_division_by_interval_containing_zero() {
    let prec = 53;
    let numerator = Number::from_f64(10.0);

    // Divisor interval containing zero: [-1.0, 1.0]
    let div_mid = Number::new_interval(Float::from_f64(-1.0, prec), Float::from_f64(1.0, prec));
    let res_mid = numerator.div(&div_mid);
    assert!(res_mid.is_nan(), "Expected NaN when dividing by interval [-1, 1]");

    // Divisor interval starting at zero: [0.0, 2.0]
    let div_start = Number::new_interval(Float::from_f64(0.0, prec), Float::from_f64(2.0, prec));
    let res_start = numerator.div(&div_start);
    assert!(res_start.is_nan(), "Expected NaN when dividing by interval [0, 2]");

    // Divisor interval ending at zero: [-2.0, 0.0]
    let div_end = Number::new_interval(Float::from_f64(-2.0, prec), Float::from_f64(0.0, prec));
    let res_end = numerator.div(&div_end);
    assert!(res_end.is_nan(), "Expected NaN when dividing by interval [-2, 0]");

    // Divisor interval point zero: [0.0, 0.0]
    let div_zero = Number::new_interval(Float::from_f64(0.0, prec), Float::from_f64(0.0, prec));
    let res_zero = numerator.div(&div_zero);
    assert!(res_zero.is_nan(), "Expected NaN when dividing by interval [0, 0]");

    // Divisor uncertainty containing zero: 1.0 +/- 2.0 -> [-1.0, 3.0]
    // Since nominal divisor is 1.0 (non-zero), division is mathematically defined.
    // Due to the complex division path in `Number::div` which canonicalizes to (a+bi)/(c+di)
    // and calculates a*c/(c^2), uncertainty is propagated to 10.0 +/- 100.0.
    let val = NumberValue::Float(Float::from_f64(1.0, prec));
    let unc = NumberValue::Float(Float::from_f64(2.0, prec));
    let div_unc = Number::new_uncertainty(val, unc.clone());
    let res_unc = numerator.div(&div_unc);
    assert!(!res_unc.is_nan(), "Uncertainty division with non-zero nominal divisor should not be NaN");
    if let NumberValue::Uncertainty { value: r_val, uncertainty: r_unc } = res_unc.value() {
        if let NumberValue::Float(fv) = &**r_val {
            assert_eq!(fv.value(), 10.0);
        } else {
            panic!("Expected Float value");
        }
        if let NumberValue::Float(fu) = &**r_unc {
            assert_eq!(fu.value(), 100.0);
        } else {
            panic!("Expected Float uncertainty");
        }
    } else {
        panic!("Expected Uncertainty result");
    }

    // Uncertainty with nominal zero divisor: 0.0 +/- 2.0
    // Dividing by this should yield NaN or infinity because the nominal divisor is 0.0.
    let val_zero = NumberValue::Float(Float::from_f64(0.0, prec));
    let div_unc_zero = Number::new_uncertainty(val_zero, unc);
    let res_unc_zero = numerator.div(&div_unc_zero);
    assert!(res_unc_zero.is_nan() || res_unc_zero.is_infinite());
}

#[test]
fn test_comparison_result_exhaustive() {
    let prec = 53;
    
    // A = [10.0, 20.0]
    let a = Number::new_interval(Float::from_f64(10.0, prec), Float::from_f64(20.0, prec));

    // B completely less: [1.0, 5.0]
    let b_less = Number::new_interval(Float::from_f64(1.0, prec), Float::from_f64(5.0, prec));
    assert_eq!(a.compare(&b_less), ComparisonResult::Less);
    assert!(a.is_greater_than(&b_less));
    assert!(!a.is_less_than(&b_less));

    // B completely greater: [25.0, 30.0]
    let b_greater = Number::new_interval(Float::from_f64(25.0, prec), Float::from_f64(30.0, prec));
    assert_eq!(a.compare(&b_greater), ComparisonResult::Greater);
    assert!(!a.is_greater_than(&b_greater));
    assert!(a.is_less_than(&b_greater));

    // B identical: [10.0, 20.0]
    let b_ident = Number::new_interval(Float::from_f64(10.0, prec), Float::from_f64(20.0, prec));
    assert_eq!(a.compare(&b_ident), ComparisonResult::EqualLimits);

    // B contains A: [5.0, 25.0]
    let b_contains = Number::new_interval(Float::from_f64(5.0, prec), Float::from_f64(25.0, prec));
    assert_eq!(a.compare(&b_contains), ComparisonResult::Contains);

    // B contained in A: [12.0, 18.0]
    let b_contained = Number::new_interval(Float::from_f64(12.0, prec), Float::from_f64(18.0, prec));
    assert_eq!(a.compare(&b_contained), ComparisonResult::Contained);

    // B overlaps A on the left: [5.0, 15.0]
    let b_overlap_left = Number::new_interval(Float::from_f64(5.0, prec), Float::from_f64(15.0, prec));
    assert_eq!(a.compare(&b_overlap_left), ComparisonResult::OverlappingLess);

    // B overlaps A on the right: [15.0, 25.0]
    let b_overlap_right = Number::new_interval(Float::from_f64(15.0, prec), Float::from_f64(25.0, prec));
    assert_eq!(a.compare(&b_overlap_right), ComparisonResult::OverlappingGreater);
}

// proptest strategies
fn interval_containing_zero_strategy() -> impl Strategy<Value = Number> {
    (
        proptest::num::f64::NEGATIVE,
        proptest::num::f64::POSITIVE,
    ).prop_map(|(l, u): (f64, f64)| {
        Number::new_interval(Float::from_f64(l, 53), Float::from_f64(u, 53))
    })
}

fn interval_excluding_zero_strategy() -> impl Strategy<Value = Number> {
    prop_oneof![
        (proptest::num::f64::POSITIVE, proptest::num::f64::POSITIVE).prop_map(|(x, y): (f64, f64)| {
            let l = x.min(y);
            let u = x.max(y);
            Number::new_interval(Float::from_f64(l, 53), Float::from_f64(u, 53))
        }),
        (proptest::num::f64::NEGATIVE, proptest::num::f64::NEGATIVE).prop_map(|(x, y): (f64, f64)| {
            let l = x.min(y);
            let u = x.max(y);
            Number::new_interval(Float::from_f64(l, 53), Float::from_f64(u, 53))
        })
    ]
}

proptest! {
    #[test]
    fn prop_test_div_by_zero_interval_always_nan(
        numerator in any::<f64>().prop_map(Number::from_f64),
        divisor in interval_containing_zero_strategy()
    ) {
        let res = numerator.div(&divisor);
        prop_assert!(res.is_nan());
    }

    #[test]
    fn prop_test_div_by_nonzero_interval_does_not_panic(
        numerator in any::<f64>().prop_map(Number::from_f64),
        divisor in interval_excluding_zero_strategy()
    ) {
        let _res = numerator.div(&divisor);
    }
}
