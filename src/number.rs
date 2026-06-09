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

/// A private helper for Euclidean GCD algorithm.
fn gcd(a: i128, b: i128) -> u128 {
    let mut ua = a.unsigned_abs();
    let mut ub = b.unsigned_abs();
    while ub != 0 {
        let temp = ub;
        ub = ua % ub;
        ua = temp;
    }
    ua
}

/// A placeholder for GMP rational representation.
///
/// # Invariants (post-canonicalization)
/// - `den > 0`
/// - `gcd(num.unsigned_abs(), den as u128) == 1`
///
/// # Known limitations
/// This placeholder uses `i128` arithmetic. Values where the reduced numerator
/// or denominator exceeds `i128::MAX` (e.g., `i128::MIN / -1`) will panic
/// during canonicalization. These cases will be handled correctly once the
/// GMP/rug backend replaces this placeholder.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Rational {
    /// Numerator of the rational number.
    num: i128,
    /// Denominator of the rational number.
    den: i128,
}

impl Rational {
    /// Create a new rational number, reducing it to lowest terms.
    pub fn new(num: i128, den: i128) -> Self {
        let mut r = Self { num, den };
        r.canonicalize();
        r
    }

    /// Create a rational from an i32.
    pub fn from_i32(val: i32) -> Self {
        Self::new(val as i128, 1)
    }

    /// Returns true if the rational is zero.
    pub fn is_zero(&self) -> bool {
        self.num == 0
    }

    /// Returns true if the rational is exactly one.
    pub fn is_one(&self) -> bool {
        self.num == 1 && self.den == 1
    }

    /// Normalize the sign and reduce the fraction.
    ///
    /// # Panics
    /// Panics if `den == 0` (undefined value) or if the reduced numerator or
    /// denominator overflows `i128` (e.g., `i128::MIN / -1`).
    fn canonicalize(&mut self) {
        assert!(self.den != 0, "Rational denominator must not be zero");
        if self.num == 0 {
            self.den = 1;
            return;
        }

        let u_num = self.num.unsigned_abs();
        let u_den = self.den.unsigned_abs();
        let g = gcd(self.num, self.den);

        let reduced_u_num = u_num / g;
        let reduced_u_den = u_den / g;
        // Determine the final sign: negative if exactly one of num/den is negative
        let num_neg = self.num < 0;
        let den_neg = self.den < 0;
        let result_neg = num_neg ^ den_neg;

        // Guard against overflow: if the reduced value exceeds the representable
        // range for i128 with the given sign, we cannot represent it.
        // i128 range: -2^127 to 2^127-1, so negative values allow one more magnitude.
        let num_limit = if result_neg {
            i128::MIN.unsigned_abs()
        } else {
            i128::MAX as u128
        };
        assert!(
            reduced_u_num <= num_limit,
            "Rational numerator overflow: {} cannot be represented as i128",
            reduced_u_num
        );
        // Denominator is always positive after canonicalization
        assert!(
            reduced_u_den <= i128::MAX as u128,
            "Rational denominator overflow: {} cannot be represented as i128",
            reduced_u_den
        );

        let mut reduced_num = if result_neg && reduced_u_num == i128::MIN.unsigned_abs() {
            i128::MIN
        } else {
            reduced_u_num as i128
        };
        let reduced_den = reduced_u_den as i128;

        if result_neg && reduced_num != i128::MIN {
            reduced_num = -reduced_num;
        }

        self.num = reduced_num;
        self.den = reduced_den;
    }

    /// Returns the numerator.
    pub fn num(&self) -> i128 {
        self.num
    }

    /// Returns the denominator.
    pub fn den(&self) -> i128 {
        self.den
    }
}

/// A placeholder for MPFR float representation.
#[derive(Debug, Clone)]
pub struct Float {
    /// The numeric value as an f64.
    value: f64,
    /// The precision in bits.
    prec: u32,
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
        Self { value: val, prec }
    }
    /// Returns true if the float is zero.
    ///
    /// Note: treats `-0.0` as zero (matching IEEE 754 semantics).
    // TODO: replace with MPFR `mpfr_zero_p` when backend is upgraded.
    pub fn is_zero(&self) -> bool {
        self.value == 0.0
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
        self.value
    }

    /// Returns the precision in bits.
    pub fn prec(&self) -> u32 {
        self.prec
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
            NumberValue::Float(f) => f.prec as i32,
            NumberValue::Interval { lower, .. } => lower.prec as i32,
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
            NumberValue::Rational(r) => r
                .num
                .checked_neg()
                .map(|num| NumberValue::Rational(Rational::new(num, r.den)))
                .unwrap_or(NumberValue::NaN),
            NumberValue::Float(f) => NumberValue::Float(Float::from_f64(-f.value, f.prec)),
            NumberValue::Interval { lower, upper } => NumberValue::Interval {
                lower: Float::from_f64(-upper.value, upper.prec),
                upper: Float::from_f64(-lower.value, lower.prec),
            },
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
                    (NumberValue::Float(f1), NumberValue::Float(f2)) => NumberValue::Float(
                        Float::from_f64(f1.value + f2.value, std::cmp::max(f1.prec, f2.prec)),
                    ),
                    (NumberValue::Rational(r), NumberValue::Float(f)) => {
                        let val = (r.num as f64 / r.den as f64) + f.value;
                        NumberValue::Float(Float::from_f64(val, f.prec))
                    }
                    (NumberValue::Float(f), NumberValue::Rational(r)) => {
                        let val = f.value + (r.num as f64 / r.den as f64);
                        NumberValue::Float(Float::from_f64(val, f.prec))
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
                    ) => NumberValue::Interval {
                        lower: Float::from_f64(
                            l1.value + l2.value,
                            std::cmp::max(l1.prec, l2.prec),
                        ),
                        upper: Float::from_f64(
                            u1.value + u2.value,
                            std::cmp::max(u1.prec, u2.prec),
                        ),
                    },
                    (NumberValue::Interval { lower, upper }, other_val) => {
                        let other_f = to_float_val(other_val);
                        NumberValue::Interval {
                            lower: Float::from_f64(
                                lower.value + other_f.value,
                                std::cmp::max(lower.prec, other_f.prec),
                            ),
                            upper: Float::from_f64(
                                upper.value + other_f.value,
                                std::cmp::max(upper.prec, other_f.prec),
                            ),
                        }
                    }
                    (self_val, NumberValue::Interval { lower, upper }) => {
                        let self_f = to_float_val(self_val);
                        NumberValue::Interval {
                            lower: Float::from_f64(
                                self_f.value + lower.value,
                                std::cmp::max(self_f.prec, lower.prec),
                            ),
                            upper: Float::from_f64(
                                self_f.value + upper.value,
                                std::cmp::max(self_f.prec, upper.prec),
                            ),
                        }
                    }
                    _ => NumberValue::NaN,
                },
            }
        }
    }
}

fn get_infinity_sign(val: &NumberValue) -> Option<bool> {
    match val {
        NumberValue::PlusInfinity => Some(true),
        NumberValue::MinusInfinity => Some(false),
        NumberValue::Float(f) => {
            if f.value == f64::INFINITY {
                Some(true)
            } else if f.value == f64::NEG_INFINITY {
                Some(false)
            } else {
                None
            }
        }
        NumberValue::Uncertainty { value, .. } => get_infinity_sign(value),
        _ => None,
    }
}

fn add_rationals(lhs: &Rational, rhs: &Rational) -> Option<Rational> {
    let lhs_num = lhs.num.checked_mul(rhs.den)?;
    let rhs_num = rhs.num.checked_mul(lhs.den)?;
    let num = lhs_num.checked_add(rhs_num)?;
    let den = lhs.den.checked_mul(rhs.den)?;
    Some(Rational::new(num, den))
}

fn to_float_val(val: &NumberValue) -> Float {
    match val {
        NumberValue::Float(f) => f.clone(),
        NumberValue::Rational(r) => Float::from_f64(r.num as f64 / r.den as f64, 53),
        _ => Float::from_f64(f64::NAN, 53),
    }
}

fn try_unwrap_single_val(val: &NumberValue) -> Option<NumberValue> {
    match val {
        NumberValue::Interval { lower, upper } => {
            if lower.value == upper.value {
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
        (NumberValue::Rational(r1), NumberValue::Rational(r2)) => {
            r1.num == r2.num && r1.den == r2.den
        }
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
        ) => l1.value == l2.value && u1.value == u2.value,
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
        let prec = f.prec as i32;
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
        let prec = lower.prec as i32;
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

    /// Helper to convert the number into canonical real and imaginary components.
    pub fn to_canonical_real_imag(&self) -> (NumberValue, NumberValue) {
        if self.is_imaginary {
            (
                NumberValue::Rational(Rational::from_i32(0)),
                self.value.clone(),
            )
        } else {
            let real_val = self.value.clone();
            if let Some(imag) = &self.imaginary {
                let (_, imag_coeff) = imag.to_canonical_real_imag();
                (real_val, imag_coeff)
            } else {
                (real_val, NumberValue::Rational(Rational::from_i32(0)))
            }
        }
    }

    /// Returns true if the number has an imaginary part or is itself marked imaginary.
    pub fn is_complex(&self) -> bool {
        let (_, imag) = self.to_canonical_real_imag();
        !imag.is_real_zero()
    }

    /// Returns true if the number has a real part (is not purely imaginary).
    pub fn has_real_part(&self) -> bool {
        let (real, imag) = self.to_canonical_real_imag();
        !real.is_real_zero() || imag.is_real_zero()
    }

    /// Returns true if the number has an imaginary part or is purely imaginary.
    pub fn has_imaginary_part(&self) -> bool {
        self.is_complex()
    }

    /// Returns true if the entire number is zero.
    pub fn is_zero(&self) -> bool {
        let (real, imag) = self.to_canonical_real_imag();
        real.is_real_zero() && imag.is_real_zero()
    }

    /// Returns true if the real part of the number is zero.
    pub fn is_real_zero(&self) -> bool {
        let (real, _) = self.to_canonical_real_imag();
        real.is_real_zero()
    }

    /// Returns true if the number is exactly one.
    pub fn is_one(&self) -> bool {
        let (real, imag) = self.to_canonical_real_imag();
        real.is_real_one() && imag.is_real_zero()
    }

    /// Returns true if the real part of the number is exactly one.
    pub fn is_real_one(&self) -> bool {
        let (real, _) = self.to_canonical_real_imag();
        real.is_real_one()
    }

    /// Returns true if either the real or the imaginary part is an interval.
    pub fn is_interval(&self) -> bool {
        let (real, imag) = self.to_canonical_real_imag();
        real.is_interval() || imag.is_interval()
    }

    /// Returns true if either the real or the imaginary part is infinite.
    pub fn is_infinite(&self) -> bool {
        let (real, imag) = self.to_canonical_real_imag();
        real.is_infinite() || imag.is_infinite()
    }

    /// Returns true if either the real or imaginary part is or contains infinity.
    pub fn includes_infinity(&self) -> bool {
        let (real, imag) = self.to_canonical_real_imag();
        real.includes_infinity() || imag.includes_infinity()
    }

    /// Returns true if either the real or the imaginary part is NaN.
    pub fn is_nan(&self) -> bool {
        let (real, imag) = self.to_canonical_real_imag();
        real.is_nan() || imag.is_nan()
    }
}

impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        let (lhs_real, lhs_imag) = self.to_canonical_real_imag();
        let (rhs_real, rhs_imag) = other.to_canonical_real_imag();
        lhs_real == rhs_real && lhs_imag == rhs_imag
    }
}

#[cfg(test)]
mod tests;
