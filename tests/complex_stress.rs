use libqalculate_rust::number::{Float, Number, NumberValue, Rational};

#[test]
fn test_complex_stress_nan_and_infinity() {
    // 1. Infinity + i addition and multiplication
    let inf_real = Number::from_f64(f64::INFINITY);
    let one_imag = Number::from_i32(1);
    let inf_complex = Number::new_complex(inf_real.clone(), one_imag.clone());

    // inf + 1i has real part PlusInfinity, imag part Rational(1)
    let (real_part, imag_part) = inf_complex.to_canonical_real_imag();
    assert_eq!(real_part, NumberValue::PlusInfinity);
    assert_eq!(imag_part, NumberValue::Rational(Rational::new(1, 1)));

    // (inf + 1i) + (5 - 3i) = inf - 2i
    let other = Number::new_complex(Number::from_i32(5), Number::from_i32(-3));
    let sum = inf_complex.add(&other);
    let (real_sum, imag_sum) = sum.to_canonical_real_imag();
    assert_eq!(real_sum, NumberValue::PlusInfinity);
    assert_eq!(imag_sum, NumberValue::Rational(Rational::new(-2, 1)));

    // (inf + 1i) * (2 + 0i) = inf + 2i
    let scalar = Number::from_i32(2);
    let prod1 = inf_complex.mul(&scalar);
    let (real_prod1, imag_prod1) = prod1.to_canonical_real_imag();
    assert_eq!(real_prod1, NumberValue::PlusInfinity);
    assert_eq!(imag_prod1, NumberValue::Rational(Rational::new(2, 1)));

    // (inf + 1i) * (0 + 2i) = -2 + inf i
    // Mathematically: (inf + i) * 2i = -2 + 2*inf i = -2 + inf i
    let pure_imag_scalar = Number::new_complex(Number::from_i32(0), Number::from_i32(2));
    let prod2 = inf_complex.mul(&pure_imag_scalar);
    let (real_prod2, imag_prod2) = prod2.to_canonical_real_imag();
    assert_eq!(real_prod2, NumberValue::Rational(Rational::new(-2, 1)));
    assert_eq!(imag_prod2, NumberValue::PlusInfinity);

    // 2. Division by infinity
    // (1 + 1i) / inf = 0 + 0i = 0
    let one_complex = Number::new_complex(Number::from_i32(1), Number::from_i32(1));
    let div_inf = one_complex.div(&inf_real);
    assert!(div_inf.is_zero());

    // 3. Division by exact zero declines evaluation instead of fabricating infinities.
    let zero = Number::from_i32(0);
    let div_zero = one_complex.div(&zero);
    assert!(div_zero.is_nan());

    // 4. NaN propagation
    let nan_num = Number::from_f64(f64::NAN);
    let nan_complex = Number::new_complex(nan_num.clone(), Number::from_i32(1));
    assert!(nan_complex.is_nan());

    let nan_add = nan_complex.add(&one_complex);
    assert!(nan_add.is_nan());

    // (1 + 1i) / (inf + inf i)
    let inf_complex3 = Number::new_complex(
        Number::from_f64(f64::INFINITY),
        Number::from_f64(f64::INFINITY),
    );
    let div_inf_complex = one_complex.div(&inf_complex3);
    println!("(1 + i) / (inf + inf i) = {:?}", div_inf_complex);
}

#[test]
fn test_complex_stress_conjugate_and_norm() {
    // 1. conjugate(inf + inf i) = inf - inf i
    let inf_complex = Number::new_complex(
        Number::from_f64(f64::INFINITY),
        Number::from_f64(f64::INFINITY),
    );
    let conj = inf_complex.conjugate();
    let (real_conj, imag_conj) = conj.to_canonical_real_imag();
    assert_eq!(real_conj, NumberValue::PlusInfinity);
    assert_eq!(imag_conj, NumberValue::MinusInfinity);

    // 2. norm(inf + 1i) = inf
    let inf_complex2 = Number::new_complex(Number::from_f64(f64::INFINITY), Number::from_i32(1));
    let norm = inf_complex2.norm();
    assert_eq!(norm.value(), &NumberValue::PlusInfinity);

    // 3. norm(NaN + 1i) = NaN
    let nan_complex = Number::new_complex(Number::from_f64(f64::NAN), Number::from_i32(1));
    let norm_nan = nan_complex.norm();
    assert_eq!(norm_nan.value(), &NumberValue::NaN);
}

#[test]
fn test_complex_stress_intervals() {
    // ([1, 2] + [3, 4]i) * ([0, 0] + [1, 2]i)
    // Mathematically: real part: - [3, 4] * [1, 2] = -[3, 8] = [-8, -3]
    // imaginary part: [1, 2] * [1, 2] = [1, 4]
    // So result should be [-8, -3] + [1, 4]i
    let real_a = Number::new_interval(Float::from_f64(1.0, 53), Float::from_f64(2.0, 53));
    let imag_a = Number::new_interval(Float::from_f64(3.0, 53), Float::from_f64(4.0, 53));
    let a = Number::new_complex(real_a, imag_a);

    let real_b = Number::new_interval(Float::from_f64(0.0, 53), Float::from_f64(0.0, 53));
    let imag_b = Number::new_interval(Float::from_f64(1.0, 53), Float::from_f64(2.0, 53));
    let b = Number::new_complex(real_b, imag_b);

    let prod = a.mul(&b);
    let (real_prod, imag_prod) = prod.to_canonical_real_imag();

    if let NumberValue::Interval {
        lower: rl,
        upper: ru,
    } = real_prod
    {
        assert_eq!(rl.value(), -8.0);
        assert_eq!(ru.value(), -3.0);
    } else {
        panic!("Expected interval real part");
    }

    if let NumberValue::Interval {
        lower: il,
        upper: iu,
    } = imag_prod
    {
        assert_eq!(il.value(), 1.0);
        assert_eq!(iu.value(), 4.0);
    } else {
        panic!("Expected interval imaginary part");
    }
}

#[test]
fn test_complex_stress_uncertainty() {
    // a = (2 +/- 0.1) + (3 +/- 0.2)i
    let real_a = Number::new_uncertainty(
        NumberValue::Rational(Rational::new(2, 1)),
        NumberValue::Float(Float::from_f64(0.1, 53)),
        false,
    );
    let imag_a = Number::new_uncertainty(
        NumberValue::Rational(Rational::new(3, 1)),
        NumberValue::Float(Float::from_f64(0.2, 53)),
        false,
    );
    let a = Number::new_complex(real_a, imag_a);

    // scalar = 2
    let scalar = Number::from_i32(2);

    // prod = a * 2 = (4 +/- 0.2) + (6 +/- 0.4)i
    let prod = a.mul(&scalar);
    let (real_prod, imag_prod) = prod.to_canonical_real_imag();

    if let NumberValue::Uncertainty {
        value: rv,
        uncertainty: ru,
        ..
    } = real_prod
    {
        assert_eq!(*rv, NumberValue::Rational(Rational::new(4, 1)));
        if let NumberValue::Float(ruf) = &*ru {
            assert!((ruf.value() - 0.2).abs() < 1e-9);
        } else {
            panic!("Expected float uncertainty in real part");
        }
    } else {
        panic!("Expected uncertainty in real part");
    }

    if let NumberValue::Uncertainty {
        value: iv,
        uncertainty: iu,
        ..
    } = imag_prod
    {
        assert_eq!(*iv, NumberValue::Rational(Rational::new(6, 1)));
        if let NumberValue::Float(iuf) = &*iu {
            assert!((iuf.value() - 0.4).abs() < 1e-9);
        } else {
            panic!("Expected float uncertainty in imag part");
        }
    } else {
        panic!("Expected uncertainty in imag part");
    }
}

#[test]
fn test_complex_stress_nested_complex() {
    // 1. (1 + 2i) + (3 + 4i)i = -3 + 5i
    let real_part = Number::new_complex(Number::from_i32(1), Number::from_i32(2)); // 1 + 2i
    let imag_part = Number::new_complex(Number::from_i32(3), Number::from_i32(4)); // 3 + 4i
    let nested = Number::new_complex(real_part, imag_part);

    let (real_val, imag_val) = nested.to_canonical_real_imag();
    assert_eq!(real_val, NumberValue::Rational(Rational::new(-3, 1)));
    assert_eq!(imag_val, NumberValue::Rational(Rational::new(5, 1)));

    // 2. Nested with infinity and NaN: (inf + 2i) + (3 + NaN i)i = NaN + 5i
    let inf_real = Number::from_f64(f64::INFINITY);
    let real_part_inf = Number::new_complex(inf_real, Number::from_i32(2)); // inf + 2i
    let nan_val = Number::from_f64(f64::NAN);
    let imag_part_nan = Number::new_complex(Number::from_i32(3), nan_val); // 3 + NaN i
    let nested_nan = Number::new_complex(real_part_inf, imag_part_nan);
    assert!(nested_nan.is_nan());
}

#[test]
fn test_complex_stress_mixed_interval_uncertainty() {
    // z = (2 +/- 0.1) + [3, 4]i
    let real_z = Number::new_uncertainty(
        NumberValue::Rational(Rational::new(2, 1)),
        NumberValue::Float(Float::from_f64(0.1, 53)),
        false,
    );
    let imag_z = Number::new_interval(Float::from_f64(3.0, 53), Float::from_f64(4.0, 53));
    let z = Number::new_complex(real_z, imag_z);

    // 1. conjugate(z) = (2 +/- 0.1) - [3, 4]i = (2 +/- 0.1) + [-4, -3]i
    let conj = z.conjugate();
    let (real_conj, imag_conj) = conj.to_canonical_real_imag();
    if let NumberValue::Uncertainty {
        value, uncertainty, ..
    } = real_conj
    {
        assert_eq!(*value, NumberValue::Rational(Rational::new(2, 1)));
        if let NumberValue::Float(u) = &*uncertainty {
            assert!((u.value() - 0.1).abs() < 1e-9);
        } else {
            panic!("Expected float uncertainty");
        }
    } else {
        panic!("Expected uncertainty real part");
    }
    if let NumberValue::Interval { lower, upper } = imag_conj {
        assert_eq!(lower.value(), -4.0);
        assert_eq!(upper.value(), -3.0);
    } else {
        panic!("Expected interval imag part");
    }

    // 2. z + (1 + 1i) = (3 +/- 0.1) + [4, 5]i
    let one_plus_i = Number::new_complex(Number::from_i32(1), Number::from_i32(1));
    let sum = z.add(&one_plus_i);
    let (real_sum, imag_sum) = sum.to_canonical_real_imag();
    if let NumberValue::Uncertainty {
        value, uncertainty, ..
    } = real_sum
    {
        assert_eq!(*value, NumberValue::Rational(Rational::new(3, 1)));
        if let NumberValue::Float(u) = &*uncertainty {
            assert!((u.value() - 0.1).abs() < 1e-9);
        } else {
            panic!("Expected float uncertainty");
        }
    } else {
        panic!("Expected uncertainty real part");
    }
    if let NumberValue::Interval { lower, upper } = imag_sum {
        assert_eq!(lower.value(), 4.0);
        assert_eq!(upper.value(), 5.0);
    } else {
        panic!("Expected interval imag part");
    }

    // 3. z * 2 = (4 +/- 0.2) + [6, 8]i
    let scalar_two = Number::from_i32(2);
    let prod_scalar = z.mul(&scalar_two);
    let (real_prod_scalar, imag_prod_scalar) = prod_scalar.to_canonical_real_imag();
    if let NumberValue::Uncertainty {
        value, uncertainty, ..
    } = real_prod_scalar
    {
        assert_eq!(*value, NumberValue::Rational(Rational::new(4, 1)));
        if let NumberValue::Float(u) = &*uncertainty {
            assert!((u.value() - 0.2).abs() < 1e-9);
        } else {
            panic!("Expected float uncertainty");
        }
    } else {
        panic!("Expected uncertainty real part");
    }
    if let NumberValue::Interval { lower, upper } = imag_prod_scalar {
        assert_eq!(lower.value(), 6.0);
        assert_eq!(upper.value(), 8.0);
    } else {
        panic!("Expected interval imag part");
    }

    // 4. z * i = [-4, -3] + (2 +/- 0.1)i
    let i_unit = Number::new_complex(Number::from_i32(0), Number::from_i32(1));
    let prod_i = z.mul(&i_unit);
    let (real_prod_i, imag_prod_i) = prod_i.to_canonical_real_imag();
    if let NumberValue::Interval { lower, upper } = real_prod_i {
        assert_eq!(lower.value(), -4.0);
        assert_eq!(upper.value(), -3.0);
    } else {
        panic!("Expected interval real part");
    }
    if let NumberValue::Uncertainty {
        value, uncertainty, ..
    } = imag_prod_i
    {
        assert_eq!(*value, NumberValue::Rational(Rational::new(2, 1)));
        if let NumberValue::Float(u) = &*uncertainty {
            assert!((u.value() - 0.1).abs() < 1e-9);
        } else {
            panic!("Expected float uncertainty");
        }
    } else {
        panic!("Expected uncertainty imag part");
    }
}

#[test]
fn test_complex_stress_extreme_division() {
    // 1. Exact zero divisor declines evaluation instead of fabricating infinities.
    let one_minus_i = Number::new_complex(Number::from_i32(1), Number::from_i32(-1));
    let div_zero = one_minus_i.div(&Number::from_i32(0));
    assert!(div_zero.is_nan());

    // 2. (inf + 2i) / (2 + 3i) = inf - inf i
    let inf_real = Number::from_f64(f64::INFINITY);
    let inf_complex = Number::new_complex(inf_real, Number::from_i32(2));
    let divisor = Number::new_complex(Number::from_i32(2), Number::from_i32(3));
    let div_inf = inf_complex.div(&divisor);
    let (real_div_inf, imag_div_inf) = div_inf.to_canonical_real_imag();
    assert_eq!(real_div_inf, NumberValue::PlusInfinity);
    assert_eq!(imag_div_inf, NumberValue::MinusInfinity);

    // 3. (2 + 3i) / (0 + 1i) = 3 - 2i
    let num = Number::new_complex(Number::from_i32(2), Number::from_i32(3));
    let i_divisor = Number::new_complex(Number::from_i32(0), Number::from_i32(1));
    let res = num.div(&i_divisor);
    let (real_res, imag_res) = res.to_canonical_real_imag();
    assert_eq!(real_res, NumberValue::Rational(Rational::new(3, 1)));
    assert_eq!(imag_res, NumberValue::Rational(Rational::new(-2, 1)));
}
