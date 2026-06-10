//! Core `Number` representation — placeholder for GMP/MPFR.
//!
//! # Upstream oracle
//! - `../libqalculate/libqalculate/Number.h`
//! - `../libqalculate/libqalculate/Number.cc`
//!
//! # Known divergences from upstream
//! - **Precision**: `Rational` uses `i128` instead of GMP `mpq_t`. Values
//!   exceeding `i128::MAX` (e.g., `i128::MIN / -1`) will panic during
//!   canonicalization. Upstream uses arbitrary-precision rationals.
//! - **Float backend**: `Float` uses `f64` instead of MPFR `mpfr_t`. Precision
//!   is limited to ~53 bits (IEEE 754 double) instead of arbitrary.
//! - **Mixed equality**: `Rational` and `Float` values are not compared across
//!   representations in this scaffold. Upstream GMP/MPFR comparisons can do
//!   this exactly; the placeholder backend cannot.
//!
//! These divergences will be resolved when the GMP/MPFR backend replaces this
//! placeholder implementation.

use rug::ops::AssignRound;

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
        let a = self.num();
        let b = self.den();
        let c = other.num();
        let d = other.den();

        let g = gcd(b, d);
        let b_prime = (b as u128) / g;
        let d_prime = (d as u128) / g;

        let neg1 = a < 0;
        let abs1 = U256::mul_u128(a.unsigned_abs(), d_prime);

        let neg2 = c < 0;
        let abs2 = U256::mul_u128(c.unsigned_abs(), b_prime);

        let (neg_t, abs_t) = if neg1 == neg2 {
            (neg1, abs1.add(abs2))
        } else if abs1 >= abs2 {
            (neg1, abs1.sub(abs2))
        } else {
            (neg2, abs2.sub(abs1))
        };

        let abs_rem = abs_t.div_rem(U256::from_u128(g)).1.as_u128();
        let g2 = gcd_u128(g, abs_rem);

        let num_u256 = abs_t.div_rem(U256::from_u128(g2)).0;

        let limit = if neg_t {
            i128::MIN.unsigned_abs()
        } else {
            i128::MAX as u128
        };
        if !num_u256.fits_in_u128() || num_u256.as_u128() > limit {
            return None;
        }

        let num = if neg_t {
            let val = num_u256.as_u128();
            if val == i128::MIN.unsigned_abs() {
                i128::MIN
            } else {
                -(val as i128)
            }
        } else {
            num_u256.as_u128() as i128
        };

        let g_i128 = g as i128;
        let g2_i128 = g2 as i128;
        let b_prime_i128 = b_prime as i128;
        let d_prime_i128 = d_prime as i128;

        let den = g_i128
            .checked_div(g2_i128)?
            .checked_mul(b_prime_i128)?
            .checked_mul(d_prime_i128)?;

        Self::try_new(num, den)
    }

    /// GCD-optimized checked subtraction.
    pub fn checked_sub(&self, other: &Self) -> Option<Self> {
        let a = self.num();
        let b = self.den();
        let c = other.num();
        let d = other.den();

        let g = gcd(b, d);
        let b_prime = (b as u128) / g;
        let d_prime = (d as u128) / g;

        let neg1 = a < 0;
        let abs1 = U256::mul_u128(a.unsigned_abs(), d_prime);

        let neg2 = c >= 0;
        let abs2 = U256::mul_u128(c.unsigned_abs(), b_prime);

        let (neg_t, abs_t) = if neg1 == neg2 {
            (neg1, abs1.add(abs2))
        } else if abs1 >= abs2 {
            (neg1, abs1.sub(abs2))
        } else {
            (neg2, abs2.sub(abs1))
        };

        let abs_rem = abs_t.div_rem(U256::from_u128(g)).1.as_u128();
        let g2 = gcd_u128(g, abs_rem);

        let num_u256 = abs_t.div_rem(U256::from_u128(g2)).0;

        let limit = if neg_t {
            i128::MIN.unsigned_abs()
        } else {
            i128::MAX as u128
        };
        if !num_u256.fits_in_u128() || num_u256.as_u128() > limit {
            return None;
        }

        let num = if neg_t {
            let val = num_u256.as_u128();
            if val == i128::MIN.unsigned_abs() {
                i128::MIN
            } else {
                -(val as i128)
            }
        } else {
            num_u256.as_u128() as i128
        };

        let g_i128 = g as i128;
        let g2_i128 = g2 as i128;
        let b_prime_i128 = b_prime as i128;
        let d_prime_i128 = d_prime as i128;

        let den = g_i128
            .checked_div(g2_i128)?
            .checked_mul(b_prime_i128)?
            .checked_mul(d_prime_i128)?;

        Self::try_new(num, den)
    }

    /// GCD-optimized checked multiplication.
    pub fn checked_mul(&self, other: &Self) -> Option<Self> {
        let a = self.num();
        let b = self.den();
        let c = other.num();
        let d = other.den();

        let g1 = gcd(a, d) as i128;
        let g2 = gcd(c, b) as i128;

        let a_prime = a / g1;
        let d_prime = d / g1;
        let c_prime = c / g2;
        let b_prime = b / g2;

        let num = a_prime.checked_mul(c_prime)?;
        let den = b_prime.checked_mul(d_prime)?;

        Self::try_new(num, den)
    }

    /// GCD-optimized checked division.
    pub fn checked_div(&self, other: &Self) -> Option<Self> {
        let a = self.num();
        let b = self.den();
        let c = other.num();
        let d = other.den();

        if c == 0 {
            return None;
        }

        let g1 = gcd(a, c) as i128;
        let g2 = gcd(d, b) as i128;

        let a_prime = a / g1;
        let c_prime = c / g1;
        let d_prime = d / g2;
        let b_prime = b / g2;

        let num = a_prime.checked_mul(d_prime)?;
        let den = b_prime.checked_mul(c_prime)?;

        Self::try_new(num, den)
    }
}

impl Ord for Rational {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.num() == 0 && other.num() == 0 {
            return std::cmp::Ordering::Equal;
        }
        if self.num() == 0 {
            return if other.num() > 0 {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            };
        }
        if other.num() == 0 {
            return if self.num() > 0 {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Less
            };
        }

        let self_pos = self.num() > 0;
        let other_pos = other.num() > 0;

        match (self_pos, other_pos) {
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            (true, true) => cmp_u128_rational(
                self.num() as u128,
                self.den() as u128,
                other.num() as u128,
                other.den() as u128,
                false,
            ),
            (false, false) => cmp_u128_rational(
                self.num().unsigned_abs(),
                self.den() as u128,
                other.num().unsigned_abs(),
                other.den() as u128,
                true,
            ),
        }
    }
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

/// Inner enum representing the type and value of a number.
#[derive(Debug, Clone)]
pub enum NumberValue {
    /// Rational number representation.
    Rational(Rational),
    /// Float number representation.
    Float(Float),
    /// Interval representation with lower and upper bounds.
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
            NumberValue::Interval { lower, .. } => lower.prec() as i32,
            NumberValue::Uncertainty { value, uncertainty } => {
                std::cmp::max(value.precision(), uncertainty.precision())
            }
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
            NumberValue::Uncertainty { value, uncertainty } => {
                value.is_real_zero() && uncertainty.is_real_zero()
            }
            _ => false,
        }
    }

    /// Check if rational is 1/1, float is 1.0, interval point bounds are 1.0, or uncertainty is 1.0 and zero.
    pub fn is_real_one(&self) -> bool {
        match self {
            NumberValue::Rational(r) => r.is_one(),
            NumberValue::Float(f) => f.is_one(),
            NumberValue::Interval { lower, upper } => lower.is_one() && upper.is_one(),
            NumberValue::Uncertainty { value, uncertainty } => {
                value.is_real_one() && uncertainty.is_real_zero()
            }
            _ => false,
        }
    }

    /// Check if infinity.
    pub fn is_infinite(&self) -> bool {
        match self {
            NumberValue::PlusInfinity | NumberValue::MinusInfinity => true,
            NumberValue::Float(f) => f.is_infinite(),
            NumberValue::Uncertainty { value, uncertainty } => {
                value.is_infinite() || uncertainty.is_infinite()
            }
            _ => false,
        }
    }

    /// Check if this value is or contains infinity.
    pub fn includes_infinity(&self) -> bool {
        match self {
            NumberValue::Interval { lower, upper } => lower.is_infinite() || upper.is_infinite(),
            NumberValue::Uncertainty { value, uncertainty } => {
                value.includes_infinity() || uncertainty.includes_infinity()
            }
            _ => self.is_infinite(),
        }
    }

    /// Check if NaN.
    pub fn is_nan(&self) -> bool {
        match self {
            NumberValue::NaN => true,
            NumberValue::Float(f) => f.is_nan(),
            NumberValue::Interval { lower, upper } => lower.is_nan() || upper.is_nan(),
            NumberValue::Uncertainty { value, uncertainty } => {
                value.is_nan() || uncertainty.is_nan()
            }
            _ => false,
        }
    }

    /// Check if interval.
    pub fn is_interval(&self) -> bool {
        match self {
            NumberValue::Interval { .. } => true,
            NumberValue::Uncertainty { value, uncertainty } => {
                value.is_interval() || uncertainty.is_interval()
            }
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
            NumberValue::Uncertainty { value, uncertainty } => NumberValue::Uncertainty {
                value: Box::new(value.negate()),
                uncertainty: Box::new((**uncertainty).clone()),
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
                    },
                    NumberValue::Uncertainty {
                        value: v2,
                        uncertainty: u2,
                    },
                ) => NumberValue::Uncertainty {
                    value: Box::new(v1.add(v2)),
                    uncertainty: Box::new(u1.add(u2)),
                },
                (NumberValue::Uncertainty { value, uncertainty }, other) => {
                    NumberValue::Uncertainty {
                        value: Box::new(value.add(other)),
                        uncertainty: Box::new((**uncertainty).clone()),
                    }
                }
                (self_val, NumberValue::Uncertainty { value, uncertainty }) => {
                    NumberValue::Uncertainty {
                        value: Box::new(self_val.add(value)),
                        uncertainty: Box::new((**uncertainty).clone()),
                    }
                }
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
                    },
                    NumberValue::Uncertainty {
                        value: v2,
                        uncertainty: u2,
                    },
                ) => NumberValue::Uncertainty {
                    value: Box::new(v1.sub(v2)),
                    uncertainty: Box::new(u1.add(u2)),
                },
                (NumberValue::Uncertainty { value, uncertainty }, other) => {
                    NumberValue::Uncertainty {
                        value: Box::new(value.sub(other)),
                        uncertainty: Box::new((**uncertainty).clone()),
                    }
                }
                (self_val, NumberValue::Uncertainty { value, uncertainty }) => {
                    NumberValue::Uncertainty {
                        value: Box::new(self_val.sub(value)),
                        uncertainty: Box::new((**uncertainty).clone()),
                    }
                }
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
                },
                NumberValue::Uncertainty {
                    value: v2,
                    uncertainty: u2,
                },
            ) => {
                let val = v1.mul(v2);
                let term1 = v1.abs().mul(u2);
                let term2 = v2.abs().mul(u1);
                let term3 = u1.mul(u2);
                let unc = term1.add(&term2).add(&term3);
                NumberValue::Uncertainty {
                    value: Box::new(val),
                    uncertainty: Box::new(unc),
                }
            }
            (NumberValue::Uncertainty { value, uncertainty }, other) => {
                let val = value.mul(other);
                let unc = uncertainty.mul(&other.abs());
                NumberValue::Uncertainty {
                    value: Box::new(val),
                    uncertainty: Box::new(unc),
                }
            }
            (self_val, NumberValue::Uncertainty { value, uncertainty }) => {
                let val = self_val.mul(value);
                let unc = self_val.abs().mul(uncertainty);
                NumberValue::Uncertainty {
                    value: Box::new(val),
                    uncertainty: Box::new(unc),
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

        if other.is_real_zero() {
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
                    },
                    NumberValue::Uncertainty {
                        value: v2,
                        uncertainty: u2,
                    },
                ) => {
                    if v2.is_real_zero() {
                        return NumberValue::NaN;
                    }
                    let val = v1.div(v2);
                    let term1 = v1.abs().mul(u2);
                    let term2 = v2.abs().mul(u1);
                    let num_unc = term1.add(&term2);
                    let den_unc = v2.mul(v2);
                    let unc = num_unc.div(&den_unc);
                    NumberValue::Uncertainty {
                        value: Box::new(val),
                        uncertainty: Box::new(unc),
                    }
                }
                (NumberValue::Uncertainty { value, uncertainty }, other) => {
                    if other.is_real_zero() {
                        return NumberValue::NaN;
                    }
                    let val = value.div(other);
                    let unc = uncertainty.div(&other.abs());
                    NumberValue::Uncertainty {
                        value: Box::new(val),
                        uncertainty: Box::new(unc),
                    }
                }
                (self_val, NumberValue::Uncertainty { value, uncertainty }) => {
                    if value.is_real_zero() {
                        return NumberValue::NaN;
                    }
                    let val = self_val.div(value);
                    let num_unc = self_val.abs().mul(uncertainty);
                    let den_unc = value.mul(value);
                    let unc = num_unc.div(&den_unc);
                    NumberValue::Uncertainty {
                        value: Box::new(val),
                        uncertainty: Box::new(unc),
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
            NumberValue::Uncertainty { value, uncertainty } => NumberValue::Uncertainty {
                value: Box::new(value.abs()),
                uncertainty: Box::new((**uncertainty).clone()),
            },
            NumberValue::PlusInfinity | NumberValue::MinusInfinity => NumberValue::PlusInfinity,
            NumberValue::NaN => NumberValue::NaN,
        }
    }

    /// Returns the square root of the number value.
    pub fn sqrt(&self) -> Self {
        match self {
            NumberValue::Rational(r) => {
                if r.num() < 0 {
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
            NumberValue::Uncertainty { value, uncertainty } => {
                let val_sqrt = value.sqrt();
                if val_sqrt.is_nan() {
                    NumberValue::NaN
                } else {
                    let two = NumberValue::Rational(Rational::from_i32(2));
                    let denom = two.mul(&val_sqrt);
                    let new_uncertainty = uncertainty.div(&denom);
                    NumberValue::Uncertainty {
                        value: Box::new(val_sqrt),
                        uncertainty: Box::new(new_uncertainty),
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
                let val = r.num() as f64 / r.den() as f64;
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
            NumberValue::Uncertainty { value, uncertainty } => {
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
            NumberValue::Rational(r) => {
                if r.num() == 0 {
                    Some(0.0)
                } else if r.num() > 0 {
                    Some(1.0)
                } else {
                    Some(-1.0)
                }
            }
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
        NumberValue::Uncertainty { value, uncertainty } => {
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

fn get_finite_sign(val: &NumberValue) -> Option<bool> {
    match val {
        NumberValue::Rational(r) => Some(r.num() >= 0),
        NumberValue::Float(f) => Some(f.value >= 0.0),
        NumberValue::Interval { lower, upper } => {
            if lower.value >= 0.0 {
                Some(true)
            } else if upper.value <= 0.0 {
                Some(false)
            } else {
                None
            }
        }
        NumberValue::Uncertainty { value, .. } => get_finite_sign(value),
        _ => None,
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
        NumberValue::Uncertainty { value, uncertainty } => {
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
            },
            NumberValue::Uncertainty {
                value: v2,
                uncertainty: u2,
            },
        ) => eq_values(v1, v2) && eq_values(u1, u2),
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
                },
                NumberValue::Uncertainty {
                    value: v2,
                    uncertainty: _,
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
    pub fn new_interval(lower: Float, upper: Float) -> Self {
        let prec = lower.prec() as i32;
        Self {
            value: NumberValue::Interval { lower, upper },
            imaginary: None,
            precision: prec,
            approximate: true,
            is_imaginary: false,
        }
    }

    /// Creates a new uncertainty `Number`.
    pub fn new_uncertainty(value: NumberValue, uncertainty: NumberValue) -> Self {
        let prec = std::cmp::max(value.precision(), uncertainty.precision());
        Self {
            value: NumberValue::Uncertainty {
                value: Box::new(value),
                uncertainty: Box::new(uncertainty),
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
        match val {
            NumberValue::Interval { .. } => true,
            NumberValue::Uncertainty { .. } => true,
            _ => false,
        }
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
            if self.is_zero() {
                return Number::from_f64(f64::NAN);
            }
            let (x1, y1) = self.to_canonical_ref();
            let real_val = if x1.is_real_zero() {
                NumberValue::Rational(Rational::from_i32(0))
            } else {
                let sign = get_finite_sign(&x1).unwrap_or(true);
                if sign {
                    NumberValue::PlusInfinity
                } else {
                    NumberValue::MinusInfinity
                }
            };
            let imag_val = if y1.is_real_zero() {
                NumberValue::Rational(Rational::from_i32(0))
            } else {
                let sign = get_finite_sign(&y1).unwrap_or(true);
                if sign {
                    NumberValue::PlusInfinity
                } else {
                    NumberValue::MinusInfinity
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
            return if imag_num.is_zero() {
                real_num
            } else {
                Number::new_complex(real_num, imag_num)
            };
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
}

impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        let (lhs_real, lhs_imag) = self.to_canonical_ref();
        let (rhs_real, rhs_imag) = other.to_canonical_ref();
        lhs_real == rhs_real && lhs_imag == rhs_imag
    }
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

fn gcd_u128(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let temp = b;
        b = a % b;
        a = temp;
    }
    a
}

fn gcd(a: i128, b: i128) -> u128 {
    gcd_u128(a.unsigned_abs(), b.unsigned_abs())
}

/// A private helper to compare positive rationals without overflow using continued fractions.
fn cmp_u128_rational(a: u128, b: u128, c: u128, d: u128, invert: bool) -> std::cmp::Ordering {
    let q1 = a / b;
    let q2 = c / d;
    if q1 != q2 {
        let ord = q1.cmp(&q2);
        return if invert { ord.reverse() } else { ord };
    }
    let r1 = a % b;
    let r2 = c % d;
    if r1 == 0 && r2 == 0 {
        return std::cmp::Ordering::Equal;
    }
    if r1 == 0 {
        return if invert {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Less
        };
    }
    if r2 == 0 {
        return if invert {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        };
    }
    cmp_u128_rational(b, r1, d, r2, !invert)
}

#[cfg(test)]
mod tests;
