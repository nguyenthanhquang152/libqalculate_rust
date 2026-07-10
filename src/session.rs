use crate::parser::commands::{parse_command, ApproximationMode, SessionCommand, SetSetting};
use std::path::PathBuf;

const DEFAULT_QALC_PRECISION_DIGITS: usize = 10;
// Native precision evidence is deliberately bounded so CLI settings cannot
// request MPFR allocation through the fallback-disabled scaffold.
const MAX_NATIVE_PRECISION_DIGITS: usize = 4096;

fn parse_standard_base(value: &str) -> Option<u32> {
    let base = if value.eq_ignore_ascii_case("bin") || value.eq_ignore_ascii_case("binary") {
        2
    } else if value.eq_ignore_ascii_case("oct") || value.eq_ignore_ascii_case("octal") {
        8
    } else if value.eq_ignore_ascii_case("dec") || value.eq_ignore_ascii_case("decimal") {
        10
    } else if value.eq_ignore_ascii_case("hex") || value.eq_ignore_ascii_case("hexadecimal") {
        16
    } else {
        value.parse::<u32>().ok()?
    };
    (2..=36).contains(&base).then_some(base)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct NativeSessionSettings {
    input_base: Option<u32>,
    pub(crate) output_base: Option<u32>,
    pub(crate) programming_mode: bool,
    invalid_programming_base: bool,
    xor_caret: bool,
    unicode: bool,
    unicode_setting_seen: bool,
    precision_digits: Option<usize>,
    interval_display: Option<u8>,
    interval_calculation: Option<u8>,
    concise_uncertainty: bool,
    approximation: Option<ApproximationMode>,
    fraction_format: Option<u32>,
    min_exp: Option<i32>,
    exp_display: Option<u8>,
    min_decimals: Option<i32>,
    max_decimals: Option<i32>,
}

impl NativeSessionSettings {
    pub(crate) fn from_raw(settings: &[&str]) -> Option<Self> {
        let mut state = Self::default();
        for setting in settings {
            let s_trimmed = setting.trim();
            if s_trimmed == "base -- --" {
                state.invalid_programming_base = true;
                continue;
            } else if let Some(stripped) = s_trimmed.strip_prefix("base ") {
                let parts: Vec<&str> = stripped.split_whitespace().collect();
                if parts.len() == 1 {
                    state.output_base = Some(parse_standard_base(parts[0])?);
                } else if parts.len() == 2 {
                    state.input_base = Some(parse_standard_base(parts[0])?);
                    state.output_base = Some(parse_standard_base(parts[1])?);
                } else {
                    return None;
                }
                continue;
            } else if let Some(stripped) = s_trimmed.strip_prefix("xor^ ") {
                let val = stripped.trim();
                state.xor_caret = match val {
                    "0" => false,
                    "1" => true,
                    _ => return None,
                };
                continue;
            } else if let Some(stripped) = s_trimmed.strip_prefix("programming mode ") {
                let val = stripped.trim();
                if val == "1" {
                    state.programming_mode = true;
                } else if val == "0" {
                    state.programming_mode = false;
                } else {
                    return None;
                }
                continue;
            }

            let cmd_str = normalized_context_command(setting);
            let cmd = parse_command(&cmd_str).ok()?;
            match cmd {
                SessionCommand::Set(c) => match c.setting {
                    SetSetting::InputBase(b) if (2..=36).contains(&b) => {
                        state.input_base = Some(b);
                    }
                    SetSetting::OutputBase(b) if (2..=36).contains(&b) => {
                        state.output_base = Some(b);
                    }
                    SetSetting::Unicode(value) => {
                        state.unicode = value;
                        state.unicode_setting_seen = true;
                    }
                    SetSetting::Precision(p) if p > 0 && p <= MAX_NATIVE_PRECISION_DIGITS => {
                        state.precision_digits = Some(p);
                    }
                    SetSetting::IntervalDisplay(2) => {
                        state.interval_display = Some(2);
                    }
                    SetSetting::IntervalCalculation(2) => {
                        state.interval_calculation = Some(2);
                    }
                    SetSetting::ConciseUncertainty(true) => {
                        state.concise_uncertainty = true;
                    }
                    SetSetting::Approximation(a)
                        if a == ApproximationMode::Exact || a == ApproximationMode::TryExact =>
                    {
                        state.approximation = Some(a);
                    }
                    SetSetting::FractionFormat(2) => {
                        state.fraction_format = Some(2);
                    }
                    SetSetting::EngineeringDisplay(ed) if ed <= 2 => {
                        state.exp_display = Some(ed);
                    }
                    SetSetting::MinExponent(v) => {
                        state.min_exp = Some(v);
                    }
                    SetSetting::MinDecimals(v) => {
                        state.min_decimals = Some(v);
                    }
                    SetSetting::MaxDecimals(v) => {
                        state.max_decimals = Some(v);
                    }
                    _ => return None,
                },
                SessionCommand::Assume(_) => {}
            }
        }
        Some(state)
    }

    pub(crate) const fn input_base(self) -> Option<u32> {
        self.input_base
    }

    pub(crate) const fn unicode(self) -> bool {
        self.unicode
    }

    pub(crate) const fn caret_is_xor(self) -> bool {
        self.xor_caret
    }

    pub(crate) const fn has_invalid_programming_base(self) -> bool {
        self.invalid_programming_base
    }

    pub(crate) const fn has_unicode_setting(self) -> bool {
        self.unicode_setting_seen
    }

    pub(crate) const fn precision_digits(self) -> usize {
        match self.precision_digits {
            Some(precision) => precision,
            None => DEFAULT_QALC_PRECISION_DIGITS,
        }
    }

    pub(crate) const fn is_empty(self) -> bool {
        self.input_base.is_none()
            && self.output_base.is_none()
            && !self.programming_mode
            && !self.invalid_programming_base
            && !self.xor_caret
            && !self.unicode
            && !self.unicode_setting_seen
            && self.precision_digits.is_none()
            && self.interval_display.is_none()
            && self.interval_calculation.is_none()
            && !self.concise_uncertainty
            && self.approximation.is_none()
            && self.fraction_format.is_none()
            && self.min_exp.is_none()
            && self.exp_display.is_none()
            && self.min_decimals.is_none()
            && self.max_decimals.is_none()
    }

    /// Returns true when settings can be applied to the vetted numeric
    /// scaffold path without invoking numberbase-specific interpretation.
    pub(crate) const fn is_numeric_scaffold_compatible(self) -> bool {
        matches!(self.input_base, None | Some(10))
    }

    pub(crate) const fn has_precision(self) -> bool {
        self.precision_digits.is_some()
    }

    pub(crate) const fn has_interval_display(self) -> bool {
        self.interval_display.is_some()
    }

    pub(crate) const fn has_interval_calculation(self) -> bool {
        self.interval_calculation.is_some()
    }

    pub(crate) const fn has_concise_uncertainty(self) -> bool {
        self.concise_uncertainty
    }

    #[allow(dead_code)]
    pub(crate) const fn approximation(self) -> Option<ApproximationMode> {
        self.approximation
    }

    pub(crate) const fn number_fraction_format(
        self,
    ) -> Option<crate::options::NumberFractionFormat> {
        match self.fraction_format {
            Some(2) => Some(crate::options::NumberFractionFormat::Fractional),
            _ => None,
        }
    }

    pub(crate) const fn has_non_default_approximation(self) -> bool {
        !matches!(self.approximation, None | Some(ApproximationMode::TryExact))
    }

    pub(crate) const fn has_print_format_settings(self) -> bool {
        self.min_exp.is_some()
            || self.exp_display.is_some()
            || self.min_decimals.is_some()
            || self.max_decimals.is_some()
    }

    #[allow(dead_code)]
    pub(crate) const fn min_exp(self) -> Option<i32> {
        self.min_exp
    }

    #[allow(dead_code)]
    pub(crate) const fn exp_display(self) -> Option<u8> {
        self.exp_display
    }

    #[allow(dead_code)]
    pub(crate) const fn min_decimals(self) -> Option<i32> {
        self.min_decimals
    }

    #[allow(dead_code)]
    pub(crate) const fn max_decimals(self) -> Option<i32> {
        self.max_decimals
    }
}

fn normalized_context_command(setting: &str) -> String {
    let trimmed = setting.trim_start();
    if let Some(rest) = trimmed.strip_prefix("assumptions ") {
        format!("assume {rest}")
    } else if let Some(rest) = trimmed.strip_prefix("/assumptions ") {
        format!("assume {rest}")
    } else if trimmed.starts_with("set ")
        || trimmed.starts_with("/set ")
        || trimmed.starts_with("assume ")
        || trimmed.starts_with("/assume ")
    {
        setting.to_string()
    } else {
        format!("set {setting}")
    }
}

pub(crate) fn apply_raw_settings_to_context(
    context: &mut crate::context::CalculatorContext,
    settings: &[&str],
) -> Option<()> {
    for setting in settings {
        let trimmed = setting.trim();
        if let Some(base) = trimmed.strip_prefix("base ") {
            let parts = base.split_whitespace().collect::<Vec<_>>();
            match parts.as_slice() {
                [output] => {
                    let output = parse_standard_base(output)?;
                    context.output_base = output;
                    context.print_options.base = i32::try_from(output).ok()?;
                }
                [input, output] => {
                    let input = parse_standard_base(input)?;
                    let output = parse_standard_base(output)?;
                    context.input_base = input;
                    context.parse_options.base = i32::try_from(input).ok()?;
                    context.output_base = output;
                    context.print_options.base = i32::try_from(output).ok()?;
                }
                _ => return None,
            }
            continue;
        }
        if trimmed == "base -- --"
            || trimmed.starts_with("xor^ ")
            || trimmed.starts_with("programming mode ")
        {
            continue;
        }
        context
            .apply_command(&normalized_context_command(setting))
            .ok()?;
    }
    Some(())
}

pub(crate) fn native_output(
    expr: &str,
    context: &mut crate::context::CalculatorContext,
) -> Result<Option<String>, crate::data::CsvLoadError> {
    let trimmed = expr.trim();
    if let Some((variable, path)) = parse_load_assignment(trimmed) {
        let vector = crate::data::load_csv_vector(path)?;
        context.variables.insert(variable.to_owned(), vector);
        return Ok(Some(String::new()));
    }

    if let Some(variable) = parse_delete(trimmed) {
        context.variables.remove(variable);
        return Ok(Some(String::new()));
    }

    crate::statistics::native_context_output(trimmed, context)
}

fn parse_load_assignment(expr: &str) -> Option<(&str, PathBuf)> {
    if expr.contains(":=") || expr.contains("=:") {
        return None;
    }
    let (variable, value) = expr.split_once('=')?;
    if value.contains('=') {
        return None;
    }
    let variable = variable.trim();
    if !is_session_variable_name(variable) {
        return None;
    }
    Some((variable, parse_load_path(value.trim())?))
}

fn parse_delete(expr: &str) -> Option<&str> {
    let rest = expr
        .trim_start()
        .strip_prefix("delete ")
        .or_else(|| expr.trim_start().strip_prefix("DELETE "))?;
    let variable = rest.trim();
    is_session_variable_name(variable).then_some(variable)
}

fn parse_load_path(expr: &str) -> Option<PathBuf> {
    let inner = expr.strip_prefix("load(")?.strip_suffix(')')?.trim();
    if inner.is_empty() {
        return None;
    }

    let path = inner
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .unwrap_or(inner);
    Some(PathBuf::from(path))
}

fn is_session_variable_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return false;
    }
    chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_supported_settings() {
        assert_eq!(
            NativeSessionSettings::from_raw(&[
                "set input base 16",
                "input base 10",
                "/set unicode 1",
                "/set precision 128",
                "/set interval display 2",
                "/set ic 2",
                "/set concise uncertainty 1"
            ]),
            Some(NativeSessionSettings {
                input_base: Some(10),
                output_base: None,
                programming_mode: false,
                invalid_programming_base: false,
                xor_caret: false,
                unicode: true,
                unicode_setting_seen: true,
                precision_digits: Some(128),
                interval_display: Some(2),
                interval_calculation: Some(2),
                concise_uncertainty: true,
                approximation: None,
                fraction_format: None,
                min_exp: None,
                exp_display: None,
                min_decimals: None,
                max_decimals: None,
            })
        );
        assert_eq!(
            NativeSessionSettings::from_raw(&["set approximation exact", "set fr 2",]),
            Some(NativeSessionSettings {
                input_base: None,
                output_base: None,
                programming_mode: false,
                invalid_programming_base: false,
                xor_caret: false,
                unicode: false,
                unicode_setting_seen: false,
                precision_digits: None,
                interval_display: None,
                interval_calculation: None,
                concise_uncertainty: false,
                approximation: Some(ApproximationMode::Exact),
                fraction_format: Some(2),
                min_exp: None,
                exp_display: None,
                min_decimals: None,
                max_decimals: None,
            })
        );
        assert_eq!(
            NativeSessionSettings::from_raw(&[
                "set exp 3",
                "set edisp 2",
                "set min decimals 2",
                "set max decimals 4",
            ]),
            Some(NativeSessionSettings {
                input_base: None,
                output_base: None,
                programming_mode: false,
                invalid_programming_base: false,
                xor_caret: false,
                unicode: false,
                unicode_setting_seen: false,
                precision_digits: None,
                interval_display: None,
                interval_calculation: None,
                concise_uncertainty: false,
                approximation: None,
                fraction_format: None,
                min_exp: Some(3),
                exp_display: Some(2),
                min_decimals: Some(2),
                max_decimals: Some(4),
            })
        );
    }

    #[test]
    fn rejects_unsupported_settings() {
        assert_eq!(NativeSessionSettings::from_raw(&["precision 0"]), None);
        assert_eq!(NativeSessionSettings::from_raw(&["precision 4097"]), None);
        assert_eq!(
            NativeSessionSettings::from_raw(&["interval display 1"]),
            None
        );
        assert_eq!(NativeSessionSettings::from_raw(&["ic 1"]), None);
        assert_eq!(
            NativeSessionSettings::from_raw(&["concise uncertainty 0"]),
            None
        );
        assert_eq!(
            NativeSessionSettings::from_raw(&["angle unit radians"]),
            None
        );
    }

    #[test]
    fn parses_stats_batch_csv_setup_and_delete_lines() {
        assert_eq!(
            parse_load_assignment("libqalculate_tests_vector=load(tests/vectordata.csv)")
                .map(|(name, path)| (name.to_owned(), path)),
            Some((
                "libqalculate_tests_vector".to_owned(),
                PathBuf::from("tests/vectordata.csv")
            ))
        );
        assert_eq!(
            parse_load_assignment("libqalculate_tests_vector=load(\"tests/vectordata.csv\")")
                .map(|(name, path)| (name.to_owned(), path)),
            Some((
                "libqalculate_tests_vector".to_owned(),
                PathBuf::from("tests/vectordata.csv")
            ))
        );
        assert_eq!(parse_load_assignment("1=load(tests/vectordata.csv)"), None);
        assert_eq!(parse_load_assignment("x=1"), None);
        assert_eq!(
            parse_delete("delete libqalculate_tests_vector"),
            Some("libqalculate_tests_vector")
        );
    }

    #[test]
    fn test_native_session_settings_radix_validation() {
        // Supported bases
        let base_16 = NativeSessionSettings::from_raw(&["base 16"]).unwrap();
        assert_eq!(base_16.input_base(), None);

        let prog_10_16 = NativeSessionSettings::from_raw(&["base 10 16"]).unwrap();
        assert_eq!(prog_10_16.input_base(), Some(10));

        for (name, expected) in [
            ("bin", 2),
            ("binary", 2),
            ("oct", 8),
            ("octal", 8),
            ("dec", 10),
            ("decimal", 10),
            ("hex", 16),
            ("hexadecimal", 16),
        ] {
            let output = NativeSessionSettings::from_raw(&[&format!("base {name}")]).unwrap();
            assert_eq!(output.output_base, Some(expected));

            let programming =
                NativeSessionSettings::from_raw(&[&format!("base {name} {name}")]).unwrap();
            assert_eq!(programming.input_base(), Some(expected));
            assert_eq!(programming.output_base, Some(expected));
        }

        // Unsupported/invalid/malformed bases (must reject / return None, not panic)
        assert!(NativeSessionSettings::from_raw(&["base 0"]).is_none());
        assert!(NativeSessionSettings::from_raw(&["base 1"]).is_none());
        assert!(NativeSessionSettings::from_raw(&["base 37"]).is_none());
        assert!(NativeSessionSettings::from_raw(&["base 9999999999"]).is_none());
        assert!(NativeSessionSettings::from_raw(&["base -5"]).is_none());
        assert!(NativeSessionSettings::from_raw(&["base 16 99"]).is_none());
        assert!(NativeSessionSettings::from_raw(&["base 1 16"]).is_none());
        assert!(NativeSessionSettings::from_raw(&["base 10 37"]).is_none());
        assert!(NativeSessionSettings::from_raw(&["set base 16"]).is_none());
        assert!(NativeSessionSettings::from_raw(&["/set base 16"]).is_none());
    }
}
