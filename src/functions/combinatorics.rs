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
        match (&args[0], &args[1]) {
            (Expression::Number(n), Expression::Number(k)) => {
                if let Some(res) = n.binomial(k) {
                    Ok(Expression::Number(res))
                } else {
                    if context.evaluation_options.approximation == crate::options::ApproximationMode::Exact {
                        Ok(make_unevaluated("binomial", args))
                    } else {
                        // gamma(n+1) / (gamma(n-k+1) * gamma(k+1))
                        let one = Number::from_i32(1);
                        let n_plus_1 = n.add(&one);
                        let n_minus_k_plus_1 = n.sub(k).add(&one);
                        let k_plus_1 = k.add(&one);
                        let g1 = n_plus_1.gamma();
                        let g2 = n_minus_k_plus_1.gamma();
                        let g3 = k_plus_1.gamma();
                        let res = g1.div(&g2.mul(&g3));
                        Ok(Expression::Number(res))
                    }
                }
            }
            _ => Ok(make_unevaluated("binomial", args)),
        }
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
                    if ni < 0 || ki < 0 || ki > ni {
                        return Ok(Expression::Number(Number::from_i32(0)));
                    }
                    if let Some(res_bin) = n.binomial(k) {
                        let fact_k = k.factorial();
                        let res = res_bin.mul(&fact_k);
                        return Ok(Expression::Number(res));
                    }
                }
                // Non-integers: perm(n, k) = gamma(n+1) / gamma(n-k+1)
                if context.evaluation_options.approximation == crate::options::ApproximationMode::Exact {
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
        // Identical to binomial
        match (&args[0], &args[1]) {
            (Expression::Number(n), Expression::Number(k)) => {
                if let Some(res) = n.binomial(k) {
                    Ok(Expression::Number(res))
                } else {
                    if context.evaluation_options.approximation == crate::options::ApproximationMode::Exact {
                        Ok(make_unevaluated("comb", args))
                    } else {
                        let one = Number::from_i32(1);
                        let n_plus_1 = n.add(&one);
                        let n_minus_k_plus_1 = n.sub(k).add(&one);
                        let k_plus_1 = k.add(&one);
                        let g1 = n_plus_1.gamma();
                        let g2 = n_minus_k_plus_1.gamma();
                        let g3 = k_plus_1.gamma();
                        let res = g1.div(&g2.mul(&g3));
                        Ok(Expression::Number(res))
                    }
                }
            }
            _ => Ok(make_unevaluated("comb", args)),
        }
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
                    if let Some(n_u32) = n_int.to_u32() {
                        if n_u32 > 10_000 {
                            return Ok(Expression::Number(Number::nan()));
                        }
                        let mut a = rug::Integer::from(1);
                        let mut b = rug::Integer::from(0);
                        for i in 2..=n_u32 {
                            let sum_ab = rug::Integer::from(&a + &b);
                            let c = rug::Integer::from(i - 1) * &sum_ab;
                            a = b;
                            b = c;
                        }
                        return Ok(Expression::Number(Number::from_rational(crate::number::Rational {
                            value: rug::Rational::from(b),
                        })));
                    }
                }
                if context.evaluation_options.approximation == crate::options::ApproximationMode::Exact {
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
                    if let Some(n_u32) = n_int.to_u32() {
                        if n_u32 > 150 {
                            return Ok(Expression::Number(Number::nan()));
                        }
                        let mut res = rug::Integer::from(1);
                        for i in 2..=n_u32 {
                            let val = rug::Integer::from(i).pow(i);
                            res *= val;
                        }
                        return Ok(Expression::Number(Number::from_rational(crate::number::Rational {
                            value: rug::Rational::from(res),
                        })));
                    }
                }
                if context.evaluation_options.approximation == crate::options::ApproximationMode::Exact {
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
                    if let Some(n_u32) = n_int.to_u32() {
                        if n_u32 > 1000 {
                            return Ok(Expression::Number(Number::nan()));
                        }
                        let mut res = rug::Integer::from(1);
                        let mut current_fact = rug::Integer::from(1);
                        for i in 2..=n_u32 {
                            current_fact *= i;
                            res *= &current_fact;
                        }
                        return Ok(Expression::Number(Number::from_rational(crate::number::Rational {
                            value: rug::Rational::from(res),
                        })));
                    }
                }
                if context.evaluation_options.approximation == crate::options::ApproximationMode::Exact {
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

    fn eval_func(name: &str, args: &[Number]) -> Result<Number, String> {
        let func = lookup(name).ok_or_else(|| format!("function {name} not found"))?;
        let expr_args: Vec<_> = args.iter().map(|n| Expression::Number(n.clone())).collect();
        let mut ctx = make_ctx();
        match func.evaluate(&expr_args, &mut ctx) {
            Ok(Expression::Number(n)) => Ok(n),
            Ok(other) => Err(format!("Expected Number, got {other:?}")),
            Err(e) => Err(e.to_string()),
        }
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
    }

    #[test]
    fn test_hyperfactorial() {
        assert_eq!(eval_func("hyperfactorial", &[Number::from_i32(1)]).unwrap().to_string(), "1");
        assert_eq!(eval_func("hyperfactorial", &[Number::from_i32(2)]).unwrap().to_string(), "4");
        assert_eq!(eval_func("hyperfactorial", &[Number::from_i32(3)]).unwrap().to_string(), "108");
    }

    #[test]
    fn test_superfactorial() {
        assert_eq!(eval_func("superfactorial", &[Number::from_i32(1)]).unwrap().to_string(), "1");
        assert_eq!(eval_func("superfactorial", &[Number::from_i32(2)]).unwrap().to_string(), "2");
        assert_eq!(eval_func("superfactorial", &[Number::from_i32(3)]).unwrap().to_string(), "12");
        assert_eq!(eval_func("superfactorial", &[Number::from_i32(4)]).unwrap().to_string(), "288");
    }
}
