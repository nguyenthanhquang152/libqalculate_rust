use std::sync::Mutex;

use libqalculate_rust::ffi::{Calculator, FallbackState};

static ENV_LOCK: Mutex<()> = Mutex::new(());

struct EnvGuard {
    key: &'static str,
    previous: Option<std::ffi::OsString>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let previous = std::env::var_os(key);
        std::env::set_var(key, value);
        Self { key, previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        if let Some(previous) = &self.previous {
            std::env::set_var(self.key, previous);
        } else {
            std::env::remove_var(self.key);
        }
    }
}

fn native_qalc_output(expression: &str) -> String {
    let _lock = ENV_LOCK.lock().expect("format-number env lock poisoned");
    let _guard = EnvGuard::set("QALCULATE_DISABLE_FALLBACK", "1");
    let mut calc = Calculator::new();
    let output = calc
        .calculate_and_print_qalc_with_fallback_state(expression, 1000)
        .expect("format-number expression should run natively");
    assert_eq!(output.fallback_state, FallbackState::Native);
    output.output
}

fn native_qalc_output_with_settings(expression: &str, settings: &[&str]) -> String {
    let _lock = ENV_LOCK.lock().expect("format-number env lock poisoned");
    let _guard = EnvGuard::set("QALCULATE_DISABLE_FALLBACK", "1");
    let mut calc = Calculator::new();
    let output = calc
        .calculate_and_print_qalc_with_settings_and_fallback_state(expression, settings, 1000)
        .expect("format-number expression with settings should run natively");
    assert_eq!(output.fallback_state, FallbackState::Native);
    output.output
}

#[test]
fn native_formats_parser_batch_decimal_normalization_cases() {
    for (expression, expected) in [
        ("-0", "0"),
        ("-0.", "0"),
        ("12345.67890", "12345.6789"),
        ("1e-3", "0.001"),
        ("1.23e-5", "0.0000123"),
        ("1e303", "1E303"),
    ] {
        assert_eq!(native_qalc_output(expression), expected, "{expression}");
    }
}

#[test]
fn native_formats_large_integer_scientific_edges() {
    for (expression, expected) in [
        ("1000000000000", "1000000000000"),
        ("10000000000000", "1E13"),
        ("12345000000000", "1.2345E13"),
        ("12345678901234", "1.234567890E13"),
        ("99999999999999", "1E14"),
        ("99999999994999", "9.999999999E13"),
        ("12345678905000", "1.234567891E13"),
    ] {
        assert_eq!(native_qalc_output(expression), expected, "{expression}");
    }
}

#[test]
fn native_formats_print_option_exponent_settings() {
    for (expression, settings, expected) in [
        ("10000000000000", &["exp 0"][..], "10000000000000"),
        ("12345678901234", &["exp off"][..], "12345678901234"),
        ("10000", &["exp 3"][..], "1E4"),
        ("10000", &["exp -3"][..], "10E3"),
        ("1e303", &["edisp 1"][..], "1e303"),
        ("12345678901234", &["edisp 2"][..], "1.234567890 × 10^13"),
        ("10000000000000", &["edisp 2"][..], "10^13"),
    ] {
        assert_eq!(
            native_qalc_output_with_settings(expression, settings),
            expected,
            "{expression} with {settings:?}"
        );
    }
}

#[test]
fn native_formats_print_option_decimal_limits() {
    for (expression, settings, expected) in [
        ("1/3", &["max decimals 2"][..], "0.33"),
        ("2/3", &["max decimals 2"][..], "0.67"),
        ("12345678901234", &["max decimals 2"][..], "1.23E13"),
        ("1", &["min decimals 2"][..], "1.00"),
        ("1.2", &["min decimals 4"][..], "1.2000"),
        ("10000000000000", &["min decimals 2"][..], "1.00E13"),
        ("1e303", &["min decimals 4"][..], "1.0000E303"),
    ] {
        assert_eq!(
            native_qalc_output_with_settings(expression, settings),
            expected,
            "{expression} with {settings:?}"
        );
    }
}

#[test]
fn native_formats_numberbase_batch_cases() {
    for (expression, expected) in [
        ("52 to bin", "0011 0100"),
        ("52 to bin16", "0000 0000 0011 0100"),
        ("52 to oct", "064"),
        ("52 to hex", "0x34"),
        ("1978 to roman", "MCMLXXVIII"),
        ("52 to base 32", "1K"),
        ("sqrt(32) to base sqrt(2)", "100000"),
    ] {
        assert_eq!(native_qalc_output(expression), expected, "{expression}");
    }
}
