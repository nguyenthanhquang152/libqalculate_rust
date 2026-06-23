use libqalculate_rust::ast::Expression;
use libqalculate_rust::context::CalculatorContext;
use libqalculate_rust::eval::evaluate_ast;

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

/// Assert that a native evaluation produces a specific numeric string.
fn assert_eval(input: &str, expected: &str) {
    let result = eval_native(input).unwrap_or_else(|e| panic!("eval failed for '{input}': {e}"));
    let formatted = match &result {
        Expression::Number(n) => format!("{n}"),
        other => panic!("Expected Number for '{input}', got: {other:?}"),
    };
    assert_eq!(formatted, expected, "Failed for input: '{input}'");
}

// ============================================================
// Group 1: Simple percent literals (cases 1-6)
// ============================================================
#[test]
fn test_percent_literal_zero() {
    assert_eval("0%", "0");
}

#[test]
fn test_percent_literal_one() {
    assert_eval("1%", "0.01");
}

#[test]
fn test_percent_literal_fractional() {
    assert_eval(".000123 %", "0.00000123");
}

#[test]
fn test_percent_literal_negative() {
    assert_eval("-15%", "-0.15");
}

#[test]
fn test_percent_literal_large() {
    assert_eval("1234%", "12.34");
}

#[test]
fn test_percent_literal_scientific() {
    assert_eval("1e-3%", "0.00001");
}

// ============================================================
// Group 2: Percent-only arithmetic (cases 7-14)
// ============================================================
#[test]
fn test_percent_add_percent() {
    assert_eval("10% + 5%", "0.15");
}

#[test]
fn test_percent_sub_percent() {
    assert_eval("10%-6%", "0.04");
}

#[test]
fn test_percent_multi_add_sub() {
    assert_eval("123% - 3% + 10%", "1.3");
}

#[test]
fn test_percent_sub_equal() {
    assert_eval("10% - 10%", "0");
}

#[test]
fn test_percent_sub_larger() {
    assert_eval("10% - 20%", "-0.1");
}

#[test]
fn test_percent_mul_percent() {
    assert_eval("10% * 2%", "0.002");
}

#[test]
fn test_percent_div_percent() {
    assert_eval("10% / 2%", "5");
}

#[test]
fn test_percent_mixed_mul_div() {
    assert_eval("10%*20%-30%/15%", "-1.98");
}

// ============================================================
// Group 3: Relative percent (cases 15-20)
// ============================================================
#[test]
fn test_relative_percent_add() {
    // 100 + 10% → 110
    assert_eval("100 + 10%", "110");
}

#[test]
fn test_relative_percent_add_chain() {
    // 100 + 10% + 10% → 121
    assert_eval("100 + 10% + 10%", "121");
}

#[test]
fn test_relative_percent_add_paren() {
    // 100 + (10 + 10)% → 120
    assert_eval("100 + (10 + 10)%", "120");
}

#[test]
fn test_relative_percent_sub() {
    // 100 - 10% → 90
    assert_eval("100 - 10%", "90");
}

#[test]
fn test_relative_percent_sub_chain() {
    // 100 - 10% - 10% → 81
    assert_eval("100 - 10% - 10%", "81");
}

#[test]
fn test_relative_percent_sub_paren() {
    // 100 - (10-5)% → 95
    assert_eval("100 - (10-5) %", "95");
}

// ============================================================
// Group 4: Percent on left, number on right (cases 21-22)
// ============================================================
#[test]
fn test_percent_left_add() {
    // 10% + 100 → 100.1
    assert_eval("10% + 100", "100.1");
}

#[test]
fn test_percent_left_sub() {
    // 10% - 100 → -99.9
    assert_eval("10% - 100", "-99.9");
}

// ============================================================
// Group 5: Multiplication/Division with percent (cases 23-24)
// ============================================================
#[test]
fn test_number_mul_percent() {
    // 100 * 10% → 10
    assert_eval("100 * 10%", "10");
}

#[test]
fn test_number_div_percent() {
    // 100 / 10% → 1000
    assert_eval("100 / 10%", "1000");
}
