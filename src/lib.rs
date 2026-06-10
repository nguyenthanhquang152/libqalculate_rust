#![deny(unsafe_code)]
#![deny(missing_docs)]
#![doc = "Quality scaffold for the Rust port of libqalculate."]

/// Upstream libqalculate version recorded when this port workspace was initialized.
pub const UPSTREAM_LIBQALCULATE_VERSION: &str = "5.11.0";

/// Utilities for reading upstream libqalculate batch fixtures.
pub mod batch;

/// FFI bindings to C++ Calculator.
///
/// This is the **only** module that uses `#![allow(unsafe_code)]` to override
/// the crate-level `#![deny(unsafe_code)]`. All unsafe FFI operations are
/// contained here behind the safe `Calculator` wrapper.
pub mod ffi;

/// Core `Number` representation backed by `rug` GMP/MPFR values.
///
/// Upstream oracle: `../libqalculate/libqalculate/Number.h` and `Number.cc`.
/// This module preserves a small Rust-facing scaffold while the full upstream
/// `Number` API surface is ported incrementally.
pub mod number;

#[cfg(test)]
mod tests {
    use super::UPSTREAM_LIBQALCULATE_VERSION;

    #[test]
    fn upstream_version_is_recorded() {
        assert_eq!(UPSTREAM_LIBQALCULATE_VERSION, "5.11.0");
    }
}
