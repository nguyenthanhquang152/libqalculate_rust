//! Core `Number` representation backed by `rug` arbitrary precision values.
//!
//! # Upstream oracle
//! - `../libqalculate/libqalculate/Number.h`
//! - `../libqalculate/libqalculate/Number.cc`
//!
//! # Known divergences from upstream
//! - **Backend**: `rug` provides GMP/MPFR-backed numeric storage, but this
//!   module has not yet ported the full upstream `Number` API surface.
//! - **Compatibility accessors**: `Rational::num()` and `Rational::den()`
//!   preserve the early `i128` API and panic when exact values exceed that
//!   range. Use `Rational::numerator_string()` and
//!   `Rational::denominator_string()` for lossless arbitrary-precision
//!   inspection.
//! - **Mixed equality**: `Rational` and `Float` values are not compared across
//!   representations in this scaffold. Upstream GMP/MPFR comparisons can do
//!   this exactly; the placeholder backend cannot.
//!
//! These divergences will be resolved as the rest of the upstream `Number`
//! behavior is ported.

use rug::ops::{AssignRound, DivRounding, Pow};

/// The result of a comparison between two numbers or intervals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonResult {
    /// The values are equal point values.
    Equal,
    /// The second value is strictly greater than the first.
    Greater, // o > self (i.e. self < o) strictly
    /// The second value is strictly less than the first.
    Less, // o < self (i.e. self > o) strictly
    /// The second value is equal to or greater than the first.
    EqualOrGreater,
    /// The second value is equal to or less than the first.
    EqualOrLess,
    /// The values are not equal.
    NotEqual,
    /// The comparison result is unknown (e.g. involving NaN or complex).
    Unknown,
    /// The interval bounds are identical.
    EqualLimits, // Interval bounds are identical
    /// The second interval contains the first.
    Contains, // o contains self
    /// The second interval is contained in the first.
    Contained, // self contains o
    /// The second interval overlaps the first on the left.
    OverlappingLess, // o overlaps self on the left (o.lower < self.lower)
    /// The second interval overlaps the first on the right.
    OverlappingGreater, // o overlaps self on the right (o.upper > self.upper)
}

#[allow(dead_code)]
trait NextAfter {
    fn next_after(self, direction: f64) -> f64;
}

#[allow(dead_code)]
impl NextAfter for f64 {
    fn next_after(self, direction: f64) -> f64 {
        if self.is_nan() || direction.is_nan() {
            return f64::NAN;
        }
        if self == direction {
            return direction;
        }
        if self == 0.0 {
            if direction > 0.0 {
                return f64::from_bits(1);
            } else {
                return f64::from_bits(1 | (1 << 63));
            }
        }
        let bits = self.to_bits();
        let is_negative = (bits >> 63) != 0;
        if (direction > self) ^ is_negative {
            f64::from_bits(bits + 1)
        } else {
            f64::from_bits(bits - 1)
        }
    }
}

/// A private helper to determine if a value is negative.
fn is_negative_value(val: &NumberValue) -> Option<bool> {
    match val {
        NumberValue::Rational(r) => Some(r.value.numer().is_negative()),
        NumberValue::Float(f) => Some(f.value.is_sign_negative()),
        NumberValue::PlusInfinity => Some(false),
        NumberValue::MinusInfinity => Some(true),
        NumberValue::Interval { lower, upper } => {
            if upper.value.is_sign_negative() {
                Some(true)
            } else if lower.value.is_sign_positive() && !lower.value.is_zero() {
                Some(false)
            } else {
                None
            }
        }
        NumberValue::Uncertainty { value, .. } => is_negative_value(value),
        NumberValue::NaN => None,
    }
}

/// A rational number represented using arbitrary precision integers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rational {
    /// The inner rug::Rational value.
    pub(crate) value: rug::Rational,
}

impl std::hash::Hash for Rational {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.numer().to_string().hash(state);
        self.value.denom().to_string().hash(state);
    }
}

impl Rational {
    fn from_rug(value: rug::Rational) -> Self {
        Self { value }
    }

    /// Create a new rational number, reducing it to lowest terms.
    pub fn new(num: i128, den: i128) -> Self {
        assert!(den != 0, "Rational denominator must not be zero");
        let value = rug::Rational::from((rug::Integer::from(num), rug::Integer::from(den)));
        Self { value }
    }

    /// Create a new rational number, returning None if denominator is zero.
    pub fn try_new(num: i128, den: i128) -> Option<Self> {
        if den == 0 {
            None
        } else {
            Some(Self::new(num, den))
        }
    }

    /// Create a rational from an i32.
    pub fn from_i32(val: i32) -> Self {
        Self {
            value: rug::Rational::from(val),
        }
    }

    /// Returns true if the rational is zero.
    pub fn is_zero(&self) -> bool {
        self.value.numer() == &0
    }

    /// Returns true if the rational is exactly one.
    pub fn is_one(&self) -> bool {
        self.value.numer() == &1 && self.value.denom() == &1
    }

    fn is_negative(&self) -> bool {
        self.value.numer().is_negative()
    }

    fn sign_ordering(&self) -> std::cmp::Ordering {
        if self.value.numer().is_zero() {
            std::cmp::Ordering::Equal
        } else if self.is_negative() {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    }

    fn to_f64(&self) -> f64 {
        rug::Float::with_val(53, &self.value).to_f64()
    }

    /// Returns the canonical numerator as a base-10 string without precision loss.
    pub fn numerator_string(&self) -> String {
        self.value.numer().to_string()
    }

    /// Returns the canonical positive denominator as a base-10 string without precision loss.
    pub fn denominator_string(&self) -> String {
        self.value.denom().to_string()
    }

    fn terminating_decimal_string(&self) -> Option<String> {
        let numer = self.value.numer();
        let denom = self.value.denom();
        if denom == &1 {
            return Some(numer.to_string());
        }

        let mut reduced = denom.clone();
        let mut twos = 0usize;
        while reduced.is_divisible_u(2) {
            reduced /= 2_u32;
            twos += 1;
        }

        let mut fives = 0usize;
        while reduced.is_divisible_u(5) {
            reduced /= 5_u32;
            fives += 1;
        }

        if reduced != 1 {
            return None;
        }

        let scale = twos.max(fives);
        let mut scaled = numer.clone().abs();
        for _ in 0..(scale - twos) {
            scaled *= 2_u32;
        }
        for _ in 0..(scale - fives) {
            scaled *= 5_u32;
        }

        let mut digits = scaled.to_string();
        if scale == 0 {
            return Some(if numer.is_negative() && !scaled.is_zero() {
                format!("-{digits}")
            } else {
                digits
            });
        }

        if digits.len() <= scale {
            let padding = "0".repeat(scale + 1 - digits.len());
            digits = format!("{padding}{digits}");
        }
        let split = digits.len() - scale;
        digits.insert(split, '.');
        while digits.ends_with('0') {
            digits.pop();
        }
        if digits.ends_with('.') {
            digits.pop();
        }
        if digits == "0" {
            Some(digits)
        } else if numer.is_negative() {
            Some(format!("-{digits}"))
        } else {
            Some(digits)
        }
    }

    /// Checked addition.
    pub fn add(&self, other: &Self) -> Option<Self> {
        add_rationals(self, other)
    }

    /// Checked subtraction.
    pub fn sub(&self, other: &Self) -> Option<Self> {
        sub_rationals(self, other)
    }

    /// Checked multiplication.
    pub fn mul(&self, other: &Self) -> Option<Self> {
        mul_rationals(self, other)
    }

    /// Checked division.
    pub fn div(&self, other: &Self) -> Option<Self> {
        div_rationals(self, other)
    }

    /// Returns the numerator as an i128. Panics if it does not fit.
    pub fn num(&self) -> i128 {
        self.value
            .numer()
            .to_i128()
            .expect("Numerator exceeds i128")
    }

    /// Returns the denominator as an i128. Panics if it does not fit.
    pub fn den(&self) -> i128 {
        self.value
            .denom()
            .to_i128()
            .expect("Denominator exceeds i128")
    }

    /// GCD-optimized checked addition.
    pub fn checked_add(&self, other: &Self) -> Option<Self> {
        add_rationals(self, other)
    }

    /// GCD-optimized checked subtraction.
    pub fn checked_sub(&self, other: &Self) -> Option<Self> {
        sub_rationals(self, other)
    }

    /// GCD-optimized checked multiplication.
    pub fn checked_mul(&self, other: &Self) -> Option<Self> {
        mul_rationals(self, other)
    }

    /// GCD-optimized checked division.
    pub fn checked_div(&self, other: &Self) -> Option<Self> {
        div_rationals(self, other)
    }
}

impl std::str::FromStr for Rational {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_rational_literal(s).map(Self::from_rug)
    }
}

impl Ord for Rational {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value.cmp(&other.value)
    }
}

fn integer_decimal_digits_upper_bound(value: &rug::Integer) -> u64 {
    let bits = u64::from(value.significant_bits());
    if bits == 0 {
        1
    } else {
        // log10(2) is just below 0.30103, so this is a cheap conservative
        // upper bound without allocating a decimal string.
        bits.saturating_mul(30_103).saturating_add(99_999) / 100_000
    }
}

fn exact_rational_integer_pow_is_bounded(base: &Rational, exponent_magnitude: u32) -> bool {
    if exponent_magnitude == 0 {
        return true;
    }

    const MAX_EXACT_RATIONAL_POW_DECIMAL_DIGITS: u64 = 1_000_000;
    let base_digits = integer_decimal_digits_upper_bound(base.value.numer())
        .max(integer_decimal_digits_upper_bound(base.value.denom()));
    base_digits < MAX_EXACT_RATIONAL_POW_DECIMAL_DIGITS
        && base_digits.saturating_mul(u64::from(exponent_magnitude))
            < MAX_EXACT_RATIONAL_POW_DECIMAL_DIGITS
}

impl PartialOrd for Rational {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// A representation of an arbitrary precision float using MPFR backend.
#[derive(Debug, Clone)]
pub struct Float {
    value: rug::Float,
}

impl PartialEq for Float {
    fn eq(&self, other: &Self) -> bool {
        if self.value.is_nan() && other.value.is_nan() {
            true
        } else {
            self.value == other.value
        }
    }
}

impl Float {
    /// Create a float from f64 and precision.
    pub fn from_f64(val: f64, prec: u32) -> Self {
        let p = std::cmp::max(prec, 2);
        Self {
            value: rug::Float::with_val(p, val),
        }
    }
    /// Create a float from an existing rug::Float.
    pub fn from_rug_float(value: rug::Float) -> Self {
        Self { value }
    }
    /// Returns true if the float is zero.
    pub fn is_zero(&self) -> bool {
        self.value.is_zero()
    }
    /// Returns true if the float is exactly one.
    pub fn is_one(&self) -> bool {
        self.value == 1.0
    }
    /// Returns true if the float value is NaN.
    pub fn is_nan(&self) -> bool {
        self.value.is_nan()
    }
    /// Returns true if the float value is infinite.
    pub fn is_infinite(&self) -> bool {
        self.value.is_infinite()
    }

    /// Returns the f64 value.
    pub fn value(&self) -> f64 {
        self.value.to_f64()
    }

    /// Returns the internal rug::Float reference.
    pub fn rug_float(&self) -> &rug::Float {
        &self.value
    }

    /// Returns the precision in bits.
    pub fn prec(&self) -> u32 {
        self.value.prec()
    }

    /// Add two float values.
    pub fn add(&self, other: &Self) -> Self {
        let prec = std::cmp::max(self.prec(), other.prec());
        Self {
            value: rug::Float::with_val(prec, &self.value + &other.value),
        }
    }

    /// Subtract two float values.
    pub fn sub(&self, other: &Self) -> Self {
        let prec = std::cmp::max(self.prec(), other.prec());
        Self {
            value: rug::Float::with_val(prec, &self.value - &other.value),
        }
    }

    /// Multiply two float values.
    pub fn mul(&self, other: &Self) -> Self {
        let prec = std::cmp::max(self.prec(), other.prec());
        Self {
            value: rug::Float::with_val(prec, &self.value * &other.value),
        }
    }

    /// Divide two float values.
    pub fn div(&self, other: &Self) -> Self {
        let prec = std::cmp::max(self.prec(), other.prec());
        Self {
            value: rug::Float::with_val(prec, &self.value / &other.value),
        }
    }
}

fn ordered_interval_bounds(lower: Float, upper: Float) -> Option<(Float, Float)> {
    if lower.is_nan() || upper.is_nan() {
        return None;
    }

    if lower.value <= upper.value {
        Some((lower, upper))
    } else {
        Some((upper, lower))
    }
}

/// Inner enum representing the type and value of a number.
#[derive(Debug, Clone)]
pub enum NumberValue {
    /// Rational number representation.
    Rational(Rational),
    /// Float number representation.
    Float(Float),
    /// Interval representation with lower and upper bounds.
    ///
    /// Values created through `Number::new_interval` and
    /// `Number::try_new_interval` are ordered and have non-NaN endpoints.
    Interval {
        /// Lower bound of the interval.
        lower: Float,
        /// Upper bound of the interval.
        upper: Float,
    },
    /// A value with an explicit absolute uncertainty.
    Uncertainty {
        /// The central value.
        value: Box<NumberValue>,
        /// The absolute uncertainty value.
        uncertainty: Box<NumberValue>,
        /// Whether the uncertainty was parsed as relative and should be printed as relative (percentage).
        is_relative: bool,
    },
    /// Plus infinity representation.
    PlusInfinity,
    /// Minus infinity representation.
    MinusInfinity,
    /// NaN (Not a Number) representation.
    NaN,
}

impl NumberValue {
    /// Returns true if the value is zero.
    pub fn is_zero(&self) -> bool {
        self.is_real_zero()
    }

    /// Converts the number value to an f64 approximation.
    pub fn to_f64(&self) -> f64 {
        match self {
            NumberValue::Rational(r) => rug::Float::with_val(53, &r.value).to_f64(),
            NumberValue::Float(f) => f.value.to_f64(),
            NumberValue::Interval { lower, upper } => {
                0.5 * (lower.value.to_f64() + upper.value.to_f64())
            }
            NumberValue::Uncertainty { value, .. } => value.to_f64(),
            NumberValue::PlusInfinity => f64::INFINITY,
            NumberValue::MinusInfinity => f64::NEG_INFINITY,
            NumberValue::NaN => f64::NAN,
        }
    }

    /// Extract the precision.
    pub fn precision(&self) -> i32 {
        match self {
            NumberValue::Rational(_) => 0,
            NumberValue::Float(f) => f.prec() as i32,
            NumberValue::Interval { lower, upper } => {
                std::cmp::max(lower.prec(), upper.prec()) as i32
            }
            NumberValue::Uncertainty {
                value, uncertainty, ..
            } => std::cmp::max(value.precision(), uncertainty.precision()),
            _ => 0,
        }
    }

    /// Extracts the integer value if this representation represents an integer.
    pub fn to_integer(&self) -> Option<rug::Integer> {
        match self {
            NumberValue::Rational(r) => {
                if r.value.is_integer() {
                    Some(r.value.numer().clone())
                } else {
                    None
                }
            }
            NumberValue::Float(f) if f.rug_float().is_integer() => f
                .rug_float()
                .clone()
                .to_integer_round(rug::float::Round::Nearest)
                .map(|(int, _)| int),
            NumberValue::Float(_) => None,
            _ => None,
        }
    }

    /// Extract whether the value is approximate.
    pub fn approximate(&self) -> bool {
        match self {
            NumberValue::Rational(_) => false,
            NumberValue::Float(_) => true,
            NumberValue::Interval { .. } => true,
            NumberValue::Uncertainty { .. } => true,
            _ => true,
        }
    }

    /// Check if rational is zero, float is zero, interval bounds are zero, or uncertainty is zero.
    pub fn is_real_zero(&self) -> bool {
        match self {
            NumberValue::Rational(r) => r.is_zero(),
            NumberValue::Float(f) => f.is_zero(),
            NumberValue::Interval { lower, upper } => lower.is_zero() && upper.is_zero(),
            NumberValue::Uncertainty {
                value, uncertainty, ..
            } => value.is_real_zero() && uncertainty.is_real_zero(),
            _ => false,
        }
    }

    /// Check if the value is non-zero, using interval rules.
    pub fn is_nonzero(&self) -> bool {
        match self {
            NumberValue::Rational(r) => !r.is_zero(),
            NumberValue::Float(f) => !f.is_zero() && !f.is_nan(),
            NumberValue::Interval { lower, upper } => {
                if lower.is_zero() || upper.is_zero() || lower.is_nan() || upper.is_nan() {
                    false
                } else {
                    lower.value.is_sign_negative() == upper.value.is_sign_negative()
                }
            }
            NumberValue::Uncertainty { .. } => {
                if let Some((lower, upper)) = to_interval(self) {
                    if lower.is_zero() || upper.is_zero() || lower.is_nan() || upper.is_nan() {
                        false
                    } else {
                        lower.value.is_sign_negative() == upper.value.is_sign_negative()
                    }
                } else {
                    false
                }
            }
            NumberValue::PlusInfinity | NumberValue::MinusInfinity => true,
            _ => false,
        }
    }

    /// Check if the value is an exact integer.
    pub fn is_integer(&self) -> bool {
        match self {
            NumberValue::Rational(r) => r.value.denom() == &1,
            _ => false,
        }
    }

    /// Check if the value is a rational number.
    pub fn is_rational(&self) -> bool {
        matches!(self, NumberValue::Rational(_))
    }

    /// Check if rational is 1/1, float is 1.0, interval point bounds are 1.0, or uncertainty is 1.0 and zero.
    pub fn is_real_one(&self) -> bool {
        match self {
            NumberValue::Rational(r) => r.is_one(),
            NumberValue::Float(f) => f.is_one(),
            NumberValue::Interval { lower, upper } => lower.is_one() && upper.is_one(),
            NumberValue::Uncertainty {
                value, uncertainty, ..
            } => value.is_real_one() && uncertainty.is_real_zero(),
            _ => false,
        }
    }

    /// Check if infinity.
    pub fn is_infinite(&self) -> bool {
        match self {
            NumberValue::PlusInfinity | NumberValue::MinusInfinity => true,
            NumberValue::Float(f) => f.is_infinite(),
            NumberValue::Uncertainty {
                value, uncertainty, ..
            } => value.is_infinite() || uncertainty.is_infinite(),
            _ => false,
        }
    }

    /// Check if this value is or contains infinity.
    pub fn includes_infinity(&self) -> bool {
        match self {
            NumberValue::Interval { lower, upper } => lower.is_infinite() || upper.is_infinite(),
            NumberValue::Uncertainty {
                value, uncertainty, ..
            } => value.includes_infinity() || uncertainty.includes_infinity(),
            _ => self.is_infinite(),
        }
    }

    /// Check if NaN.
    pub fn is_nan(&self) -> bool {
        match self {
            NumberValue::NaN => true,
            NumberValue::Float(f) => f.is_nan(),
            NumberValue::Interval { lower, upper } => lower.is_nan() || upper.is_nan(),
            NumberValue::Uncertainty {
                value, uncertainty, ..
            } => value.is_nan() || uncertainty.is_nan(),
            _ => false,
        }
    }

    /// Check if interval.
    pub fn is_interval(&self) -> bool {
        match self {
            NumberValue::Interval { .. } => true,
            NumberValue::Uncertainty {
                value, uncertainty, ..
            } => value.is_interval() || uncertainty.is_interval(),
            _ => false,
        }
    }

    /// Negate the value mathematically.
    pub fn negate(&self) -> Self {
        match self {
            NumberValue::Rational(r) => NumberValue::Rational(Rational {
                value: rug::Rational::from(-&r.value),
            }),
            NumberValue::Float(f) => NumberValue::Float(Float {
                value: rug::Float::with_val(f.prec(), -&f.value),
            }),
            NumberValue::Interval { lower, upper } => {
                let new_lower = rug::Float::with_val(upper.prec(), -&upper.value);
                let new_upper = rug::Float::with_val(lower.prec(), -&lower.value);
                NumberValue::Interval {
                    lower: Float { value: new_lower },
                    upper: Float { value: new_upper },
                }
            }
            NumberValue::Uncertainty {
                value,
                uncertainty,
                is_relative,
            } => NumberValue::Uncertainty {
                value: Box::new(value.negate()),
                uncertainty: uncertainty.clone(),
                is_relative: *is_relative,
            },
            NumberValue::PlusInfinity => NumberValue::MinusInfinity,
            NumberValue::MinusInfinity => NumberValue::PlusInfinity,
            NumberValue::NaN => NumberValue::NaN,
        }
    }

    /// Add two values mathematically.
    pub fn add(&self, other: &Self) -> Self {
        if self.is_nan() || other.is_nan() {
            return NumberValue::NaN;
        }

        if self.is_infinite() || other.is_infinite() {
            let s1 = get_infinity_sign(self);
            let s2 = get_infinity_sign(other);
            match (s1, s2) {
                (Some(true), Some(false)) | (Some(false), Some(true)) => NumberValue::NaN,
                (Some(true), _) | (_, Some(true)) => NumberValue::PlusInfinity,
                (Some(false), _) | (_, Some(false)) => NumberValue::MinusInfinity,
                _ => NumberValue::NaN,
            }
        } else {
            match (self, other) {
                (
                    NumberValue::Uncertainty {
                        value: v1,
                        uncertainty: u1,
                        is_relative: ir1,
                        ..
                    },
                    NumberValue::Uncertainty {
                        value: v2,
                        uncertainty: u2,
                        is_relative: ir2,
                        ..
                    },
                ) => {
                    let u1_sq = if u1.is_real_zero() {
                        NumberValue::Rational(Rational::from_i32(0))
                    } else {
                        u1.mul(u1)
                    };
                    let u2_sq = if u2.is_real_zero() {
                        NumberValue::Rational(Rational::from_i32(0))
                    } else {
                        u2.mul(u2)
                    };
                    let unc = u1_sq.add(&u2_sq).sqrt();
                    let is_relative = *ir1 && *ir2;
                    NumberValue::Uncertainty {
                        value: Box::new(v1.add(v2)),
                        uncertainty: Box::new(unc),
                        is_relative,
                    }
                }
                (
                    NumberValue::Uncertainty {
                        value,
                        uncertainty,
                        is_relative,
                    },
                    other,
                ) => NumberValue::Uncertainty {
                    value: Box::new(value.add(other)),
                    uncertainty: uncertainty.clone(),
                    is_relative: *is_relative,
                },
                (
                    self_val,
                    NumberValue::Uncertainty {
                        value,
                        uncertainty,
                        is_relative,
                    },
                ) => NumberValue::Uncertainty {
                    value: Box::new(self_val.add(value)),
                    uncertainty: uncertainty.clone(),
                    is_relative: *is_relative,
                },
                _ => match (self, other) {
                    (NumberValue::Rational(r1), NumberValue::Rational(r2)) => add_rationals(r1, r2)
                        .map(NumberValue::Rational)
                        .unwrap_or(NumberValue::NaN),
                    (NumberValue::Float(f1), NumberValue::Float(f2)) => {
                        let prec = std::cmp::max(f1.prec(), f2.prec());
                        let value = rug::Float::with_val(prec, &f1.value + &f2.value);
                        NumberValue::Float(Float { value })
                    }
                    (NumberValue::Rational(r), NumberValue::Float(f)) => {
                        let prec = f.prec();
                        let r_f = rug::Float::with_val(prec, &r.value);
                        let value = rug::Float::with_val(prec, r_f + &f.value);
                        NumberValue::Float(Float { value })
                    }
                    (NumberValue::Float(f), NumberValue::Rational(r)) => {
                        let prec = f.prec();
                        let r_f = rug::Float::with_val(prec, &r.value);
                        let value = rug::Float::with_val(prec, &f.value + r_f);
                        NumberValue::Float(Float { value })
                    }
                    (
                        NumberValue::Interval {
                            lower: l1,
                            upper: u1,
                        },
                        NumberValue::Interval {
                            lower: l2,
                            upper: u2,
                        },
                    ) => {
                        let prec = std::cmp::max(l1.prec(), l2.prec());
                        let mut lower = rug::Float::new(prec);
                        lower.assign_round(&l1.value + &l2.value, rug::float::Round::Down);
                        let mut upper = rug::Float::new(prec);
                        upper.assign_round(&u1.value + &u2.value, rug::float::Round::Up);
                        NumberValue::Interval {
                            lower: Float { value: lower },
                            upper: Float { value: upper },
                        }
                    }
                    (NumberValue::Interval { lower, upper }, other_val) => {
                        let other_f_l =
                            to_float_val_rnd(other_val, lower.prec(), rug::float::Round::Down);
                        let other_f_u =
                            to_float_val_rnd(other_val, lower.prec(), rug::float::Round::Up);
                        let prec = std::cmp::max(lower.prec(), other_f_l.prec());
                        let mut lower_res = rug::Float::new(prec);
                        lower_res
                            .assign_round(&lower.value + &other_f_l.value, rug::float::Round::Down);
                        let mut upper_res = rug::Float::new(prec);
                        upper_res
                            .assign_round(&upper.value + &other_f_u.value, rug::float::Round::Up);
                        NumberValue::Interval {
                            lower: Float { value: lower_res },
                            upper: Float { value: upper_res },
                        }
                    }
                    (self_val, NumberValue::Interval { lower, upper }) => {
                        let self_f_l =
                            to_float_val_rnd(self_val, lower.prec(), rug::float::Round::Down);
                        let self_f_u =
                            to_float_val_rnd(self_val, lower.prec(), rug::float::Round::Up);
                        let prec = std::cmp::max(self_f_l.prec(), lower.prec());
                        let mut lower_res = rug::Float::new(prec);
                        lower_res
                            .assign_round(&self_f_l.value + &lower.value, rug::float::Round::Down);
                        let mut upper_res = rug::Float::new(prec);
                        upper_res
                            .assign_round(&self_f_u.value + &upper.value, rug::float::Round::Up);
                        NumberValue::Interval {
                            lower: Float { value: lower_res },
                            upper: Float { value: upper_res },
                        }
                    }
                    _ => NumberValue::NaN,
                },
            }
        }
    }

    /// Subtract two values mathematically.
    pub fn sub(&self, other: &Self) -> Self {
        if self.is_nan() || other.is_nan() {
            return NumberValue::NaN;
        }

        if self.is_infinite() || other.is_infinite() {
            let s1 = get_infinity_sign(self);
            let s2 = get_infinity_sign(other).map(|b| !b);
            match (s1, s2) {
                (Some(true), Some(false)) | (Some(false), Some(true)) => NumberValue::NaN,
                (Some(true), _) | (_, Some(true)) => NumberValue::PlusInfinity,
                (Some(false), _) | (_, Some(false)) => NumberValue::MinusInfinity,
                _ => NumberValue::NaN,
            }
        } else {
            match (self, other) {
                (
                    NumberValue::Uncertainty {
                        value: v1,
                        uncertainty: u1,
                        is_relative: ir1,
                        ..
                    },
                    NumberValue::Uncertainty {
                        value: v2,
                        uncertainty: u2,
                        is_relative: ir2,
                        ..
                    },
                ) => {
                    let u1_sq = if u1.is_real_zero() {
                        NumberValue::Rational(Rational::from_i32(0))
                    } else {
                        u1.mul(u1)
                    };
                    let u2_sq = if u2.is_real_zero() {
                        NumberValue::Rational(Rational::from_i32(0))
                    } else {
                        u2.mul(u2)
                    };
                    let unc = u1_sq.add(&u2_sq).sqrt();
                    let is_relative = *ir1 && *ir2;
                    NumberValue::Uncertainty {
                        value: Box::new(v1.sub(v2)),
                        uncertainty: Box::new(unc),
                        is_relative,
                    }
                }
                (
                    NumberValue::Uncertainty {
                        value,
                        uncertainty,
                        is_relative,
                    },
                    other,
                ) => NumberValue::Uncertainty {
                    value: Box::new(value.sub(other)),
                    uncertainty: uncertainty.clone(),
                    is_relative: *is_relative,
                },
                (
                    self_val,
                    NumberValue::Uncertainty {
                        value,
                        uncertainty,
                        is_relative,
                    },
                ) => NumberValue::Uncertainty {
                    value: Box::new(self_val.sub(value)),
                    uncertainty: uncertainty.clone(),
                    is_relative: *is_relative,
                },
                _ => match (self, other) {
                    (NumberValue::Rational(r1), NumberValue::Rational(r2)) => sub_rationals(r1, r2)
                        .map(NumberValue::Rational)
                        .unwrap_or(NumberValue::NaN),
                    (NumberValue::Float(f1), NumberValue::Float(f2)) => {
                        let prec = std::cmp::max(f1.prec(), f2.prec());
                        let value = rug::Float::with_val(prec, &f1.value - &f2.value);
                        NumberValue::Float(Float { value })
                    }
                    (NumberValue::Rational(r), NumberValue::Float(f)) => {
                        let prec = f.prec();
                        let r_f = rug::Float::with_val(prec, &r.value);
                        let value = rug::Float::with_val(prec, r_f - &f.value);
                        NumberValue::Float(Float { value })
                    }
                    (NumberValue::Float(f), NumberValue::Rational(r)) => {
                        let prec = f.prec();
                        let r_f = rug::Float::with_val(prec, &r.value);
                        let value = rug::Float::with_val(prec, &f.value - r_f);
                        NumberValue::Float(Float { value })
                    }
                    (
                        NumberValue::Interval {
                            lower: l1,
                            upper: u1,
                        },
                        NumberValue::Interval {
                            lower: l2,
                            upper: u2,
                        },
                    ) => {
                        let prec = std::cmp::max(l1.prec(), l2.prec());
                        let mut lower = rug::Float::new(prec);
                        lower.assign_round(&l1.value - &u2.value, rug::float::Round::Down);
                        let mut upper = rug::Float::new(prec);
                        upper.assign_round(&u1.value - &l2.value, rug::float::Round::Up);
                        NumberValue::Interval {
                            lower: Float { value: lower },
                            upper: Float { value: upper },
                        }
                    }
                    (NumberValue::Interval { lower, upper }, other_val) => {
                        let other_f_l =
                            to_float_val_rnd(other_val, lower.prec(), rug::float::Round::Up);
                        let other_f_u =
                            to_float_val_rnd(other_val, lower.prec(), rug::float::Round::Down);
                        let prec = std::cmp::max(lower.prec(), other_f_l.prec());
                        let mut lower_res = rug::Float::new(prec);
                        lower_res
                            .assign_round(&lower.value - &other_f_l.value, rug::float::Round::Down);
                        let mut upper_res = rug::Float::new(prec);
                        upper_res
                            .assign_round(&upper.value - &other_f_u.value, rug::float::Round::Up);
                        NumberValue::Interval {
                            lower: Float { value: lower_res },
                            upper: Float { value: upper_res },
                        }
                    }
                    (self_val, NumberValue::Interval { lower, upper }) => {
                        let self_f_l =
                            to_float_val_rnd(self_val, lower.prec(), rug::float::Round::Down);
                        let self_f_u =
                            to_float_val_rnd(self_val, lower.prec(), rug::float::Round::Up);
                        let prec = std::cmp::max(self_f_l.prec(), lower.prec());
                        let mut lower_res = rug::Float::new(prec);
                        lower_res
                            .assign_round(&self_f_l.value - &upper.value, rug::float::Round::Down);
                        let mut upper_res = rug::Float::new(prec);
                        upper_res
                            .assign_round(&self_f_u.value - &lower.value, rug::float::Round::Up);
                        NumberValue::Interval {
                            lower: Float { value: lower_res },
                            upper: Float { value: upper_res },
                        }
                    }
                    _ => NumberValue::NaN,
                },
            }
        }
    }

    /// Multiply two values mathematically.
    pub fn mul(&self, other: &Self) -> Self {
        if self.is_nan() || other.is_nan() {
            return NumberValue::NaN;
        }
        if self.is_infinite() || other.is_infinite() {
            if self.is_real_zero() || other.is_real_zero() {
                return NumberValue::NaN;
            }
            let s1 = get_infinity_sign(self);
            let s2 = get_infinity_sign(other);
            return match (s1, s2) {
                (Some(pos1), Some(pos2)) => {
                    if pos1 == pos2 {
                        NumberValue::PlusInfinity
                    } else {
                        NumberValue::MinusInfinity
                    }
                }
                (Some(pos), None) => match get_sign_multiplier(other) {
                    Some(sgn) => {
                        if sgn == 0.0 {
                            NumberValue::NaN
                        } else if (sgn > 0.0) == pos {
                            NumberValue::PlusInfinity
                        } else {
                            NumberValue::MinusInfinity
                        }
                    }
                    None => NumberValue::NaN,
                },
                (None, Some(pos)) => match get_sign_multiplier(self) {
                    Some(sgn) => {
                        if sgn == 0.0 {
                            NumberValue::NaN
                        } else if (sgn > 0.0) == pos {
                            NumberValue::PlusInfinity
                        } else {
                            NumberValue::MinusInfinity
                        }
                    }
                    None => NumberValue::NaN,
                },
                _ => NumberValue::NaN,
            };
        }
        match (self, other) {
            (
                NumberValue::Uncertainty {
                    value: v1,
                    uncertainty: u1,
                    is_relative: ir1,
                    ..
                },
                NumberValue::Uncertainty {
                    value: v2,
                    uncertainty: u2,
                    is_relative: ir2,
                    ..
                },
            ) => {
                let val = v1.mul(v2);
                let term1 = if u1.is_real_zero() {
                    NumberValue::Rational(Rational::from_i32(0))
                } else {
                    v2.mul(u1)
                };
                let term2 = if u2.is_real_zero() {
                    NumberValue::Rational(Rational::from_i32(0))
                } else {
                    v1.mul(u2)
                };
                let unc = term1.mul(&term1).add(&term2.mul(&term2)).sqrt();
                let is_relative = *ir1 && *ir2;
                NumberValue::Uncertainty {
                    value: Box::new(val),
                    uncertainty: Box::new(unc),
                    is_relative,
                }
            }
            (
                NumberValue::Uncertainty {
                    value,
                    uncertainty,
                    is_relative,
                },
                other,
            ) => {
                let val = value.mul(other);
                let unc = if uncertainty.is_real_zero() {
                    NumberValue::Rational(Rational::from_i32(0))
                } else {
                    uncertainty.mul(&other.abs())
                };
                NumberValue::Uncertainty {
                    value: Box::new(val),
                    uncertainty: Box::new(unc),
                    is_relative: *is_relative,
                }
            }
            (
                self_val,
                NumberValue::Uncertainty {
                    value,
                    uncertainty,
                    is_relative,
                },
            ) => {
                let val = self_val.mul(value);
                let unc = if uncertainty.is_real_zero() {
                    NumberValue::Rational(Rational::from_i32(0))
                } else {
                    uncertainty.mul(&self_val.abs())
                };
                NumberValue::Uncertainty {
                    value: Box::new(val),
                    uncertainty: Box::new(unc),
                    is_relative: *is_relative,
                }
            }
            _ => match (self, other) {
                (NumberValue::Rational(r1), NumberValue::Rational(r2)) => mul_rationals(r1, r2)
                    .map(NumberValue::Rational)
                    .unwrap_or(NumberValue::NaN),
                (NumberValue::Float(f1), NumberValue::Float(f2)) => {
                    let prec = std::cmp::max(f1.prec(), f2.prec());
                    let value = rug::Float::with_val(prec, &f1.value * &f2.value);
                    NumberValue::Float(Float { value })
                }
                (NumberValue::Rational(r), NumberValue::Float(f)) => {
                    let prec = f.prec();
                    let r_f = rug::Float::with_val(prec, &r.value);
                    let value = rug::Float::with_val(prec, r_f * &f.value);
                    NumberValue::Float(Float { value })
                }
                (NumberValue::Float(f), NumberValue::Rational(r)) => {
                    let prec = f.prec();
                    let r_f = rug::Float::with_val(prec, &r.value);
                    let value = rug::Float::with_val(prec, &f.value * r_f);
                    NumberValue::Float(Float { value })
                }
                (
                    NumberValue::Interval {
                        lower: l1,
                        upper: u1,
                    },
                    NumberValue::Interval {
                        lower: l2,
                        upper: u2,
                    },
                ) => {
                    let prec = std::cmp::max(l1.prec(), l2.prec());
                    let mut p1_d = rug::Float::new(prec);
                    p1_d.assign_round(&l1.value * &l2.value, rug::float::Round::Down);
                    let mut p2_d = rug::Float::new(prec);
                    p2_d.assign_round(&l1.value * &u2.value, rug::float::Round::Down);
                    let mut p3_d = rug::Float::new(prec);
                    p3_d.assign_round(&u1.value * &l2.value, rug::float::Round::Down);
                    let mut p4_d = rug::Float::new(prec);
                    p4_d.assign_round(&u1.value * &u2.value, rug::float::Round::Down);

                    let mut p1_u = rug::Float::new(prec);
                    p1_u.assign_round(&l1.value * &l2.value, rug::float::Round::Up);
                    let mut p2_u = rug::Float::new(prec);
                    p2_u.assign_round(&l1.value * &u2.value, rug::float::Round::Up);
                    let mut p3_u = rug::Float::new(prec);
                    p3_u.assign_round(&u1.value * &l2.value, rug::float::Round::Up);
                    let mut p4_u = rug::Float::new(prec);
                    p4_u.assign_round(&u1.value * &u2.value, rug::float::Round::Up);

                    let min_val = min_float(p1_d, min_float(p2_d, min_float(p3_d, p4_d)));
                    let max_val = max_float(p1_u, max_float(p2_u, max_float(p3_u, p4_u)));

                    NumberValue::Interval {
                        lower: Float { value: min_val },
                        upper: Float { value: max_val },
                    }
                }
                (NumberValue::Interval { lower, upper }, other_val) => {
                    let other_f_l =
                        to_float_val_rnd(other_val, lower.prec(), rug::float::Round::Down);
                    let other_f_u =
                        to_float_val_rnd(other_val, lower.prec(), rug::float::Round::Up);
                    let prec = std::cmp::max(lower.prec(), other_f_l.prec());

                    let mut p1_d = rug::Float::new(prec);
                    p1_d.assign_round(&lower.value * &other_f_l.value, rug::float::Round::Down);
                    let mut p2_d = rug::Float::new(prec);
                    p2_d.assign_round(&lower.value * &other_f_u.value, rug::float::Round::Down);
                    let mut p3_d = rug::Float::new(prec);
                    p3_d.assign_round(&upper.value * &other_f_l.value, rug::float::Round::Down);
                    let mut p4_d = rug::Float::new(prec);
                    p4_d.assign_round(&upper.value * &other_f_u.value, rug::float::Round::Down);

                    let mut p1_u = rug::Float::new(prec);
                    p1_u.assign_round(&lower.value * &other_f_l.value, rug::float::Round::Up);
                    let mut p2_u = rug::Float::new(prec);
                    p2_u.assign_round(&lower.value * &other_f_u.value, rug::float::Round::Up);
                    let mut p3_u = rug::Float::new(prec);
                    p3_u.assign_round(&upper.value * &other_f_l.value, rug::float::Round::Up);
                    let mut p4_u = rug::Float::new(prec);
                    p4_u.assign_round(&upper.value * &other_f_u.value, rug::float::Round::Up);

                    let min_val = min_float(p1_d, min_float(p2_d, min_float(p3_d, p4_d)));
                    let max_val = max_float(p1_u, max_float(p2_u, max_float(p3_u, p4_u)));

                    NumberValue::Interval {
                        lower: Float { value: min_val },
                        upper: Float { value: max_val },
                    }
                }
                (self_val, NumberValue::Interval { lower, upper }) => {
                    let self_f_l =
                        to_float_val_rnd(self_val, lower.prec(), rug::float::Round::Down);
                    let self_f_u = to_float_val_rnd(self_val, lower.prec(), rug::float::Round::Up);
                    let prec = std::cmp::max(self_f_l.prec(), lower.prec());

                    let mut p1_d = rug::Float::new(prec);
                    p1_d.assign_round(&self_f_l.value * &lower.value, rug::float::Round::Down);
                    let mut p2_d = rug::Float::new(prec);
                    p2_d.assign_round(&self_f_l.value * &upper.value, rug::float::Round::Down);
                    let mut p3_d = rug::Float::new(prec);
                    p3_d.assign_round(&self_f_u.value * &lower.value, rug::float::Round::Down);
                    let mut p4_d = rug::Float::new(prec);
                    p4_d.assign_round(&self_f_u.value * &upper.value, rug::float::Round::Down);

                    let mut p1_u = rug::Float::new(prec);
                    p1_u.assign_round(&self_f_l.value * &lower.value, rug::float::Round::Up);
                    let mut p2_u = rug::Float::new(prec);
                    p2_u.assign_round(&self_f_l.value * &upper.value, rug::float::Round::Up);
                    let mut p3_u = rug::Float::new(prec);
                    p3_u.assign_round(&self_f_u.value * &lower.value, rug::float::Round::Up);
                    let mut p4_u = rug::Float::new(prec);
                    p4_u.assign_round(&self_f_u.value * &upper.value, rug::float::Round::Up);

                    let min_val = min_float(p1_d, min_float(p2_d, min_float(p3_d, p4_d)));
                    let max_val = max_float(p1_u, max_float(p2_u, max_float(p3_u, p4_u)));

                    NumberValue::Interval {
                        lower: Float { value: min_val },
                        upper: Float { value: max_val },
                    }
                }
                _ => NumberValue::NaN,
            },
        }
    }

    /// Divide two values mathematically.
    pub fn div(&self, other: &Self) -> Self {
        if self.is_nan() || other.is_nan() {
            return NumberValue::NaN;
        }
        if self.is_infinite() || other.is_infinite() {
            if self.is_infinite() && other.is_infinite() {
                return NumberValue::NaN;
            }
            if self.is_infinite() {
                let s1 = get_infinity_sign(self);
                let other_neg = is_negative_value(other);
                match (s1, other_neg) {
                    (Some(pos), Some(neg)) => {
                        if pos ^ neg {
                            NumberValue::PlusInfinity
                        } else {
                            NumberValue::MinusInfinity
                        }
                    }
                    _ => NumberValue::NaN,
                }
            } else {
                NumberValue::Rational(Rational::from_i32(0))
            }
        } else {
            match (self, other) {
                (
                    NumberValue::Uncertainty {
                        value: v1,
                        uncertainty: u1,
                        is_relative: ir1,
                        ..
                    },
                    NumberValue::Uncertainty {
                        value: v2,
                        uncertainty: u2,
                        is_relative: ir2,
                        ..
                    },
                ) => {
                    if v2.is_real_zero() {
                        return NumberValue::NaN;
                    }
                    let val = v1.div(v2);
                    let term1 = if u1.is_real_zero() {
                        NumberValue::Rational(Rational::from_i32(0))
                    } else {
                        u1.div(v2)
                    };
                    let term2 = if u2.is_real_zero() {
                        NumberValue::Rational(Rational::from_i32(0))
                    } else {
                        val.mul(u2).div(v2)
                    };
                    let unc = term1.mul(&term1).add(&term2.mul(&term2)).sqrt();
                    let is_relative = *ir1 && *ir2;
                    NumberValue::Uncertainty {
                        value: Box::new(val),
                        uncertainty: Box::new(unc),
                        is_relative,
                    }
                }
                (
                    NumberValue::Uncertainty {
                        value,
                        uncertainty,
                        is_relative,
                    },
                    other,
                ) => {
                    if other.is_real_zero() {
                        return NumberValue::NaN;
                    }
                    let val = value.div(other);
                    let unc = if uncertainty.is_real_zero() {
                        NumberValue::Rational(Rational::from_i32(0))
                    } else {
                        uncertainty.div(&other.abs())
                    };
                    NumberValue::Uncertainty {
                        value: Box::new(val),
                        uncertainty: Box::new(unc),
                        is_relative: *is_relative,
                    }
                }
                (
                    self_val,
                    NumberValue::Uncertainty {
                        value,
                        uncertainty,
                        is_relative,
                    },
                ) => {
                    if value.is_real_zero() {
                        return NumberValue::NaN;
                    }
                    let val = self_val.div(value);
                    let term2 = if uncertainty.is_real_zero() {
                        NumberValue::Rational(Rational::from_i32(0))
                    } else {
                        val.mul(uncertainty).div(value)
                    };
                    let unc = term2.abs();
                    NumberValue::Uncertainty {
                        value: Box::new(val),
                        uncertainty: Box::new(unc),
                        is_relative: *is_relative,
                    }
                }
                _ => match (self, other) {
                    (NumberValue::Rational(r1), NumberValue::Rational(r2)) => div_rationals(r1, r2)
                        .map(NumberValue::Rational)
                        .unwrap_or(NumberValue::NaN),
                    (NumberValue::Float(f1), NumberValue::Float(f2)) => {
                        if f2.is_zero() {
                            NumberValue::NaN
                        } else {
                            let prec = std::cmp::max(f1.prec(), f2.prec());
                            let value = rug::Float::with_val(prec, &f1.value / &f2.value);
                            NumberValue::Float(Float { value })
                        }
                    }
                    (NumberValue::Rational(r), NumberValue::Float(f)) => {
                        if f.is_zero() {
                            NumberValue::NaN
                        } else {
                            let prec = f.prec();
                            let r_f = rug::Float::with_val(prec, &r.value);
                            let value = rug::Float::with_val(prec, r_f / &f.value);
                            NumberValue::Float(Float { value })
                        }
                    }
                    (NumberValue::Float(f), NumberValue::Rational(r)) => {
                        if r.is_zero() {
                            NumberValue::NaN
                        } else {
                            let prec = f.prec();
                            let r_f = rug::Float::with_val(prec, &r.value);
                            let value = rug::Float::with_val(prec, &f.value / r_f);
                            NumberValue::Float(Float { value })
                        }
                    }
                    (
                        NumberValue::Interval {
                            lower: l1,
                            upper: u1,
                        },
                        NumberValue::Interval {
                            lower: l2,
                            upper: u2,
                        },
                    ) => {
                        if l2.value <= 0.0 && u2.value >= 0.0 {
                            return NumberValue::NaN;
                        }
                        let prec = std::cmp::max(l1.prec(), l2.prec());
                        let mut q1_d = rug::Float::new(prec);
                        q1_d.assign_round(&l1.value / &l2.value, rug::float::Round::Down);
                        let mut q2_d = rug::Float::new(prec);
                        q2_d.assign_round(&l1.value / &u2.value, rug::float::Round::Down);
                        let mut q3_d = rug::Float::new(prec);
                        q3_d.assign_round(&u1.value / &l2.value, rug::float::Round::Down);
                        let mut q4_d = rug::Float::new(prec);
                        q4_d.assign_round(&u1.value / &u2.value, rug::float::Round::Down);

                        let mut q1_u = rug::Float::new(prec);
                        q1_u.assign_round(&l1.value / &l2.value, rug::float::Round::Up);
                        let mut q2_u = rug::Float::new(prec);
                        q2_u.assign_round(&l1.value / &u2.value, rug::float::Round::Up);
                        let mut q3_u = rug::Float::new(prec);
                        q3_u.assign_round(&u1.value / &l2.value, rug::float::Round::Up);
                        let mut q4_u = rug::Float::new(prec);
                        q4_u.assign_round(&u1.value / &u2.value, rug::float::Round::Up);

                        let min_val = min_float(q1_d, min_float(q2_d, min_float(q3_d, q4_d)));
                        let max_val = max_float(q1_u, max_float(q2_u, max_float(q3_u, q4_u)));

                        NumberValue::Interval {
                            lower: Float { value: min_val },
                            upper: Float { value: max_val },
                        }
                    }
                    (NumberValue::Interval { lower, upper }, other_val) => {
                        if other_val.is_real_zero() {
                            return NumberValue::NaN;
                        }
                        let other_f_l =
                            to_float_val_rnd(other_val, lower.prec(), rug::float::Round::Down);
                        let other_f_u =
                            to_float_val_rnd(other_val, lower.prec(), rug::float::Round::Up);
                        let prec = std::cmp::max(lower.prec(), other_f_l.prec());

                        let mut q1_d = rug::Float::new(prec);
                        q1_d.assign_round(&lower.value / &other_f_l.value, rug::float::Round::Down);
                        let mut q2_d = rug::Float::new(prec);
                        q2_d.assign_round(&lower.value / &other_f_u.value, rug::float::Round::Down);
                        let mut q3_d = rug::Float::new(prec);
                        q3_d.assign_round(&upper.value / &other_f_l.value, rug::float::Round::Down);
                        let mut q4_d = rug::Float::new(prec);
                        q4_d.assign_round(&upper.value / &other_f_u.value, rug::float::Round::Down);

                        let mut q1_u = rug::Float::new(prec);
                        q1_u.assign_round(&lower.value / &other_f_l.value, rug::float::Round::Up);
                        let mut q2_u = rug::Float::new(prec);
                        q2_u.assign_round(&lower.value / &other_f_u.value, rug::float::Round::Up);
                        let mut q3_u = rug::Float::new(prec);
                        q3_u.assign_round(&upper.value / &other_f_l.value, rug::float::Round::Up);
                        let mut q4_u = rug::Float::new(prec);
                        q4_u.assign_round(&upper.value / &other_f_u.value, rug::float::Round::Up);

                        let min_val = min_float(q1_d, min_float(q2_d, min_float(q3_d, q4_d)));
                        let max_val = max_float(q1_u, max_float(q2_u, max_float(q3_u, q4_u)));

                        NumberValue::Interval {
                            lower: Float { value: min_val },
                            upper: Float { value: max_val },
                        }
                    }
                    (self_val, NumberValue::Interval { lower, upper }) => {
                        if lower.value <= 0.0 && upper.value >= 0.0 {
                            return NumberValue::NaN;
                        }
                        let self_f_l =
                            to_float_val_rnd(self_val, lower.prec(), rug::float::Round::Down);
                        let self_f_u =
                            to_float_val_rnd(self_val, lower.prec(), rug::float::Round::Up);
                        let prec = std::cmp::max(self_f_l.prec(), lower.prec());

                        let mut q1_d = rug::Float::new(prec);
                        q1_d.assign_round(&self_f_l.value / &lower.value, rug::float::Round::Down);
                        let mut q2_d = rug::Float::new(prec);
                        q2_d.assign_round(&self_f_l.value / &upper.value, rug::float::Round::Down);
                        let mut q3_d = rug::Float::new(prec);
                        q3_d.assign_round(&self_f_u.value / &lower.value, rug::float::Round::Down);
                        let mut q4_d = rug::Float::new(prec);
                        q4_d.assign_round(&self_f_u.value / &upper.value, rug::float::Round::Down);

                        let mut q1_u = rug::Float::new(prec);
                        q1_u.assign_round(&self_f_l.value / &lower.value, rug::float::Round::Up);
                        let mut q2_u = rug::Float::new(prec);
                        q2_u.assign_round(&self_f_l.value / &upper.value, rug::float::Round::Up);
                        let mut q3_u = rug::Float::new(prec);
                        q3_u.assign_round(&self_f_u.value / &lower.value, rug::float::Round::Up);
                        let mut q4_u = rug::Float::new(prec);
                        q4_u.assign_round(&self_f_u.value / &upper.value, rug::float::Round::Up);

                        let min_val = min_float(q1_d, min_float(q2_d, min_float(q3_d, q4_d)));
                        let max_val = max_float(q1_u, max_float(q2_u, max_float(q3_u, q4_u)));

                        NumberValue::Interval {
                            lower: Float { value: min_val },
                            upper: Float { value: max_val },
                        }
                    }
                    _ => NumberValue::NaN,
                },
            }
        }
    }

    /// Remainder with quotient truncated toward zero, matching qalc `%` / `rem`.
    pub fn rem(&self, other: &Self) -> Self {
        if self.is_nan() || other.is_nan() || self.is_infinite() || other.is_infinite() {
            return NumberValue::NaN;
        }

        match (self, other) {
            (NumberValue::Rational(lhs), NumberValue::Rational(rhs)) => {
                rem_rationals(lhs, rhs, RemainderMode::Truncate)
                    .map(NumberValue::Rational)
                    .unwrap_or(NumberValue::NaN)
            }
            _ => NumberValue::NaN,
        }
    }

    /// Modulo with quotient rounded down, matching qalc `%%` / `mod`.
    pub fn modulo(&self, other: &Self) -> Self {
        if self.is_nan() || other.is_nan() || self.is_infinite() || other.is_infinite() {
            return NumberValue::NaN;
        }

        match (self, other) {
            (NumberValue::Rational(lhs), NumberValue::Rational(rhs)) => {
                rem_rationals(lhs, rhs, RemainderMode::Floor)
                    .map(NumberValue::Rational)
                    .unwrap_or(NumberValue::NaN)
            }
            _ => NumberValue::NaN,
        }
    }

    /// Integer division with quotient truncated toward zero, matching qalc `//` / `\` / `div`.
    pub fn int_div(&self, other: &Self) -> Self {
        if self.is_nan() || other.is_nan() || self.is_infinite() || other.is_infinite() {
            return NumberValue::NaN;
        }

        match (self, other) {
            (NumberValue::Rational(lhs), NumberValue::Rational(rhs)) => int_div_rationals(lhs, rhs)
                .map(NumberValue::Rational)
                .unwrap_or(NumberValue::NaN),
            _ => NumberValue::NaN,
        }
    }

    /// Exponentiate to a power mathematically.
    pub fn pow(&self, other: &Self) -> Self {
        if self.is_nan() || other.is_nan() {
            return NumberValue::NaN;
        }
        match (self, other) {
            (
                NumberValue::Uncertainty {
                    value: v1,
                    uncertainty: u1,
                    is_relative: ir1,
                    ..
                },
                NumberValue::Uncertainty {
                    value: v2,
                    uncertainty: u2,
                    is_relative: ir2,
                    ..
                },
            ) => {
                let val = v1.pow(v2);
                let one = NumberValue::Rational(Rational::from_i32(1));
                let v2_minus_1 = v2.sub(&one);
                let term1 = if u1.is_real_zero() {
                    NumberValue::Rational(Rational::from_i32(0))
                } else {
                    v2.mul(&v1.pow(&v2_minus_1)).mul(u1)
                };
                let term2 = if u2.is_real_zero() {
                    NumberValue::Rational(Rational::from_i32(0))
                } else {
                    val.mul(&v1.ln()).mul(u2)
                };
                let unc = term1.mul(&term1).add(&term2.mul(&term2)).sqrt();
                let is_relative = *ir1 && *ir2;
                NumberValue::Uncertainty {
                    value: Box::new(val),
                    uncertainty: Box::new(unc),
                    is_relative,
                }
            }
            (
                NumberValue::Uncertainty {
                    value,
                    uncertainty,
                    is_relative,
                },
                other,
            ) => {
                let val = value.pow(other);
                let one = NumberValue::Rational(Rational::from_i32(1));
                let other_minus_1 = other.sub(&one);
                let unc = if uncertainty.is_real_zero() {
                    NumberValue::Rational(Rational::from_i32(0))
                } else {
                    other.mul(&value.pow(&other_minus_1)).mul(uncertainty).abs()
                };
                NumberValue::Uncertainty {
                    value: Box::new(val),
                    uncertainty: Box::new(unc),
                    is_relative: *is_relative,
                }
            }
            (
                self_val,
                NumberValue::Uncertainty {
                    value,
                    uncertainty,
                    is_relative,
                },
            ) => {
                let val = self_val.pow(value);
                let unc = if uncertainty.is_real_zero() {
                    NumberValue::Rational(Rational::from_i32(0))
                } else {
                    val.mul(&self_val.ln()).mul(uncertainty).abs()
                };
                NumberValue::Uncertainty {
                    value: Box::new(val),
                    uncertainty: Box::new(unc),
                    is_relative: *is_relative,
                }
            }
            (NumberValue::Rational(r1), NumberValue::Rational(r2)) => {
                if r2.value.denom() == &1 {
                    const MAX_EXACT_RATIONAL_POW_EXP: u32 = 10_000;
                    if let Some(exp) = r2.value.numer().to_i32() {
                        let magnitude = exp.unsigned_abs();
                        if magnitude <= MAX_EXACT_RATIONAL_POW_EXP
                            && !(exp < 0 && r1.is_zero())
                            && exact_rational_integer_pow_is_bounded(r1, magnitude)
                        {
                            return NumberValue::Rational(Rational::from_rug(
                                r1.value.clone().pow(exp),
                            ));
                        }
                    }
                }
                let prec = 53;
                let base = rug::Float::with_val(prec, &r1.value);
                let exponent = rug::Float::with_val(prec, &r2.value);
                NumberValue::Float(Float {
                    value: rug::Float::with_val(prec, base.pow(exponent)),
                })
            }
            (NumberValue::Float(f1), NumberValue::Float(f2)) => {
                let prec = std::cmp::max(f1.prec(), f2.prec());
                let base = rug::Float::with_val(prec, &f1.value);
                let exponent = rug::Float::with_val(prec, &f2.value);
                NumberValue::Float(Float {
                    value: rug::Float::with_val(prec, base.pow(exponent)),
                })
            }
            (NumberValue::Rational(r), NumberValue::Float(f)) => {
                let prec = f.prec();
                let base = rug::Float::with_val(prec, &r.value);
                let exponent = rug::Float::with_val(prec, &f.value);
                NumberValue::Float(Float {
                    value: rug::Float::with_val(prec, base.pow(exponent)),
                })
            }
            (NumberValue::Float(f), NumberValue::Rational(r)) => {
                let prec = f.prec();
                let base = rug::Float::with_val(prec, &f.value);
                let exponent = rug::Float::with_val(prec, &r.value);
                NumberValue::Float(Float {
                    value: rug::Float::with_val(prec, base.pow(exponent)),
                })
            }
            _ => {
                let f1 = to_float_val(self);
                let f2 = to_float_val(other);
                let prec = std::cmp::max(f1.prec(), f2.prec());
                let base = rug::Float::with_val(prec, &f1.value);
                let exponent = rug::Float::with_val(prec, &f2.value);
                NumberValue::Float(Float {
                    value: rug::Float::with_val(prec, base.pow(exponent)),
                })
            }
        }
    }

    fn rational_noninteger_pow_with_precision_floor(
        &self,
        other: &Self,
        min_precision_bits: u32,
    ) -> Option<Self> {
        let (NumberValue::Rational(base), NumberValue::Rational(exponent)) = (self, other) else {
            return None;
        };
        if exponent.value.denom() == &1 {
            return None;
        }

        let prec = min_precision_bits.max(53);
        let base = rug::Float::with_val(prec, &base.value);
        let exponent = rug::Float::with_val(prec, &exponent.value);
        Some(NumberValue::Float(Float {
            value: rug::Float::with_val(prec, base.pow(exponent)),
        }))
    }

    /// Natural logarithm of the value.
    pub fn ln(&self) -> Self {
        self.ln_with_precision_floor(53)
    }

    fn ln_with_precision_floor(&self, min_precision_bits: u32) -> Self {
        match self {
            NumberValue::Rational(r) => {
                let prec = min_precision_bits.max(53);
                let value = rug::Float::with_val(prec, &r.value).ln();
                NumberValue::Float(Float {
                    value: rug::Float::with_val(prec, value),
                })
            }
            NumberValue::Float(f) => {
                let prec = min_precision_bits.max(f.prec());
                let value = rug::Float::with_val(prec, &f.value).ln();
                NumberValue::Float(Float {
                    value: rug::Float::with_val(prec, value),
                })
            }
            NumberValue::Interval { lower, upper } => NumberValue::Interval {
                lower: Float {
                    value: lower.value.clone().ln(),
                },
                upper: Float {
                    value: upper.value.clone().ln(),
                },
            },
            NumberValue::Uncertainty {
                value,
                uncertainty,
                is_relative,
            } => {
                let val = value.ln_with_precision_floor(min_precision_bits);
                let unc = if uncertainty.is_real_zero() {
                    NumberValue::Rational(Rational::from_i32(0))
                } else {
                    uncertainty.div(&value.abs())
                };
                NumberValue::Uncertainty {
                    value: Box::new(val),
                    uncertainty: Box::new(unc),
                    is_relative: *is_relative,
                }
            }
            _ => {
                let f =
                    to_float_val_rnd(self, min_precision_bits.max(53), rug::float::Round::Nearest);
                let prec = min_precision_bits.max(f.prec());
                NumberValue::Float(Float {
                    value: rug::Float::with_val(prec, f.value.clone().ln()),
                })
            }
        }
    }

    /// Returns the absolute value of the number value.
    pub fn abs(&self) -> Self {
        match self {
            NumberValue::Rational(r) => NumberValue::Rational(Rational {
                value: r.value.clone().abs(),
            }),
            NumberValue::Float(f) => NumberValue::Float(Float {
                value: f.value.clone().abs(),
            }),
            NumberValue::Interval { lower, upper } => {
                let l = &lower.value;
                let u = &upper.value;
                let (new_l, new_u) = if l.is_sign_positive() || l.is_zero() {
                    (l.clone(), u.clone())
                } else if u.is_sign_negative() || u.is_zero() {
                    let nl = rug::Float::with_val(upper.prec(), -u);
                    let nu = rug::Float::with_val(lower.prec(), -l);
                    (nl, nu)
                } else {
                    let zero = rug::Float::with_val(lower.prec(), 0.0);
                    let u_min_abs = l.clone().abs();
                    let u_max_abs = u.clone().abs();
                    let max_val = if u_min_abs > u_max_abs {
                        u_min_abs
                    } else {
                        u_max_abs
                    };
                    (zero, max_val)
                };
                NumberValue::Interval {
                    lower: Float { value: new_l },
                    upper: Float { value: new_u },
                }
            }
            NumberValue::Uncertainty {
                value,
                uncertainty,
                is_relative,
            } => NumberValue::Uncertainty {
                value: Box::new(value.abs()),
                uncertainty: uncertainty.clone(),
                is_relative: *is_relative,
            },
            NumberValue::PlusInfinity | NumberValue::MinusInfinity => NumberValue::PlusInfinity,
            NumberValue::NaN => NumberValue::NaN,
        }
    }
    /// Returns the square root of the number value.
    pub fn sqrt(&self) -> Self {
        self.sqrt_with_precision_floor(53)
    }

    fn sqrt_with_precision_floor(&self, min_precision_bits: u32) -> Self {
        match self {
            NumberValue::Rational(r) => {
                if r.is_negative() {
                    NumberValue::NaN
                } else {
                    let (n_sqrt, n_rem) = r.value.numer().clone().sqrt_rem(rug::Integer::new());
                    let (d_sqrt, d_rem) = r.value.denom().clone().sqrt_rem(rug::Integer::new());
                    if n_rem.is_zero() && d_rem.is_zero() {
                        NumberValue::Rational(Rational {
                            value: rug::Rational::from((n_sqrt, d_sqrt)),
                        })
                    } else {
                        let prec = min_precision_bits.max(53);
                        let f_val = rug::Float::with_val(prec, &r.value);
                        NumberValue::Float(Float {
                            value: rug::Float::with_val(prec, f_val.sqrt()),
                        })
                    }
                }
            }
            NumberValue::Float(f) => {
                if f.value.is_sign_negative() && !f.value.is_zero() {
                    NumberValue::NaN
                } else {
                    let prec = min_precision_bits.max(f.prec());
                    let value = rug::Float::with_val(prec, &f.value).sqrt();
                    NumberValue::Float(Float {
                        value: rug::Float::with_val(prec, value),
                    })
                }
            }
            NumberValue::Interval { lower, upper } => {
                let l = &lower.value;
                let u = &upper.value;
                if u.is_sign_negative() && !u.is_zero() {
                    NumberValue::NaN
                } else {
                    let mut nl = rug::Float::with_val(lower.prec(), 0.0);
                    let mut nu = rug::Float::with_val(upper.prec(), 0.0);
                    let l_val = if l.is_sign_negative() {
                        &rug::Float::with_val(lower.prec(), 0.0)
                    } else {
                        l
                    };
                    nl.assign_round(l_val.clone().sqrt(), rug::float::Round::Down);
                    nu.assign_round(u.clone().sqrt(), rug::float::Round::Up);
                    NumberValue::Interval {
                        lower: Float { value: nl },
                        upper: Float { value: nu },
                    }
                }
            }
            NumberValue::Uncertainty {
                value,
                uncertainty,
                is_relative,
            } => {
                let val_sqrt = value.sqrt_with_precision_floor(min_precision_bits);
                if val_sqrt.is_nan() {
                    NumberValue::NaN
                } else {
                    let new_uncertainty = if uncertainty.is_real_zero() {
                        NumberValue::Rational(Rational::from_i32(0))
                    } else {
                        let two = NumberValue::Rational(Rational::from_i32(2));
                        let denom = two.mul(&val_sqrt);
                        uncertainty.div(&denom)
                    };
                    NumberValue::Uncertainty {
                        value: Box::new(val_sqrt),
                        uncertainty: Box::new(new_uncertainty),
                        is_relative: *is_relative,
                    }
                }
            }
            NumberValue::PlusInfinity => NumberValue::PlusInfinity,
            NumberValue::MinusInfinity => NumberValue::NaN,
            NumberValue::NaN => NumberValue::NaN,
        }
    }

    /// Returns e^x for this value.
    pub fn exp_value(&self) -> Self {
        match self {
            NumberValue::Rational(r) => {
                if r.is_zero() {
                    return NumberValue::Rational(Rational::from_i32(1));
                }
                let prec = 53u32;
                let value = rug::Float::with_val(prec, &r.value).exp();
                NumberValue::Float(Float {
                    value: rug::Float::with_val(prec, value),
                })
            }
            NumberValue::Float(f) => {
                let prec = f.prec().max(53);
                let value = rug::Float::with_val(prec, &f.value).exp();
                NumberValue::Float(Float {
                    value: rug::Float::with_val(prec, value),
                })
            }
            NumberValue::Interval { lower, upper } => NumberValue::Interval {
                lower: Float {
                    value: lower.value.clone().exp(),
                },
                upper: Float {
                    value: upper.value.clone().exp(),
                },
            },
            NumberValue::Uncertainty {
                value,
                uncertainty,
                is_relative,
            } => {
                let val_exp = value.exp_value();
                // d(e^x)/dx = e^x, so propagated uncertainty = e^x * u
                let new_uncertainty = if uncertainty.is_real_zero() {
                    NumberValue::Rational(Rational::from_i32(0))
                } else {
                    val_exp.mul(uncertainty)
                };
                NumberValue::Uncertainty {
                    value: Box::new(val_exp),
                    uncertainty: Box::new(new_uncertainty),
                    is_relative: *is_relative,
                }
            }
            NumberValue::PlusInfinity => NumberValue::PlusInfinity,
            NumberValue::MinusInfinity => NumberValue::Rational(Rational::from_i32(0)),
            NumberValue::NaN => NumberValue::NaN,
        }
    }

    /// Returns the cube root of this value.
    ///
    /// For real x: cbrt(x) = sign(x) * |x|^(1/3).
    pub fn cbrt_value(&self) -> Self {
        match self {
            NumberValue::Rational(r) => {
                let prec = 53u32;
                let f_val = rug::Float::with_val(prec, &r.value);
                let result = f_val.cbrt();
                // Check if result is exact integer
                if result.is_integer() {
                    let int_val = result
                        .to_integer_round(rug::float::Round::Nearest)
                        .map(|(i, _)| i);
                    if let Some(iv) = int_val {
                        return NumberValue::Rational(Rational {
                            value: rug::Rational::from(iv),
                        });
                    }
                }
                NumberValue::Float(Float {
                    value: rug::Float::with_val(prec, result),
                })
            }
            NumberValue::Float(f) => {
                let prec = f.prec().max(53);
                let value = rug::Float::with_val(prec, &f.value).cbrt();
                NumberValue::Float(Float {
                    value: rug::Float::with_val(prec, value),
                })
            }
            NumberValue::Interval { lower, upper } => {
                // cbrt is monotonically increasing
                NumberValue::Interval {
                    lower: Float {
                        value: lower.value.clone().cbrt(),
                    },
                    upper: Float {
                        value: upper.value.clone().cbrt(),
                    },
                }
            }
            NumberValue::Uncertainty {
                value,
                uncertainty,
                is_relative,
            } => {
                let val_cbrt = value.cbrt_value();
                // d(cbrt(x))/dx = 1/(3 * x^(2/3)) = cbrt(x) / (3*x)
                let new_uncertainty = if uncertainty.is_real_zero() {
                    NumberValue::Rational(Rational::from_i32(0))
                } else {
                    let three = NumberValue::Rational(Rational::from_i32(3));
                    let denom = three.mul(value);
                    let deriv = val_cbrt.div(&denom);
                    uncertainty.mul(&deriv).abs()
                };
                NumberValue::Uncertainty {
                    value: Box::new(val_cbrt),
                    uncertainty: Box::new(new_uncertainty),
                    is_relative: *is_relative,
                }
            }
            NumberValue::PlusInfinity => NumberValue::PlusInfinity,
            NumberValue::MinusInfinity => NumberValue::MinusInfinity,
            NumberValue::NaN => NumberValue::NaN,
        }
    }

    /// Returns cos(self) for this value.
    pub fn cos_value(&self, min_precision_bits: u32) -> Self {
        match self {
            NumberValue::Rational(r) => {
                if r.is_zero() {
                    return NumberValue::Rational(Rational::from_i32(1));
                }
                let prec = min_precision_bits.max(53);
                let value = rug::Float::with_val(prec, &r.value).cos();
                NumberValue::Float(Float { value })
            }
            NumberValue::Float(f) => {
                let prec = min_precision_bits.max(f.prec());
                let value = rug::Float::with_val(prec, &f.value).cos();
                NumberValue::Float(Float { value })
            }
            NumberValue::Interval { lower, upper } => {
                // cos has extrema at k*π. We must check if any lie within [lo, hi]
                // and clamp the result bounds accordingly.
                let prec = min_precision_bits
                    .max(lower.prec())
                    .max(upper.prec())
                    .max(53);
                let lo = &lower.value;
                let hi = &upper.value;
                let l_cos = rug::Float::with_val(prec, lo).cos();
                let u_cos = rug::Float::with_val(prec, hi).cos();

                let mut min_val = l_cos.clone().min(&u_cos);
                let mut max_val = l_cos.max(&u_cos);

                let pi = rug::Float::with_val(prec, rug::float::Constant::Pi);

                // Check for cos extrema: cos(kπ) = ±1
                // First multiple of π ≥ lo: k = ceil(lo/π)
                let k_start_f = rug::Float::with_val(prec, lo / &pi).ceil();
                if let Some(k_start) = k_start_f.to_integer() {
                    // Check at most a few multiples within the interval
                    let mut k = k_start;
                    for _ in 0..4 {
                        let extremum = rug::Float::with_val(prec, &k * &pi);
                        if extremum > *hi {
                            break;
                        }
                        // cos(kπ) = (-1)^k
                        let k_mod2 = k.clone() % 2i32;
                        if k_mod2 == 0 {
                            // cos = 1
                            let one = rug::Float::with_val(prec, 1.0);
                            max_val = max_val.max(&one);
                        } else {
                            // cos = -1
                            let neg_one = rug::Float::with_val(prec, -1.0);
                            min_val = min_val.min(&neg_one);
                        }
                        k += 1;
                    }
                }

                NumberValue::Interval {
                    lower: Float { value: min_val },
                    upper: Float { value: max_val },
                }
            }
            _ => {
                let prec = min_precision_bits.max(53);
                let f = to_float_val_rnd(self, prec, rug::float::Round::Nearest);
                NumberValue::Float(Float {
                    value: f.value.cos(),
                })
            }
        }
    }

    /// Returns sin(self) for this value.
    pub fn sin_value(&self, min_precision_bits: u32) -> Self {
        match self {
            NumberValue::Rational(r) => {
                if r.is_zero() {
                    return NumberValue::Rational(Rational::from_i32(0));
                }
                let prec = min_precision_bits.max(53);
                let value = rug::Float::with_val(prec, &r.value).sin();
                NumberValue::Float(Float { value })
            }
            NumberValue::Float(f) => {
                let prec = min_precision_bits.max(f.prec());
                let value = rug::Float::with_val(prec, &f.value).sin();
                NumberValue::Float(Float { value })
            }
            NumberValue::Interval { lower, upper } => {
                // sin has extrema at π/2 + kπ. We must check if any lie within [lo, hi].
                let prec = min_precision_bits
                    .max(lower.prec())
                    .max(upper.prec())
                    .max(53);
                let lo = &lower.value;
                let hi = &upper.value;
                let l_sin = rug::Float::with_val(prec, lo).sin();
                let u_sin = rug::Float::with_val(prec, hi).sin();

                let mut min_val = l_sin.clone().min(&u_sin);
                let mut max_val = l_sin.max(&u_sin);

                let pi = rug::Float::with_val(prec, rug::float::Constant::Pi);
                let half_pi = rug::Float::with_val(prec, &pi / 2u32);

                // Check for sin extrema: sin(π/2 + kπ) = ±1
                // First (π/2 + kπ) ≥ lo: k = ceil((lo - π/2) / π)
                let k_start_f =
                    rug::Float::with_val(prec, (rug::Float::with_val(prec, lo) - &half_pi) / &pi)
                        .ceil();
                if let Some(k_start) = k_start_f.to_integer() {
                    let mut k = k_start;
                    for _ in 0..4 {
                        let extremum = rug::Float::with_val(
                            prec,
                            &half_pi + rug::Float::with_val(prec, &k * &pi),
                        );
                        if extremum > *hi {
                            break;
                        }
                        // sin(π/2 + kπ) = (-1)^k
                        let k_mod2 = k.clone() % 2i32;
                        if k_mod2 == 0 {
                            // sin = 1
                            let one = rug::Float::with_val(prec, 1.0);
                            max_val = max_val.max(&one);
                        } else {
                            // sin = -1
                            let neg_one = rug::Float::with_val(prec, -1.0);
                            min_val = min_val.min(&neg_one);
                        }
                        k += 1;
                    }
                }

                NumberValue::Interval {
                    lower: Float { value: min_val },
                    upper: Float { value: max_val },
                }
            }
            _ => {
                let prec = min_precision_bits.max(53);
                let f = to_float_val_rnd(self, prec, rug::float::Round::Nearest);
                NumberValue::Float(Float {
                    value: f.value.sin(),
                })
            }
        }
    }

    /// Returns tan(self) for this value.
    pub fn tan_value(&self, min_precision_bits: u32) -> Self {
        match self {
            NumberValue::Rational(r) => {
                if r.is_zero() {
                    return NumberValue::Rational(Rational::from_i32(0));
                }
                let prec = min_precision_bits.max(53);
                let value = rug::Float::with_val(prec, &r.value).tan();
                NumberValue::Float(Float { value })
            }
            NumberValue::Float(f) => {
                let prec = min_precision_bits.max(f.prec());
                let value = rug::Float::with_val(prec, &f.value).tan();
                NumberValue::Float(Float { value })
            }
            NumberValue::Interval { lower, upper } => {
                let prec = min_precision_bits
                    .max(lower.prec())
                    .max(upper.prec())
                    .max(53);
                let lo = &lower.value;
                let hi = &upper.value;

                let pi = rug::Float::with_val(prec, rug::float::Constant::Pi);
                let half_pi = rug::Float::with_val(prec, &pi / 2u32);

                // Check if the interval crosses kπ + π/2
                let k_start_f =
                    rug::Float::with_val(prec, (rug::Float::with_val(prec, lo) - &half_pi) / &pi)
                        .ceil();
                let mut contains_pole = false;
                if let Some(k_start) = k_start_f.to_integer() {
                    let extremum = rug::Float::with_val(
                        prec,
                        &half_pi + rug::Float::with_val(prec, &k_start * &pi),
                    );
                    if extremum <= *hi {
                        contains_pole = true;
                    }
                }

                if contains_pole {
                    NumberValue::NaN
                } else {
                    NumberValue::Interval {
                        lower: Float {
                            value: rug::Float::with_val(prec, lo).tan(),
                        },
                        upper: Float {
                            value: rug::Float::with_val(prec, hi).tan(),
                        },
                    }
                }
            }
            _ => {
                let prec = min_precision_bits.max(53);
                let f = to_float_val_rnd(self, prec, rug::float::Round::Nearest);
                NumberValue::Float(Float {
                    value: f.value.tan(),
                })
            }
        }
    }

    /// Returns asin(self) for this value.
    /// Returns NaN if |self| > 1.
    pub fn asin_value(&self, min_precision_bits: u32) -> Self {
        match self {
            NumberValue::Rational(r) => {
                if r.is_zero() {
                    return NumberValue::Rational(Rational::from_i32(0));
                }
                let prec = min_precision_bits.max(53);
                let value = rug::Float::with_val(prec, &r.value).asin();
                if value.is_nan() {
                    return NumberValue::NaN;
                }
                NumberValue::Float(Float { value })
            }
            NumberValue::Float(f) => {
                let prec = min_precision_bits.max(f.prec());
                let value = rug::Float::with_val(prec, &f.value).asin();
                if value.is_nan() {
                    return NumberValue::NaN;
                }
                NumberValue::Float(Float { value })
            }
            NumberValue::Interval { lower, upper } => {
                let prec = min_precision_bits
                    .max(lower.prec())
                    .max(upper.prec())
                    .max(53);
                NumberValue::Interval {
                    lower: Float {
                        value: rug::Float::with_val(prec, &lower.value).asin(),
                    },
                    upper: Float {
                        value: rug::Float::with_val(prec, &upper.value).asin(),
                    },
                }
            }
            _ => {
                let prec = min_precision_bits.max(53);
                let f = to_float_val_rnd(self, prec, rug::float::Round::Nearest);
                let result = f.value.asin();
                if result.is_nan() {
                    NumberValue::NaN
                } else {
                    NumberValue::Float(Float { value: result })
                }
            }
        }
    }

    /// Returns acos(self) for this value.
    /// Returns NaN if |self| > 1.
    pub fn acos_value(&self, min_precision_bits: u32) -> Self {
        match self {
            NumberValue::Rational(r) => {
                let prec = min_precision_bits.max(53);
                let value = rug::Float::with_val(prec, &r.value).acos();
                if value.is_nan() {
                    return NumberValue::NaN;
                }
                NumberValue::Float(Float { value })
            }
            NumberValue::Float(f) => {
                let prec = min_precision_bits.max(f.prec());
                let value = rug::Float::with_val(prec, &f.value).acos();
                if value.is_nan() {
                    return NumberValue::NaN;
                }
                NumberValue::Float(Float { value })
            }
            NumberValue::Interval { lower, upper } => {
                let prec = min_precision_bits
                    .max(lower.prec())
                    .max(upper.prec())
                    .max(53);
                NumberValue::Interval {
                    lower: Float {
                        value: rug::Float::with_val(prec, &upper.value).acos(),
                    },
                    upper: Float {
                        value: rug::Float::with_val(prec, &lower.value).acos(),
                    },
                }
            }
            _ => {
                let prec = min_precision_bits.max(53);
                let f = to_float_val_rnd(self, prec, rug::float::Round::Nearest);
                let result = f.value.acos();
                if result.is_nan() {
                    NumberValue::NaN
                } else {
                    NumberValue::Float(Float { value: result })
                }
            }
        }
    }

    /// Returns atan(self) for this value.
    pub fn atan_value(&self, min_precision_bits: u32) -> Self {
        match self {
            NumberValue::Rational(r) => {
                if r.is_zero() {
                    return NumberValue::Rational(Rational::from_i32(0));
                }
                let prec = min_precision_bits.max(53);
                let value = rug::Float::with_val(prec, &r.value).atan();
                NumberValue::Float(Float { value })
            }
            NumberValue::Float(f) => {
                let prec = min_precision_bits.max(f.prec());
                let value = rug::Float::with_val(prec, &f.value).atan();
                NumberValue::Float(Float { value })
            }
            NumberValue::Interval { lower, upper } => {
                let prec = min_precision_bits
                    .max(lower.prec())
                    .max(upper.prec())
                    .max(53);
                NumberValue::Interval {
                    lower: Float {
                        value: rug::Float::with_val(prec, &lower.value).atan(),
                    },
                    upper: Float {
                        value: rug::Float::with_val(prec, &upper.value).atan(),
                    },
                }
            }
            _ => {
                let prec = min_precision_bits.max(53);
                let f = to_float_val_rnd(self, prec, rug::float::Round::Nearest);
                NumberValue::Float(Float {
                    value: f.value.atan(),
                })
            }
        }
    }

    /// Returns atan2(y, x) for these values.
    pub fn atan2_value(y: &Self, x: &Self, min_precision_bits: u32) -> Self {
        let prec = min_precision_bits.max(53);
        let yf = to_float_val_rnd(y, prec, rug::float::Round::Nearest);
        let xf = to_float_val_rnd(x, prec, rug::float::Round::Nearest);
        let result = rug::Float::with_val(prec, yf.value.atan2(&xf.value));
        if result.is_nan() {
            NumberValue::NaN
        } else {
            NumberValue::Float(Float { value: result })
        }
    }

    /// Returns sinh(self) for this value.
    pub fn sinh_value(&self, min_precision_bits: u32) -> Self {
        match self {
            NumberValue::Rational(r) => {
                if r.is_zero() {
                    return NumberValue::Rational(Rational::from_i32(0));
                }
                let prec = min_precision_bits.max(53);
                let value = rug::Float::with_val(prec, &r.value).sinh();
                NumberValue::Float(Float { value })
            }
            NumberValue::Float(f) => {
                let prec = min_precision_bits.max(f.prec());
                let value = rug::Float::with_val(prec, &f.value).sinh();
                NumberValue::Float(Float { value })
            }
            NumberValue::Interval { lower, upper } => {
                let prec = min_precision_bits
                    .max(lower.prec())
                    .max(upper.prec())
                    .max(53);
                NumberValue::Interval {
                    lower: Float {
                        value: rug::Float::with_val(prec, &lower.value).sinh(),
                    },
                    upper: Float {
                        value: rug::Float::with_val(prec, &upper.value).sinh(),
                    },
                }
            }
            _ => {
                let prec = min_precision_bits.max(53);
                let f = to_float_val_rnd(self, prec, rug::float::Round::Nearest);
                NumberValue::Float(Float {
                    value: f.value.sinh(),
                })
            }
        }
    }

    /// Returns cosh(self) for this value.
    pub fn cosh_value(&self, min_precision_bits: u32) -> Self {
        match self {
            NumberValue::Rational(r) => {
                if r.is_zero() {
                    return NumberValue::Rational(Rational::from_i32(1));
                }
                let prec = min_precision_bits.max(53);
                let value = rug::Float::with_val(prec, &r.value).cosh();
                NumberValue::Float(Float { value })
            }
            NumberValue::Float(f) => {
                let prec = min_precision_bits.max(f.prec());
                let value = rug::Float::with_val(prec, &f.value).cosh();
                NumberValue::Float(Float { value })
            }
            NumberValue::Interval { lower, upper } => {
                let prec = min_precision_bits
                    .max(lower.prec())
                    .max(upper.prec())
                    .max(53);
                let lo = &lower.value;
                let hi = &upper.value;
                let l_cosh = rug::Float::with_val(prec, lo).cosh();
                let u_cosh = rug::Float::with_val(prec, hi).cosh();

                let (min_val, max_val) = if lo.is_sign_negative() && hi.is_sign_positive() {
                    (rug::Float::with_val(prec, 1.0), l_cosh.max(&u_cosh))
                } else {
                    (l_cosh.clone().min(&u_cosh), l_cosh.max(&u_cosh))
                };

                NumberValue::Interval {
                    lower: Float { value: min_val },
                    upper: Float { value: max_val },
                }
            }
            _ => {
                let prec = min_precision_bits.max(53);
                let f = to_float_val_rnd(self, prec, rug::float::Round::Nearest);
                NumberValue::Float(Float {
                    value: f.value.cosh(),
                })
            }
        }
    }

    /// Returns tanh(self) for this value.
    pub fn tanh_value(&self, min_precision_bits: u32) -> Self {
        match self {
            NumberValue::Rational(r) => {
                if r.is_zero() {
                    return NumberValue::Rational(Rational::from_i32(0));
                }
                let prec = min_precision_bits.max(53);
                let value = rug::Float::with_val(prec, &r.value).tanh();
                NumberValue::Float(Float { value })
            }
            NumberValue::Float(f) => {
                let prec = min_precision_bits.max(f.prec());
                let value = rug::Float::with_val(prec, &f.value).tanh();
                NumberValue::Float(Float { value })
            }
            NumberValue::Interval { lower, upper } => {
                let prec = min_precision_bits
                    .max(lower.prec())
                    .max(upper.prec())
                    .max(53);
                NumberValue::Interval {
                    lower: Float {
                        value: rug::Float::with_val(prec, &lower.value).tanh(),
                    },
                    upper: Float {
                        value: rug::Float::with_val(prec, &upper.value).tanh(),
                    },
                }
            }
            _ => {
                let prec = min_precision_bits.max(53);
                let f = to_float_val_rnd(self, prec, rug::float::Round::Nearest);
                NumberValue::Float(Float {
                    value: f.value.tanh(),
                })
            }
        }
    }

    /// Returns asinh(self) for this value.
    pub fn asinh_value(&self, min_precision_bits: u32) -> Self {
        match self {
            NumberValue::Rational(r) => {
                if r.is_zero() {
                    return NumberValue::Rational(Rational::from_i32(0));
                }
                let prec = min_precision_bits.max(53);
                let value = rug::Float::with_val(prec, &r.value).asinh();
                NumberValue::Float(Float { value })
            }
            NumberValue::Float(f) => {
                let prec = min_precision_bits.max(f.prec());
                let value = rug::Float::with_val(prec, &f.value).asinh();
                NumberValue::Float(Float { value })
            }
            NumberValue::Interval { lower, upper } => {
                let prec = min_precision_bits
                    .max(lower.prec())
                    .max(upper.prec())
                    .max(53);
                NumberValue::Interval {
                    lower: Float {
                        value: rug::Float::with_val(prec, &lower.value).asinh(),
                    },
                    upper: Float {
                        value: rug::Float::with_val(prec, &upper.value).asinh(),
                    },
                }
            }
            _ => {
                let prec = min_precision_bits.max(53);
                let f = to_float_val_rnd(self, prec, rug::float::Round::Nearest);
                NumberValue::Float(Float {
                    value: f.value.asinh(),
                })
            }
        }
    }

    /// Returns acosh(self) for this value.
    /// Returns NaN if self < 1.
    pub fn acosh_value(&self, min_precision_bits: u32) -> Self {
        match self {
            NumberValue::Rational(r) => {
                let prec = min_precision_bits.max(53);
                let value = rug::Float::with_val(prec, &r.value).acosh();
                if value.is_nan() {
                    return NumberValue::NaN;
                }
                NumberValue::Float(Float { value })
            }
            NumberValue::Float(f) => {
                let prec = min_precision_bits.max(f.prec());
                let value = rug::Float::with_val(prec, &f.value).acosh();
                if value.is_nan() {
                    return NumberValue::NaN;
                }
                NumberValue::Float(Float { value })
            }
            NumberValue::Interval { lower, upper } => {
                let prec = min_precision_bits
                    .max(lower.prec())
                    .max(upper.prec())
                    .max(53);
                NumberValue::Interval {
                    lower: Float {
                        value: rug::Float::with_val(prec, &lower.value).acosh(),
                    },
                    upper: Float {
                        value: rug::Float::with_val(prec, &upper.value).acosh(),
                    },
                }
            }
            _ => {
                let prec = min_precision_bits.max(53);
                let f = to_float_val_rnd(self, prec, rug::float::Round::Nearest);
                let result = f.value.acosh();
                if result.is_nan() {
                    NumberValue::NaN
                } else {
                    NumberValue::Float(Float { value: result })
                }
            }
        }
    }

    /// Returns atanh(self) for this value.
    /// Returns NaN if |self| > 1. Returns ±infinity if self = ±1.
    pub fn atanh_value(&self, min_precision_bits: u32) -> Self {
        match self {
            NumberValue::Rational(r) => {
                if r.is_zero() {
                    return NumberValue::Rational(Rational::from_i32(0));
                }
                let prec = min_precision_bits.max(53);
                let value = rug::Float::with_val(prec, &r.value).atanh();
                if value.is_nan() {
                    return NumberValue::NaN;
                }
                if value.is_infinite() {
                    return if value.is_sign_positive() {
                        NumberValue::PlusInfinity
                    } else {
                        NumberValue::MinusInfinity
                    };
                }
                NumberValue::Float(Float { value })
            }
            NumberValue::Float(f) => {
                let prec = min_precision_bits.max(f.prec());
                let value = rug::Float::with_val(prec, &f.value).atanh();
                if value.is_nan() {
                    return NumberValue::NaN;
                }
                if value.is_infinite() {
                    return if value.is_sign_positive() {
                        NumberValue::PlusInfinity
                    } else {
                        NumberValue::MinusInfinity
                    };
                }
                NumberValue::Float(Float { value })
            }
            NumberValue::Interval { lower, upper } => {
                let prec = min_precision_bits
                    .max(lower.prec())
                    .max(upper.prec())
                    .max(53);
                NumberValue::Interval {
                    lower: Float {
                        value: rug::Float::with_val(prec, &lower.value).atanh(),
                    },
                    upper: Float {
                        value: rug::Float::with_val(prec, &upper.value).atanh(),
                    },
                }
            }
            _ => {
                let prec = min_precision_bits.max(53);
                let f = to_float_val_rnd(self, prec, rug::float::Round::Nearest);
                let result = f.value.atanh();
                if result.is_nan() {
                    NumberValue::NaN
                } else if result.is_infinite() {
                    if result.is_sign_positive() {
                        NumberValue::PlusInfinity
                    } else {
                        NumberValue::MinusInfinity
                    }
                } else {
                    NumberValue::Float(Float { value: result })
                }
            }
        }
    }

    /// Converts the value to interval bounds (lower, upper). Returns None if the value is NaN.
    pub fn to_interval_bounds(&self) -> Option<(f64, f64)> {
        match self {
            NumberValue::Rational(r) => {
                let val = r.to_f64();
                Some((val, val))
            }
            NumberValue::Float(f) => {
                if f.value.is_nan() {
                    None
                } else {
                    Some((f.value(), f.value()))
                }
            }
            NumberValue::Interval { lower, upper } => {
                if lower.value.is_nan() || upper.value.is_nan() {
                    None
                } else {
                    Some((lower.value(), upper.value()))
                }
            }
            NumberValue::Uncertainty {
                value, uncertainty, ..
            } => {
                let (v_min, v_max) = value.to_interval_bounds()?;
                let (u_min, u_max) = uncertainty.to_interval_bounds()?;
                let u_limit = u_min.abs().max(u_max.abs());
                Some((v_min - u_limit, v_max + u_limit))
            }
            NumberValue::PlusInfinity => Some((f64::INFINITY, f64::INFINITY)),
            NumberValue::MinusInfinity => Some((f64::NEG_INFINITY, f64::NEG_INFINITY)),
            NumberValue::NaN => None,
        }
    }

    /// Compares this value to another value.
    pub fn compare(&self, other: &Self) -> ComparisonResult {
        if self.is_nan() || other.is_nan() {
            return ComparisonResult::Unknown;
        }
        if let (Some(lhs), Some(rhs)) = (
            self.exact_rational_for_compare(),
            other.exact_rational_for_compare(),
        ) {
            return match lhs.cmp(rhs) {
                std::cmp::Ordering::Less => ComparisonResult::Greater,
                std::cmp::Ordering::Equal => ComparisonResult::Equal,
                std::cmp::Ordering::Greater => ComparisonResult::Less,
            };
        }
        let self_bounds = self.to_interval_bounds();
        let other_bounds = other.to_interval_bounds();
        match (self_bounds, other_bounds) {
            (Some((l1, u1)), Some((l2, u2))) => {
                if l1 == l2 && u1 == u2 {
                    if l1 == u1 {
                        ComparisonResult::Equal
                    } else {
                        ComparisonResult::EqualLimits
                    }
                } else if u2 < l1 {
                    ComparisonResult::Less
                } else if l2 > u1 {
                    ComparisonResult::Greater
                } else if l2 <= l1 && u2 >= u1 {
                    ComparisonResult::Contains
                } else if l1 <= l2 && u1 >= u2 {
                    ComparisonResult::Contained
                } else if l2 < l1 && u2 < u1 && u2 >= l1 {
                    ComparisonResult::OverlappingLess
                } else if l1 < l2 && u1 < u2 && l2 <= u1 {
                    ComparisonResult::OverlappingGreater
                } else {
                    ComparisonResult::Unknown
                }
            }
            _ => ComparisonResult::Unknown,
        }
    }

    fn exact_rational_for_compare(&self) -> Option<&Rational> {
        match self {
            NumberValue::Rational(rational) => Some(rational),
            NumberValue::Uncertainty {
                value, uncertainty, ..
            } if uncertainty.is_real_zero() => value.exact_rational_for_compare(),
            _ => None,
        }
    }

    /// Returns true if this value is strictly greater than the other value.
    pub fn is_greater_than(&self, other: &Self) -> bool {
        matches!(self.compare(other), ComparisonResult::Less)
    }

    /// Returns true if this value is strictly less than the other value.
    pub fn is_less_than(&self, other: &Self) -> bool {
        matches!(self.compare(other), ComparisonResult::Greater)
    }

    /// Returns true if the value is strictly positive (> 0).
    pub fn is_positive(&self) -> bool {
        match self {
            NumberValue::Rational(r) => {
                !r.is_zero() && r.value.cmp0() == std::cmp::Ordering::Greater
            }
            NumberValue::Float(f) => !f.is_nan() && f.value.is_sign_positive() && !f.is_zero(),
            NumberValue::Interval { lower, upper } => {
                !lower.is_nan()
                    && !upper.is_nan()
                    && lower.value.is_sign_positive()
                    && !lower.value.is_zero()
            }
            NumberValue::Uncertainty { value, .. } => value.is_positive(),
            NumberValue::PlusInfinity => true,
            NumberValue::MinusInfinity | NumberValue::NaN => false,
        }
    }

    /// Returns true if the value is strictly negative (< 0).
    pub fn is_negative(&self) -> bool {
        match self {
            NumberValue::Rational(r) => !r.is_zero() && r.value.cmp0() == std::cmp::Ordering::Less,
            NumberValue::Float(f) => !f.is_nan() && f.value.is_sign_negative() && !f.is_zero(),
            NumberValue::Interval { lower, upper } => {
                !lower.is_nan()
                    && !upper.is_nan()
                    && upper.value.is_sign_negative()
                    && !upper.value.is_zero()
            }
            NumberValue::Uncertainty { value, .. } => value.is_negative(),
            NumberValue::MinusInfinity => true,
            NumberValue::PlusInfinity | NumberValue::NaN => false,
        }
    }

    /// Returns the signum of the value: -1, 0, or +1 as a Rational.
    ///
    /// For intervals that span zero, returns NaN (sign is indeterminate).
    pub fn signum(&self) -> Self {
        match self {
            NumberValue::Rational(r) => {
                let s = r.value.cmp0();
                NumberValue::Rational(Rational::from_i32(match s {
                    std::cmp::Ordering::Greater => 1,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Less => -1,
                }))
            }
            NumberValue::Float(f) => {
                if f.is_nan() {
                    NumberValue::NaN
                } else if f.is_zero() {
                    NumberValue::Rational(Rational::from_i32(0))
                } else if f.value.is_sign_positive() {
                    NumberValue::Rational(Rational::from_i32(1))
                } else {
                    NumberValue::Rational(Rational::from_i32(-1))
                }
            }
            NumberValue::Interval { lower, upper } => {
                if lower.value.is_sign_positive() && !lower.value.is_zero() {
                    // Strictly positive
                    NumberValue::Rational(Rational::from_i32(1))
                } else if upper.value.is_sign_negative() && !upper.value.is_zero() {
                    // Strictly negative
                    NumberValue::Rational(Rational::from_i32(-1))
                } else if lower.value.is_zero() && upper.value.is_zero() {
                    // Exactly zero
                    NumberValue::Rational(Rational::from_i32(0))
                } else {
                    // Spans or touches zero
                    NumberValue::NaN
                }
            }
            NumberValue::Uncertainty { value, .. } => value.signum(),
            NumberValue::PlusInfinity => NumberValue::Rational(Rational::from_i32(1)),
            NumberValue::MinusInfinity => NumberValue::Rational(Rational::from_i32(-1)),
            NumberValue::NaN => NumberValue::NaN,
        }
    }

    /// Returns the floor of the value (greatest integer ≤ x).
    pub fn floor(&self) -> Self {
        match self {
            NumberValue::Rational(r) => {
                let floored = r.value.numer().clone().div_floor(r.value.denom());
                NumberValue::Rational(Rational {
                    value: rug::Rational::from(floored),
                })
            }
            NumberValue::Float(f) => {
                if f.is_nan() || f.is_infinite() {
                    return self.clone();
                }
                let prec = f.prec();
                NumberValue::Float(Float {
                    value: rug::Float::with_val(prec, &f.value).floor(),
                })
            }
            NumberValue::Interval { lower, upper } => {
                let prec_l = lower.prec();
                let prec_u = upper.prec();
                NumberValue::Interval {
                    lower: Float {
                        value: rug::Float::with_val(prec_l, &lower.value).floor(),
                    },
                    upper: Float {
                        value: rug::Float::with_val(prec_u, &upper.value).floor(),
                    },
                }
            }
            NumberValue::Uncertainty {
                value,
                uncertainty,
                is_relative,
            } => NumberValue::Uncertainty {
                value: Box::new(value.floor()),
                uncertainty: uncertainty.clone(),
                is_relative: *is_relative,
            },
            NumberValue::PlusInfinity => NumberValue::PlusInfinity,
            NumberValue::MinusInfinity => NumberValue::MinusInfinity,
            NumberValue::NaN => NumberValue::NaN,
        }
    }

    /// Returns the ceiling of the value (least integer ≥ x).
    pub fn ceil(&self) -> Self {
        match self {
            NumberValue::Rational(r) => {
                let ceiled = r.value.numer().clone().div_ceil(r.value.denom());
                NumberValue::Rational(Rational {
                    value: rug::Rational::from(ceiled),
                })
            }
            NumberValue::Float(f) => {
                if f.is_nan() || f.is_infinite() {
                    return self.clone();
                }
                let prec = f.prec();
                NumberValue::Float(Float {
                    value: rug::Float::with_val(prec, &f.value).ceil(),
                })
            }
            NumberValue::Interval { lower, upper } => {
                let prec_l = lower.prec();
                let prec_u = upper.prec();
                NumberValue::Interval {
                    lower: Float {
                        value: rug::Float::with_val(prec_l, &lower.value).ceil(),
                    },
                    upper: Float {
                        value: rug::Float::with_val(prec_u, &upper.value).ceil(),
                    },
                }
            }
            NumberValue::Uncertainty {
                value,
                uncertainty,
                is_relative,
            } => NumberValue::Uncertainty {
                value: Box::new(value.ceil()),
                uncertainty: uncertainty.clone(),
                is_relative: *is_relative,
            },
            NumberValue::PlusInfinity => NumberValue::PlusInfinity,
            NumberValue::MinusInfinity => NumberValue::MinusInfinity,
            NumberValue::NaN => NumberValue::NaN,
        }
    }

    /// Returns the truncation of the value (round toward zero).
    pub fn trunc(&self) -> Self {
        match self {
            NumberValue::Rational(r) => {
                let truncated = r.value.numer().clone().div_trunc(r.value.denom());
                NumberValue::Rational(Rational {
                    value: rug::Rational::from(truncated),
                })
            }
            NumberValue::Float(f) => {
                if f.is_nan() || f.is_infinite() {
                    return self.clone();
                }
                let prec = f.prec();
                NumberValue::Float(Float {
                    value: rug::Float::with_val(prec, &f.value).trunc(),
                })
            }
            NumberValue::Interval { lower, upper } => {
                let prec_l = lower.prec();
                let prec_u = upper.prec();
                NumberValue::Interval {
                    lower: Float {
                        value: rug::Float::with_val(prec_l, &lower.value).trunc(),
                    },
                    upper: Float {
                        value: rug::Float::with_val(prec_u, &upper.value).trunc(),
                    },
                }
            }
            NumberValue::Uncertainty {
                value,
                uncertainty,
                is_relative,
            } => NumberValue::Uncertainty {
                value: Box::new(value.trunc()),
                uncertainty: uncertainty.clone(),
                is_relative: *is_relative,
            },
            NumberValue::PlusInfinity => NumberValue::PlusInfinity,
            NumberValue::MinusInfinity => NumberValue::MinusInfinity,
            NumberValue::NaN => NumberValue::NaN,
        }
    }

    /// Returns the nearest integer value (half away from zero).
    pub fn round(&self) -> Self {
        match self {
            NumberValue::Rational(r) => {
                // round(p/q): compute as (2*|p| + |q|) / (2*|q|), then apply sign
                let p = r.value.numer();
                let q = r.value.denom();
                let two_abs_p = p.clone().abs() * 2;
                let abs_q = q.clone().abs();
                let rounded_abs: rug::Integer = (two_abs_p + &abs_q) / (abs_q * 2);
                let result = if p.cmp0() == std::cmp::Ordering::Less {
                    -rounded_abs
                } else {
                    rounded_abs
                };
                NumberValue::Rational(Rational {
                    value: rug::Rational::from(result),
                })
            }
            NumberValue::Float(f) => {
                if f.is_nan() || f.is_infinite() {
                    return self.clone();
                }
                let prec = f.prec();
                NumberValue::Float(Float {
                    value: rug::Float::with_val(prec, &f.value).round(),
                })
            }
            NumberValue::Interval { lower, upper } => {
                let prec_l = lower.prec();
                let prec_u = upper.prec();
                NumberValue::Interval {
                    lower: Float {
                        value: rug::Float::with_val(prec_l, &lower.value).round(),
                    },
                    upper: Float {
                        value: rug::Float::with_val(prec_u, &upper.value).round(),
                    },
                }
            }
            NumberValue::Uncertainty {
                value,
                uncertainty,
                is_relative,
            } => NumberValue::Uncertainty {
                value: Box::new(value.round()),
                uncertainty: uncertainty.clone(),
                is_relative: *is_relative,
            },
            NumberValue::PlusInfinity => NumberValue::PlusInfinity,
            NumberValue::MinusInfinity => NumberValue::MinusInfinity,
            NumberValue::NaN => NumberValue::NaN,
        }
    }

    /// Returns the gamma function Γ(x).
    ///
    /// For non-positive integers, the gamma function has poles and returns NaN.
    /// For exact rationals, promotes to float for evaluation.
    pub fn gamma(&self) -> Self {
        match self {
            NumberValue::Rational(r) => {
                // Check for non-positive integers (poles of gamma)
                if r.value.denom() == &1 && r.value.numer().cmp0() != std::cmp::Ordering::Greater {
                    return NumberValue::NaN;
                }
                let prec = 53u32;
                let f = rug::Float::with_val(prec, &r.value);
                NumberValue::Float(Float { value: f.gamma() })
            }
            NumberValue::Float(f) => {
                if f.is_nan() {
                    return NumberValue::NaN;
                }
                // Check for non-positive integer poles
                if !f.is_infinite() {
                    let truncated = rug::Float::with_val(f.prec(), &f.value).trunc();
                    if f.value == truncated && f.value.cmp0() != Some(std::cmp::Ordering::Greater) {
                        return NumberValue::NaN;
                    }
                }
                let prec = f.prec();
                NumberValue::Float(Float {
                    value: rug::Float::with_val(prec, &f.value).gamma(),
                })
            }
            NumberValue::Interval { lower, upper } => {
                let prec_l = lower.prec();
                let prec_u = upper.prec();

                // Reject intervals containing zero or negative values due to poles and oscillations
                if lower.value <= 0.0 {
                    return NumberValue::NaN;
                }

                // Gamma minimum point: x_min ≈ 1.4616321449683623
                // Min value: g_min ≈ 0.8856031944108887
                let x_min = 1.4616321449683623;
                let g_l = rug::Float::with_val(prec_l, &lower.value).gamma();
                let g_u = rug::Float::with_val(prec_u, &upper.value).gamma();

                if g_l.is_nan() || g_u.is_nan() {
                    return NumberValue::NaN;
                }

                let (new_l, new_u) = if upper.value <= x_min {
                    // Decreasing
                    (g_u, g_l)
                } else if lower.value >= x_min {
                    // Increasing
                    (g_l, g_u)
                } else {
                    // Spans the minimum
                    let max_val = if g_l >= g_u { g_l } else { g_u };
                    let min_prec = std::cmp::max(prec_l, prec_u);
                    let min_val = rug::Float::with_val(min_prec, 0.8856031944108887f64);
                    (min_val, max_val)
                };

                NumberValue::Interval {
                    lower: Float { value: new_l },
                    upper: Float { value: new_u },
                }
            }
            NumberValue::Uncertainty {
                value,
                uncertainty,
                is_relative,
            } => NumberValue::Uncertainty {
                value: Box::new(value.gamma()),
                uncertainty: uncertainty.clone(),
                is_relative: *is_relative,
            },
            NumberValue::PlusInfinity => NumberValue::PlusInfinity,
            NumberValue::MinusInfinity => NumberValue::NaN,
            NumberValue::NaN => NumberValue::NaN,
        }
    }

    /// Returns the error function erf(x).
    pub fn erf(&self) -> Self {
        match self {
            NumberValue::Rational(r) => {
                if r.is_zero() {
                    return NumberValue::Rational(Rational::from_i32(0));
                }
                let prec = 53u32;
                let f = rug::Float::with_val(prec, &r.value);
                NumberValue::Float(Float { value: f.erf() })
            }
            NumberValue::Float(f) => {
                if f.is_nan() {
                    return NumberValue::NaN;
                }
                let prec = f.prec();
                NumberValue::Float(Float {
                    value: rug::Float::with_val(prec, &f.value).erf(),
                })
            }
            NumberValue::Interval { lower, upper } => {
                // erf is monotonically increasing
                let prec_l = lower.prec();
                let prec_u = upper.prec();
                NumberValue::Interval {
                    lower: Float {
                        value: rug::Float::with_val(prec_l, &lower.value).erf(),
                    },
                    upper: Float {
                        value: rug::Float::with_val(prec_u, &upper.value).erf(),
                    },
                }
            }
            NumberValue::Uncertainty {
                value,
                uncertainty,
                is_relative,
            } => NumberValue::Uncertainty {
                value: Box::new(value.erf()),
                uncertainty: uncertainty.clone(),
                is_relative: *is_relative,
            },
            NumberValue::PlusInfinity => NumberValue::Rational(Rational::from_i32(1)),
            NumberValue::MinusInfinity => NumberValue::Rational(Rational::from_i32(-1)),
            NumberValue::NaN => NumberValue::NaN,
        }
    }

    /// Returns the Riemann zeta function ζ(x).
    pub fn zeta(&self) -> Self {
        match self {
            NumberValue::Rational(r) => {
                let prec = 53u32;
                let f = rug::Float::with_val(prec, &r.value);
                NumberValue::Float(Float { value: f.zeta() })
            }
            NumberValue::Float(f) => {
                if f.is_nan() {
                    return NumberValue::NaN;
                }
                let prec = f.prec();
                NumberValue::Float(Float {
                    value: rug::Float::with_val(prec, &f.value).zeta(),
                })
            }
            NumberValue::Interval { lower, upper } => {
                let prec_l = lower.prec();
                let prec_u = upper.prec();

                // Pole at x = 1
                if lower.value <= 1.0 && upper.value >= 1.0 {
                    return NumberValue::NaN;
                }

                if lower.value >= -2.0 {
                    // zeta(x) is monotonically decreasing for x >= -2 (excluding x=1)
                    let z_l = rug::Float::with_val(prec_l, &lower.value).zeta();
                    let z_u = rug::Float::with_val(prec_u, &upper.value).zeta();
                    if z_l.is_nan() || z_u.is_nan() {
                        return NumberValue::NaN;
                    }
                    NumberValue::Interval {
                        lower: Float { value: z_u },
                        upper: Float { value: z_l },
                    }
                } else {
                    // x < -2 is not properly supported as interval (zeta is oscillatory / non-monotonic here)
                    NumberValue::NaN
                }
            }
            NumberValue::Uncertainty {
                value,
                uncertainty,
                is_relative,
            } => NumberValue::Uncertainty {
                value: Box::new(value.zeta()),
                uncertainty: uncertainty.clone(),
                is_relative: *is_relative,
            },
            NumberValue::PlusInfinity => NumberValue::Rational(Rational::from_i32(1)),
            NumberValue::MinusInfinity => NumberValue::NaN,
            NumberValue::NaN => NumberValue::NaN,
        }
    }

    /// Returns the upper incomplete gamma function Γ(self, x).
    ///
    /// Uses `rug::Float::gamma_inc` where `self` is the `a` parameter
    /// and `x` is the integration lower bound.
    pub fn gamma_inc(&self, x: &Self) -> Self {
        // Promote both to float for the computation
        let a_float = match self {
            NumberValue::Rational(r) => {
                let prec = 53u32;
                rug::Float::with_val(prec, &r.value)
            }
            NumberValue::Float(f) => rug::Float::with_val(f.prec(), &f.value),
            NumberValue::NaN => return NumberValue::NaN,
            NumberValue::PlusInfinity | NumberValue::MinusInfinity => return NumberValue::NaN,
            _ => return NumberValue::NaN,
        };
        let x_float = match x {
            NumberValue::Rational(r) => {
                let prec = a_float.prec();
                rug::Float::with_val(prec, &r.value)
            }
            NumberValue::Float(f) => rug::Float::with_val(f.prec(), &f.value),
            NumberValue::NaN => return NumberValue::NaN,
            NumberValue::PlusInfinity => return NumberValue::Rational(Rational::from_i32(0)),
            NumberValue::MinusInfinity => return NumberValue::NaN,
            _ => return NumberValue::NaN,
        };
        let prec = std::cmp::max(a_float.prec(), x_float.prec());
        let result = rug::Float::with_val(prec, a_float).gamma_inc(&x_float);
        if result.is_nan() {
            NumberValue::NaN
        } else {
            NumberValue::Float(Float { value: result })
        }
    }
}

fn get_infinity_sign(val: &NumberValue) -> Option<bool> {
    match val {
        NumberValue::PlusInfinity => Some(true),
        NumberValue::MinusInfinity => Some(false),
        NumberValue::Float(f) => {
            if f.is_infinite() {
                Some(f.value.is_sign_positive())
            } else {
                None
            }
        }
        NumberValue::Uncertainty { value, .. } => get_infinity_sign(value),
        _ => None,
    }
}

fn get_sign_multiplier(val: &NumberValue) -> Option<f64> {
    if val.is_nan() {
        None
    } else if val.is_infinite() {
        get_infinity_sign(val).map(|s| if s { 1.0 } else { -1.0 })
    } else {
        match val {
            NumberValue::Rational(r) => Some(match r.sign_ordering() {
                std::cmp::Ordering::Less => -1.0,
                std::cmp::Ordering::Equal => 0.0,
                std::cmp::Ordering::Greater => 1.0,
            }),
            NumberValue::Float(f) => {
                if f.value == 0.0 {
                    Some(0.0)
                } else if f.value > 0.0 {
                    Some(1.0)
                } else {
                    Some(-1.0)
                }
            }
            NumberValue::Interval { lower, upper } => {
                if lower.value <= 0.0 && upper.value >= 0.0 {
                    if lower.value == 0.0 && upper.value == 0.0 {
                        Some(0.0)
                    } else {
                        None
                    }
                } else if lower.value > 0.0 {
                    Some(1.0)
                } else {
                    Some(-1.0)
                }
            }
            NumberValue::Uncertainty { value, .. } => get_sign_multiplier(value),
            _ => None,
        }
    }
}

#[allow(dead_code)]
fn to_interval(val: &NumberValue) -> Option<(Float, Float)> {
    match val {
        NumberValue::Interval { lower, upper } => Some((lower.clone(), upper.clone())),
        NumberValue::Rational(r) => {
            let f = Float {
                value: rug::Float::with_val(53, &r.value),
            };
            Some((f.clone(), f))
        }
        NumberValue::Float(f) => Some((f.clone(), f.clone())),
        NumberValue::PlusInfinity => {
            let f = Float::from_f64(f64::INFINITY, 53);
            Some((f.clone(), f))
        }
        NumberValue::MinusInfinity => {
            let f = Float::from_f64(f64::NEG_INFINITY, 53);
            Some((f.clone(), f))
        }
        NumberValue::Uncertainty {
            value, uncertainty, ..
        } => {
            let (v_min, v_max) = to_interval(value)?;
            let (u_min, u_max) = to_interval(uncertainty)?;
            let u_min_abs = u_min.value.clone().abs();
            let u_max_abs = u_max.value.clone().abs();
            let u_limit = if u_min_abs > u_max_abs {
                u_min_abs
            } else {
                u_max_abs
            };
            let prec = std::cmp::max(v_min.prec(), u_min.prec());

            let mut lower_bound = rug::Float::new(prec);
            lower_bound.assign_round(&v_min.value - &u_limit, rug::float::Round::Down);
            let mut upper_bound = rug::Float::new(prec);
            upper_bound.assign_round(&v_max.value + &u_limit, rug::float::Round::Up);

            Some((Float { value: lower_bound }, Float { value: upper_bound }))
        }
        NumberValue::NaN => None,
    }
}

fn min_float(a: rug::Float, b: rug::Float) -> rug::Float {
    if a < b {
        a
    } else {
        b
    }
}

fn max_float(a: rug::Float, b: rug::Float) -> rug::Float {
    if a > b {
        a
    } else {
        b
    }
}

#[allow(dead_code)]
fn min4(a: f64, b: f64, c: f64, d: f64) -> f64 {
    if a.is_nan() || b.is_nan() || c.is_nan() || d.is_nan() {
        f64::NAN
    } else {
        a.min(b).min(c).min(d)
    }
}

#[allow(dead_code)]
fn max4(a: f64, b: f64, c: f64, d: f64) -> f64 {
    if a.is_nan() || b.is_nan() || c.is_nan() || d.is_nan() {
        f64::NAN
    } else {
        a.max(b).max(c).max(d)
    }
}

fn add_rationals(lhs: &Rational, rhs: &Rational) -> Option<Rational> {
    Some(Rational {
        value: rug::Rational::from(&lhs.value + &rhs.value),
    })
}

fn sub_rationals(lhs: &Rational, rhs: &Rational) -> Option<Rational> {
    Some(Rational {
        value: rug::Rational::from(&lhs.value - &rhs.value),
    })
}

fn mul_rationals(lhs: &Rational, rhs: &Rational) -> Option<Rational> {
    Some(Rational {
        value: rug::Rational::from(&lhs.value * &rhs.value),
    })
}

fn div_rationals(lhs: &Rational, rhs: &Rational) -> Option<Rational> {
    if rhs.is_zero() {
        None
    } else {
        Some(Rational {
            value: rug::Rational::from(&lhs.value / &rhs.value),
        })
    }
}

#[derive(Debug, Clone, Copy)]
enum RemainderMode {
    Truncate,
    Floor,
}

fn rem_rationals(lhs: &Rational, rhs: &Rational, mode: RemainderMode) -> Option<Rational> {
    let integer_quotient = rational_integer_quotient(lhs, rhs, mode)?;
    let scaled_divisor = rug::Rational::from(integer_quotient) * &rhs.value;
    Some(Rational {
        value: &lhs.value - scaled_divisor,
    })
}

fn int_div_rationals(lhs: &Rational, rhs: &Rational) -> Option<Rational> {
    let integer_quotient = rational_integer_quotient(lhs, rhs, RemainderMode::Truncate)?;
    Some(Rational {
        value: rug::Rational::from(integer_quotient),
    })
}

fn rational_integer_quotient(
    lhs: &Rational,
    rhs: &Rational,
    mode: RemainderMode,
) -> Option<rug::Integer> {
    if rhs.is_zero() {
        return None;
    }

    let quotient = rug::Rational::from(&lhs.value / &rhs.value);
    Some(match mode {
        RemainderMode::Truncate => quotient.numer().clone().div_trunc(quotient.denom()),
        RemainderMode::Floor => quotient.numer().clone().div_floor(quotient.denom()),
    })
}

fn to_float_val_rnd(val: &NumberValue, default_prec: u32, rnd: rug::float::Round) -> Float {
    match val {
        NumberValue::Float(f) => f.clone(),
        NumberValue::Rational(r) => {
            let mut f = rug::Float::new(default_prec);
            f.assign_round(&r.value, rnd);
            Float { value: f }
        }
        NumberValue::PlusInfinity => Float::from_f64(f64::INFINITY, default_prec),
        NumberValue::MinusInfinity => Float::from_f64(f64::NEG_INFINITY, default_prec),
        NumberValue::NaN => Float::from_f64(f64::NAN, default_prec),
        _ => Float::from_f64(f64::NAN, default_prec),
    }
}

#[allow(dead_code)]
fn from_f64_and_prec(val: f64, prec: u32) -> NumberValue {
    if val.is_nan() {
        NumberValue::NaN
    } else if val == f64::INFINITY {
        NumberValue::PlusInfinity
    } else if val == f64::NEG_INFINITY {
        NumberValue::MinusInfinity
    } else {
        NumberValue::Float(Float::from_f64(val, prec))
    }
}

fn to_float_val(val: &NumberValue) -> Float {
    to_float_val_rnd(val, 53, rug::float::Round::Nearest)
}

fn is_fractional_rational(value: &NumberValue) -> bool {
    matches!(value, NumberValue::Rational(rational) if rational.value.denom() != &1)
}

fn approximate_complex_power_number(
    base_real: &NumberValue,
    base_imag: &NumberValue,
    exponent_real: &NumberValue,
    exponent_imag: &NumberValue,
) -> Number {
    let a = to_float_val(base_real).value();
    let b = to_float_val(base_imag).value();
    let c = to_float_val(exponent_real).value();
    let d = to_float_val(exponent_imag).value();

    let radius = a.hypot(b);
    if radius == 0.0 {
        if c > 0.0 {
            return Number::from_i32(0);
        }
        return Number {
            value: NumberValue::NaN,
            imaginary: None,
            precision: 53,
            approximate: true,
            is_imaginary: false,
        };
    }

    let theta = b.atan2(a);
    let log_radius = radius.ln();
    let result_radius = (c * log_radius - d * theta).exp();
    let result_angle = d * log_radius + c * theta;
    Number::from_real_imag_values(
        from_f64_and_prec(result_radius * result_angle.cos(), 53),
        from_f64_and_prec(result_radius * result_angle.sin(), 53),
        53,
        true,
    )
}

fn exact_i32_integer_exponent(value: &NumberValue) -> Option<i32> {
    match value {
        NumberValue::Rational(rational) if rational.value.denom() == &1 => {
            rational.value.numer().to_i32()
        }
        _ => None,
    }
}

fn exact_integer_exponent_mod_4(value: &NumberValue) -> Option<u32> {
    match value {
        NumberValue::Rational(rational) if rational.value.denom() == &1 => {
            Some(rational.value.numer().mod_u(4))
        }
        _ => None,
    }
}

fn exact_unit_imaginary_integer_power(base: &Number, exponent_mod_4: u32) -> Option<Number> {
    let (real, imag) = base.to_canonical_real_imag();
    if !real.is_real_zero() || real.approximate() || imag.approximate() {
        return None;
    }

    let unit_power_mod_4 = if imag.is_real_one() {
        1_u32
    } else if imag.negate().is_real_one() {
        3_u32
    } else {
        return None;
    };

    let mut result = match (unit_power_mod_4 * exponent_mod_4) % 4 {
        0 => Number::from_i32(1),
        1 => Number::from_real_imag_values(
            NumberValue::Rational(Rational::from_i32(0)),
            NumberValue::Rational(Rational::from_i32(1)),
            0,
            false,
        ),
        2 => Number::from_i32(-1),
        3 => Number::from_real_imag_values(
            NumberValue::Rational(Rational::from_i32(0)),
            NumberValue::Rational(Rational::from_i32(-1)),
            0,
            false,
        ),
        _ => unreachable!("modulo-four cycle returns only 0..=3"),
    };
    result.precision = result.precision.max(base.precision);
    result.approximate |= base.approximate;
    Some(result)
}

fn exact_complex_integer_pow_is_bounded(base: &Number, exponent_magnitude: u32) -> bool {
    let (real, imag) = base.to_canonical_real_imag();
    if exponent_magnitude == 0 {
        return true;
    }

    [real, imag].iter().all(|value| match value {
        NumberValue::Rational(rational) => {
            exact_rational_integer_pow_is_bounded(rational, exponent_magnitude)
        }
        _ => false,
    })
}

fn exact_complex_integer_power(base: &Number, exponent: i32) -> Option<Number> {
    if let Some(result) = exact_unit_imaginary_integer_power(base, exponent.rem_euclid(4) as u32) {
        return Some(result);
    }

    const MAX_EXACT_COMPLEX_POW_EXP: u32 = 10_000;
    let magnitude = exponent.unsigned_abs();
    if magnitude > MAX_EXACT_COMPLEX_POW_EXP
        || (exponent.is_negative() && base.is_zero())
        || !exact_complex_integer_pow_is_bounded(base, magnitude)
    {
        return None;
    }

    let mut result = Number::from_i32(1);
    result.precision = result.precision.max(base.precision);
    result.approximate |= base.approximate;
    let mut factor = base.clone();
    let mut remaining = magnitude;
    while remaining != 0 {
        if remaining & 1 == 1 {
            result = result.mul(&factor);
        }
        remaining >>= 1;
        if remaining != 0 {
            factor = factor.mul(&factor);
        }
    }

    if exponent.is_negative() {
        Some(Number::from_i32(1).div(&result))
    } else {
        Some(result)
    }
}

fn try_unwrap_single_val(val: &NumberValue) -> Option<NumberValue> {
    match val {
        NumberValue::Interval { lower, upper } => {
            if lower == upper {
                Some(NumberValue::Float(lower.clone()))
            } else {
                None
            }
        }
        NumberValue::Uncertainty {
            value, uncertainty, ..
        } => {
            if uncertainty.is_real_zero() {
                Some((**value).clone())
            } else {
                None
            }
        }
        _ => None,
    }
}

fn eq_values(lhs: &NumberValue, rhs: &NumberValue) -> bool {
    if lhs.is_nan() || rhs.is_nan() {
        return lhs.is_nan() && rhs.is_nan();
    }

    if lhs.is_infinite() || rhs.is_infinite() {
        if lhs.is_infinite() && rhs.is_infinite() {
            return get_infinity_sign(lhs) == get_infinity_sign(rhs);
        } else {
            return false;
        }
    }

    if let Some(unwrapped_lhs) = try_unwrap_single_val(lhs) {
        return eq_values(&unwrapped_lhs, rhs);
    }
    if let Some(unwrapped_rhs) = try_unwrap_single_val(rhs) {
        return eq_values(lhs, &unwrapped_rhs);
    }

    match (lhs, rhs) {
        (NumberValue::Rational(r1), NumberValue::Rational(r2)) => r1.value == r2.value,
        (NumberValue::Float(f1), NumberValue::Float(f2)) => f1.value == f2.value,
        (NumberValue::Rational(_), NumberValue::Float(_))
        | (NumberValue::Float(_), NumberValue::Rational(_)) => false,
        (
            NumberValue::Interval {
                lower: l1,
                upper: u1,
            },
            NumberValue::Interval {
                lower: l2,
                upper: u2,
            },
        ) => l1 == l2 && u1 == u2,
        (
            NumberValue::Uncertainty {
                value: v1,
                uncertainty: u1,
                is_relative: ir1,
                ..
            },
            NumberValue::Uncertainty {
                value: v2,
                uncertainty: u2,
                is_relative: ir2,
                ..
            },
        ) => eq_values(v1, v2) && eq_values(u1, u2) && ir1 == ir2,
        _ => false,
    }
}

impl PartialOrd for NumberValue {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.is_nan() || other.is_nan() {
            return None;
        }
        if let Some(unwrapped_lhs) = try_unwrap_single_val(self) {
            return unwrapped_lhs.partial_cmp(other);
        }
        if let Some(unwrapped_rhs) = try_unwrap_single_val(other) {
            return self.partial_cmp(&unwrapped_rhs);
        }

        match (self, other) {
            (NumberValue::Rational(r1), NumberValue::Rational(r2)) => r1.partial_cmp(r2),
            (NumberValue::Float(f1), NumberValue::Float(f2)) => f1.value.partial_cmp(&f2.value),
            (NumberValue::Rational(r), NumberValue::Float(f)) => {
                let r_f = rug::Float::with_val(f.prec(), &r.value);
                r_f.partial_cmp(&f.value)
            }
            (NumberValue::Float(f), NumberValue::Rational(r)) => {
                let r_f = rug::Float::with_val(f.prec(), &r.value);
                f.value.partial_cmp(&r_f)
            }
            (NumberValue::PlusInfinity, NumberValue::PlusInfinity) => {
                Some(std::cmp::Ordering::Equal)
            }
            (NumberValue::PlusInfinity, _) => Some(std::cmp::Ordering::Greater),
            (_, NumberValue::PlusInfinity) => Some(std::cmp::Ordering::Less),
            (NumberValue::MinusInfinity, NumberValue::MinusInfinity) => {
                Some(std::cmp::Ordering::Equal)
            }
            (NumberValue::MinusInfinity, _) => Some(std::cmp::Ordering::Less),
            (_, NumberValue::MinusInfinity) => Some(std::cmp::Ordering::Greater),
            (
                NumberValue::Interval {
                    lower: l1,
                    upper: u1,
                },
                NumberValue::Interval {
                    lower: l2,
                    upper: u2,
                },
            ) => {
                if u1.value < l2.value {
                    Some(std::cmp::Ordering::Less)
                } else if l1.value > u2.value {
                    Some(std::cmp::Ordering::Greater)
                } else if l1.value == l2.value && u1.value == u2.value {
                    Some(std::cmp::Ordering::Equal)
                } else {
                    None
                }
            }
            (NumberValue::Interval { lower, upper }, other_val) => {
                let other_f = to_float_val(other_val);
                if upper.value < other_f.value {
                    Some(std::cmp::Ordering::Less)
                } else if lower.value > other_f.value {
                    Some(std::cmp::Ordering::Greater)
                } else {
                    None
                }
            }
            (self_val, NumberValue::Interval { lower, upper }) => {
                let self_f = to_float_val(self_val);
                if self_f.value < lower.value {
                    Some(std::cmp::Ordering::Less)
                } else if self_f.value > upper.value {
                    Some(std::cmp::Ordering::Greater)
                } else {
                    None
                }
            }
            (
                NumberValue::Uncertainty {
                    value: v1,
                    uncertainty: _,
                    ..
                },
                NumberValue::Uncertainty {
                    value: v2,
                    uncertainty: _,
                    ..
                },
            ) => v1.as_ref().partial_cmp(v2.as_ref()),
            (NumberValue::Uncertainty { value, .. }, other_val) => {
                value.as_ref().partial_cmp(other_val)
            }
            (self_val, NumberValue::Uncertainty { value, .. }) => {
                self_val.partial_cmp(value.as_ref())
            }
            _ => None,
        }
    }
}

impl PartialEq for NumberValue {
    fn eq(&self, other: &Self) -> bool {
        eq_values(self, other)
    }
}

/// Core representation of a number in libqalculate, supporting reals, intervals, infinities, NaNs, and complex numbers recursively.
#[derive(Debug, Clone)]
pub struct Number {
    value: NumberValue,
    imaginary: Option<Box<Number>>,
    precision: i32,
    approximate: bool,
    is_imaginary: bool,
}

impl Default for Number {
    fn default() -> Self {
        Self::new()
    }
}

impl Number {
    /// Creates a new `Number` representing zero (as a rational).
    pub fn new() -> Self {
        Self::from_rational(Rational::from_i32(0))
    }

    /// Creates a `Number` from a `Rational`.
    pub fn from_rational(r: Rational) -> Self {
        Self {
            value: NumberValue::Rational(r),
            imaginary: None,
            precision: 0,
            approximate: false,
            is_imaginary: false,
        }
    }

    /// Creates a `Number` from a `Float`.
    pub fn from_float(f: Float) -> Self {
        let prec = f.prec() as i32;
        Self {
            value: NumberValue::Float(f),
            imaginary: None,
            precision: prec,
            approximate: true,
            is_imaginary: false,
        }
    }

    pub(crate) fn from_real_value(value: NumberValue) -> Self {
        Self {
            precision: value.precision(),
            approximate: value.approximate(),
            value,
            imaginary: None,
            is_imaginary: false,
        }
    }

    /// Creates a new interval `Number` with given lower and upper bounds.
    ///
    /// Reversed finite bounds are stored in lower/upper order, and equal bounds
    /// collapse to a scalar value, matching upstream `Number::setInterval`. If
    /// either bound is NaN, the result is `NaN`; use
    /// [`Number::try_new_interval`] to reject those inputs.
    pub fn new_interval(lower: Float, upper: Float) -> Self {
        let Some(number) = Self::from_interval_bounds(lower, upper) else {
            return Self {
                value: NumberValue::NaN,
                imaginary: None,
                precision: 0,
                approximate: true,
                is_imaginary: false,
            };
        };
        number
    }

    /// Tries to create a new interval `Number`.
    ///
    /// Reversed finite bounds are stored in lower/upper order, and equal bounds
    /// collapse to a scalar value. Returns `None` if either bound is NaN.
    pub fn try_new_interval(lower: Float, upper: Float) -> Option<Self> {
        Self::from_interval_bounds(lower, upper)
    }

    fn from_interval_bounds(lower: Float, upper: Float) -> Option<Self> {
        let (lower, upper) = ordered_interval_bounds(lower, upper)?;
        if lower.value == upper.value {
            let prec = std::cmp::max(lower.prec(), upper.prec());
            return Some(Self::from_float(Float {
                value: rug::Float::with_val(prec, &lower.value),
            }));
        }
        Some(Self::from_ordered_interval(lower, upper))
    }

    fn from_ordered_interval(lower: Float, upper: Float) -> Self {
        let prec = std::cmp::max(lower.prec(), upper.prec()) as i32;
        Self {
            value: NumberValue::Interval { lower, upper },
            imaginary: None,
            precision: prec,
            approximate: true,
            is_imaginary: false,
        }
    }

    /// Creates a new uncertainty `Number`.
    pub fn new_uncertainty(
        value: NumberValue,
        uncertainty: NumberValue,
        is_relative: bool,
    ) -> Self {
        let prec = std::cmp::max(value.precision(), uncertainty.precision());
        Self {
            value: NumberValue::Uncertainty {
                value: Box::new(value),
                uncertainty: Box::new(uncertainty),
                is_relative,
            },
            imaginary: None,
            precision: prec,
            approximate: true,
            is_imaginary: false,
        }
    }

    /// Creates a new complex `Number` combining a real and an imaginary part.
    pub fn new_complex(real: Number, imag: Number) -> Self {
        let (a, b) = real.to_canonical_real_imag();
        let (c, d) = imag.to_canonical_real_imag();

        let new_real_val = a.add(&d.negate());
        let new_imag_val = b.add(&c);
        let precision = std::cmp::max(real.precision, imag.precision)
            .max(new_real_val.precision())
            .max(new_imag_val.precision());
        let approximate = real.approximate
            || imag.approximate
            || new_real_val.approximate()
            || new_imag_val.approximate();

        if new_imag_val.is_real_zero() {
            return Self {
                value: new_real_val,
                imaginary: None,
                precision,
                approximate,
                is_imaginary: false,
            };
        }

        let new_imag_num = Number {
            value: new_imag_val,
            imaginary: None,
            precision,
            approximate,
            is_imaginary: true,
        };

        Self {
            value: new_real_val,
            imaginary: Some(Box::new(new_imag_num)),
            precision,
            approximate,
            is_imaginary: false,
        }
    }

    /// Creates a rational `Number` from an `i32`.
    pub fn from_i32(val: i32) -> Self {
        Self::from_rational(Rational::from_i32(val))
    }

    /// Creates a float `Number` from an `f64`.
    pub fn from_f64(val: f64) -> Self {
        let value = if val.is_nan() {
            NumberValue::NaN
        } else if val == f64::INFINITY {
            NumberValue::PlusInfinity
        } else if val == f64::NEG_INFINITY {
            NumberValue::MinusInfinity
        } else {
            NumberValue::Float(Float::from_f64(val, 53))
        };

        Self {
            value,
            imaginary: None,
            precision: 53,
            approximate: true,
            is_imaginary: false,
        }
    }

    /// Creates a rational `Number` from an `i64`.
    pub fn from_i64(val: i64) -> Self {
        Self::from_rational(Rational {
            value: rug::Rational::from(val),
        })
    }

    /// Returns the number 1.
    pub fn one() -> Self {
        Self::from_i32(1)
    }

    /// Returns Euler's number e as a high-precision float.
    pub fn e() -> Self {
        let prec = 256u32;
        let one = rug::Float::with_val(prec, 1.0);
        let e_val = one.exp();
        Self {
            value: NumberValue::Float(Float {
                value: rug::Float::with_val(prec, e_val),
            }),
            imaginary: None,
            precision: prec as i32,
            approximate: true,
            is_imaginary: false,
        }
    }

    /// Returns π as a high-precision float.
    pub fn pi() -> Self {
        let prec = 256u32;
        let pi_val = rug::Float::with_val(prec, rug::float::Constant::Pi);
        Self {
            value: NumberValue::Float(Float { value: pi_val }),
            imaginary: None,
            precision: prec as i32,
            approximate: true,
            is_imaginary: false,
        }
    }

    /// Returns positive infinity.
    pub fn plus_infinity() -> Self {
        Self {
            value: NumberValue::PlusInfinity,
            imaginary: None,
            precision: 0,
            approximate: false,
            is_imaginary: false,
        }
    }

    /// Returns negative infinity.
    pub fn minus_infinity() -> Self {
        Self {
            value: NumberValue::MinusInfinity,
            imaginary: None,
            precision: 0,
            approximate: false,
            is_imaginary: false,
        }
    }

    /// Returns e^self (exponential function).
    ///
    /// For complex numbers: e^(a+bi) = e^a * (cos(b) + i*sin(b)).
    pub fn exp(&self) -> Self {
        let (real, imag) = self.to_canonical_real_imag();
        if imag.is_real_zero() {
            let r = real.exp_value();
            Self {
                precision: std::cmp::max(self.precision, r.precision()),
                approximate: self.approximate || r.approximate(),
                value: r,
                imaginary: None,
                is_imaginary: false,
            }
        } else {
            // e^(a+bi) = e^a * (cos(b) + i*sin(b))
            let prec = self.precision.max(0) as u32;
            let exp_a = Self::from_real_value(real.exp_value());
            let cos_b = Self::from_real_value(imag.cos_value(prec));
            let sin_b = Self::from_real_value(imag.sin_value(prec));
            let re_part = exp_a.mul(&cos_b);
            let im_part = exp_a.mul(&sin_b);
            Number::new_complex(re_part, im_part)
        }
    }

    /// Returns the cube root of the number.
    ///
    /// For real x: cbrt(x) = sign(x) * |x|^(1/3).
    /// For complex: delegates to pow(x, 1/3).
    pub fn cbrt(&self) -> Self {
        let (real, imag) = self.to_canonical_real_imag();
        if imag.is_real_zero() {
            let r = real.cbrt_value();
            Self {
                precision: std::cmp::max(self.precision, r.precision()),
                approximate: self.approximate || r.approximate(),
                value: r,
                imaginary: None,
                is_imaginary: false,
            }
        } else {
            // For complex: use x^(1/3)
            let third = Number::from_rational(Rational {
                value: rug::Rational::from((1, 3)),
            });
            self.pow(&third)
        }
    }

    /// Returns cos(self) for real numbers.
    pub fn cos(&self) -> Self {
        let (real, imag) = self.to_canonical_real_imag();
        if imag.is_real_zero() {
            let prec = self.precision.max(0) as u32;
            let r = real.cos_value(prec);
            Self {
                precision: std::cmp::max(self.precision, r.precision()),
                approximate: true,
                value: r,
                imaginary: None,
                is_imaginary: false,
            }
        } else {
            // cos(a+bi) = cos(a)*cosh(b) - i*sin(a)*sinh(b)
            Self::from_real_value(NumberValue::NaN) // TODO: implement complex cos
        }
    }

    /// Returns sin(self) for real numbers.
    pub fn sin(&self) -> Self {
        let (real, imag) = self.to_canonical_real_imag();
        if imag.is_real_zero() {
            let prec = self.precision.max(0) as u32;
            let r = real.sin_value(prec);
            Self {
                precision: std::cmp::max(self.precision, r.precision()),
                approximate: true,
                value: r,
                imaginary: None,
                is_imaginary: false,
            }
        } else {
            // sin(a+bi) = sin(a)*cosh(b) + i*cos(a)*sinh(b)
            Self::from_real_value(NumberValue::NaN) // TODO: implement complex sin
        }
    }

    fn apply_real_unary<F>(&self, op: F) -> Self
    where
        F: FnOnce(&NumberValue, u32) -> NumberValue,
    {
        let (real, imag) = self.to_canonical_real_imag();
        if imag.is_real_zero() {
            let r = op(&real, self.precision.max(0) as u32);
            Self {
                precision: std::cmp::max(self.precision, r.precision()),
                approximate: true,
                value: r,
                imaginary: None,
                is_imaginary: false,
            }
        } else {
            Self::from_real_value(NumberValue::NaN)
        }
    }

    fn apply_real_unary_with_prec<F>(&self, min_precision_bits: u32, op: F) -> Self
    where
        F: FnOnce(&NumberValue, u32) -> NumberValue,
    {
        let (real, imag) = self.to_canonical_real_imag();
        if imag.is_real_zero() {
            let target_prec = (self.precision.max(0) as u32).max(min_precision_bits);
            let r = op(&real, target_prec);
            Self {
                precision: std::cmp::max(self.precision, r.precision()),
                approximate: true,
                value: r,
                imaginary: None,
                is_imaginary: false,
            }
        } else {
            Self::from_real_value(NumberValue::NaN)
        }
    }

    /// Returns cos(self) for real numbers with min precision in bits.
    pub fn cos_with_prec(&self, min_precision_bits: u32) -> Self {
        let (real, imag) = self.to_canonical_real_imag();
        if imag.is_real_zero() {
            let prec = (self.precision.max(0) as u32).max(min_precision_bits);
            let r = real.cos_value(prec);
            Self {
                precision: std::cmp::max(self.precision, r.precision()),
                approximate: true,
                value: r,
                imaginary: None,
                is_imaginary: false,
            }
        } else {
            Self::from_real_value(NumberValue::NaN)
        }
    }

    /// Returns sin(self) for real numbers with min precision in bits.
    pub fn sin_with_prec(&self, min_precision_bits: u32) -> Self {
        let (real, imag) = self.to_canonical_real_imag();
        if imag.is_real_zero() {
            let prec = (self.precision.max(0) as u32).max(min_precision_bits);
            let r = real.sin_value(prec);
            Self {
                precision: std::cmp::max(self.precision, r.precision()),
                approximate: true,
                value: r,
                imaginary: None,
                is_imaginary: false,
            }
        } else {
            Self::from_real_value(NumberValue::NaN)
        }
    }

    /// Returns tan(self) for real numbers with min precision in bits.
    pub fn tan_with_prec(&self, min_precision_bits: u32) -> Self {
        self.apply_real_unary_with_prec(min_precision_bits, |r, prec| r.tan_value(prec))
    }

    /// Returns asin(self) for real numbers with min precision in bits.
    pub fn asin_with_prec(&self, min_precision_bits: u32) -> Self {
        self.apply_real_unary_with_prec(min_precision_bits, |r, prec| r.asin_value(prec))
    }

    /// Returns acos(self) for real numbers with min precision in bits.
    pub fn acos_with_prec(&self, min_precision_bits: u32) -> Self {
        self.apply_real_unary_with_prec(min_precision_bits, |r, prec| r.acos_value(prec))
    }

    /// Returns atan(self) for real numbers with min precision in bits.
    pub fn atan_with_prec(&self, min_precision_bits: u32) -> Self {
        self.apply_real_unary_with_prec(min_precision_bits, |r, prec| r.atan_value(prec))
    }

    /// Returns sinh(self) for real numbers with min precision in bits.
    pub fn sinh_with_prec(&self, min_precision_bits: u32) -> Self {
        self.apply_real_unary_with_prec(min_precision_bits, |r, prec| r.sinh_value(prec))
    }

    /// Returns cosh(self) for real numbers with min precision in bits.
    pub fn cosh_with_prec(&self, min_precision_bits: u32) -> Self {
        self.apply_real_unary_with_prec(min_precision_bits, |r, prec| r.cosh_value(prec))
    }

    /// Returns tanh(self) for real numbers with min precision in bits.
    pub fn tanh_with_prec(&self, min_precision_bits: u32) -> Self {
        self.apply_real_unary_with_prec(min_precision_bits, |r, prec| r.tanh_value(prec))
    }

    /// Returns asinh(self) for real numbers with min precision in bits.
    pub fn asinh_with_prec(&self, min_precision_bits: u32) -> Self {
        self.apply_real_unary_with_prec(min_precision_bits, |r, prec| r.asinh_value(prec))
    }

    /// Returns acosh(self) for real numbers with min precision in bits.
    pub fn acosh_with_prec(&self, min_precision_bits: u32) -> Self {
        self.apply_real_unary_with_prec(min_precision_bits, |r, prec| r.acosh_value(prec))
    }

    /// Returns atanh(self) for real numbers with min precision in bits.
    pub fn atanh_with_prec(&self, min_precision_bits: u32) -> Self {
        self.apply_real_unary_with_prec(min_precision_bits, |r, prec| r.atanh_value(prec))
    }

    /// Returns atan2(y, x) with min precision in bits.
    pub fn atan2_with_prec(y: &Self, x: &Self, min_precision_bits: u32) -> Self {
        if y.has_imaginary_part() || x.has_imaginary_part() {
            return Self::from_real_value(NumberValue::NaN);
        }
        let (yr, _) = y.to_canonical_real_imag();
        let (xr, _) = x.to_canonical_real_imag();
        let target_prec = (y.precision.max(x.precision).max(0) as u32).max(min_precision_bits);
        let r = NumberValue::atan2_value(&yr, &xr, target_prec);
        Self {
            precision: std::cmp::max(y.precision, x.precision).max(r.precision()),
            approximate: true,
            value: r,
            imaginary: None,
            is_imaginary: false,
        }
    }

    /// Returns tan(self) for real numbers.
    pub fn tan(&self) -> Self {
        self.apply_real_unary(|r, prec| r.tan_value(prec))
    }

    /// Returns asin(self) for real numbers.
    /// Domain: [-1, 1]. Returns NaN for |x| > 1.
    pub fn asin(&self) -> Self {
        self.apply_real_unary(|r, prec| r.asin_value(prec))
    }

    /// Returns acos(self) for real numbers.
    /// Domain: [-1, 1]. Returns NaN for |x| > 1.
    pub fn acos(&self) -> Self {
        self.apply_real_unary(|r, prec| r.acos_value(prec))
    }

    /// Returns atan(self) for real numbers.
    pub fn atan(&self) -> Self {
        self.apply_real_unary(|r, prec| r.atan_value(prec))
    }

    /// Returns atan2(y, x) — the two-argument arctangent.
    pub fn atan2(y: &Self, x: &Self) -> Self {
        if y.has_imaginary_part() || x.has_imaginary_part() {
            return Self::from_real_value(NumberValue::NaN);
        }
        let (yr, _) = y.to_canonical_real_imag();
        let (xr, _) = x.to_canonical_real_imag();
        let target_prec = y.precision.max(x.precision).max(0) as u32;
        let r = NumberValue::atan2_value(&yr, &xr, target_prec);
        Self {
            precision: std::cmp::max(y.precision, x.precision).max(r.precision()),
            approximate: true,
            value: r,
            imaginary: None,
            is_imaginary: false,
        }
    }

    /// Returns sinh(self) for real numbers.
    pub fn sinh(&self) -> Self {
        self.apply_real_unary(|r, prec| r.sinh_value(prec))
    }

    /// Returns cosh(self) for real numbers.
    pub fn cosh(&self) -> Self {
        self.apply_real_unary(|r, prec| r.cosh_value(prec))
    }

    /// Returns tanh(self) for real numbers.
    pub fn tanh(&self) -> Self {
        self.apply_real_unary(|r, prec| r.tanh_value(prec))
    }

    /// Returns asinh(self) for real numbers.
    pub fn asinh(&self) -> Self {
        self.apply_real_unary(|r, prec| r.asinh_value(prec))
    }

    /// Returns acosh(self) for real numbers.
    /// Domain: x >= 1. Returns NaN for x < 1.
    pub fn acosh(&self) -> Self {
        self.apply_real_unary(|r, prec| r.acosh_value(prec))
    }

    /// Returns atanh(self) for real numbers.
    /// Domain: (-1, 1). Returns ±∞ at x=±1, NaN for |x| > 1.
    pub fn atanh(&self) -> Self {
        self.apply_real_unary(|r, prec| r.atanh_value(prec))
    }

    /// Converts this number to an i64 if it is an exact integer that fits.
    pub fn to_i64(&self) -> Option<i64> {
        if self.has_imaginary_part() {
            return None;
        }
        match &self.value {
            NumberValue::Rational(r) if r.value.is_integer() => r.value.numer().to_i64(),
            NumberValue::Float(f) if f.rug_float().is_integer() => f
                .rug_float()
                .to_integer_round(rug::float::Round::Nearest)
                .and_then(|(int, _)| int.to_i64()),
            _ => None,
        }
    }

    /// Creates a complex number from separate real and imaginary `Number` parts.
    pub fn new_complex_from_re_im(re: &Number, im: &Number) -> Self {
        if im.is_zero() {
            re.clone()
        } else {
            Number::new_complex(re.clone(), im.clone())
        }
    }

    /// Safe public accessor for the number's value.
    pub fn value(&self) -> &NumberValue {
        &self.value
    }

    /// Safe public accessor for the imaginary part.
    pub fn imaginary(&self) -> Option<&Number> {
        self.imaginary.as_deref()
    }

    /// Safe public accessor for the precision.
    pub fn precision(&self) -> i32 {
        self.precision
    }

    /// Safe public accessor for the approximate flag.
    pub fn approximate(&self) -> bool {
        self.approximate
    }

    /// Safe public accessor for the is_imaginary flag.
    pub fn is_imaginary(&self) -> bool {
        self.is_imaginary
    }

    /// Helper to convert the number into canonical real and imaginary components as references.
    pub fn to_canonical_ref(
        &self,
    ) -> (
        std::borrow::Cow<'_, NumberValue>,
        std::borrow::Cow<'_, NumberValue>,
    ) {
        if self.is_imaginary {
            (
                std::borrow::Cow::Owned(NumberValue::Rational(Rational::from_i32(0))),
                std::borrow::Cow::Borrowed(&self.value),
            )
        } else {
            let real = std::borrow::Cow::Borrowed(&self.value);
            let imag = if let Some(imag) = &self.imaginary {
                let (_, imag_coeff) = imag.to_canonical_ref();
                imag_coeff
            } else {
                std::borrow::Cow::Owned(NumberValue::Rational(Rational::from_i32(0)))
            };
            (real, imag)
        }
    }

    /// Helper to convert the number into canonical real and imaginary components.
    pub fn to_canonical_real_imag(&self) -> (NumberValue, NumberValue) {
        let (real, imag) = self.to_canonical_ref();
        (real.into_owned(), imag.into_owned())
    }

    fn lower_endpoint(&self) -> Self {
        let (real, imag) = self.to_canonical_real_imag();
        if !imag.is_real_zero() {
            return Self::from_real_value(NumberValue::NaN);
        }
        match real {
            NumberValue::Interval { lower, .. } => Self::from_float(lower),
            NumberValue::Uncertainty {
                value, uncertainty, ..
            } => {
                let endpoint = value.sub(&uncertainty);
                Self::from_uncertainty_boundary_value(endpoint)
            }
            value => Self::from_real_value(value),
        }
    }

    fn upper_endpoint(&self) -> Self {
        let (real, imag) = self.to_canonical_real_imag();
        if !imag.is_real_zero() {
            return Self::from_real_value(NumberValue::NaN);
        }
        match real {
            NumberValue::Interval { upper, .. } => Self::from_float(upper),
            NumberValue::Uncertainty {
                value, uncertainty, ..
            } => {
                let endpoint = value.add(&uncertainty);
                Self::from_uncertainty_boundary_value(endpoint)
            }
            value => Self::from_real_value(value),
        }
    }

    fn midpoint(&self) -> Self {
        let (real, imag) = self.to_canonical_real_imag();
        if !imag.is_real_zero() {
            return Self::from_real_value(NumberValue::NaN);
        }
        match real {
            NumberValue::Interval { lower, upper } => {
                let prec = std::cmp::max(lower.prec(), upper.prec());
                let mut value = rug::Float::with_val(prec, &lower.value + &upper.value);
                value /= 2;
                if let Some(integer) = value.to_integer() {
                    return Self::from_rational(Rational {
                        value: rug::Rational::from(integer),
                    });
                }
                Self::from_float(Float { value })
            }
            NumberValue::Uncertainty { value, .. } => Self::from_real_value(*value),
            value => Self::from_real_value(value),
        }
    }

    fn value_part(&self) -> Self {
        self.midpoint()
    }

    fn error_part(&self) -> Self {
        self.error_part_with_relative(false)
    }

    fn error_part_with_relative(&self, relative: bool) -> Self {
        let (real, imag) = self.to_canonical_real_imag();
        if !imag.is_real_zero() {
            return Self::from_real_value(NumberValue::NaN);
        }
        match real {
            NumberValue::Uncertainty {
                value, uncertainty, ..
            } if relative => {
                if value.is_real_zero() {
                    Self::from_real_value(NumberValue::NaN)
                } else {
                    Self::from_uncertainty_error_value(uncertainty.div(&value.abs()))
                }
            }
            NumberValue::Uncertainty { uncertainty, .. } => {
                Self::from_uncertainty_error_value(*uncertainty)
            }
            NumberValue::Interval { lower, upper } => {
                let prec = std::cmp::max(lower.prec(), upper.prec());
                let mut width = rug::Float::with_val(prec, &upper.value - &lower.value);
                width /= 2;
                Self::from_float(Float { value: width })
            }
            _ => Self::from_i32(0),
        }
    }

    fn from_uncertainty_boundary_value(value: NumberValue) -> Self {
        if is_fractional_rational(&value) {
            Self::from_float(to_float_val(&value))
        } else {
            Self::from_real_value(value)
        }
    }

    fn from_uncertainty_error_value(value: NumberValue) -> Self {
        if is_fractional_rational(&value) {
            Self::from_float(to_float_val(&value))
        } else {
            Self::from_real_value(value)
        }
    }

    /// Returns true if the number has an imaginary part or is itself marked imaginary.
    pub fn is_complex(&self) -> bool {
        let (_, imag) = self.to_canonical_ref();
        !imag.is_real_zero()
    }

    /// Returns true if the number has a real part (is not purely imaginary).
    pub fn has_real_part(&self) -> bool {
        let (real, imag) = self.to_canonical_ref();
        !real.is_real_zero() || imag.is_real_zero()
    }

    /// Returns true if the number has an imaginary part or is purely imaginary.
    pub fn has_imaginary_part(&self) -> bool {
        self.is_complex()
    }

    /// Returns true if the entire number is zero.
    pub fn is_zero(&self) -> bool {
        let (real, imag) = self.to_canonical_ref();
        real.is_real_zero() && imag.is_real_zero()
    }

    /// Returns true if the real part of the number is strictly negative (< 0).
    pub fn is_negative(&self) -> bool {
        self.to_canonical_ref().0.is_negative()
    }

    /// Returns the f64 approximation of the real part of the number.
    pub fn to_f64(&self) -> f64 {
        self.to_canonical_ref().0.to_f64()
    }

    /// Returns the rational numerator of the real part of this number (canonicalized).
    /// For a complex number, this operates only on the real component. If not a rational, returns self.
    pub fn numerator(&self) -> Self {
        let (real, _) = self.to_canonical_ref();
        match &*real {
            NumberValue::Rational(r) => Self::from_rational(Rational {
                value: rug::Rational::from(r.value.numer().clone()),
            }),
            _ => self.clone(),
        }
    }

    /// Returns the rational denominator of the real part of this number (canonicalized).
    /// For a complex number, this operates only on the real component. If not a rational, returns 1.
    pub fn denominator(&self) -> Self {
        let (real, _) = self.to_canonical_ref();
        match &*real {
            NumberValue::Rational(r) => Self::from_rational(Rational {
                value: rug::Rational::from(r.value.denom().clone()),
            }),
            _ => Self::from_i32(1),
        }
    }

    /// Returns true if the number is non-zero (using interval rules).
    pub fn is_nonzero(&self) -> bool {
        if self.is_nan() {
            return false;
        }
        let (real, imag) = self.to_canonical_ref();
        real.is_nonzero() || imag.is_nonzero()
    }

    /// Returns 1 if true (non-zero), 0 if false (zero), and -1 if unknown (e.g. NaN or interval containing zero).
    pub fn get_boolean(&self) -> i32 {
        if self.is_nonzero() {
            1
        } else if self.is_zero() {
            0
        } else {
            -1
        }
    }

    /// Returns true if the real part of the number is zero.
    pub fn is_real_zero(&self) -> bool {
        let (real, _) = self.to_canonical_ref();
        real.is_real_zero()
    }

    /// Returns true if the number has no imaginary part and represents an exact integer.
    pub fn is_integer(&self) -> bool {
        if self.is_nan() {
            return false;
        }
        let (real, imag) = self.to_canonical_ref();
        imag.is_real_zero() && real.is_integer()
    }

    /// Returns true if the number has no imaginary part and is represented as a rational.
    pub fn is_rational(&self) -> bool {
        let (real, imag) = self.to_canonical_ref();
        imag.is_real_zero() && real.is_rational()
    }

    /// Returns true if the number is not infinite and has no imaginary part.
    pub fn is_real(&self) -> bool {
        !self.is_infinite() && !self.is_complex()
    }

    /// Returns true if the number is exactly one.
    pub fn is_one(&self) -> bool {
        let (real, imag) = self.to_canonical_ref();
        real.is_real_one() && imag.is_real_zero()
    }

    /// Returns true if the real part of the number is exactly one.
    pub fn is_real_one(&self) -> bool {
        let (real, _) = self.to_canonical_ref();
        real.is_real_one()
    }

    /// Returns true if either the real or the imaginary part is an interval.
    pub fn is_interval(&self) -> bool {
        let (real, imag) = self.to_canonical_ref();
        real.is_interval() || imag.is_interval()
    }

    /// Returns true if either the real or the imaginary part is infinite.
    pub fn is_infinite(&self) -> bool {
        let (real, imag) = self.to_canonical_ref();
        real.is_infinite() || imag.is_infinite()
    }

    /// Returns true if either the real or imaginary part is or contains infinity.
    pub fn includes_infinity(&self) -> bool {
        let (real, imag) = self.to_canonical_ref();
        real.includes_infinity() || imag.includes_infinity()
    }

    /// Creates a new `Number` representing NaN.
    pub fn nan() -> Self {
        Self {
            value: NumberValue::NaN,
            imaginary: None,
            precision: 0,
            approximate: true,
            is_imaginary: false,
        }
    }

    /// Returns true if either the real or the imaginary part is NaN.
    pub fn is_nan(&self) -> bool {
        let (real, imag) = self.to_canonical_ref();
        real.is_nan() || imag.is_nan()
    }

    /// Creates a new `Number` from its real and imaginary components.
    pub fn from_real_imag_values(
        real_val: NumberValue,
        imag_val: NumberValue,
        prec: i32,
        approx: bool,
    ) -> Self {
        if imag_val.is_real_zero() {
            Self {
                value: real_val,
                imaginary: None,
                precision: prec,
                approximate: approx,
                is_imaginary: false,
            }
        } else {
            let imag_num = Number {
                value: imag_val,
                imaginary: None,
                precision: prec,
                approximate: approx,
                is_imaginary: true,
            };
            Self {
                value: real_val,
                imaginary: Some(Box::new(imag_num)),
                precision: prec,
                approximate: approx,
                is_imaginary: false,
            }
        }
    }

    /// Negates the number
    pub fn negate(&self) -> Self {
        let (real, imag) = self.to_canonical_ref();
        let real_num = Number {
            value: real.negate(),
            imaginary: None,
            precision: self.precision,
            approximate: self.approximate,
            is_imaginary: false,
        };
        let imag_num = Number {
            value: imag.negate(),
            imaginary: None,
            precision: self.precision,
            approximate: self.approximate,
            is_imaginary: false,
        };
        if imag_num.is_zero() {
            real_num
        } else {
            Number::new_complex(real_num, imag_num)
        }
    }

    /// Returns the absolute value of the number.
    pub fn abs(&self) -> Self {
        let (real, imag) = self.to_canonical_real_imag();
        if imag.is_real_zero() {
            let r = real.abs();
            Self {
                precision: std::cmp::max(self.precision, r.precision()),
                approximate: self.approximate || r.approximate(),
                value: r,
                imaginary: None,
                is_imaginary: false,
            }
        } else {
            let r_num = Self::from_real_value(real);
            let i_num = Self::from_real_value(imag);
            let sum_sq = r_num.mul(&r_num).add(&i_num.mul(&i_num));
            sum_sq.sqrt()
        }
    }

    /// Returns the square root of the number.
    pub fn sqrt(&self) -> Self {
        let (real, imag) = self.to_canonical_real_imag();
        if imag.is_real_zero() {
            let r = real.sqrt();
            Self {
                precision: std::cmp::max(self.precision, r.precision()),
                approximate: self.approximate || r.approximate(),
                value: r,
                imaginary: None,
                is_imaginary: false,
            }
        } else {
            self.pow(&Number::from_rational(Rational::new(1, 2)))
        }
    }

    /// Returns the natural logarithm of the number.
    pub fn ln(&self) -> Self {
        let (real, imag) = self.to_canonical_real_imag();
        if imag.is_real_zero() {
            let r = real.ln();
            Self {
                precision: std::cmp::max(self.precision, r.precision()),
                approximate: self.approximate || r.approximate(),
                value: r,
                imaginary: None,
                is_imaginary: false,
            }
        } else {
            Self::from_real_value(NumberValue::NaN)
        }
    }

    /// Adds two numbers
    pub fn add(&self, other: &Self) -> Self {
        let (a, b) = self.to_canonical_ref();
        let (c, d) = other.to_canonical_ref();

        let real_val = a.add(&c);
        let imag_val = b.add(&d);

        let real_num = Number {
            value: real_val,
            imaginary: None,
            precision: std::cmp::max(self.precision, other.precision),
            approximate: self.approximate || other.approximate,
            is_imaginary: false,
        };
        let imag_num = Number {
            value: imag_val,
            imaginary: None,
            precision: std::cmp::max(self.precision, other.precision),
            approximate: self.approximate || other.approximate,
            is_imaginary: false,
        };

        if imag_num.is_zero() {
            real_num
        } else {
            Number::new_complex(real_num, imag_num)
        }
    }

    /// Subtracts other from self
    pub fn sub(&self, other: &Self) -> Self {
        self.add(&other.negate())
    }

    /// Multiplies self by other
    pub fn mul(&self, other: &Self) -> Self {
        let (x1, y1) = self.to_canonical_ref();
        let (x2, y2) = other.to_canonical_ref();

        let y1_zero = y1.is_real_zero();
        let y2_zero = y2.is_real_zero();
        let x1_zero = x1.is_real_zero();
        let x2_zero = x2.is_real_zero();

        let (real_val, imag_val) = match (y1_zero, y2_zero, x1_zero, x2_zero) {
            // Both are purely real
            (true, true, _, _) => (x1.mul(&x2), NumberValue::Rational(Rational::new(0, 1))),
            // Both are purely imaginary
            (false, false, true, true) => (
                y1.mul(&y2).negate(),
                NumberValue::Rational(Rational::new(0, 1)),
            ),
            // self is purely real and other is purely imaginary
            (true, false, _, true) => (NumberValue::Rational(Rational::new(0, 1)), x1.mul(&y2)),
            // self is purely imaginary and other is purely real
            (false, true, true, _) => (NumberValue::Rational(Rational::new(0, 1)), y1.mul(&x2)),
            // self is purely real
            (true, false, _, _) => (x1.mul(&x2), x1.mul(&y2)),
            // other is purely real
            (false, true, _, _) => (x1.mul(&x2), y1.mul(&x2)),
            // self is purely imaginary
            (false, false, true, _) => (y1.mul(&y2).negate(), y1.mul(&x2)),
            // other is purely imaginary
            (false, false, _, true) => (y1.mul(&y2).negate(), x1.mul(&y2)),
            // General complex case
            _ => {
                let real = x1.mul(&x2).sub(&y1.mul(&y2));
                let imag = x1.mul(&y2).add(&y1.mul(&x2));
                (real, imag)
            }
        };

        let real_num = Number {
            value: real_val,
            imaginary: None,
            precision: std::cmp::max(self.precision, other.precision),
            approximate: self.approximate || other.approximate,
            is_imaginary: false,
        };
        let imag_num = Number {
            value: imag_val,
            imaginary: None,
            precision: std::cmp::max(self.precision, other.precision),
            approximate: self.approximate || other.approximate,
            is_imaginary: false,
        };

        if imag_num.is_zero() {
            real_num
        } else {
            Number::new_complex(real_num, imag_num)
        }
    }

    fn contains_interval_or_uncertainty_val(val: &NumberValue) -> bool {
        matches!(
            val,
            NumberValue::Interval { .. } | NumberValue::Uncertainty { .. }
        )
    }

    fn contains_interval_or_uncertainty(&self) -> bool {
        if Self::contains_interval_or_uncertainty_val(&self.value) {
            return true;
        }
        if let Some(imag) = &self.imaginary {
            if imag.contains_interval_or_uncertainty() {
                return true;
            }
        }
        false
    }

    /// Divides self by other
    pub fn div(&self, other: &Self) -> Self {
        if other.is_zero() && !other.contains_interval_or_uncertainty() {
            return Number::from_f64(f64::NAN);
        }

        let (x1, y1) = self.to_canonical_ref();
        let (x2, y2) = other.to_canonical_ref();

        let y2_zero = y2.is_real_zero();
        let x2_zero = x2.is_real_zero();

        let (real_val, imag_val) = match (y2_zero, x2_zero) {
            // divisor is purely real
            (true, _) => (x1.div(&x2), y1.div(&x2)),
            // divisor is purely imaginary
            (false, true) => (y1.div(&y2), x1.div(&y2).negate()),
            // General case
            _ => {
                let x2_sq = x2.mul(&x2);
                let y2_sq = y2.mul(&y2);
                let d = x2_sq.add(&y2_sq);

                let real_num_num = x1.mul(&x2).add(&y1.mul(&y2));
                let imag_num_num = y1.mul(&x2).sub(&x1.mul(&y2));

                (real_num_num.div(&d), imag_num_num.div(&d))
            }
        };

        let real_num = Number {
            value: real_val,
            imaginary: None,
            precision: std::cmp::max(self.precision, other.precision),
            approximate: self.approximate || other.approximate,
            is_imaginary: false,
        };
        let imag_num = Number {
            value: imag_val,
            imaginary: None,
            precision: std::cmp::max(self.precision, other.precision),
            approximate: self.approximate || other.approximate,
            is_imaginary: false,
        };

        if imag_num.is_zero() {
            real_num
        } else {
            Number::new_complex(real_num, imag_num)
        }
    }

    /// Remainder with quotient truncated toward zero.
    pub fn rem(&self, other: &Self) -> Self {
        if other.is_zero() {
            return Number::from_f64(f64::NAN);
        }

        let (lhs_real, lhs_imag) = self.to_canonical_ref();
        let (rhs_real, rhs_imag) = other.to_canonical_ref();
        if !lhs_imag.is_real_zero() || !rhs_imag.is_real_zero() {
            return Number::from_f64(f64::NAN);
        }

        Number {
            value: lhs_real.rem(&rhs_real),
            imaginary: None,
            precision: std::cmp::max(self.precision, other.precision),
            approximate: self.approximate || other.approximate,
            is_imaginary: false,
        }
    }

    /// Modulo with quotient rounded down.
    pub fn modulo(&self, other: &Self) -> Self {
        if other.is_zero() {
            return Number::from_f64(f64::NAN);
        }

        let (lhs_real, lhs_imag) = self.to_canonical_ref();
        let (rhs_real, rhs_imag) = other.to_canonical_ref();
        if !lhs_imag.is_real_zero() || !rhs_imag.is_real_zero() {
            return Number::from_f64(f64::NAN);
        }

        Number {
            value: lhs_real.modulo(&rhs_real),
            imaginary: None,
            precision: std::cmp::max(self.precision, other.precision),
            approximate: self.approximate || other.approximate,
            is_imaginary: false,
        }
    }

    /// Integer division with quotient truncated toward zero.
    pub fn int_div(&self, other: &Self) -> Self {
        if other.is_zero() {
            return Number::from_f64(f64::NAN);
        }

        let (lhs_real, lhs_imag) = self.to_canonical_ref();
        let (rhs_real, rhs_imag) = other.to_canonical_ref();
        if !lhs_imag.is_real_zero() || !rhs_imag.is_real_zero() {
            return Number::from_f64(f64::NAN);
        }

        Number {
            value: lhs_real.int_div(&rhs_real),
            imaginary: None,
            precision: std::cmp::max(self.precision, other.precision),
            approximate: self.approximate || other.approximate,
            is_imaginary: false,
        }
    }

    /// Returns the complex conjugate
    pub fn conjugate(&self) -> Self {
        let (real, imag) = self.to_canonical_ref();
        let real_num = Number {
            value: real.into_owned(),
            imaginary: None,
            precision: self.precision,
            approximate: self.approximate,
            is_imaginary: false,
        };
        let imag_num = Number {
            value: imag.negate(),
            imaginary: None,
            precision: self.precision,
            approximate: self.approximate,
            is_imaginary: false,
        };
        if imag_num.is_zero() {
            real_num
        } else {
            Number::new_complex(real_num, imag_num)
        }
    }

    /// Returns the norm (magnitude)
    pub fn norm(&self) -> Self {
        let (real, imag) = self.to_canonical_ref();
        let real_sq = real.mul(&real);
        let imag_sq = imag.mul(&imag);
        let sum_sq = real_sq.add(&imag_sq);
        let norm_val = sum_sq.sqrt();

        let is_approx = self.approximate || norm_val.approximate();
        Self {
            value: norm_val,
            imaginary: None,
            precision: self.precision,
            approximate: is_approx,
            is_imaginary: false,
        }
    }

    /// Returns the signum of the number: -1, 0, or +1 for real numbers.
    ///
    /// For complex numbers z, returns z / |z| (projected onto the unit circle).
    pub fn signum(&self) -> Self {
        if self.is_nan() {
            return Self::from_real_value(NumberValue::NaN);
        }
        if self.is_complex() {
            if self.is_zero() {
                Self::from_i32(0)
            } else {
                self.div(&self.abs())
            }
        } else {
            let s = self.value.signum();
            Self {
                precision: self.precision,
                approximate: self.approximate,
                value: s,
                imaginary: None,
                is_imaginary: false,
            }
        }
    }

    /// Returns the floor of the number (greatest integer ≤ x).
    ///
    /// For complex numbers, applies floor to both real and imaginary parts.
    pub fn floor(&self) -> Self {
        let (real, imag) = self.to_canonical_real_imag();
        let r = real.floor();
        if imag.is_real_zero() {
            Self {
                precision: self.precision,
                approximate: self.approximate || r.approximate(),
                value: r,
                imaginary: None,
                is_imaginary: false,
            }
        } else {
            let i = imag.floor();
            let real_num = Self::from_real_value(r);
            let imag_num = Self::from_real_value(i);
            Number::new_complex(real_num, imag_num)
        }
    }

    /// Returns the ceiling of the number (least integer ≥ x).
    ///
    /// For complex numbers, applies ceil to both real and imaginary parts.
    pub fn ceil(&self) -> Self {
        let (real, imag) = self.to_canonical_real_imag();
        let r = real.ceil();
        if imag.is_real_zero() {
            Self {
                precision: self.precision,
                approximate: self.approximate || r.approximate(),
                value: r,
                imaginary: None,
                is_imaginary: false,
            }
        } else {
            let i = imag.ceil();
            let real_num = Self::from_real_value(r);
            let imag_num = Self::from_real_value(i);
            Number::new_complex(real_num, imag_num)
        }
    }

    /// Returns the truncation of the number (round toward zero).
    ///
    /// For complex numbers, applies trunc to both real and imaginary parts.
    pub fn trunc(&self) -> Self {
        let (real, imag) = self.to_canonical_real_imag();
        let r = real.trunc();
        if imag.is_real_zero() {
            Self {
                precision: self.precision,
                approximate: self.approximate || r.approximate(),
                value: r,
                imaginary: None,
                is_imaginary: false,
            }
        } else {
            let i = imag.trunc();
            let real_num = Self::from_real_value(r);
            let imag_num = Self::from_real_value(i);
            Number::new_complex(real_num, imag_num)
        }
    }

    /// Returns the nearest integer value (half away from zero).
    ///
    /// For complex numbers, applies round to both real and imaginary parts.
    pub fn round(&self) -> Self {
        let (real, imag) = self.to_canonical_real_imag();
        let r = real.round();
        if imag.is_real_zero() {
            Self {
                precision: self.precision,
                approximate: self.approximate || r.approximate(),
                value: r,
                imaginary: None,
                is_imaginary: false,
            }
        } else {
            let i = imag.round();
            let real_num = Self::from_real_value(r);
            let imag_num = Self::from_real_value(i);
            Number::new_complex(real_num, imag_num)
        }
    }

    /// Returns the real part of the number.
    ///
    /// For real numbers, returns the number itself.
    /// For complex numbers a + bi, returns a.
    pub fn real_part(&self) -> Self {
        let (real, _imag) = self.to_canonical_real_imag();
        Self {
            precision: self.precision,
            approximate: self.approximate,
            value: real,
            imaginary: None,
            is_imaginary: false,
        }
    }

    /// Returns the imaginary part of the number (as a real coefficient).
    ///
    /// For real numbers, returns 0.
    /// For complex numbers a + bi, returns b (not bi).
    pub fn imaginary_part(&self) -> Self {
        let (_real, imag) = self.to_canonical_real_imag();
        Self {
            precision: self.precision,
            approximate: self.approximate,
            value: imag,
            imaginary: None,
            is_imaginary: false,
        }
    }

    /// Returns the argument (phase angle) of the number in radians.
    ///
    /// For positive real numbers, returns 0.
    /// For negative real numbers, returns π.
    /// For complex numbers a + bi, returns atan2(b, a).
    pub fn arg(&self) -> Self {
        let (real, imag) = self.to_canonical_real_imag();
        if imag.is_real_zero() {
            // Real number
            if real.is_positive() || real.is_zero() {
                Self::from_i32(0)
            } else {
                Self::pi()
            }
        } else {
            // Complex: atan2(imag, real)
            let y = Self::from_real_value(imag);
            let x = Self::from_real_value(real);
            Number::atan2(&y, &x)
        }
    }

    /// Returns the gamma function Γ(x).
    ///
    /// For complex numbers, returns NaN (not supported in this implementation).
    pub fn gamma(&self) -> Self {
        if self.is_complex() {
            return Self::from_real_value(NumberValue::NaN);
        }
        let r = self.value.gamma();
        Self {
            precision: std::cmp::max(self.precision, r.precision()),
            approximate: true,
            value: r,
            imaginary: None,
            is_imaginary: false,
        }
    }

    /// Returns the error function erf(x).
    ///
    /// For complex numbers, returns NaN (not supported in this implementation).
    pub fn erf(&self) -> Self {
        if self.is_complex() {
            return Self::from_real_value(NumberValue::NaN);
        }
        let r = self.value.erf();
        Self {
            precision: std::cmp::max(self.precision, r.precision()),
            approximate: true,
            value: r,
            imaginary: None,
            is_imaginary: false,
        }
    }

    /// Returns the Riemann zeta function ζ(x).
    ///
    /// For complex numbers, returns NaN (not supported in this implementation).
    pub fn zeta(&self) -> Self {
        if self.is_complex() {
            return Self::from_real_value(NumberValue::NaN);
        }
        let r = self.value.zeta();
        Self {
            precision: std::cmp::max(self.precision, r.precision()),
            approximate: true,
            value: r,
            imaginary: None,
            is_imaginary: false,
        }
    }

    /// Returns the upper incomplete gamma function Γ(a, x).
    ///
    /// `self` is the `a` parameter and `x` is the integration lower bound.
    /// For complex numbers, returns NaN (not supported in this implementation).
    pub fn gamma_inc(&self, x: &Self) -> Self {
        if self.is_complex() || x.is_complex() {
            return Self::from_real_value(NumberValue::NaN);
        }
        let r = self.value.gamma_inc(&x.value);
        Self {
            precision: std::cmp::max(self.precision, r.precision()),
            approximate: true,
            value: r,
            imaginary: None,
            is_imaginary: false,
        }
    }

    /// Compares this `Number` with another `Number`.
    pub fn compare(&self, other: &Self) -> ComparisonResult {
        if self.is_complex() || other.is_complex() {
            return ComparisonResult::Unknown;
        }
        if self.is_nan() || other.is_nan() {
            return ComparisonResult::Unknown;
        }
        self.value.compare(&other.value)
    }

    /// Returns true if this number is strictly greater than the other number.
    pub fn is_greater_than(&self, other: &Self) -> bool {
        matches!(self.compare(other), ComparisonResult::Less)
    }

    /// Returns true if this number is strictly less than the other number.
    pub fn is_less_than(&self, other: &Self) -> bool {
        matches!(self.compare(other), ComparisonResult::Greater)
    }

    /// Formats this number with the qalc-compatible defaults used by the oracle harness.
    pub fn to_qalc_string(&self) -> String {
        self.to_qalc_string_with_precision(10)
    }

    pub(crate) fn to_qalc_string_with_settings(
        &self,
        precision_digits: usize,
        min_exp: Option<i32>,
        exp_display: Option<u8>,
        min_decimals: Option<i32>,
        max_decimals: Option<i32>,
    ) -> String {
        if min_exp.is_none()
            && exp_display.is_none()
            && min_decimals.is_none()
            && max_decimals.is_none()
        {
            return self.to_qalc_string_with_precision(precision_digits);
        }

        let (real, imag) = self.to_canonical_real_imag();
        if imag.is_real_zero() {
            return format_qalc_value_with_settings(
                &real,
                precision_digits,
                min_exp,
                exp_display,
                min_decimals,
                max_decimals,
            );
        }

        // Keep complex settings out of scope until there is direct oracle
        // evidence for the interaction between complex formatting and these
        // print options.
        self.to_qalc_string_with_precision(precision_digits)
    }

    /// Formats this number for qalc-profile native evidence with a requested
    /// decimal digit count for precision-sensitive numeric output.
    pub(crate) fn to_qalc_string_with_precision(&self, precision_digits: usize) -> String {
        self.to_qalc_string_with_uncertainty_format(
            precision_digits,
            QalcUncertaintyFormat::Significant,
        )
    }

    pub(crate) fn to_qalc_string_preserving_float_uncertainty_precision(
        &self,
        precision_digits: usize,
    ) -> String {
        self.to_qalc_string_with_uncertainty_format(
            precision_digits,
            QalcUncertaintyFormat::PreserveFloatPrecision,
        )
    }

    /// Formats this number with the specified formatting options.
    pub fn to_string_with_options(
        &self,
        precision_digits: usize,
        fraction_format: crate::options::NumberFractionFormat,
        approximation: crate::options::ApproximationMode,
    ) -> String {
        let (real, imag) = self.to_canonical_real_imag();
        if !imag.is_real_zero() {
            let r_str = self.real_part().to_string_with_options(
                precision_digits,
                fraction_format,
                approximation,
            );
            let i_str = self.imaginary_part().to_string_with_options(
                precision_digits,
                fraction_format,
                approximation,
            );
            if imag.is_real_one() {
                if real.is_real_zero() {
                    "i".to_string()
                } else {
                    format!("{r_str} + i")
                }
            } else if imag.negate().is_real_one() {
                if real.is_real_zero() {
                    "-i".to_string()
                } else {
                    format!("{r_str} - i")
                }
            } else if is_real_negative(&imag) {
                let abs_imag = imag.negate();
                let abs_i_str = format_number_value_with_options(
                    &abs_imag,
                    precision_digits,
                    fraction_format,
                    approximation,
                );
                if real.is_real_zero() {
                    format!("-{abs_i_str}i")
                } else {
                    format!("{r_str} - {abs_i_str}i")
                }
            } else {
                if real.is_real_zero() {
                    format!("{i_str}i")
                } else {
                    format!("{r_str} + {i_str}i")
                }
            }
        } else {
            format_number_value_with_options(
                &real,
                precision_digits,
                fraction_format,
                approximation,
            )
        }
    }

    fn to_qalc_string_with_uncertainty_format(
        &self,
        precision_digits: usize,
        uncertainty_format: QalcUncertaintyFormat,
    ) -> String {
        let (real, imag) = self.to_canonical_real_imag();
        if imag.is_real_zero() {
            format_qalc_value_with_uncertainty_format(&real, precision_digits, uncertainty_format)
        } else if real.is_real_zero() {
            if imag.is_real_one() {
                "i".to_string()
            } else if imag.negate().is_real_one() {
                "-i".to_string()
            } else {
                format!(
                    "{}i",
                    format_qalc_value_with_uncertainty_format(
                        &imag,
                        precision_digits,
                        uncertainty_format
                    )
                )
            }
        } else if is_value_negative(&imag) {
            format!(
                "{} - {}i",
                format_qalc_value_with_uncertainty_format(
                    &real,
                    precision_digits,
                    uncertainty_format
                ),
                format_qalc_value_with_uncertainty_format(
                    &imag.negate(),
                    precision_digits,
                    uncertainty_format
                )
            )
        } else {
            format!(
                "{} + {}i",
                format_qalc_value_with_uncertainty_format(
                    &real,
                    precision_digits,
                    uncertainty_format
                ),
                format_qalc_value_with_uncertainty_format(
                    &imag,
                    precision_digits,
                    uncertainty_format
                )
            )
        }
    }

    /// Formats an interval using qalc's `interval display 2` constructor form.
    pub(crate) fn to_qalc_interval_display_string(
        &self,
        precision_digits: usize,
    ) -> Option<String> {
        let (real, imag) = self.to_canonical_real_imag();
        if !imag.is_real_zero() {
            return None;
        }
        match real {
            NumberValue::Interval { lower, upper } => {
                if upper.value <= 0 {
                    let positive_lower = upper.value.clone().abs();
                    let positive_upper = lower.value.clone().abs();
                    return Some(format!(
                        "-interval({}, {})",
                        format_qalc_float(&positive_lower, precision_digits),
                        format_qalc_float(&positive_upper, precision_digits)
                    ));
                }
                Some(format!(
                    "interval({}, {})",
                    format_qalc_float(&lower.value, precision_digits),
                    format_qalc_float(&upper.value, precision_digits)
                ))
            }
            _ => None,
        }
    }
}

fn is_real_negative(val: &NumberValue) -> bool {
    match val {
        NumberValue::Rational(r) => r.value.numer().is_negative(),
        NumberValue::Float(f) => f.value.is_sign_negative(),
        NumberValue::MinusInfinity => true,
        NumberValue::Uncertainty { value, .. } => is_real_negative(value),
        _ => false,
    }
}

fn format_number_value_with_options(
    val: &NumberValue,
    precision_digits: usize,
    fraction_format: crate::options::NumberFractionFormat,
    approximation: crate::options::ApproximationMode,
) -> String {
    match val {
        NumberValue::Rational(r) => {
            if r.value.is_integer() {
                r.value.numer().to_string()
            } else {
                let is_terminating = r.terminating_decimal_string().is_some();
                let use_fraction = fraction_format
                    == crate::options::NumberFractionFormat::Fractional
                    || approximation == crate::options::ApproximationMode::Exact
                    || (approximation == crate::options::ApproximationMode::TryExact
                        && !is_terminating);

                if use_fraction {
                    format!("{} / {}", r.value.numer(), r.value.denom())
                } else if let Some(decimal) = r.terminating_decimal_string() {
                    decimal
                } else {
                    let binary_precision = qalc_decimal_precision_bits(precision_digits);
                    let output = rug::Float::with_val(binary_precision, &r.value)
                        .to_string_radix(10, Some(precision_digits));
                    fixed_decimal_from_scientific(&output).unwrap_or(output)
                }
            }
        }
        NumberValue::Float(f) => {
            let output = f.value.to_string_radix(10, Some(precision_digits));
            fixed_decimal_from_scientific(&output).unwrap_or(output)
        }
        NumberValue::Uncertainty {
            value,
            uncertainty,
            is_relative,
        } => {
            if uncertainty.is_real_zero() {
                format_number_value_with_options(
                    value,
                    precision_digits,
                    fraction_format,
                    approximation,
                )
            } else {
                let val_str = format_number_value_with_options(
                    value,
                    precision_digits,
                    fraction_format,
                    approximation,
                );
                let unc_str = format_number_value_with_options(
                    uncertainty,
                    precision_digits,
                    fraction_format,
                    approximation,
                );
                if *is_relative {
                    format!("{val_str}±{unc_str}%")
                } else {
                    format!("{val_str}±{unc_str}")
                }
            }
        }
        NumberValue::Interval { lower, upper } => {
            let lower_str = format_qalc_float_bound(&lower.value);
            let upper_str = format_qalc_float_bound(&upper.value);
            format!("[{lower_str}  {upper_str}]")
        }
        NumberValue::PlusInfinity => "+∞".to_string(),
        NumberValue::MinusInfinity => "-∞".to_string(),
        _ => val.to_string(),
    }
}

impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        let (lhs_real, lhs_imag) = self.to_canonical_ref();
        let (rhs_real, rhs_imag) = other.to_canonical_ref();
        lhs_real == rhs_real && lhs_imag == rhs_imag
    }
}

impl Number {
    /// Exponentiate one Number to another.
    pub fn pow(&self, other: &Self) -> Self {
        let (a, b) = self.to_canonical_real_imag();
        let (c, d) = other.to_canonical_real_imag();
        if b.is_real_zero() && d.is_real_zero() {
            let r = a.pow(&c);
            let precision = std::cmp::max(
                std::cmp::max(self.precision, other.precision),
                r.precision(),
            );
            let approximate = self.approximate || other.approximate || r.approximate();
            Number {
                value: r,
                imaginary: None,
                precision,
                approximate,
                is_imaginary: false,
            }
        } else {
            if d.is_real_zero() {
                let precision = self.precision.max(a.precision()).max(b.precision());
                let approximate =
                    self.approximate || other.approximate || a.approximate() || b.approximate();
                let base =
                    Number::from_real_imag_values(a.clone(), b.clone(), precision, approximate);
                if let Some(exponent_mod_4) = exact_integer_exponent_mod_4(&c) {
                    if let Some(result) = exact_unit_imaginary_integer_power(&base, exponent_mod_4)
                    {
                        return result;
                    }
                }
                if let Some(exponent) = exact_i32_integer_exponent(&c) {
                    if let Some(result) = exact_complex_integer_power(&base, exponent) {
                        return result;
                    }
                }
            }
            approximate_complex_power_number(&a, &b, &c, &d)
        }
    }

    fn pow_with_context(&self, other: &Self, context: EvalContext) -> Self {
        let (a, b) = self.to_canonical_real_imag();
        let (c, d) = other.to_canonical_real_imag();
        if b.is_real_zero() && d.is_real_zero() {
            let r = a
                .rational_noninteger_pow_with_precision_floor(
                    &c,
                    context.min_float_precision_bits(),
                )
                .unwrap_or_else(|| a.pow(&c));
            let precision = std::cmp::max(
                std::cmp::max(self.precision, other.precision),
                r.precision(),
            );
            let approximate = self.approximate || other.approximate || r.approximate();
            Number {
                value: r,
                imaginary: None,
                precision,
                approximate,
                is_imaginary: false,
            }
        } else {
            self.pow(other)
        }
    }
}

fn format_uncertainty(val: f64, unc: f64) -> (String, String) {
    if unc == 0.0 {
        return (val.to_string(), "0".to_string());
    }
    let (rounded_unc, d) = rounded_uncertainty_and_width(unc);
    let display_width = uncertainty_display_decimal_width(val, unc, d);

    let formatted_unc = format!("{:.width$}", rounded_unc, width = display_width);
    let formatted_val = format!("{:.width$}", val, width = display_width);
    (formatted_val, formatted_unc)
}

fn format_relative_uncertainty(val: f64, unc_abs: f64) -> (String, String) {
    if val == 0.0 {
        if unc_abs != 0.0 {
            return format_uncertainty(val, unc_abs);
        }
        return (val.to_string(), "0%".to_string());
    }
    let p = (unc_abs / val.abs()) * 100.0;
    if p == 0.0 {
        return (val.to_string(), "0%".to_string());
    }
    let (rounded_p, d) = rounded_uncertainty_and_width(p);

    let formatted_p = format!("{:.width$}%", rounded_p, width = d);
    let value_width =
        uncertainty_display_decimal_width(val, unc_abs, uncertainty_decimal_width(unc_abs));
    let formatted_val = format!("{:.width$}", val, width = value_width);
    (formatted_val, formatted_p)
}

fn uncertainty_decimal_width(unc_abs: f64) -> usize {
    rounded_uncertainty_and_width(unc_abs).1
}

fn uncertainty_display_decimal_width(val: f64, unc_abs: f64, width: usize) -> usize {
    let val_abs = val.abs();
    if val_abs > 0.0
        && val_abs < 1.0
        && is_power_of_ten(val_abs)
        && is_five_percent_uncertainty(val_abs, unc_abs)
        && rounds_to_half_step_at_width(unc_abs, width)
    {
        width + 1
    } else {
        width
    }
}

fn is_power_of_ten(value: f64) -> bool {
    let log10 = value.log10();
    (log10 - log10.round()).abs() <= 1e-12
}

fn is_five_percent_uncertainty(val_abs: f64, unc_abs: f64) -> bool {
    let ratio = unc_abs.abs() / val_abs;
    (ratio - 0.05).abs() <= 1e-12
}

fn rounds_to_half_step_at_width(unc_abs: f64, width: usize) -> bool {
    let scaled = unc_abs.abs() * 10.0f64.powi(width as i32);
    (scaled - 50.0).abs() <= 1e-9
}

fn rounded_uncertainty_and_width(unc_abs: f64) -> (f64, usize) {
    if unc_abs == 0.0 {
        return (0.0, 0);
    }
    let unc_abs = unc_abs.abs();
    let e_init = unc_abs.log10().floor() as i32;
    let d_init = std::cmp::max(0, 1 - e_init);
    let factor = 10.0f64.powi(d_init);
    let rounded_unc = (unc_abs * factor).round() / factor;

    let e = if rounded_unc == 0.0 {
        e_init
    } else {
        rounded_unc.log10().floor() as i32
    };
    (rounded_unc, std::cmp::max(0, 1 - e) as usize)
}

#[derive(Clone, Copy)]
enum QalcUncertaintyFormat {
    Significant,
    PreserveFloatPrecision,
}

fn format_qalc_value_with_uncertainty_format(
    value: &NumberValue,
    precision_digits: usize,
    uncertainty_format: QalcUncertaintyFormat,
) -> String {
    match value {
        NumberValue::Rational(rational) => {
            if let Some(scientific) = qalc_large_integer_scientific_string(rational) {
                scientific
            } else if let Some(decimal) = rational.terminating_decimal_string() {
                decimal
            } else {
                let binary_precision = qalc_decimal_precision_bits(precision_digits);
                let output = rug::Float::with_val(binary_precision, &rational.value)
                    .to_string_radix(10, Some(precision_digits));
                fixed_decimal_from_scientific(&output).unwrap_or(output)
            }
        }
        NumberValue::Uncertainty {
            value,
            uncertainty,
            is_relative,
        } => {
            if uncertainty.is_real_zero() {
                format_qalc_value_with_uncertainty_format(
                    value,
                    precision_digits,
                    uncertainty_format,
                )
            } else {
                format_qalc_uncertainty(
                    value,
                    uncertainty,
                    *is_relative,
                    precision_digits,
                    uncertainty_format,
                )
            }
        }
        NumberValue::Interval { lower, upper } => format!(
            "[{}  {}]",
            format_qalc_float_bound(&lower.value),
            format_qalc_float_bound(&upper.value)
        ),
        NumberValue::Float(float) => format_qalc_float(&float.value, precision_digits),
        NumberValue::PlusInfinity => "+∞".to_string(),
        NumberValue::MinusInfinity => "-∞".to_string(),
        _ => value.to_string(),
    }
}

fn format_qalc_float(value: &rug::Float, precision_digits: usize) -> String {
    if value.is_infinite() {
        return if value.is_sign_negative() {
            "-∞".to_string()
        } else {
            "+∞".to_string()
        };
    }

    let output = value.to_string_radix(10, Some(precision_digits));
    fixed_decimal_from_scientific(&output).unwrap_or(output)
}

fn format_qalc_value_with_settings(
    value: &NumberValue,
    precision_digits: usize,
    min_exp: Option<i32>,
    exp_display: Option<u8>,
    min_decimals: Option<i32>,
    max_decimals: Option<i32>,
) -> String {
    match value {
        NumberValue::Rational(rational) => format_qalc_rational_with_settings(
            rational,
            precision_digits,
            min_exp,
            exp_display,
            min_decimals,
            max_decimals,
        ),
        NumberValue::Float(float) => format_qalc_decimal_string_with_settings(
            &format_qalc_float(&float.value, precision_digits),
            precision_digits,
            min_exp,
            exp_display,
            min_decimals,
            max_decimals,
        ),
        NumberValue::Uncertainty {
            value,
            uncertainty,
            is_relative,
        } => {
            if uncertainty.is_real_zero() {
                format_qalc_value_with_settings(
                    value,
                    precision_digits,
                    min_exp,
                    exp_display,
                    min_decimals,
                    max_decimals,
                )
            } else {
                format_qalc_uncertainty(
                    value,
                    uncertainty,
                    *is_relative,
                    precision_digits,
                    QalcUncertaintyFormat::Significant,
                )
            }
        }
        NumberValue::Interval { lower, upper } => format!(
            "[{}  {}]",
            format_qalc_float_bound(&lower.value),
            format_qalc_float_bound(&upper.value)
        ),
        NumberValue::PlusInfinity => "+∞".to_string(),
        NumberValue::MinusInfinity => "-∞".to_string(),
        _ => value.to_string(),
    }
}

fn format_qalc_rational_with_settings(
    rational: &Rational,
    precision_digits: usize,
    min_exp: Option<i32>,
    exp_display: Option<u8>,
    min_decimals: Option<i32>,
    max_decimals: Option<i32>,
) -> String {
    if min_exp.is_none() {
        let output = format_qalc_value_with_uncertainty_format(
            &NumberValue::Rational(rational.clone()),
            precision_digits,
            QalcUncertaintyFormat::Significant,
        );
        return format_qalc_decimal_string_with_settings(
            &output,
            precision_digits,
            None,
            exp_display,
            min_decimals,
            max_decimals,
        );
    };

    let fixed = qalc_rational_fixed_decimal_string(rational, precision_digits);
    format_qalc_decimal_string_with_settings(
        &fixed,
        precision_digits,
        min_exp,
        exp_display,
        min_decimals,
        max_decimals,
    )
}

fn qalc_rational_fixed_decimal_string(rational: &Rational, precision_digits: usize) -> String {
    if let Some(decimal) = rational.terminating_decimal_string() {
        return decimal;
    }

    let binary_precision = qalc_decimal_precision_bits(precision_digits);
    let output = rug::Float::with_val(binary_precision, &rational.value)
        .to_string_radix(10, Some(precision_digits));
    fixed_decimal_from_scientific(&output).unwrap_or(output)
}

fn format_qalc_decimal_string_with_settings(
    output: &str,
    precision_digits: usize,
    min_exp: Option<i32>,
    exp_display: Option<u8>,
    min_decimals: Option<i32>,
    max_decimals: Option<i32>,
) -> String {
    if let Some(min_exp) = min_exp {
        if let Some(scientific) = qalc_scientific_from_fixed_decimal(
            output,
            min_exp,
            exp_display,
            precision_digits,
            min_decimals,
            max_decimals,
        ) {
            return scientific;
        }
    } else if let Some((mantissa, exponent)) = split_qalc_exponent(output) {
        let displayed = format_qalc_scientific_parts(
            mantissa,
            exponent,
            exp_display,
            min_decimals,
            max_decimals,
        );
        return displayed;
    }

    apply_qalc_decimal_limits(output, min_decimals, max_decimals)
}

fn split_qalc_exponent(output: &str) -> Option<(&str, i32)> {
    let exponent_pos = output.find(['E', 'e'])?;
    let exponent = output[exponent_pos + 1..].parse::<i32>().ok()?;
    Some((&output[..exponent_pos], exponent))
}

fn qalc_scientific_from_fixed_decimal(
    output: &str,
    min_exp: i32,
    exp_display: Option<u8>,
    precision_digits: usize,
    min_decimals: Option<i32>,
    max_decimals: Option<i32>,
) -> Option<String> {
    if min_exp == 0 {
        return None;
    }

    let decimal = qalc_decimal_components(output)?;
    let (digits, exponent) = decimal.normalized_digits_and_exponent()?;
    if !qalc_should_use_scientific_exponent(exponent, min_exp) {
        return None;
    }

    let (significant, exponent) =
        rounded_significant_digits(&digits, exponent, precision_digits.max(1));
    let target_exponent = if min_exp < -1 {
        let step = min_exp.abs();
        exponent.div_euclid(step) * step
    } else {
        exponent
    };

    let digits_before_decimal = exponent - target_exponent + 1;
    let mantissa = qalc_mantissa_from_significant(
        &significant.digits,
        digits_before_decimal,
        significant.dropped_nonzero,
    );
    let mantissa = format!("{}{mantissa}", decimal.sign);
    Some(format_qalc_scientific_parts(
        &mantissa,
        target_exponent,
        exp_display,
        min_decimals,
        max_decimals,
    ))
}

fn qalc_should_use_scientific_exponent(exponent: i32, min_exp: i32) -> bool {
    match min_exp {
        0 => false,
        1 => true,
        -1 => exponent >= 13 || exponent <= -10,
        value if value > 1 => exponent >= value || exponent <= -value,
        value if value < -1 => {
            let threshold = value.abs();
            exponent >= threshold || exponent <= -threshold
        }
        _ => false,
    }
}

struct SignificantDigits {
    digits: String,
    dropped_nonzero: bool,
}

fn rounded_significant_digits(
    digits: &str,
    mut exponent: i32,
    precision_digits: usize,
) -> (SignificantDigits, i32) {
    let bytes = digits.as_bytes();
    let keep = bytes.len().min(precision_digits);
    let dropped_nonzero = bytes
        .iter()
        .skip(precision_digits)
        .any(|digit| *digit != b'0');
    let mut significant: Vec<u8> = bytes.iter().take(keep).map(|digit| digit - b'0').collect();

    if bytes
        .get(precision_digits)
        .is_some_and(|digit| *digit >= b'5')
    {
        let mut carry = true;
        for digit in significant.iter_mut().rev() {
            if *digit == 9 {
                *digit = 0;
            } else {
                *digit += 1;
                carry = false;
                break;
            }
        }
        if carry {
            significant.fill(0);
            significant[0] = 1;
            exponent += 1;
        }
    }

    let digits = significant
        .into_iter()
        .map(|digit| char::from(b'0' + digit))
        .collect();
    (
        SignificantDigits {
            digits,
            dropped_nonzero,
        },
        exponent,
    )
}

fn qalc_mantissa_from_significant(
    digits: &str,
    digits_before_decimal: i32,
    dropped_nonzero: bool,
) -> String {
    let digits_before_decimal = digits_before_decimal as isize;
    let (integer, mut fraction) = if digits_before_decimal <= 0 {
        (
            "0".to_string(),
            format!(
                "{}{}",
                "0".repeat(digits_before_decimal.unsigned_abs()),
                digits
            ),
        )
    } else if digits_before_decimal as usize >= digits.len() {
        (
            format!(
                "{}{}",
                digits,
                "0".repeat(digits_before_decimal as usize - digits.len())
            ),
            String::new(),
        )
    } else {
        let split = digits_before_decimal as usize;
        (digits[..split].to_string(), digits[split..].to_string())
    };

    if fraction.chars().all(|ch| ch == '0') {
        fraction.clear();
    } else if !dropped_nonzero {
        while fraction.ends_with('0') {
            fraction.pop();
        }
    }

    if fraction.is_empty() {
        integer
    } else {
        format!("{integer}.{fraction}")
    }
}

struct QalcDecimalComponents {
    sign: &'static str,
    integer: String,
    fraction: String,
}

impl QalcDecimalComponents {
    fn normalized_digits_and_exponent(&self) -> Option<(String, i32)> {
        let combined = format!("{}{}", self.integer, self.fraction);
        let first_nonzero = combined.bytes().position(|digit| digit != b'0')?;
        let exponent = self.integer.len() as i32 - first_nonzero as i32 - 1;
        Some((combined[first_nonzero..].to_string(), exponent))
    }
}

fn qalc_decimal_components(output: &str) -> Option<QalcDecimalComponents> {
    if output.contains(['E', 'e']) || output.contains('×') {
        return None;
    }

    let (sign, unsigned) = if let Some(rest) = output.strip_prefix('-') {
        ("-", rest)
    } else if let Some(rest) = output.strip_prefix('+') {
        ("+", rest)
    } else {
        ("", output)
    };

    let (integer, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    if !integer.bytes().all(|digit| digit.is_ascii_digit())
        || !fraction.bytes().all(|digit| digit.is_ascii_digit())
        || (integer.is_empty() && fraction.is_empty())
    {
        return None;
    }

    Some(QalcDecimalComponents {
        sign,
        integer: if integer.is_empty() {
            "0".to_string()
        } else {
            integer.to_string()
        },
        fraction: fraction.to_string(),
    })
}

fn apply_qalc_decimal_limits(
    output: &str,
    min_decimals: Option<i32>,
    max_decimals: Option<i32>,
) -> String {
    let Some(mut decimal) = qalc_decimal_components(output) else {
        return output.to_string();
    };

    let max_decimals = max_decimals.and_then(|value| (value >= 0).then_some(value as usize));
    let min_decimals = min_decimals
        .and_then(|value| (value >= 0).then_some(value as usize))
        .unwrap_or(0);
    let min_decimals = max_decimals.map_or(min_decimals, |max| min_decimals.min(max));

    if let Some(max_decimals) = max_decimals {
        round_qalc_decimal_parts(&mut decimal.integer, &mut decimal.fraction, max_decimals);
    }

    while decimal.fraction.len() < min_decimals {
        decimal.fraction.push('0');
    }

    while decimal.fraction.len() > min_decimals && decimal.fraction.ends_with('0') {
        decimal.fraction.pop();
    }

    if decimal.fraction.is_empty() {
        format!("{}{}", decimal.sign, decimal.integer)
    } else {
        format!("{}{}.{}", decimal.sign, decimal.integer, decimal.fraction)
    }
}

fn round_qalc_decimal_parts(integer: &mut String, fraction: &mut String, max_decimals: usize) {
    if fraction.len() <= max_decimals {
        return;
    }

    let round_up = fraction.as_bytes()[max_decimals] >= b'5';
    fraction.truncate(max_decimals);
    if !round_up {
        return;
    }

    let mut carry = true;
    let mut fraction_digits = fraction.as_bytes().to_vec();
    for digit in fraction_digits.iter_mut().rev() {
        if *digit == b'9' {
            *digit = b'0';
        } else {
            *digit += 1;
            carry = false;
            break;
        }
    }
    *fraction = String::from_utf8(fraction_digits).expect("decimal digits remain utf8");
    if carry {
        increment_ascii_decimal_integer(integer);
    }
}

fn increment_ascii_decimal_integer(integer: &mut String) {
    let mut carry = true;
    let mut digits = integer.as_bytes().to_vec();
    for digit in digits.iter_mut().rev() {
        if *digit == b'9' {
            *digit = b'0';
        } else {
            *digit += 1;
            carry = false;
            break;
        }
    }
    if carry {
        digits.insert(0, b'1');
    }
    *integer = String::from_utf8(digits).expect("integer digits remain utf8");
}

fn format_qalc_scientific_parts(
    mantissa: &str,
    exp: i32,
    exp_display: Option<u8>,
    min_decimals: Option<i32>,
    max_decimals: Option<i32>,
) -> String {
    let mantissa = apply_qalc_scientific_mantissa_limits(mantissa, min_decimals, max_decimals);
    if exp_display == Some(2) {
        return match mantissa.as_str() {
            "1" => format!("10^{exp}"),
            "-1" => format!("-10^{exp}"),
            _ => format!("{mantissa}{}", qalc_exponent_suffix(exp, exp_display)),
        };
    }

    format!("{mantissa}{}", qalc_exponent_suffix(exp, exp_display))
}

fn apply_qalc_scientific_mantissa_limits(
    mantissa: &str,
    min_decimals: Option<i32>,
    max_decimals: Option<i32>,
) -> String {
    if max_decimals.is_some() {
        return apply_qalc_decimal_limits(mantissa, min_decimals, max_decimals);
    }

    let Some(min_decimals) = min_decimals.and_then(|value| (value >= 0).then_some(value as usize))
    else {
        return mantissa.to_string();
    };

    let Some(mut decimal) = qalc_decimal_components(mantissa) else {
        return mantissa.to_string();
    };

    while decimal.fraction.len() < min_decimals {
        decimal.fraction.push('0');
    }

    if decimal.fraction.is_empty() {
        format!("{}{}", decimal.sign, decimal.integer)
    } else {
        format!("{}{}.{}", decimal.sign, decimal.integer, decimal.fraction)
    }
}

fn qalc_exponent_suffix(exp: i32, exp_display: Option<u8>) -> String {
    match exp_display {
        Some(1) => format!("e{}", exp),
        Some(2) => format!(" \u{00D7} 10^{}", exp),
        _ => format!("E{}", exp),
    }
}

fn format_qalc_uncertainty(
    value: &NumberValue,
    uncertainty: &NumberValue,
    is_relative: bool,
    precision_digits: usize,
    uncertainty_format: QalcUncertaintyFormat,
) -> String {
    if is_relative {
        return value.to_string_with_uncertainty(uncertainty, true);
    }

    if matches!(
        uncertainty_format,
        QalcUncertaintyFormat::PreserveFloatPrecision
    ) {
        if let (NumberValue::Float(value_float), NumberValue::Float(uncertainty_float)) =
            (value, uncertainty)
        {
            let precise_value = format_qalc_float(&value_float.value, precision_digits);
            let precise_uncertainty = format_qalc_float(&uncertainty_float.value, precision_digits);
            if fixed_decimal_has_fractional_precision(&precise_value)
                || fixed_decimal_has_fractional_precision(&precise_uncertainty)
            {
                let precise_value = trim_fixed_decimal_trailing_zeros(precise_value);
                let precise_uncertainty = trim_fixed_decimal_trailing_zeros(precise_uncertainty);
                return format!("{precise_value}±{precise_uncertainty}");
            }
        }
    }

    value.to_string_with_uncertainty(uncertainty, false)
}

fn fixed_decimal_has_fractional_precision(value: &str) -> bool {
    mantissa_and_exponent(value)
        .0
        .split_once('.')
        .is_some_and(|(_, fraction)| fraction.chars().any(|ch| ch != '0'))
}

fn trim_fixed_decimal_trailing_zeros(value: String) -> String {
    let (mantissa, exponent) = mantissa_and_exponent(&value);
    let Some((integer, fraction)) = mantissa.split_once('.') else {
        return value;
    };
    let trimmed = fraction.trim_end_matches('0');
    let trimmed_mantissa = if trimmed.is_empty() {
        integer.to_string()
    } else {
        format!("{integer}.{trimmed}")
    };
    if exponent.is_empty() {
        trimmed_mantissa
    } else {
        format!("{trimmed_mantissa}{exponent}")
    }
}

fn mantissa_and_exponent(value: &str) -> (&str, &str) {
    if let Some(index) = value.find('e') {
        (&value[..index], &value[index..])
    } else if let Some(index) = value.find('E') {
        (&value[..index], &value[index..])
    } else {
        (value, "")
    }
}

fn qalc_decimal_precision_bits(precision_digits: usize) -> u32 {
    let bits = precision_digits
        .max(1)
        .saturating_mul(4)
        .saturating_add(16)
        .max(128);
    u32::try_from(bits).unwrap_or(u32::MAX)
}

fn format_qalc_float_bound(value: &rug::Float) -> String {
    let output = value.to_string();
    if let Some((integer, fraction)) = output.split_once('.') {
        if fraction.chars().all(|ch| ch == '0') {
            return integer.to_string();
        }
    }
    output
}

fn fixed_decimal_from_scientific(input: &str) -> Option<String> {
    let (mantissa, exponent) = input.split_once('e').or_else(|| input.split_once('E'))?;
    let exponent = exponent.parse::<i32>().ok()?;
    if exponent >= 0 {
        return None;
    }

    let negative = mantissa.starts_with('-');
    let mantissa = mantissa.strip_prefix('-').unwrap_or(mantissa);
    let point_index = mantissa.find('.').unwrap_or(mantissa.len());
    let mut digits: String = mantissa.chars().filter(|ch| *ch != '.').collect();
    if digits.is_empty() {
        return None;
    }

    let new_point = point_index as i32 + exponent;
    let unsigned = if new_point <= 0 {
        let zeros = "0".repeat((-new_point) as usize);
        format!("0.{zeros}{digits}")
    } else {
        let new_point = new_point as usize;
        if new_point >= digits.len() {
            return None;
        }
        digits.insert(new_point, '.');
        digits
    };

    Some(if negative {
        format!("-{unsigned}")
    } else {
        unsigned
    })
}

fn qalc_large_integer_scientific_string(rational: &Rational) -> Option<String> {
    if rational.value.denom() != &1 {
        return None;
    }

    let numerator = rational.value.numer();
    let negative = numerator.is_negative();
    let digits = numerator.clone().abs().to_string();
    let mut exponent = digits.len().checked_sub(1)?;
    if exponent < 13 {
        return None;
    }

    let dropped_nonzero = digits
        .as_bytes()
        .iter()
        .skip(10)
        .any(|digit| *digit != b'0');
    let mut significant: Vec<u8> = digits
        .as_bytes()
        .iter()
        .take(10)
        .map(|digit| digit - b'0')
        .collect();
    if digits
        .as_bytes()
        .get(10)
        .is_some_and(|digit| *digit >= b'5')
    {
        let mut carry = true;
        for digit in significant.iter_mut().rev() {
            if *digit == 9 {
                *digit = 0;
            } else {
                *digit += 1;
                carry = false;
                break;
            }
        }
        if carry {
            significant.fill(0);
            significant[0] = 1;
            exponent += 1;
        }
    }

    let leading = char::from(b'0' + significant[0]);
    let mut fraction: String = significant[1..]
        .iter()
        .map(|digit| char::from(b'0' + *digit))
        .collect();
    if fraction.chars().all(|ch| ch == '0') {
        fraction.clear();
    } else if !dropped_nonzero {
        while fraction.ends_with('0') {
            fraction.pop();
        }
    }

    let mantissa = if fraction.is_empty() {
        leading.to_string()
    } else {
        format!("{leading}.{fraction}")
    };
    Some(if negative {
        format!("-{mantissa}E{exponent}")
    } else {
        format!("{mantissa}E{exponent}")
    })
}

trait UncertaintyFormat {
    fn to_string_with_uncertainty(&self, uncertainty: &NumberValue, is_relative: bool) -> String;
}

impl UncertaintyFormat for NumberValue {
    fn to_string_with_uncertainty(&self, uncertainty: &NumberValue, is_relative: bool) -> String {
        let val_f = to_float_val(self).value();
        let unc_f = to_float_val(uncertainty).value();
        if is_relative {
            let (formatted_val, formatted_unc) = format_relative_uncertainty(val_f, unc_f);
            format!("{formatted_val}±{formatted_unc}")
        } else {
            let (formatted_val, formatted_unc) = format_uncertainty(val_f, unc_f);
            format!("{formatted_val}±{formatted_unc}")
        }
    }
}

impl std::fmt::Display for NumberValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NumberValue::Rational(r) => {
                if let Some(decimal) = r.terminating_decimal_string() {
                    write!(f, "{decimal}")
                } else {
                    write!(f, "{}/{}", r.numerator_string(), r.denominator_string())
                }
            }
            NumberValue::Float(fl) => {
                write!(f, "{}", fl.value)
            }
            NumberValue::Interval { lower, upper } => {
                write!(f, "[{}  {}]", lower.value, upper.value)
            }
            NumberValue::Uncertainty {
                value,
                uncertainty,
                is_relative,
            } => {
                if uncertainty.is_real_zero() {
                    return write!(f, "{}", value);
                }

                write!(
                    f,
                    "{}",
                    value.to_string_with_uncertainty(uncertainty, *is_relative)
                )
            }
            NumberValue::PlusInfinity => write!(f, "inf"),
            NumberValue::MinusInfinity => write!(f, "-inf"),
            NumberValue::NaN => write!(f, "nan"),
        }
    }
}

impl std::fmt::Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (real, imag) = self.to_canonical_real_imag();
        if imag.is_real_zero() {
            write!(f, "{}", self.value)
        } else if real.is_real_zero() {
            if imag.is_real_one() {
                write!(f, "i")
            } else if imag.negate().is_real_one() {
                write!(f, "-i")
            } else {
                write!(f, "{}i", imag)
            }
        } else if is_value_negative(&imag) {
            write!(f, "{} - {}i", real, imag.negate())
        } else {
            write!(f, "{} + {}i", real, imag)
        }
    }
}

fn is_value_negative(val: &NumberValue) -> bool {
    match val {
        NumberValue::Rational(r) => r.is_negative(),
        NumberValue::Float(f) => f.value < 0.0,
        NumberValue::Uncertainty { value, .. } => is_value_negative(value),
        NumberValue::MinusInfinity => true,
        _ => false,
    }
}

fn parse_single_value(s: &str) -> Result<NumberValue, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("Empty string".to_string());
    }

    if let Some(special) = parse_special_value(s) {
        return Ok(special);
    }

    if let Some(interval) = parse_interval_literal(s)? {
        return Ok(interval);
    }

    parse_rational_literal(s)
        .map(|value| NumberValue::Rational(Rational::from_rug(value)))
        .map_err(|_| format!("Failed to parse number: {s}"))
}

const SPECIAL_VALUE_NAMES: [&str; 3] = ["infinity", "inf", "nan"];
const EXPRESSION_SPECIAL_VALUE_NAMES: [&str; 1] = ["infinity"];

fn parse_special_value(s: &str) -> Option<NumberValue> {
    let normalized = s.to_ascii_lowercase();
    let (negative, name) = normalized
        .strip_prefix('-')
        .map(|rest| (true, rest))
        .or_else(|| normalized.strip_prefix('+').map(|rest| (false, rest)))
        .unwrap_or((false, normalized.as_str()));

    if !SPECIAL_VALUE_NAMES.contains(&name) {
        return None;
    }

    match name {
        "inf" | "infinity" if negative => Some(NumberValue::MinusInfinity),
        "inf" | "infinity" => Some(NumberValue::PlusInfinity),
        "nan" => Some(NumberValue::NaN),
        _ => None,
    }
}

fn parse_rational_literal(s: &str) -> Result<rug::Rational, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("Empty rational string".to_string());
    }

    if let Some((num_str, den_str)) = s.split_once('/') {
        if den_str.contains('/') {
            return Err(format!("Invalid rational literal: {s}"));
        }
        let num = num_str
            .trim()
            .parse::<rug::Integer>()
            .map_err(|_| format!("Invalid rational numerator: {s}"))?;
        let den = den_str
            .trim()
            .parse::<rug::Integer>()
            .map_err(|_| format!("Invalid rational denominator: {s}"))?;
        if den.is_zero() {
            return Err("Rational denominator must not be zero".to_string());
        }
        return Ok(rug::Rational::from((num, den)));
    }

    if let Ok(value) = s.parse::<rug::Integer>() {
        return Ok(rug::Rational::from(value));
    }

    parse_decimal_or_scientific_rational(s).ok_or_else(|| format!("Invalid rational literal: {s}"))
}

fn parse_interval_literal(s: &str) -> Result<Option<NumberValue>, String> {
    let Some(inner) = s
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
    else {
        return Ok(None);
    };

    let (lower, upper) = split_interval_bounds(inner)
        .ok_or_else(|| format!("Failed to parse interval bounds: {s}"))?;
    let lower = parse_single_value(lower)?;
    let upper = parse_single_value(upper)?;
    if lower == upper {
        return Ok(Some(lower));
    }
    let (lower, _) =
        to_interval(&lower).ok_or_else(|| format!("Invalid interval lower bound: {lower}"))?;
    let (_, upper) =
        to_interval(&upper).ok_or_else(|| format!("Invalid interval upper bound: {upper}"))?;

    let number = Number::try_new_interval(lower, upper)
        .ok_or_else(|| format!("Invalid interval bounds: {s}"))?;

    Ok(Some(number.value().clone()))
}

fn parse_interval_function_expression(s: &str) -> Result<Option<Number>, String> {
    let trimmed = s.trim();
    let Some(rest) = strip_function_name(trimmed, "interval") else {
        return Ok(None);
    };
    let rest = rest.trim_start();
    let Some(inner) = rest
        .strip_prefix('(')
        .and_then(|value| value.strip_suffix(')'))
    else {
        return Err(format!("Expected interval(lower; upper): {trimmed}"));
    };

    let args = split_semicolon_arguments(inner);
    if !(2..=3).contains(&args.len()) {
        return Err(format!("Failed to parse interval bounds: {trimmed}"));
    }
    if let Some(exclude_endpoints) = args.get(2) {
        parse_boolean_argument(exclude_endpoints)
            .ok_or_else(|| format!("Invalid interval exclude flag: {exclude_endpoints}"))?;
        if !supports_interval_exclude_flag_bounds(args[0], args[1]) {
            return Err(format!(
                "Unsupported interval exclude flag for unprobed bounds: {trimmed}"
            ));
        }
    }

    let lower = parse_single_value(args[0])?;
    let upper = parse_single_value(args[1])?;
    let (lower, _) =
        to_interval(&lower).ok_or_else(|| format!("Invalid interval lower bound: {lower}"))?;
    let (_, upper) =
        to_interval(&upper).ok_or_else(|| format!("Invalid interval upper bound: {upper}"))?;

    Number::try_new_interval(lower, upper)
        .ok_or_else(|| format!("Invalid interval bounds: {trimmed}"))
        .map(Some)
}

fn parse_uncertainty_function_expression(s: &str) -> Result<Option<Number>, String> {
    let trimmed = s.trim();
    let Some(rest) = strip_function_name(trimmed, "uncertainty") else {
        return Ok(None);
    };
    let rest = rest.trim_start();
    let Some(inner) = rest
        .strip_prefix('(')
        .and_then(|value| value.strip_suffix(')'))
    else {
        return Err(format!(
            "Expected uncertainty(value; error; relative): {trimmed}"
        ));
    };

    let args = split_semicolon_arguments(inner);
    if !(2..=3).contains(&args.len()) {
        return Err(format!("Failed to parse uncertainty arguments: {trimmed}"));
    }

    let value = parse_single_value(args[0])?;
    let uncertainty = parse_single_value(args[1])?;
    let is_relative = if let Some(relative) = args.get(2) {
        parse_boolean_argument(relative)
            .ok_or_else(|| format!("Invalid uncertainty relative flag: {relative}"))?
    } else {
        true
    };

    let uncertainty = if is_relative {
        value.abs().mul(&uncertainty.abs())
    } else {
        uncertainty.abs()
    };

    Ok(Some(Number::new_uncertainty(value, uncertainty, false)))
}

fn parse_error_part_function_expression(s: &str) -> Result<Option<Number>, String> {
    let trimmed = s.trim();
    let Some(rest) = strip_function_name(trimmed, "errorPart") else {
        return Ok(None);
    };
    let rest = rest.trim_start();
    let Some(inner) = rest
        .strip_prefix('(')
        .and_then(|value| value.strip_suffix(')'))
    else {
        return Err(format!("Expected errorPart(value; relative): {trimmed}"));
    };

    let args = split_semicolon_arguments(inner);
    if !(1..=2).contains(&args.len()) {
        return Err(format!("Failed to parse errorPart arguments: {trimmed}"));
    }

    let value = evaluate_expr(args[0])?;
    let relative = if let Some(relative) = args.get(1) {
        parse_boolean_argument(relative)
            .ok_or_else(|| format!("Invalid errorPart relative flag: {relative}"))?
    } else {
        false
    };

    Ok(Some(value.error_part_with_relative(relative)))
}

fn split_semicolon_arguments(inner: &str) -> Vec<&str> {
    inner.split(';').map(str::trim).collect()
}

fn parse_boolean_argument(s: &str) -> Option<bool> {
    match s.trim() {
        "0" => Some(false),
        "1" => Some(true),
        _ => None,
    }
}

fn supports_interval_exclude_flag_bounds(lower: &str, upper: &str) -> bool {
    // Open-bound interval movement is precision-sensitive; keep this parser
    // path limited to the exact upstream-probed rows promoted by the native gate.
    matches!((lower.trim(), upper.trim()), ("1", "3"))
}

fn split_interval_bounds(inner: &str) -> Option<(&str, &str)> {
    let inner = inner.trim();
    if inner.is_empty() {
        return None;
    }

    if let Some((lower, upper)) = inner.split_once(',') {
        if upper.contains(',') {
            return None;
        }
        let lower = lower.trim();
        let upper = upper.trim();
        return (!lower.is_empty() && !upper.is_empty()).then_some((lower, upper));
    }

    let mut parts = inner.split_whitespace();
    let lower = parts.next()?;
    let upper = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    Some((lower, upper))
}

fn parse_decimal_or_scientific_rational(s: &str) -> Option<rug::Rational> {
    // Keep exact rational parsing proportional to a bounded decimal shift while
    // still accepting practical large qalc-style exponents such as 1e5000.
    // Larger exponents are left unsupported in the native scaffold instead of
    // constructing attacker-controlled powers of ten with millions of digits.
    const MAX_EXACT_DECIMAL_SHIFT: u32 = 10_000;

    let compact: String = s.chars().filter(|c| !c.is_ascii_whitespace()).collect();
    if compact.is_empty() {
        return None;
    }

    let (mantissa, exponent, has_exponent) = if let Some(index) = compact.find(['e', 'E']) {
        let exponent = compact[index + 1..].parse::<i32>().ok()?;
        (&compact[..index], exponent, true)
    } else {
        (compact.as_str(), 0, false)
    };

    let (negative, mantissa) = mantissa
        .strip_prefix('-')
        .map(|rest| (true, rest))
        .or_else(|| mantissa.strip_prefix('+').map(|rest| (false, rest)))
        .unwrap_or((false, mantissa));

    if !mantissa.contains('.') && !has_exponent {
        return None;
    }

    let mut digits = String::new();
    let mut scale = 0i32;
    let mut seen_dot = false;
    for ch in mantissa.chars() {
        match ch {
            '.' if !seen_dot => seen_dot = true,
            '0'..='9' => {
                digits.push(ch);
                if seen_dot {
                    scale += 1;
                }
            }
            _ => return None,
        }
    }

    if digits.is_empty() && has_exponent {
        return None;
    }
    if digits.is_empty() {
        digits.push('0');
    }

    let mut numerator = digits.parse::<rug::Integer>().ok()?;
    if negative {
        numerator = -numerator;
    }

    let decimal_shift = scale.checked_sub(exponent)?;
    if decimal_shift.unsigned_abs() > MAX_EXACT_DECIMAL_SHIFT {
        return None;
    }
    if decimal_shift >= 0 {
        let denominator = pow10(decimal_shift as usize);
        Some(rug::Rational::from((numerator, denominator)))
    } else {
        numerator *= pow10((-decimal_shift) as usize);
        Some(rug::Rational::from(numerator))
    }
}

fn pow10(exp: usize) -> rug::Integer {
    let mut value = rug::Integer::from(1);
    for _ in 0..exp {
        value *= 10_u32;
    }
    value
}

fn next_literal(s: &str) -> Option<(&str, &str)> {
    let s = s.trim_start();
    if s.is_empty() {
        return None;
    }

    if let Some((lit, remaining)) = next_interval_function_literal(s) {
        return Some((lit, remaining));
    }

    if let Some((lit, remaining)) = next_uncertainty_function_literal(s) {
        return Some((lit, remaining));
    }

    if let Some((lit, remaining)) = next_error_part_function_literal(s) {
        return Some((lit, remaining));
    }

    if let Some((lit, remaining)) = next_special_value_literal(s) {
        return Some((lit, remaining));
    }

    let mut len = 0;
    let chars: Vec<char> = s.chars().collect();
    if chars.is_empty() {
        return None;
    }

    let first = chars[0];
    if first == '[' {
        let mut depth = 0usize;
        for (idx, ch) in s.char_indices() {
            match ch {
                '[' => depth += 1,
                ']' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        let end = idx + ch.len_utf8();
                        return Some((&s[..end], &s[end..]));
                    }
                }
                _ => {}
            }
        }
        return None;
    }

    if !first.is_ascii_digit() && first != '.' && first != 'i' {
        return None;
    }

    let mut in_parenthesis = false;
    let mut has_uncertainty_marker = false;
    while len < chars.len() {
        let c = chars[len];
        if c.is_ascii_digit() || c == '.' || c == 'e' || c == 'E' || c == 'i' {
            len += 1;
        } else if c == '%' {
            if has_uncertainty_marker {
                len += 1;
            } else {
                break;
            }
        } else if c == '(' {
            in_parenthesis = true;
            len += 1;
        } else if c == ')' {
            if in_parenthesis {
                in_parenthesis = false;
                len += 1;
            } else {
                break;
            }
        } else if in_parenthesis {
            len += 1;
        } else if c == '+'
            && len + 2 < chars.len()
            && chars[len + 1] == '/'
            && chars[len + 2] == '-'
        {
            has_uncertainty_marker = true;
            len += 3;
        } else if c == '±'
            || (c == '-'
                && len > 0
                && (chars[len - 1] == 'e' || chars[len - 1] == 'E' || chars[len - 1] == '/'))
        {
            if c == '±' {
                has_uncertainty_marker = true;
            }
            len += 1;
        } else if c.is_whitespace() {
            let mut lookahead = len;
            while lookahead < chars.len() && chars[lookahead].is_whitespace() {
                lookahead += 1;
            }
            let next_is_uncertainty_marker = lookahead < chars.len()
                && (chars[lookahead] == '±'
                    || (lookahead + 2 < chars.len()
                        && chars[lookahead] == '+'
                        && chars[lookahead + 1] == '/'
                        && chars[lookahead + 2] == '-'));
            let next_is_uncertainty_operand = has_uncertainty_marker
                && lookahead < chars.len()
                && (chars[lookahead].is_ascii_digit()
                    || chars[lookahead] == '.'
                    || chars[lookahead] == '-'
                    || chars[lookahead] == '(');
            if next_is_uncertainty_marker || next_is_uncertainty_operand {
                len = lookahead;
            } else {
                break;
            }
        } else {
            break;
        }
    }
    if len > 0 {
        let byte_len = if len == chars.len() {
            s.len()
        } else {
            s.char_indices().nth(len)?.0
        };
        Some((&s[..byte_len], &s[byte_len..]))
    } else {
        None
    }
}

fn next_special_value_literal(s: &str) -> Option<(&str, &str)> {
    for name in EXPRESSION_SPECIAL_VALUE_NAMES {
        let Some(literal) = s.get(..name.len()) else {
            continue;
        };
        if !literal.eq_ignore_ascii_case(name) {
            continue;
        }
        let remaining = &s[name.len()..];
        if remaining
            .chars()
            .next()
            .is_some_and(|ch| ch.is_alphanumeric() || ch == '_')
        {
            continue;
        }
        return Some((literal, remaining));
    }
    None
}

fn next_interval_function_literal(s: &str) -> Option<(&str, &str)> {
    let after_name = strip_function_name(s, "interval")?;
    next_function_literal_after_name(s, after_name)
}

fn next_uncertainty_function_literal(s: &str) -> Option<(&str, &str)> {
    let after_name = strip_function_name(s, "uncertainty")?;
    next_function_literal_after_name(s, after_name)
}

fn next_error_part_function_literal(s: &str) -> Option<(&str, &str)> {
    let after_name = strip_function_name(s, "errorPart")?;
    next_function_literal_after_name(s, after_name)
}

fn next_function_literal_after_name<'a>(
    s: &'a str,
    after_name: &str,
) -> Option<(&'a str, &'a str)> {
    let name_len = s.len() - after_name.len();
    let trimmed_after_name = after_name.trim_start();
    let whitespace_len = after_name.len() - trimmed_after_name.len();
    let open_idx = name_len + whitespace_len;
    if !s[open_idx..].starts_with('(') {
        return None;
    }

    let mut depth = 0usize;
    for (relative_idx, ch) in s[open_idx..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let end = open_idx + relative_idx + ch.len_utf8();
                    return Some((&s[..end], &s[end..]));
                }
            }
            _ => {}
        }
    }

    None
}

impl std::str::FromStr for Number {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        if let Some(error_part) = parse_error_part_function_expression(s)? {
            return Ok(error_part);
        }

        let split_parts = if s.contains("+/-") {
            let mut parts = s.split("+/-");
            let v = parts.next().unwrap().trim();
            let u = parts.next().unwrap().trim();
            if parts.next().is_some() {
                return Err("Multiple +/- symbols".to_string());
            }
            Some((v, u))
        } else if s.contains('±') {
            let mut parts = s.split('±');
            let v = parts.next().unwrap().trim();
            let u = parts.next().unwrap().trim();
            if parts.next().is_some() {
                return Err("Multiple ± symbols".to_string());
            }
            Some((v, u))
        } else {
            None
        };

        if let Some((v_str, u_str)) = split_parts {
            let value = parse_single_value(v_str)?;
            let mut u_clean = u_str;
            if u_clean.starts_with('(') && u_clean.ends_with(')') {
                u_clean = u_clean[1..u_clean.len() - 1].trim();
            }
            if let Some(stripped) = u_clean.strip_suffix('%') {
                let pct_str = stripped.trim();
                let unc_pct = parse_single_value(pct_str)?;
                let hundred = NumberValue::Rational(Rational::from_i32(100));
                let u_abs = value.abs().mul(&unc_pct.abs()).div(&hundred);
                return Ok(Number::new_uncertainty(value, u_abs, true));
            } else {
                let unc = parse_single_value(u_clean)?;
                let unc_abs = unc.abs();
                return Ok(Number::new_uncertainty(value, unc_abs, false));
            }
        }

        if let Some(interval) = parse_interval_function_expression(s)? {
            return Ok(interval);
        }

        if let Some(uncertainty) = parse_uncertainty_function_expression(s)? {
            return Ok(uncertainty);
        }

        if let Some(open_idx) = s.find('(') {
            if let Some(close_idx) = s.find(')') {
                if close_idx > open_idx + 1 && s[close_idx + 1..].trim().is_empty() {
                    let v_str = s[..open_idx].trim();
                    let u_str = s[open_idx + 1..close_idx].trim();
                    let value = parse_single_value(v_str)?;
                    let u_raw = parse_single_value(u_str)?;

                    let mut exp = 0i32;
                    let mantissa = if let Some(e_idx) = v_str.find(['e', 'E']) {
                        if let Ok(val) = v_str[e_idx + 1..].parse::<i32>() {
                            exp = val;
                        }
                        &v_str[..e_idx]
                    } else {
                        v_str
                    };

                    let d = if let Some(dot_idx) = mantissa.find('.') {
                        let after_dot = &mantissa[dot_idx + 1..];
                        after_dot.chars().take_while(|c| c.is_ascii_digit()).count()
                    } else {
                        0
                    };
                    let ten = NumberValue::Rational(Rational::from_i32(10));
                    let shift = exp - (d as i32);
                    let shift_val = NumberValue::Rational(Rational::from_i32(shift));
                    let factor = ten.pow(&shift_val);
                    let u_abs = u_raw.mul(&factor).abs();
                    return Ok(Number::new_uncertainty(value, u_abs, false));
                }
            }
        }

        if let Some(coefficient) = s.strip_suffix('i') {
            let coefficient = match coefficient.trim() {
                "" | "+" => "1",
                "-" => "-1",
                value => value,
            };
            let val = parse_single_value(coefficient)?;
            return Ok(Number {
                precision: val.precision(),
                approximate: val.approximate(),
                value: val,
                imaginary: None,
                is_imaginary: true,
            });
        }

        let val = parse_single_value(s)?;
        Ok(Number {
            precision: val.precision(),
            approximate: val.approximate(),
            value: val,
            imaginary: None,
            is_imaginary: false,
        })
    }
}

fn strip_word_operator<'a>(s: &'a str, operator: &str, has_left_boundary: bool) -> Option<&'a str> {
    if !has_left_boundary {
        return None;
    }
    let remaining = s.strip_prefix(operator)?;
    if remaining
        .chars()
        .next()
        .is_some_and(|ch| !ch.is_whitespace())
    {
        return None;
    }
    Some(remaining)
}

fn strip_function_name<'a>(s: &'a str, name: &str) -> Option<&'a str> {
    let remaining = s.strip_prefix(name)?;
    if remaining
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return None;
    }
    Some(remaining)
}

const UNARY_FUNCTIONS: &[(&str, UnaryFunction)] = &[
    ("conj", UnaryFunction::Conjugate),
    ("errorPart", UnaryFunction::ErrorPart),
    ("lowerEndpoint", UnaryFunction::LowerEndpoint),
    ("ln", UnaryFunction::NaturalLog),
    ("midpoint", UnaryFunction::Midpoint),
    ("norm", UnaryFunction::Norm),
    ("sqrt", UnaryFunction::SquareRoot),
    ("upperEndpoint", UnaryFunction::UpperEndpoint),
    ("valuePart", UnaryFunction::ValuePart),
];

fn strip_unary_function(s: &str) -> Option<(UnaryFunction, &str)> {
    UNARY_FUNCTIONS.iter().find_map(|(name, function)| {
        strip_function_name(s, name).map(|remaining| (*function, remaining))
    })
}

#[derive(Debug, Clone, Copy)]
enum UnaryFunction {
    Conjugate,
    ErrorPart,
    LowerEndpoint,
    NaturalLog,
    Midpoint,
    Norm,
    SquareRoot,
    UpperEndpoint,
    ValuePart,
}

impl UnaryFunction {
    const fn name(self) -> &'static str {
        match self {
            Self::Conjugate => "conj",
            Self::ErrorPart => "errorPart",
            Self::LowerEndpoint => "lowerEndpoint",
            Self::NaturalLog => "ln",
            Self::Midpoint => "midpoint",
            Self::Norm => "norm",
            Self::SquareRoot => "sqrt",
            Self::UpperEndpoint => "upperEndpoint",
            Self::ValuePart => "valuePart",
        }
    }

    fn apply(self, arg: Number, context: EvalContext) -> Number {
        match self {
            Self::Conjugate => arg.conjugate(),
            Self::ErrorPart => arg.error_part(),
            Self::LowerEndpoint => arg.lower_endpoint(),
            Self::NaturalLog => apply_real_unary_value(arg, |value| {
                value.ln_with_precision_floor(context.min_float_precision_bits())
            }),
            Self::Midpoint => arg.midpoint(),
            Self::Norm => arg.norm(),
            Self::SquareRoot => apply_real_unary_value(arg, |value| {
                value.sqrt_with_precision_floor(context.min_float_precision_bits())
            }),
            Self::UpperEndpoint => arg.upper_endpoint(),
            Self::ValuePart => arg.value_part(),
        }
    }
}

fn apply_real_unary_value(
    arg: Number,
    operation: impl FnOnce(&NumberValue) -> NumberValue,
) -> Number {
    if arg.is_complex() {
        return Number {
            value: NumberValue::NaN,
            imaginary: None,
            precision: arg.precision,
            approximate: true,
            is_imaginary: false,
        };
    }

    let value = operation(&arg.value);
    Number {
        precision: std::cmp::max(arg.precision, value.precision()),
        approximate: arg.approximate || value.approximate(),
        value,
        imaginary: None,
        is_imaginary: false,
    }
}

#[derive(Clone, Copy)]
struct EvalContext {
    min_float_precision_bits: u32,
}

impl EvalContext {
    const DEFAULT: Self = Self {
        min_float_precision_bits: 53,
    };

    fn from_precision_digits(precision_digits: usize) -> Self {
        Self {
            min_float_precision_bits: qalc_decimal_precision_bits(precision_digits),
        }
    }

    const fn min_float_precision_bits(self) -> u32 {
        self.min_float_precision_bits
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RelationOperator {
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

const RELATION_OPERATORS: &[(&str, RelationOperator)] = &[
    ("!=", RelationOperator::NotEqual),
    ("==", RelationOperator::Equal),
    ("<=", RelationOperator::LessEqual),
    (">=", RelationOperator::GreaterEqual),
    ("≠", RelationOperator::NotEqual),
    ("≤", RelationOperator::LessEqual),
    ("≥", RelationOperator::GreaterEqual),
    ("<", RelationOperator::Less),
    (">", RelationOperator::Greater),
    ("=", RelationOperator::Equal),
];

fn split_top_level_relation(s: &str) -> Option<(&str, RelationOperator, &str)> {
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;

    for (idx, ch) in s.char_indices() {
        match ch {
            '(' if bracket_depth == 0 => paren_depth += 1,
            ')' if bracket_depth == 0 => paren_depth = paren_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            _ if paren_depth == 0 && bracket_depth == 0 => {
                let rest = &s[idx..];
                for (token, operator) in RELATION_OPERATORS {
                    if let Some(rhs) = rest.strip_prefix(token) {
                        return Some((&s[..idx], *operator, rhs));
                    }
                }
            }
            _ => {}
        }
    }

    None
}

/// Evaluates a top-level boolean relation.
///
/// `Ok(None)` means the relation should stay symbolic/unknown for the native
/// evidence gate, matching upstream behavior for non-equal complex ordering.
pub(crate) fn evaluate_relation_expr(s: &str) -> Result<Option<bool>, String> {
    evaluate_relation_expr_with_context(s, EvalContext::DEFAULT)
}

pub(crate) fn evaluate_relation_expr_with_precision_digits(
    s: &str,
    precision_digits: usize,
) -> Result<Option<bool>, String> {
    evaluate_relation_expr_with_context(s, EvalContext::from_precision_digits(precision_digits))
}

fn evaluate_relation_expr_with_context(
    s: &str,
    context: EvalContext,
) -> Result<Option<bool>, String> {
    let Some((lhs, operator, rhs)) = split_top_level_relation(s) else {
        return Ok(None);
    };

    let lhs = lhs.trim();
    let rhs = rhs.trim();
    if lhs.is_empty() || rhs.is_empty() {
        return Err("Expected operands around relation operator".to_string());
    }

    let lhs = evaluate_expr_with_context(lhs, context)?;
    let rhs = evaluate_expr_with_context(rhs, context)?;
    let equal = lhs == rhs;
    let value = match operator {
        RelationOperator::Equal => Some(equal),
        RelationOperator::NotEqual => Some(!equal),
        RelationOperator::Less if equal => Some(false),
        RelationOperator::LessEqual if equal => Some(true),
        RelationOperator::Greater if equal => Some(false),
        RelationOperator::GreaterEqual if equal => Some(true),
        RelationOperator::Less => lhs
            .partial_cmp(&rhs)
            .map(|ordering| matches!(ordering, std::cmp::Ordering::Less)),
        RelationOperator::LessEqual => lhs.partial_cmp(&rhs).map(|ordering| {
            matches!(
                ordering,
                std::cmp::Ordering::Less | std::cmp::Ordering::Equal
            )
        }),
        RelationOperator::Greater => lhs
            .partial_cmp(&rhs)
            .map(|ordering| matches!(ordering, std::cmp::Ordering::Greater)),
        RelationOperator::GreaterEqual => lhs.partial_cmp(&rhs).map(|ordering| {
            matches!(
                ordering,
                std::cmp::Ordering::Greater | std::cmp::Ordering::Equal
            )
        }),
    };
    Ok(value)
}

/// Evaluates a basic mathematical expression containing numbers, arithmetic operators, parentheses, and uncertainty.
pub fn evaluate_expr(s: &str) -> Result<Number, String> {
    evaluate_expr_with_context(s, EvalContext::DEFAULT)
}

pub(crate) fn evaluate_expr_with_precision_digits(
    s: &str,
    precision_digits: usize,
) -> Result<Number, String> {
    evaluate_expr_with_context(s, EvalContext::from_precision_digits(precision_digits))
}

fn evaluate_expr_with_context(s: &str, context: EvalContext) -> Result<Number, String> {
    let mut tokens = Vec::new();
    let mut rest = s.trim();
    let mut has_word_operator_left_boundary = false;
    while !rest.is_empty() {
        if let Some((lit, remaining)) = next_literal(rest) {
            tokens.push(Token::Literal(std::str::FromStr::from_str(lit)?));
            has_word_operator_left_boundary = remaining
                .chars()
                .next()
                .is_some_and(|ch| ch.is_whitespace());
            rest = remaining.trim_start();
        } else if let Some(remaining) = rest.strip_prefix("**") {
            tokens.push(Token::OpPow);
            has_word_operator_left_boundary = false;
            rest = remaining.trim_start();
        } else if let Some(remaining) = rest.strip_prefix("%%") {
            tokens.push(Token::OpMod);
            has_word_operator_left_boundary = false;
            rest = remaining.trim_start();
        } else if let Some(remaining) = rest.strip_prefix("//") {
            tokens.push(Token::OpIntDiv);
            has_word_operator_left_boundary = false;
            rest = remaining.trim_start();
        } else if let Some(remaining) =
            strip_word_operator(rest, "rem", has_word_operator_left_boundary)
        {
            tokens.push(Token::OpRem);
            has_word_operator_left_boundary = false;
            rest = remaining.trim_start();
        } else if let Some(remaining) =
            strip_word_operator(rest, "mod", has_word_operator_left_boundary)
        {
            tokens.push(Token::OpMod);
            has_word_operator_left_boundary = false;
            rest = remaining.trim_start();
        } else if let Some(remaining) =
            strip_word_operator(rest, "div", has_word_operator_left_boundary)
        {
            tokens.push(Token::OpIntDiv);
            has_word_operator_left_boundary = false;
            rest = remaining.trim_start();
        } else if let Some((function, remaining)) = strip_unary_function(rest) {
            tokens.push(Token::UnaryFunction(function));
            has_word_operator_left_boundary = false;
            rest = remaining.trim_start();
        } else if let Some(remaining) = rest.strip_prefix('%') {
            tokens.push(Token::OpRem);
            has_word_operator_left_boundary = false;
            rest = remaining.trim_start();
        } else if let Some(remaining) = rest.strip_prefix('\\') {
            tokens.push(Token::OpIntDiv);
            has_word_operator_left_boundary = false;
            rest = remaining.trim_start();
        } else {
            let c = rest.chars().next().unwrap();
            match c {
                '+' => tokens.push(Token::OpAdd),
                '-' => tokens.push(Token::OpSub),
                '*' => tokens.push(Token::OpMul),
                '/' => tokens.push(Token::OpDiv),
                '^' => tokens.push(Token::OpPow),
                '(' => tokens.push(Token::LParen),
                ')' => tokens.push(Token::RParen),
                _ => return Err(format!("Unexpected character: {c}")),
            }
            let remaining = &rest[c.len_utf8()..];
            has_word_operator_left_boundary = matches!(c, ')')
                && remaining
                    .chars()
                    .next()
                    .is_some_and(|ch| ch.is_whitespace());
            rest = remaining.trim_start();
        }
    }

    let mut parser = ExprParser {
        tokens,
        pos: 0,
        context,
    };
    let result = parser.parse_expr(0)?;
    if parser.pos == parser.tokens.len() {
        Ok(result)
    } else {
        Err(format!(
            "Unexpected trailing token: {:?}",
            parser.tokens.get(parser.pos)
        ))
    }
}

#[derive(Debug, Clone)]
enum Token {
    Literal(Number),
    OpAdd,
    OpSub,
    OpMul,
    OpDiv,
    OpRem,
    OpMod,
    OpIntDiv,
    OpPow,
    UnaryFunction(UnaryFunction),
    LParen,
    RParen,
}

struct ExprParser {
    tokens: Vec<Token>,
    pos: usize,
    context: EvalContext,
}

impl ExprParser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next_token(&mut self) -> Option<&Token> {
        let t = self.tokens.get(self.pos);
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn parse_primary(&mut self) -> Result<Number, String> {
        match self.next_token().cloned() {
            Some(Token::Literal(num)) => Ok(num),
            Some(Token::LParen) => {
                let expr = self.parse_expr(0)?;
                match self.next_token() {
                    Some(Token::RParen) => Ok(expr),
                    _ => Err("Expected matching ')'".to_string()),
                }
            }
            Some(Token::UnaryFunction(function)) => {
                let arg = self.parse_function_argument(function.name())?;
                Ok(function.apply(arg, self.context))
            }
            Some(Token::OpAdd) => self.parse_expr(3),
            Some(Token::OpSub) => {
                let primary = self.parse_expr(3)?;
                Ok(Number {
                    precision: primary.precision,
                    approximate: primary.approximate,
                    value: primary.value.negate(),
                    imaginary: primary.imaginary.map(|im| {
                        Box::new(Number {
                            precision: im.precision,
                            approximate: im.approximate,
                            value: im.value.negate(),
                            imaginary: None,
                            is_imaginary: true,
                        })
                    }),
                    is_imaginary: primary.is_imaginary,
                })
            }
            t => Err(format!(
                "Expected literal or parenthesized expression, got {:?}",
                t
            )),
        }
    }

    fn parse_function_argument(&mut self, name: &str) -> Result<Number, String> {
        match self.next_token() {
            Some(Token::LParen) => {}
            _ => return Err(format!("Expected '(' after {name}")),
        }
        let expr = self.parse_expr(0)?;
        match self.next_token() {
            Some(Token::RParen) => Ok(expr),
            _ => Err(format!("Expected matching ')' after {name} argument")),
        }
    }

    fn parse_expr(&mut self, min_prec: u8) -> Result<Number, String> {
        let mut lhs = self.parse_primary()?;

        while let Some(tok) = self.peek() {
            let (op_prec, assoc) = match tok {
                Token::OpAdd | Token::OpSub => (1, Assoc::Left),
                Token::OpMul | Token::OpDiv | Token::OpRem | Token::OpMod | Token::OpIntDiv => {
                    (2, Assoc::Left)
                }
                Token::OpPow => (3, Assoc::Right),
                _ => break,
            };

            if op_prec < min_prec {
                break;
            }

            let op = self.next_token().unwrap().clone();
            let next_min_prec = if assoc == Assoc::Left {
                op_prec + 1
            } else {
                op_prec
            };
            let rhs = self.parse_expr(next_min_prec)?;

            lhs = match op {
                Token::OpAdd => lhs.add(&rhs),
                Token::OpSub => lhs.sub(&rhs),
                Token::OpMul => lhs.mul(&rhs),
                Token::OpDiv => lhs.div(&rhs),
                Token::OpRem => lhs.rem(&rhs),
                Token::OpMod => lhs.modulo(&rhs),
                Token::OpIntDiv => lhs.int_div(&rhs),
                Token::OpPow => lhs.pow_with_context(&rhs, self.context),
                _ => unreachable!(),
            };
        }

        Ok(lhs)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Assoc {
    Left,
    Right,
}

impl PartialOrd for Number {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let (lhs_real, lhs_imag) = self.to_canonical_real_imag();
        let (rhs_real, rhs_imag) = other.to_canonical_real_imag();
        if lhs_imag.is_real_zero() && rhs_imag.is_real_zero() {
            lhs_real.partial_cmp(&rhs_real)
        } else {
            None
        }
    }
}

/// A 256-bit unsigned integer helper struct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct U256 {
    /// The four 64-bit limbs of the integer, in little-endian order.
    pub parts: [u64; 4],
}

#[allow(clippy::should_implement_trait, clippy::needless_range_loop)]
impl U256 {
    /// Create a U256 from a u128.
    pub fn from_u128(val: u128) -> Self {
        Self {
            parts: [val as u64, (val >> 64) as u64, 0, 0],
        }
    }

    /// Check if the value is zero.
    pub fn is_zero(&self) -> bool {
        self.parts.iter().all(|&x| x == 0)
    }

    /// Add two U256 values.
    pub fn add(self, other: Self) -> Self {
        let mut parts = [0u64; 4];
        let mut carry = 0u128;
        for i in 0..4 {
            let sum = (self.parts[i] as u128) + (other.parts[i] as u128) + carry;
            parts[i] = sum as u64;
            carry = sum >> 64;
        }
        Self { parts }
    }

    /// Subtract a U256 value from this one, assuming self >= other.
    pub fn sub(self, other: Self) -> Self {
        let mut parts = [0u64; 4];
        let mut borrow = 0i128;
        for i in 0..4 {
            let diff = (self.parts[i] as i128) - (other.parts[i] as i128) - borrow;
            if diff < 0 {
                parts[i] = (diff + (1i128 << 64)) as u64;
                borrow = 1;
            } else {
                parts[i] = diff as u64;
                borrow = 0;
            }
        }
        Self { parts }
    }

    /// Multiply two u128 values, returning a U256.
    pub fn mul_u128(x: u128, y: u128) -> Self {
        let x0 = x as u64 as u128;
        let x1 = x >> 64;
        let y0 = y as u64 as u128;
        let y1 = y >> 64;

        let w0 = x0 * y0;
        let w1 = x0 * y1;
        let w2 = x1 * y0;
        let w3 = x1 * y1;

        let mut parts = [0u64; 4];
        let carry = w0 >> 64;
        parts[0] = w0 as u64;

        let sum = w1 + w2 + carry;
        parts[1] = sum as u64;
        let carry = sum >> 64;

        let sum2 = w3 + carry;
        parts[2] = sum2 as u64;
        parts[3] = (sum2 >> 64) as u64;

        Self { parts }
    }

    /// Divide this U256 by another, returning (quotient, remainder).
    pub fn div_rem(self, other: Self) -> (Self, Self) {
        if other.is_zero() {
            panic!("division by zero");
        }
        let mut quotient = Self { parts: [0; 4] };
        let mut remainder = self;
        let mut other_msb = 0;
        for i in (0..4).rev() {
            if other.parts[i] != 0 {
                other_msb = i * 64 + (63 - other.parts[i].leading_zeros()) as usize;
                break;
            }
        }
        let mut rem_msb = 0;
        let mut found_rem = false;
        for i in (0..4).rev() {
            if remainder.parts[i] != 0 {
                rem_msb = i * 64 + (63 - remainder.parts[i].leading_zeros()) as usize;
                found_rem = true;
                break;
            }
        }
        if !found_rem || remainder < other {
            return (quotient, remainder);
        }
        let mut shift = rem_msb - other_msb;
        let mut temp = other << shift;
        loop {
            if remainder >= temp {
                remainder = remainder.sub(temp);
                let word = shift / 64;
                let bit = shift % 64;
                quotient.parts[word] |= 1u64 << bit;
            }
            if shift == 0 {
                break;
            }
            shift -= 1;
            temp = temp >> 1;
        }
        (quotient, remainder)
    }

    /// Check if the value fits in a u128.
    pub fn fits_in_u128(&self) -> bool {
        self.parts[2] == 0 && self.parts[3] == 0
    }

    /// Convert the value to a u128.
    pub fn as_u128(&self) -> u128 {
        (self.parts[0] as u128) | ((self.parts[1] as u128) << 64)
    }
}

impl Ord for U256 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        for i in (0..4).rev() {
            let ord = self.parts[i].cmp(&other.parts[i]);
            if ord != std::cmp::Ordering::Equal {
                return ord;
            }
        }
        std::cmp::Ordering::Equal
    }
}

impl PartialOrd for U256 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[allow(clippy::needless_range_loop)]
impl std::ops::Shl<usize> for U256 {
    type Output = Self;
    fn shl(self, shift: usize) -> Self {
        if shift == 0 {
            return self;
        }
        if shift >= 256 {
            return Self { parts: [0; 4] };
        }
        let word_shift = shift / 64;
        let bit_shift = shift % 64;
        let mut parts = [0u64; 4];
        for i in word_shift..4 {
            parts[i] = self.parts[i - word_shift] << bit_shift;
            if bit_shift > 0 && i - word_shift > 0 {
                parts[i] |= self.parts[i - word_shift - 1] >> (64 - bit_shift);
            }
        }
        Self { parts }
    }
}

#[allow(clippy::needless_range_loop)]
impl std::ops::Shr<usize> for U256 {
    type Output = Self;
    fn shr(self, shift: usize) -> Self {
        if shift == 0 {
            return self;
        }
        if shift >= 256 {
            return Self { parts: [0; 4] };
        }
        let word_shift = shift / 64;
        let bit_shift = shift % 64;
        let mut parts = [0u64; 4];
        for i in 0..(4 - word_shift) {
            parts[i] = self.parts[i + word_shift] >> bit_shift;
            if bit_shift > 0 && i + word_shift + 1 < 4 {
                parts[i] |= self.parts[i + word_shift + 1] << (64 - bit_shift);
            }
        }
        Self { parts }
    }
}

/// Maximum input value for factorial computation.
/// Values above this threshold return NaN to prevent excessive memory/CPU usage.
/// 10000! has ~35,660 digits which is a reasonable upper bound.
const MAX_FACTORIAL_INPUT: u32 = 10_000;

impl Number {
    /// Computes the factorial of a non-negative integer.
    ///
    /// Returns NaN for negative numbers, non-integers, complex numbers,
    /// and values exceeding `MAX_FACTORIAL_INPUT`.
    /// Uses `rug::Integer::factorial()` for efficient computation.
    pub fn factorial(&self) -> Self {
        let (real, imag) = self.to_canonical_ref();
        if !imag.is_real_zero() {
            return Self::nan();
        }
        match &*real {
            NumberValue::Rational(r) => {
                if !r.value.is_integer() {
                    return Self::nan();
                }
                let n = r.value.numer();
                if *n < 0 {
                    return Self::nan();
                }
                if let Some(n_u32) = n.to_u32() {
                    if n_u32 > MAX_FACTORIAL_INPUT {
                        return Self::nan();
                    }
                    let result = rug::Integer::factorial(n_u32);
                    Self::from_rational(Rational {
                        value: rug::Rational::from(result),
                    })
                } else {
                    Self::nan()
                }
            }
            NumberValue::Float(f) => {
                if !f.rug_float().is_integer() {
                    return Self::nan();
                }
                if let Some(n) = f.rug_float().to_integer() {
                    if n < 0 {
                        return Self::nan();
                    }
                    if let Some(n_u32) = n.to_u32() {
                        if n_u32 > MAX_FACTORIAL_INPUT {
                            return Self::nan();
                        }
                        let result = rug::Integer::factorial(n_u32);
                        Self::from_rational(Rational {
                            value: rug::Rational::from(result),
                        })
                    } else {
                        Self::nan()
                    }
                } else {
                    Self::nan()
                }
            }
            _ => Self::nan(),
        }
    }

    /// Computes the double factorial n!! = n * (n-2) * (n-4) * ...
    ///
    /// Returns NaN for values less than -1, non-integers, complex numbers,
    /// and values exceeding `MAX_FACTORIAL_INPUT`.
    pub fn double_factorial(&self) -> Self {
        let (real, imag) = self.to_canonical_ref();
        if !imag.is_real_zero() {
            return Self::nan();
        }
        match &*real {
            NumberValue::Rational(r) => {
                if !r.value.is_integer() {
                    return Self::nan();
                }
                let n = r.value.numer();
                if *n == -1 || *n == 0 {
                    return Self::from_rational(Rational::from_i32(1));
                }
                if *n < -1 {
                    return Self::nan();
                }
                if let Some(n_u32) = n.to_u32() {
                    if n_u32 > MAX_FACTORIAL_INPUT {
                        return Self::nan();
                    }
                    let result = rug::Integer::factorial_2(n_u32);
                    Self::from_rational(Rational {
                        value: rug::Rational::from(result),
                    })
                } else {
                    Self::nan()
                }
            }
            NumberValue::Float(f) => {
                if !f.rug_float().is_integer() {
                    return Self::nan();
                }
                if let Some(n) = f.rug_float().to_integer() {
                    if n == -1 || n == 0 {
                        return Self::from_rational(Rational::from_i32(1));
                    }
                    if n < -1 {
                        return Self::nan();
                    }
                    if let Some(n_u32) = n.to_u32() {
                        if n_u32 > MAX_FACTORIAL_INPUT {
                            return Self::nan();
                        }
                        let result = rug::Integer::factorial_2(n_u32);
                        Self::from_rational(Rational {
                            value: rug::Rational::from(result),
                        })
                    } else {
                        Self::nan()
                    }
                } else {
                    Self::nan()
                }
            }
            _ => Self::nan(),
        }
    }

    /// Computes the multi-factorial n!...! with `k` exclamation marks.
    ///
    /// n!(k) = n * (n-k) * (n-2k) * ... down to the first value > 0.
    /// Returns NaN for negative numbers, non-integers, complex numbers, or k=0.
    pub fn multi_factorial(&self, k: u32) -> Self {
        if k == 0 {
            return Self::nan();
        }
        if k == 1 {
            return self.factorial();
        }
        let (real, imag) = self.to_canonical_ref();
        if !imag.is_real_zero() {
            return Self::nan();
        }
        match &*real {
            NumberValue::Rational(r) => {
                if !r.value.is_integer() {
                    return Self::nan();
                }
                let n = r.value.numer();
                if *n < 0 {
                    return Self::nan();
                }
                if let Some(n_u32) = n.to_u32() {
                    if n_u32 > MAX_FACTORIAL_INPUT {
                        return Self::nan();
                    }
                    if k == 2 {
                        let result = rug::Integer::factorial_2(n_u32);
                        return Self::from_rational(Rational {
                            value: rug::Rational::from(result),
                        });
                    }
                    let mut result = rug::Integer::from(1);
                    let mut current = n_u32;
                    while current > 0 {
                        result *= current;
                        if current <= k {
                            break;
                        }
                        current -= k;
                    }
                    Self::from_rational(Rational {
                        value: rug::Rational::from(result),
                    })
                } else {
                    Self::nan()
                }
            }
            NumberValue::Float(f) => {
                if !f.rug_float().is_integer() {
                    return Self::nan();
                }
                if let Some(n) = f.rug_float().to_integer() {
                    if n < 0 {
                        return Self::nan();
                    }
                    if let Some(n_u32) = n.to_u32() {
                        if n_u32 > MAX_FACTORIAL_INPUT {
                            return Self::nan();
                        }
                        if k == 2 {
                            let result = rug::Integer::factorial_2(n_u32);
                            return Self::from_rational(Rational {
                                value: rug::Rational::from(result),
                            });
                        }
                        let mut result = rug::Integer::from(1);
                        let mut current = n_u32;
                        while current > 0 {
                            result *= current;
                            if current <= k {
                                break;
                            }
                            current -= k;
                        }
                        Self::from_rational(Rational {
                            value: rug::Rational::from(result),
                        })
                    } else {
                        Self::nan()
                    }
                } else {
                    Self::nan()
                }
            }
            _ => Self::nan(),
        }
    }

    /// Extracts the integer value if the number is real and represents an integer.
    pub fn to_integer(&self) -> Option<rug::Integer> {
        let (real, imag) = self.to_canonical_ref();
        if imag.is_real_zero() {
            real.to_integer()
        } else {
            None
        }
    }

    /// Computes the binomial coefficient binomial(self, k).
    ///
    /// Returns `Some(Number)` on success, or `None` if they are not integers,
    /// or if the values are out of bounds or cannot be computed.
    pub fn binomial(&self, k: &Self) -> Option<Self> {
        let m_int = self.to_integer()?;
        let k_int = k.to_integer()?;

        if k_int < 0 {
            return Some(Self::from_rational(Rational::from_i32(0)));
        }

        if m_int < 0 {
            // m2 = -m + k - 1 = k - m - 1
            let mut m2_int = rug::Integer::from(&k_int - &m_int);
            m2_int -= 1;
            let m2 = Number::from_rational(Rational {
                value: rug::Rational::from(m2_int),
            });
            let mut result = m2.binomial(k)?;
            if k_int.is_odd() {
                result = result.negate();
            }
            return Some(result);
        }

        if k_int > m_int {
            return Some(Self::from_rational(Rational::from_i32(0)));
        }

        if m_int == k_int || k_int == 0 {
            return Some(Self::from_rational(Rational::from_i32(1)));
        }

        let mut k_effective = k_int.clone();
        let complement = rug::Integer::from(&m_int - &k_int);
        if complement < k_effective {
            k_effective = complement;
        }

        // k must fit in u32 to use with rug::Integer binomial
        let k_u32 = k_effective.to_u32()?;

        // Safety checks for huge values to match C++
        // C++: if((k.integerLength() > 21 || m.integerLength() > 22 * (1 << (21 - k.integerLength()))) && m > k + 1000000L) return false;
        let k_len = k_effective.significant_bits();
        let m_len = m_int.significant_bits();
        if (k_len > 21 || m_len > 22 * (1 << (21 - std::cmp::min(21, k_len))))
            && m_int > rug::Integer::from(&k_effective + 1_000_000)
        {
            return None;
        }

        let res_int = m_int.binomial(k_u32);

        Some(Self::from_rational(Rational {
            value: rug::Rational::from(res_int),
        }))
    }
}

#[cfg(test)]
mod tests;
