use libqalculate_rust::number::{Float, Number, NumberValue, Rational};

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
    };

    let v2 = NumberValue::Rational(Rational::new(10, 1));
    let u2 = NumberValue::Rational(Rational::new(3, 2));
    let unc2 = NumberValue::Uncertainty {
        value: Box::new(v2),
        uncertainty: Box::new(u2),
    };

    let sum = unc1.add(&unc2);
    if let NumberValue::Uncertainty { value, uncertainty } = sum {
        if let NumberValue::Rational(r_val) = &*value {
            assert_eq!(r_val.num(), 15);
            assert_eq!(r_val.den(), 1);
        } else {
            panic!("Expected rational value");
        }

        if let NumberValue::Rational(r_unc) = &*uncertainty {
            assert_eq!(r_unc.num(), 2);
            assert_eq!(r_unc.den(), 1);
        } else {
            panic!("Expected rational uncertainty");
        }
    } else {
        panic!("Expected uncertainty result");
    }
}
