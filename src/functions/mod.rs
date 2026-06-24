//! Built-in function catalog for the Rust port.
//!
//! This module provides the function registry, dispatch traits, and
//! family implementations that correspond to upstream's
//! `BuiltinFunctions*.cc` files.
//!
//! # Upstream oracle
//! - `../libqalculate/libqalculate/Function.h`
//! - `../libqalculate/libqalculate/Function.cc`
//! - `../libqalculate/libqalculate/BuiltinFunctions.h`
//! - `../libqalculate/data/functions.xml.in`

pub mod explog;

use crate::ast::Expression;
use crate::context::CalculatorContext;

/// Result type for built-in function evaluation.
pub type FunctionResult = Result<Expression, FunctionError>;

/// Error returned when a built-in function evaluation fails.
#[derive(Debug, Clone)]
pub struct FunctionError {
    /// The function name that failed.
    pub function_name: String,
    /// Human-readable error description.
    pub message: String,
}

impl std::fmt::Display for FunctionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.function_name, self.message)
    }
}

/// Metadata describing a built-in function's signature.
#[derive(Debug, Clone)]
pub struct BuiltinFunctionInfo {
    /// Primary function name (e.g. "sqrt").
    pub name: &'static str,
    /// Alternative names (e.g. "log2", "log10" for LogFunction).
    pub aliases: &'static [&'static str],
    /// Minimum number of arguments.
    pub min_args: usize,
    /// Maximum number of arguments (`None` = variadic).
    pub max_args: Option<usize>,
    /// Brief description matching upstream `functions.xml.in`.
    pub description: &'static str,
}

/// Trait for built-in function evaluation.
///
/// Each function family (explog, trig, etc.) implements this trait
/// for its member functions, providing both metadata and evaluation.
pub trait BuiltinFunction: Send + Sync {
    /// Returns metadata about this function.
    fn info(&self) -> &BuiltinFunctionInfo;

    /// Evaluates the function with the given arguments.
    ///
    /// Arguments are pre-evaluated expressions. The function should
    /// validate argument count and types, and return appropriate
    /// error messages through the context.
    fn evaluate(
        &self,
        args: &[Expression],
        context: &mut CalculatorContext,
    ) -> FunctionResult;
}

/// Dispatches a function call to a built-in implementation if one exists.
///
/// Returns `Some(result)` if the function was handled natively,
/// or `None` if no built-in implementation exists for the given name.
pub fn dispatch_builtin(
    name: &str,
    args: &[Expression],
    context: &mut CalculatorContext,
) -> Option<FunctionResult> {
    // Try explog family first
    if let Some(func) = explog::lookup(name) {
        return Some(func.evaluate(args, context));
    }

    None
}

/// Returns the [`BuiltinFunctionInfo`] for a built-in function name, if known.
pub fn builtin_info(name: &str) -> Option<&'static BuiltinFunctionInfo> {
    explog::lookup(name).map(|f| f.info())
}

/// Returns all registered built-in function infos for the explog family.
pub fn explog_catalog() -> Vec<&'static BuiltinFunctionInfo> {
    explog::catalog()
}
