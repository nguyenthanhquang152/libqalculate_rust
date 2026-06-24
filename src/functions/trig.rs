//! Trigonometric and hyperbolic built-in function family.
//!
//! # Upstream oracle
//! - `../libqalculate/libqalculate/BuiltinFunctions-trigonometry.cc`
//! - `../libqalculate/libqalculate/BuiltinFunctions.h`
//! - `../libqalculate/data/functions.xml.in` (sin, cos, tan, asin, acos, atan, atan2,
//!   sinh, cosh, tanh, asinh, acosh, atanh, sinc, csc, sec, cot, csch, sech, coth,
//!   acsc, asec, acot, acsch, asech, acoth, radtodef)
//! - Indirect trig coverage in `../libqalculate/tests/solver.batch`,
//!   `../libqalculate/tests/limits.batch`, `../libqalculate/tests/calculus.batch`

use crate::ast::Expression;
use crate::context::CalculatorContext;
use crate::functions::{BuiltinFunction, BuiltinFunctionInfo, FunctionError, FunctionResult};
use crate::number::{Number, NumberValue, Float};

// ---------------------------------------------------------------------------
// Function info constants
// ---------------------------------------------------------------------------

static SIN_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "sin",
    aliases: &[],
    min_args: 1,
    max_args: Some(1),
    description: "Sine",
};

static COS_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "cos",
    aliases: &[],
    min_args: 1,
    max_args: Some(1),
    description: "Cosine",
};

static TAN_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "tan",
    aliases: &[],
    min_args: 1,
    max_args: Some(1),
    description: "Tangent",
};

static ASIN_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "asin",
    aliases: &["arcsin"],
    min_args: 1,
    max_args: Some(1),
    description: "Inverse sine",
};

static ACOS_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "acos",
    aliases: &["arccos"],
    min_args: 1,
    max_args: Some(1),
    description: "Inverse cosine",
};

static ATAN_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "atan",
    aliases: &["arctan"],
    min_args: 1,
    max_args: Some(1),
    description: "Inverse tangent",
};

static ATAN2_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "atan2",
    aliases: &[],
    min_args: 2,
    max_args: Some(2),
    description: "Two-argument inverse tangent",
};

static SINH_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "sinh",
    aliases: &[],
    min_args: 1,
    max_args: Some(1),
    description: "Hyperbolic sine",
};

static COSH_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "cosh",
    aliases: &[],
    min_args: 1,
    max_args: Some(1),
    description: "Hyperbolic cosine",
};

static TANH_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "tanh",
    aliases: &[],
    min_args: 1,
    max_args: Some(1),
    description: "Hyperbolic tangent",
};

static ASINH_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "asinh",
    aliases: &["arcsinh"],
    min_args: 1,
    max_args: Some(1),
    description: "Inverse hyperbolic sine",
};

static ACOSH_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "acosh",
    aliases: &["arccosh"],
    min_args: 1,
    max_args: Some(1),
    description: "Inverse hyperbolic cosine",
};

static ATANH_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "atanh",
    aliases: &["arctanh"],
    min_args: 1,
    max_args: Some(1),
    description: "Inverse hyperbolic tangent",
};

static SINC_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "sinc",
    aliases: &[],
    min_args: 1,
    max_args: Some(1),
    description: "Sinc function (sin(x)/x)",
};

static CSC_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "csc",
    aliases: &["cosec"],
    min_args: 1,
    max_args: Some(1),
    description: "Cosecant (1/sin)",
};

static SEC_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "sec",
    aliases: &[],
    min_args: 1,
    max_args: Some(1),
    description: "Secant (1/cos)",
};

static COT_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "cot",
    aliases: &[],
    min_args: 1,
    max_args: Some(1),
    description: "Cotangent (cos/sin)",
};

static CSCH_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "csch",
    aliases: &["cosech"],
    min_args: 1,
    max_args: Some(1),
    description: "Hyperbolic cosecant (1/sinh)",
};

static SECH_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "sech",
    aliases: &[],
    min_args: 1,
    max_args: Some(1),
    description: "Hyperbolic secant (1/cosh)",
};

static COTH_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "coth",
    aliases: &[],
    min_args: 1,
    max_args: Some(1),
    description: "Hyperbolic cotangent (cosh/sinh)",
};

static ACSC_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "acsc",
    aliases: &["arccsc", "arccosec"],
    min_args: 1,
    max_args: Some(1),
    description: "Inverse cosecant",
};

static ASEC_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "asec",
    aliases: &["arcsec"],
    min_args: 1,
    max_args: Some(1),
    description: "Inverse secant",
};

static ACOT_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "acot",
    aliases: &["arccot"],
    min_args: 1,
    max_args: Some(1),
    description: "Inverse cotangent",
};

static ACSCH_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "acsch",
    aliases: &["arccsch"],
    min_args: 1,
    max_args: Some(1),
    description: "Inverse hyperbolic cosecant",
};

static ASECH_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "asech",
    aliases: &["arcsech"],
    min_args: 1,
    max_args: Some(1),
    description: "Inverse hyperbolic secant",
};

static ACOTH_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "acoth",
    aliases: &["arccoth"],
    min_args: 1,
    max_args: Some(1),
    description: "Inverse hyperbolic cotangent",
};

// ---------------------------------------------------------------------------
// Catalog
// ---------------------------------------------------------------------------

static CATALOG: &[&BuiltinFunctionInfo] = &[
    &SIN_INFO, &COS_INFO, &TAN_INFO,
    &ASIN_INFO, &ACOS_INFO, &ATAN_INFO, &ATAN2_INFO,
    &SINH_INFO, &COSH_INFO, &TANH_INFO,
    &ASINH_INFO, &ACOSH_INFO, &ATANH_INFO,
    &SINC_INFO,
    &CSC_INFO, &SEC_INFO, &COT_INFO,
    &CSCH_INFO, &SECH_INFO, &COTH_INFO,
    &ACSC_INFO, &ASEC_INFO, &ACOT_INFO,
    &ACSCH_INFO, &ASECH_INFO, &ACOTH_INFO,
];

/// Returns all registered trig function infos.
pub fn catalog() -> Vec<&'static BuiltinFunctionInfo> {
    CATALOG.to_vec()
}

// ---------------------------------------------------------------------------
// TrigOp Enum and Parametric TrigFn Implementation
// ---------------------------------------------------------------------------

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum TrigOp {
    Sin, Cos, Tan,
    Asin, Acos, Atan, Atan2,
    Sinh, Cosh, Tanh,
    Asinh, Acosh, Atanh,
    Sinc,
    Csc, Sec, Cot,
    Csch, Sech, Coth,
    Acsc, Asec, Acot,
    Acsch, Asech, Acoth,
}

impl TrigOp {
    fn info(self) -> &'static BuiltinFunctionInfo {
        match self {
            TrigOp::Sin => &SIN_INFO,
            TrigOp::Cos => &COS_INFO,
            TrigOp::Tan => &TAN_INFO,
            TrigOp::Asin => &ASIN_INFO,
            TrigOp::Acos => &ACOS_INFO,
            TrigOp::Atan => &ATAN_INFO,
            TrigOp::Atan2 => &ATAN2_INFO,
            TrigOp::Sinh => &SINH_INFO,
            TrigOp::Cosh => &COSH_INFO,
            TrigOp::Tanh => &TANH_INFO,
            TrigOp::Asinh => &ASINH_INFO,
            TrigOp::Acosh => &ACOSH_INFO,
            TrigOp::Atanh => &ATANH_INFO,
            TrigOp::Sinc => &SINC_INFO,
            TrigOp::Csc => &CSC_INFO,
            TrigOp::Sec => &SEC_INFO,
            TrigOp::Cot => &COT_INFO,
            TrigOp::Csch => &CSCH_INFO,
            TrigOp::Sech => &SECH_INFO,
            TrigOp::Coth => &COTH_INFO,
            TrigOp::Acsc => &ACSC_INFO,
            TrigOp::Asec => &ASEC_INFO,
            TrigOp::Acot => &ACOT_INFO,
            TrigOp::Acsch => &ACSCH_INFO,
            TrigOp::Asech => &ASECH_INFO,
            TrigOp::Acoth => &ACOTH_INFO,
        }
    }
}

struct TrigFn(TrigOp);

impl BuiltinFunction for TrigFn {
    fn info(&self) -> &BuiltinFunctionInfo {
        self.0.info()
    }

    fn evaluate(&self, args: &[Expression], context: &mut CalculatorContext) -> FunctionResult {
        let op = self.0;
        let info = op.info();
        validate_arity(info.name, args, info.min_args, info.max_args)?;

        // Special handling for two-argument functions (atan2)
        if op == TrigOp::Atan2 {
            match (&args[0], &args[1]) {
                (Expression::Number(y), Expression::Number(x)) => {
                    let prec = context.min_precision_bits();
                    let result = Number::atan2_with_prec(y, x, prec);
                    if result.is_nan() && y.is_zero() && x.is_zero() {
                        push_warning(context, "atan2", "atan2(0, 0) is undefined");
                    }
                    return Ok(Expression::Number(result));
                }
                _ => return Ok(make_unevaluated(info.name, args)),
            }
        }

        // For all other (unary) functions:
        match &args[0] {
            Expression::Number(num) => {
                let prec = context.min_precision_bits();
                let res = match op {
                    TrigOp::Sin => num.sin_with_prec(prec),
                    TrigOp::Cos => num.cos_with_prec(prec),
                    TrigOp::Tan => num.tan_with_prec(prec),
                    TrigOp::Asin => {
                        let r = num.asin_with_prec(prec);
                        if r.is_nan() && !num.is_nan() {
                            push_warning(context, "asin", "argument outside domain [-1, 1]");
                        }
                        r
                    }
                    TrigOp::Acos => {
                        let r = num.acos_with_prec(prec);
                        if r.is_nan() && !num.is_nan() {
                            push_warning(context, "acos", "argument outside domain [-1, 1]");
                        }
                        r
                    }
                    TrigOp::Atan => num.atan_with_prec(prec),
                    TrigOp::Sinh => num.sinh_with_prec(prec),
                    TrigOp::Cosh => num.cosh_with_prec(prec),
                    TrigOp::Tanh => num.tanh_with_prec(prec),
                    TrigOp::Asinh => num.asinh_with_prec(prec),
                    TrigOp::Acosh => {
                        let r = num.acosh_with_prec(prec);
                        if r.is_nan() && !num.is_nan() {
                            push_warning(context, "acosh", "argument must be >= 1");
                        }
                        r
                    }
                    TrigOp::Atanh => {
                        let r = num.atanh_with_prec(prec);
                        if r.is_nan() && !num.is_nan() {
                            push_warning(context, "atanh", "argument outside domain (-1, 1)");
                        }
                        r
                    }
                    TrigOp::Sinc => {
                        if num.is_zero() {
                            Number::one()
                        } else {
                            num.sin_with_prec(prec).div(num)
                        }
                    }
                    TrigOp::Csc => {
                        let sin_val = num.sin_with_prec(prec);
                        if sin_val.is_zero() {
                            push_warning(context, "csc", "division by zero (sin(x) = 0)");
                            Number::nan()
                        } else {
                            Number::one().div(&sin_val)
                        }
                    }
                    TrigOp::Sec => {
                        let cos_val = num.cos_with_prec(prec);
                        if cos_val.is_zero() {
                            push_warning(context, "sec", "division by zero (cos(x) = 0)");
                            Number::nan()
                        } else {
                            Number::one().div(&cos_val)
                        }
                    }
                    TrigOp::Cot => {
                        let sin_val = num.sin_with_prec(prec);
                        if sin_val.is_zero() {
                            push_warning(context, "cot", "division by zero (sin(x) = 0)");
                            Number::nan()
                        } else {
                            num.cos_with_prec(prec).div(&sin_val)
                        }
                    }
                    TrigOp::Csch => {
                        let sinh_val = num.sinh_with_prec(prec);
                        if sinh_val.is_zero() {
                            push_warning(context, "csch", "division by zero (sinh(0) = 0)");
                            Number::nan()
                        } else {
                            Number::one().div(&sinh_val)
                        }
                    }
                    TrigOp::Sech => {
                        let cosh_val = num.cosh_with_prec(prec);
                        Number::one().div(&cosh_val)
                    }
                    TrigOp::Coth => {
                        let sinh_val = num.sinh_with_prec(prec);
                        if sinh_val.is_zero() {
                            push_warning(context, "coth", "division by zero (sinh(0) = 0)");
                            Number::nan()
                        } else {
                            num.cosh_with_prec(prec).div(&sinh_val)
                        }
                    }
                    TrigOp::Acsc => {
                        if num.is_zero() {
                            push_warning(context, "acsc", "division by zero");
                            Number::nan()
                        } else {
                            let recip = Number::one().div(num);
                            let r = recip.asin_with_prec(prec);
                            if r.is_nan() && !num.is_nan() {
                                push_warning(context, "acsc", "argument outside domain |x| >= 1");
                            }
                            r
                        }
                    }
                    TrigOp::Asec => {
                        if num.is_zero() {
                            push_warning(context, "asec", "division by zero");
                            Number::nan()
                        } else {
                            let recip = Number::one().div(num);
                            let r = recip.acos_with_prec(prec);
                            if r.is_nan() && !num.is_nan() {
                                push_warning(context, "asec", "argument outside domain |x| >= 1");
                            }
                            r
                        }
                    }
                    TrigOp::Acot => {
                        if num.is_zero() {
                            let pi_val = rug::Float::with_val(prec.max(53), rug::float::Constant::Pi);
                            let half_pi = Number::from_real_value(NumberValue::Float(Float::from_rug_float(
                                rug::Float::with_val(prec.max(53), pi_val / 2u32),
                            )));
                            half_pi
                        } else {
                            let recip = Number::one().div(num);
                            recip.atan_with_prec(prec)
                        }
                    }
                    TrigOp::Acsch => {
                        if num.is_zero() {
                            Number::plus_infinity()
                        } else {
                            let recip = Number::one().div(num);
                            recip.asinh_with_prec(prec)
                        }
                    }
                    TrigOp::Asech => {
                        if num.is_zero() {
                            push_warning(context, "asech", "division by zero");
                            Number::nan()
                        } else {
                            let recip = Number::one().div(num);
                            let r = recip.acosh_with_prec(prec);
                            if r.is_nan() && !num.is_nan() {
                                push_warning(context, "asech", "argument outside domain (0, 1]");
                            }
                            r
                        }
                    }
                    TrigOp::Acoth => {
                        if num.is_zero() {
                            let pi_val = rug::Float::with_val(prec.max(53), rug::float::Constant::Pi);
                            let half_pi = Number::from_real_value(NumberValue::Float(Float::from_rug_float(
                                rug::Float::with_val(prec.max(53), pi_val / 2u32),
                            )));
                            Number::new_complex(Number::from_i64(0), half_pi)
                        } else {
                            let recip = Number::one().div(num);
                            let r = recip.atanh_with_prec(prec);
                            if r.is_nan() && !num.is_nan() {
                                push_warning(context, "acoth", "argument outside domain |x| > 1");
                            }
                            r
                        }
                    }
                    TrigOp::Atan2 => unreachable!(),
                };
                Ok(Expression::Number(res))
            }
            _ => Ok(make_unevaluated(info.name, args)),
        }
    }
}

// ---------------------------------------------------------------------------
// Static Fn Instantiations
// ---------------------------------------------------------------------------

static SIN_FN: TrigFn = TrigFn(TrigOp::Sin);
static COS_FN: TrigFn = TrigFn(TrigOp::Cos);
static TAN_FN: TrigFn = TrigFn(TrigOp::Tan);
static ASIN_FN: TrigFn = TrigFn(TrigOp::Asin);
static ACOS_FN: TrigFn = TrigFn(TrigOp::Acos);
static ATAN_FN: TrigFn = TrigFn(TrigOp::Atan);
static ATAN2_FN: TrigFn = TrigFn(TrigOp::Atan2);
static SINH_FN: TrigFn = TrigFn(TrigOp::Sinh);
static COSH_FN: TrigFn = TrigFn(TrigOp::Cosh);
static TANH_FN: TrigFn = TrigFn(TrigOp::Tanh);
static ASINH_FN: TrigFn = TrigFn(TrigOp::Asinh);
static ACOSH_FN: TrigFn = TrigFn(TrigOp::Acosh);
static ATANH_FN: TrigFn = TrigFn(TrigOp::Atanh);
static SINC_FN: TrigFn = TrigFn(TrigOp::Sinc);
static CSC_FN: TrigFn = TrigFn(TrigOp::Csc);
static SEC_FN: TrigFn = TrigFn(TrigOp::Sec);
static COT_FN: TrigFn = TrigFn(TrigOp::Cot);
static CSCH_FN: TrigFn = TrigFn(TrigOp::Csch);
static SECH_FN: TrigFn = TrigFn(TrigOp::Sech);
static COTH_FN: TrigFn = TrigFn(TrigOp::Coth);
static ACSC_FN: TrigFn = TrigFn(TrigOp::Acsc);
static ASEC_FN: TrigFn = TrigFn(TrigOp::Asec);
static ACOT_FN: TrigFn = TrigFn(TrigOp::Acot);
static ACSCH_FN: TrigFn = TrigFn(TrigOp::Acsch);
static ASECH_FN: TrigFn = TrigFn(TrigOp::Asech);
static ACOTH_FN: TrigFn = TrigFn(TrigOp::Acoth);

/// Looks up a built-in trig function by name (including aliases).
pub fn lookup(name: &str) -> Option<&'static dyn BuiltinFunction> {
    match name {
        "sin" => Some(&SIN_FN),
        "cos" => Some(&COS_FN),
        "tan" => Some(&TAN_FN),
        "asin" | "arcsin" => Some(&ASIN_FN),
        "acos" | "arccos" => Some(&ACOS_FN),
        "atan" | "arctan" => Some(&ATAN_FN),
        "atan2" => Some(&ATAN2_FN),
        "sinh" => Some(&SINH_FN),
        "cosh" => Some(&COSH_FN),
        "tanh" => Some(&TANH_FN),
        "asinh" | "arcsinh" => Some(&ASINH_FN),
        "acosh" | "arccosh" => Some(&ACOSH_FN),
        "atanh" | "arctanh" => Some(&ATANH_FN),
        "sinc" => Some(&SINC_FN),
        "csc" | "cosec" => Some(&CSC_FN),
        "sec" => Some(&SEC_FN),
        "cot" => Some(&COT_FN),
        "csch" | "cosech" => Some(&CSCH_FN),
        "sech" => Some(&SECH_FN),
        "coth" => Some(&COTH_FN),
        "acsc" | "arccsc" | "arccosec" => Some(&ACSC_FN),
        "asec" | "arcsec" => Some(&ASEC_FN),
        "acot" | "arccot" => Some(&ACOT_FN),
        "acsch" | "arccsch" => Some(&ACSCH_FN),
        "asech" | "arcsech" => Some(&ASECH_FN),
        "acoth" | "arccoth" => Some(&ACOTH_FN),
        _ => None,
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::CalculatorContext;

    fn make_ctx() -> CalculatorContext {
        CalculatorContext::default()
    }

    fn eval_num(func: &dyn BuiltinFunction, num: Number) -> Number {
        let mut ctx = make_ctx();
        match func.evaluate(&[Expression::Number(num)], &mut ctx).unwrap() {
            Expression::Number(n) => n,
            other => panic!("Expected Number, got {:?}", other),
        }
    }

    fn assert_approx(actual: &Number, expected: f64, tol: f64, label: &str) {
        let s = actual.to_qalc_string_with_precision(20);
        let val: f64 = s.parse().unwrap_or_else(|_| panic!("{label}: could not parse '{s}' as f64"));
        assert!(
            (val - expected).abs() < tol,
            "{label}: expected ≈{expected}, got {val} (string: {s})"
        );
    }

    // -----------------------------------------------------------------------
    // Lookup tests
    // -----------------------------------------------------------------------

    #[test]
    fn lookup_finds_sin() {
        assert!(lookup("sin").is_some());
    }

    #[test]
    fn lookup_finds_arcsin_alias() {
        assert!(lookup("arcsin").is_some());
        assert_eq!(lookup("arcsin").unwrap().info().name, "asin");
    }

    #[test]
    fn lookup_finds_cos() {
        assert!(lookup("cos").is_some());
    }

    #[test]
    fn lookup_finds_tan() {
        assert!(lookup("tan").is_some());
    }

    #[test]
    fn lookup_finds_atan2() {
        assert!(lookup("atan2").is_some());
    }

    #[test]
    fn lookup_finds_hyperbolic() {
        for name in &["sinh", "cosh", "tanh", "asinh", "acosh", "atanh"] {
            assert!(lookup(name).is_some(), "should find {name}");
        }
    }

    #[test]
    fn lookup_finds_reciprocal() {
        for name in &["csc", "sec", "cot", "csch", "sech", "coth"] {
            assert!(lookup(name).is_some(), "should find {name}");
        }
    }

    #[test]
    fn lookup_finds_inverse_reciprocal() {
        for name in &["acsc", "asec", "acot", "acsch", "asech", "acoth"] {
            assert!(lookup(name).is_some(), "should find {name}");
        }
    }

    #[test]
    fn lookup_returns_none_for_unknown() {
        assert!(lookup("notrig").is_none());
    }

    // -----------------------------------------------------------------------
    // sin/cos/tan basic
    // -----------------------------------------------------------------------

    #[test]
    fn sin_of_zero_returns_zero() {
        let result = eval_num(lookup("sin").unwrap(), Number::from_i64(0));
        assert!(result.is_zero(), "sin(0) should be 0, got {}", result.to_qalc_string());
    }

    #[test]
    fn cos_of_zero_returns_one() {
        let result = eval_num(lookup("cos").unwrap(), Number::from_i64(0));
        assert_approx(&result, 1.0, 1e-10, "cos(0)");
    }

    #[test]
    fn tan_of_zero_returns_zero() {
        let result = eval_num(lookup("tan").unwrap(), Number::from_i64(0));
        assert_approx(&result, 0.0, 1e-10, "tan(0)");
    }

    #[test]
    fn sin_of_pi_half_returns_one() {
        let pi = Number::pi();
        let half_pi = pi.div(&Number::from_i64(2));
        let result = eval_num(lookup("sin").unwrap(), half_pi);
        assert_approx(&result, 1.0, 1e-10, "sin(π/2)");
    }

    // -----------------------------------------------------------------------
    // Inverse trig
    // -----------------------------------------------------------------------

    #[test]
    fn asin_of_one_returns_pi_half() {
        let result = eval_num(lookup("asin").unwrap(), Number::one());
        let expected = std::f64::consts::FRAC_PI_2;
        assert_approx(&result, expected, 1e-10, "asin(1)");
    }

    #[test]
    fn acos_of_one_returns_zero() {
        let result = eval_num(lookup("acos").unwrap(), Number::one());
        assert_approx(&result, 0.0, 1e-10, "acos(1)");
    }

    #[test]
    fn atan_of_one_returns_pi_quarter() {
        let result = eval_num(lookup("atan").unwrap(), Number::one());
        let expected = std::f64::consts::FRAC_PI_4;
        assert_approx(&result, expected, 1e-10, "atan(1)");
    }

    #[test]
    fn atan2_one_one_returns_pi_quarter() {
        let mut ctx = make_ctx();
        let f = lookup("atan2").unwrap();
        let result = f
            .evaluate(
                &[
                    Expression::Number(Number::one()),
                    Expression::Number(Number::one()),
                ],
                &mut ctx,
            )
            .unwrap();
        match result {
            Expression::Number(n) => {
                assert_approx(&n, std::f64::consts::FRAC_PI_4, 1e-10, "atan2(1,1)");
            }
            _ => panic!("Expected Number"),
        }
    }

    // -----------------------------------------------------------------------
    // Hyperbolic
    // -----------------------------------------------------------------------

    #[test]
    fn sinh_of_zero_returns_zero() {
        let result = eval_num(lookup("sinh").unwrap(), Number::from_i64(0));
        assert_approx(&result, 0.0, 1e-10, "sinh(0)");
    }

    #[test]
    fn cosh_of_zero_returns_one() {
        let result = eval_num(lookup("cosh").unwrap(), Number::from_i64(0));
        assert_approx(&result, 1.0, 1e-10, "cosh(0)");
    }

    #[test]
    fn tanh_of_zero_returns_zero() {
        let result = eval_num(lookup("tanh").unwrap(), Number::from_i64(0));
        assert_approx(&result, 0.0, 1e-10, "tanh(0)");
    }

    #[test]
    fn atanh_of_zero_returns_zero() {
        let result = eval_num(lookup("atanh").unwrap(), Number::from_i64(0));
        assert_approx(&result, 0.0, 1e-10, "atanh(0)");
    }

    // -----------------------------------------------------------------------
    // Sinc
    // -----------------------------------------------------------------------

    #[test]
    fn sinc_of_zero_returns_one() {
        let result = eval_num(lookup("sinc").unwrap(), Number::from_i64(0));
        assert_approx(&result, 1.0, 1e-10, "sinc(0)");
    }

    #[test]
    fn sinc_of_pi_returns_zero() {
        let result = eval_num(lookup("sinc").unwrap(), Number::pi());
        assert_approx(&result, 0.0, 1e-10, "sinc(π)");
    }

    // -----------------------------------------------------------------------
    // Reciprocal trig edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn csc_of_pi_half_returns_one() {
        let half_pi = Number::pi().div(&Number::from_i64(2));
        let result = eval_num(lookup("csc").unwrap(), half_pi);
        assert_approx(&result, 1.0, 1e-10, "csc(π/2)");
    }

    #[test]
    fn sec_of_zero_returns_one() {
        let result = eval_num(lookup("sec").unwrap(), Number::from_i64(0));
        assert_approx(&result, 1.0, 1e-10, "sec(0)");
    }

    #[test]
    fn sech_of_zero_returns_one() {
        let result = eval_num(lookup("sech").unwrap(), Number::from_i64(0));
        assert_approx(&result, 1.0, 1e-10, "sech(0)");
    }

    // -----------------------------------------------------------------------
    // Arity validation
    // -----------------------------------------------------------------------

    #[test]
    fn sin_rejects_two_args() {
        let mut ctx = make_ctx();
        let f = lookup("sin").unwrap();
        let result = f.evaluate(
            &[
                Expression::Number(Number::from_i64(0)),
                Expression::Number(Number::one()),
            ],
            &mut ctx,
        );
        assert!(result.is_err());
    }

    #[test]
    fn atan2_rejects_one_arg() {
        let mut ctx = make_ctx();
        let f = lookup("atan2").unwrap();
        let result = f.evaluate(&[Expression::Number(Number::one())], &mut ctx);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Domain warnings
    // -----------------------------------------------------------------------

    #[test]
    fn asin_of_2_warns_out_of_domain() {
        let mut ctx = make_ctx();
        let f = lookup("asin").unwrap();
        let _result = f.evaluate(&[Expression::Number(Number::from_i64(2))], &mut ctx);
        assert!(!ctx.messages.is_empty(), "asin(2) should produce a warning");
    }

    #[test]
    fn acosh_of_half_warns_out_of_domain() {
        let mut ctx = make_ctx();
        let f = lookup("acosh").unwrap();
        let half = Number::from_rational(crate::number::Rational {
            value: rug::Rational::from((1, 2)),
        });
        let _result = f.evaluate(&[Expression::Number(half)], &mut ctx);
        assert!(!ctx.messages.is_empty(), "acosh(0.5) should produce a warning");
    }

    // -----------------------------------------------------------------------
    // Inverse reciprocal basic
    // -----------------------------------------------------------------------

    #[test]
    fn acot_of_zero_returns_pi_half() {
        let result = eval_num(lookup("acot").unwrap(), Number::from_i64(0));
        let expected = std::f64::consts::FRAC_PI_2;
        assert_approx(&result, expected, 1e-10, "acot(0)");
    }

    #[test]
    fn acot_of_one_returns_pi_quarter() {
        let result = eval_num(lookup("acot").unwrap(), Number::one());
        let expected = std::f64::consts::FRAC_PI_4;
        assert_approx(&result, expected, 1e-10, "acot(1)");
    }

    // -----------------------------------------------------------------------
    // Edge cases for acoth(0) and acsch(0)
    // -----------------------------------------------------------------------

    #[test]
    fn acoth_of_zero_returns_i_pi_half() {
        let result = eval_num(lookup("acoth").unwrap(), Number::from_i64(0));
        // acoth(0) = i * pi / 2
        assert!(result.has_imaginary_part(), "acoth(0) should be complex");
        let (real, imag) = result.to_canonical_real_imag();
        assert!(real.is_real_zero(), "real part should be zero");
        
        let pi = Number::pi();
        let expected_imag = pi.div(&Number::from_i64(2));
        let diff = Number::from_real_value(imag).sub(&expected_imag);
        assert_approx(&diff, 0.0, 1e-10, "imaginary part difference");
    }

    #[test]
    fn acsch_of_zero_returns_plus_infinity() {
        let result = eval_num(lookup("acsch").unwrap(), Number::from_i64(0));
        assert!(result.is_infinite(), "acsch(0) should be infinite");
        let is_positive = match result.value() {
            crate::number::NumberValue::PlusInfinity => true,
            _ => false,
        };
        assert!(is_positive, "acsch(0) should be plus_infinity");
    }

    // -----------------------------------------------------------------------
    // Catalog coverage
    // -----------------------------------------------------------------------

    #[test]
    fn catalog_contains_26_functions() {
        let cat = catalog();
        assert_eq!(cat.len(), 26, "should have 26 trig functions");
    }

    #[test]
    fn test_trig_intervals() {
        use crate::number::NumberValue;

        // acos([0, 1]) -> [0, pi/2]
        let interval = Number::from_real_value(NumberValue::Interval {
            lower: crate::number::Float::from_f64(0.0, 53),
            upper: crate::number::Float::from_f64(1.0, 53),
        });
        let res = eval_num(lookup("acos").unwrap(), interval);
        match res.value() {
            NumberValue::Interval { lower, upper } => {
                assert_approx(&Number::from_real_value(NumberValue::Float(lower.clone())), 0.0, 1e-10, "acos([0,1]) lower");
                assert_approx(&Number::from_real_value(NumberValue::Float(upper.clone())), std::f64::consts::FRAC_PI_2, 1e-10, "acos([0,1]) upper");
            }
            other => panic!("Expected Interval, got {:?}", other),
        }

        // cosh([-1, 1]) -> [1.0, cosh(1.0)]
        let interval_cosh = Number::from_real_value(NumberValue::Interval {
            lower: crate::number::Float::from_f64(-1.0, 53),
            upper: crate::number::Float::from_f64(1.0, 53),
        });
        let res_cosh = eval_num(lookup("cosh").unwrap(), interval_cosh);
        match res_cosh.value() {
            NumberValue::Interval { lower, upper } => {
                assert_approx(&Number::from_real_value(NumberValue::Float(lower.clone())), 1.0, 1e-10, "cosh([-1,1]) lower");
                assert_approx(&Number::from_real_value(NumberValue::Float(upper.clone())), 1.5430806348152437, 1e-10, "cosh([-1,1]) upper");
            }
            other => panic!("Expected Interval, got {:?}", other),
        }

        // tan([0, pi]) -> NaN (since it crosses pi/2)
        let interval_tan = Number::from_real_value(NumberValue::Interval {
            lower: crate::number::Float::from_f64(0.0, 53),
            upper: crate::number::Float::from_f64(3.141592653589793, 53),
        });
        let res_tan = eval_num(lookup("tan").unwrap(), interval_tan);
        assert!(res_tan.is_nan(), "tan([0, pi]) should be NaN due to pole crossing");
    }
}
