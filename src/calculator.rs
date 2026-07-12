//! Native, session-oriented calculator API.
//!
//! [`Calculator`] is the safe Rust entry point for the native parser,
//! evaluator, formatter, definition catalogs, options, and structured
//! messages. It does not construct or call the transitional C++ FFI
//! calculator.
//!
//! # Example
//!
//! ```
//! use libqalculate_rust::Calculator;
//!
//! let mut calculator = Calculator::new();
//! assert_eq!(calculator.calculate_and_print("1 + 1")?, "2");
//! # Ok::<(), libqalculate_rust::CalculatorError>(())
//! ```

use crate::ast::Expression;
use crate::context::CalculatorContext;
use crate::datasets::DatasetCatalog;
use crate::definitions::DefinitionIoError;
use crate::definitions_catalog::FunctionVariableCatalog;
use crate::messages::{CalculatorMessage, MessageCategory, MessageStage, MessageType};
use crate::options::{EvaluationOptions, ParseOptions, PrintOptions};
use crate::parser::names::StaticRegistry;
use crate::parser::operators::{parse_expression, ParseError};
use crate::units::PrefixUnitCatalog;
use std::fmt;
use std::path::Path;

/// Error returned by native calculator evaluation or formatting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalculatorError {
    message: String,
}

impl CalculatorError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Returns the underlying native error message.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for CalculatorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for CalculatorError {}

/// A native calculator session with explicit, Rust-owned state.
///
/// The session owns options, variables, definition-name registration, and
/// structured messages. Loaded XML catalogs are retained so callers can query
/// the same definitions used for parsing and focused unit conversion.
#[derive(Debug, Default)]
pub struct Calculator {
    context: CalculatorContext,
    definitions: Option<FunctionVariableCatalog>,
    units: Option<PrefixUnitCatalog>,
    datasets: Option<DatasetCatalog>,
}

impl Calculator {
    /// Creates a native calculator session with upstream-compatible option defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Parses expression text into the public Rust expression tree.
    pub fn parse(&self, input: &str) -> Result<Expression, ParseError> {
        parse_expression(input)
    }

    /// Evaluates and simplifies an already-parsed expression in this session.
    ///
    /// Evaluation failures are also appended to the structured message queue.
    pub fn evaluate(&mut self, expression: &Expression) -> Result<Expression, CalculatorError> {
        self.context
            .evaluate_expression(expression)
            .map_err(CalculatorError::new)
    }

    /// Parses, evaluates, and simplifies expression text in this session.
    ///
    /// Parser and evaluator failures are also appended to the structured
    /// message queue.
    pub fn calculate(&mut self, input: &str) -> Result<Expression, CalculatorError> {
        self.context
            .parse_and_evaluate_expression(input)
            .map_err(CalculatorError::new)
    }

    /// Formats a structured expression with this session's numeric options.
    pub fn print(&self, expression: &Expression) -> Result<String, CalculatorError> {
        let precision = self.context.precision_digits;
        let fraction_format = self.context.print_options.number_fraction_format;
        let approximation = self.context.evaluation_options.approximation;
        crate::text::format_result_with_numbers(expression, &|number| {
            number.to_string_with_options(precision, fraction_format, approximation)
        })
        .ok_or_else(|| CalculatorError::new("expression cannot be formatted natively"))
    }

    /// Parses, evaluates, and formats an expression using only native Rust code.
    ///
    /// When definition catalogs have been loaded, the focused native unit
    /// conversion engine is consulted before general expression evaluation.
    pub fn calculate_and_print(&mut self, input: &str) -> Result<String, CalculatorError> {
        if let Some(units) = &self.units {
            match crate::unit_conversion::native_output_with_catalog(
                input,
                units,
                &mut self.context,
            ) {
                Ok(Some(output)) => return Ok(output.output),
                Ok(None) => {}
                Err(message) => return Err(self.record_calculation_error(message)),
            }
        }

        let result = self.calculate(input)?;
        self.print(&result)
    }

    /// Converts an expression to a target unit or base and returns formatted output.
    ///
    /// Unit conversion requires [`load_definitions_from_dir`](Self::load_definitions_from_dir)
    /// or [`load_global_definitions`](Self::load_global_definitions) first. Base
    /// conversion uses the native expression evaluator directly for dimensionless
    /// numeric expressions. Mixed unit-to-number-base formatting remains pending.
    pub fn convert_and_print(
        &mut self,
        input: &str,
        target: &str,
    ) -> Result<String, CalculatorError> {
        self.calculate_and_print(&format!("{input} to {target}"))
    }

    /// Loads function, variable, prefix, unit, currency, and dataset XML catalogs.
    ///
    /// All source files are parsed before session state changes. If any load
    /// fails, the previously loaded catalogs and name registry remain intact.
    pub fn load_definitions_from_dir(
        &mut self,
        data_dir: impl AsRef<Path>,
    ) -> Result<(), DefinitionIoError> {
        let data_dir = data_dir.as_ref();
        let definitions =
            crate::definitions_catalog::load_function_variable_catalog_from_dir(data_dir)?;
        let units = crate::units::load_prefix_unit_catalog_from_dir(data_dir)?;
        let datasets = crate::datasets::load_dataset_catalog_from_dir(data_dir)?;

        let mut registry = StaticRegistry::with_builtins();
        definitions.register_into(&mut registry);
        units.register_into(&mut registry);

        self.context.definitions = registry;
        self.definitions = Some(definitions);
        self.units = Some(units);
        self.datasets = Some(datasets);
        Ok(())
    }

    /// Loads definition catalogs from the configured global definitions directory.
    ///
    /// The directory is selected by `QALCULATE_DEFINITIONS_DIR`, falling back to
    /// the adjacent upstream checkout used by this port workspace.
    pub fn load_global_definitions(&mut self) -> Result<(), DefinitionIoError> {
        self.load_definitions_from_dir(crate::rates::definitions_dir())
    }

    /// Returns the loaded function and variable catalog, if available.
    pub fn definitions(&self) -> Option<&FunctionVariableCatalog> {
        self.definitions.as_ref()
    }

    /// Returns the loaded prefix and unit catalog, if available.
    pub fn units(&self) -> Option<&PrefixUnitCatalog> {
        self.units.as_ref()
    }

    /// Returns the loaded dataset catalog, if available.
    pub fn datasets(&self) -> Option<&DatasetCatalog> {
        self.datasets.as_ref()
    }

    /// Returns this session's parse options.
    pub fn parse_options(&self) -> &ParseOptions {
        &self.context.parse_options
    }

    /// Returns mutable access to this session's parse options.
    ///
    /// Only option interactions classified as native in the public API parity
    /// matrix are currently guaranteed to affect evaluation.
    pub fn parse_options_mut(&mut self) -> &mut ParseOptions {
        &mut self.context.parse_options
    }

    /// Returns this session's evaluation options.
    pub fn evaluation_options(&self) -> &EvaluationOptions {
        &self.context.evaluation_options
    }

    /// Returns mutable access to this session's evaluation options.
    ///
    /// Only option interactions classified as native in the public API parity
    /// matrix are currently guaranteed to affect evaluation.
    pub fn evaluation_options_mut(&mut self) -> &mut EvaluationOptions {
        &mut self.context.evaluation_options
    }

    /// Returns this session's print options.
    pub fn print_options(&self) -> &PrintOptions {
        &self.context.print_options
    }

    /// Returns mutable access to this session's print options.
    ///
    /// Only option interactions classified as native in the public API parity
    /// matrix are currently guaranteed to affect formatting.
    pub fn print_options_mut(&mut self) -> &mut PrintOptions {
        &mut self.context.print_options
    }

    /// Returns the current decimal evaluation precision.
    pub fn precision(&self) -> usize {
        self.context.precision_digits
    }

    /// Sets the decimal evaluation precision, clamped to at least one digit.
    pub fn set_precision(&mut self, precision_digits: usize) {
        self.context.precision_digits = precision_digits.max(1);
    }

    /// Applies a typed session command such as `/set precision 20`.
    ///
    /// The `set` or `assume` command prefix is required; bare setting names are
    /// not normalized by this strict library API.
    pub fn apply_command(&mut self, command: &str) -> Result<(), ParseError> {
        self.context.apply_command(command)
    }

    /// Returns all queued structured messages without removing them.
    pub fn messages(&self) -> &[CalculatorMessage] {
        self.context.messages.get_messages()
    }

    /// Removes and returns the oldest queued structured message.
    pub fn next_message(&mut self) -> Option<CalculatorMessage> {
        self.context.messages.next_message()
    }

    /// Removes and returns all queued structured messages in source order.
    pub fn take_messages(&mut self) -> Vec<CalculatorMessage> {
        let mut messages = Vec::with_capacity(self.context.messages.len());
        while let Some(message) = self.context.messages.next_message() {
            messages.push(message);
        }
        messages
    }

    /// Clears all queued structured messages.
    pub fn clear_messages(&mut self) {
        self.context.clear_messages();
    }

    fn record_calculation_error(&mut self, message: String) -> CalculatorError {
        self.context.messages.push(CalculatorMessage::new(
            message.clone(),
            MessageType::Error,
            MessageCategory::None,
            MessageStage::Calculation,
        ));
        CalculatorError::new(message)
    }
}
