use libqalculate_rust::ffi::FallbackState;
use std::path::Path;
use std::process::Command;

fn defs_dir() -> std::path::PathBuf {
    Path::new("../libqalculate/data")
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from("../libqalculate/data"))
}

fn run_qalc_rs(
    expression: &str,
    disable_fallback: Option<&str>,
    report_fallback: Option<&str>,
) -> (String, String, i32) {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    let mut cmd = Command::new(&cargo);
    cmd.arg("run")
        .arg("--quiet")
        .arg("--bin")
        .arg("qalc-rs")
        .arg("--manifest-path")
        .arg(manifest_dir.join("Cargo.toml"))
        .arg("--")
        .arg("--")
        .arg(expression)
        .env("LC_ALL", "C.UTF-8")
        .env("TZ", "UTC")
        .env("QALCULATE_DEFINITIONS_DIR", defs_dir())
        .env_remove("QALCULATE_DISABLE_FALLBACK")
        .env_remove("QALCULATE_REPORT_FALLBACK");

    if let Some(df) = disable_fallback {
        cmd.env("QALCULATE_DISABLE_FALLBACK", df);
    }
    if let Some(rf) = report_fallback {
        cmd.env("QALCULATE_REPORT_FALLBACK", rf);
    }

    let output = cmd.output().expect("failed to execute qalc-rs");

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    (stdout, stderr, exit_code)
}

#[test]
fn fallback_state_markers_are_stable() {
    assert_eq!(FallbackState::Native.marker(), "fallback=native");
    assert_eq!(
        FallbackState::CppFallbackEnabled.marker(),
        "fallback=cpp-fallback-enabled"
    );
    assert_eq!(FallbackState::Disabled.marker(), "fallback=disabled");

    assert_eq!(
        FallbackState::from_marker("[qalc-rs-metadata] fallback=native"),
        Some(FallbackState::Native)
    );
    assert_eq!(
        FallbackState::from_marker("fallback=cpp-fallback-enabled"),
        Some(FallbackState::CppFallbackEnabled)
    );
    assert_eq!(FallbackState::from_marker("fallback=unknown"), None);
}

#[test]
fn cli_native_scaffold_succeeds_when_fallback_disabled() {
    let (stdout, stderr, exit_code) = run_qalc_rs("1 + 2", Some("1"), Some("1"));
    assert_eq!(stdout, "3");
    assert!(stderr.contains("[qalc-rs-metadata] fallback=native"));
    assert_eq!(exit_code, 0);

    let (stdout, stderr, exit_code) = run_qalc_rs("native-scaffold-test", Some("1"), Some("1"));
    assert_eq!(stdout, "native-scaffold-test-success");
    assert!(stderr.contains("[qalc-rs-metadata] fallback=native"));
    assert_eq!(exit_code, 0);
}

#[test]
fn cli_native_expression_succeeds_when_fallback_disabled() {
    let (stdout, stderr, exit_code) = run_qalc_rs("1 + 2", Some("1"), Some("1"));
    assert_eq!(stdout, "3");
    assert!(stderr.contains("[qalc-rs-metadata] fallback=native"));
    assert_eq!(exit_code, 0);

    let native_cases = [
        ("0.01", "0.01"),
        ("1/2", "0.5"),
        ("1/3", "0.3333333333"),
        ("1e10", "10000000000"),
        ("1e303", "1E303"),
        ("1 + 1", "2"),
        ("52 to bin", "0011 0100"),
        ("52 to bin16", "0000 0000 0011 0100"),
        ("52 to oct", "064"),
        ("52 to hex", "0x34"),
        ("0x34", "52"),
        ("hex(34)", "52"),
        ("523<<2&250 to bin", "0010 1000"),
        ("52.345 to float", "0100 0010 0101 0001 0110 0001 0100 1000"),
        ("float(01000010010100010110000101001000)", "52.34500122"),
        ("floatError(52.345)", "0.000001220703125"),
        ("1978 to roman", "MCMLXXVIII"),
        ("52 to base 32", "1K"),
        ("sqrt(32) to base sqrt(2)", "100000"),
        ("6%2", "0"),
        ("7 rem 2", "1"),
        ("-8%3", "−2"),
        ("3 %% 2", "1"),
        ("3 %% -2", "−1"),
        ("3 mod -2", "−1"),
        ("5//2", "2"),
        ("5\\2", "2"),
        ("5 div 2", "2"),
        ("5 ^ 2", "25"),
        ("2 ^ -3", "0.125"),
        ("(-2) ^ -3", "−0.125"),
        ("(1/2) ^ -3", "8"),
        ("5 ** 3", "125"),
        ("4 ** 3 ** 2", "262144"),
        ("ln(0)", "−∞"),
        ("ln(2)", "0.6931471806"),
        ("sqrt(2)", "1.414213562"),
        ("sqrt(4)", "2"),
        ("-123", "−123"),
        ("(1 + 2i) + (3 + 4i)", "4 + 6i"),
        ("(1 + 2i) / (3 + 4i)", "0.44 + 0.08i"),
        ("100+/-5 + 200+/-10%", "300±21"),
        ("100+/-5% + 200+/-10%", "300±6.9%"),
        ("100+/-5% * 2", "200±5.0%"),
    ];
    for (expression, expected) in native_cases {
        let (stdout, stderr, exit_code) = run_qalc_rs(expression, Some("1"), Some("1"));
        assert_eq!(stdout, expected, "{expression} produced unexpected output");
        assert!(
            stderr.contains("[qalc-rs-metadata] fallback=native"),
            "{expression} did not report native fallback state: {stderr}"
        );
        assert_eq!(exit_code, 0, "{expression} returned unexpected exit code");
    }

    let (stdout, stderr, exit_code) = run_qalc_rs("10 +/- 0", Some("1"), Some("1"));
    assert_eq!(stdout, "10");
    assert!(stderr.contains("[qalc-rs-metadata] fallback=native"));
    assert_eq!(exit_code, 0);
}

#[test]
fn cli_invalid_native_expression_fails_when_fallback_disabled() {
    let (stdout, stderr, exit_code) = run_qalc_rs("1 / 0", Some("1"), Some("1"));
    assert!(stdout.is_empty());
    assert!(stderr.contains("[qalc-rs-metadata] fallback=disabled"));
    assert!(stderr.contains("error: calculation failed: C++ FFI fallback is disabled"));
    assert_eq!(exit_code, 2);

    let (stdout, stderr, exit_code) = run_qalc_rs("-2^2", Some("1"), Some("1"));
    assert!(stdout.is_empty());
    assert!(stderr.contains("[qalc-rs-metadata] fallback=disabled"));
    assert!(stderr.contains("error: calculation failed: C++ FFI fallback is disabled"));
    assert_eq!(exit_code, 2);

    let (stdout, stderr, exit_code) = run_qalc_rs("1 2", Some("1"), Some("1"));
    assert!(stdout.is_empty());
    assert!(stderr.contains("[qalc-rs-metadata] fallback=disabled"));
    assert!(stderr.contains("error: calculation failed: C++ FFI fallback is disabled"));
    assert_eq!(exit_code, 2);

    for expression in [
        "1(2)3",
        "1.23(4)5",
        "1(2)%",
        "2 + 2",
        "0 ^ -1",
        "53 to bin",
        "65536 to bin16",
        "hex(35)",
        "52 to base 36",
        "inf",
        "nan",
        "[1,2]",
        "[1,2] + [3,4]",
        "1/[2,3]",
        "170141183460469231731687303715884105728 + 1",
    ] {
        let (stdout, stderr, exit_code) = run_qalc_rs(expression, Some("1"), Some("1"));
        assert!(stdout.is_empty(), "{expression} unexpectedly wrote stdout");
        assert!(
            stderr.contains("[qalc-rs-metadata] fallback=disabled"),
            "{expression} did not report disabled fallback: {stderr}"
        );
        assert!(
            stderr.contains("error: calculation failed: C++ FFI fallback is disabled"),
            "{expression} did not report disabled fallback error: {stderr}"
        );
        assert_eq!(exit_code, 2, "{expression} returned unexpected exit code");
    }
}

#[test]
fn cli_uses_cpp_fallback_when_fallback_enabled() {
    let (stdout, stderr, exit_code) = run_qalc_rs("2 + 2", None, Some("1"));
    assert_eq!(stdout, "4");
    assert!(stderr.contains("[qalc-rs-metadata] fallback=cpp-fallback-enabled"));
    assert_eq!(exit_code, 0);
}
