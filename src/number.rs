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
        match self {
            NumberValue::Rational(r) => {
                let value = rug::Float::with_val(53, &r.value).ln();
                NumberValue::Float(Float { value })
            }
            NumberValue::Float(f) => NumberValue::Float(Float {
                value: f.value.clone().ln(),
            }),
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
                let val = value.ln();
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
                let f = to_float_val(self);
                NumberValue::Float(Float {
                    value: f.value.clone().ln(),
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
                        let f_val = rug::Float::with_val(53, &r.value);
                        NumberValue::Float(Float {
                            value: f_val.sqrt(),
                        })
                    }
                }
            }
            NumberValue::Float(f) => {
                if f.value.is_sign_negative() && !f.value.is_zero() {
                    NumberValue::NaN
                } else {
                    NumberValue::Float(Float {
                        value: f.value.clone().sqrt(),
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
                let val_sqrt = value.sqrt();
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

        let new_imag_num = Number {
            value: new_imag_val,
            imaginary: None,
            precision: imag.precision,
            approximate: imag.approximate,
            is_imaginary: true,
        };

        Self {
            value: new_real_val,
            imaginary: Some(Box::new(new_imag_num)),
            precision: std::cmp::max(real.precision, imag.precision),
            approximate: real.approximate || imag.approximate,
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

    /// Returns true if the real part of the number is zero.
    pub fn is_real_zero(&self) -> bool {
        let (real, _) = self.to_canonical_ref();
        real.is_real_zero()
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

    /// Formats this number for qalc-profile native evidence with a requested
    /// decimal digit count for precision-sensitive numeric output.
    pub(crate) fn to_qalc_string_with_precision(&self, precision_digits: usize) -> String {
        let (real, imag) = self.to_canonical_real_imag();
        if imag.is_real_zero() {
            format_qalc_value_with_precision(&real, precision_digits)
        } else if real.is_real_zero() {
            if imag.is_real_one() {
                "i".to_string()
            } else if imag.negate().is_real_one() {
                "-i".to_string()
            } else {
                format!(
                    "{}i",
                    format_qalc_value_with_precision(&imag, precision_digits)
                )
            }
        } else if is_value_negative(&imag) {
            format!(
                "{} - {}i",
                format_qalc_value_with_precision(&real, precision_digits),
                format_qalc_value_with_precision(&imag.negate(), precision_digits)
            )
        } else {
            format!(
                "{} + {}i",
                format_qalc_value_with_precision(&real, precision_digits),
                format_qalc_value_with_precision(&imag, precision_digits)
            )
        }
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
            let f1 = to_float_val(&a);
            let f2 = to_float_val(&c);
            let val = NumberValue::Float(Float::from_f64(f1.value().powf(f2.value()), 53));
            Number {
                value: val,
                imaginary: None,
                precision: 53,
                approximate: true,
                is_imaginary: false,
            }
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

fn format_qalc_value_with_precision(value: &NumberValue, precision_digits: usize) -> String {
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
                format_qalc_value_with_precision(value, precision_digits)
            } else {
                value.to_string_with_uncertainty(uncertainty, *is_relative)
            }
        }
        NumberValue::Interval { lower, upper } => format!(
            "[{}  {}]",
            format_qalc_float_bound(&lower.value),
            format_qalc_float_bound(&upper.value)
        ),
        NumberValue::Float(float) => format_qalc_float(&float.value, precision_digits),
        _ => value.to_string(),
    }
}

fn format_qalc_float(value: &rug::Float, precision_digits: usize) -> String {
    let output = value.to_string_radix(10, Some(precision_digits));
    fixed_decimal_from_scientific(&output).unwrap_or(output)
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

impl std::str::FromStr for Number {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
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
    ("norm", UnaryFunction::Norm),
];

fn strip_unary_function(s: &str) -> Option<(UnaryFunction, &str)> {
    UNARY_FUNCTIONS.iter().find_map(|(name, function)| {
        strip_function_name(s, name).map(|remaining| (*function, remaining))
    })
}

#[derive(Debug, Clone, Copy)]
enum UnaryFunction {
    Conjugate,
    Norm,
}

impl UnaryFunction {
    const fn name(self) -> &'static str {
        match self {
            Self::Conjugate => "conj",
            Self::Norm => "norm",
        }
    }

    fn apply(self, arg: Number) -> Number {
        match self {
            Self::Conjugate => arg.conjugate(),
            Self::Norm => arg.norm(),
        }
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
                Ok(function.apply(arg))
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

#[cfg(test)]
mod tests;
