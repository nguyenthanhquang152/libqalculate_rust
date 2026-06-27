use libqalculate_rust::context::CalculatorContext;
use libqalculate_rust::eval::evaluate_ast;
use libqalculate_rust::parser::operators::parse_expression;

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

fn assert_evaluates_to(input: &str, expected: &str) {
    let mut context = CalculatorContext::default();
    let expr = parse_expression(input).unwrap();
    let evaluated = evaluate_ast(&expr, &mut context).unwrap();
    let simplified = libqalculate_rust::simplify::simplify_ast(&evaluated, &mut context);
    let expected_expr = parse_expression(expected).unwrap();
    let expected_evaluated = evaluate_ast(&expected_expr, &mut context).unwrap();
    let expected_simplified =
        libqalculate_rust::simplify::simplify_ast(&expected_evaluated, &mut context);

    assert_eq!(
        format!("{:?}", simplified),
        format!("{:?}", expected_simplified),
        "Evaluating '{}' failed to match expected '{}'",
        input,
        expected
    );
}

#[test]
fn test_polynomial_degree() {
    let _guard = EnvGuard::set("QALCULATE_DISABLE_FALLBACK", "1");
    assert_evaluates_to("degree(2*x^2 + x, x)", "2");
    assert_evaluates_to("degree(5, x)", "0");
    assert_evaluates_to("degree(x, x)", "1");
    assert_evaluates_to("degree(x^3, x)", "3");
    assert_evaluates_to("ldegree(2*x^2 + 3*x, x)", "1");
    assert_evaluates_to("degree(x^2 - x^2, x)", "0");
}

#[test]
fn test_polynomial_coeff() {
    let _guard = EnvGuard::set("QALCULATE_DISABLE_FALLBACK", "1");
    assert_evaluates_to("coeff(2*x^2 + 5*x + 3, 2, x)", "2");
    assert_evaluates_to("coeff(2*x^2 + 5*x + 3, 1, x)", "5");
    assert_evaluates_to("coeff(2*x^2 + 5*x + 3, 0, x)", "3");
    assert_evaluates_to("lcoeff(2*x^2 + 5*x + 3, x)", "2");
    assert_evaluates_to("tcoeff(2*x^2 + 5*x + 3, x)", "3");
    assert_evaluates_to("coeff(-(3)*x, 1, x)", "-3");
    assert_evaluates_to("tcoeff(-5x^2 + 3x - x)", "2");
}

#[test]
fn test_polynomial_content_and_primpart() {
    let _guard = EnvGuard::set("QALCULATE_DISABLE_FALLBACK", "1");
    assert_evaluates_to("pcontent(2*x + 4, x)", "2");
    assert_evaluates_to("primpart(2*x + 4, x)", "x + 2");
    assert_evaluates_to("punit(2*x + 4, x)", "1");
    assert_evaluates_to("punit(-2*x + 4, x)", "-1");
}

#[test]
fn test_factor() {
    let _guard = EnvGuard::set("QALCULATE_DISABLE_FALLBACK", "1");
    assert_evaluates_to("factor(12)", "[2, 2, 3]");
    assert_evaluates_to("factor(-12)", "[-1, 2, 2, 3]");
}
