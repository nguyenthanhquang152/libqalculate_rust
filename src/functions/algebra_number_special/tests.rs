//! Tests for algebra, number, and special built-in functions.
//!
//! Covers `abs`, `sgn`, `round`, `floor`, `ceil`, `trunc`, `re`, `im`,
//! `arg`, `conj`, `gamma`, `erf`, `zeta`, `gammainc` and their aliases.

use crate::context::CalculatorContext;
use crate::functions::algebra_number_special::lookup;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_test_number(v: f64) -> crate::number::Number {
    if v.is_nan() || v.is_infinite() {
        crate::number::Number::from_f64(v)
    } else {
        match rug::Rational::from_f64(v) {
            Some(r) => crate::number::Number::from_rational(crate::number::Rational { value: r }),
            None => crate::number::Number::from_f64(v),
        }
    }
}

/// Evaluate a built-in function with numeric arguments and return the
/// result as a string.
fn eval_num(name: &str, args: &[f64]) -> String {
    let func = lookup(name).unwrap_or_else(|| panic!("no function '{name}'"));
    let expr_args: Vec<_> = args
        .iter()
        .map(|v| crate::ast::Expression::Number(make_test_number(*v)))
        .collect();
    let mut ctx = CalculatorContext::new();
    match func.evaluate(&expr_args, &mut ctx) {
        Ok(crate::ast::Expression::Number(n)) => n.to_qalc_string(),
        Ok(other) => format!("{other:?}"),
        Err(e) => format!("ERR: {e}"),
    }
}

/// Evaluate a built-in function with a complex argument and return the
/// result as a string.
fn eval_complex(name: &str, re: f64, im: f64) -> String {
    let func = lookup(name).unwrap_or_else(|| panic!("no function '{name}'"));
    let num = crate::number::Number::new_complex(
        make_test_number(re),
        make_test_number(im),
    );
    let expr_args = vec![crate::ast::Expression::Number(num)];
    let mut ctx = CalculatorContext::new();
    match func.evaluate(&expr_args, &mut ctx) {
        Ok(crate::ast::Expression::Number(n)) => n.to_qalc_string(),
        Ok(other) => format!("{other:?}"),
        Err(e) => format!("ERR: {e}"),
    }
}

// ---------------------------------------------------------------------------
// Catalog and lookup
// ---------------------------------------------------------------------------

#[test]
fn catalog_returns_all_functions() {
    let catalog = super::catalog();
    assert!(catalog.len() >= 14, "Expected ≥14 functions, got {}", catalog.len());
    let names: Vec<_> = catalog.iter().map(|f| f.name).collect();
    for expected in &[
        "abs", "sgn", "round", "floor", "ceil", "trunc", "re", "im",
        "arg", "conj", "gamma", "erf", "zeta", "gammainc",
    ] {
        assert!(names.contains(expected), "missing '{expected}' in catalog");
    }
}

#[test]
fn lookup_by_alias() {
    assert!(lookup("signum").is_some(), "signum should resolve");
    assert!(lookup("sign").is_some(), "sign should resolve");
    assert!(lookup("int").is_some(), "int (floor alias) should resolve");
    assert!(lookup("truncate").is_some(), "truncate should resolve");
    assert!(lookup("real").is_some(), "real (re alias) should resolve");
    assert!(lookup("imag").is_some(), "imag should resolve");
    assert!(lookup("imaginary").is_some(), "imaginary should resolve");
    assert!(lookup("argument").is_some(), "argument should resolve");
    assert!(lookup("conjugate").is_some(), "conjugate should resolve");
    assert!(lookup("igamma").is_some(), "igamma should resolve");
    assert!(lookup("nonexistent").is_none());
}

// ---------------------------------------------------------------------------
// abs
// ---------------------------------------------------------------------------

#[test]
fn abs_positive() {
    assert_eq!(eval_num("abs", &[3.5]), "3.5");
}

#[test]
fn abs_negative() {
    assert_eq!(eval_num("abs", &[-7.0]), "7");
}

#[test]
fn abs_zero() {
    assert_eq!(eval_num("abs", &[0.0]), "0");
}

#[test]
fn abs_complex() {
    // |3+4i| = 5
    assert_eq!(eval_complex("abs", 3.0, 4.0), "5");
}

// ---------------------------------------------------------------------------
// sgn
// ---------------------------------------------------------------------------

#[test]
fn sgn_positive() {
    assert_eq!(eval_num("sgn", &[42.0]), "1");
}

#[test]
fn sgn_negative() {
    assert_eq!(eval_num("sgn", &[-42.0]), "-1");
}

#[test]
fn sgn_zero() {
    assert_eq!(eval_num("sgn", &[0.0]), "0");
}

// ---------------------------------------------------------------------------
// round / floor / ceil / trunc
// ---------------------------------------------------------------------------

#[test]
fn round_half_up() {
    assert_eq!(eval_num("round", &[2.5]), "3");
}

#[test]
fn round_half_down_negative() {
    assert_eq!(eval_num("round", &[-2.5]), "-3");
}

#[test]
fn floor_positive() {
    assert_eq!(eval_num("floor", &[2.7]), "2");
}

#[test]
fn floor_negative() {
    assert_eq!(eval_num("floor", &[-2.3]), "-3");
}

#[test]
fn ceil_positive() {
    assert_eq!(eval_num("ceil", &[2.3]), "3");
}

#[test]
fn ceil_negative() {
    assert_eq!(eval_num("ceil", &[-2.7]), "-2");
}

#[test]
fn trunc_positive() {
    assert_eq!(eval_num("trunc", &[2.9]), "2");
}

#[test]
fn trunc_negative() {
    assert_eq!(eval_num("trunc", &[-2.9]), "-2");
}

// ---------------------------------------------------------------------------
// re / im / arg / conj
// ---------------------------------------------------------------------------

#[test]
fn re_real() {
    assert_eq!(eval_num("re", &[5.0]), "5");
}

#[test]
fn re_complex() {
    assert_eq!(eval_complex("re", 3.0, 4.0), "3");
}

#[test]
fn im_real() {
    assert_eq!(eval_num("im", &[5.0]), "0");
}

#[test]
fn im_complex() {
    assert_eq!(eval_complex("im", 3.0, 4.0), "4");
}

#[test]
fn arg_positive() {
    assert_eq!(eval_num("arg", &[1.0]), "0");
}

#[test]
fn arg_negative() {
    // arg(-1) = π
    let result = eval_num("arg", &[-1.0]);
    let val: f64 = result.parse().unwrap();
    assert!((val - std::f64::consts::PI).abs() < 1e-6, "expected π, got {val}");
}

#[test]
fn conj_real() {
    assert_eq!(eval_num("conj", &[5.0]), "5");
}

#[test]
fn conj_complex() {
    // conj(3+4i) = 3-4i
    let result = eval_complex("conj", 3.0, 4.0);
    assert!(result.contains("- 4i"), "expected negative imaginary, got {result}");
}

// ---------------------------------------------------------------------------
// gamma
// ---------------------------------------------------------------------------

#[test]
fn gamma_five() {
    // Γ(5) = 4! = 24
    let result = eval_num("gamma", &[5.0]);
    let val: f64 = result.parse().unwrap();
    assert!((val - 24.0).abs() < 1e-6, "expected 24, got {val}");
}

#[test]
fn gamma_half() {
    // Γ(0.5) = √π ≈ 1.7724538509
    let result = eval_num("gamma", &[0.5]);
    let val: f64 = result.parse().unwrap();
    assert!(
        (val - std::f64::consts::PI.sqrt()).abs() < 1e-6,
        "expected √π, got {val}"
    );
}

#[test]
fn gamma_pole() {
    // Γ(0) is a pole → NaN
    let result = eval_num("gamma", &[0.0]);
    assert!(result.to_lowercase().contains("nan"), "expected NaN, got {result}");
}

#[test]
fn gamma_negative_integer_pole() {
    // Γ(-1) is a pole → NaN
    let result = eval_num("gamma", &[-1.0]);
    assert!(result.to_lowercase().contains("nan"), "expected NaN, got {result}");
}

// ---------------------------------------------------------------------------
// erf
// ---------------------------------------------------------------------------

#[test]
fn erf_zero() {
    assert_eq!(eval_num("erf", &[0.0]), "0");
}

#[test]
fn erf_large() {
    // erf(5) ≈ 1
    let result = eval_num("erf", &[5.0]);
    let val: f64 = result.parse().unwrap();
    assert!((val - 1.0).abs() < 1e-6, "expected ~1, got {val}");
}

// ---------------------------------------------------------------------------
// zeta
// ---------------------------------------------------------------------------

#[test]
fn zeta_two() {
    // ζ(2) = π²/6 ≈ 1.6449340669
    let result = eval_num("zeta", &[2.0]);
    let val: f64 = result.parse().unwrap();
    let expected = std::f64::consts::PI.powi(2) / 6.0;
    assert!((val - expected).abs() < 1e-6, "expected {expected}, got {val}");
}

// ---------------------------------------------------------------------------
// gammainc
// ---------------------------------------------------------------------------

#[test]
fn gammainc_basic() {
    // Γ(1, 0) = Γ(1) = 1
    let result = eval_num("gammainc", &[1.0, 0.0]);
    let val: f64 = result.parse().unwrap();
    assert!((val - 1.0).abs() < 1e-6, "expected 1, got {val}");
}

#[test]
fn gammainc_limits() {
    // Γ(1, ∞) = 0 (approximated with large x)
    let result = eval_num("gammainc", &[1.0, 100.0]);
    let val: f64 = result.parse().unwrap();
    assert!(val.abs() < 1e-10, "expected ~0, got {val}");
}

// ---------------------------------------------------------------------------
// Arity validation
// ---------------------------------------------------------------------------

#[test]
fn arity_too_few() {
    let func = lookup("abs").unwrap();
    let mut ctx = CalculatorContext::new();
    let result = func.evaluate(&[], &mut ctx);
    assert!(result.is_err(), "abs with 0 args should error");
}

#[test]
fn arity_too_many() {
    let func = lookup("abs").unwrap();
    let args = vec![
        crate::ast::Expression::Number(crate::number::Number::from_f64(1.0)),
        crate::ast::Expression::Number(crate::number::Number::from_f64(2.0)),
    ];
    let mut ctx = CalculatorContext::new();
    let result = func.evaluate(&args, &mut ctx);
    assert!(result.is_err(), "abs with 2 args should error");
}

#[test]
fn gammainc_arity() {
    let func = lookup("gammainc").unwrap();
    let args = vec![crate::ast::Expression::Number(crate::number::Number::from_f64(1.0))];
    let mut ctx = CalculatorContext::new();
    let result = func.evaluate(&args, &mut ctx);
    assert!(result.is_err(), "gammainc with 1 arg should error");
}

#[test]
fn sgn_complex() {
    assert_eq!(eval_complex("sgn", 3.0, 4.0), "0.6 + 0.8i");
}

#[test]
fn sgn_complex_zero() {
    assert_eq!(eval_complex("sgn", 0.0, 0.0), "0");
}

fn eval_interval(name: &str, lower: f64, upper: f64) -> String {
    let func = lookup(name).unwrap_or_else(|| panic!("no function '{name}'"));
    let num = crate::number::Number::new_interval(
        crate::number::Float::from_f64(lower, 53),
        crate::number::Float::from_f64(upper, 53),
    );
    let expr_args = vec![crate::ast::Expression::Number(num)];
    let mut ctx = CalculatorContext::new();
    match func.evaluate(&expr_args, &mut ctx) {
        Ok(crate::ast::Expression::Number(n)) => n.to_qalc_string(),
        Ok(other) => format!("{other:?}"),
        Err(e) => format!("ERR: {e}"),
    }
}

#[test]
fn sgn_interval_spans_zero() {
    // Spanning zero -> NaN
    assert!(eval_interval("sgn", -1.0, 1.0).to_lowercase().contains("nan"));
}

#[test]
fn sgn_interval_strictly_positive() {
    // Strictly positive (lower > 0) -> 1
    assert_eq!(eval_interval("sgn", 1.0, 3.0), "1");
}

#[test]
fn sgn_interval_strictly_negative() {
    // Strictly negative (upper < 0) -> -1
    assert_eq!(eval_interval("sgn", -3.0, -1.0), "-1");
}

#[test]
fn sgn_interval_touches_zero_positive() {
    // Touching zero at lower bound [0, 2] -> NaN
    assert!(eval_interval("sgn", 0.0, 2.0).to_lowercase().contains("nan"));
}

#[test]
fn sgn_interval_touches_zero_negative() {
    // Touching zero at upper bound [-2, 0] -> NaN
    assert!(eval_interval("sgn", -2.0, 0.0).to_lowercase().contains("nan"));
}

#[test]
fn gamma_interval_spans_min() {
    // Spanning minimum at 1.46... (e.g. [1.0, 2.0]) -> [min_val, max_val] where min_val ~ 0.8856, max_val = 1
    let res = eval_interval("gamma", 1.0, 2.0);
    assert!(res.contains("8.856") && res.contains("1"), "got: {}", res);
}

#[test]
fn gamma_interval_pole_spanning_zero() {
    // [-0.5, 0.5] spans pole at 0 -> NaN
    assert!(eval_interval("gamma", -0.5, 0.5).to_lowercase().contains("nan"));
}

#[test]
fn gamma_interval_pole_at_bound() {
    // [-1.0, -0.5] has pole at -1.0 -> NaN
    assert!(eval_interval("gamma", -1.0, -0.5).to_lowercase().contains("nan"));
}

#[test]
fn zeta_interval_spans_pole() {
    // [0.5, 1.5] spans pole at 1 -> NaN
    assert!(eval_interval("zeta", 0.5, 1.5).to_lowercase().contains("nan"));
}

#[test]
fn zeta_interval_monotonic() {
    // [2.0, 3.0] -> [zeta(3), zeta(2)] -> [1.202..., 1.644...]
    let res = eval_interval("zeta", 2.0, 3.0);
    assert!(res.contains("1.202") && res.contains("1.644"), "got: {}", res);
}

#[test]
fn zeta_interval_oscillatory() {
    // [-3.0, -2.5] (x < -2) -> NaN
    assert!(eval_interval("zeta", -3.0, -2.5).to_lowercase().contains("nan"));
}

