use libqalculate_rust::ffi::Calculator;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, Once};
use tempfile::tempdir;

static TEST_MUTEX: Mutex<()> = Mutex::new(());
static DEFINITIONS_DIR: Once = Once::new();

fn configure_definitions_dir() {
    DEFINITIONS_DIR.call_once(|| {
        let path = Path::new("../libqalculate/data")
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from("../libqalculate/data"));
        std::env::set_var("QALCULATE_DEFINITIONS_DIR", path);
    });
}

// Helper to get Resident Set Size (RSS) in bytes on Linux.
fn get_rss() -> usize {
    if let Ok(statm) = fs::read_to_string("/proc/self/statm") {
        let fields: Vec<&str> = statm.split_whitespace().collect();
        if let Some(rss_pages_str) = fields.get(1) {
            if let Ok(rss_pages) = rss_pages_str.parse::<usize>() {
                return rss_pages * 4096; // Page size is typically 4KB
            }
        }
    }
    0
}

#[test]
fn test_memory_cleanup_and_drop() {
    if std::env::var("QALCULATE_DISABLE_FALLBACK").as_deref() == Ok("1") {
        return;
    }
    let _guard = TEST_MUTEX.lock().unwrap();
    configure_definitions_dir();

    // Warm up phase to settle static / initial library allocations (e.g. static C++ data)
    for _ in 0..100 {
        let mut calc = Calculator::new();
        let _ = calc.load_global_definitions();
        let _ = calc.calculate_and_print("1 + 1", 1000);
    }

    let base_rss = get_rss();

    // Loop a large number of times to check for memory leaks
    for i in 0..2000 {
        let mut calc = Calculator::new();
        // Alternating loading definitions and not loading to test different C++ paths
        if i % 2 == 0 {
            calc.load_global_definitions();
        }
        let res = calc.calculate_and_print("123 * 456 + sin(45) - log(10)", 1000);
        assert!(res.is_ok(), "Calculation failed during loop: {:?}", res);
        let res_str = res.unwrap();
        assert!(!res_str.is_empty(), "Empty result returned");
    }

    let end_rss = get_rss();

    if base_rss > 0 && end_rss > 0 {
        let diff = end_rss.saturating_sub(base_rss);
        println!(
            "Memory stress test: base RSS = {} bytes, end RSS = {} bytes, diff = {} bytes",
            base_rss, end_rss, diff
        );
        // Allow a small delta (e.g. 5 MB) for allocator fragmentation/retention, but block on actual memory leaks
        // A single Calculator with loaded definitions is relatively large. Leaking 2000 of them would leak many megabytes/gigabytes.
        assert!(
            diff < 5 * 1024 * 1024,
            "Potential memory leak detected: RSS grew by {} bytes",
            diff
        );
    }
}

#[test]
fn test_exception_safety_invalid_inputs() {
    let _guard = TEST_MUTEX.lock().unwrap();
    configure_definitions_dir();

    let mut calc = Calculator::new();
    calc.load_global_definitions();

    // Verify behavior for various invalid input strings
    let invalid_inputs = vec![
        "1 + ",
        "((1+2",
        "nonexistent_function(1, 2)",
        "",
        "1 / 0",
        "nested_unclosed(func(1",
    ];

    for expr in invalid_inputs {
        println!("Evaluating expression: '{}'", expr);
        let result = calc.calculate_and_print(expr, 1000);
        // Ensure it returns Result (either Ok containing an error string, or Err(cxx::Exception)),
        // but crucially, it must NOT crash or abort the process.
        match result {
            Ok(output) => {
                println!("Result for '{}': Ok('{}')", expr, output);
            }
            Err(e) => {
                println!("Result for '{}': Err('{}')", expr, e);
            }
        }
    }
}

#[test]
fn test_thread_safety_compile_fail_checks() {
    let _guard = TEST_MUTEX.lock().unwrap();
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let proj_dir = std::env::current_dir().expect("Failed to get current directory");

    let deps_dir = proj_dir.join("target/debug/deps");
    let mut rlib_path = None;
    for entry in std::fs::read_dir(&deps_dir)
        .expect("failed to read deps dir")
        .flatten()
    {
        let path = entry.path();
        if path.is_file() {
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if filename.starts_with("liblibqalculate_rust-") && filename.ends_with(".rlib") {
                    rlib_path = Some(path);
                    break;
                }
            }
        }
    }
    let rlib = rlib_path.expect("Could not find liblibqalculate_rust rlib for compile test");

    // Test case 1: Send compilation failure
    {
        let main_rs_path = temp_dir.path().join("main_send.rs");
        let main_rs = r#"use libqalculate_rust::ffi::Calculator;
fn assert_send<T: Send>() {}
fn main() {
    assert_send::<Calculator>();
}
"#;
        fs::write(&main_rs_path, main_rs).unwrap();

        let output = Command::new("rustc")
            .arg(&main_rs_path)
            .arg("--crate-type")
            .arg("bin")
            .arg("--extern")
            .arg(format!("libqalculate_rust={}", rlib.display()))
            .arg("-L")
            .arg(format!("dependency={}", deps_dir.display()))
            .arg("--edition")
            .arg("2021")
            .arg("--out-dir")
            .arg(temp_dir.path())
            .output()
            .expect("Failed to run rustc for Send test");

        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            !output.status.success(),
            "Compilation succeeded, but sending Calculator across threads should fail!\nstdout:\n{}\nstderr:\n{}",
            stdout,
            stderr
        );
        assert!(
            stderr.contains("cannot be sent between threads safely")
                || stderr.contains("`*mut ()` cannot be sent between threads safely"),
            "Expected Send compilation failure message, but got:\n{}",
            stderr
        );
        println!(
            "Verified: Calculator cannot be Sent across threads (compilation failed as expected)."
        );
    }

    // Test case 2: Sync compilation failure
    {
        let main_rs_path = temp_dir.path().join("main_sync.rs");
        let main_rs = r#"use libqalculate_rust::ffi::Calculator;
fn assert_sync<T: Sync>() {}
fn main() {
    assert_sync::<Calculator>();
}
"#;
        fs::write(&main_rs_path, main_rs).unwrap();

        let output = Command::new("rustc")
            .arg(&main_rs_path)
            .arg("--crate-type")
            .arg("bin")
            .arg("--extern")
            .arg(format!("libqalculate_rust={}", rlib.display()))
            .arg("-L")
            .arg(format!("dependency={}", deps_dir.display()))
            .arg("--edition")
            .arg("2021")
            .arg("--out-dir")
            .arg(temp_dir.path())
            .output()
            .expect("Failed to run rustc for Sync test");

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !output.status.success(),
            "Compilation succeeded, but sharing Calculator across threads should fail!"
        );
        assert!(
            stderr.contains("cannot be shared between threads safely")
                || stderr.contains("`*mut ()` cannot be shared between threads safely"),
            "Expected Sync compilation failure message, but got:\n{}",
            stderr
        );
        println!("Verified: Calculator cannot be Shared (Sync) across threads (compilation failed as expected).");
    }
}
