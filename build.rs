use std::env;

fn main() {
    // Tell Cargo to rerun this script if the C++ sources or this script change
    println!("cargo:rerun-if-changed=build.rs");

    let upstream_dir =
        env::var("LIBQALCULATE_UPSTREAM_DIR").unwrap_or_else(|_| "../libqalculate".to_string());

    // Monitor all C++ header files in upstream_dir/libqalculate
    if let Ok(entries) = std::fs::read_dir(format!("{}/libqalculate", upstream_dir)) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "h" {
                        if let Some(path_str) = path.to_str() {
                            println!("cargo:rerun-if-changed={}", path_str);
                        }
                    }
                }
            }
        }
    }

    // Discover libxml2 include path and libs using pkg-config
    let xml2 = pkg_config::probe_library("libxml-2.0")
        .expect("libxml-2.0 developer package is required to compile libqalculate");

    // Configure C++ static compilation using the cc crate
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++17")
        .warnings(false) // Suppress upstream compiler warnings to avoid build log noise
        .include(&upstream_dir)
        .include(format!("{}/libqalculate", upstream_dir))
        .define("VERSION", "\"5.11.0\"")
        .define("PACKAGE_DATA_DIR", "\"/usr/share\"")
        .define("PACKAGE_LOCALE_DIR", "\"/usr/share/locale\"");

    // Add libxml2 include paths
    for path in &xml2.include_paths {
        build.include(path);
    }

    // Add all 41 core source files
    let source_files = [
        "BuiltinFunctions-algebra.cc",
        "BuiltinFunctions-calculus.cc",
        "BuiltinFunctions-combinatorics.cc",
        "BuiltinFunctions-datetime.cc",
        "BuiltinFunctions-explog.cc",
        "BuiltinFunctions-logical.cc",
        "BuiltinFunctions-matrixvector.cc",
        "BuiltinFunctions-number.cc",
        "BuiltinFunctions-special.cc",
        "BuiltinFunctions-statistics.cc",
        "BuiltinFunctions-trigonometry.cc",
        "BuiltinFunctions-util.cc",
        "Calculator-calculate.cc",
        "Calculator-convert.cc",
        "Calculator-definitions.cc",
        "Calculator-parse.cc",
        "Calculator-plot.cc",
        "Calculator.cc",
        "DataSet.cc",
        "ExpressionItem.cc",
        "Function.cc",
        "MathStructure-calculate.cc",
        "MathStructure-convert.cc",
        "MathStructure-decompose.cc",
        "MathStructure-differentiate.cc",
        "MathStructure-eval.cc",
        "MathStructure-factor.cc",
        "MathStructure-gcd.cc",
        "MathStructure-integrate.cc",
        "MathStructure-isolatex.cc",
        "MathStructure-limit.cc",
        "MathStructure-matrixvector.cc",
        "MathStructure-polynomial.cc",
        "MathStructure-print.cc",
        "MathStructure.cc",
        "Number.cc",
        "Prefix.cc",
        "QalculateDateTime.cc",
        "Unit.cc",
        "Variable.cc",
        "util.cc",
    ];

    for file in &source_files {
        build.file(format!("{}/libqalculate/{}", upstream_dir, file));
    }

    // Compile the static library
    build.compile("qalculate");

    // Compile the C++ FFI bridge separately
    let mut bridge_build = cxx_build::bridge("src/ffi.rs");
    bridge_build
        .file("src/ffi_bridge.cc")
        .std("c++17")
        .warnings(false)
        .include(&upstream_dir)
        .include(format!("{}/libqalculate", upstream_dir));

    for path in &xml2.include_paths {
        bridge_build.include(path);
    }

    bridge_build.compile("qalculate_bridge");

    println!("cargo:rerun-if-changed=src/ffi.rs");
    println!("cargo:rerun-if-changed=src/ffi_bridge.h");
    println!("cargo:rerun-if-changed=src/ffi_bridge.cc");

    // Link necessary system libraries
    println!("cargo:rustc-link-lib=gmp");
    println!("cargo:rustc-link-lib=mpfr");

    // Link C++ runtime library
    let target = env::var("TARGET").unwrap();
    if target.contains("apple") || target.contains("freebsd") {
        println!("cargo:rustc-link-lib=dylib=c++");
    } else if !target.contains("msvc") {
        println!("cargo:rustc-link-lib=dylib=stdc++");
    }
}
