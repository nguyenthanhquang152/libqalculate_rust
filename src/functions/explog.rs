//! Exponential and logarithmic built-in function family.
//!
//! # Upstream oracle
//! - `../libqalculate/libqalculate/BuiltinFunctions-explog.cc`
//! - `../libqalculate/libqalculate/BuiltinFunctions.h`
//! - `../libqalculate/data/functions.xml.in` (sqrt, cbrt, root, exp, ln, log, lambertw, cis, powertower, allroots)
//! - `../libqalculate/tests/explog.batch`

use crate::ast::Expression;
use crate::context::CalculatorContext;
use crate::functions::{BuiltinFunction, BuiltinFunctionInfo, FunctionError, FunctionResult};
use crate::number::Number;

// ---------------------------------------------------------------------------
// Function info constants
// ---------------------------------------------------------------------------

static SQRT_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "sqrt",
    aliases: &["sqr"],
    min_args: 1,
    max_args: Some(1),
    description: "Square root",
};

static CBRT_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "cbrt",
    aliases: &[],
    min_args: 1,
    max_args: Some(1),
    description: "Cube root",
};

static ROOT_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "root",
    aliases: &[],
    min_args: 2,
    max_args: Some(2),
    description: "Nth root",
};

static EXP_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "exp",
    aliases: &[],
    min_args: 1,
    max_args: Some(1),
    description: "Exponential (e^x)",
};

static LOG_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "ln",
    aliases: &["log"],
    min_args: 1,
    max_args: Some(1),
    description: "Natural logarithm",
};

static LOGN_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "logn",
    aliases: &["log2", "log10"],
    min_args: 1,
    max_args: Some(2),
    description: "Logarithm with base",
};

static POWERTOWER_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "powertower",
    aliases: &[],
    min_args: 2,
    max_args: Some(2),
    description: "Power tower (repeated exponentiation)",
};

static ALLROOTS_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "allroots",
    aliases: &[],
    min_args: 2,
    max_args: Some(2),
    description: "All nth roots of a number",
};

// ---------------------------------------------------------------------------
// Function implementations
// ---------------------------------------------------------------------------

/// `sqrt(x)` — Square root.
///
/// Upstream: `SqrtFunction` in `BuiltinFunctions-explog.cc`.
/// For negative reals, returns `i * sqrt(|x|)`.
/// For complex: delegates to `pow(x, 1/2)`.
struct SqrtFn;

impl BuiltinFunction for SqrtFn {
    fn info(&self) -> &BuiltinFunctionInfo {
        &SQRT_INFO
    }

    fn evaluate(&self, args: &[Expression], _context: &mut CalculatorContext) -> FunctionResult {
        validate_arity("sqrt", args, 1, Some(1))?;
        match &args[0] {
            Expression::Number(num) => Ok(Expression::Number(num.sqrt())),
            _ => Ok(make_unevaluated("sqrt", args)),
        }
    }
}

/// `cbrt(x)` — Cube root.
///
/// Upstream: `CbrtFunction` in `BuiltinFunctions-explog.cc`.
/// cbrt(x) = x^(1/3) for all real x (including negative).
struct CbrtFn;

impl BuiltinFunction for CbrtFn {
    fn info(&self) -> &BuiltinFunctionInfo {
        &CBRT_INFO
    }

    fn evaluate(&self, args: &[Expression], _context: &mut CalculatorContext) -> FunctionResult {
        validate_arity("cbrt", args, 1, Some(1))?;
        match &args[0] {
            Expression::Number(num) => Ok(Expression::Number(num.cbrt())),
            _ => Ok(make_unevaluated("cbrt", args)),
        }
    }
}

/// `root(x, n)` — Nth root.
///
/// Upstream: `RootFunction` in `BuiltinFunctions-explog.cc`.
/// root(x, n) = x^(1/n).
struct RootFn;

impl BuiltinFunction for RootFn {
    fn info(&self) -> &BuiltinFunctionInfo {
        &ROOT_INFO
    }

    fn evaluate(&self, args: &[Expression], context: &mut CalculatorContext) -> FunctionResult {
        validate_arity("root", args, 2, Some(2))?;
        match (&args[0], &args[1]) {
            (Expression::Number(x), Expression::Number(n)) => {
                if n.is_zero() {
                    push_error(context, "root", "Division by zero");
                    return Ok(Expression::Number(Number::nan()));
                }
                let reciprocal_n = Number::one().div(n);
                Ok(Expression::Number(x.pow(&reciprocal_n)))
            }
            _ => Ok(make_unevaluated("root", args)),
        }
    }
}

/// `exp(x)` — Exponential function e^x.
///
/// Upstream: `ExpFunction` in `BuiltinFunctions-explog.cc`.
struct ExpFn;

impl BuiltinFunction for ExpFn {
    fn info(&self) -> &BuiltinFunctionInfo {
        &EXP_INFO
    }

    fn evaluate(&self, args: &[Expression], _context: &mut CalculatorContext) -> FunctionResult {
        validate_arity("exp", args, 1, Some(1))?;
        match &args[0] {
            Expression::Number(num) => Ok(Expression::Number(num.exp())),
            _ => Ok(make_unevaluated("exp", args)),
        }
    }
}

/// `ln(x)` / `log(x)` — Natural logarithm.
///
/// Upstream: `LogFunction` in `BuiltinFunctions-explog.cc`.
/// `log` with one argument is treated as `ln` (natural log).
struct LogFn;

impl BuiltinFunction for LogFn {
    fn info(&self) -> &BuiltinFunctionInfo {
        &LOG_INFO
    }

    fn evaluate(&self, args: &[Expression], context: &mut CalculatorContext) -> FunctionResult {
        validate_arity("ln", args, 1, Some(1))?;
        match &args[0] {
            Expression::Number(num) => {
                if num.is_zero() {
                    push_warning(context, "ln", "Logarithm of zero");
                    return Ok(Expression::Number(Number::minus_infinity()));
                }
                Ok(Expression::Number(num.ln()))
            }
            _ => Ok(make_unevaluated("ln", args)),
        }
    }
}

/// `logn(x, base)` / `log2(x)` / `log10(x)` — Logarithm with base.
///
/// Upstream: `LognFunction` in `BuiltinFunctions-explog.cc`.
/// logn(x, b) = ln(x) / ln(b).
/// log2(x) is logn(x, 2), log10(x) is logn(x, 10).
struct LognFn;

impl BuiltinFunction for LognFn {
    fn info(&self) -> &BuiltinFunctionInfo {
        &LOGN_INFO
    }

    fn evaluate(&self, args: &[Expression], context: &mut CalculatorContext) -> FunctionResult {
        validate_arity("logn", args, 1, Some(2))?;
        let base = if args.len() == 2 {
            match &args[1] {
                Expression::Number(b) => b.clone(),
                _ => return Ok(make_unevaluated("logn", args)),
            }
        } else {
            // Default base = e (natural log)
            Number::e()
        };

        match &args[0] {
            Expression::Number(x) => {
                if x.is_zero() {
                    push_warning(context, "logn", "Logarithm of zero");
                    return Ok(Expression::Number(Number::minus_infinity()));
                }
                if base.is_zero() {
                    push_error(context, "logn", "Logarithm base is zero");
                    return Ok(Expression::Number(Number::nan()));
                }
                // logn(x, b) = ln(x) / ln(b)
                let ln_x = x.ln();
                let ln_b = base.ln();
                if ln_b.is_zero() {
                    push_error(context, "logn", "Logarithm base is 1");
                    return Ok(Expression::Number(Number::nan()));
                }
                Ok(Expression::Number(ln_x.div(&ln_b)))
            }
            _ => Ok(make_unevaluated("logn", args)),
        }
    }
}

/// `powertower(x, n)` — Power tower (repeated exponentiation).
///
/// Upstream: `PowerTowerFunction` in `BuiltinFunctions-explog.cc`.
/// powertower(x, n) = x^x^x^...^x (n times), evaluated right-to-left.
struct PowerTowerFn;

impl BuiltinFunction for PowerTowerFn {
    fn info(&self) -> &BuiltinFunctionInfo {
        &POWERTOWER_INFO
    }

    fn evaluate(&self, args: &[Expression], context: &mut CalculatorContext) -> FunctionResult {
        validate_arity("powertower", args, 2, Some(2))?;
        match (&args[0], &args[1]) {
            (Expression::Number(base), Expression::Number(height)) => {
                // height must be a positive integer
                let h = match height.to_i64() {
                    Some(h) if h > 0 => h as u64,
                    _ => {
                        push_error(
                            context,
                            "powertower",
                            "Height must be a positive integer",
                        );
                        return Ok(Expression::Number(Number::nan()));
                    }
                };

                // Evaluate right-to-left: powertower(x, n) = x^(x^(x^...))
                let mut result = base.clone();
                for _ in 1..h {
                    result = base.pow(&result);
                }
                Ok(Expression::Number(result))
            }
            _ => Ok(make_unevaluated("powertower", args)),
        }
    }
}

/// `allroots(x, n)` — All nth roots of a number.
///
/// Upstream: `AllRootsFunction` in `BuiltinFunctions-explog.cc`.
/// Returns a vector of all n complex nth roots of x.
struct AllRootsFn;

impl BuiltinFunction for AllRootsFn {
    fn info(&self) -> &BuiltinFunctionInfo {
        &ALLROOTS_INFO
    }

    fn evaluate(&self, args: &[Expression], context: &mut CalculatorContext) -> FunctionResult {
        validate_arity("allroots", args, 2, Some(2))?;
        match (&args[0], &args[1]) {
            (Expression::Number(x), Expression::Number(n_num)) => {
                let n = match n_num.to_i64() {
                    Some(n) if n > 0 => n as u64,
                    _ => {
                        push_error(
                            context,
                            "allroots",
                            "Degree must be a positive integer",
                        );
                        return Ok(Expression::Number(Number::nan()));
                    }
                };

                // All nth roots of x: x^(1/n) * e^(2πik/n) for k = 0..n-1
                // Principal root: x^(1/n)
                let reciprocal_n = Number::one().div(n_num);
                let principal = x.pow(&reciprocal_n);

                if n == 1 {
                    return Ok(Expression::Vector(vec![Expression::Number(principal)]));
                }

                let mut roots = Vec::with_capacity(n as usize);
                let pi = Number::pi();
                let two = Number::from_i64(2);

                for k in 0..n {
                    if k == 0 {
                        roots.push(Expression::Number(principal.clone()));
                    } else {
                        // angle = 2*pi*k/n
                        let angle = two
                            .mul(&pi)
                            .mul(&Number::from_i64(k as i64))
                            .div(&Number::from_i64(n as i64));
                        let cos_a = angle.cos();
                        let sin_a = angle.sin();
                        // root_k = principal_abs * (cos(angle) + i*sin(angle))
                        // Since principal might be complex, we multiply:
                        // root_k = |x|^(1/n) * cis(arg(x)/n + 2πk/n)
                        // For real x: principal * cis(2πk/n)
                        let re_part = principal.mul(&cos_a);
                        let im_part = principal.mul(&sin_a);
                        let root = Number::new_complex_from_re_im(&re_part, &im_part);
                        roots.push(Expression::Number(root));
                    }
                }

                Ok(Expression::Vector(roots))
            }
            _ => Ok(make_unevaluated("allroots", args)),
        }
    }
}

// ---------------------------------------------------------------------------
// Catalog and lookup
// ---------------------------------------------------------------------------

/// All explog function infos for registry enumeration.
static CATALOG: &[&BuiltinFunctionInfo] = &[
    &SQRT_INFO,
    &CBRT_INFO,
    &ROOT_INFO,
    &EXP_INFO,
    &LOG_INFO,
    &LOGN_INFO,
    &POWERTOWER_INFO,
    &ALLROOTS_INFO,
];

/// Returns all explog function infos.
pub fn catalog() -> Vec<&'static BuiltinFunctionInfo> {
    CATALOG.to_vec()
}

/// Looks up a built-in explog function by name (including aliases).
pub fn lookup(name: &str) -> Option<&'static dyn BuiltinFunction> {
    match name {
        "sqrt" | "sqr" => Some(&SqrtFn),
        "cbrt" => Some(&CbrtFn),
        "root" => Some(&RootFn),
        "exp" => Some(&ExpFn),
        "ln" | "log" => Some(&LogFn),
        "logn" => Some(&LognFn),
        "log2" => Some(&LOG2_DISPATCH),
        "log10" => Some(&LOG10_DISPATCH),
        "powertower" => Some(&PowerTowerFn),
        "allroots" => Some(&AllRootsFn),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Convenience dispatchers for log2/log10
// ---------------------------------------------------------------------------

/// `log2(x)` — shorthand for `logn(x, 2)`.
struct Log2Dispatch;
static LOG2_DISPATCH: Log2Dispatch = Log2Dispatch;

impl BuiltinFunction for Log2Dispatch {
    fn info(&self) -> &BuiltinFunctionInfo {
        &LOGN_INFO
    }

    fn evaluate(&self, args: &[Expression], context: &mut CalculatorContext) -> FunctionResult {
        let mut extended_args = args.to_vec();
        extended_args.push(Expression::Number(Number::from_i64(2)));
        LognFn.evaluate(&extended_args, context)
    }
}

/// `log10(x)` — shorthand for `logn(x, 10)`.
struct Log10Dispatch;
static LOG10_DISPATCH: Log10Dispatch = Log10Dispatch;

impl BuiltinFunction for Log10Dispatch {
    fn info(&self) -> &BuiltinFunctionInfo {
        &LOGN_INFO
    }

    fn evaluate(&self, args: &[Expression], context: &mut CalculatorContext) -> FunctionResult {
        let mut extended_args = args.to_vec();
        extended_args.push(Expression::Number(Number::from_i64(10)));
        LognFn.evaluate(&extended_args, context)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Validates argument count and returns an error if out of range.
fn validate_arity(
    name: &str,
    args: &[Expression],
    min: usize,
    max: Option<usize>,
) -> Result<(), FunctionError> {
    if args.len() < min || max.is_some_and(|m| args.len() > m) {
        return Err(FunctionError {
            function_name: name.to_string(),
            message: format!(
                "Expected {} argument(s), got {}",
                if max == Some(min) {
                    min.to_string()
                } else if let Some(m) = max {
                    format!("{min}-{m}")
                } else {
                    format!("at least {min}")
                },
                args.len()
            ),
        });
    }
    Ok(())
}

/// Creates an unevaluated function call expression (for symbolic args).
fn make_unevaluated(name: &str, args: &[Expression]) -> Expression {
    use crate::ast::FunctionRef;
    Expression::FunctionCall {
        function: FunctionRef::new(name),
        args: args.to_vec(),
    }
}

/// Pushes a warning message to the context.
fn push_warning(context: &mut CalculatorContext, func_name: &str, message: &str) {
    let msg = crate::messages::CalculatorMessage::new(
        format!("{func_name}: {message}"),
        crate::messages::MessageType::Warning,
        crate::messages::MessageCategory::None,
        crate::messages::MessageStage::Calculation,
    );
    context.messages.push(msg);
}

/// Pushes an error message to the context.
fn push_error(context: &mut CalculatorContext, func_name: &str, message: &str) {
    let msg = crate::messages::CalculatorMessage::new(
        format!("{func_name}: {message}"),
        crate::messages::MessageType::Error,
        crate::messages::MessageCategory::None,
        crate::messages::MessageStage::Calculation,
    );
    context.messages.push(msg);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ctx() -> CalculatorContext {
        CalculatorContext::default()
    }

    // --- sqrt ---
    #[test]
    fn sqrt_of_4() {
        let mut ctx = make_ctx();
        let result = SqrtFn
            .evaluate(&[Expression::Number(Number::from_i64(4))], &mut ctx)
            .unwrap();
        match result {
            Expression::Number(n) => assert_eq!(n.to_string(), "2"),
            _ => panic!("Expected Number, got {result:?}"),
        }
    }

    #[test]
    fn sqrt_of_9() {
        let mut ctx = make_ctx();
        let result = SqrtFn
            .evaluate(&[Expression::Number(Number::from_i64(9))], &mut ctx)
            .unwrap();
        match result {
            Expression::Number(n) => assert_eq!(n.to_string(), "3"),
            _ => panic!("Expected Number"),
        }
    }

    #[test]
    fn sqrt_wrong_arity() {
        let mut ctx = make_ctx();
        let result = SqrtFn.evaluate(&[], &mut ctx);
        assert!(result.is_err());
    }

    // --- cbrt ---
    #[test]
    fn cbrt_of_8() {
        let mut ctx = make_ctx();
        let result = CbrtFn
            .evaluate(&[Expression::Number(Number::from_i64(8))], &mut ctx)
            .unwrap();
        match result {
            Expression::Number(n) => assert_eq!(n.to_string(), "2"),
            _ => panic!("Expected Number"),
        }
    }

    #[test]
    fn cbrt_of_27() {
        let mut ctx = make_ctx();
        let result = CbrtFn
            .evaluate(&[Expression::Number(Number::from_i64(27))], &mut ctx)
            .unwrap();
        match result {
            Expression::Number(n) => assert_eq!(n.to_string(), "3"),
            _ => panic!("Expected Number"),
        }
    }

    // --- root ---
    #[test]
    fn root_4_2() {
        let mut ctx = make_ctx();
        let result = RootFn
            .evaluate(
                &[
                    Expression::Number(Number::from_i64(4)),
                    Expression::Number(Number::from_i64(2)),
                ],
                &mut ctx,
            )
            .unwrap();
        match result {
            Expression::Number(n) => {
                // root(4, 2) = 4^(1/2) = 2 (returns float via pow)
                let s = n.to_qalc_string();
                assert!(
                    s.starts_with("2.000") || s == "2",
                    "root(4, 2) should be ~2, got {s}",
                );
            }
            _ => panic!("Expected Number"),
        }
    }

    // --- exp ---
    #[test]
    fn exp_of_0() {
        let mut ctx = make_ctx();
        let result = ExpFn
            .evaluate(&[Expression::Number(Number::from_i64(0))], &mut ctx)
            .unwrap();
        match result {
            Expression::Number(n) => assert_eq!(n.to_string(), "1"),
            _ => panic!("Expected Number"),
        }
    }

    // --- ln ---
    #[test]
    fn ln_of_1() {
        let mut ctx = make_ctx();
        let result = LogFn
            .evaluate(&[Expression::Number(Number::from_i64(1))], &mut ctx)
            .unwrap();
        match result {
            Expression::Number(n) => {
                // ln(1) = 0
                assert!(n.is_zero(), "ln(1) should be 0, got {}", n.to_string());
            }
            _ => panic!("Expected Number"),
        }
    }

    // --- powertower ---
    #[test]
    fn powertower_2_4() {
        let mut ctx = make_ctx();
        let result = PowerTowerFn
            .evaluate(
                &[
                    Expression::Number(Number::from_i64(2)),
                    Expression::Number(Number::from_i64(4)),
                ],
                &mut ctx,
            )
            .unwrap();
        match result {
            Expression::Number(n) => {
                // powertower(2, 4) = 2^(2^(2^2)) = 2^(2^4) = 2^16 = 65536
                assert_eq!(n.to_string(), "65536");
            }
            _ => panic!("Expected Number"),
        }
    }

    // --- allroots ---
    #[test]
    fn allroots_returns_correct_count() {
        let mut ctx = make_ctx();
        let result = AllRootsFn
            .evaluate(
                &[
                    Expression::Number(Number::from_i64(4)),
                    Expression::Number(Number::from_i64(7)),
                ],
                &mut ctx,
            )
            .unwrap();
        match result {
            Expression::Vector(roots) => {
                assert_eq!(roots.len(), 7, "allroots(4, 7) should return 7 roots");
            }
            _ => panic!("Expected Vector"),
        }
    }

    // --- catalog ---
    #[test]
    fn catalog_is_non_empty() {
        let cat = catalog();
        assert!(!cat.is_empty());
        assert!(cat.iter().any(|f| f.name == "sqrt"));
        assert!(cat.iter().any(|f| f.name == "exp"));
        assert!(cat.iter().any(|f| f.name == "ln"));
    }

    // --- lookup ---
    #[test]
    fn lookup_finds_sqrt() {
        assert!(lookup("sqrt").is_some());
    }

    #[test]
    fn lookup_finds_log_alias() {
        assert!(lookup("log").is_some());
    }

    #[test]
    fn lookup_finds_log2() {
        assert!(lookup("log2").is_some());
    }

    #[test]
    fn lookup_finds_log10() {
        assert!(lookup("log10").is_some());
    }

    #[test]
    fn lookup_returns_none_for_unknown() {
        assert!(lookup("unknown_func").is_none());
    }
}
