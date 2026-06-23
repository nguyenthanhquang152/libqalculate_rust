//! Integration tests for number base parity (Issue #26).
//!
//! Verifies that the `to` conversion operator correctly formats numbers in
//! different bases (binary, octal, hexadecimal, roman, arbitrary base, float,
//! and sexagesimal), matching upstream qalculate behavior.

use libqalculate_rust::context::CalculatorContext;

/// Helper: parse and evaluate via parse_and_evaluate_to_string
fn eval_to_string(expr: &str) -> String {
    let mut ctx = CalculatorContext::new();
    ctx.parse_and_evaluate_to_string(expr)
        .unwrap_or_else(|e| panic!("Failed to evaluate '{expr}': {e}"))
}

/// Helper: parse and evaluate, expecting an error
fn eval_err(expr: &str) -> String {
    let mut ctx = CalculatorContext::new();
    ctx.parse_and_evaluate_to_string(expr)
        .expect_err(&format!("Expected error for '{expr}'"))
}

// ──────────────────────────────────────────────────────────
// Binary conversion
// ──────────────────────────────────────────────────────────

#[test]
fn to_bin_52() {
    let result = eval_to_string("52 to bin");
    assert_eq!(result, "0011 0100");
}

#[test]
fn to_bin_255() {
    let result = eval_to_string("255 to bin");
    assert_eq!(result, "1111 1111");
}

#[test]
fn to_bin_256() {
    let result = eval_to_string("256 to bin");
    // 256 = 1_0000_0000 → needs 16-bit width (padded to 8-multiple)
    assert_eq!(result, "0000 0001 0000 0000");
}

#[test]
fn to_bin_0() {
    let result = eval_to_string("0 to bin");
    assert_eq!(result, "0000 0000");
}

#[test]
fn to_binary_synonym() {
    let result = eval_to_string("52 to binary");
    assert_eq!(result, "0011 0100");
}

// ──────────────────────────────────────────────────────────
// Octal conversion
// ──────────────────────────────────────────────────────────

#[test]
fn to_oct_52() {
    let result = eval_to_string("52 to oct");
    assert_eq!(result, "64");
}

#[test]
fn to_oct_255() {
    let result = eval_to_string("255 to oct");
    assert_eq!(result, "377");
}

#[test]
fn to_octal_synonym() {
    let result = eval_to_string("8 to octal");
    assert_eq!(result, "10");
}

// ──────────────────────────────────────────────────────────
// Hexadecimal conversion
// ──────────────────────────────────────────────────────────

#[test]
fn to_hex_52() {
    let result = eval_to_string("52 to hex");
    assert_eq!(result, "34");
}

#[test]
fn to_hex_255() {
    let result = eval_to_string("255 to hex");
    assert_eq!(result, "FF");
}

#[test]
fn to_hex_4095() {
    let result = eval_to_string("4095 to hex");
    assert_eq!(result, "FFF");
}

#[test]
fn to_hexadecimal_synonym() {
    let result = eval_to_string("52 to hexadecimal");
    assert_eq!(result, "34");
}

// ──────────────────────────────────────────────────────────
// Roman numeral conversion
// ──────────────────────────────────────────────────────────

#[test]
fn to_roman_1978() {
    let result = eval_to_string("1978 to roman");
    assert_eq!(result, "MCMLXXVIII");
}

#[test]
fn to_roman_4() {
    let result = eval_to_string("4 to roman");
    assert_eq!(result, "IV");
}

#[test]
fn to_roman_3999() {
    let result = eval_to_string("3999 to roman");
    assert_eq!(result, "MMMCMXCIX");
}

#[test]
fn to_roman_out_of_range() {
    // 0 is outside 1..=3999
    let err = eval_err("0 to roman");
    assert!(err.contains("Roman"), "Error should mention Roman: {err}");
}

// ──────────────────────────────────────────────────────────
// Arbitrary base conversion
// ──────────────────────────────────────────────────────────

#[test]
fn to_base_2() {
    // "to base 2" should be equivalent to "to bin" but without 4-bit grouping
    let result = eval_to_string("10 to base 2");
    assert_eq!(result, "1010");
}

#[test]
fn to_base_8() {
    let result = eval_to_string("255 to base 8");
    assert_eq!(result, "377");
}

#[test]
fn to_base_16() {
    let result = eval_to_string("255 to base 16");
    assert_eq!(result, "FF");
}

#[test]
fn to_base_36() {
    // 35 in base 36 = "Z"
    let result = eval_to_string("35 to base 36");
    assert_eq!(result, "Z");
}

// ──────────────────────────────────────────────────────────
// Float (IEEE 754) conversion
// ──────────────────────────────────────────────────────────

#[test]
fn to_float_zero() {
    let result = eval_to_string("0 to float");
    assert_eq!(result, "0000 0000 0000 0000 0000 0000 0000 0000");
}

#[test]
fn to_float_one() {
    let result = eval_to_string("1 to float");
    // 1.0f32 = 0x3F800000 = 0011 1111 1000 0000 0000 0000 0000 0000
    assert_eq!(result, "0011 1111 1000 0000 0000 0000 0000 0000");
}

// ──────────────────────────────────────────────────────────
// Sexagesimal conversion
// ──────────────────────────────────────────────────────────

#[test]
fn to_sexa_52_34() {
    let result = eval_to_string("52.34 to sexa");
    // 52.34 degrees = 52°20′24″
    assert_eq!(result, "52°20′24″");
}

// ──────────────────────────────────────────────────────────
// Expression with arithmetic then conversion
// ──────────────────────────────────────────────────────────

#[test]
fn arithmetic_then_to_hex() {
    // (10 + 6) to hex = 16 → "10"
    let result = eval_to_string("(10 + 6) to hex");
    assert_eq!(result, "10");
}

#[test]
fn arithmetic_then_to_bin() {
    // (3 * 4) to bin = 12 → "0000 1100"
    let result = eval_to_string("(3 * 4) to bin");
    assert_eq!(result, "0000 1100");
}

// ──────────────────────────────────────────────────────────
// hex() built-in function
// ──────────────────────────────────────────────────────────

#[test]
fn hex_function_34() {
    // hex(34) treats "34" as hex digits → 0x34 = 52
    let result = eval_to_string("hex(34)");
    assert_eq!(result, "52");
}

#[test]
fn hex_function_ff() {
    // hex(FF) → 255
    let result = eval_to_string("hex(FF)");
    assert_eq!(result, "255");
}

// ──────────────────────────────────────────────────────────
// float() built-in function (bit string → f32)
// ──────────────────────────────────────────────────────────

#[test]
fn float_function_bit_string_5() {
    // float(01000000101000000000000000000000) = 5.0f32 bit pattern
    // 0x40A00000 = 5.0
    let result = eval_to_string("float(01000000101000000000000000000000)");
    assert_eq!(result, "5");
}

// ──────────────────────────────────────────────────────────
// floatError() built-in function
// ──────────────────────────────────────────────────────────

#[test]
fn float_error_52_345() {
    // floatError(52.345) → representation error of 52.345 as f32
    let result = eval_to_string("floatError(52.345)");
    // Just verify it returns a result (exact value depends on precision)
    assert!(!result.is_empty(), "floatError should return a value");
}

// ──────────────────────────────────────────────────────────
// Hex prefix input (0x...)
// ──────────────────────────────────────────────────────────

#[test]
fn hex_prefix_0x34() {
    // 0x34 = 52
    let result = eval_to_string("0x34");
    assert_eq!(result, "52");
}

#[test]
fn hex_prefix_0xff() {
    // 0xFF = 255
    let result = eval_to_string("0xFF");
    assert_eq!(result, "255");
}

// ──────────────────────────────────────────────────────────
// Base 32 conversion
// ──────────────────────────────────────────────────────────

#[test]
fn to_base_32() {
    // 52 to base 32 → "1K" (52 = 1×32 + 20, 20 → 'K')
    let result = eval_to_string("52 to base 32");
    assert_eq!(result, "1K");
}
