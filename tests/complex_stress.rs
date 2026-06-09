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

    // 3. Division by zero (special case)
    // (1 + 1i) / 0 = inf + inf i
    let zero = Number::from_i32(0);
    let div_zero = one_complex.div(&zero);
    let (real_div_zero, imag_div_zero) = div_zero.to_canonical_real_imag();
    assert_eq!(real_div_zero, NumberValue::PlusInfinity);
    assert_eq!(imag_div_zero, NumberValue::PlusInfinity);

    // 4. NaN propagation
    let nan_num = Number::from_f64(f64::NAN);
    let nan_complex = Number::new_complex(nan_num.clone(), Number::from_i32(1));
    assert!(nan_complex.is_nan());

    let nan_add = nan_complex.add(&one_complex);
    assert!(nan_add.is_nan());

    // (1 + 1i) / (inf + inf i)
    let inf_complex3 = Number::new_complex(Number::from_f64(f64::INFINITY), Number::from_f64(f64::INFINITY));
    let div_inf_complex = one_complex.div(&inf_complex3);
    println!("(1 + i) / (inf + inf i) = {:?}", div_inf_complex);
}

#[test]
fn test_complex_stress_conjugate_and_norm() {
    // 1. conjugate(inf + inf i) = inf - inf i
    let inf_complex = Number::new_complex(Number::from_f64(f64::INFINITY), Number::from_f64(f64::INFINITY));
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

    if let NumberValue::Interval { lower: rl, upper: ru } = real_prod {
        assert_eq!(rl.value(), -8.0);
        assert_eq!(ru.value(), -3.0);
    } else {
        panic!("Expected interval real part");
    }

    if let NumberValue::Interval { lower: il, upper: iu } = imag_prod {
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
    );
    let imag_a = Number::new_uncertainty(
        NumberValue::Rational(Rational::new(3, 1)),
        NumberValue::Float(Float::from_f64(0.2, 53)),
    );
    let a = Number::new_complex(real_a, imag_a);

    // scalar = 2
    let scalar = Number::from_i32(2);

    // prod = a * 2 = (4 +/- 0.2) + (6 +/- 0.4)i
    let prod = a.mul(&scalar);
    let (real_prod, imag_prod) = prod.to_canonical_real_imag();

    if let NumberValue::Uncertainty { value: rv, uncertainty: ru } = real_prod {
        assert_eq!(*rv, NumberValue::Rational(Rational::new(4, 1)));
        if let NumberValue::Float(ruf) = &*ru {
            assert!((ruf.value() - 0.2).abs() < 1e-9);
        } else {
            panic!("Expected float uncertainty in real part");
        }
    } else {
        panic!("Expected uncertainty in real part");
    }

    if let NumberValue::Uncertainty { value: iv, uncertainty: iu } = imag_prod {
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
