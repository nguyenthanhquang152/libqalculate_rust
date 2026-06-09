use std::env;

fn main() {
    // Tell Cargo to rerun this script if the C++ sources or this script change
    println!("cargo:rerun-if-changed=build.rs");
    // Monitor all C++ header files in ../libqalculate/libqalculate
    if let Ok(entries) = std::fs::read_dir("../libqalculate/libqalculate") {
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
        .include("../libqalculate")
        .include("../libqalculate/libqalculate")
        .define("VERSION", "\"5.11.0\"")
        .define("PACKAGE_DATA_DIR", "\"/usr/share\"")
        .define("PACKAGE_LOCALE_DIR", "\"/usr/share/locale\"");

    // Add libxml2 include paths
    for path in xml2.include_paths {
        build.include(path);
    }

    // Add all 41 core source files
    let source_files = [
        "../libqalculate/libqalculate/BuiltinFunctions-algebra.cc",
        "../libqalculate/libqalculate/BuiltinFunctions-calculus.cc",
        "../libqalculate/libqalculate/BuiltinFunctions-combinatorics.cc",
        "../libqalculate/libqalculate/BuiltinFunctions-datetime.cc",
        "../libqalculate/libqalculate/BuiltinFunctions-explog.cc",
        "../libqalculate/libqalculate/BuiltinFunctions-logical.cc",
        "../libqalculate/libqalculate/BuiltinFunctions-matrixvector.cc",
        "../libqalculate/libqalculate/BuiltinFunctions-number.cc",
        "../libqalculate/libqalculate/BuiltinFunctions-special.cc",
        "../libqalculate/libqalculate/BuiltinFunctions-statistics.cc",
        "../libqalculate/libqalculate/BuiltinFunctions-trigonometry.cc",
        "../libqalculate/libqalculate/BuiltinFunctions-util.cc",
        "../libqalculate/libqalculate/Calculator-calculate.cc",
        "../libqalculate/libqalculate/Calculator-convert.cc",
        "../libqalculate/libqalculate/Calculator-definitions.cc",
        "../libqalculate/libqalculate/Calculator-parse.cc",
        "../libqalculate/libqalculate/Calculator-plot.cc",
        "../libqalculate/libqalculate/Calculator.cc",
        "../libqalculate/libqalculate/DataSet.cc",
        "../libqalculate/libqalculate/ExpressionItem.cc",
        "../libqalculate/libqalculate/Function.cc",
        "../libqalculate/libqalculate/MathStructure-calculate.cc",
        "../libqalculate/libqalculate/MathStructure-convert.cc",
        "../libqalculate/libqalculate/MathStructure-decompose.cc",
        "../libqalculate/libqalculate/MathStructure-differentiate.cc",
        "../libqalculate/libqalculate/MathStructure-eval.cc",
        "../libqalculate/libqalculate/MathStructure-factor.cc",
        "../libqalculate/libqalculate/MathStructure-gcd.cc",
        "../libqalculate/libqalculate/MathStructure-integrate.cc",
        "../libqalculate/libqalculate/MathStructure-isolatex.cc",
        "../libqalculate/libqalculate/MathStructure-limit.cc",
        "../libqalculate/libqalculate/MathStructure-matrixvector.cc",
        "../libqalculate/libqalculate/MathStructure-polynomial.cc",
        "../libqalculate/libqalculate/MathStructure-print.cc",
        "../libqalculate/libqalculate/MathStructure.cc",
        "../libqalculate/libqalculate/Number.cc",
        "../libqalculate/libqalculate/Prefix.cc",
        "../libqalculate/libqalculate/QalculateDateTime.cc",
        "../libqalculate/libqalculate/Unit.cc",
        "../libqalculate/libqalculate/Variable.cc",
        "../libqalculate/libqalculate/util.cc",
    ];

    for file in &source_files {
        build.file(file);
    }

    // Compile the static library
    build.compile("qalculate");

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
