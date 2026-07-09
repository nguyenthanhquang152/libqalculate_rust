//! Session command parser for qalc commands (`/set`, `/assume`).
//!
//! This module parses command strings (optionally prefixed with `/`)
//! into a typed [`SessionCommand`](crate::parser::commands::SessionCommand) AST model with validation and source span tracking.

use crate::parser::lexer::Span;
use crate::parser::operators::{ParseError, ParseErrorKind};

/// A parsed qalc session command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionCommand {
    /// A set command (e.g. `/set unicode 1` or `set input base 16`).
    Set(SetCommand),
    /// An assume command (e.g. `/assume positive` or `/assume unknown`).
    Assume(AssumeCommand),
}

/// A set command containing the setting kind and its parsed typed value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetCommand {
    /// The setting kind and value.
    pub setting: SetSetting,
    /// The source span of the command.
    pub span: Span,
}

/// Specific settings supported by set command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetSetting {
    /// Mode for mathematical approximation.
    Approximation(ApproximationMode),
    /// Fraction format mode (e.g. `/set fr 2`).
    FractionFormat(u32),
    /// Whether to use Unicode output/characters.
    Unicode(bool),
    /// Interval calculation mode (e.g. `/set ic 2`).
    IntervalCalculation(u8),
    /// Number base for input.
    InputBase(u32),
    /// Number base for output.
    OutputBase(u32),
    /// Evaluation precision in digits.
    Precision(usize),
    /// Interval display style.
    IntervalDisplay(u8),
    /// Whether to print uncertainty concisely.
    ConciseUncertainty(bool),
    /// Complex number mode (cplx).
    Complex(u8),
    /// Whether to use decimal comma instead of dot.
    DecimalComma(bool),
    /// Currency conversion mode.
    CurrencyConversion(u8),
    /// Percent mode.
    Percent(u8),
    /// Abbreviations mode.
    Abbreviations(bool),
    /// Engineering display mode (edisp).
    EngineeringDisplay(u8),
    /// Minimum exponent for scientific notation (exp).
    MinExponent(i32),
    /// Minimum decimal places (min decimals / mindeci).
    MinDecimals(i32),
    /// Maximum decimal places (max decimals / maxdeci).
    MaxDecimals(i32),
}

/// Modes for approximation setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApproximationMode {
    /// Return exact fraction/expression results when possible.
    Exact,
    /// Try exact first, then fallback to approximate.
    TryExact,
    /// Always approximate.
    Approximate,
}

/// An assume command specifying an assumption.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssumeCommand {
    /// The kind of assumption.
    pub kind: AssumeKind,
    /// The source span of the command.
    pub span: Span,
}

/// Supported variable assumptions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssumeKind {
    /// Assume variables are positive reals.
    Positive,
    /// Clear all assumptions (variables are unknown).
    Unknown,
}

/// Parse a raw command line into a typed [`SessionCommand`].
///
/// Supports commands prefixed with `/` (e.g. `/set unicode 1`) or
/// without `/` (e.g. `set input base 16`).
pub fn parse_command(input: &str) -> Result<SessionCommand, ParseError> {
    let span = Span::new(0, input.len());
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ParseError::new(ParseErrorKind::UnexpectedEnd, span));
    }

    // Strip leading slash if present
    let command_line = trimmed.strip_prefix('/').unwrap_or(trimmed).trim();

    // Find the command name (word before space)
    let (cmd_name, rest) = match command_line.find(char::is_whitespace) {
        Some(idx) => {
            let (cmd, args) = command_line.split_at(idx);
            (cmd.trim(), args.trim())
        }
        None => (command_line, ""),
    };

    if cmd_name.eq_ignore_ascii_case("set") {
        let setting = parse_set_setting(rest, span)?;
        Ok(SessionCommand::Set(SetCommand { setting, span }))
    } else if cmd_name.eq_ignore_ascii_case("assume")
        || cmd_name.eq_ignore_ascii_case("assumptions")
    {
        let kind = parse_assume_kind(rest, span)?;
        Ok(SessionCommand::Assume(AssumeCommand { kind, span }))
    } else {
        Err(ParseError::new(ParseErrorKind::UnknownCommand, span))
    }
}

fn parse_set_setting(args: &str, span: Span) -> Result<SetSetting, ParseError> {
    let lower_args = args.to_ascii_lowercase();

    if let Some(val) = strip_setting_prefix(&lower_args, &["approximation", "approx"]) {
        let mode = match val {
            "exact" => ApproximationMode::Exact,
            "try exact" | "try_exact" => ApproximationMode::TryExact,
            "approximate" | "approx" => ApproximationMode::Approximate,
            _ => return Err(ParseError::new(ParseErrorKind::InvalidSettingValue, span)),
        };
        Ok(SetSetting::Approximation(mode))
    } else if let Some(val) = strip_setting_prefix(&lower_args, &["fraction format", "fr"]) {
        let num = val
            .parse::<u32>()
            .map_err(|_| ParseError::new(ParseErrorKind::InvalidSettingValue, span))?;
        Ok(SetSetting::FractionFormat(num))
    } else if let Some(val) = strip_setting_prefix(&lower_args, &["unicode"]) {
        let boolean = parse_bool(val)
            .ok_or_else(|| ParseError::new(ParseErrorKind::InvalidSettingValue, span))?;
        Ok(SetSetting::Unicode(boolean))
    } else if let Some(val) = strip_setting_prefix(&lower_args, &["interval calculation", "ic"]) {
        let num = val
            .parse::<u8>()
            .map_err(|_| ParseError::new(ParseErrorKind::InvalidSettingValue, span))?;
        Ok(SetSetting::IntervalCalculation(num))
    } else if let Some(val) = strip_setting_prefix(&lower_args, &["input base", "inbase"]) {
        let num = val
            .parse::<u32>()
            .map_err(|_| ParseError::new(ParseErrorKind::InvalidSettingValue, span))?;
        Ok(SetSetting::InputBase(num))
    } else if let Some(val) = strip_setting_prefix(&lower_args, &["output base", "outbase"]) {
        let num = val
            .parse::<u32>()
            .map_err(|_| ParseError::new(ParseErrorKind::InvalidSettingValue, span))?;
        Ok(SetSetting::OutputBase(num))
    } else if let Some(val) = strip_setting_prefix(&lower_args, &["precision"]) {
        let digits = val
            .parse::<usize>()
            .map_err(|_| ParseError::new(ParseErrorKind::InvalidSettingValue, span))?;
        if digits == 0 {
            return Err(ParseError::new(ParseErrorKind::InvalidSettingValue, span));
        }
        Ok(SetSetting::Precision(digits))
    } else if let Some(val) = strip_setting_prefix(&lower_args, &["interval display", "id"]) {
        let num = val
            .parse::<u8>()
            .map_err(|_| ParseError::new(ParseErrorKind::InvalidSettingValue, span))?;
        Ok(SetSetting::IntervalDisplay(num))
    } else if let Some(val) = strip_setting_prefix(&lower_args, &["concise uncertainty", "cu"]) {
        let boolean = parse_bool(val)
            .ok_or_else(|| ParseError::new(ParseErrorKind::InvalidSettingValue, span))?;
        Ok(SetSetting::ConciseUncertainty(boolean))
    } else if let Some(val) = strip_setting_prefix(&lower_args, &["complex", "cplx"]) {
        let num = val
            .parse::<u8>()
            .map_err(|_| ParseError::new(ParseErrorKind::InvalidSettingValue, span))?;
        Ok(SetSetting::Complex(num))
    } else if let Some(val) = strip_setting_prefix(&lower_args, &["decimal comma"]) {
        let boolean = parse_bool(val)
            .ok_or_else(|| ParseError::new(ParseErrorKind::InvalidSettingValue, span))?;
        Ok(SetSetting::DecimalComma(boolean))
    } else if let Some(val) = strip_setting_prefix(&lower_args, &["curconv"]) {
        let num = val
            .parse::<u8>()
            .map_err(|_| ParseError::new(ParseErrorKind::InvalidSettingValue, span))?;
        Ok(SetSetting::CurrencyConversion(num))
    } else if let Some(val) = strip_setting_prefix(&lower_args, &["percent"]) {
        let num = val
            .parse::<u8>()
            .map_err(|_| ParseError::new(ParseErrorKind::InvalidSettingValue, span))?;
        Ok(SetSetting::Percent(num))
    } else if let Some(val) = strip_setting_prefix(&lower_args, &["abbreviations", "abbr"]) {
        let boolean = parse_bool(val)
            .ok_or_else(|| ParseError::new(ParseErrorKind::InvalidSettingValue, span))?;
        Ok(SetSetting::Abbreviations(boolean))
    } else if let Some(val) = strip_setting_prefix(&lower_args, &["edisp"]) {
        let num = val
            .parse::<u8>()
            .map_err(|_| ParseError::new(ParseErrorKind::InvalidSettingValue, span))?;
        Ok(SetSetting::EngineeringDisplay(num))
    } else if let Some(val) = strip_setting_prefix(&lower_args, &["exp"]) {
        let v = match val {
            "off" => 0,
            "auto" => -1,
            "pure" => 1,
            "scientific" | "sci" | "on" => 3,
            "engineering" | "eng" => -3,
            other => other
                .parse::<i32>()
                .map_err(|_| ParseError::new(ParseErrorKind::InvalidSettingValue, span))?,
        };
        Ok(SetSetting::MinExponent(v))
    } else if let Some(val) = strip_setting_prefix(&lower_args, &["min decimals", "mindeci"]) {
        let num = val
            .parse::<i32>()
            .map_err(|_| ParseError::new(ParseErrorKind::InvalidSettingValue, span))?;
        Ok(SetSetting::MinDecimals(num))
    } else if let Some(val) = strip_setting_prefix(&lower_args, &["max decimals", "maxdeci"]) {
        let num = val
            .parse::<i32>()
            .map_err(|_| ParseError::new(ParseErrorKind::InvalidSettingValue, span))?;
        Ok(SetSetting::MaxDecimals(num))
    } else {
        Err(ParseError::new(ParseErrorKind::UnknownSetting, span))
    }
}

fn strip_setting_prefix<'a>(args: &'a str, prefixes: &[&str]) -> Option<&'a str> {
    for prefix in prefixes {
        if let Some(rest) = args.strip_prefix(*prefix) {
            let trimmed = rest.trim();
            if rest.starts_with(char::is_whitespace) || trimmed.is_empty() {
                return Some(trimmed);
            }
        }
    }
    None
}

fn parse_bool(val: &str) -> Option<bool> {
    match val {
        "1" | "true" | "on" | "yes" => Some(true),
        "0" | "false" | "off" | "no" => Some(false),
        _ => None,
    }
}

fn parse_assume_kind(args: &str, span: Span) -> Result<AssumeKind, ParseError> {
    let trimmed = args.trim().to_ascii_lowercase();
    match trimmed.as_str() {
        "positive" => Ok(AssumeKind::Positive),
        "unknown" => Ok(AssumeKind::Unknown),
        _ => Err(ParseError::new(ParseErrorKind::InvalidSettingValue, span)),
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_command, SessionCommand, SetSetting};

    fn parsed_setting(input: &str) -> SetSetting {
        match parse_command(input).expect("command should parse") {
            SessionCommand::Set(command) => command.setting,
            SessionCommand::Assume(_) => panic!("expected set command"),
        }
    }

    #[test]
    fn parses_exponent_display_and_decimal_print_settings() {
        assert_eq!(parsed_setting("set exp off"), SetSetting::MinExponent(0));
        assert_eq!(parsed_setting("set exp auto"), SetSetting::MinExponent(-1));
        assert_eq!(parsed_setting("set exp 3"), SetSetting::MinExponent(3));
        assert_eq!(parsed_setting("set exp -3"), SetSetting::MinExponent(-3));
        assert_eq!(
            parsed_setting("set edisp 2"),
            SetSetting::EngineeringDisplay(2)
        );
        assert_eq!(
            parsed_setting("set min decimals 4"),
            SetSetting::MinDecimals(4)
        );
        assert_eq!(
            parsed_setting("set max decimals 2"),
            SetSetting::MaxDecimals(2)
        );
    }
}
