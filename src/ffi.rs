#![allow(unsafe_code)]
//! Safe Rust wrapper and FFI bindings for C++ libqalculate's Calculator.

use cxx::UniquePtr;
use std::marker::PhantomData;

#[cxx::bridge]
#[allow(missing_docs)]
pub(crate) mod sys {
    // SAFETY: The FFI declarations below reference C++ symbols implemented in `ffi_bridge.cc`
    // and the upstream `libqalculate` library. CXX guarantees that these signatures are
    // checked and generated correctly at build time, ensuring safety under normal C++ linking assumptions.
    unsafe extern "C++" {
        include!("libqalculate_rust/src/ffi_bridge.h");

        /// Opaque C++ Calculator type.
        type Calculator;

        /// Create a std::unique_ptr to a Calculator.
        fn new_calculator() -> UniquePtr<Calculator>;

        /// Load exchange rates.
        fn load_exchange_rates(calc: Pin<&mut Calculator>) -> bool;

        /// Load global definitions.
        fn load_global_definitions(calc: Pin<&mut Calculator>) -> bool;

        /// Load local definitions.
        fn load_local_definitions(calc: Pin<&mut Calculator>) -> bool;

        /// Calculate and print an expression.
        fn calculate_and_print(
            calc: Pin<&mut Calculator>,
            expr: &str,
            timeout_ms: i32,
        ) -> Result<String>;
    }
}

/// Safe wrapper around the C++ `Calculator` class.
pub struct Calculator {
    inner: UniquePtr<sys::Calculator>,
    _phantom: PhantomData<*mut ()>,
}

impl Calculator {
    /// Create a new `Calculator` instance.
    pub fn new() -> Self {
        // SAFETY: Calling C++ factory function to instantiate a new Calculator on the C++ heap.
        // The returned UniquePtr safely manages the lifetime of the object.
        let inner = sys::new_calculator();
        Self {
            inner,
            _phantom: PhantomData,
        }
    }

    /// Load the exchange rates for currencies.
    /// Returns `true` if loaded successfully.
    pub fn load_exchange_rates(&mut self) -> bool {
        if self.inner.is_null() {
            return false;
        }
        let pin = self.inner.pin_mut();
        // SAFETY: Passing a pinned mutable reference of the Calculator to the FFI function.
        // The pinned reference ensures the C++ object is not moved and is valid.
        sys::load_exchange_rates(pin)
    }

    /// Load the standard global definitions (system wide).
    /// Returns `true` if loaded successfully.
    pub fn load_global_definitions(&mut self) -> bool {
        if self.inner.is_null() {
            return false;
        }
        let pin = self.inner.pin_mut();
        // SAFETY: Passing a pinned mutable reference of the Calculator to the FFI function.
        // The pinned reference ensures the C++ object is not moved and is valid.
        sys::load_global_definitions(pin)
    }

    /// Load user-specific local definitions.
    /// Returns `true` if loaded successfully.
    pub fn load_local_definitions(&mut self) -> bool {
        if self.inner.is_null() {
            return false;
        }
        let pin = self.inner.pin_mut();
        // SAFETY: Passing a pinned mutable reference of the Calculator to the FFI function.
        // The pinned reference ensures the C++ object is not moved and is valid.
        sys::load_local_definitions(pin)
    }

    /// Evaluate a mathematical expression string and return the formatted result.
    ///
    /// # Errors
    /// Returns a `cxx::Exception` if a C++ exception occurs during parsing/evaluation.
    ///
    /// # Panics
    /// Panics if the inner Calculator pointer is null, which indicates a bug
    /// (e.g., use-after-move). This should never happen in normal usage since
    /// `new()` always constructs a valid Calculator.
    pub fn calculate_and_print(
        &mut self,
        expr: &str,
        timeout_ms: i32,
    ) -> Result<String, cxx::Exception> {
        assert!(
            !self.inner.is_null(),
            "BUG: Calculator inner pointer is null — possible use-after-move"
        );
        let pin = self.inner.pin_mut();
        // SAFETY: Calling calculate_and_print FFI function with a pinned mutable reference to
        // the Calculator and a valid string slice. The cxx crate safely handles the FFI boundary
        // and converts any C++ exceptions into a Rust Result::Err containing the cxx::Exception.
        sys::calculate_and_print(pin, expr, timeout_ms)
    }
}

impl Default for Calculator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_simple_addition() {
        let mut calc = Calculator::new();
        calc.load_global_definitions();
        let result = calc.calculate_and_print("1 + 1", 1000).unwrap();
        assert_eq!(result, "2");
    }
}
