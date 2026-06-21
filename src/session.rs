use crate::parser::commands::{parse_command, ApproximationMode, SessionCommand, SetSetting};

const DEFAULT_QALC_PRECISION_DIGITS: usize = 10;
// Native precision evidence is deliberately bounded so CLI settings cannot
// request MPFR allocation through the fallback-disabled scaffold.
const MAX_NATIVE_PRECISION_DIGITS: usize = 4096;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct NativeSessionSettings {
    input_base: Option<u32>,
    unicode: bool,
    precision_digits: Option<usize>,
    interval_display: Option<u8>,
    interval_calculation: Option<u8>,
    concise_uncertainty: bool,
    approximation: Option<ApproximationMode>,
    fraction_format: Option<u32>,
}

impl NativeSessionSettings {
    pub(crate) fn from_raw(settings: &[&str]) -> Option<Self> {
        let mut state = Self::default();
        for setting in settings {
            let cmd_str = if setting.trim_start().starts_with("set ")
                || setting.trim_start().starts_with("/set ")
                || setting.trim_start().starts_with("assume ")
                || setting.trim_start().starts_with("/assume ")
            {
                setting.to_string()
            } else {
                format!("set {}", setting)
            };
            let cmd = parse_command(&cmd_str).ok()?;
            match cmd {
                SessionCommand::Set(c) => match c.setting {
                    SetSetting::InputBase(b) if b == 10 || b == 16 => {
                        state.input_base = Some(b);
                    }
                    SetSetting::Unicode(true) => {
                        state.unicode = true;
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

    pub(crate) const fn precision_digits(self) -> usize {
        match self.precision_digits {
            Some(precision) => precision,
            None => DEFAULT_QALC_PRECISION_DIGITS,
        }
    }

    pub(crate) const fn is_empty(self) -> bool {
        self.input_base.is_none()
            && !self.unicode
            && self.precision_digits.is_none()
            && self.interval_display.is_none()
            && self.interval_calculation.is_none()
            && !self.concise_uncertainty
            && self.approximation.is_none()
            && self.fraction_format.is_none()
    }

    /// Returns true when settings can be applied to the vetted numeric
    /// scaffold path without invoking numberbase-specific interpretation.
    pub(crate) const fn is_numeric_scaffold_compatible(self) -> bool {
        self.input_base.is_none() && !self.unicode
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

    #[allow(dead_code)]
    pub(crate) const fn fraction_format(self) -> Option<u32> {
        self.fraction_format
    }

    #[allow(dead_code)]
    pub(crate) const fn has_approximation(self) -> bool {
        self.approximation.is_some()
    }

    #[allow(dead_code)]
    pub(crate) const fn has_fraction_format(self) -> bool {
        self.fraction_format.is_some()
    }
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
                unicode: true,
                precision_digits: Some(128),
                interval_display: Some(2),
                interval_calculation: Some(2),
                concise_uncertainty: true,
                approximation: None,
                fraction_format: None,
            })
        );
        assert_eq!(
            NativeSessionSettings::from_raw(&["set approximation exact", "set fr 2",]),
            Some(NativeSessionSettings {
                input_base: None,
                unicode: false,
                precision_digits: None,
                interval_display: None,
                interval_calculation: None,
                concise_uncertainty: false,
                approximation: Some(ApproximationMode::Exact),
                fraction_format: Some(2),
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
}
