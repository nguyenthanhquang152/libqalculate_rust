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
    let _lock = ENV_LOCK
        .lock()
        .expect("datetime function env lock poisoned");
    let _guard = EnvGuard::set("QALCULATE_DISABLE_FALLBACK", "1");
    let mut calc = Calculator::new();
    let output = calc
        .calculate_and_print_qalc_with_fallback_state(expression, 1000)
        .expect("datetime function expression should run natively");
    assert_eq!(output.fallback_state, FallbackState::Native);
    output.output
}

fn native_qalc_error(expression: &str) -> String {
    let _lock = ENV_LOCK
        .lock()
        .expect("datetime function env lock poisoned");
    let _guard = EnvGuard::set("QALCULATE_DISABLE_FALLBACK", "1");
    let mut calc = Calculator::new();
    calc.calculate_and_print_qalc_with_fallback_state(expression, 1000)
        .expect_err("invalid datetime function expression should fail natively")
        .to_string()
}

#[test]
fn native_outputs_selected_dates_batch_datetime_function_cases() {
    assert_eq!(
        native_qalc_output(r#""2020-05-20" + 523d"#),
        "\"2021-10-25\""
    );
    assert_eq!(
        native_qalc_output("addDays(2020-05-20; 523)"),
        "\"2021-10-25\""
    );
    assert_eq!(native_qalc_output(r#""2020-11-05" - "2020-10-05""#), "31 d");
    assert_eq!(
        native_qalc_output(r#""2020-10-05" - "2020-10-15""#),
        "−10 d"
    );
    assert_eq!(
        native_qalc_output("timestamp(2020-05-20T00:00:00Z)"),
        "1589932800"
    );
    assert_eq!(
        native_qalc_output("timestamp(2020-05-20T01:00:00+01:00)"),
        "1589932800"
    );
    assert_eq!(
        native_qalc_output("stamptodate(1 589 932 800) to utc"),
        "\"2020-05-20T00:00:00Z\""
    );
    assert_eq!(
        native_qalc_output("stamptodate(1589932800) to utc+1"),
        "\"2020-05-20T01:00:00+01:00\""
    );
    assert_eq!(
        native_qalc_output(r#""2020-05-20T00:00:00Z" to utc+1"#),
        "\"2020-05-20T01:00:00+01:00\""
    );
    assert_eq!(
        native_qalc_output("lunarphase(2022-02-11T00:00Z)"),
        "0.32288434"
    );
    assert_eq!(
        native_qalc_output("nextlunarphase(0.5, 2022-02-11T00:00Z) to utc"),
        "\"2022-02-16T16:56:27Z\""
    );
    assert_eq!(
        native_qalc_output("nextlunarphase(0.5, 2022-02-11T00:00Z) to utc+1"),
        "\"2022-02-16T17:56:27+01:00\""
    );
    assert_eq!(
        native_qalc_output("nextlunarphase(180, 2022-02-11T00:00Z) to utc"),
        "\"2022-02-16T16:56:27Z\""
    );
    assert_eq!(
        native_qalc_output("nextlunarphase(0, 2022-02-11T00:00Z) to utc"),
        "\"2022-03-02T17:34:42Z\""
    );
}

#[test]
fn native_datetime_functions_reject_invalid_dates_without_fallback() {
    let error = native_qalc_error("timestamp(2020-02-30T00:00:00Z)");
    assert!(
        error.contains("invalid day 30 for 2020-02"),
        "unexpected error: {error}"
    );

    for expression in [
        "nextlunarphase(-0.1, 2022-02-11T00:00Z) to utc",
        "nextlunarphase(1, 2022-02-11T00:00Z) to utc",
        "nextlunarphase(360, 2022-02-11T00:00Z) to utc",
    ] {
        let error = native_qalc_error(expression);
        assert!(
            error.contains("invalid lunar phase"),
            "unexpected error for {expression}: {error}"
        );
    }

    let error = native_qalc_error("nextlunarphase(0.5, 1000000000000000000-01-01T00:00Z) to utc");
    assert!(
        error.contains("date/time value is out of range")
            || error.contains("lunar phase timestamp is out of range"),
        "unexpected error: {error}"
    );
}
