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

fn native_scaffold_output(profile: PrintProfile, expr: &str, settings: &[&str]) -> Option<String> {
    let settings = crate::session::NativeSessionSettings::from_raw(settings)?;

    if let Some(output) = crate::numberbase::native_output(expr, settings) {
        return Some(output);
    }

    if !settings.is_numeric_scaffold_compatible() {
        return None;
    }

    if expr == "native-scaffold-test" {
        return Some("native-scaffold-test-success".to_string());
    }

    if !is_vetted_native_numeric_expr(expr) {
        return None;
    }

    match crate::number::evaluate_expr(expr) {
        Ok(num) if !num.is_nan() => {
            let output = match profile {
                PrintProfile::Api => num.to_string(),
                PrintProfile::Qalc => {
                    num.to_qalc_string_with_precision(settings.precision_digits())
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
    }
}
