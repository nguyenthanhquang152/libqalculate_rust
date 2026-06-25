//! Combinatorics built-in function family.
//!
//! # Upstream oracle
//! - `../libqalculate/libqalculate/BuiltinFunctions-combinatorics.cc`
//! - `../libqalculate/data/functions.xml.in` (factorial, factorial2, multifactorial, binomial, perm, comb, derangements, hyperfactorial, superfactorial)
//! - `../libqalculate/tests/operators.batch`

use crate::ast::Expression;
use crate::context::CalculatorContext;
use crate::functions::{
    make_unevaluated, validate_arity, BuiltinFunction, BuiltinFunctionInfo, FunctionResult,
};
use crate::number::Number;
use rug::ops::Pow;

/// Shared finite-product budget until the evaluator has a cancellable resource policy.
///
/// This keeps exact combinatorics from accepting `u32::MAX`-scale loops while
/// still covering the upstream-compatible cases that exceeded the previous
/// per-function caps.
const MAX_EXACT_COMBINATORICS_STEPS: u32 = 1_000_000;

fn integer_number(value: rug::Integer) -> Number {
    Number::from_rational(crate::number::Rational {
        value: rug::Rational::from(value),
    })
}

fn bounded_exact_steps(value: &rug::Integer) -> Option<u32> {
    let steps = value.to_u32()?;
    (steps <= MAX_EXACT_COMBINATORICS_STEPS).then_some(steps)
}

fn binomial_or_gamma(
    name: &str,
    n: &Number,
    k: &Number,
    context: &CalculatorContext,
) -> Expression {
    if let Some(res) = n.binomial(k) {
        Expression::Number(res)
    } else if context.evaluation_options.approximation == crate::options::ApproximationMode::Exact {
        make_unevaluated(
            name,
            &[Expression::Number(n.clone()), Expression::Number(k.clone())],
        )
    } else {
        // gamma(n+1) / (gamma(n-k+1) * gamma(k+1))
        let one = Number::from_i32(1);
        let n_plus_1 = n.add(&one);
        let n_minus_k_plus_1 = n.sub(k).add(&one);
        let k_plus_1 = k.add(&one);
        let g1 = n_plus_1.gamma();
        let g2 = n_minus_k_plus_1.gamma();
        let g3 = k_plus_1.gamma();
        Expression::Number(g1.div(&g2.mul(&g3)))
    }
}

fn evaluate_binomial_like(
    name: &str,
    lhs: &Expression,
    rhs: &Expression,
    context: &mut CalculatorContext,
) -> FunctionResult {
    match (lhs, rhs) {
        (Expression::Vector(left), Expression::Vector(right)) => {
            if left.len() == right.len() {
                let mut out = Vec::with_capacity(left.len());
                for (l, r) in left.iter().zip(right) {
                    out.push(evaluate_binomial_like(name, l, r, context)?);
                }
                Ok(Expression::Vector(out))
            } else if left.len() == 1 {
                let mut out = Vec::with_capacity(right.len());
                for r in right {
                    out.push(evaluate_binomial_like(name, &left[0], r, context)?);
                }
                Ok(Expression::Vector(out))
            } else if right.len() == 1 {
                let mut out = Vec::with_capacity(left.len());
                for l in left {
                    out.push(evaluate_binomial_like(name, l, &right[0], context)?);
                }
                Ok(Expression::Vector(out))
            } else {
                Ok(make_unevaluated(name, &[lhs.clone(), rhs.clone()]))
            }
        }
        (Expression::Vector(items), _) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(evaluate_binomial_like(name, item, rhs, context)?);
            }
            Ok(Expression::Vector(out))
        }
        (_, Expression::Vector(items)) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(evaluate_binomial_like(name, lhs, item, context)?);
            }
            Ok(Expression::Vector(out))
        }
        (Expression::Number(n), Expression::Number(k)) => {
            Ok(binomial_or_gamma(name, n, k, context))
        }
        _ => Ok(make_unevaluated(name, &[lhs.clone(), rhs.clone()])),
    }
}

// ---------------------------------------------------------------------------
// Function info constants
// ---------------------------------------------------------------------------

static FACTORIAL_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "factorial",
    aliases: &[],
    min_args: 1,
    max_args: Some(1),
    description: "Factorial",
};

static FACTORIAL2_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "factorial2",
    aliases: &[],
    min_args: 1,
    max_args: Some(1),
    description: "Double factorial",
};

static MULTIFACTORIAL_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "multifactorial",
    aliases: &[],
    min_args: 2,
    max_args: Some(2),
    description: "Multi-factorial",
};

static BINOMIAL_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "binomial",
    aliases: &[],
    min_args: 2,
    max_args: Some(2),
    description: "Binomial coefficient",
};

static PERM_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "perm",
    aliases: &["variations"],
    min_args: 2,
    max_args: Some(2),
    description: "Permutations (variations)",
};

static COMB_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "comb",
    aliases: &[],
    min_args: 2,
    max_args: Some(2),
    description: "Combinations",
};

static DERANGEMENTS_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "derangements",
    aliases: &[],
    min_args: 1,
    max_args: Some(1),
    description: "Derangements",
};

static HYPERFACTORIAL_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "hyperfactorial",
    aliases: &[],
    min_args: 1,
    max_args: Some(1),
    description: "Hyperfactorial",
};

static SUPERFACTORIAL_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "superfactorial",
    aliases: &[],
    min_args: 1,
    max_args: Some(1),
    description: "Superfactorial",
};

// ---------------------------------------------------------------------------
// Function implementations
// ---------------------------------------------------------------------------

struct FactorialFn;
impl BuiltinFunction for FactorialFn {
    fn info(&self) -> &BuiltinFunctionInfo {
        &FACTORIAL_INFO
    }
    fn evaluate(&self, args: &[Expression], _context: &mut CalculatorContext) -> FunctionResult {
        validate_arity("factorial", args, 1, Some(1))?;
        match &args[0] {
            Expression::Number(num) => {
                let res = num.factorial();
                Ok(Expression::Number(res))
            }
            _ => Ok(make_unevaluated("factorial", args)),
        }
    }
}

struct Factorial2Fn;
impl BuiltinFunction for Factorial2Fn {
    fn info(&self) -> &BuiltinFunctionInfo {
        &FACTORIAL2_INFO
    }
    fn evaluate(&self, args: &[Expression], _context: &mut CalculatorContext) -> FunctionResult {
        validate_arity("factorial2", args, 1, Some(1))?;
        match &args[0] {
            Expression::Number(num) => {
                let res = num.double_factorial();
                Ok(Expression::Number(res))
            }
            _ => Ok(make_unevaluated("factorial2", args)),
        }
    }
}

struct MultiFactorialFn;
impl BuiltinFunction for MultiFactorialFn {
    fn info(&self) -> &BuiltinFunctionInfo {
        &MULTIFACTORIAL_INFO
    }
    fn evaluate(&self, args: &[Expression], _context: &mut CalculatorContext) -> FunctionResult {
        validate_arity("multifactorial", args, 2, Some(2))?;
        match (&args[0], &args[1]) {
            (Expression::Number(n), Expression::Number(k_num)) => {
                if let Some(k_int) = k_num.to_integer() {
                    if k_int > 0 {
                        if let Some(k_u32) = k_int.to_u32() {
                            let res = n.multi_factorial(k_u32);
                            return Ok(Expression::Number(res));
                        }
                    }
                }
                Ok(Expression::Number(Number::nan()))
            }
            _ => Ok(make_unevaluated("multifactorial", args)),
        }
    }
}

struct BinomialFn;
impl BuiltinFunction for BinomialFn {
    fn info(&self) -> &BuiltinFunctionInfo {
        &BINOMIAL_INFO
    }
    fn evaluate(&self, args: &[Expression], context: &mut CalculatorContext) -> FunctionResult {
        validate_arity("binomial", args, 2, Some(2))?;
        evaluate_binomial_like("binomial", &args[0], &args[1], context)
    }
}

struct PermFn;
impl BuiltinFunction for PermFn {
    fn info(&self) -> &BuiltinFunctionInfo {
        &PERM_INFO
    }
    fn evaluate(&self, args: &[Expression], context: &mut CalculatorContext) -> FunctionResult {
        validate_arity("perm", args, 2, Some(2))?;
        match (&args[0], &args[1]) {
            (Expression::Number(n), Expression::Number(k)) => {
                let n_int = n.to_integer();
                let k_int = k.to_integer();
                if let (Some(ni), Some(ki)) = (n_int, k_int) {
                    if ki < 0 || (ni >= 0 && ki > ni) {
                        return Ok(Expression::Number(Number::from_i32(0)));
                    }
                    if let Some(k_u32) = bounded_exact_steps(&ki) {
                        let mut result = rug::Integer::from(1);
                        for i in 0..k_u32 {
                            result *= rug::Integer::from(&ni - i);
                        }
                        return Ok(Expression::Number(integer_number(result)));
                    }
                }
                // Non-integers: perm(n, k) = gamma(n+1) / gamma(n-k+1)
                if context.evaluation_options.approximation
                    == crate::options::ApproximationMode::Exact
                {
                    Ok(make_unevaluated("perm", args))
                } else {
                    let one = Number::from_i32(1);
                    let n_plus_1 = n.add(&one);
                    let n_minus_k_plus_1 = n.sub(k).add(&one);
                    let g1 = n_plus_1.gamma();
                    let g2 = n_minus_k_plus_1.gamma();
                    let res = g1.div(&g2);
                    Ok(Expression::Number(res))
                }
            }
            _ => Ok(make_unevaluated("perm", args)),
        }
    }
}

struct CombFn;
impl BuiltinFunction for CombFn {
    fn info(&self) -> &BuiltinFunctionInfo {
        &COMB_INFO
    }
    fn evaluate(&self, args: &[Expression], context: &mut CalculatorContext) -> FunctionResult {
        validate_arity("comb", args, 2, Some(2))?;
        evaluate_binomial_like("comb", &args[0], &args[1], context)
    }
}

struct DerangementsFn;
impl BuiltinFunction for DerangementsFn {
    fn info(&self) -> &BuiltinFunctionInfo {
        &DERANGEMENTS_INFO
    }
    fn evaluate(&self, args: &[Expression], context: &mut CalculatorContext) -> FunctionResult {
        validate_arity("derangements", args, 1, Some(1))?;
        match &args[0] {
            Expression::Number(n) => {
                if let Some(n_int) = n.to_integer() {
                    if n_int < 0 {
                        return Ok(Expression::Number(Number::nan()));
                    }
                    if n_int == 0 {
                        return Ok(Expression::Number(Number::from_i32(1)));
                    }
                    if n_int == 1 {
                        return Ok(Expression::Number(Number::from_i32(0)));
                    }
                    if let Some(n_u32) = bounded_exact_steps(&n_int) {
                        let mut a = rug::Integer::from(1);
                        let mut b = rug::Integer::from(0);
                        for i in 2..=n_u32 {
                            let sum_ab = rug::Integer::from(&a + &b);
                            let c = rug::Integer::from(i - 1) * &sum_ab;
                            a = b;
                            b = c;
                        }
                        return Ok(Expression::Number(integer_number(b)));
                    }
                }
                if context.evaluation_options.approximation
                    == crate::options::ApproximationMode::Exact
                {
                    Ok(make_unevaluated("derangements", args))
                } else {
                    Ok(Expression::Number(Number::nan()))
                }
            }
            _ => Ok(make_unevaluated("derangements", args)),
        }
    }
}

struct HyperfactorialFn;
impl BuiltinFunction for HyperfactorialFn {
    fn info(&self) -> &BuiltinFunctionInfo {
        &HYPERFACTORIAL_INFO
    }
    fn evaluate(&self, args: &[Expression], context: &mut CalculatorContext) -> FunctionResult {
        validate_arity("hyperfactorial", args, 1, Some(1))?;
        match &args[0] {
            Expression::Number(n) => {
                if let Some(n_int) = n.to_integer() {
                    if n_int < 0 {
                        return Ok(Expression::Number(Number::nan()));
                    }
                    if n_int == 0 || n_int == 1 {
                        return Ok(Expression::Number(Number::from_i32(1)));
                    }
                    if let Some(n_u32) = bounded_exact_steps(&n_int) {
                        let mut res = rug::Integer::from(1);
                        for i in 2..=n_u32 {
                            let val = rug::Integer::from(i).pow(i);
                            res *= val;
                        }
                        return Ok(Expression::Number(integer_number(res)));
                    }
                }
                if context.evaluation_options.approximation
                    == crate::options::ApproximationMode::Exact
                {
                    Ok(make_unevaluated("hyperfactorial", args))
                } else {
                    Ok(Expression::Number(Number::nan()))
                }
            }
            _ => Ok(make_unevaluated("hyperfactorial", args)),
        }
    }
}

struct SuperfactorialFn;
impl BuiltinFunction for SuperfactorialFn {
    fn info(&self) -> &BuiltinFunctionInfo {
        &SUPERFACTORIAL_INFO
    }
    fn evaluate(&self, args: &[Expression], context: &mut CalculatorContext) -> FunctionResult {
        validate_arity("superfactorial", args, 1, Some(1))?;
        match &args[0] {
            Expression::Number(n) => {
                if let Some(n_int) = n.to_integer() {
                    if n_int < 0 {
                        return Ok(Expression::Number(Number::nan()));
                    }
                    if n_int == 0 || n_int == 1 {
                        return Ok(Expression::Number(Number::from_i32(1)));
                    }
                    if let Some(n_u32) = bounded_exact_steps(&n_int) {
                        let mut res = rug::Integer::from(1);
                        let mut current_fact = rug::Integer::from(1);
                        for i in 2..=n_u32 {
                            current_fact *= i;
                            res *= &current_fact;
                        }
                        return Ok(Expression::Number(integer_number(res)));
                    }
                }
                if context.evaluation_options.approximation
                    == crate::options::ApproximationMode::Exact
                {
                    Ok(make_unevaluated("superfactorial", args))
                } else {
                    Ok(Expression::Number(Number::nan()))
                }
            }
            _ => Ok(make_unevaluated("superfactorial", args)),
        }
    }
}

// ---------------------------------------------------------------------------
// Catalog and lookup
// ---------------------------------------------------------------------------

static CATALOG: &[&BuiltinFunctionInfo] = &[
    &FACTORIAL_INFO,
    &FACTORIAL2_INFO,
    &MULTIFACTORIAL_INFO,
    &BINOMIAL_INFO,
    &PERM_INFO,
    &COMB_INFO,
    &DERANGEMENTS_INFO,
    &HYPERFACTORIAL_INFO,
    &SUPERFACTORIAL_INFO,
];

/// Returns all combinatorics function infos.
pub fn catalog() -> Vec<&'static BuiltinFunctionInfo> {
    CATALOG.to_vec()
}

/// Looks up a built-in combinatorics function by name (including aliases).
pub fn lookup(name: &str) -> Option<&'static dyn BuiltinFunction> {
    match name {
        "factorial" => Some(&FactorialFn),
        "factorial2" => Some(&Factorial2Fn),
        "multifactorial" => Some(&MultiFactorialFn),
        "binomial" => Some(&BinomialFn),
        "perm" | "variations" => Some(&PermFn),
        "comb" => Some(&CombFn),
        "derangements" => Some(&DerangementsFn),
        "hyperfactorial" => Some(&HyperfactorialFn),
        "superfactorial" => Some(&SuperfactorialFn),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ctx() -> CalculatorContext {
        CalculatorContext::default()
    }

    fn eval_expr(name: &str, args: &[Expression]) -> Result<Expression, String> {
        let func = lookup(name).ok_or_else(|| format!("function {name} not found"))?;
        let mut ctx = make_ctx();
        func.evaluate(args, &mut ctx).map_err(|e| e.to_string())
    }

    fn eval_func(name: &str, args: &[Number]) -> Result<Number, String> {
        let expr_args: Vec<_> = args.iter().map(|n| Expression::Number(n.clone())).collect();
        match eval_expr(name, &expr_args) {
            Ok(Expression::Number(n)) => Ok(n),
            Ok(other) => Err(format!("Expected Number, got {other:?}")),
            Err(e) => Err(e.to_string()),
        }
    }

    fn assert_exact_integer(value: Number) {
        assert!(value.to_integer().is_some(), "expected exact integer, got {value}");
    }

    #[test]
    fn test_factorial() {
        assert_eq!(eval_func("factorial", &[Number::from_i32(0)]).unwrap().to_string(), "1");
        assert_eq!(eval_func("factorial", &[Number::from_i32(1)]).unwrap().to_string(), "1");
        assert_eq!(eval_func("factorial", &[Number::from_i32(5)]).unwrap().to_string(), "120");
    }

    #[test]
    fn test_factorial2() {
        assert_eq!(eval_func("factorial2", &[Number::from_i32(0)]).unwrap().to_string(), "1");
        assert_eq!(eval_func("factorial2", &[Number::from_i32(-1)]).unwrap().to_string(), "1");
        assert_eq!(eval_func("factorial2", &[Number::from_i32(1)]).unwrap().to_string(), "1");
        assert_eq!(eval_func("factorial2", &[Number::from_i32(5)]).unwrap().to_string(), "15");
        assert_eq!(eval_func("factorial2", &[Number::from_i32(6)]).unwrap().to_string(), "48");
    }

    #[test]
    fn test_multifactorial() {
        assert_eq!(
            eval_func("multifactorial", &[Number::from_i32(5), Number::from_i32(2)]).unwrap().to_string(),
            "15"
        );
        assert_eq!(
            eval_func("multifactorial", &[Number::from_i32(6), Number::from_i32(3)]).unwrap().to_string(),
            "18"
        );
        assert!(eval_func("multifactorial", &[Number::from_i32(-1), Number::from_i32(2)])
            .unwrap()
            .is_nan());
    }

    #[test]
    fn test_binomial() {
        assert_eq!(
            eval_func("binomial", &[Number::from_i32(5), Number::from_i32(2)]).unwrap().to_string(),
            "10"
        );
        assert_eq!(
            eval_func("binomial", &[Number::from_i32(5), Number::from_i32(0)]).unwrap().to_string(),
            "1"
        );
        assert_eq!(
            eval_func("binomial", &[Number::from_i32(5), Number::from_i32(5)]).unwrap().to_string(),
            "1"
        );
        assert_eq!(
            eval_func("binomial", &[Number::from_i32(-3), Number::from_i32(-1)]).unwrap().to_string(),
            "0"
        );
        assert_eq!(
            eval_func(
                "binomial",
                &[
                    Number::from_i64(4_294_967_297),
                    Number::from_i64(4_294_967_296)
                ]
            )
            .unwrap()
            .to_string(),
            "4294967297"
        );
    }

    #[test]
    fn test_binomial_vectorizes_arguments() {
        let result = eval_expr(
            "binomial",
            &[
                Expression::Vector(vec![
                    Expression::Number(Number::from_i32(2)),
                    Expression::Number(Number::from_i32(3)),
                ]),
                Expression::Number(Number::from_i32(1)),
            ],
        )
        .unwrap();

        assert_eq!(
            result,
            Expression::Vector(vec![
                Expression::Number(Number::from_i32(2)),
                Expression::Number(Number::from_i32(3)),
            ])
        );
    }

    #[test]
    fn test_binomial_vectorizes_length_one_arguments() {
        let result = eval_expr(
            "binomial",
            &[
                Expression::Vector(vec![Expression::Number(Number::from_i32(2))]),
                Expression::Vector(vec![
                    Expression::Number(Number::from_i32(1)),
                    Expression::Number(Number::from_i32(2)),
                ]),
            ],
        )
        .unwrap();

        assert_eq!(
            result,
            Expression::Vector(vec![
                Expression::Number(Number::from_i32(2)),
                Expression::Number(Number::from_i32(1)),
            ])
        );
    }

    #[test]
    fn test_perm() {
        assert_eq!(
            eval_func("perm", &[Number::from_i32(5), Number::from_i32(2)]).unwrap().to_string(),
            "20"
        );
        assert_eq!(
            eval_func("perm", &[Number::from_i32(5), Number::from_i32(5)]).unwrap().to_string(),
            "120"
        );
        assert_eq!(
            eval_func("perm", &[Number::from_i32(5), Number::from_i32(6)]).unwrap().to_string(),
            "0"
        );
        assert_eq!(
            eval_func("perm", &[Number::from_i32(-3), Number::from_i32(2)]).unwrap().to_string(),
            "12"
        );
        assert_exact_integer(eval_func("perm", &[Number::from_i32(10_001), Number::from_i32(10_001)]).unwrap());
    }

    #[test]
    fn test_comb() {
        assert_eq!(
            eval_func("comb", &[Number::from_i32(5), Number::from_i32(2)]).unwrap().to_string(),
            "10"
        );
    }

    #[test]
    fn test_derangements() {
        assert_eq!(eval_func("derangements", &[Number::from_i32(0)]).unwrap().to_string(), "1");
        assert_eq!(eval_func("derangements", &[Number::from_i32(1)]).unwrap().to_string(), "0");
        assert_eq!(eval_func("derangements", &[Number::from_i32(2)]).unwrap().to_string(), "1");
        assert_eq!(eval_func("derangements", &[Number::from_i32(3)]).unwrap().to_string(), "2");
        assert_eq!(eval_func("derangements", &[Number::from_i32(4)]).unwrap().to_string(), "9");
        assert_eq!(eval_func("derangements", &[Number::from_i32(5)]).unwrap().to_string(), "44");
        assert_exact_integer(eval_func("derangements", &[Number::from_i32(10_001)]).unwrap());
    }

    #[test]
    fn test_hyperfactorial() {
        assert_eq!(eval_func("hyperfactorial", &[Number::from_i32(1)]).unwrap().to_string(), "1");
        assert_eq!(eval_func("hyperfactorial", &[Number::from_i32(2)]).unwrap().to_string(), "4");
        assert_eq!(eval_func("hyperfactorial", &[Number::from_i32(3)]).unwrap().to_string(), "108");
        assert_exact_integer(eval_func("hyperfactorial", &[Number::from_i32(151)]).unwrap());
    }

    #[test]
    fn test_superfactorial() {
        assert_eq!(eval_func("superfactorial", &[Number::from_i32(1)]).unwrap().to_string(), "1");
        assert_eq!(eval_func("superfactorial", &[Number::from_i32(2)]).unwrap().to_string(), "2");
        assert_eq!(eval_func("superfactorial", &[Number::from_i32(3)]).unwrap().to_string(), "12");
        assert_eq!(eval_func("superfactorial", &[Number::from_i32(4)]).unwrap().to_string(), "288");
        assert_exact_integer(eval_func("superfactorial", &[Number::from_i32(1_001)]).unwrap());
    }

    #[test]
    fn test_exact_combinatorics_budget_leaves_large_calls_unevaluated() {
        let func = lookup("derangements").expect("derangements registered");
        let mut ctx = make_ctx();
        ctx.evaluation_options.approximation = crate::options::ApproximationMode::Exact;
        let oversized = Expression::Number(Number::from_i64(MAX_EXACT_COMBINATORICS_STEPS as i64 + 1));
        let result = func.evaluate(&[oversized], &mut ctx).unwrap();

        assert!(matches!(result, Expression::FunctionCall { .. }));
    }
}
