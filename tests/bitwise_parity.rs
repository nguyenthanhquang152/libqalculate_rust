use libqalculate_rust::ast::Expression;
use libqalculate_rust::context::CalculatorContext;
use libqalculate_rust::eval::evaluate_ast;
use libqalculate_rust::number::Number;

use std::sync::{Mutex, MutexGuard, OnceLock};

static ENV_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

struct EnvGuard {
    name: &'static str,
    old_value: Option<String>,
    _lock_guard: MutexGuard<'static, ()>,
}

impl EnvGuard {
    fn set(name: &'static str, value: &str) -> Self {
        let lock_guard = ENV_MUTEX.get_or_init(|| Mutex::new(())).lock().unwrap();
        let old_value = std::env::var(name).ok();
        #[allow(unused_unsafe)]
        unsafe {
            std::env::set_var(name, value);
        }
        Self { name, old_value, _lock_guard: lock_guard }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        #[allow(unused_unsafe)]
        unsafe {
            match &self.old_value {
                Some(val) => std::env::set_var(self.name, val),
                None => std::env::remove_var(self.name),
            }
        }
    }
}

/// Evaluate an expression natively (fallback disabled) with default context.
fn eval_native(input: &str) -> Result<Expression, String> {
    let _guard = EnvGuard::set("QALCULATE_DISABLE_FALLBACK", "1");
    let mut context = CalculatorContext::default();
    let expr = libqalculate_rust::parser::operators::parse_expression(input)
        .map_err(|e| e.to_string())?;
    evaluate_ast(&expr, &mut context)
}

#[test]
fn test_bitwise_not() {
    // ~0 -> -1
    assert_eq!(eval_native("~0").unwrap(), Expression::Number(Number::from_i32(-1)));

    // ~-1 -> 0
    assert_eq!(eval_native("~-1").unwrap(), Expression::Number(Number::from_i32(0)));

    // ~-812 -> 811
    assert_eq!(eval_native("~-812").unwrap(), Expression::Number(Number::from_i32(811)));

    // ¬1 -> -2
    assert_eq!(eval_native("¬1").unwrap(), Expression::Number(Number::from_i32(-2)));
}

#[test]
fn test_bitwise_shifts_basic() {
    // 0 >> 0 -> 0
    assert_eq!(eval_native("0 >> 0").unwrap(), Expression::Number(Number::from_i32(0)));

    // 0 >> 1 -> 0
    assert_eq!(eval_native("0 >> 1").unwrap(), Expression::Number(Number::from_i32(0)));

    // 18 >> 2 -> 4
    assert_eq!(eval_native("18 >> 2").unwrap(), Expression::Number(Number::from_i32(4)));

    // 11 >> 0 -> 11
    assert_eq!(eval_native("11 >> 0").unwrap(), Expression::Number(Number::from_i32(11)));

    // -11 >> 0 -> -11
    assert_eq!(eval_native("-11 >> 0").unwrap(), Expression::Number(Number::from_i32(-11)));

    // -18 >> 1 -> -9
    assert_eq!(eval_native("-18 >> 1").unwrap(), Expression::Number(Number::from_i32(-9)));

    // 0 << 0 -> 0
    assert_eq!(eval_native("0 << 0").unwrap(), Expression::Number(Number::from_i32(0)));

    // 0 << 1 -> 0
    assert_eq!(eval_native("0 << 1").unwrap(), Expression::Number(Number::from_i32(0)));

    // 18 << 0 -> 18
    assert_eq!(eval_native("18 << 0").unwrap(), Expression::Number(Number::from_i32(18)));

    // 18 << 1 -> 36
    assert_eq!(eval_native("18 << 1").unwrap(), Expression::Number(Number::from_i32(36)));

    // -18 << 2 -> -72
    assert_eq!(eval_native("-18 << 2").unwrap(), Expression::Number(Number::from_i32(-72)));
}

#[test]
fn test_bitwise_shifts_negative_amount() {
    // 5 << -2 -> 1
    assert_eq!(eval_native("5 << -2").unwrap(), Expression::Number(Number::from_i32(1)));

    // 5 >> -2 -> 20
    assert_eq!(eval_native("5 >> -2").unwrap(), Expression::Number(Number::from_i32(20)));

    // -18 << -1 -> -9
    assert_eq!(eval_native("-18 << -1").unwrap(), Expression::Number(Number::from_i32(-9)));

    // -18 >> -1 -> -36
    assert_eq!(eval_native("-18 >> -1").unwrap(), Expression::Number(Number::from_i32(-36)));
}

#[test]
fn test_shift_builtin() {
    // shift(5, 2) -> 20
    assert_eq!(eval_native("shift(5, 2)").unwrap(), Expression::Number(Number::from_i32(20)));

    // shift(5, -2) -> 1
    assert_eq!(eval_native("shift(5, -2)").unwrap(), Expression::Number(Number::from_i32(1)));

    // shift(x, 2) -> unevaluated
    let res = eval_native("shift(x, 2)").unwrap();
    assert!(matches!(res, Expression::FunctionCall { .. }));
}

#[test]
fn test_bitwise_and_or_xor() {
    // 7 & 3 -> 3
    assert_eq!(eval_native("7 & 3").unwrap(), Expression::Number(Number::from_i32(3)));

    // 5 | 9 -> 13
    assert_eq!(eval_native("5 | 9").unwrap(), Expression::Number(Number::from_i32(13)));

    // 5 xor 9 -> 12
    assert_eq!(eval_native("5 xor 9").unwrap(), Expression::Number(Number::from_i32(12)));

    // 5 ⊻ 9 -> 12
    assert_eq!(eval_native("5 ⊻ 9").unwrap(), Expression::Number(Number::from_i32(12)));

    // 5 ^^ 9 -> 12
    assert_eq!(eval_native("5 ^^ 9").unwrap(), Expression::Number(Number::from_i32(12)));
}

#[test]
fn test_bitwise_literal_spaces() {
    // 0b1011 0010 -> 178
    assert_eq!(eval_native("0b1011 0010").unwrap(), Expression::Number(Number::from_i32(178)));

    // 0b0111 0001 -> 113
    assert_eq!(eval_native("0b0111 0001").unwrap(), Expression::Number(Number::from_i32(113)));
}

#[test]
fn test_bitwise_precedence() {
    // 5 | 9 xor 5 & 7 -> 13 (which is 5 | (9 xor (5 & 7)))
    assert_eq!(eval_native("5 | 9 xor 5 & 7").unwrap(), Expression::Number(Number::from_i32(13)));

    // 5 & 7 xor 9 | 5 -> 13
    assert_eq!(eval_native("5 & 7 xor 9 | 5").unwrap(), Expression::Number(Number::from_i32(13)));
}

#[test]
fn test_bitwise_non_integer_unevaluated() {
    // 0.5 & 3 -> unevaluated
    let res = eval_native("0.5 & 3").unwrap();
    assert!(matches!(res, Expression::BitwiseAnd { .. }));

    // 5.2 >> 1 -> unevaluated
    let res = eval_native("5.2 >> 1").unwrap();
    assert!(matches!(res, Expression::ShiftRight { .. }));
}
