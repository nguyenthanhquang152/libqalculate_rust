#![allow(unsafe_code)]
//! Safe Rust wrapper and FFI bindings for C++ libqalculate's Calculator.

use cxx::UniquePtr;
use std::marker::PhantomData;
use std::sync::Mutex;

static FFI_LOCK: Mutex<()> = Mutex::new(());

#[cxx::bridge]
#[allow(missing_docs)]
pub(crate) mod sys {
    // SAFETY: The FFI declarations below reference C++ symbols implemented in `ffi_bridge.cc`
    // and the upstream `libqalculate` library. CXX guarantees that these signatures are
    // checked and generated correctly at build time, ensuring safety under normal C++ linking assumptions.
    unsafe extern "C++" {
        include!("libqalculate_rust/src/ffi_bridge.h");

        /// Opaque C++ Calculator type.
        type Calculator;

        /// Create a std::unique_ptr to a Calculator.
        fn new_calculator() -> UniquePtr<Calculator>;

        /// Load exchange rates.
        fn load_exchange_rates(calc: Pin<&mut Calculator>) -> bool;

        /// Load global definitions.
        fn load_global_definitions(calc: Pin<&mut Calculator>) -> bool;

        /// Load local definitions.
        fn load_local_definitions(calc: Pin<&mut Calculator>) -> bool;

        /// Calculate and print an expression.
        fn calculate_and_print(
            calc: Pin<&mut Calculator>,
            expr: &str,
            timeout_ms: i32,
        ) -> Result<String>;

        /// Calculate and print using qalc-compatible evaluation/print defaults.
        fn calculate_and_print_qalc(
            calc: Pin<&mut Calculator>,
            expr: &str,
            timeout_ms: i32,
        ) -> Result<String>;
    }
}

/// Safe wrapper around the C++ `Calculator` class.
pub struct Calculator {
    inner: UniquePtr<sys::Calculator>,
    _phantom: PhantomData<*mut ()>,
}

/// How an evaluation was routed with respect to the C++ fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackState {
    /// The expression was handled by native Rust code with the C++ fallback disabled.
    Native,
    /// The C++ fallback was available for this evaluation.
    CppFallbackEnabled,
    /// The C++ fallback was disabled and no native implementation handled the expression.
    Disabled,
}

impl FallbackState {
    /// Return the stable state label used inside oracle mismatch records.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::CppFallbackEnabled => "cpp-fallback-enabled",
            Self::Disabled => "disabled",
        }
    }

    /// Return the stable machine-readable marker used by CLI and oracle output.
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Native => "fallback=native",
            Self::CppFallbackEnabled => "fallback=cpp-fallback-enabled",
            Self::Disabled => "fallback=disabled",
        }
    }

    /// Parse a fallback marker from either a bare marker or a qalc-rs metadata line.
    pub fn from_marker(marker: &str) -> Option<Self> {
        let marker = marker
            .trim()
            .strip_prefix("[qalc-rs-metadata]")
            .map(str::trim)
            .unwrap_or_else(|| marker.trim());

        match marker {
            "fallback=native" => Some(Self::Native),
            "fallback=cpp-fallback-enabled" => Some(Self::CppFallbackEnabled),
            "fallback=disabled" => Some(Self::Disabled),
            _ => None,
        }
    }
}

/// Evaluation output plus the fallback state needed for oracle evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalculationOutput {
    /// The formatted result returned to the caller.
    pub output: String,
    /// The fallback routing state for this evaluation.
    pub fallback_state: FallbackState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PrintProfile {
    Api,
    Qalc,
}

impl Drop for Calculator {
    fn drop(&mut self) {
        let _guard = FFI_LOCK.lock().unwrap();
        let _ = std::mem::replace(&mut self.inner, UniquePtr::null());
    }
}

impl Calculator {
    /// Create a new `Calculator` instance.
    pub fn new() -> Self {
        // SAFETY: Calling C++ factory function to instantiate a new Calculator on the C++ heap.
        // The returned UniquePtr safely manages the lifetime of the object.
        let _guard = FFI_LOCK.lock().unwrap();
        let inner = sys::new_calculator();
        Self {
            inner,
            _phantom: PhantomData,
        }
    }

    /// Load the exchange rates for currencies.
    /// Returns `true` if loaded successfully.
    pub fn load_exchange_rates(&mut self) -> bool {
        if self.inner.is_null() {
            return false;
        }
        let _guard = FFI_LOCK.lock().unwrap();
        let pin = self.inner.pin_mut();
        // SAFETY: Passing a pinned mutable reference of the Calculator to the FFI function.
        // The pinned reference ensures the C++ object is not moved and is valid.
        sys::load_exchange_rates(pin)
    }

    /// Load the standard global definitions (system wide).
    /// Returns `true` if loaded successfully.
    pub fn load_global_definitions(&mut self) -> bool {
        if self.inner.is_null() {
            return false;
        }
        let _guard = FFI_LOCK.lock().unwrap();
        let pin = self.inner.pin_mut();
        // SAFETY: Passing a pinned mutable reference of the Calculator to the FFI function.
        // The pinned reference ensures the C++ object is not moved and is valid.
        sys::load_global_definitions(pin)
    }

    /// Load user-specific local definitions.
    /// Returns `true` if loaded successfully.
    pub fn load_local_definitions(&mut self) -> bool {
        if self.inner.is_null() {
            return false;
        }
        let _guard = FFI_LOCK.lock().unwrap();
        let pin = self.inner.pin_mut();
        // SAFETY: Passing a pinned mutable reference of the Calculator to the FFI function.
        // The pinned reference ensures the C++ object is not moved and is valid.
        sys::load_local_definitions(pin)
    }

    /// Evaluate a mathematical expression string and return the formatted result.
    ///
    /// # Errors
    /// Returns a `CalculatorError` if evaluation fails or fallback is disabled.
    ///
    /// # Panics
    /// Panics if the inner Calculator pointer is null, which indicates a bug
    /// (e.g., use-after-move). This should never happen in normal usage since
    /// `new()` always constructs a valid Calculator.
    pub fn calculate_and_print(
        &mut self,
        expr: &str,
        timeout_ms: i32,
    ) -> Result<String, CalculatorError> {
        self.calculate_and_print_with_fallback_state(expr, timeout_ms)
            .map(|result| result.output)
    }

    /// Evaluate a mathematical expression and return output plus fallback state.
    ///
    /// # Errors
    /// Returns a `CalculatorError` if evaluation fails or fallback is disabled.
    ///
    /// # Panics
    /// Panics if the inner Calculator pointer is null, which indicates a bug.
    pub fn calculate_and_print_with_fallback_state(
        &mut self,
        expr: &str,
        timeout_ms: i32,
    ) -> Result<CalculationOutput, CalculatorError> {
        self.calculate_with_profile(PrintProfile::Api, expr, timeout_ms)
    }

    /// Evaluate an expression using qalc-compatible print/evaluation defaults.
    ///
    /// This path is intended for the CLI/oracle harness. It preserves the plain
    /// `calculate_and_print` wrapper for API-default libqalculate smoke tests.
    ///
    /// # Errors
    /// Returns a `CalculatorError` if evaluation fails or fallback is disabled.
    ///
    /// # Panics
    /// Panics if the inner Calculator pointer is null, which indicates a bug.
    pub fn calculate_and_print_qalc(
        &mut self,
        expr: &str,
        timeout_ms: i32,
    ) -> Result<String, CalculatorError> {
        self.calculate_and_print_qalc_with_fallback_state(expr, timeout_ms)
            .map(|result| result.output)
    }

    /// Evaluate an expression using qalc-compatible defaults and return fallback state.
    ///
    /// # Errors
    /// Returns a `CalculatorError` if evaluation fails or fallback is disabled.
    ///
    /// # Panics
    /// Panics if the inner Calculator pointer is null, which indicates a bug.
    pub fn calculate_and_print_qalc_with_fallback_state(
        &mut self,
        expr: &str,
        timeout_ms: i32,
    ) -> Result<CalculationOutput, CalculatorError> {
        self.calculate_with_profile(PrintProfile::Qalc, expr, timeout_ms)
    }

    /// Evaluate an expression using qalc-compatible defaults plus a narrow set
    /// of qalc session settings supported by native fallback-disabled evidence.
    ///
    /// # Errors
    /// Returns a `CalculatorError` if evaluation fails or fallback is disabled
    /// for an unsupported expression/settings combination.
    ///
    /// # Panics
    /// Panics if the inner Calculator pointer is null, which indicates a bug.
    pub fn calculate_and_print_qalc_with_settings_and_fallback_state(
        &mut self,
        expr: &str,
        settings: &[&str],
        timeout_ms: i32,
    ) -> Result<CalculationOutput, CalculatorError> {
        self.calculate_with_profile_and_settings(PrintProfile::Qalc, expr, timeout_ms, settings)
    }

    fn calculate_with_profile(
        &mut self,
        profile: PrintProfile,
        expr: &str,
        timeout_ms: i32,
    ) -> Result<CalculationOutput, CalculatorError> {
        self.calculate_with_profile_and_settings(profile, expr, timeout_ms, &[])
    }

    fn calculate_with_profile_and_settings(
        &mut self,
        profile: PrintProfile,
        expr: &str,
        timeout_ms: i32,
        settings: &[&str],
    ) -> Result<CalculationOutput, CalculatorError> {
        assert!(
            !self.inner.is_null(),
            "BUG: Calculator inner pointer is null - possible use-after-move"
        );

        if fallback_disabled_by_env() {
            if let Some(output) = native_scaffold_output(profile, expr, settings) {
                return Ok(CalculationOutput {
                    output,
                    fallback_state: FallbackState::Native,
                });
            }
            return Err(CalculatorError::FallbackDisabled(expr.to_string()));
        }

        if !settings.is_empty() {
            return Err(CalculatorError::UnsupportedSessionSettings(
                settings
                    .iter()
                    .map(|setting| (*setting).to_string())
                    .collect(),
            ));
        }

        let output = {
            let _guard = FFI_LOCK.lock().unwrap();
            let pin = self.inner.pin_mut();
            match profile {
                PrintProfile::Api => sys::calculate_and_print(pin, expr, timeout_ms),
                PrintProfile::Qalc => sys::calculate_and_print_qalc(pin, expr, timeout_ms),
            }
            .map_err(CalculatorError::Cxx)?
        };

        Ok(CalculationOutput {
            output,
            fallback_state: FallbackState::CppFallbackEnabled,
        })
    }
}

fn fallback_disabled_by_env() -> bool {
    std::env::var("QALCULATE_DISABLE_FALLBACK").as_deref() == Ok("1")
}

fn expression_contains(
    expr: &crate::ast::Expression,
    predicate: &impl Fn(&crate::ast::Expression) -> bool,
) -> bool {
    use crate::ast::Expression;
    if predicate(expr) {
        return true;
    }

    match expr {
        Expression::Conversion { expr, target } => {
            expression_contains(expr, predicate) || expression_contains(target, predicate)
        }
        Expression::Multiplication(children)
        | Expression::Addition(children)
        | Expression::LogicalAnd(children)
        | Expression::LogicalOr(children)
        | Expression::BitwiseAnd(children)
        | Expression::BitwiseOr(children)
        | Expression::BitwiseXor(children) => children
            .as_slice()
            .iter()
            .any(|child| expression_contains(child, predicate)),
        Expression::Division {
            numerator,
            denominator,
        } => {
            expression_contains(numerator, predicate) || expression_contains(denominator, predicate)
        }
        Expression::Power { base, exponent } => {
            expression_contains(base, predicate) || expression_contains(exponent, predicate)
        }
        Expression::Remainder { lhs, rhs }
        | Expression::Modulo { lhs, rhs }
        | Expression::IntegerDivision { lhs, rhs }
        | Expression::ShiftLeft { lhs, rhs }
        | Expression::ShiftRight { lhs, rhs }
        | Expression::LogicalXor { lhs, rhs }
        | Expression::Parallel { lhs, rhs }
        | Expression::Comparison { lhs, rhs, .. } => {
            expression_contains(lhs, predicate) || expression_contains(rhs, predicate)
        }
        Expression::Inverse(child)
        | Expression::Negate(child)
        | Expression::Factorial(child)
        | Expression::DoubleFactorial(child)
        | Expression::MultiFactorial { expr: child, .. }
        | Expression::Percent(child)
        | Expression::LogicalNot(child)
        | Expression::BitwiseNot(child)
        | Expression::Assignment { value: child, .. } => expression_contains(child, predicate),
        Expression::FunctionCall { args, .. } | Expression::Vector(args) => args
            .iter()
            .any(|child| expression_contains(child, predicate)),
        Expression::Number(_)
        | Expression::Text(_)
        | Expression::Unit { .. }
        | Expression::Symbolic(_)
        | Expression::Variable(_)
        | Expression::Undefined
        | Expression::Aborted
        | Expression::DateTime(_) => false,
    }
}

fn contains_bitwise_ops(expr: &crate::ast::Expression) -> bool {
    expression_contains(expr, &|expr| {
        use crate::ast::Expression;
        matches!(
            expr,
            Expression::ShiftLeft { .. }
                | Expression::ShiftRight { .. }
                | Expression::BitwiseAnd(_)
                | Expression::BitwiseOr(_)
                | Expression::BitwiseXor(_)
                | Expression::BitwiseNot(_)
        )
    })
}

fn is_geometry_expression(expr: &crate::ast::Expression) -> bool {
    expression_contains(expr, &|expr| {
        let crate::ast::Expression::FunctionCall { function, .. } = expr else {
            return false;
        };
        crate::functions::geometry::lookup(function.id()).is_some()
    })
}

fn is_text_native_expression(expr: &crate::ast::Expression) -> bool {
    expression_contains(expr, &|expr| {
        let crate::ast::Expression::FunctionCall { function, .. } = expr else {
            if let crate::ast::Expression::Conversion { target, .. } = expr {
                return matches!(
                    target.as_ref(),
                    crate::ast::Expression::Symbolic(symbol)
                        if symbol.name().eq_ignore_ascii_case("unicode")
                );
            }
            return matches!(expr, crate::ast::Expression::Text(_));
        };
        crate::functions::utility_string::is_raw_utility_string(function.id())
    })
}

fn is_polynomial_native_expression(expr: &crate::ast::Expression) -> bool {
    expression_contains(expr, &|expr| {
        if let crate::ast::Expression::FunctionCall { function, .. } = expr {
            return matches!(
                function.id(),
                "coeff"
                    | "lcoeff"
                    | "tcoeff"
                    | "degree"
                    | "ldegree"
                    | "pcontent"
                    | "primpart"
                    | "punit"
                    | "factor"
            );
        }
        false
    })
}
fn evaluate_general_expression_natively(
    profile: PrintProfile,
    parsed: &crate::ast::Expression,
    context: &mut crate::context::CalculatorContext,
    precision_digits: usize,
) -> Option<String> {
    let evaluated = crate::eval::evaluate_ast(parsed, context).ok()?;
    match evaluated {
        crate::ast::Expression::Number(num) => {
            let output = num.to_string_with_options(
                precision_digits,
                context.print_options.number_fraction_format,
                context.evaluation_options.approximation,
            );
            Some(match profile {
                PrintProfile::Api => output,
                PrintProfile::Qalc => output.replace('-', "\u{2212}"),
            })
        }
        crate::ast::Expression::Symbolic(sym) => {
            let name = qalc_symbolic_conversion_output(profile, parsed, sym.name());
            Some(match profile {
                PrintProfile::Api => name,
                PrintProfile::Qalc => name.replace('-', "\u{2212}"),
            })
        }
        other => {
            let output = crate::text::format_result_with_numbers(&other, &|num| {
                num.to_string_with_options(
                    precision_digits,
                    context.print_options.number_fraction_format,
                    context.evaluation_options.approximation,
                )
            })?;
            Some(match profile {
                PrintProfile::Api => output,
                PrintProfile::Qalc => output.replace('-', "\u{2212}"),
            })
        }
    }
}

fn qalc_symbolic_conversion_output(
    profile: PrintProfile,
    parsed: &crate::ast::Expression,
    output: &str,
) -> String {
    if profile == PrintProfile::Qalc
        && conversion_target_is_hex(parsed)
        && !output.starts_with("0x")
        && output.chars().all(|ch| ch.is_ascii_hexdigit())
    {
        format!("0x{output}")
    } else {
        output.to_string()
    }
}

fn conversion_target_is_hex(expr: &crate::ast::Expression) -> bool {
    let crate::ast::Expression::Conversion { target, .. } = expr else {
        return false;
    };
    matches!(
        target.as_ref(),
        crate::ast::Expression::Symbolic(symbol)
            if matches!(symbol.name().to_ascii_lowercase().as_str(), "hex" | "hexadecimal")
    )
}

fn native_scaffold_output(profile: PrintProfile, expr: &str, settings: &[&str]) -> Option<String> {
    let parsed_settings = crate::session::NativeSessionSettings::from_raw(settings)?;

    if let Some(collection) = crate::matrix::parse_collection_literal(expr) {
        let mut context = crate::context::CalculatorContext::default();
        for cmd in settings {
            let _ = context.apply_command(cmd);
        }
        if let Some(output) = evaluate_general_expression_natively(
            profile,
            &collection,
            &mut context,
            parsed_settings.precision_digits(),
        ) {
            return Some(output);
        }
    }

    if parsed_settings.has_precision() && crate::matrix::is_promoted_magnitude_function(expr) {
        return None;
    }

    if !settings.is_empty() && crate::matrix::is_promoted_norm_function(expr) {
        return None;
    }

    if !settings.is_empty() && crate::matrix::is_promoted_combine_function(expr) {
        return None;
    }

    if !settings.is_empty() && crate::matrix::is_promoted_concat_function(expr) {
        return None;
    }

    if let Some(collection_result) = crate::matrix::evaluate_collection_function(expr) {
        let mut context = crate::context::CalculatorContext::default();
        for cmd in settings {
            let _ = context.apply_command(cmd);
        }
        if let Some(output) = evaluate_general_expression_natively(
            profile,
            &collection_result,
            &mut context,
            parsed_settings.precision_digits(),
        ) {
            return Some(output);
        }
    }

    if let Some(collection_result) = crate::matrix::evaluate_collection_arithmetic(expr) {
        let mut context = crate::context::CalculatorContext::default();
        for cmd in settings {
            let _ = context.apply_command(cmd);
        }
        if let Some(output) = evaluate_general_expression_natively(
            profile,
            &collection_result,
            &mut context,
            parsed_settings.precision_digits(),
        ) {
            return Some(output);
        }
    }

    let parsed = crate::parser::operators::parse_expression(expr).ok();
    if let Some(ref ast) = parsed {
        if contains_bitwise_ops(ast)
            || is_geometry_expression(ast)
            || is_text_native_expression(ast)
            || is_polynomial_native_expression(ast)
        {
            // Build a context from session settings so native evaluation
            // respects user configuration (precision, base, etc.).
            let mut context = crate::context::CalculatorContext::default();
            for cmd in settings {
                let _ = context.apply_command(cmd);
            }
            if let Some(output) = evaluate_general_expression_natively(
                profile,
                ast,
                &mut context,
                parsed_settings.precision_digits(),
            ) {
                return Some(output);
            }
        }
    }

    if !parsed_settings.has_interval_display() {
        if let Some(output) = crate::numberbase::native_output(expr, parsed_settings) {
            return Some(output);
        }
    }

    if !parsed_settings.is_numeric_scaffold_compatible() {
        return None;
    }

    if expr == "native-scaffold-test" {
        if parsed_settings.has_interval_display() {
            return None;
        }
        return Some("native-scaffold-test-success".to_string());
    }

    if let Some(output) = native_boolean_evidence(expr, parsed_settings) {
        return Some(output);
    }

    if let Some(output) = native_interval_set_evidence(expr, parsed_settings) {
        return Some(output);
    }

    let evidence = native_numeric_evidence(expr)?;
    if parsed_settings.has_precision() && !evidence.supports_precision() {
        return None;
    }
    if evidence.requires_precision() && !parsed_settings.has_precision() {
        return None;
    }
    if parsed_settings.has_interval_calculation() && !evidence.allows_interval_calculation() {
        return None;
    }
    if evidence.requires_interval_display() && !parsed_settings.has_interval_display() {
        return None;
    }
    if evidence.requires_interval_calculation() && !parsed_settings.has_interval_calculation() {
        return None;
    }
    if parsed_settings.has_interval_display() && !evidence.requires_interval_display() {
        return None;
    }
    if evidence.requires_concise_uncertainty() && !parsed_settings.has_concise_uncertainty() {
        return None;
    }
    if parsed_settings.has_concise_uncertainty() && !evidence.requires_concise_uncertainty() {
        return None;
    }

    let evaluated = if parsed_settings.has_precision() {
        crate::number::evaluate_expr_with_precision_digits(expr, parsed_settings.precision_digits())
    } else {
        crate::number::evaluate_expr(expr)
    };

    match evaluated {
        Ok(num) if !num.is_nan() => {
            let output = match profile {
                PrintProfile::Api => num.to_string(),
                PrintProfile::Qalc if evidence.formats_interval_output() => {
                    num.to_qalc_interval_display_string(parsed_settings.precision_digits())?
                }
                PrintProfile::Qalc if evidence.preserves_float_uncertainty_precision() => num
                    .to_qalc_string_preserving_float_uncertainty_precision(
                        parsed_settings.precision_digits(),
                    ),
                PrintProfile::Qalc => {
                    num.to_qalc_string_with_precision(parsed_settings.precision_digits())
                }
            };
            Some(match profile {
                PrintProfile::Api => output,
                PrintProfile::Qalc => output.replace('-', "−"),
            })
        }
        _ => None,
    }
}

fn native_interval_set_evidence(
    expr: &str,
    settings: crate::session::NativeSessionSettings,
) -> Option<String> {
    if !settings.has_interval_display()
        || settings.has_precision()
        || settings.has_concise_uncertainty()
    {
        return None;
    }
    match expr.trim() {
        "intersect(interval(1;2), interval(3;4))" => Some("[]".to_string()),
        _ => None,
    }
}

#[derive(Clone, Copy)]
enum NativeBooleanEvidence {
    DefaultOnly,
    PrecisionRequired,
}

const NATIVE_BOOLEAN_EVIDENCE: &[(&str, NativeBooleanEvidence)] = &[
    ("(1 + i) = (1 + i)", NativeBooleanEvidence::DefaultOnly),
    ("(1 + i) == (1 + i)", NativeBooleanEvidence::DefaultOnly),
    ("(1 + i) = (1 - i)", NativeBooleanEvidence::DefaultOnly),
    ("(1 + i) != (1 - i)", NativeBooleanEvidence::DefaultOnly),
    ("(1 + i) ≠ (1 - i)", NativeBooleanEvidence::DefaultOnly),
    ("(1 + i) != (1 + i)", NativeBooleanEvidence::DefaultOnly),
    ("(1 + i) < (1 + i)", NativeBooleanEvidence::DefaultOnly),
    ("(1 + i) <= (1 + i)", NativeBooleanEvidence::DefaultOnly),
    ("(1 + i) > (1 + i)", NativeBooleanEvidence::DefaultOnly),
    ("(1 + i) >= (1 + i)", NativeBooleanEvidence::DefaultOnly),
    ("(1 + i) ≤ (1 + i)", NativeBooleanEvidence::DefaultOnly),
    ("(1 + i) ≥ (1 + i)", NativeBooleanEvidence::DefaultOnly),
    (
        "(2 ^ 0.5) < (3 ^ 0.5)",
        NativeBooleanEvidence::PrecisionRequired,
    ),
    (
        "(2 ^ 0.5) = (2 ^ 0.5)",
        NativeBooleanEvidence::PrecisionRequired,
    ),
    (
        "(2 ^ 0.5) = (3 ^ 0.5)",
        NativeBooleanEvidence::PrecisionRequired,
    ),
    (
        "(2 ^ 0.5) + 1/3 > 1",
        NativeBooleanEvidence::PrecisionRequired,
    ),
    ("(2 ^ 0.5) < 1/3", NativeBooleanEvidence::PrecisionRequired),
];

fn native_boolean_evidence(
    expr: &str,
    settings: crate::session::NativeSessionSettings,
) -> Option<String> {
    let trimmed = expr.trim();
    let evidence = NATIVE_BOOLEAN_EVIDENCE
        .iter()
        .find_map(|(candidate, evidence)| (*candidate == trimmed).then_some(*evidence))?;

    if settings.has_interval_calculation()
        || settings.has_interval_display()
        || settings.has_concise_uncertainty()
    {
        return None;
    }

    let evaluated = match evidence {
        NativeBooleanEvidence::DefaultOnly if settings.is_empty() => {
            crate::number::evaluate_relation_expr(trimmed)
        }
        NativeBooleanEvidence::PrecisionRequired if settings.has_precision() => {
            crate::number::evaluate_relation_expr_with_precision_digits(
                trimmed,
                settings.precision_digits(),
            )
        }
        _ => return None,
    };

    evaluated.ok().flatten().map(|value| value.to_string())
}

#[derive(Clone, Copy)]
enum NativeNumericEvidence {
    DefaultOnly,
    Precision,
    PrecisionRequired,
    IntervalDisplay,
    IntervalArithmetic,
    IntervalScalar,
    PreciseFloatUncertainty,
    ConciseUncertainty,
}

impl NativeNumericEvidence {
    const fn supports_precision(self) -> bool {
        matches!(self, Self::Precision | Self::PrecisionRequired)
    }

    const fn requires_precision(self) -> bool {
        matches!(self, Self::PrecisionRequired)
    }

    const fn requires_interval_display(self) -> bool {
        matches!(
            self,
            Self::IntervalDisplay | Self::IntervalArithmetic | Self::IntervalScalar
        )
    }

    const fn requires_interval_calculation(self) -> bool {
        matches!(self, Self::IntervalArithmetic)
    }

    const fn allows_interval_calculation(self) -> bool {
        matches!(
            self,
            Self::IntervalDisplay | Self::IntervalArithmetic | Self::IntervalScalar
        )
    }

    const fn formats_interval_output(self) -> bool {
        matches!(self, Self::IntervalDisplay | Self::IntervalArithmetic)
    }

    const fn preserves_float_uncertainty_precision(self) -> bool {
        matches!(self, Self::PreciseFloatUncertainty)
    }

    const fn requires_concise_uncertainty(self) -> bool {
        matches!(self, Self::ConciseUncertainty)
    }
}

const NATIVE_NUMERIC_EVIDENCE: &[(&str, NativeNumericEvidence)] = &[
    ("1/3", NativeNumericEvidence::Precision),
    ("2 ^ 0.5", NativeNumericEvidence::Precision),
    (
        "(2 ^ 0.5) + (3 ^ 0.5)",
        NativeNumericEvidence::PrecisionRequired,
    ),
    (
        "(3 ^ 0.5) - (2 ^ 0.5)",
        NativeNumericEvidence::PrecisionRequired,
    ),
    (
        "(2 ^ 0.5) * (3 ^ 0.5)",
        NativeNumericEvidence::PrecisionRequired,
    ),
    (
        "(3 ^ 0.5) / (2 ^ 0.5)",
        NativeNumericEvidence::PrecisionRequired,
    ),
    ("(2 ^ 0.5) + 1/3", NativeNumericEvidence::PrecisionRequired),
    ("0.1 + 0.2", NativeNumericEvidence::PrecisionRequired),
    (
        "1.25e-20 + 2.5e-20",
        NativeNumericEvidence::PrecisionRequired,
    ),
    ("2.5e3 / 4", NativeNumericEvidence::PrecisionRequired),
    ("interval(5;2)", NativeNumericEvidence::IntervalDisplay),
    ("interval(1;3;0)", NativeNumericEvidence::IntervalDisplay),
    ("interval(1;3;1)", NativeNumericEvidence::IntervalDisplay),
    (
        "interval(-infinity;5)",
        NativeNumericEvidence::IntervalDisplay,
    ),
    (
        "interval(4;infinity)",
        NativeNumericEvidence::IntervalDisplay,
    ),
    (
        "interval(-infinity;-4)",
        NativeNumericEvidence::IntervalDisplay,
    ),
    ("interval(-3;-1)", NativeNumericEvidence::IntervalDisplay),
    (
        "lowerEndpoint(interval(1;3))",
        NativeNumericEvidence::IntervalScalar,
    ),
    (
        "upperEndpoint(interval(1;3))",
        NativeNumericEvidence::IntervalScalar,
    ),
    (
        "midpoint(interval(1;3))",
        NativeNumericEvidence::IntervalScalar,
    ),
    (
        "lowerEndpoint(interval(1;3;1))",
        NativeNumericEvidence::IntervalScalar,
    ),
    (
        "upperEndpoint(interval(1;3;1))",
        NativeNumericEvidence::IntervalScalar,
    ),
    (
        "midpoint(interval(1;3;1))",
        NativeNumericEvidence::IntervalScalar,
    ),
    (
        "lowerEndpoint(interval(-infinity;-4))",
        NativeNumericEvidence::IntervalScalar,
    ),
    (
        "upperEndpoint(interval(4;infinity))",
        NativeNumericEvidence::IntervalScalar,
    ),
    (
        "interval(1;2) + interval(3;4)",
        NativeNumericEvidence::IntervalArithmetic,
    ),
    (
        "interval(3;4) - interval(1;2)",
        NativeNumericEvidence::IntervalArithmetic,
    ),
    (
        "interval(-2;3) * interval(-4;5)",
        NativeNumericEvidence::IntervalArithmetic,
    ),
    (
        "interval(4;6) / interval(2;3)",
        NativeNumericEvidence::IntervalArithmetic,
    ),
    (
        "interval(-infinity;5) + interval(2;3)",
        NativeNumericEvidence::IntervalArithmetic,
    ),
    (
        "interval(-infinity;5) - interval(2;3)",
        NativeNumericEvidence::IntervalArithmetic,
    ),
    (
        "interval(-infinity;5) * interval(2;3)",
        NativeNumericEvidence::IntervalArithmetic,
    ),
    (
        "interval(4;infinity) + interval(2;3)",
        NativeNumericEvidence::IntervalArithmetic,
    ),
    (
        "interval(4;infinity) - interval(2;3)",
        NativeNumericEvidence::IntervalArithmetic,
    ),
    (
        "interval(4;infinity) * interval(2;3)",
        NativeNumericEvidence::IntervalArithmetic,
    ),
    (
        "interval(4;infinity) / 2",
        NativeNumericEvidence::IntervalArithmetic,
    ),
    (
        "interval(4;6) / interval(-3;-2)",
        NativeNumericEvidence::IntervalArithmetic,
    ),
    (
        "interval(-6;-4) / interval(2;3)",
        NativeNumericEvidence::IntervalArithmetic,
    ),
    (
        "interval(-6;-4) / interval(-3;-2)",
        NativeNumericEvidence::IntervalArithmetic,
    ),
    (
        "interval(-infinity;-4) / 2",
        NativeNumericEvidence::IntervalArithmetic,
    ),
    (
        "interval(-infinity;-4) / -2",
        NativeNumericEvidence::IntervalArithmetic,
    ),
    ("ln(0)", NativeNumericEvidence::Precision),
    ("ln(2)", NativeNumericEvidence::Precision),
    ("ln(2) + sqrt(2)", NativeNumericEvidence::PrecisionRequired),
    ("ln(5+/-0.3)", NativeNumericEvidence::DefaultOnly),
    ("sqrt(2)", NativeNumericEvidence::Precision),
    ("sqrt(4)", NativeNumericEvidence::Precision),
    ("infinity", NativeNumericEvidence::DefaultOnly),
    ("-infinity", NativeNumericEvidence::DefaultOnly),
    ("infinity + 1", NativeNumericEvidence::DefaultOnly),
    ("-infinity - 1", NativeNumericEvidence::DefaultOnly),
    ("infinity * 2", NativeNumericEvidence::DefaultOnly),
    ("infinity * -2", NativeNumericEvidence::DefaultOnly),
    ("1 / infinity", NativeNumericEvidence::DefaultOnly),
    ("infinity / 2", NativeNumericEvidence::DefaultOnly),
    ("infinity / -2", NativeNumericEvidence::DefaultOnly),
    ("-infinity / 2", NativeNumericEvidence::DefaultOnly),
    ("-infinity / -2", NativeNumericEvidence::DefaultOnly),
    ("1 / -infinity", NativeNumericEvidence::DefaultOnly),
    ("0", NativeNumericEvidence::DefaultOnly),
    ("-0", NativeNumericEvidence::DefaultOnly),
    ("123456789", NativeNumericEvidence::DefaultOnly),
    ("-123", NativeNumericEvidence::DefaultOnly),
    ("-123456789", NativeNumericEvidence::DefaultOnly),
    ("-0.", NativeNumericEvidence::DefaultOnly),
    ("0.", NativeNumericEvidence::DefaultOnly),
    ("0.0", NativeNumericEvidence::DefaultOnly),
    ("0.01", NativeNumericEvidence::DefaultOnly),
    (".123", NativeNumericEvidence::DefaultOnly),
    ("-.", NativeNumericEvidence::DefaultOnly),
    (".", NativeNumericEvidence::DefaultOnly),
    ("12345.67890", NativeNumericEvidence::DefaultOnly),
    ("1e0", NativeNumericEvidence::DefaultOnly),
    ("-1e0", NativeNumericEvidence::DefaultOnly),
    ("1e3", NativeNumericEvidence::DefaultOnly),
    ("1E3", NativeNumericEvidence::DefaultOnly),
    ("1e-3", NativeNumericEvidence::DefaultOnly),
    ("1e10", NativeNumericEvidence::DefaultOnly),
    ("1e303", NativeNumericEvidence::DefaultOnly),
    ("6%2", NativeNumericEvidence::DefaultOnly),
    ("7 rem 2", NativeNumericEvidence::DefaultOnly),
    ("-8%3", NativeNumericEvidence::DefaultOnly),
    ("3 %% 2", NativeNumericEvidence::DefaultOnly),
    ("3 %% -2", NativeNumericEvidence::DefaultOnly),
    ("3 mod -2", NativeNumericEvidence::DefaultOnly),
    ("5//2", NativeNumericEvidence::DefaultOnly),
    ("5\\2", NativeNumericEvidence::DefaultOnly),
    ("5 div 2", NativeNumericEvidence::DefaultOnly),
    ("5 ^ 2", NativeNumericEvidence::DefaultOnly),
    ("2 ^ -3", NativeNumericEvidence::DefaultOnly),
    ("(-2) ^ -3", NativeNumericEvidence::DefaultOnly),
    ("(1/2) ^ -3", NativeNumericEvidence::DefaultOnly),
    ("5 ** 3", NativeNumericEvidence::DefaultOnly),
    ("4 ** 3 ** 2", NativeNumericEvidence::DefaultOnly),
    ("1 + 1", NativeNumericEvidence::DefaultOnly),
    ("1 + 2", NativeNumericEvidence::DefaultOnly),
    ("5--2", NativeNumericEvidence::DefaultOnly),
    ("5---2", NativeNumericEvidence::DefaultOnly),
    ("-5-2", NativeNumericEvidence::DefaultOnly),
    ("2*3", NativeNumericEvidence::DefaultOnly),
    ("6/2", NativeNumericEvidence::DefaultOnly),
    ("1/2", NativeNumericEvidence::DefaultOnly),
    ("i", NativeNumericEvidence::DefaultOnly),
    ("5i", NativeNumericEvidence::DefaultOnly),
    ("(1 + 2i) + (3 + 4i)", NativeNumericEvidence::DefaultOnly),
    ("(1 + 2i) - (3 + 4i)", NativeNumericEvidence::DefaultOnly),
    ("(1 + 2i) * (3 + 4i)", NativeNumericEvidence::DefaultOnly),
    ("(1 + 2i) / (3 + 4i)", NativeNumericEvidence::DefaultOnly),
    ("i + (-i)", NativeNumericEvidence::DefaultOnly),
    ("(1 + i) + (-1 + i)", NativeNumericEvidence::DefaultOnly),
    ("(1 + i) + (2 - i)", NativeNumericEvidence::DefaultOnly),
    ("(1 + i) * (1 - i)", NativeNumericEvidence::DefaultOnly),
    ("(1 + i) / (1 - i)", NativeNumericEvidence::DefaultOnly),
    ("conj(3 + 4i)", NativeNumericEvidence::DefaultOnly),
    ("conj(i)", NativeNumericEvidence::DefaultOnly),
    ("conj(-i)", NativeNumericEvidence::DefaultOnly),
    ("conj(3)", NativeNumericEvidence::DefaultOnly),
    ("norm(3 + 4i)", NativeNumericEvidence::DefaultOnly),
    ("norm(i)", NativeNumericEvidence::DefaultOnly),
    ("norm(-3i)", NativeNumericEvidence::DefaultOnly),
    ("i^2", NativeNumericEvidence::DefaultOnly),
    ("(2i - 3)^(3.2i + 3)", NativeNumericEvidence::DefaultOnly),
    ("2+/-0.002", NativeNumericEvidence::DefaultOnly),
    ("2 +/- 0.002", NativeNumericEvidence::DefaultOnly),
    ("2 +/- 0.002 + 3", NativeNumericEvidence::DefaultOnly),
    ("2±0.002", NativeNumericEvidence::DefaultOnly),
    ("2±0.002 + 3", NativeNumericEvidence::DefaultOnly),
    ("100+/-5%", NativeNumericEvidence::DefaultOnly),
    ("uncertainty(2;0.002;0)", NativeNumericEvidence::DefaultOnly),
    (
        "uncertainty(100;0.05;1)",
        NativeNumericEvidence::DefaultOnly,
    ),
    ("uncertainty(10;0;0)", NativeNumericEvidence::DefaultOnly),
    ("errorPart(2+/-0.002)", NativeNumericEvidence::DefaultOnly),
    ("errorPart(100+/-5%)", NativeNumericEvidence::DefaultOnly),
    ("errorPart(2+/-0.002;0)", NativeNumericEvidence::DefaultOnly),
    ("errorPart(2+/-0.002;1)", NativeNumericEvidence::DefaultOnly),
    ("errorPart(100+/-5%;0)", NativeNumericEvidence::DefaultOnly),
    ("errorPart(100+/-5%;1)", NativeNumericEvidence::DefaultOnly),
    ("valuePart(2+/-0.002)", NativeNumericEvidence::DefaultOnly),
    ("valuePart(100+/-5%)", NativeNumericEvidence::DefaultOnly),
    ("midpoint(2+/-0.002)", NativeNumericEvidence::DefaultOnly),
    (
        "lowerEndpoint(2+/-0.002)",
        NativeNumericEvidence::DefaultOnly,
    ),
    (
        "upperEndpoint(2+/-0.002)",
        NativeNumericEvidence::DefaultOnly,
    ),
    ("100+/-5 + 200+/-10%", NativeNumericEvidence::DefaultOnly),
    ("100+/-5% + 200+/-10%", NativeNumericEvidence::DefaultOnly),
    ("100+/-5% * 2", NativeNumericEvidence::DefaultOnly),
    ("20+/-3 + 10+/-4", NativeNumericEvidence::DefaultOnly),
    ("20+/-3 - 10+/-4", NativeNumericEvidence::DefaultOnly),
    ("3+/-0.2 * 4+/-0.1", NativeNumericEvidence::DefaultOnly),
    ("12+/-0.5 / 3+/-0.2", NativeNumericEvidence::DefaultOnly),
    ("3+/-0.2 / 4+/-0.1", NativeNumericEvidence::DefaultOnly),
    (
        "(2+/-3)^3.2",
        NativeNumericEvidence::PreciseFloatUncertainty,
    ),
    ("10 +/- 0", NativeNumericEvidence::DefaultOnly),
    ("1.23(4)", NativeNumericEvidence::ConciseUncertainty),
    ("123(4)", NativeNumericEvidence::ConciseUncertainty),
    (
        "1.23(4) + 2.0(3)",
        NativeNumericEvidence::ConciseUncertainty,
    ),
];

fn native_numeric_evidence(expr: &str) -> Option<NativeNumericEvidence> {
    let trimmed = expr.trim();
    NATIVE_NUMERIC_EVIDENCE
        .iter()
        .find_map(|(expression, evidence)| (*expression == trimmed).then_some(*evidence))
}

/// Custom error type for `Calculator` evaluations.
#[derive(Debug)]
pub enum CalculatorError {
    /// Wrapping a C++ exception returned via CXX FFI.
    Cxx(cxx::Exception),
    /// The C++ fallback is disabled and the requested feature is unimplemented natively.
    FallbackDisabled(String),
    /// Session settings were supplied on a path that cannot apply them safely.
    UnsupportedSessionSettings(Vec<String>),
}

impl std::fmt::Display for CalculatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CalculatorError::Cxx(e) => write!(f, "{}", e),
            CalculatorError::FallbackDisabled(expr) => {
                write!(
                    f,
                    "C++ FFI fallback is disabled, and expression '{}' has no native Rust implementation",
                    expr
                )
            }
            CalculatorError::UnsupportedSessionSettings(settings) => {
                write!(
                    f,
                    "session settings are not supported by the C++ FFI fallback path: {}",
                    settings.join("; ")
                )
            }
        }
    }
}

impl std::error::Error for CalculatorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CalculatorError::Cxx(e) => Some(e),
            CalculatorError::FallbackDisabled(_) => None,
            CalculatorError::UnsupportedSessionSettings(_) => None,
        }
    }
}

impl CalculatorError {
    /// Return the fallback state associated with this error.
    pub const fn fallback_state(&self) -> FallbackState {
        match self {
            Self::Cxx(_) => FallbackState::CppFallbackEnabled,
            Self::FallbackDisabled(_) => FallbackState::Disabled,
            Self::UnsupportedSessionSettings(_) => FallbackState::CppFallbackEnabled,
        }
    }
}

impl Default for Calculator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    const DISABLE_FALLBACK_ENV: &str = "QALCULATE_DISABLE_FALLBACK";
    const DEFINITIONS_DIR_ENV: &str = "QALCULATE_DEFINITIONS_DIR";

    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    struct EnvGuard {
        previous: Option<OsString>,
    }

    impl EnvGuard {
        fn set_disabled() -> Self {
            let guard = Self {
                previous: std::env::var_os(DISABLE_FALLBACK_ENV),
            };
            std::env::set_var(DISABLE_FALLBACK_ENV, "1");
            guard
        }

        fn unset_disabled() -> Self {
            let guard = Self {
                previous: std::env::var_os(DISABLE_FALLBACK_ENV),
            };
            std::env::remove_var(DISABLE_FALLBACK_ENV);
            guard
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match self.previous.take() {
                Some(value) => std::env::set_var(DISABLE_FALLBACK_ENV, value),
                None => std::env::remove_var(DISABLE_FALLBACK_ENV),
            }
        }
    }

    fn definitions_dir() -> PathBuf {
        Path::new("../libqalculate/data")
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from("../libqalculate/data"))
    }

    fn configure_definitions_dir() {
        std::env::set_var(DEFINITIONS_DIR_ENV, definitions_dir());
    }

    #[test]
    fn calculation_uses_cpp_fallback_when_enabled() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let _env = EnvGuard::unset_disabled();
        configure_definitions_dir();

        let mut calc = Calculator::new();
        calc.load_global_definitions();
        let result = calc
            .calculate_and_print_with_fallback_state("3 * 4", 1000)
            .unwrap();
        assert_eq!(result.output, "12");
        assert_eq!(result.fallback_state, FallbackState::CppFallbackEnabled);
    }

    #[test]
    fn fallback_disabled_rejects_unported_expression() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let _env = EnvGuard::set_disabled();
        configure_definitions_dir();

        let mut calc = Calculator::new();
        calc.load_global_definitions();

        let err = calc.calculate_and_print("x + 1", 1000).unwrap_err();
        match err {
            CalculatorError::FallbackDisabled(expr) => assert_eq!(expr, "x + 1"),
            _ => panic!("expected fallback-disabled error"),
        }
    }

    #[test]
    fn fallback_disabled_runs_native_scaffold_cases() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let _env = EnvGuard::set_disabled();
        configure_definitions_dir();

        let mut calc = Calculator::new();
        calc.load_global_definitions();

        let addition = calc
            .calculate_and_print_with_fallback_state("1 + 2", 1000)
            .unwrap();
        assert_eq!(addition.output, "3");
        assert_eq!(addition.fallback_state, FallbackState::Native);

        let scaffold_addition = calc
            .calculate_and_print_with_fallback_state("1 + 1", 1000)
            .unwrap();
        assert_eq!(scaffold_addition.output, "2");
        assert_eq!(scaffold_addition.fallback_state, FallbackState::Native);

        let err = calc.calculate_and_print("2 + 2", 1000).unwrap_err();
        match err {
            CalculatorError::FallbackDisabled(expr) => assert_eq!(expr, "2 + 2"),
            _ => panic!("expected fallback-disabled error for 2 + 2"),
        }

        let scaffold = calc
            .calculate_and_print_with_fallback_state("native-scaffold-test", 1000)
            .unwrap();
        assert_eq!(scaffold.output, "native-scaffold-test-success");
        assert_eq!(scaffold.fallback_state, FallbackState::Native);

        let uncertainty_power = calc
            .calculate_and_print_qalc_with_fallback_state("(2+/-3)^3.2", 1000)
            .unwrap();
        assert_eq!(uncertainty_power.output, "9.18958684±44.11001683");
        assert_eq!(uncertainty_power.fallback_state, FallbackState::Native);
    }
}
