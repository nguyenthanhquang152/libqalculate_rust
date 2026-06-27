use libqalculate_rust::context::CalculatorContext;
use libqalculate_rust::parser::operators::parse_expression;
use libqalculate_rust::simplify::simplify_ast;

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

fn assert_simplifies_to(input: &str, expected: &str) {
    let mut context = CalculatorContext::default();
    let expr = parse_expression(input).unwrap();
    let simplified = simplify_ast(&expr, &mut context);
    let expected_expr = parse_expression(expected).unwrap();
    let expected_simplified = simplify_ast(&expected_expr, &mut context);

    assert_eq!(
        format!("{:?}", simplified),
        format!("{:?}", expected_simplified),
        "Simplifying '{}' failed to match expected '{}'",
        input,
        expected
    );
}

#[test]
fn test_identity_and_folding() {
    let _guard = EnvGuard::set("QALCULATE_DISABLE_FALLBACK", "1");

    // Identity operations
    assert_simplifies_to("x + 0", "x");
    assert_simplifies_to("x * 1", "x");
    assert_simplifies_to("x * 0", "0");
    assert_simplifies_to("0 * x", "0");
    assert_simplifies_to("1 * x", "x");
    assert_simplifies_to("0 + x", "x");
}

#[test]
fn test_nested_negation() {
    let _guard = EnvGuard::set("QALCULATE_DISABLE_FALLBACK", "1");
    assert_simplifies_to("--x", "x");
    assert_simplifies_to("---x", "-x");
}

#[test]
fn test_absolute_value_simplification() {
    let _guard = EnvGuard::set("QALCULATE_DISABLE_FALLBACK", "1");
    // abs(x - y) - abs(y - x) -> 0
    assert_simplifies_to("abs(x - y) - abs(y - x)", "0");

    // abs(x - abs(x)) - (abs(x) - x) -> 0
    assert_simplifies_to("abs(x - abs(x)) - (abs(x) - x)", "0");
}

#[test]
fn test_division_to_signum() {
    let _guard = EnvGuard::set("QALCULATE_DISABLE_FALLBACK", "1");
    // x / abs(x) -> sgn(x)
    assert_simplifies_to("x / abs(x)", "sgn(x)");

    // x / abs(x) - sgn(x) -> 0
    assert_simplifies_to("x / abs(x) - sgn(x)", "0");
}
