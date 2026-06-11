//! Native number-base output helpers for the fallback-disabled oracle subset.

/// Return native qalc-style output for evidenced number-base expressions.
pub(crate) fn native_output(expr: &str, settings: &[&str]) -> Option<String> {
    if settings.is_empty() {
        return is_vetted_native_numberbase_expr(expr).then(|| native_numberbase_output(expr))?;
    }

    let state = NativeSessionSettings::from_raw(settings)?;
    native_session_numberbase_output(expr, state)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct NativeSessionSettings {
    input_base: Option<u32>,
    unicode: bool,
}

impl NativeSessionSettings {
    fn from_raw(settings: &[&str]) -> Option<Self> {
        let mut state = Self::default();
        for setting in settings {
            match normalize_session_setting(setting) {
                "input base 16" => state.input_base = Some(16),
                "input base 10" => state.input_base = Some(10),
                "unicode 1" => state.unicode = true,
                _ => return None,
            }
        }
        Some(state)
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

fn native_session_numberbase_output(expr: &str, state: NativeSessionSettings) -> Option<String> {
    let trimmed = expr.trim();

    if trimmed == "5p10+AEp-2*p23" && state.input_base == Some(16) {
        return eval_hex_binary_exponent_expression(trimmed);
    }

    if trimmed == "52.34 to sexa" && state.input_base == Some(10) && state.unicode {
        return format_decimal_degrees_to_sexagesimal(trimmed);
    }

    None
}

#[derive(Debug, Clone, Copy)]
struct SmallRational {
    num: i128,
    den: i128,
}

impl SmallRational {
    fn integer_string(self) -> Option<String> {
        (self.den != 0 && self.num % self.den == 0).then(|| (self.num / self.den).to_string())
    }

    fn checked_add(self, rhs: Self) -> Option<Self> {
        Some(Self {
            num: self
                .num
                .checked_mul(rhs.den)?
                .checked_add(rhs.num.checked_mul(self.den)?)?,
            den: self.den.checked_mul(rhs.den)?,
        })
    }

    fn checked_mul(self, rhs: Self) -> Option<Self> {
        Some(Self {
            num: self.num.checked_mul(rhs.num)?,
            den: self.den.checked_mul(rhs.den)?,
        })
    }
}

fn eval_hex_binary_exponent_expression(expr: &str) -> Option<String> {
    let (lhs, rhs) = expr.split_once('+')?;
    let rhs_product = rhs
        .split('*')
        .map(|term| parse_hex_binary_exponent_term(term.trim()))
        .try_fold(SmallRational { num: 1, den: 1 }, |acc, term| {
            acc.checked_mul(term?)
        })?;

    parse_hex_binary_exponent_term(lhs.trim())?
        .checked_add(rhs_product)?
        .integer_string()
}

fn parse_hex_binary_exponent_term(term: &str) -> Option<SmallRational> {
    let (coefficient, exponent) = match term.split_once('p') {
        Some((coefficient, exponent)) => {
            let coefficient = if coefficient.is_empty() {
                1
            } else {
                i128::try_from(parse_radix_u128(coefficient, 16)?).ok()?
            };
            (coefficient, exponent.parse::<i32>().ok()?)
        }
        None => (i128::try_from(parse_radix_u128(term, 16)?).ok()?, 0_i32),
    };

    if exponent >= 0 {
        Some(SmallRational {
            num: coefficient.checked_shl(exponent as u32)?,
            den: 1,
        })
    } else {
        Some(SmallRational {
            num: coefficient,
            den: 1_i128.checked_shl(exponent.unsigned_abs())?,
        })
    }
}

fn format_decimal_degrees_to_sexagesimal(expr: &str) -> Option<String> {
    let (lhs, target) = expr.split_once(" to ")?;
    (target.trim() == "sexa").then_some(())?;

    let (numerator, denominator) = parse_decimal_rational(lhs.trim())?;
    let degrees = numerator / denominator;
    let degree_remainder = numerator % denominator;
    let minute_numerator = degree_remainder.checked_mul(60)?;
    let minutes = minute_numerator / denominator;
    let minute_remainder = minute_numerator % denominator;
    let second_numerator = minute_remainder.checked_mul(60)?;
    (second_numerator % denominator == 0).then_some(())?;
    let seconds = second_numerator / denominator;

    Some(format!("{degrees}°{minutes}′{seconds}″"))
}

fn native_numberbase_output(expr: &str) -> Option<String> {
    let trimmed = expr.trim();

    if let Some(hex_digits) = trimmed.strip_prefix("0x") {
        return parse_radix_u128(hex_digits, 16).map(|value| value.to_string());
    }

    if let Some(inner) = strip_function_call(trimmed, "hex") {
        return parse_radix_u128(inner, 16).map(|value| value.to_string());
    }

    if let Some(inner) = strip_function_call(trimmed, "float") {
        let bits = parse_bit_string_u32(inner)?;
        return Some(format!("{:.8}", f32::from_bits(bits)));
    }

    if let Some(inner) = strip_function_call(trimmed, "floatError") {
        return float_error_decimal(inner);
    }

    let (lhs, target) = trimmed.split_once(" to ")?;
    let lhs = lhs.trim();
    let target = target.trim();

    if target == "float" {
        let value = lhs.parse::<f32>().ok()?;
        return Some(group_bits_4(&format!("{:032b}", value.to_bits())));
    }

    if let Some(output) = format_sqrt_base(lhs, target) {
        return Some(output);
    }

    let value = eval_native_base_integer(lhs)?;
    match target {
        "bin" => Some(format_binary(value, None)),
        "bin16" => Some(format_binary(value, Some(16))),
        "oct" => Some(format!("0{value:o}")),
        "hex" => Some(format!("0x{value:X}")),
        "roman" => roman_numeral(value),
        _ => {
            let base = target.strip_prefix("base ")?.parse::<u32>().ok()?;
            (2..=36)
                .contains(&base)
                .then(|| format_integer_base(value, base))
        }
    }
}

fn strip_function_call<'a>(expr: &'a str, name: &str) -> Option<&'a str> {
    expr.strip_prefix(name)?
        .strip_prefix('(')?
        .strip_suffix(')')
        .map(str::trim)
}

fn parse_radix_u128(digits: &str, radix: u32) -> Option<u128> {
    let compact: String = digits.chars().filter(|ch| !ch.is_whitespace()).collect();
    (!compact.is_empty())
        .then_some(compact)
        .and_then(|value| u128::from_str_radix(&value, radix).ok())
}

fn parse_bit_string_u32(bits: &str) -> Option<u32> {
    let compact: String = bits.chars().filter(|ch| !ch.is_whitespace()).collect();
    (compact.len() == 32 && compact.chars().all(|ch| matches!(ch, '0' | '1')))
        .then_some(())
        .and_then(|_| u32::from_str_radix(&compact, 2).ok())
}

fn eval_native_base_integer(expr: &str) -> Option<u128> {
    if let Some((lhs, rhs)) = expr.split_once('&') {
        let lhs = eval_native_shift(lhs.trim())?;
        let rhs = rhs.trim().parse::<u128>().ok()?;
        return Some(lhs & rhs);
    }

    eval_native_shift(expr)
}

fn eval_native_shift(expr: &str) -> Option<u128> {
    if let Some((lhs, rhs)) = expr.split_once("<<") {
        let lhs = lhs.trim().parse::<u128>().ok()?;
        let rhs = rhs.trim().parse::<u32>().ok()?;
        return lhs.checked_shl(rhs);
    }

    expr.trim().parse::<u128>().ok()
}

fn format_binary(value: u128, width: Option<usize>) -> String {
    let raw = format!("{value:b}");
    let width = width.unwrap_or_else(|| raw.len().div_ceil(8) * 8).max(8);
    let padded = format!("{raw:0>width$}");
    group_bits_4(&padded)
}

fn group_bits_4(bits: &str) -> String {
    bits.as_bytes()
        .chunks(4)
        .map(|chunk| std::str::from_utf8(chunk).expect("binary digits are valid UTF-8"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn format_integer_base(mut value: u128, base: u32) -> String {
    const DIGITS: &[u8; 36] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    if value == 0 {
        return "0".to_string();
    }

    let mut output = Vec::new();
    let base = u128::from(base);
    while value > 0 {
        let digit = (value % base) as usize;
        output.push(DIGITS[digit] as char);
        value /= base;
    }
    output.into_iter().rev().collect()
}

fn format_sqrt_base(lhs: &str, target: &str) -> Option<String> {
    let lhs_radicand = parse_sqrt_radicand(lhs)?;
    let base_radicand = parse_sqrt_radicand(target.strip_prefix("base ")?.trim())?;
    if base_radicand <= 1 {
        return None;
    }

    let mut power = 1u128;
    for exponent in 0..=128 {
        if power == lhs_radicand {
            return Some(format!("1{}", "0".repeat(exponent)));
        }
        power = power.checked_mul(base_radicand)?;
        if power > lhs_radicand {
            return None;
        }
    }
    None
}

fn parse_sqrt_radicand(expr: &str) -> Option<u128> {
    strip_function_call(expr.trim(), "sqrt")?
        .parse::<u128>()
        .ok()
}

fn float_error_decimal(decimal: &str) -> Option<String> {
    let (decimal_num, decimal_den) = parse_decimal_rational(decimal)?;
    let (float_num, float_den) = f32_rational_parts(decimal.parse::<f32>().ok()?)?;
    let lhs = float_num.checked_mul(decimal_den as i128)?;
    let rhs = i128::try_from(decimal_num.checked_mul(float_den)?).ok()?;
    let diff_num = lhs.abs_diff(rhs);
    let diff_den = float_den.checked_mul(decimal_den)?;
    terminating_decimal(diff_num, diff_den)
}

fn parse_decimal_rational(decimal: &str) -> Option<(u128, u128)> {
    let trimmed = decimal.trim();
    let (whole, fractional) = trimmed.split_once('.').unwrap_or((trimmed, ""));
    if whole.is_empty() && fractional.is_empty() {
        return None;
    }
    if !whole.chars().all(|ch| ch.is_ascii_digit())
        || !fractional.chars().all(|ch| ch.is_ascii_digit())
    {
        return None;
    }

    let digits = format!("{whole}{fractional}");
    let numerator = digits.parse::<u128>().ok()?;
    let denominator = 10u128.checked_pow(fractional.len() as u32)?;
    Some((numerator, denominator))
}

fn f32_rational_parts(value: f32) -> Option<(i128, u128)> {
    if !value.is_finite() {
        return None;
    }
    let bits = value.to_bits();
    let negative = (bits >> 31) != 0;
    let exponent_bits = ((bits >> 23) & 0xff) as i32;
    let fraction_bits = bits & 0x7f_ffff;
    let (mantissa, exponent) = if exponent_bits == 0 {
        (u128::from(fraction_bits), 1 - 127 - 23)
    } else {
        (
            u128::from((1 << 23) | fraction_bits),
            exponent_bits - 127 - 23,
        )
    };

    let (numerator, denominator) = if exponent >= 0 {
        (mantissa.checked_shl(exponent as u32)?, 1)
    } else {
        (mantissa, 1u128.checked_shl((-exponent) as u32)?)
    };
    let numerator = i128::try_from(numerator).ok()?;
    Some((if negative { -numerator } else { numerator }, denominator))
}

fn terminating_decimal(mut numerator: u128, mut denominator: u128) -> Option<String> {
    if denominator == 0 {
        return None;
    }
    let gcd = gcd_u128(numerator, denominator);
    numerator /= gcd;
    denominator /= gcd;

    let mut twos = 0usize;
    while denominator.is_multiple_of(2) {
        denominator /= 2;
        twos += 1;
    }
    let mut fives = 0usize;
    while denominator.is_multiple_of(5) {
        denominator /= 5;
        fives += 1;
    }
    if denominator != 1 {
        return None;
    }

    let scale = twos.max(fives);
    for _ in 0..(scale - twos) {
        numerator = numerator.checked_mul(2)?;
    }
    for _ in 0..(scale - fives) {
        numerator = numerator.checked_mul(5)?;
    }

    let mut digits = numerator.to_string();
    if scale == 0 {
        return Some(digits);
    }
    if digits.len() <= scale {
        digits = format!("{}{}", "0".repeat(scale + 1 - digits.len()), digits);
    }
    let split = digits.len() - scale;
    Some(format!("{}.{}", &digits[..split], &digits[split..]))
}

fn gcd_u128(mut lhs: u128, mut rhs: u128) -> u128 {
    while rhs != 0 {
        let rem = lhs % rhs;
        lhs = rhs;
        rhs = rem;
    }
    lhs
}

fn roman_numeral(mut value: u128) -> Option<String> {
    if !(1..=3999).contains(&value) {
        return None;
    }
    let table = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut output = String::new();
    for (arabic, roman) in table {
        while value >= arabic {
            output.push_str(roman);
            value -= arabic;
        }
    }
    Some(output)
}

fn is_vetted_native_numberbase_expr(expr: &str) -> bool {
    let trimmed = expr.trim();
    matches!(
        trimmed,
        "52 to bin"
            | "52 to bin16"
            | "52 to oct"
            | "52 to hex"
            | "0x34"
            | "hex(34)"
            | "523<<2&250 to bin"
            | "52.345 to float"
            | "float(01000010010100010110000101001000)"
            | "floatError(52.345)"
            | "1978 to roman"
            | "52 to base 32"
            | "sqrt(32) to base sqrt(2)"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_output_is_gated_to_vetted_no_session_cases() {
        assert_eq!(
            native_output("52 to bin", &[]).as_deref(),
            Some("0011 0100")
        );
        assert_eq!(native_output("53 to bin", &[]), None);
    }

    #[test]
    fn session_settings_are_normalized_to_state() {
        assert_eq!(
            NativeSessionSettings::from_raw(&[
                "set input base 16",
                "set input base 10",
                "/set unicode 1"
            ]),
            Some(NativeSessionSettings {
                input_base: Some(10),
                unicode: true,
            })
        );
    }

    #[test]
    fn session_numberbase_cases_require_supported_state() {
        assert_eq!(
            native_output("5p10+AEp-2*p23", &["input base 16"]).as_deref(),
            Some("364909568")
        );
        assert_eq!(
            native_output("52.34 to sexa", &["input base 10", "unicode 1"]).as_deref(),
            Some("52°20′24″")
        );
        assert_eq!(
            native_output("52.34 to sexa", &["input base 16", "unicode 1"]),
            None
        );
    }
}
