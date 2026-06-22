use libqalculate_rust::context::CalculatorContext;
use libqalculate_rust::messages::{MessageStage, MessageType};

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

#[test]
fn test_native_arithmetic_evaluation() {
    let _guard = EnvGuard::set("QALCULATE_DISABLE_FALLBACK", "1");
    let mut context = CalculatorContext::default();

    // 1 + 2
    let res = context.parse_and_evaluate_with_context("1 + 2").unwrap();
    assert_eq!(res.to_string(), "3");

    // 5 - -2
    let res = context.parse_and_evaluate_with_context("5 - -2").unwrap();
    assert_eq!(res.to_string(), "7");

    // 2 * 3
    let res = context.parse_and_evaluate_with_context("2 * 3").unwrap();
    assert_eq!(res.to_string(), "6");

    // 6 / 2
    let res = context.parse_and_evaluate_with_context("6 / 2").unwrap();
    assert_eq!(res.to_string(), "3");

    // 5 ^ 2
    let res = context.parse_and_evaluate_with_context("5 ^ 2").unwrap();
    assert_eq!(res.to_string(), "25");

    // 1 / 3 (exact rational)
    let res = context.parse_and_evaluate_with_context("1 / 3").unwrap();
    assert_eq!(res.to_string(), "1/3");
}

#[test]
fn test_native_variable_evaluation() {
    let _guard = EnvGuard::set("QALCULATE_DISABLE_FALLBACK", "1");
    let mut context = CalculatorContext::default();

    // alpha := 5
    let res = context
        .parse_and_evaluate_with_context("alpha := 5")
        .unwrap();
    assert_eq!(res.to_string(), "5");
    assert!(context.variables.contains_key("alpha"));

    // beta := 2 + 1
    let res = context
        .parse_and_evaluate_with_context("beta := 2 + 1")
        .unwrap();
    assert_eq!(res.to_string(), "3");
    assert!(context.variables.contains_key("beta"));

    // alpha + beta
    let res = context
        .parse_and_evaluate_with_context("alpha + beta")
        .unwrap();
    assert_eq!(res.to_string(), "8");

    // Reassignment: alpha := 10
    let res = context
        .parse_and_evaluate_with_context("alpha := 10")
        .unwrap();
    assert_eq!(res.to_string(), "10");
    assert_eq!(
        context
            .parse_and_evaluate_with_context("alpha + beta")
            .unwrap()
            .to_string(),
        "13"
    );
}

#[test]
fn test_native_comparison_and_logical_operators() {
    let _guard = EnvGuard::set("QALCULATE_DISABLE_FALLBACK", "1");
    let mut context = CalculatorContext::default();

    // 1 = 1 -> 1
    let res = context.parse_and_evaluate_with_context("1 = 1").unwrap();
    assert_eq!(res.to_string(), "1");

    // 2 > 3 -> 0
    let res = context.parse_and_evaluate_with_context("2 > 3").unwrap();
    assert_eq!(res.to_string(), "0");

    // 1 && 0 -> 0
    let res = context.parse_and_evaluate_with_context("1 && 0").unwrap();
    assert_eq!(res.to_string(), "0");

    // 1 || 0 -> 1
    let res = context.parse_and_evaluate_with_context("1 || 0").unwrap();
    assert_eq!(res.to_string(), "1");

    // !1 -> 0
    let res = context.parse_and_evaluate_with_context("!1").unwrap();
    assert_eq!(res.to_string(), "0");

    // !0 -> 1
    let res = context.parse_and_evaluate_with_context("!0").unwrap();
    assert_eq!(res.to_string(), "1");
}

#[test]
fn test_postfix_percentage() {
    let _guard = EnvGuard::set("QALCULATE_DISABLE_FALLBACK", "1");
    let mut context = CalculatorContext::default();

    // 5 %
    let res = context.parse_and_evaluate_with_context("5 %").unwrap();
    assert_eq!(res.to_string(), "0.05"); // 5/100 = 1/20 = 0.05
}

#[test]
fn test_simple_unitless_functions() {
    let _guard = EnvGuard::set("QALCULATE_DISABLE_FALLBACK", "1");
    let mut context = CalculatorContext::default();

    // abs(-5)
    let res = context.parse_and_evaluate_with_context("abs(-5)").unwrap();
    assert_eq!(res.to_string(), "5");

    // sqrt(9)
    let res = context.parse_and_evaluate_with_context("sqrt(9)").unwrap();
    assert_eq!(res.to_string(), "3");

    // ln(1)
    let res = context.parse_and_evaluate_with_context("ln(1)").unwrap();
    assert_eq!(res.to_string(), "0");
}

#[test]
fn test_symbolic_and_missing_variables() {
    let _guard = EnvGuard::set("QALCULATE_DISABLE_FALLBACK", "1");
    let mut context = CalculatorContext::default();

    // Evaluating an unregistered/unknown variable e.g. "x" should return a symbolic error
    let res = context.parse_and_evaluate_with_context("x");
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Symbolic result"));
}

#[test]
fn test_division_by_zero_warnings() {
    let _guard = EnvGuard::set("QALCULATE_DISABLE_FALLBACK", "1");
    let mut context = CalculatorContext::default();

    // 1 / 0
    context.clear_messages();
    let res = context.parse_and_evaluate_with_context("1 / 0").unwrap();
    assert!(res.is_nan());

    let messages = context.messages.get_messages();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].message_type(), MessageType::Warning);
    assert_eq!(messages[0].stage(), MessageStage::Calculation);
    assert!(messages[0].message().contains("Division by zero"));

    // 1 % 0 (remainder)
    context.clear_messages();
    let res = context.parse_and_evaluate_with_context("1 % 0").unwrap();
    assert!(res.is_nan());
    assert!(context.messages.get_messages()[0]
        .message()
        .contains("Division by zero"));

    // 1 mod 0
    context.clear_messages();
    let res = context.parse_and_evaluate_with_context("1 mod 0").unwrap();
    assert!(res.is_nan());
    assert!(context.messages.get_messages()[0]
        .message()
        .contains("Division by zero"));
}
