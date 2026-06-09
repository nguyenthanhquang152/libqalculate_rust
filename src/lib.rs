#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![doc = "Quality scaffold for the Rust port of libqalculate."]

/// Upstream libqalculate version recorded when this port workspace was initialized.
pub const UPSTREAM_LIBQALCULATE_VERSION: &str = "5.11.0";

/// Utilities for reading upstream libqalculate batch fixtures.
pub mod batch;

#[cfg(test)]
mod tests {
    use super::UPSTREAM_LIBQALCULATE_VERSION;

    #[test]
    fn upstream_version_is_recorded() {
        assert_eq!(UPSTREAM_LIBQALCULATE_VERSION, "5.11.0");
    }
}
