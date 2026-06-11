//! Typed session settings supported by fallback-disabled native evidence.

const DEFAULT_QALC_PRECISION_DIGITS: usize = 10;
// Native precision evidence is deliberately bounded so CLI settings cannot
// request unbounded MPFR allocation through the fallback-disabled scaffold.
const MAX_NATIVE_PRECISION_DIGITS: usize = 4096;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct NativeSessionSettings {
    input_base: Option<u32>,
    unicode: bool,
    precision_digits: Option<usize>,
    interval_display: Option<u8>,
}

impl NativeSessionSettings {
    pub(crate) fn from_raw(settings: &[&str]) -> Option<Self> {
        let mut state = Self::default();
        for setting in settings {
            match normalize_session_setting(setting) {
                "input base 16" => state.input_base = Some(16),
                "input base 10" => state.input_base = Some(10),
                "unicode 1" => state.unicode = true,
                "interval display 2" => state.interval_display = Some(2),
                normalized => {
                    let precision = normalized.strip_prefix("precision ")?;
                    state.precision_digits = Some(parse_precision_digits(precision)?);
                }
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
}

fn normalize_session_setting(setting: &str) -> &str {
    let trimmed = setting.trim();
    trimmed
        .strip_prefix("/set ")
        .or_else(|| trimmed.strip_prefix("set "))
        .unwrap_or(trimmed)
        .trim()
}

fn parse_precision_digits(value: &str) -> Option<usize> {
    let digits = value.trim().parse::<usize>().ok()?;
    (1..=MAX_NATIVE_PRECISION_DIGITS)
        .contains(&digits)
        .then_some(digits)
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
                "/set interval display 2"
            ]),
            Some(NativeSessionSettings {
                input_base: Some(10),
                unicode: true,
                precision_digits: Some(128),
                interval_display: Some(2),
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
        assert_eq!(
            NativeSessionSettings::from_raw(&["angle unit radians"]),
            None
        );
    }
}
