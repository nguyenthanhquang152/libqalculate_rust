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
            NumberValue::Float(f) => f.prec as i32,
            NumberValue::Interval { lower, .. } => lower.prec as i32,
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
            NumberValue::Uncertainty {
                value,
                uncertainty,
                is_relative,
            } => NumberValue::Uncertainty {
                value: Box::new(value.negate()),
                uncertainty: Box::new((**uncertainty).clone()),
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
                    },
                    NumberValue::Uncertainty {
                        value: v2,
                        uncertainty: u2,
                        is_relative: ir2,
                    },
                ) => {
                    let u1_sq = u1.mul(u1);
                    let u2_sq = u2.mul(u2);
                    let unc = u1_sq.add(&u2_sq).sqrt();
                    NumberValue::Uncertainty {
                        value: Box::new(v1.add(v2)),
                        uncertainty: Box::new(unc),
                        is_relative: *ir1 || *ir2,
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
                    uncertainty: Box::new((**uncertainty).clone()),
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
                    uncertainty: Box::new((**uncertainty).clone()),
                    is_relative: *is_relative,
                },
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

    /// Subtract two values mathematically.
    pub fn sub(&self, other: &Self) -> Self {
        self.add(&other.negate())
    }

    /// Multiply two values mathematically.
    pub fn mul(&self, other: &Self) -> Self {
        if self.is_nan() || other.is_nan() {
            return NumberValue::NaN;
        }
        if self.is_infinite() || other.is_infinite() {
            let f1 = to_float_val(self);
            let f2 = to_float_val(other);
            let val = f1.value * f2.value;
            if val.is_nan() {
                return NumberValue::NaN;
            } else if val == f64::INFINITY {
                return NumberValue::PlusInfinity;
            } else if val == f64::NEG_INFINITY {
                return NumberValue::MinusInfinity;
            } else {
                return NumberValue::Float(Float::from_f64(val, std::cmp::max(f1.prec, f2.prec)));
            }
        }
        match (self, other) {
            (
                NumberValue::Uncertainty {
                    value: v1,
                    uncertainty: u1,
                    is_relative: ir1,
                },
                NumberValue::Uncertainty {
                    value: v2,
                    uncertainty: u2,
                    is_relative: ir2,
                },
            ) => {
                let val = v1.mul(v2);
                let term1 = v2.mul(u1);
                let term2 = v1.mul(u2);
                let unc = term1.mul(&term1).add(&term2.mul(&term2)).sqrt();
                NumberValue::Uncertainty {
                    value: Box::new(val),
                    uncertainty: Box::new(unc),
                    is_relative: *ir1 || *ir2,
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
                let unc = uncertainty.mul(&other.abs());
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
                let unc = uncertainty.mul(&self_val.abs());
                NumberValue::Uncertainty {
                    value: Box::new(val),
                    uncertainty: Box::new(unc),
                    is_relative: *is_relative,
                }
            }
            (NumberValue::Rational(r1), NumberValue::Rational(r2)) => {
                if let Some(num) = r1.num.checked_mul(r2.num) {
                    if let Some(den) = r1.den.checked_mul(r2.den) {
                        return NumberValue::Rational(Rational::new(num, den));
                    }
                }
                let val = (r1.num as f64 / r1.den as f64) * (r2.num as f64 / r2.den as f64);
                NumberValue::Float(Float::from_f64(val, 53))
            }
            (NumberValue::Float(f1), NumberValue::Float(f2)) => NumberValue::Float(
                Float::from_f64(f1.value * f2.value, std::cmp::max(f1.prec, f2.prec)),
            ),
            (NumberValue::Rational(r), NumberValue::Float(f)) => {
                let val = (r.num as f64 / r.den as f64) * f.value;
                NumberValue::Float(Float::from_f64(val, f.prec))
            }
            (NumberValue::Float(f), NumberValue::Rational(r)) => {
                let val = f.value * (r.num as f64 / r.den as f64);
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
            ) => {
                let p1 = l1.value * l2.value;
                let p2 = l1.value * u2.value;
                let p3 = u1.value * l2.value;
                let p4 = u1.value * u2.value;
                let min_val = [p1, p2, p3, p4]
                    .iter()
                    .copied()
                    .fold(f64::INFINITY, f64::min);
                let max_val = [p1, p2, p3, p4]
                    .iter()
                    .copied()
                    .fold(f64::NEG_INFINITY, f64::max);
                let prec = std::cmp::max(l1.prec, l2.prec);
                NumberValue::Interval {
                    lower: Float::from_f64(min_val, prec),
                    upper: Float::from_f64(max_val, prec),
                }
            }
            (NumberValue::Interval { lower, upper }, other_val) => {
                let other_f = to_float_val(other_val);
                let p1 = lower.value * other_f.value;
                let p2 = upper.value * other_f.value;
                NumberValue::Interval {
                    lower: Float::from_f64(
                        f64::min(p1, p2),
                        std::cmp::max(lower.prec, other_f.prec),
                    ),
                    upper: Float::from_f64(
                        f64::max(p1, p2),
                        std::cmp::max(upper.prec, other_f.prec),
                    ),
                }
            }
            (self_val, NumberValue::Interval { lower, upper }) => {
                let self_f = to_float_val(self_val);
                let p1 = self_f.value * lower.value;
                let p2 = self_f.value * upper.value;
                NumberValue::Interval {
                    lower: Float::from_f64(
                        f64::min(p1, p2),
                        std::cmp::max(self_f.prec, lower.prec),
                    ),
                    upper: Float::from_f64(
                        f64::max(p1, p2),
                        std::cmp::max(self_f.prec, upper.prec),
                    ),
                }
            }
            _ => NumberValue::NaN,
        }
    }

    /// Divide two values mathematically.
    pub fn div(&self, other: &Self) -> Self {
        if self.is_nan() || other.is_nan() {
            return NumberValue::NaN;
        }
        match (self, other) {
            (
                NumberValue::Uncertainty {
                    value: v1,
                    uncertainty: u1,
                    is_relative: ir1,
                },
                NumberValue::Uncertainty {
                    value: v2,
                    uncertainty: u2,
                    is_relative: ir2,
                },
            ) => {
                let val = v1.div(v2);
                let term1 = u1.div(v2);
                let term2 = val.mul(u2).div(v2);
                let unc = term1.mul(&term1).add(&term2.mul(&term2)).sqrt();
                NumberValue::Uncertainty {
                    value: Box::new(val),
                    uncertainty: Box::new(unc),
                    is_relative: *ir1 || *ir2,
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
                let val = value.div(other);
                let unc = uncertainty.div(&other.abs());
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
                let val = self_val.div(value);
                let term2 = val.mul(uncertainty).div(value);
                let unc = term2.abs();
                NumberValue::Uncertainty {
                    value: Box::new(val),
                    uncertainty: Box::new(unc),
                    is_relative: *is_relative,
                }
            }
            (NumberValue::Rational(r1), NumberValue::Rational(r2)) => {
                if r2.is_zero() {
                    return NumberValue::NaN;
                }
                if let Some(num) = r1.num.checked_mul(r2.den) {
                    if let Some(den) = r1.den.checked_mul(r2.num) {
                        if den != 0 {
                            return NumberValue::Rational(Rational::new(num, den));
                        }
                    }
                }
                let val = (r1.num as f64 / r1.den as f64) / (r2.num as f64 / r2.den as f64);
                NumberValue::Float(Float::from_f64(val, 53))
            }
            (NumberValue::Float(f1), NumberValue::Float(f2)) => NumberValue::Float(
                Float::from_f64(f1.value / f2.value, std::cmp::max(f1.prec, f2.prec)),
            ),
            (NumberValue::Rational(r), NumberValue::Float(f)) => {
                let val = (r.num as f64 / r.den as f64) / f.value;
                NumberValue::Float(Float::from_f64(val, f.prec))
            }
            (NumberValue::Float(f), NumberValue::Rational(r)) => {
                let val = f.value / (r.num as f64 / r.den as f64);
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
            ) => {
                let p1 = l1.value / l2.value;
                let p2 = l1.value / u2.value;
                let p3 = u1.value / l2.value;
                let p4 = u1.value / u2.value;
                let min_val = [p1, p2, p3, p4]
                    .iter()
                    .copied()
                    .fold(f64::INFINITY, f64::min);
                let max_val = [p1, p2, p3, p4]
                    .iter()
                    .copied()
                    .fold(f64::NEG_INFINITY, f64::max);
                let prec = std::cmp::max(l1.prec, l2.prec);
                NumberValue::Interval {
                    lower: Float::from_f64(min_val, prec),
                    upper: Float::from_f64(max_val, prec),
                }
            }
            _ => {
                let f1 = to_float_val(self);
                let f2 = to_float_val(other);
                NumberValue::Float(Float::from_f64(
                    f1.value / f2.value,
                    std::cmp::max(f1.prec, f2.prec),
                ))
            }
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
                },
                NumberValue::Uncertainty {
                    value: v2,
                    uncertainty: u2,
                    is_relative: ir2,
                },
            ) => {
                let val = v1.pow(v2);
                let one = NumberValue::Rational(Rational::from_i32(1));
                let v2_minus_1 = v2.sub(&one);
                let term1 = v2.mul(&v1.pow(&v2_minus_1)).mul(u1);
                let term2 = val.mul(&v1.ln()).mul(u2);
                let unc = term1.mul(&term1).add(&term2.mul(&term2)).sqrt();
                NumberValue::Uncertainty {
                    value: Box::new(val),
                    uncertainty: Box::new(unc),
                    is_relative: *ir1 || *ir2,
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
                let unc = other.mul(&value.pow(&other_minus_1)).mul(uncertainty).abs();
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
                let unc = val.mul(&self_val.ln()).mul(uncertainty).abs();
                NumberValue::Uncertainty {
                    value: Box::new(val),
                    uncertainty: Box::new(unc),
                    is_relative: *is_relative,
                }
            }
            (NumberValue::Rational(r1), NumberValue::Rational(r2)) => {
                if r2.den == 1 && r2.num >= 0 && r2.num <= 10 {
                    let mut num = 1i128;
                    let mut den = 1i128;
                    let mut ok = true;
                    for _ in 0..r2.num {
                        if let Some(n) = num.checked_mul(r1.num) {
                            if let Some(d) = den.checked_mul(r1.den) {
                                num = n;
                                den = d;
                                continue;
                            }
                        }
                        ok = false;
                        break;
                    }
                    if ok {
                        return NumberValue::Rational(Rational::new(num, den));
                    }
                }
                let val = (r1.num as f64 / r1.den as f64).powf(r2.num as f64 / r2.den as f64);
                NumberValue::Float(Float::from_f64(val, 53))
            }
            (NumberValue::Float(f1), NumberValue::Float(f2)) => NumberValue::Float(
                Float::from_f64(f1.value.powf(f2.value), std::cmp::max(f1.prec, f2.prec)),
            ),
            (NumberValue::Rational(r), NumberValue::Float(f)) => {
                let val = (r.num as f64 / r.den as f64).powf(f.value);
                NumberValue::Float(Float::from_f64(val, f.prec))
            }
            (NumberValue::Float(f), NumberValue::Rational(r)) => {
                let val = f.value.powf(r.num as f64 / r.den as f64);
                NumberValue::Float(Float::from_f64(val, f.prec))
            }
            _ => {
                let f1 = to_float_val(self);
                let f2 = to_float_val(other);
                NumberValue::Float(Float::from_f64(
                    f1.value.powf(f2.value),
                    std::cmp::max(f1.prec, f2.prec),
                ))
            }
        }
    }

    /// Square root of the value.
    pub fn sqrt(&self) -> Self {
        match self {
            NumberValue::Rational(r) => {
                let val = (r.num as f64 / r.den as f64).sqrt();
                NumberValue::Float(Float::from_f64(val, 53))
            }
            NumberValue::Float(f) => NumberValue::Float(Float::from_f64(f.value.sqrt(), f.prec)),
            NumberValue::Interval { lower, upper } => NumberValue::Interval {
                lower: Float::from_f64(lower.value.sqrt(), lower.prec),
                upper: Float::from_f64(upper.value.sqrt(), upper.prec),
            },
            NumberValue::Uncertainty {
                value,
                uncertainty,
                is_relative,
            } => {
                let val = value.sqrt();
                let two = NumberValue::Rational(Rational::from_i32(2));
                let unc = uncertainty.div(&two.mul(&val));
                NumberValue::Uncertainty {
                    value: Box::new(val),
                    uncertainty: Box::new(unc),
                    is_relative: *is_relative,
                }
            }
            _ => {
                let f = to_float_val(self);
                NumberValue::Float(Float::from_f64(f.value.sqrt(), f.prec))
            }
        }
    }

    /// Natural logarithm of the value.
    pub fn ln(&self) -> Self {
        match self {
            NumberValue::Rational(r) => {
                let val = (r.num as f64 / r.den as f64).ln();
                NumberValue::Float(Float::from_f64(val, 53))
            }
            NumberValue::Float(f) => NumberValue::Float(Float::from_f64(f.value.ln(), f.prec)),
            NumberValue::Interval { lower, upper } => NumberValue::Interval {
                lower: Float::from_f64(lower.value.ln(), lower.prec),
                upper: Float::from_f64(upper.value.ln(), upper.prec),
            },
            NumberValue::Uncertainty {
                value,
                uncertainty,
                is_relative,
            } => {
                let val = value.ln();
                let unc = uncertainty.div(&value.abs());
                NumberValue::Uncertainty {
                    value: Box::new(val),
                    uncertainty: Box::new(unc),
                    is_relative: *is_relative,
                }
            }
            _ => {
                let f = to_float_val(self);
                NumberValue::Float(Float::from_f64(f.value.ln(), f.prec))
            }
        }
    }

    /// Absolute value.
    pub fn abs(&self) -> Self {
        match self {
            NumberValue::Rational(r) => NumberValue::Rational(Rational::new(r.num.abs(), r.den)),
            NumberValue::Float(f) => NumberValue::Float(Float::from_f64(f.value.abs(), f.prec)),
            NumberValue::Interval { lower, upper } => {
                let l = lower.value;
                let u = upper.value;
                let min_val = if l <= 0.0 && u >= 0.0 {
                    0.0
                } else {
                    f64::min(l.abs(), u.abs())
                };
                let max_val = f64::max(l.abs(), u.abs());
                NumberValue::Interval {
                    lower: Float::from_f64(min_val, lower.prec),
                    upper: Float::from_f64(max_val, upper.prec),
                }
            }
            NumberValue::Uncertainty {
                value,
                uncertainty,
                is_relative,
            } => NumberValue::Uncertainty {
                value: Box::new(value.abs()),
                uncertainty: Box::new((**uncertainty).clone()),
                is_relative: *is_relative,
            },
            NumberValue::PlusInfinity | NumberValue::MinusInfinity => NumberValue::PlusInfinity,
            NumberValue::NaN => NumberValue::NaN,
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
                is_relative: ir1,
            },
            NumberValue::Uncertainty {
                value: v2,
                uncertainty: u2,
                is_relative: ir2,
            },
        ) => eq_values(v1, v2) && eq_values(u1, u2) && ir1 == ir2,
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

impl Number {
    /// Add two Numbers mathematically.
    pub fn add(&self, other: &Self) -> Self {
        let (a, b) = self.to_canonical_real_imag();
        let (c, d) = other.to_canonical_real_imag();
        let r = a.add(&c);
        let i = b.add(&d);
        Number::new_complex(
            Number {
                value: r,
                imaginary: None,
                precision: std::cmp::max(self.precision, other.precision),
                approximate: self.approximate || other.approximate,
                is_imaginary: false,
            },
            Number {
                value: i,
                imaginary: None,
                precision: std::cmp::max(self.precision, other.precision),
                approximate: self.approximate || other.approximate,
                is_imaginary: true,
            },
        )
    }

    /// Subtract two Numbers mathematically.
    pub fn sub(&self, other: &Self) -> Self {
        let (a, b) = self.to_canonical_real_imag();
        let (c, d) = other.to_canonical_real_imag();
        let r = a.sub(&c);
        let i = b.sub(&d);
        Number::new_complex(
            Number {
                value: r,
                imaginary: None,
                precision: std::cmp::max(self.precision, other.precision),
                approximate: self.approximate || other.approximate,
                is_imaginary: false,
            },
            Number {
                value: i,
                imaginary: None,
                precision: std::cmp::max(self.precision, other.precision),
                approximate: self.approximate || other.approximate,
                is_imaginary: true,
            },
        )
    }

    /// Multiply two Numbers mathematically.
    pub fn mul(&self, other: &Self) -> Self {
        let (a, b) = self.to_canonical_real_imag();
        let (c, d) = other.to_canonical_real_imag();
        let r = a.mul(&c).sub(&b.mul(&d));
        let i = a.mul(&d).add(&b.mul(&c));
        Number::new_complex(
            Number {
                value: r,
                imaginary: None,
                precision: std::cmp::max(self.precision, other.precision),
                approximate: self.approximate || other.approximate,
                is_imaginary: false,
            },
            Number {
                value: i,
                imaginary: None,
                precision: std::cmp::max(self.precision, other.precision),
                approximate: self.approximate || other.approximate,
                is_imaginary: true,
            },
        )
    }

    /// Divide two Numbers mathematically.
    pub fn div(&self, other: &Self) -> Self {
        let (a, b) = self.to_canonical_real_imag();
        let (c, d) = other.to_canonical_real_imag();
        let denom = c.mul(&c).add(&d.mul(&d));
        let r = a.mul(&c).add(&b.mul(&d)).div(&denom);
        let i = b.mul(&c).sub(&a.mul(&d)).div(&denom);
        Number::new_complex(
            Number {
                value: r,
                imaginary: None,
                precision: std::cmp::max(self.precision, other.precision),
                approximate: self.approximate || other.approximate,
                is_imaginary: false,
            },
            Number {
                value: i,
                imaginary: None,
                precision: std::cmp::max(self.precision, other.precision),
                approximate: self.approximate || other.approximate,
                is_imaginary: true,
            },
        )
    }

    /// Exponentiate one Number to another.
    pub fn pow(&self, other: &Self) -> Self {
        let (a, b) = self.to_canonical_real_imag();
        let (c, d) = other.to_canonical_real_imag();
        if b.is_real_zero() && d.is_real_zero() {
            let r = a.pow(&c);
            Number {
                value: r,
                imaginary: None,
                precision: std::cmp::max(self.precision, other.precision),
                approximate: self.approximate || other.approximate,
                is_imaginary: false,
            }
        } else {
            let f1 = to_float_val(&a);
            let f2 = to_float_val(&c);
            let val = NumberValue::Float(Float::from_f64(f1.value.powf(f2.value), 53));
            Number {
                value: val,
                imaginary: None,
                precision: 53,
                approximate: true,
                is_imaginary: false,
            }
        }
    }
}

fn format_uncertainty(val: f64, unc: f64) -> (String, String) {
    if unc == 0.0 {
        return (val.to_string(), "0".to_string());
    }
    let unc_abs = unc.abs();
    let e_init = unc_abs.log10().floor() as i32;
    let d_init = std::cmp::max(0, 1 - e_init);
    let factor = 10.0f64.powi(d_init);
    let rounded_unc = (unc_abs * factor).round() / factor;

    let e = if rounded_unc == 0.0 {
        e_init
    } else {
        rounded_unc.log10().floor() as i32
    };
    let d = std::cmp::max(0, 1 - e) as usize;

    let formatted_unc = format!("{:.width$}", rounded_unc, width = d);
    let formatted_val = format!("{:.width$}", val, width = d);
    (formatted_val, formatted_unc)
}

fn format_relative_uncertainty(val: f64, unc_abs: f64) -> (String, String) {
    if val == 0.0 {
        return (val.to_string(), "0%".to_string());
    }
    let p = (unc_abs / val.abs()) * 100.0;
    if p == 0.0 {
        return (val.to_string(), "0%".to_string());
    }
    let e_init = p.log10().floor() as i32;
    let d_init = std::cmp::max(0, 1 - e_init);
    let factor = 10.0f64.powi(d_init);
    let rounded_p = (p * factor).round() / factor;

    let e = if rounded_p == 0.0 {
        e_init
    } else {
        rounded_p.log10().floor() as i32
    };
    let d = std::cmp::max(0, 1 - e) as usize;

    let formatted_p = format!("{:.width$}%", rounded_p, width = d);
    let formatted_val = format!("{:.width$}", val, width = d);
    (formatted_val, formatted_p)
}

impl std::fmt::Display for NumberValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NumberValue::Rational(r) => {
                if r.den == 1 {
                    write!(f, "{}", r.num)
                } else {
                    write!(f, "{}/{}", r.num, r.den)
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
                let val_f = to_float_val(value).value;
                let unc_f = to_float_val(uncertainty).value;
                if *is_relative {
                    let (formatted_val, formatted_unc) = format_relative_uncertainty(val_f, unc_f);
                    write!(f, "{}±{}", formatted_val, formatted_unc)
                } else {
                    let (formatted_val, formatted_unc) = format_uncertainty(val_f, unc_f);
                    write!(f, "{}±{}", formatted_val, formatted_unc)
                }
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
        NumberValue::Rational(r) => r.num < 0,
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

    if let Some(slash_idx) = s.find('/') {
        let num_str = s[..slash_idx].trim();
        let den_str = s[slash_idx + 1..].trim();
        if let (Ok(num), Ok(den)) = (num_str.parse::<i128>(), den_str.parse::<i128>()) {
            if den != 0 {
                return Ok(NumberValue::Rational(Rational::new(num, den)));
            }
        }
    }

    if let Ok(val) = s.parse::<i128>() {
        return Ok(NumberValue::Rational(Rational::new(val, 1)));
    }
    if let Ok(val) = s.parse::<f64>() {
        return Ok(NumberValue::Float(Float::from_f64(val, 53)));
    }
    Err(format!("Failed to parse number: {s}"))
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
    if !first.is_ascii_digit() && first != '.' && first != '-' {
        return None;
    }
    if first == '-' && chars.len() > 1 && !chars[1].is_ascii_digit() && chars[1] != '.' {
        return None;
    }

    let mut in_parenthesis = false;
    while len < chars.len() {
        let c = chars[len];
        if c.is_ascii_digit() || c == '.' || c == 'e' || c == 'E' || c == '%' || c == 'i' {
            len += 1;
        } else if c == '(' {
            in_parenthesis = true;
            len += 1;
        } else if c == ')' {
            in_parenthesis = false;
            len += 1;
        } else if in_parenthesis {
            len += 1;
        } else if c == '+'
            && len + 2 < chars.len()
            && chars[len + 1] == '/'
            && chars[len + 2] == '-'
        {
            len += 3;
        } else if c == '±'
            || (c == '-'
                && len > 0
                && (chars[len - 1] == 'e' || chars[len - 1] == 'E' || chars[len - 1] == '/'))
        {
            len += 1;
        } else {
            break;
        }
    }
    if len > 0 {
        Some((&s[..len], &s[len..]))
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
                let u_abs = value.abs().mul(&unc_pct).div(&hundred);
                return Ok(Number::new_uncertainty(value, u_abs, true));
            } else {
                let unc = parse_single_value(u_clean)?;
                let unc_abs = unc.abs();
                return Ok(Number::new_uncertainty(value, unc_abs, false));
            }
        }

        if let Some(open_idx) = s.find('(') {
            if let Some(close_idx) = s.find(')') {
                if close_idx > open_idx + 1 {
                    let v_str = s[..open_idx].trim();
                    let u_str = s[open_idx + 1..close_idx].trim();
                    let value = parse_single_value(v_str)?;
                    let u_raw = parse_single_value(u_str)?;
                    let d = if let Some(dot_idx) = v_str.find('.') {
                        let after_dot = &v_str[dot_idx + 1..];
                        after_dot.chars().take_while(|c| c.is_ascii_digit()).count()
                    } else {
                        0
                    };
                    let ten = NumberValue::Rational(Rational::from_i32(10));
                    let d_val = NumberValue::Rational(Rational::from_i32(d as i32));
                    let factor = ten.pow(&d_val.negate());
                    let u_abs = u_raw.mul(&factor);
                    return Ok(Number::new_uncertainty(value, u_abs, false));
                }
            }
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

/// Evaluates a basic mathematical expression containing numbers, arithmetic operators, parentheses, and uncertainty.
pub fn evaluate_expr(s: &str) -> Result<Number, String> {
    let mut tokens = Vec::new();
    let mut rest = s.trim();
    while !rest.is_empty() {
        if let Some((lit, remaining)) = next_literal(rest) {
            tokens.push(Token::Literal(std::str::FromStr::from_str(lit)?));
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
            rest = rest[c.len_utf8()..].trim_start();
        }
    }

    let mut parser = ExprParser { tokens, pos: 0 };
    parser.parse_expr(0)
}

#[derive(Debug, Clone)]
enum Token {
    Literal(Number),
    OpAdd,
    OpSub,
    OpMul,
    OpDiv,
    OpPow,
    LParen,
    RParen,
}

struct ExprParser {
    tokens: Vec<Token>,
    pos: usize,
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
        match self.next_token() {
            Some(Token::Literal(num)) => Ok(num.clone()),
            Some(Token::LParen) => {
                let expr = self.parse_expr(0)?;
                match self.next_token() {
                    Some(Token::RParen) => Ok(expr),
                    _ => Err("Expected matching ')'".to_string()),
                }
            }
            Some(Token::OpSub) => {
                let primary = self.parse_primary()?;
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

    fn parse_expr(&mut self, min_prec: u8) -> Result<Number, String> {
        let mut lhs = self.parse_primary()?;

        while let Some(tok) = self.peek() {
            let (op_prec, assoc) = match tok {
                Token::OpAdd | Token::OpSub => (1, Assoc::Left),
                Token::OpMul | Token::OpDiv => (2, Assoc::Left),
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
                Token::OpPow => lhs.pow(&rhs),
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

#[cfg(test)]
mod tests;
