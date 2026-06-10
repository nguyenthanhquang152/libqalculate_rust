use libqalculate_rust::ffi::Calculator;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, Once};

// Static mutex to serialize test execution because the C++ libqalculate library
// is not thread-safe to initialize/load definitions concurrently within the same process.
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

// Helper to get RSS memory usage in KB on Linux
fn get_vm_rss() -> Option<usize> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if line.starts_with("VmRSS:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(val) = parts[1].parse::<usize>() {
                    return Some(val);
                }
            }
        }
    }
    None
}

#[test]
fn test_memory_cleanup_calculations_loop() {
    if std::env::var("QALCULATE_DISABLE_FALLBACK").as_deref() == Ok("1") {
        return;
    }
    let _guard = TEST_MUTEX.lock().unwrap();
    configure_definitions_dir();

    let mut calc = Calculator::new();
    calc.load_global_definitions();

    // Warm up
    for _ in 0..10 {
        let _ = calc.calculate_and_print("1 + 1", 1000);
    }

    let start_rss = get_vm_rss();

    // Run 2000 calculations in a loop
    for i in 0..2000 {
        let expr = format!("{} + {}", i, i * 2);
        let res = calc
            .calculate_and_print(&expr, 1000)
            .expect("Calculation failed");
        assert_eq!(res, (i + i * 2).to_string());
    }

    let end_rss = get_vm_rss();
    if let (Some(start), Some(end)) = (start_rss, end_rss) {
        println!("RSS before loop: {} KB, RSS after loop: {} KB", start, end);
        // We expect memory to be stable. Allow a small buffer for allocator fragmentation/metadata (e.g. 500KB)
        let diff = end.saturating_sub(start);
        assert!(
            diff < 500,
            "Potential memory leak in calculate_and_print loop! RSS grew by {} KB",
            diff
        );
    }
}

#[test]
fn test_memory_cleanup_calculator_creation_loop() {
    if std::env::var("QALCULATE_DISABLE_FALLBACK").as_deref() == Ok("1") {
        return;
    }
    let _guard = TEST_MUTEX.lock().unwrap();
    configure_definitions_dir();

    for _ in 0..10 {
        let mut calc = Calculator::new();
        calc.load_global_definitions();
        let _ = calc
            .calculate_and_print("sin(pi/2)", 1000)
            .expect("warm-up calculation failed");
    }

    let start_rss = get_vm_rss();

    // Create and drop Calculator in a loop 1000 times
    for i in 0..1000 {
        let mut calc = Calculator::new();
        // Load definitions to make it use heap memory
        calc.load_global_definitions();
        let res = calc
            .calculate_and_print("sin(pi/2)", 1000)
            .expect("Calculation failed");
        assert_eq!(res, "1");
        if i % 200 == 0 {
            if let Some(rss) = get_vm_rss() {
                println!("Iteration {}: RSS = {} KB", i, rss);
            }
        }
        // calc drops here
    }

    let end_rss = get_vm_rss();
    if let (Some(start), Some(end)) = (start_rss, end_rss) {
        println!(
            "RSS before creation loop: {} KB, RSS after creation loop: {} KB",
            start, end
        );
        let diff = end.saturating_sub(start);
        // If it grows linearly, 1000 iterations would grow memory significantly.
        // Let's assert a higher threshold or verify stabilization.
        assert!(
            diff < 6000,
            "Potential memory leak in Calculator creation and drop! RSS grew by {} KB",
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

    // Pass mathematically invalid expressions
    // C++ libqalculate will handle syntax errors and return them in the output string or throw.
    // We verify that it doesn't crash the process.
    let invalid_expressions = [
        "1 +",
        "invalid_function(5)",
        "((((1",
        "5 / 0", // division by zero
        "",      // empty string
    ];

    for expr in &invalid_expressions {
        let res = calc.calculate_and_print(expr, 1000);
        match res {
            Ok(val) => {
                println!("Expression '{}' returned output: '{}'", expr, val);
            }
            Err(e) => {
                println!("Expression '{}' returned C++ exception: {:?}", expr, e);
            }
        }
    }

    // Try a string with a null byte
    let res_null = calc.calculate_and_print("1 + \0 2", 1000);
    println!("Expression with null byte returned: {:?}", res_null);

    // Try an expression that triggers timeout
    let res_timeout = calc.calculate_and_print("10^10^10^10", 1);
    println!("Timeout expression returned: {:?}", res_timeout);
}

#[test]
fn test_thread_safety_compile_fail() {
    let _guard = TEST_MUTEX.lock().unwrap();

    // Locate the current test binary to find target/debug/deps directory
    let current_exe = std::env::current_exe().expect("failed to get current exe path");
    let deps_dir = current_exe.parent().expect("failed to get deps directory");

    // Find the liblibqalculate_rust-*.rlib file in deps_dir
    let mut rlib_path = None;
    for entry in std::fs::read_dir(deps_dir)
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
    println!("Found rlib path: {}", rlib.display());

    // Compile-fail test for Send
    let send_file = "tests/fixtures/compile_fail/send.rs";
    let output_send = Command::new("rustc")
        .arg("--crate-type=bin")
        .arg("--edition=2021")
        .arg("-L")
        .arg(format!("dependency={}", deps_dir.display()))
        .arg("--extern")
        .arg(format!("libqalculate_rust={}", rlib.display()))
        .arg(send_file)
        .arg("-o")
        .arg(deps_dir.join("send_test_bin"))
        .output()
        .expect("Failed to run rustc");

    assert!(
        !output_send.status.success(),
        "send.rs compiled successfully, but it should have failed!"
    );
    let stderr_send = String::from_utf8_lossy(&output_send.stderr);
    assert!(
        stderr_send.contains("cannot be sent between threads safely"),
        "Unexpected compile error for send.rs:\n{}",
        stderr_send
    );
    println!("Send trait compile-fail test passed successfully.");

    // Compile-fail test for Sync
    let sync_file = "tests/fixtures/compile_fail/sync.rs";
    let output_sync = Command::new("rustc")
        .arg("--crate-type=bin")
        .arg("--edition=2021")
        .arg("-L")
        .arg(format!("dependency={}", deps_dir.display()))
        .arg("--extern")
        .arg(format!("libqalculate_rust={}", rlib.display()))
        .arg(sync_file)
        .arg("-o")
        .arg(deps_dir.join("sync_test_bin"))
        .output()
        .expect("Failed to run rustc");

    assert!(
        !output_sync.status.success(),
        "sync.rs compiled successfully, but it should have failed!"
    );
    let stderr_sync = String::from_utf8_lossy(&output_sync.stderr);
    assert!(
        stderr_sync.contains("cannot be shared between threads safely"),
        "Unexpected compile error for sync.rs:\n{}",
        stderr_sync
    );
    println!("Sync trait compile-fail test passed successfully.");
}
