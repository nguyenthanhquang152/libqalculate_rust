//! Explicit context representing a calculator session state.
//!
//! Owns precision, number bases, angle units, option structures,
//! assumptions, definitions, and warning/error message queues.

use crate::messages::MessageQueue;
use crate::options::{AngleUnit, EvaluationOptions, ParseOptions, PrintOptions};
use crate::parser::names::StaticRegistry;

/// Assumption type for a variable or expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum AssumptionType {
    /// Commutative / matrix distinction.
    None = 0,
    /// Non-matrix value.
    NonMatrix = 1,
    /// Number value.
    Number = 2,
    /// Complex number value.
    Complex = 3,
    /// Real number value (default).
    Real = 4,
    /// Rational number value.
    Rational = 5,
    /// Integer value.
    Integer = 6,
    /// Boolean value.
    Boolean = 7,
}

/// Assumption sign/signedness for a variable or expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum AssumptionSign {
    /// Sign is unknown (default).
    Unknown = 0,
    /// Value is strictly positive (> 0).
    Positive = 1,
    /// Value is non-negative (>= 0).
    NonNegative = 2,
    /// Value is strictly negative (< 0).
    Negative = 3,
    /// Value is non-positive (<= 0).
    NonPositive = 4,
    /// Value is non-zero (!= 0).
    NonZero = 5,
}

/// Active assumptions for variables or unknowns in a session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assumptions {
    /// Default type assumption for unknowns.
    pub default_type: AssumptionType,
    /// Default sign assumption for unknowns.
    pub default_sign: AssumptionSign,
}

impl Default for Assumptions {
    fn default() -> Self {
        Self {
            default_type: AssumptionType::Real,
            default_sign: AssumptionSign::Unknown,
        }
    }
}

/// Explicit context holding all configuration, session settings, definitions, and message queues.
#[derive(Debug, Clone)]
pub struct CalculatorContext {
    /// Session evaluation precision in decimal digits.
    pub precision_digits: usize,
    /// Active number base for input parsing.
    pub input_base: u32,
    /// Active number base for output display.
    pub output_base: u32,
    /// Active default angle unit.
    pub angle_unit: AngleUnit,
    /// Print formatting options.
    pub print_options: PrintOptions,
    /// Parser configuration.
    pub parse_options: ParseOptions,
    /// Evaluator options.
    pub evaluation_options: EvaluationOptions,
    /// Active variables/unknowns assumptions.
    pub assumptions: Assumptions,
    /// Warning and error message queue for the current session.
    pub messages: MessageQueue,
    /// Name registry for definitions.
    pub definitions: StaticRegistry,
    /// Active user-defined variables.
    pub variables: std::collections::HashMap<String, crate::ast::Expression>,
}

impl Default for CalculatorContext {
    fn default() -> Self {
        Self {
            precision_digits: 8,
            input_base: 10,
            output_base: 10,
            angle_unit: AngleUnit::None,
            print_options: PrintOptions::default(),
            parse_options: ParseOptions::default(),
            evaluation_options: EvaluationOptions::default(),
            assumptions: Assumptions::default(),
            messages: MessageQueue::new(),
            definitions: StaticRegistry::with_builtins(),
            variables: std::collections::HashMap::new(),
        }
    }
}

impl CalculatorContext {
    /// Create a new `CalculatorContext` with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the target evaluation precision in bits.
    pub fn min_precision_bits(&self) -> u32 {
        let bits = self
            .precision_digits
            .max(1)
            .saturating_mul(4)
            .saturating_add(16)
            .max(128);
        u32::try_from(bits).unwrap_or(u32::MAX)
    }

    /// Clear all warning and error messages in the context.
    pub fn clear_messages(&mut self) {
        self.messages.clear();
    }

    /// Apply a session command (like `/set precision 128` or `/assume positive`) to the context.
    pub fn apply_command(
        &mut self,
        cmd_str: &str,
    ) -> Result<(), crate::parser::operators::ParseError> {
        use crate::options::ApproximationMode;
        use crate::parser::commands::{
            parse_command, ApproximationMode as CmdApproximationMode, SessionCommand, SetSetting,
        };

        let cmd = parse_command(cmd_str)?;
        match cmd {
            SessionCommand::Set(c) => match c.setting {
                SetSetting::InputBase(b) => {
                    self.input_base = b;
                    self.print_options.base = b as i32;
                    self.parse_options.base = b as i32;
                }
                SetSetting::OutputBase(b) => {
                    self.output_base = b;
                }
                SetSetting::Unicode(u) => {
                    self.print_options.use_unicode_signs = if u {
                        crate::options::UnicodeSigns::On
                    } else {
                        crate::options::UnicodeSigns::Off
                    };
                }
                SetSetting::Precision(p) => {
                    self.precision_digits = p;
                }
                SetSetting::IntervalDisplay(id) => {
                    self.print_options.interval_display = match id {
                        0 => crate::options::IntervalDisplay::SignificantDigits,
                        1 => crate::options::IntervalDisplay::Interval,
                        2 => crate::options::IntervalDisplay::PlusMinus,
                        3 => crate::options::IntervalDisplay::Midpoint,
                        4 => crate::options::IntervalDisplay::Lower,
                        5 => crate::options::IntervalDisplay::Upper,
                        6 => crate::options::IntervalDisplay::Concise,
                        7 => crate::options::IntervalDisplay::Relative,
                        _ => self.print_options.interval_display,
                    };
                }
                SetSetting::IntervalCalculation(ic) => {
                    self.evaluation_options.interval_calculation = match ic {
                        0 => crate::options::IntervalCalculation::None,
                        1 => crate::options::IntervalCalculation::VarianceFormula,
                        2 => crate::options::IntervalCalculation::IntervalArithmetic,
                        3 => crate::options::IntervalCalculation::SimpleIntervalArithmetic,
                        _ => self.evaluation_options.interval_calculation,
                    };
                }
                SetSetting::Approximation(a) => {
                    self.evaluation_options.approximation = match a {
                        CmdApproximationMode::Exact => ApproximationMode::Exact,
                        CmdApproximationMode::TryExact => ApproximationMode::TryExact,
                        CmdApproximationMode::Approximate => ApproximationMode::Approximate,
                    };
                }
                SetSetting::FractionFormat(ff) => {
                    self.print_options.number_fraction_format = match ff {
                        0 => crate::options::NumberFractionFormat::Decimal,
                        1 => crate::options::NumberFractionFormat::DecimalExact,
                        2 => crate::options::NumberFractionFormat::Fractional,
                        3 => crate::options::NumberFractionFormat::Combined,
                        4 => crate::options::NumberFractionFormat::FractionalFixedDenominator,
                        5 => crate::options::NumberFractionFormat::CombinedFixedDenominator,
                        6 => crate::options::NumberFractionFormat::Percent,
                        7 => crate::options::NumberFractionFormat::Permille,
                        8 => crate::options::NumberFractionFormat::Permyriad,
                        _ => self.print_options.number_fraction_format,
                    };
                }
                SetSetting::ConciseUncertainty(cu) => {
                    if cu {
                        self.print_options.interval_display =
                            crate::options::IntervalDisplay::Concise;
                    }
                }
                SetSetting::Complex(cplx) => {
                    self.evaluation_options.allow_complex = cplx != 0;
                }
                SetSetting::DecimalComma(dc) => {
                    if dc {
                        self.print_options.comma_sign = ",".to_string();
                        self.print_options.decimalpoint_sign = ".".to_string();
                    } else {
                        self.print_options.comma_sign = String::new();
                        self.print_options.decimalpoint_sign = String::new();
                    }
                }
                SetSetting::CurrencyConversion(cc) => {
                    self.evaluation_options.local_currency_conversion = cc != 0;
                }
                SetSetting::Percent(pct) => {
                    if pct != 0 {
                        self.print_options.number_fraction_format =
                            crate::options::NumberFractionFormat::Percent;
                    }
                }
                SetSetting::Abbreviations(abbr) => {
                    self.print_options.abbreviate_names = abbr;
                }
                SetSetting::EngineeringDisplay(ed) => {
                    self.print_options.exp_display = match ed {
                        0 => crate::options::ExpDisplay::Default,
                        1 => crate::options::ExpDisplay::UppercaseE,
                        2 => crate::options::ExpDisplay::LowercaseE,
                        3 => crate::options::ExpDisplay::PowerOf10,
                        _ => self.print_options.exp_display,
                    };
                }
            },
            SessionCommand::Assume(a) => match a.kind {
                crate::parser::commands::AssumeKind::Positive => {
                    self.assumptions.default_sign = crate::context::AssumptionSign::Positive;
                }
                crate::parser::commands::AssumeKind::Unknown => {
                    self.assumptions.default_sign = crate::context::AssumptionSign::Unknown;
                }
            },
        }
        Ok(())
    }

    /// Parse and evaluate an expression string using the context, recording warnings/errors.
    pub fn parse_and_evaluate_with_context(
        &mut self,
        input: &str,
    ) -> Result<crate::number::Number, String> {
        // 1. Parse stage
        let expr = match crate::parser::operators::parse_expression(input) {
            Ok(expr) => expr,
            Err(err) => {
                let msg = crate::messages::CalculatorMessage::new(
                    err.to_string(),
                    crate::messages::MessageType::Error,
                    crate::messages::MessageCategory::Parsing,
                    crate::messages::MessageStage::Parsing,
                );
                self.messages.push(msg);
                return Err(err.to_string());
            }
        };

        // 2. Evaluation stage
        let res = crate::eval::evaluate_ast(&expr, self);

        match res {
            Ok(expr_res) => {
                let simplified = match &expr_res {
                    crate::ast::Expression::Number(_) => expr_res,
                    _ => crate::simplify::simplify_ast(&expr_res, self),
                };
                match simplified {
                    crate::ast::Expression::Number(num) => {
                        if num.is_nan() {
                            let has_calc_warning =
                                self.messages.get_messages().iter().any(|m| {
                                    m.stage() == crate::messages::MessageStage::Calculation
                                });
                            if !has_calc_warning {
                                let msg = crate::messages::CalculatorMessage::new(
                                    "Calculation resulted in NaN".to_string(),
                                    crate::messages::MessageType::Warning,
                                    crate::messages::MessageCategory::None,
                                    crate::messages::MessageStage::Calculation,
                                );
                                self.messages.push(msg);
                            }
                        }
                        Ok(num)
                    }
                    other => Err(format!("Symbolic result: {:?}", other)),
                }
            }
            Err(err_str) => {
                let msg = crate::messages::CalculatorMessage::new(
                    err_str.clone(),
                    crate::messages::MessageType::Error,
                    crate::messages::MessageCategory::None,
                    crate::messages::MessageStage::Calculation,
                );
                self.messages.push(msg);
                Err(err_str)
            }
        }
    }

    /// Parse and evaluate an expression, returning the result as a formatted string.
    ///
    /// This handles both numeric and symbolic results (e.g., from base conversions).
    pub fn parse_and_evaluate_to_string(&mut self, input: &str) -> Result<String, String> {
        // 1. Parse stage
        let expr = match crate::parser::operators::parse_expression(input) {
            Ok(expr) => expr,
            Err(err) => {
                let msg = crate::messages::CalculatorMessage::new(
                    err.to_string(),
                    crate::messages::MessageType::Error,
                    crate::messages::MessageCategory::Parsing,
                    crate::messages::MessageStage::Parsing,
                );
                self.messages.push(msg);
                return Err(err.to_string());
            }
        };

        // 2. Evaluation stage
        let res = crate::eval::evaluate_ast(&expr, self);

        match res {
            Ok(expr_res) => {
                let simplified = match &expr_res {
                    crate::ast::Expression::Number(_) => expr_res,
                    crate::ast::Expression::Text(_) => expr_res,
                    crate::ast::Expression::Symbolic(_) => expr_res,
                    _ => crate::simplify::simplify_ast(&expr_res, self),
                };
                if let Some(output) =
                    crate::text::format_result_with_numbers(&simplified, &|num| num.to_string())
                {
                    Ok(output)
                } else {
                    Err(format!("Unevaluated expression: {:?}", simplified))
                }
            }
            Err(err_str) => {
                let msg = crate::messages::CalculatorMessage::new(
                    err_str.clone(),
                    crate::messages::MessageType::Error,
                    crate::messages::MessageCategory::None,
                    crate::messages::MessageStage::Calculation,
                );
                self.messages.push(msg);
                Err(err_str)
            }
        }
    }
}
