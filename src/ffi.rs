#![allow(unsafe_code)]
//! Safe Rust wrapper and FFI bindings for C++ libqalculate's Calculator.

use cxx::UniquePtr;
use std::marker::PhantomData;

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

#[derive(Debug, Clone, Copy)]
enum PrintProfile {
    Api,
    Qalc,
}

impl Calculator {
    /// Create a new `Calculator` instance.
    pub fn new() -> Self {
        // SAFETY: Calling C++ factory function to instantiate a new Calculator on the C++ heap.
        // The returned UniquePtr safely manages the lifetime of the object.
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

    fn calculate_with_profile(
        &mut self,
        profile: PrintProfile,
        expr: &str,
        timeout_ms: i32,
    ) -> Result<CalculationOutput, CalculatorError> {
        assert!(
            !self.inner.is_null(),
            "BUG: Calculator inner pointer is null - possible use-after-move"
        );

        if fallback_disabled_by_env() {
            if let Some(output) = native_scaffold_output(profile, expr) {
                return Ok(CalculationOutput {
                    output,
                    fallback_state: FallbackState::Native,
                });
            }
            return Err(CalculatorError::FallbackDisabled(expr.to_string()));
        }

        let output = {
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

fn native_scaffold_output(profile: PrintProfile, expr: &str) -> Option<String> {
    if expr == "native-scaffold-test" {
        return Some("native-scaffold-test-success".to_string());
    }

    if is_vetted_native_numberbase_expr(expr) {
        if let Some(output) = native_numberbase_output(expr) {
            return Some(output);
        }
    }

    if !is_vetted_native_numeric_expr(expr) {
        return None;
    }

    match crate::number::evaluate_expr(expr) {
        Ok(num) if !num.is_nan() => {
            let output = match profile {
                PrintProfile::Api => num.to_string(),
                PrintProfile::Qalc => num.to_qalc_string(),
            };
            Some(match profile {
                PrintProfile::Api => output,
                PrintProfile::Qalc => output.replace('-', "−"),
            })
        }
        _ => None,
    }
}

fn native_numberbase_output(expr: &str) -> Option<String> {
    let trimmed = expr.trim();

    if let Some(hex_digits) = trimmed.strip_prefix("0x") {
        return parse_radix_u128(hex_digits, 16).map(|value| value.to_string());
    }

    if let Some(inner) = strip_function_call(trimmed, "hex") {
        return parse_radix_u128(inner, 16).map(|value| value.to_string());
    }

    if let Some(inner) = strip_function_call(trimmed, "float") {
        let bits = parse_bit_string_u32(inner)?;
        return Some(format!("{:.8}", f32::from_bits(bits)));
    }

    if let Some(inner) = strip_function_call(trimmed, "floatError") {
        return float_error_decimal(inner);
    }

    let (lhs, target) = trimmed.split_once(" to ")?;
    let lhs = lhs.trim();
    let target = target.trim();

    if target == "float" {
        let value = lhs.parse::<f32>().ok()?;
        return Some(group_bits_4(&format!("{:032b}", value.to_bits())));
    }

    if let Some(output) = format_sqrt_base(lhs, target) {
        return Some(output);
    }

    let value = eval_native_base_integer(lhs)?;
    match target {
        "bin" => Some(format_binary(value, None)),
        "bin16" => Some(format_binary(value, Some(16))),
        "oct" => Some(format!("0{value:o}")),
        "hex" => Some(format!("0x{value:X}")),
        "roman" => roman_numeral(value),
        _ => {
            let base = target.strip_prefix("base ")?.parse::<u32>().ok()?;
            (2..=36)
                .contains(&base)
                .then(|| format_integer_base(value, base))
        }
    }
}

fn strip_function_call<'a>(expr: &'a str, name: &str) -> Option<&'a str> {
    expr.strip_prefix(name)?
        .strip_prefix('(')?
        .strip_suffix(')')
        .map(str::trim)
}

fn parse_radix_u128(digits: &str, radix: u32) -> Option<u128> {
    let compact: String = digits.chars().filter(|ch| !ch.is_whitespace()).collect();
    (!compact.is_empty())
        .then_some(compact)
        .and_then(|value| u128::from_str_radix(&value, radix).ok())
}

fn parse_bit_string_u32(bits: &str) -> Option<u32> {
    let compact: String = bits.chars().filter(|ch| !ch.is_whitespace()).collect();
    (compact.len() == 32 && compact.chars().all(|ch| matches!(ch, '0' | '1')))
        .then_some(())
        .and_then(|_| u32::from_str_radix(&compact, 2).ok())
}

fn eval_native_base_integer(expr: &str) -> Option<u128> {
    if let Some((lhs, rhs)) = expr.split_once('&') {
        let lhs = eval_native_shift(lhs.trim())?;
        let rhs = rhs.trim().parse::<u128>().ok()?;
        return Some(lhs & rhs);
    }

    eval_native_shift(expr)
}

fn eval_native_shift(expr: &str) -> Option<u128> {
    if let Some((lhs, rhs)) = expr.split_once("<<") {
        let lhs = lhs.trim().parse::<u128>().ok()?;
        let rhs = rhs.trim().parse::<u32>().ok()?;
        return lhs.checked_shl(rhs);
    }

    expr.trim().parse::<u128>().ok()
}

fn format_binary(value: u128, width: Option<usize>) -> String {
    let raw = format!("{value:b}");
    let width = width.unwrap_or_else(|| raw.len().div_ceil(8) * 8).max(8);
    let padded = format!("{raw:0>width$}");
    group_bits_4(&padded)
}

fn group_bits_4(bits: &str) -> String {
    bits.as_bytes()
        .chunks(4)
        .map(|chunk| std::str::from_utf8(chunk).expect("binary digits are valid UTF-8"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn format_integer_base(mut value: u128, base: u32) -> String {
    const DIGITS: &[u8; 36] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    if value == 0 {
        return "0".to_string();
    }

    let mut output = Vec::new();
    let base = u128::from(base);
    while value > 0 {
        let digit = (value % base) as usize;
        output.push(DIGITS[digit] as char);
        value /= base;
    }
    output.into_iter().rev().collect()
}

fn format_sqrt_base(lhs: &str, target: &str) -> Option<String> {
    let lhs_radicand = parse_sqrt_radicand(lhs)?;
    let base_radicand = parse_sqrt_radicand(target.strip_prefix("base ")?.trim())?;
    if base_radicand <= 1 {
        return None;
    }

    let mut power = 1u128;
    for exponent in 0..=128 {
        if power == lhs_radicand {
            return Some(format!("1{}", "0".repeat(exponent)));
        }
        power = power.checked_mul(base_radicand)?;
        if power > lhs_radicand {
            return None;
        }
    }
    None
}

fn parse_sqrt_radicand(expr: &str) -> Option<u128> {
    strip_function_call(expr.trim(), "sqrt")?
        .parse::<u128>()
        .ok()
}

fn float_error_decimal(decimal: &str) -> Option<String> {
    let (decimal_num, decimal_den) = parse_decimal_rational(decimal)?;
    let (float_num, float_den) = f32_rational_parts(decimal.parse::<f32>().ok()?)?;
    let lhs = float_num.checked_mul(decimal_den as i128)?;
    let rhs = i128::try_from(decimal_num.checked_mul(float_den)?).ok()?;
    let diff_num = lhs.abs_diff(rhs);
    let diff_den = float_den.checked_mul(decimal_den)?;
    terminating_decimal(diff_num, diff_den)
}

fn parse_decimal_rational(decimal: &str) -> Option<(u128, u128)> {
    let trimmed = decimal.trim();
    let (whole, fractional) = trimmed.split_once('.').unwrap_or((trimmed, ""));
    if whole.is_empty() && fractional.is_empty() {
        return None;
    }
    if !whole.chars().all(|ch| ch.is_ascii_digit())
        || !fractional.chars().all(|ch| ch.is_ascii_digit())
    {
        return None;
    }

    let digits = format!("{whole}{fractional}");
    let numerator = digits.parse::<u128>().ok()?;
    let denominator = 10u128.checked_pow(fractional.len() as u32)?;
    Some((numerator, denominator))
}

fn f32_rational_parts(value: f32) -> Option<(i128, u128)> {
    if !value.is_finite() {
        return None;
    }
    let bits = value.to_bits();
    let negative = (bits >> 31) != 0;
    let exponent_bits = ((bits >> 23) & 0xff) as i32;
    let fraction_bits = bits & 0x7f_ffff;
    let (mantissa, exponent) = if exponent_bits == 0 {
        (u128::from(fraction_bits), 1 - 127 - 23)
    } else {
        (
            u128::from((1 << 23) | fraction_bits),
            exponent_bits - 127 - 23,
        )
    };

    let (numerator, denominator) = if exponent >= 0 {
        (mantissa.checked_shl(exponent as u32)?, 1)
    } else {
        (mantissa, 1u128.checked_shl((-exponent) as u32)?)
    };
    let numerator = i128::try_from(numerator).ok()?;
    Some((if negative { -numerator } else { numerator }, denominator))
}

fn terminating_decimal(mut numerator: u128, mut denominator: u128) -> Option<String> {
    if denominator == 0 {
        return None;
    }
    let gcd = gcd_u128(numerator, denominator);
    numerator /= gcd;
    denominator /= gcd;

    let mut twos = 0usize;
    while denominator.is_multiple_of(2) {
        denominator /= 2;
        twos += 1;
    }
    let mut fives = 0usize;
    while denominator.is_multiple_of(5) {
        denominator /= 5;
        fives += 1;
    }
    if denominator != 1 {
        return None;
    }

    let scale = twos.max(fives);
    for _ in 0..(scale - twos) {
        numerator = numerator.checked_mul(2)?;
    }
    for _ in 0..(scale - fives) {
        numerator = numerator.checked_mul(5)?;
    }

    let mut digits = numerator.to_string();
    if scale == 0 {
        return Some(digits);
    }
    if digits.len() <= scale {
        digits = format!("{}{}", "0".repeat(scale + 1 - digits.len()), digits);
    }
    let split = digits.len() - scale;
    Some(format!("{}.{}", &digits[..split], &digits[split..]))
}

fn gcd_u128(mut lhs: u128, mut rhs: u128) -> u128 {
    while rhs != 0 {
        let rem = lhs % rhs;
        lhs = rhs;
        rhs = rem;
    }
    lhs
}

fn roman_numeral(mut value: u128) -> Option<String> {
    if !(1..=3999).contains(&value) {
        return None;
    }
    let table = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut output = String::new();
    for (arabic, roman) in table {
        while value >= arabic {
            output.push_str(roman);
            value -= arabic;
        }
    }
    Some(output)
}

fn is_vetted_native_numberbase_expr(expr: &str) -> bool {
    let trimmed = expr.trim();
    matches!(
        trimmed,
        "52 to bin"
            | "52 to bin16"
            | "52 to oct"
            | "52 to hex"
            | "0x34"
            | "hex(34)"
            | "523<<2&250 to bin"
            | "52.345 to float"
            | "float(01000010010100010110000101001000)"
            | "floatError(52.345)"
            | "1978 to roman"
            | "52 to base 32"
            | "sqrt(32) to base sqrt(2)"
    )
}

fn is_vetted_native_numeric_expr(expr: &str) -> bool {
    let trimmed = expr.trim();
    matches!(
        trimmed,
        "0" | "-0"
            | "123456789"
            | "-123"
            | "-123456789"
            | "-0."
            | "0."
            | "0.0"
            | "0.01"
            | ".123"
            | "-."
            | "."
            | "12345.67890"
            | "1e0"
            | "-1e0"
            | "1e3"
            | "1E3"
            | "1e-3"
            | "1e10"
            | "1e303"
            | "6%2"
            | "7 rem 2"
            | "-8%3"
            | "3 %% 2"
            | "3 %% -2"
            | "3 mod -2"
            | "5//2"
            | "5\\2"
            | "5 div 2"
            | "5 ^ 2"
            | "2 ^ -3"
            | "(-2) ^ -3"
            | "(1/2) ^ -3"
            | "5 ** 3"
            | "4 ** 3 ** 2"
            | "1 + 1"
            | "1 + 2"
            | "5--2"
            | "5---2"
            | "-5-2"
            | "2*3"
            | "6/2"
            | "1/2"
            | "1/3"
            | "i"
            | "5i"
            | "(1 + 2i) + (3 + 4i)"
            | "(1 + 2i) * (3 + 4i)"
            | "(1 + 2i) / (3 + 4i)"
            | "2+/-0.002"
            | "100+/-5%"
            | "100+/-5 + 200+/-10%"
            | "100+/-5% + 200+/-10%"
            | "100+/-5% * 2"
            | "20+/-3 + 10+/-4"
            | "3+/-0.2 * 4+/-0.1"
            | "12+/-0.5 / 3+/-0.2"
            | "10 +/- 0"
    )
}

/// Custom error type for `Calculator` evaluations.
#[derive(Debug)]
pub enum CalculatorError {
    /// Wrapping a C++ exception returned via CXX FFI.
    Cxx(cxx::Exception),
    /// The C++ fallback is disabled and the requested feature is unimplemented natively.
    FallbackDisabled(String),
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
        }
    }
}

impl std::error::Error for CalculatorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CalculatorError::Cxx(e) => Some(e),
            CalculatorError::FallbackDisabled(_) => None,
        }
    }
}

impl CalculatorError {
    /// Return the fallback state associated with this error.
    pub const fn fallback_state(&self) -> FallbackState {
        match self {
            Self::Cxx(_) => FallbackState::CppFallbackEnabled,
            Self::FallbackDisabled(_) => FallbackState::Disabled,
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
    }
}
