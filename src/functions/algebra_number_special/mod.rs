//! Algebra, number, and special built-in function family.
//!
//! # Functions
//!
//! - **Number functions:** `abs`, `sgn`, `round`, `floor`, `ceil`, `trunc`
//! - **Complex algebra:** `re`, `im`, `arg`, `conj`
//! - **Special functions:** `gamma`, `erf`, `zeta`, `gammainc`
//!
//! # Upstream oracle
//! - `../libqalculate/libqalculate/BuiltinFunctions-number.cc`
//! - `../libqalculate/libqalculate/BuiltinFunctions-algebra.cc`
//! - `../libqalculate/libqalculate/BuiltinFunctions-special.cc`
//! - `../libqalculate/libqalculate/BuiltinFunctions-calculus.cc`
//! - `../libqalculate/data/functions.xml.in`

use crate::ast::Expression;
use crate::context::CalculatorContext;
use crate::functions::{
    make_unevaluated, push_warning, validate_arity, BuiltinFunction, BuiltinFunctionInfo,
    FunctionResult,
};

// ---------------------------------------------------------------------------
// Function info constants
// ---------------------------------------------------------------------------

static ABS_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "abs",
    aliases: &[],
    min_args: 1,
    max_args: Some(1),
    description: "Absolute value",
};

static SGN_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "sgn",
    aliases: &["signum", "sign"],
    min_args: 1,
    max_args: Some(1),
    description: "Sign (-1, 0, or 1)",
};

static ROUND_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "round",
    aliases: &[],
    min_args: 1,
    max_args: Some(1),
    description: "Round to nearest integer",
};

static FLOOR_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "floor",
    aliases: &["int"],
    min_args: 1,
    max_args: Some(1),
    description: "Round toward negative infinity",
};

static CEIL_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "ceil",
    aliases: &[],
    min_args: 1,
    max_args: Some(1),
    description: "Round toward positive infinity",
};

static TRUNC_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "trunc",
    aliases: &["truncate"],
    min_args: 1,
    max_args: Some(1),
    description: "Round toward zero",
};

static RE_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "re",
    aliases: &["real"],
    min_args: 1,
    max_args: Some(1),
    description: "Real part",
};

static IM_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "im",
    aliases: &["imag", "imaginary"],
    min_args: 1,
    max_args: Some(1),
    description: "Imaginary part",
};

static ARG_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "arg",
    aliases: &["argument"],
    min_args: 1,
    max_args: Some(1),
    description: "Complex argument (phase angle)",
};

static CONJ_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "conj",
    aliases: &["conjugate"],
    min_args: 1,
    max_args: Some(1),
    description: "Complex conjugate",
};

static GAMMA_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "gamma",
    aliases: &[],
    min_args: 1,
    max_args: Some(1),
    description: "Gamma function",
};

static ERF_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "erf",
    aliases: &[],
    min_args: 1,
    max_args: Some(1),
    description: "Error function",
};

static ZETA_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "zeta",
    aliases: &[],
    min_args: 1,
    max_args: Some(1),
    description: "Riemann zeta function",
};

static GAMMAINC_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "gammainc",
    aliases: &["igamma"],
    min_args: 2,
    max_args: Some(2),
    description: "Upper incomplete gamma function",
};

// ---------------------------------------------------------------------------
// Function implementations
// ---------------------------------------------------------------------------

/// `abs(x)` — Absolute value.
///
/// Upstream: `AbsFunction` in `BuiltinFunctions-number.cc`.
struct AbsFn;

impl BuiltinFunction for AbsFn {
    fn info(&self) -> &BuiltinFunctionInfo {
        &ABS_INFO
    }

    fn evaluate(&self, args: &[Expression], _context: &mut CalculatorContext) -> FunctionResult {
        validate_arity("abs", args, 1, Some(1))?;
        match &args[0] {
            Expression::Number(num) => Ok(Expression::Number(num.abs())),
            _ => Ok(make_unevaluated("abs", args)),
        }
    }
}

/// `sgn(x)` — Sign function.
///
/// Upstream: `SgnFunction` in `BuiltinFunctions-number.cc`.
/// Returns -1, 0, or 1 for real numbers. Undefined (NaN) for complex.
struct SgnFn;

impl BuiltinFunction for SgnFn {
    fn info(&self) -> &BuiltinFunctionInfo {
        &SGN_INFO
    }

    fn evaluate(&self, args: &[Expression], _context: &mut CalculatorContext) -> FunctionResult {
        validate_arity("sgn", args, 1, Some(1))?;
        match &args[0] {
            Expression::Number(num) => Ok(Expression::Number(num.signum())),
            _ => Ok(make_unevaluated("sgn", args)),
        }
    }
}

/// `round(x)` — Round to nearest integer.
///
/// Upstream: `RoundFunction` in `BuiltinFunctions-number.cc`.
/// Half values round away from zero.
struct RoundFn;

impl BuiltinFunction for RoundFn {
    fn info(&self) -> &BuiltinFunctionInfo {
        &ROUND_INFO
    }

    fn evaluate(&self, args: &[Expression], _context: &mut CalculatorContext) -> FunctionResult {
        validate_arity("round", args, 1, Some(1))?;
        match &args[0] {
            Expression::Number(num) => Ok(Expression::Number(num.round())),
            _ => Ok(make_unevaluated("round", args)),
        }
    }
}

/// `floor(x)` — Round toward negative infinity.
///
/// Upstream: `FloorFunction` in `BuiltinFunctions-number.cc`.
struct FloorFn;

impl BuiltinFunction for FloorFn {
    fn info(&self) -> &BuiltinFunctionInfo {
        &FLOOR_INFO
    }

    fn evaluate(&self, args: &[Expression], _context: &mut CalculatorContext) -> FunctionResult {
        validate_arity("floor", args, 1, Some(1))?;
        match &args[0] {
            Expression::Number(num) => Ok(Expression::Number(num.floor())),
            _ => Ok(make_unevaluated("floor", args)),
        }
    }
}

/// `ceil(x)` — Round toward positive infinity.
///
/// Upstream: `CeilFunction` in `BuiltinFunctions-number.cc`.
struct CeilFn;

impl BuiltinFunction for CeilFn {
    fn info(&self) -> &BuiltinFunctionInfo {
        &CEIL_INFO
    }

    fn evaluate(&self, args: &[Expression], _context: &mut CalculatorContext) -> FunctionResult {
        validate_arity("ceil", args, 1, Some(1))?;
        match &args[0] {
            Expression::Number(num) => Ok(Expression::Number(num.ceil())),
            _ => Ok(make_unevaluated("ceil", args)),
        }
    }
}

/// `trunc(x)` — Round toward zero (truncate).
///
/// Upstream: `TruncFunction` in `BuiltinFunctions-number.cc`.
struct TruncFn;

impl BuiltinFunction for TruncFn {
    fn info(&self) -> &BuiltinFunctionInfo {
        &TRUNC_INFO
    }

    fn evaluate(&self, args: &[Expression], _context: &mut CalculatorContext) -> FunctionResult {
        validate_arity("trunc", args, 1, Some(1))?;
        match &args[0] {
            Expression::Number(num) => Ok(Expression::Number(num.trunc())),
            _ => Ok(make_unevaluated("trunc", args)),
        }
    }
}

/// `re(z)` — Real part.
///
/// Upstream: `ReFunction` in `BuiltinFunctions-algebra.cc`.
struct ReFn;

impl BuiltinFunction for ReFn {
    fn info(&self) -> &BuiltinFunctionInfo {
        &RE_INFO
    }

    fn evaluate(&self, args: &[Expression], _context: &mut CalculatorContext) -> FunctionResult {
        validate_arity("re", args, 1, Some(1))?;
        match &args[0] {
            Expression::Number(num) => Ok(Expression::Number(num.real_part())),
            _ => Ok(make_unevaluated("re", args)),
        }
    }
}

/// `im(z)` — Imaginary part (coefficient).
///
/// Upstream: `ImFunction` in `BuiltinFunctions-algebra.cc`.
struct ImFn;

impl BuiltinFunction for ImFn {
    fn info(&self) -> &BuiltinFunctionInfo {
        &IM_INFO
    }

    fn evaluate(&self, args: &[Expression], _context: &mut CalculatorContext) -> FunctionResult {
        validate_arity("im", args, 1, Some(1))?;
        match &args[0] {
            Expression::Number(num) => Ok(Expression::Number(num.imaginary_part())),
            _ => Ok(make_unevaluated("im", args)),
        }
    }
}

/// `arg(z)` — Complex argument (phase angle in radians).
///
/// Upstream: `ArgFunction` in `BuiltinFunctions-algebra.cc`.
struct ArgFn;

impl BuiltinFunction for ArgFn {
    fn info(&self) -> &BuiltinFunctionInfo {
        &ARG_INFO
    }

    fn evaluate(&self, args: &[Expression], _context: &mut CalculatorContext) -> FunctionResult {
        validate_arity("arg", args, 1, Some(1))?;
        match &args[0] {
            Expression::Number(num) => Ok(Expression::Number(num.arg())),
            _ => Ok(make_unevaluated("arg", args)),
        }
    }
}

/// `conj(z)` — Complex conjugate.
///
/// Upstream: `ConjFunction` in `BuiltinFunctions-algebra.cc`.
struct ConjFn;

impl BuiltinFunction for ConjFn {
    fn info(&self) -> &BuiltinFunctionInfo {
        &CONJ_INFO
    }

    fn evaluate(&self, args: &[Expression], _context: &mut CalculatorContext) -> FunctionResult {
        validate_arity("conj", args, 1, Some(1))?;
        match &args[0] {
            Expression::Number(num) => Ok(Expression::Number(num.conjugate())),
            _ => Ok(make_unevaluated("conj", args)),
        }
    }
}

/// `gamma(x)` — Gamma function Γ(x).
///
/// Upstream: `GammaFunction` in `BuiltinFunctions-special.cc`.
/// Has poles at non-positive integers, which return NaN.
struct GammaFn;

impl BuiltinFunction for GammaFn {
    fn info(&self) -> &BuiltinFunctionInfo {
        &GAMMA_INFO
    }

    fn evaluate(&self, args: &[Expression], context: &mut CalculatorContext) -> FunctionResult {
        validate_arity("gamma", args, 1, Some(1))?;
        match &args[0] {
            Expression::Number(num) => {
                if num.is_complex() {
                    push_warning(context, "gamma", "complex arguments not supported");
                }
                let result = num.gamma();
                if result.is_nan() && !num.is_nan() {
                    push_warning(
                        context,
                        "gamma",
                        "gamma function has a pole at non-positive integers",
                    );
                }
                Ok(Expression::Number(result))
            }
            _ => Ok(make_unevaluated("gamma", args)),
        }
    }
}

/// `erf(x)` — Error function.
///
/// Upstream: `ErfFunction` in `BuiltinFunctions-special.cc`.
struct ErfFn;

impl BuiltinFunction for ErfFn {
    fn info(&self) -> &BuiltinFunctionInfo {
        &ERF_INFO
    }

    fn evaluate(&self, args: &[Expression], context: &mut CalculatorContext) -> FunctionResult {
        validate_arity("erf", args, 1, Some(1))?;
        match &args[0] {
            Expression::Number(num) => {
                if num.is_complex() {
                    push_warning(context, "erf", "complex arguments not supported");
                }
                Ok(Expression::Number(num.erf()))
            }
            _ => Ok(make_unevaluated("erf", args)),
        }
    }
}

/// `zeta(x)` — Riemann zeta function ζ(x).
///
/// Upstream: `ZetaFunction` in `BuiltinFunctions-special.cc`.
struct ZetaFn;

impl BuiltinFunction for ZetaFn {
    fn info(&self) -> &BuiltinFunctionInfo {
        &ZETA_INFO
    }

    fn evaluate(&self, args: &[Expression], context: &mut CalculatorContext) -> FunctionResult {
        validate_arity("zeta", args, 1, Some(1))?;
        match &args[0] {
            Expression::Number(num) => {
                if num.is_complex() {
                    push_warning(context, "zeta", "complex arguments not supported");
                }
                Ok(Expression::Number(num.zeta()))
            }
            _ => Ok(make_unevaluated("zeta", args)),
        }
    }
}

/// `gammainc(a, x)` — Upper incomplete gamma function Γ(a, x).
///
/// Upstream: `GammaIncFunction` in `BuiltinFunctions-calculus.cc`.
struct GammaIncFn;

impl BuiltinFunction for GammaIncFn {
    fn info(&self) -> &BuiltinFunctionInfo {
        &GAMMAINC_INFO
    }

    fn evaluate(&self, args: &[Expression], context: &mut CalculatorContext) -> FunctionResult {
        validate_arity("gammainc", args, 2, Some(2))?;
        match (&args[0], &args[1]) {
            (Expression::Number(a), Expression::Number(x)) => {
                if a.is_complex() || x.is_complex() {
                    push_warning(context, "gammainc", "complex arguments not supported");
                }
                Ok(Expression::Number(a.gamma_inc(x)))
            }
            _ => Ok(make_unevaluated("gammainc", args)),
        }
    }
}

// ---------------------------------------------------------------------------
// Catalog and lookup
// ---------------------------------------------------------------------------

static CATALOG: &[&BuiltinFunctionInfo] = &[
    &ABS_INFO,
    &SGN_INFO,
    &ROUND_INFO,
    &FLOOR_INFO,
    &CEIL_INFO,
    &TRUNC_INFO,
    &RE_INFO,
    &IM_INFO,
    &ARG_INFO,
    &CONJ_INFO,
    &GAMMA_INFO,
    &ERF_INFO,
    &ZETA_INFO,
    &GAMMAINC_INFO,
];

/// Returns all algebra/number/special function infos.
pub fn catalog() -> Vec<&'static BuiltinFunctionInfo> {
    CATALOG.to_vec()
}

/// Looks up a built-in algebra/number/special function by name (including aliases).
pub fn lookup(name: &str) -> Option<&'static dyn BuiltinFunction> {
    match name {
        "abs" => Some(&AbsFn),
        "sgn" | "signum" | "sign" => Some(&SgnFn),
        "round" => Some(&RoundFn),
        "floor" | "int" => Some(&FloorFn),
        "ceil" => Some(&CeilFn),
        "trunc" | "truncate" => Some(&TruncFn),
        "re" | "real" => Some(&ReFn),
        "im" | "imag" | "imaginary" => Some(&ImFn),
        "arg" | "argument" => Some(&ArgFn),
        "conj" | "conjugate" => Some(&ConjFn),
        "gamma" => Some(&GammaFn),
        "erf" => Some(&ErfFn),
        "zeta" => Some(&ZetaFn),
        "gammainc" | "igamma" => Some(&GammaIncFn),
        _ => None,
    }
}
#[cfg(test)]
mod tests;
