//! Integration tests for operator parity with upstream `operators.batch`.
//!
//! Tests parse → evaluate → format for all operator cases from the upstream
//! Qalculate `operators.batch` fixture.

use libqalculate_rust::context::CalculatorContext;

struct EnvGuard {
    name: &'static str,
    old_value: Option<String>,
}

impl EnvGuard {
    fn set(name: &'static str, value: &str) -> Self {
        let old_value = std::env::var(name).ok();
        #[allow(unused_unsafe)]
        unsafe {
            std::env::set_var(name, value);
        }
        Self { name, old_value }
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

/// Helper: parse, evaluate, and format the result as a string.
fn eval(input: &str) -> String {
    let _guard = EnvGuard::set("QALCULATE_DISABLE_FALLBACK", "1");
    let mut ctx = CalculatorContext::default();
    let result = ctx
        .parse_and_evaluate_with_context(input)
        .unwrap_or_else(|e| panic!("Failed to evaluate '{}': {}", input, e));
    result.to_string()
}

// ─── Addition ────────────────────────────────────────────────────────

#[test]
fn addition_basic() {
    assert_eq!(eval("1 + 2"), "3");
}

#[test]
fn addition_word_plus() {
    assert_eq!(eval("1 plus 2"), "3");
}

// ─── Subtraction ─────────────────────────────────────────────────────

#[test]
fn subtraction_unicode_minus() {
    // U+2212 MINUS SIGN
    assert_eq!(eval("5\u{2212}2"), "3");
}

#[test]
fn subtraction_word_minus() {
    assert_eq!(eval("5 minus 3"), "2");
}

#[test]
fn subtraction_double_negative() {
    // 5--2 = 5 - (-2) = 7
    assert_eq!(eval("5--2"), "7");
}

#[test]
fn subtraction_triple_negative() {
    // 5---2 = 5 - (--2) = 5 - 2 = 3
    assert_eq!(eval("5---2"), "3");
}

#[test]
fn subtraction_negative_lhs() {
    assert_eq!(eval("-5-2"), "-7");
}

#[test]
fn subtraction_word_minus_minus() {
    // 5 minus minus 3 = 5 - (-3) = 8
    assert_eq!(eval("5 minus minus 3"), "8");
}

#[test]
fn subtraction_word_minus_sign() {
    // 5 minus - 3 = 5 - (-3) = 8
    assert_eq!(eval("5 minus - 3"), "8");
}

// ─── Multiplication ──────────────────────────────────────────────────

#[test]
fn multiplication_asterisk() {
    assert_eq!(eval("2*3"), "6");
}

#[test]
fn multiplication_unicode_times() {
    // U+00D7 MULTIPLICATION SIGN
    assert_eq!(eval("3 \u{00D7} 4"), "12");
}

#[test]
fn multiplication_word_times() {
    assert_eq!(eval("4 times 5"), "20");
}

#[test]
fn multiplication_unicode_dot() {
    // U+22C5 DOT OPERATOR
    assert_eq!(eval("5 \u{22C5} 6"), "30");
}

// ─── Division ────────────────────────────────────────────────────────

#[test]
fn division_slash() {
    assert_eq!(eval("6/2"), "3");
}

#[test]
fn division_word_per() {
    assert_eq!(eval("12 per 3"), "4");
}

#[test]
fn division_fraction() {
    assert_eq!(eval("1/2"), "0.5");
}

// ─── Remainder ───────────────────────────────────────────────────────

#[test]
fn remainder_percent() {
    assert_eq!(eval("6%2"), "0");
}

#[test]
fn remainder_word_rem() {
    assert_eq!(eval("7 rem 2"), "1");
}

#[test]
fn remainder_negative_lhs() {
    assert_eq!(eval("-8%3"), "-2");
}

// ─── Modulo ──────────────────────────────────────────────────────────

#[test]
fn modulo_double_percent() {
    assert_eq!(eval("3 %% 2"), "1");
}

#[test]
fn modulo_negative_rhs() {
    assert_eq!(eval("3 %% -2"), "-1");
}

#[test]
fn modulo_word_mod() {
    assert_eq!(eval("3 mod -2"), "-1");
}

// ─── Integer Division ────────────────────────────────────────────────

#[test]
fn integer_division_double_slash() {
    assert_eq!(eval("5//2"), "2");
}

#[test]
fn integer_division_backslash() {
    assert_eq!(eval("5\\2"), "2");
}

#[test]
fn integer_division_word_div() {
    assert_eq!(eval("5 div 2"), "2");
}

// ─── Exponentiation ──────────────────────────────────────────────────

#[test]
fn power_caret() {
    assert_eq!(eval("5 ^ 2"), "25");
}

#[test]
fn power_double_star() {
    assert_eq!(eval("5 ** 3"), "125");
}

#[test]
fn power_right_associative() {
    // 4 ** 3 ** 2 = 4 ** (3 ** 2) = 4 ** 9 = 262144
    assert_eq!(eval("4 ** 3 ** 2"), "262144");
}

// ─── Factorial ───────────────────────────────────────────────────────

#[test]
fn factorial_one() {
    assert_eq!(eval("1!"), "1");
}

#[test]
fn factorial_five() {
    assert_eq!(eval("5!"), "120");
}

#[test]
fn factorial_zero() {
    assert_eq!(eval("0!"), "1");
}

#[test]
fn factorial_ten() {
    assert_eq!(eval("10!"), "3628800");
}

#[test]
fn double_factorial_six() {
    // 6!! = 6 * 4 * 2 = 48
    assert_eq!(eval("6!!"), "48");
}

#[test]
fn double_factorial_seven() {
    // 7!! = 7 * 5 * 3 * 1 = 105
    assert_eq!(eval("7!!"), "105");
}
