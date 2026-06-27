use libqalculate_rust::ast::{Expression, FunctionRef};
use libqalculate_rust::context::CalculatorContext;
use libqalculate_rust::eval::evaluate_ast;
use libqalculate_rust::number::Number;

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

fn eval(context: &mut CalculatorContext, input: &str) -> Result<Expression, String> {
    let expr =
        libqalculate_rust::parser::operators::parse_expression(input).map_err(|e| e.to_string())?;
    evaluate_ast(&expr, context)
}

#[test]
fn test_logical_xor_builtin() {
    let _guard = EnvGuard::set("QALCULATE_DISABLE_FALLBACK", "1");
    let mut context = CalculatorContext::default();

    // lxor(1, 0) -> 1
    assert_eq!(
        eval(&mut context, "lxor(1, 0)").unwrap(),
        Expression::Number(Number::from_i32(1))
    );

    // lxor(1, 1) -> 0
    assert_eq!(
        eval(&mut context, "lxor(1, 1)").unwrap(),
        Expression::Number(Number::from_i32(0))
    );

    // lxor(0, 0) -> 0
    assert_eq!(
        eval(&mut context, "lxor(0, 0)").unwrap(),
        Expression::Number(Number::from_i32(0))
    );

    // lxor(x, 1) -> LogicalXor { lhs: x, rhs: 1 } (unevaluated)
    let res = eval(&mut context, "lxor(x, 1)").unwrap();
    assert!(matches!(res, Expression::LogicalXor { .. }));
}

#[test]
fn test_logical_if_builtin() {
    let _guard = EnvGuard::set("QALCULATE_DISABLE_FALLBACK", "1");
    let mut context = CalculatorContext::default();

    // if(1, 5, 6) -> 5
    assert_eq!(
        eval(&mut context, "if(1, 5, 6)").unwrap(),
        Expression::Number(Number::from_i32(5))
    );

    // if(0, 5, 6) -> 6
    assert_eq!(
        eval(&mut context, "if(0, 5, 6)").unwrap(),
        Expression::Number(Number::from_i32(6))
    );

    // if(x, 5, 6) -> if(x, 5, 6) (unevaluated)
    let res = eval(&mut context, "if(x, 5, 6)").unwrap();
    assert!(matches!(res, Expression::FunctionCall { .. }));

    // if(x, 5, 6, 1) -> 6 (assume false)
    assert_eq!(
        eval(&mut context, "if(x, 5, 6, 1)").unwrap(),
        Expression::Number(Number::from_i32(6))
    );

    // if(x, 5, 6, 0) -> if(x, 5, 6, 0)
    let res = eval(&mut context, "if(x, 5, 6, 0)").unwrap();
    assert!(matches!(res, Expression::FunctionCall { .. }));
}

#[test]
fn test_logical_if_nan() {
    let _guard = EnvGuard::set("QALCULATE_DISABLE_FALLBACK", "1");
    let mut context = CalculatorContext::default();

    // 0/0 results in NaN.
    // if(0/0, 5, 6) -> if(NaN, 5, 6) (unevaluated because NaN is unknown)
    let res = eval(&mut context, "if(0/0, 5, 6)").unwrap();
    assert!(matches!(res, Expression::FunctionCall { .. }));

    // if(0/0, 5, 6, 1) -> 6 (since NaN is unknown and assume_false = 1)
    assert_eq!(
        eval(&mut context, "if(0/0, 5, 6, 1)").unwrap(),
        Expression::Number(Number::from_i32(6))
    );
}

#[test]
fn test_logical_if_vectors() {
    let _guard = EnvGuard::set("QALCULATE_DISABLE_FALLBACK", "1");
    let mut context = CalculatorContext::default();

    // Helper to construct: if(cond, then_val, else_val) or if(cond, then_val, else_val, assume_false)
    let make_if = |cond: Expression,
                   then_val: Expression,
                   else_val: Expression,
                   assume_false: Option<Expression>| {
        let mut args = vec![cond, then_val, else_val];
        if let Some(af) = assume_false {
            args.push(af);
        }
        Expression::FunctionCall {
            function: FunctionRef::new("if"),
            args,
        }
    };

    let n = |val| Expression::Number(Number::from_i32(val));
    let x = || Expression::Symbolic(libqalculate_rust::ast::Symbol::new("x"));

    // if([1, 0], 5, 6) -> 5
    let cond = Expression::Vector(vec![n(1), n(0)]);
    let expr = make_if(cond, n(5), n(6), None);
    assert_eq!(evaluate_ast(&expr, &mut context).unwrap(), n(5));

    // if([0, 1], [5, 10], 6) -> 10
    let cond = Expression::Vector(vec![n(0), n(1)]);
    let then_val = Expression::Vector(vec![n(5), n(10)]);
    let expr = make_if(cond, then_val, n(6), None);
    assert_eq!(evaluate_ast(&expr, &mut context).unwrap(), n(10));

    // if([0, 0], 5, 6) -> 6
    let cond = Expression::Vector(vec![n(0), n(0)]);
    let expr = make_if(cond, n(5), n(6), None);
    assert_eq!(evaluate_ast(&expr, &mut context).unwrap(), n(6));

    // if([1, 0], [5, 10], [6, 12]) -> 5
    let cond = Expression::Vector(vec![n(1), n(0)]);
    let then_val = Expression::Vector(vec![n(5), n(10)]);
    let else_val = Expression::Vector(vec![n(6), n(12)]);
    let expr = make_if(cond, then_val, else_val, None);
    assert_eq!(evaluate_ast(&expr, &mut context).unwrap(), n(5));

    // if([0, x], [5, 10], 6) -> unevaluated
    let cond = Expression::Vector(vec![n(0), x()]);
    let then_val = Expression::Vector(vec![n(5), n(10)]);
    let expr = make_if(cond, then_val, n(6), None);
    let res = evaluate_ast(&expr, &mut context).unwrap();
    assert!(matches!(res, Expression::FunctionCall { .. }));

    // if([0, x], [5, 10], 6, 1) -> 6
    let cond = Expression::Vector(vec![n(0), x()]);
    let then_val = Expression::Vector(vec![n(5), n(10)]);
    let expr = make_if(cond, then_val, n(6), Some(n(1)));
    assert_eq!(evaluate_ast(&expr, &mut context).unwrap(), n(6));
}

#[test]
fn test_logical_if_uncertainty() {
    let _guard = EnvGuard::set("QALCULATE_DISABLE_FALLBACK", "1");
    let mut context = CalculatorContext::default();

    let make_if =
        |cond: Expression, then_val: Expression, else_val: Expression| Expression::FunctionCall {
            function: FunctionRef::new("if"),
            args: vec![cond, then_val, else_val],
        };

    let n = |val| Expression::Number(Number::from_i32(val));
    let parsed = |s: &str| Expression::Number(s.parse::<Number>().unwrap());

    // if(3 +/- 5, 1, 2) -> if(3 +/- 5, 1, 2) (unevaluated because it crosses zero)
    let expr = make_if(parsed("3 +/- 5"), n(1), n(2));
    let res = evaluate_ast(&expr, &mut context).unwrap();
    assert!(matches!(res, Expression::FunctionCall { .. }));

    // if(5 +/- 2, 1, 2) -> 1 (strictly positive)
    let expr = make_if(parsed("5 +/- 2"), n(1), n(2));
    assert_eq!(evaluate_ast(&expr, &mut context).unwrap(), n(1));

    // if(3 +/- 3, 1, 2) -> if(3 +/- 3, 1, 2) (touches zero)
    let expr = make_if(parsed("3 +/- 3"), n(1), n(2));
    let res = evaluate_ast(&expr, &mut context).unwrap();
    assert!(matches!(res, Expression::FunctionCall { .. }));
}
