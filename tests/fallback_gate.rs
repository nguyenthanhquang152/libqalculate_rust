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
    run_qalc_rs_args(&["--", expression], disable_fallback, report_fallback)
}

fn run_qalc_rs_args(
    args: &[&str],
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
        .args(args)
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
        ("ln(5+/-0.3)", "1.609±0.060"),
        ("sqrt(2)", "1.414213562"),
        ("sqrt(4)", "2"),
        ("infinity", "+∞"),
        ("-infinity", "−∞"),
        ("infinity + 1", "+∞"),
        ("-infinity - 1", "−∞"),
        ("infinity * 2", "+∞"),
        ("infinity * -2", "−∞"),
        ("1 / infinity", "0"),
        ("infinity / 2", "+∞"),
        ("infinity / -2", "−∞"),
        ("-infinity / 2", "−∞"),
        ("-infinity / -2", "+∞"),
        ("1 / -infinity", "0"),
        ("-123", "−123"),
        ("(1 + 2i) + (3 + 4i)", "4 + 6i"),
        ("(1 + 2i) / (3 + 4i)", "0.44 + 0.08i"),
        ("i + (-i)", "0"),
        ("(1 + i) + (-1 + i)", "2i"),
        ("(1 + i) + (2 - i)", "3"),
        ("(1 + i) * (1 - i)", "2"),
        ("(1 + i) / (1 - i)", "i"),
        ("conj(i)", "−i"),
        ("conj(-i)", "i"),
        ("conj(3)", "3"),
        ("norm(i)", "1"),
        ("norm(-3i)", "3"),
        ("2±0.002", "2.0000±0.0020"),
        ("2±0.002 + 3", "5.0000±0.0020"),
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
fn cli_native_vector_matrix_literals_succeed_when_fallback_disabled() {
    let native_cases = [
        ("(1,)", "[1  0]"),
        ("1,", "[1  0]"),
        ("[,,,]", "[0  0  0  0]"),
        ("(,,,-2)", "[0  0  0  −2]"),
        ("(1;;2)", "[1  0  2]"),
        ("(1,1)", "[1  1]"),
        ("((1, 2), (4, 5))", "[1  2; 4  5]"),
        ("((1; 2; 3); (4; 5; 6))", "[1  2  3; 4  5  6]"),
        ("[[1, 2], [4, 5]]", "[1  2; 4  5]"),
        (
            "[-0.1, 1.23, ], [.1, , -.2], [,,]",
            "[−0.1  1.23  0; 0.1  0  −0.2; 0  0  0]",
        ),
        ("columns([])", "0"),
        ("columns([1])", "1"),
        ("columns([[,,,]])", "4"),
        ("dimension([])", "0"),
        ("dimension([0])", "1"),
        ("dimension([1 2 3 4])", "4"),
        ("matrix(1, 1, [2])", "2"),
        ("matrix(1, 3, 2)", "[2  0  0]"),
        ("matrix(3, 1, [1 2])", "[1; 2; 0]"),
        ("vector(1, 2, 3)", "[1  2  3]"),
        ("vector()", "[]"),
        ("vector(,)", "[0  0]"),
        ("matrix(3, 3, [])", "[0  0  0; 0  0  0; 0  0  0]"),
        (
            "matrix(3, 3, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10)",
            "[1  2  3; 4  5  6; 7  8  9]",
        ),
        ("matrix2vector([[0]])", "0"),
        ("matrix2vector([1 2; 4 5])", "[1  2  4  5]"),
        (
            "matrix2vector([1 2 3; 4 5 6; 7 8 9])",
            "[1  2  3  4  5  6  7  8  9]",
        ),
        ("horzcat([1], [2 3], [4 5 6 7])", "[1  2  3  4  5  6  7]"),
        (
            "horzcat([1; 2], [3 4; 5 6], [7 8 9; 10 11 12])",
            "[1  3  4  7  8  9; 2  5  6  10  11  12]",
        ),
        ("vertcat([1 2], [3 4], [5 6])", "[1  2; 3  4; 5  6]"),
        ("dot((2); (3))", "6"),
        ("dot((1; 2); (3, 4))", "11"),
        ("dot((1; 2; 3); (4; 5; 6))", "32"),
        ("(1; 2; 3).(4; 5; 6)", "32"),
        ("(1; 2; 3, 4) . (5; 6; 7, 8)", "70"),
        ("cross((1; 2; 3); (4; 5; 6))", "[−3  6  −3]"),
        ("columns([1 2; 4 5])", "2"),
        ("column([1], 1)", "1"),
        ("column([1, 2], 1)", "1"),
        ("column([1 2; 3 4], 2)", "[2  4]"),
        ("element([1 2; 3 4], 1)", "[1  2]"),
        ("element([1 2 3; 4 5 6; 1 0 9], 1, 3)", "3"),
        ("element([1 2 3; 4 5 6], 2, 1)", "4"),
        ("elements([])", "0"),
        ("elements([1 2])", "2"),
        ("elements([1 2; 3 4])", "4"),
        ("row([1], 1)", "1"),
        ("row([1 2], 1)", "[1  2]"),
        ("row([1 2; 3 4], 2)", "[3  4]"),
        ("rows([1])", "1"),
        ("rows([1 2; 3 4])", "2"),
        ("[1,2] + [3,4]", "[4  6]"),
        ("multiply(1)", "1"),
        ("multiply([1 2; 4 5], 2)", "[2  4; 8  10]"),
        ("multiply([1 2], 3)", "[3  6]"),
        ("multiply([1 2], 3, 4)", "[12  24]"),
        ("multiply([1 2; 4 5], 2, 3)", "[6  12; 24  30]"),
        ("hadamard([2], [3], [4])", "24"),
        (
            "hadamard([1 2 3; 4 5 6]; [7 8 9; 10 11 12])",
            "[7  16  27; 40  55  72]",
        ),
        ("identity(1)", "1"),
        ("identity(3)", "[1  0  0; 0  1  0; 0  0  1]"),
        ("identity([1 2; 4 5])", "[1  0; 0  1]"),
        ("combine([1, 2])", "[1  2]"),
        ("combine([1, 2], [3], [4, 5, 6])", "[1  2  3  4  5  6]"),
        ("magnitude(-2)", "2"),
        ("magnitude([-2])", "2"),
        ("magnitude([-2, 3, 4])", "5.385164807"),
        ("norm([2])", "2"),
        ("norm([3, 4])", "5"),
        ("norm([2, 3, 6])", "7"),
        ("adj([1 2; 4 5])", "[5  −2; −4  1]"),
        (
            "adj([1, 2, 3; 4, 5, 6; 1, 0, 9])",
            "[45  −18  −3; −30  6  6; −5  2  −3]",
        ),
        (
            "adj([3 4 7 9; 5 4 -1 4; 8 7 8 5; 4 3 0 9])",
            "[240  264  −177  −259; −284  −436  194  370; 16  100  −53  −31; −12  28  14  −54]",
        ),
        ("cofactor([1 2; 4 5], 1, 1)", "5"),
        ("cofactor([1 2 3; 4 5 6; 1 0 9], 1, 2)", "−30"),
        (
            "cofactor([3 4 7 9; 5 4 -1 4; 8 7 8 5; 4 3 0 9], 4, 4)",
            "−54",
        ),
        ("permanent([1])", "1"),
        ("permanent([1 2; 4 5])", "13"),
        ("permanent([1 2 3; 4 5 6; 1 0 9])", "144"),
        ("permanent([3 4 7 9; 5 4 -1 4; 8 7 8 5; 4 3 0 9])", "11028"),
        ("det([[1]])", "1"),
        ("det([1 2; 4 5])", "−3"),
        ("det([1 2 3; 4 5 6; 1 0 9])", "−30"),
        ("det([3 4 7 9; 5 4 -1 4; 8 7 8 5; 4 3 0 9])", "−412"),
        ("part([1], 1, 1, 1, 1)", "1"),
        ("part([1 2 3; 4 5 6; 7 8 9; 10 11 12], 2, 2, 2, 2)", "5"),
        (
            "part([1 2 3; 4 5 6; 7 8 9; 10 11 12], 1, 3, 2, 3)",
            "[3; 6]",
        ),
        (
            "part([1 2 3; 4 5 6; 7 8 9; 10 11 12], 1, 2, 4, 3)",
            "[2  3; 5  6; 8  9; 11  12]",
        ),
        ("slice([5], 1, 1)", "5"),
        ("slice([5, 6, 7, 8, 9], 2, 4)", "[6  7  8]"),
        ("sort([5, 2, 0, 1, 3, -4, 0])", "[−4  0  0  1  2  3  5]"),
        ("sort([5, 2, 0, 1, 3, -4, 0], 1)", "[−4  0  0  1  2  3  5]"),
        ("sort([5, 2, 0, 1, 3, -4, 0], 0)", "[5  3  2  1  0  0  −4]"),
        ("rank([6, 7, 1, 4])", "[3  4  1  2]"),
        ("rank([-1, 2, 5, 10], 1)", "[1  2  3  4]"),
        ("rank([-1, 2, 5, 10], 0)", "[4  3  2  1]"),
        ("rk([1 2 3; 3 6 9])", "1"),
        ("rk([1 2 3; 0 2 2; 1 4 5])", "2"),
        ("rk([1 2 3; 0 2 2; 1 -2 -1])", "2"),
        ("rk(identity(3))", "3"),
        (
            "rref([1 3 1 9; 1 1 -1 1; 3 11 5 35])",
            "[1  0  −2  −3; 0  1  1  4; 0  0  0  0]",
        ),
        ("entrywise(x, [4 10 12], x)", "[4  10  12]"),
        ("entrywise(x / y, [4 10 12], x, [2 2 4], y)", "[2  5  3]"),
        (
            "entrywise(x / y + z, [4 10 12], x, [2 2 4], y, [1 2 3], z)",
            "[3  7  6]",
        ),
        ("transpose([1 2; 3 4])", "[1  3; 2  4]"),
        ("[1 2 3; 4 5 6].'", "[1  4; 2  5; 3  6]"),
        ("[1 2] times 3 times 4", "[12  24]"),
        ("[1 2] Times 3", "[3  6]"),
        ("(1; 2; 3) * 2 - 2", "[0  2  4]"),
        ("[1 2; 4 5] * 2", "[2  4; 8  10]"),
        (
            "((1; 2; 3); (4; 5; 6)) * ((7; 8); (9; 10); (11; 12))",
            "[58  64; 139  154]",
        ),
        ("[1 2; 3 4] times [5 6; 7 8]", "[19  22; 43  50]"),
        ("[1 2].*[3 4]", "[3  8]"),
        ("[1; 2].*[3 4]", "[3  4; 6  8]"),
        ("[1 2; 3 4].*[1 2; 3 4]", "[1  4; 9  16]"),
        ("pow([1 2; 3 4], 2)", "[1  4; 9  16]"),
        ("[1 2; 3 4].^2", "[1  4; 9  16]"),
        ("[2 4; 3 4].^[-1; 2]", "[0.5  0.25; 9  16]"),
        ("[2; 3].^[3 4]", "[8  16; 27  81]"),
        ("[2 4 12] / 2", "[1  2  6]"),
        ("divide([2 4 12], 2)", "[1  2  6]"),
        ("rdivide([2 4 12], 2)", "[1  2  6]"),
        ("divide([2 4; 6 12], 2)", "[1  2; 3  6]"),
        ("rdivide([2 4], [1 2])", "[2  2]"),
        ("[2 4 12]./2", "[1  2  6]"),
        ("[2 4]./1/2", "[1  2]"),
        ("[[2, 4], [6, 12]] / 2", "[1  2; 3  6]"),
        ("divide([2 4; 6 12], [1 2; 3 4])", "[2  2; 2  3]"),
        ("[2 4; 6 12]./[1 2; 3 4]", "[2  2; 2  3]"),
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

    let (stdout, stderr, exit_code) = run_qalc_rs_args(
        &["-set", "precision 128", "--", "magnitude([-2, 3, 4])"],
        Some("1"),
        Some("1"),
    );
    assert!(stdout.is_empty());
    assert!(stderr.contains("[qalc-rs-metadata] fallback=disabled"));
    assert!(stderr.contains("error: calculation failed: C++ FFI fallback is disabled"));
    assert_eq!(exit_code, 2);

    let (stdout, stderr, exit_code) = run_qalc_rs_args(
        &["-set", "precision 128", "--", "rk([1 2 3; 3 6 9])"],
        Some("1"),
        Some("1"),
    );
    assert!(stdout.is_empty());
    assert!(stderr.contains("[qalc-rs-metadata] fallback=disabled"));
    assert!(stderr.contains("error: calculation failed: C++ FFI fallback is disabled"));
    assert_eq!(exit_code, 2);

    let (stdout, stderr, exit_code) = run_qalc_rs_args(
        &[
            "-set",
            "precision 128",
            "--",
            "rref([1 3 1 9; 1 1 -1 1; 3 11 5 35])",
        ],
        Some("1"),
        Some("1"),
    );
    assert!(stdout.is_empty());
    assert!(stderr.contains("[qalc-rs-metadata] fallback=disabled"));
    assert!(stderr.contains("error: calculation failed: C++ FFI fallback is disabled"));
    assert_eq!(exit_code, 2);

    let (stdout, stderr, exit_code) = run_qalc_rs_args(
        &["-set", "precision 128", "--", "combine([1, 2])"],
        Some("1"),
        Some("1"),
    );
    assert!(stdout.is_empty());
    assert!(stderr.contains("[qalc-rs-metadata] fallback=disabled"));
    assert!(stderr.contains("error: calculation failed: C++ FFI fallback is disabled"));
    assert_eq!(exit_code, 2);

    let (stdout, stderr, exit_code) = run_qalc_rs_args(
        &["-set", "precision 128", "--", "(1; 2; 3).(4; 5; 6)"],
        Some("1"),
        Some("1"),
    );
    assert!(stdout.is_empty());
    assert!(stderr.contains("[qalc-rs-metadata] fallback=disabled"));
    assert!(stderr.contains("error: calculation failed: C++ FFI fallback is disabled"));
    assert_eq!(exit_code, 2);

    let (stdout, stderr, exit_code) = run_qalc_rs_args(
        &[
            "-set",
            "precision 128",
            "--",
            "horzcat([1], [2 3], [4 5 6 7])",
        ],
        Some("1"),
        Some("1"),
    );
    assert!(stdout.is_empty());
    assert!(stderr.contains("[qalc-rs-metadata] fallback=disabled"));
    assert!(stderr.contains("error: calculation failed: C++ FFI fallback is disabled"));
    assert_eq!(exit_code, 2);

    let (stdout, stderr, exit_code) = run_qalc_rs_args(
        &[
            "-set",
            "precision 128",
            "--",
            "vertcat([1 2], [3 4], [5 6])",
        ],
        Some("1"),
        Some("1"),
    );
    assert!(stdout.is_empty());
    assert!(stderr.contains("[qalc-rs-metadata] fallback=disabled"));
    assert!(stderr.contains("error: calculation failed: C++ FFI fallback is disabled"));
    assert_eq!(exit_code, 2);

    let (stdout, stderr, exit_code) = run_qalc_rs_args(
        &["-set", "precision 128", "--", "dot((1; 2); (3, 4))"],
        Some("1"),
        Some("1"),
    );
    assert!(stdout.is_empty());
    assert!(stderr.contains("[qalc-rs-metadata] fallback=disabled"));
    assert!(stderr.contains("error: calculation failed: C++ FFI fallback is disabled"));
    assert_eq!(exit_code, 2);

    let (stdout, stderr, exit_code) = run_qalc_rs_args(
        &[
            "-set",
            "precision 128",
            "--",
            "slice([5, 6, 7, 8, 9], 2, 4)",
        ],
        Some("1"),
        Some("1"),
    );
    assert!(stdout.is_empty());
    assert!(stderr.contains("[qalc-rs-metadata] fallback=disabled"));
    assert!(stderr.contains("error: calculation failed: C++ FFI fallback is disabled"));
    assert_eq!(exit_code, 2);

    let (stdout, stderr, exit_code) = run_qalc_rs_args(
        &[
            "-set",
            "precision 128",
            "--",
            "sort([5, 2, 0, 1, 3, -4, 0], 0)",
        ],
        Some("1"),
        Some("1"),
    );
    assert!(stdout.is_empty());
    assert!(stderr.contains("[qalc-rs-metadata] fallback=disabled"));
    assert!(stderr.contains("error: calculation failed: C++ FFI fallback is disabled"));
    assert_eq!(exit_code, 2);

    let (stdout, stderr, exit_code) = run_qalc_rs_args(
        &["-set", "precision 128", "--", "rank([-1, 2, 5, 10], 0)"],
        Some("1"),
        Some("1"),
    );
    assert!(stdout.is_empty());
    assert!(stderr.contains("[qalc-rs-metadata] fallback=disabled"));
    assert!(stderr.contains("error: calculation failed: C++ FFI fallback is disabled"));
    assert_eq!(exit_code, 2);

    let (stdout, stderr, exit_code) = run_qalc_rs_args(
        &["-set", "precision 128", "--", "entrywise(x, [4 10 12], x)"],
        Some("1"),
        Some("1"),
    );
    assert!(stdout.is_empty());
    assert!(stderr.contains("[qalc-rs-metadata] fallback=disabled"));
    assert!(stderr.contains("error: calculation failed: C++ FFI fallback is disabled"));
    assert_eq!(exit_code, 2);

    let (stdout, stderr, exit_code) = run_qalc_rs_args(
        &["-set", "precision 128", "--", "[2 4; 3 4].^[-1; 2]"],
        Some("1"),
        Some("1"),
    );
    assert!(stdout.is_empty());
    assert!(stderr.contains("[qalc-rs-metadata] fallback=disabled"));
    assert!(stderr.contains("error: calculation failed: C++ FFI fallback is disabled"));
    assert_eq!(exit_code, 2);

    let (stdout, stderr, exit_code) = run_qalc_rs_args(
        &["-set", "precision 128", "--", "[1 2; 3 4].^3"],
        Some("1"),
        Some("1"),
    );
    assert!(stdout.is_empty());
    assert!(stderr.contains("[qalc-rs-metadata] fallback=disabled"));
    assert!(stderr.contains("error: calculation failed: C++ FFI fallback is disabled"));
    assert_eq!(exit_code, 2);

    let (stdout, stderr, exit_code) = run_qalc_rs_args(
        &["-set", "precision 128", "--", "transpose([1 2; 3 4])"],
        Some("1"),
        Some("1"),
    );
    assert!(stdout.is_empty());
    assert!(stderr.contains("[qalc-rs-metadata] fallback=disabled"));
    assert!(stderr.contains("error: calculation failed: C++ FFI fallback is disabled"));
    assert_eq!(exit_code, 2);

    let (stdout, stderr, exit_code) = run_qalc_rs_args(
        &["-set", "precision 128", "--", "[1 2 3; 4 5 6].'"],
        Some("1"),
        Some("1"),
    );
    assert!(stdout.is_empty());
    assert!(stderr.contains("[qalc-rs-metadata] fallback=disabled"));
    assert!(stderr.contains("error: calculation failed: C++ FFI fallback is disabled"));
    assert_eq!(exit_code, 2);

    let (stdout, stderr, exit_code) = run_qalc_rs_args(
        &["-set", "precision 128", "--", "norm([3, 4])"],
        Some("1"),
        Some("1"),
    );
    assert!(stdout.is_empty());
    assert!(stderr.contains("[qalc-rs-metadata] fallback=disabled"));
    assert!(stderr.contains("error: calculation failed: C++ FFI fallback is disabled"));
    assert_eq!(exit_code, 2);

    let (stdout, stderr, exit_code) = run_qalc_rs_args(
        &["-set", "precision 128", "--", "adj([1 2; 4 5])"],
        Some("1"),
        Some("1"),
    );
    assert!(stdout.is_empty());
    assert!(stderr.contains("[qalc-rs-metadata] fallback=disabled"));
    assert!(stderr.contains("error: calculation failed: C++ FFI fallback is disabled"));
    assert_eq!(exit_code, 2);

    let (stdout, stderr, exit_code) = run_qalc_rs_args(
        &["-set", "precision 128", "--", "cofactor([1 2; 4 5], 1, 1)"],
        Some("1"),
        Some("1"),
    );
    assert!(stdout.is_empty());
    assert!(stderr.contains("[qalc-rs-metadata] fallback=disabled"));
    assert!(stderr.contains("error: calculation failed: C++ FFI fallback is disabled"));
    assert_eq!(exit_code, 2);

    let (stdout, stderr, exit_code) = run_qalc_rs_args(
        &["-set", "precision 128", "--", "permanent([1 2; 4 5])"],
        Some("1"),
        Some("1"),
    );
    assert!(stdout.is_empty());
    assert!(stderr.contains("[qalc-rs-metadata] fallback=disabled"));
    assert!(stderr.contains("error: calculation failed: C++ FFI fallback is disabled"));
    assert_eq!(exit_code, 2);

    let (stdout, stderr, exit_code) = run_qalc_rs_args(
        &["-set", "precision 128", "--", "cross((1; 2; 3); (4; 5; 6))"],
        Some("1"),
        Some("1"),
    );
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
        "Ei(3+/-0.3)",
        "matrix(0, 3, [])",
        "matrix(3, 0, [])",
        "[1,2] + [3,4,5]",
        "[2; 4]./[1 2]",
        "[1 2]times 3",
        "[1 2] times3",
        "columns([[1], [2,3]])",
        "multiply([1 2])",
        "multiply(1, 2)",
        "multiply([1 2; 3 4], [5 6; 7 8])",
        "hadamard([2], [3])",
        "hadamard([1 2], [3 4])",
        "hadamard([1 2], [3 4 5])",
        "hadamard([1; 2], [3 4])",
        "hadamard(1, 2)",
        "identity(2)",
        "identity([1 2])",
        "mergevectors([1, 2])",
        " combine([1, 2])",
        "combine([1, 2]) ",
        "combine ([1, 2])",
        "combine( [1, 2])",
        "combine([1, 2] )",
        "combine([1,2])",
        "combine([1, 2], [3])",
        "combine([1, 2], [3], [4, 5, 6], [7])",
        "combine([1.0, 2])",
        "combine([1 2; 3 4])",
        "cat([1], [2 3], [4 5 6 7])",
        " horzcat([1], [2 3], [4 5 6 7])",
        "horzcat([1], [2 3], [4 5 6 7]) ",
        "horzcat ([1], [2 3], [4 5 6 7])",
        "horzcat([1],[2 3],[4 5 6 7])",
        "horzcat([1], [2, 3], [4 5 6 7])",
        "horzcat([1], [2 3])",
        "horzcat([1], [2 3], [4 5 6 7], [8])",
        "horzcat([1; 2], [3 4; 5 6], [7 8 9])",
        " vertcat([1 2], [3 4], [5 6])",
        "vertcat([1 2], [3 4], [5 6]) ",
        "vertcat ([1 2], [3 4], [5 6])",
        "vertcat([1, 2], [3 4], [5 6])",
        "vertcat([1 2], [3 4])",
        "vertcat([1 2], [3 4], [5 6], [7 8])",
        "vertcat([1 2], [3 4 5], [6 7])",
        " dot((2); (3))",
        "dot((2); (3)) ",
        "dot ((2); (3))",
        "dot((2), (3))",
        "dot((1; 2); (3,4))",
        "dot((1; 2); (3, 4); (5))",
        "dot((1; 2); (3, 5))",
        "dot((1; 2; 3); (4; 5))",
        "dot((1.0; 2); (3, 4))",
        "dot([1 2; 3 4]; [5 6; 7 8])",
        " (1; 2; 3).(4; 5; 6)",
        "(1; 2; 3).(4; 5; 6) ",
        "(1;2;3).(4;5;6)",
        "(1; 2; 3) . (4; 5; 6)",
        "(1; 2; 3).(4; 5)",
        "(1.0; 2; 3).(4; 5; 6)",
        "[1 2; 3 4].[5 6; 7 8]",
        " cross((1; 2; 3); (4; 5; 6))",
        "cross((1; 2; 3); (4; 5; 6)) ",
        "cross ((1; 2; 3); (4; 5; 6))",
        "cross((1;2;3); (4;5;6))",
        "cross((1; 2; 3), (4; 5; 6))",
        "cross((1; 2; 3); (4; 5; 6); (7; 8; 9))",
        "cross((1; 2; 3))",
        "cross((1; 2); (4; 5))",
        "cross((1.0; 2; 3); (4; 5; 6))",
        "cross([[1]]; [[2]])",
        "magnitude(2)",
        "magnitude(-2.0)",
        "magnitude(-4/2)",
        "magnitude([3, 4])",
        "magnitude([-2.0])",
        "magnitude([-4/2])",
        "magnitude([1, 2, 3])",
        "magnitude([-2.0, 3, 4])",
        "magnitude([1 2; 3 4])",
        "norm([2.0])",
        "norm([4/2])",
        " norm([2])",
        "norm([2]) ",
        " norm([2]) ",
        "norm ([2])",
        "norm( [2])",
        "norm([2] )",
        "norm([3,4])",
        "norm([3, 4] )",
        "norm([3, 4.0])",
        "norm([2, 3, 6, 0])",
        "norm([1 2; 3 4])",
        "adj([1 2; 4 5]) ",
        " adj([1 2; 4 5])",
        "adj ([1 2; 4 5])",
        "adj([1 2])",
        "adj([[1]])",
        "adj([1.0 2; 4 5])",
        "adj([1 2; 4 5], 1)",
        "cofactor([1 2; 4 5], 1, 1) ",
        " cofactor([1 2; 4 5], 1, 1)",
        "cofactor ([1 2; 4 5], 1, 1)",
        "cofactor([1 2; 4 5], 1.0, 1)",
        "cofactor([1 2; 4 5], 0, 1)",
        "cofactor([1 2; 4 5], 1, 0)",
        "cofactor([1 2; 4 5], 3, 1)",
        "cofactor([1 2; 4 5], 1, 3)",
        "cofactor([1 2], 1, 1)",
        "cofactor([[1]], 1, 1)",
        "cofactor([1.0 2; 4 5], 1, 1)",
        "cofactor([1 2; 4 5], 1, 1, 1)",
        "permanent([1]) ",
        " permanent([1])",
        "permanent ([1])",
        "permanent([1.0])",
        "permanent([1 2])",
        "permanent(1)",
        "permanent([1], 1)",
        "det([[1.0]])",
        "det([1 2])",
        "det(1)",
        "det([1 2; 3 4], 2)",
        " det([[1]])",
        "det([[1]]) ",
        "det ([[1]])",
        "det( [[1]])",
        "det([[1]] )",
        "part([1], 1.0, 1, 1, 1)",
        "part([1], 1, 1, 1)",
        "part([1, 2], 1, 1, 1, 1)",
        "part([1 2 3; 4 5 6; 7 8 9; 10 11 12], 1, 1, 1, 1)",
        " slice([5], 1, 1)",
        "slice([5], 1, 1) ",
        "slice ([5], 1, 1)",
        "slice([5],1,1)",
        "slice([5], 1, 1, 1)",
        "slice([5.0], 1, 1)",
        "slice([5, 6, 7, 8, 9], 2.0, 4)",
        "slice([5, 6, 7, 8, 9], 4, 2)",
        "slice([5, 6, 7, 8, 9], 2, 5)",
        "slice([5 6; 7 8], 1, 2)",
        " sort([5, 2, 0, 1, 3, -4, 0])",
        "sort([5, 2, 0, 1, 3, -4, 0]) ",
        "sort ([5, 2, 0, 1, 3, -4, 0])",
        "sort([5,2,0,1,3,-4,0])",
        "sort([5, 2, 0, 1, 3, -4, 0], 2)",
        "sort([5, 2, 0, 1, 3, -4, 0], 1.0)",
        "sort([5, 2, 0, 1, 3, -4])",
        "sort([5, 2, 0, 1, 3, -4, 0, 0])",
        "sort([5.0, 2, 0, 1, 3, -4, 0])",
        "sort([5 2; 0 1], 1)",
        " rank([6, 7, 1, 4])",
        "rank([6, 7, 1, 4]) ",
        "rank ([6, 7, 1, 4])",
        "rank([6,7,1,4])",
        "rank([6, 7, 1, 4], 2)",
        "rank([6, 7, 1, 4], 1.0)",
        "rank([6, 7, 1])",
        "rank([6, 7, 1, 4, 0])",
        "rank([6.0, 7, 1, 4])",
        "rank([6 7; 1 4])",
        "rank([-1,2,5,10], 1)",
        "rank([-1, 2, 5, 10], 2)",
        "rank([-1, 2, 5, 11], 1)",
        " entrywise(x, [4 10 12], x)",
        "entrywise(x, [4 10 12], x) ",
        "entrywise(x,[4 10 12],x)",
        "entrywise(y, [4 10 12], x)",
        "entrywise(x, [4 10 11], x)",
        "entrywise(x, [4 10 12 0], x)",
        "entrywise(x / y, [4 10 12], x, [2 2 0], y)",
        "entrywise(x / y, [4 10 12], x, [2 2], y)",
        "entrywise(x / y + z, [4 10 12], x, [2 2 4], y, [1 2 4], z)",
        "pow([1 2; 3 4])",
        "pow([1 2; 3 4], 2, 3)",
        "pow(1, 2)",
        "pow([1 2], [3 4 5])",
        "[1 2].^[3 4 5]",
        " transpose([1 2; 3 4])",
        "transpose([1 2; 3 4]) ",
        "transpose ([1 2; 3 4])",
        "transpose([1, 2; 3, 4])",
        "transpose([1 2])",
        "transpose([1 2; 3 4], 1)",
        "transpose([1.0 2; 3 4])",
        " [1 2 3; 4 5 6].'",
        "[1 2 3; 4 5 6].' ",
        "[1 2 3; 4 5 6] .'",
        "[1 2; 3 4].'",
        "[1 2 3; 4 5 6].t",
        "divide([1], 0+/-1)",
        "divide(1, [2 4])",
        "rdivide([1; 2], [3 4])",
        "1/[2,3]",
        "(1 + i) < (1 + 2i)",
        "(1 + i) <= (1 + 2i)",
        "(1 + i) > (1 + 2i)",
        "(1 + i) >= (1 + 2i)",
        "(1 + i) ≤ (1 + 2i)",
        "(1 + i) ≥ (1 + 2i)",
        "infinity + -infinity",
        "infinity - infinity",
        "0 * infinity",
        "0 / 0",
        "170141183460469231731687303715884105728 + 1",
        "rk([1 2])",
        "rk([1 2 3; 3 6 8])",
        "rk([1.0 2 3; 3 6 9])",
        "rk([1 2 3; 3 6 9], 1)",
        "rk(1)",
        "rk(identity(2))",
        " rk(identity(3))",
        "rk(identity(3)) ",
        "rk (identity(3))",
        "rref([1 3 1 9; 1 1 -1 1; 3 11 5 34])",
        "rref([1.0 3 1 9; 1 1 -1 1; 3 11 5 35])",
        "rref([1 3 1 9; 1 1 -1 1; 3 11 5 35], 1)",
        "rref([1 3 1 9])",
        "rref(1)",
        " rref([1 3 1 9; 1 1 -1 1; 3 11 5 35])",
        "rref([1 3 1 9; 1 1 -1 1; 3 11 5 35]) ",
        "rref ([1 3 1 9; 1 1 -1 1; 3 11 5 35])",
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
