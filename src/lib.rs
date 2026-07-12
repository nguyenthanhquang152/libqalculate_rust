#![deny(unsafe_code)]
#![deny(missing_docs)]
#![doc = "Native Rust port surface for libqalculate 5.11.0 compatibility work."]

/// Upstream libqalculate version recorded when this port workspace was initialized.
pub const UPSTREAM_LIBQALCULATE_VERSION: &str = "5.11.0";

/// Native calculator session façade.
pub mod calculator;

pub use calculator::{Calculator, CalculatorError};

/// Utilities for reading upstream libqalculate batch fixtures.
pub mod batch;

/// Expression tree model for the Rust `MathStructure` port.
pub mod ast;

/// Parser and lexer modules for the Rust port.
pub mod parser;

/// Native AST evaluator module.
pub mod eval;

/// Native AST simplifier.
pub mod simplify;

/// Native symbolic / polynomial algorithms.
pub mod symbolic;

/// Transitional FFI bindings to the C++ calculator.
///
/// This is the **only** module that uses `#![allow(unsafe_code)]` to override
/// the crate-level `#![deny(unsafe_code)]`. All unsafe FFI operations are
/// contained here behind the separately named [`ffi::Calculator`]. This module
/// is fallback/oracle infrastructure and is not the native public parity API;
/// use the crate-root [`Calculator`] for native sessions.
pub mod ffi;

mod markup;
mod matrix;
mod numberbase;
mod session;
mod statistics;
mod text;
mod unit_conversion;

/// Calculator options module.
pub mod options;

/// Warning and error messages module.
pub mod messages;

/// Session context module.
pub mod context;

/// CSV-backed data loading helpers.
pub mod data;

/// XML definition loader core with provenance and recoverable diagnostics.
pub mod definitions;

/// Typed prefix and unit definitions catalog.
pub mod units;

/// Typed function and variable definitions catalog.
pub mod definitions_catalog;

/// Typed dataset definitions and built-in object data catalog.
pub mod datasets;

/// Native exchange-rate parsing and currency conversion.
pub mod rates;

/// Native date/time value model.
pub mod datetime;

/// Core `Number` representation backed by `rug` GMP/MPFR values.
///
/// Upstream oracle: `../libqalculate/libqalculate/Number.h` and `Number.cc`.
/// This module preserves a small Rust-facing scaffold while the full upstream
/// `Number` API surface is ported incrementally.
pub mod number;

/// Built-in function catalog.
///
/// Upstream oracle: `../libqalculate/libqalculate/BuiltinFunctions*.cc` and
/// `../libqalculate/data/functions.xml.in`.
pub mod functions;

#[cfg(test)]
mod tests {
    use super::UPSTREAM_LIBQALCULATE_VERSION;

    #[test]
    fn upstream_version_is_recorded() {
        assert_eq!(UPSTREAM_LIBQALCULATE_VERSION, "5.11.0");
    }
}
