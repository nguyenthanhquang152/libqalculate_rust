use libqalculate_rust::number::{Rational, U256};
use proptest::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct I256 {
    neg: bool,
    abs: U256,
}

impl I256 {
    fn add(self, other: Self) -> Self {
        if self.neg == other.neg {
            Self {
                neg: self.neg,
                abs: self.abs.add(other.abs),
            }
        } else if self.abs >= other.abs {
            Self {
                neg: self.neg,
                abs: self.abs.sub(other.abs),
            }
        } else {
            Self {
                neg: other.neg,
                abs: other.abs.sub(self.abs),
            }
        }
    }

    fn sub(self, other: Self) -> Self {
        let neg_other = Self {
            neg: !other.neg,
            abs: other.abs,
        };
        self.add(neg_other)
    }
}

fn gcd_u256(mut a: U256, mut b: U256) -> U256 {
    while !b.is_zero() {
        let r = a.div_rem(b).1;
        a = b;
        b = r;
    }
    a
}

fn u128_gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let temp = b;
        b = a % b;
        a = temp;
    }
    a
}

fn oracle_add(r1: &Rational, r2: &Rational) -> Option<Rational> {
    let a = r1.num();
    let b = r1.den() as u128;
    let c = r2.num();
    let d = r2.den() as u128;

    let g = u128_gcd(b, d);
    let b_prime = b / g;
    let d_prime = d / g;

    let term1 = I256 {
        neg: a < 0,
        abs: U256::mul_u128(a.unsigned_abs(), d_prime),
    };
    let term2 = I256 {
        neg: c < 0,
        abs: U256::mul_u128(c.unsigned_abs(), b_prime),
    };

    let num_256 = term1.add(term2);
    let den_256 = U256::mul_u128(b, d_prime);

    if den_256.is_zero() {
        return None;
    }

    let g2 = gcd_u256(num_256.abs, den_256);
    let reduced_num = num_256.abs.div_rem(g2).0;
    let reduced_den = den_256.div_rem(g2).0;

    let num_limit = if num_256.neg {
        i128::MIN.unsigned_abs()
    } else {
        i128::MAX as u128
    };

    if !reduced_num.fits_in_u128() || reduced_num.as_u128() > num_limit {
        return None;
    }
    if !reduced_den.fits_in_u128() || reduced_den.as_u128() > i128::MAX as u128 {
        return None;
    }

    let n = if num_256.neg {
        let val = reduced_num.as_u128();
        if val == i128::MIN.unsigned_abs() {
            i128::MIN
        } else {
            -(val as i128)
        }
    } else {
        reduced_num.as_u128() as i128
    };
    let d_val = reduced_den.as_u128() as i128;

    Rational::try_new(n, d_val)
}

fn oracle_sub(r1: &Rational, r2: &Rational) -> Option<Rational> {
    let a = r1.num();
    let b = r1.den() as u128;
    let c = r2.num();
    let d = r2.den() as u128;

    let g = u128_gcd(b, d);
    let b_prime = b / g;
    let d_prime = d / g;

    let term1 = I256 {
        neg: a < 0,
        abs: U256::mul_u128(a.unsigned_abs(), d_prime),
    };
    let term2 = I256 {
        neg: c < 0,
        abs: U256::mul_u128(c.unsigned_abs(), b_prime),
    };

    let num_256 = term1.sub(term2);
    let den_256 = U256::mul_u128(b, d_prime);

    if den_256.is_zero() {
        return None;
    }

    let g2 = gcd_u256(num_256.abs, den_256);
    let reduced_num = num_256.abs.div_rem(g2).0;
    let reduced_den = den_256.div_rem(g2).0;

    let num_limit = if num_256.neg {
        i128::MIN.unsigned_abs()
    } else {
        i128::MAX as u128
    };

    if !reduced_num.fits_in_u128() || reduced_num.as_u128() > num_limit {
        return None;
    }
    if !reduced_den.fits_in_u128() || reduced_den.as_u128() > i128::MAX as u128 {
        return None;
    }

    let n = if num_256.neg {
        let val = reduced_num.as_u128();
        if val == i128::MIN.unsigned_abs() {
            i128::MIN
        } else {
            -(val as i128)
        }
    } else {
        reduced_num.as_u128() as i128
    };
    let d_val = reduced_den.as_u128() as i128;

    Rational::try_new(n, d_val)
}

fn rational_strategy() -> impl Strategy<Value = Rational> {
    prop_oneof![
        // Small numbers
        (
            any::<i32>(),
            any::<i32>().prop_filter("den not zero", |d| *d != 0)
        )
            .prop_map(|(n, d)| Rational::try_new(n as i128, d as i128).unwrap()),
        // i64 range
        (
            any::<i64>(),
            any::<i64>().prop_filter("den not zero", |d| *d != 0)
        )
            .prop_map(|(n, d)| Rational::try_new(n as i128, d as i128).unwrap()),
        // i128 range, filtered to valid ones
        (
            any::<i128>(),
            any::<i128>().prop_filter("den not zero", |d| *d != 0)
        )
            .prop_filter_map("valid rational", |(n, d)| Rational::try_new(n, d)),
        // Specific boundary values
        prop_oneof![
            Just(Rational::try_new(i128::MAX, 1).unwrap()),
            Just(Rational::try_new(i128::MIN, 1).unwrap()),
            Just(Rational::try_new(i128::MAX, 2).unwrap()),
            Just(Rational::try_new(i128::MIN, 2).unwrap()),
            Just(Rational::try_new(i128::MAX - 1, 1).unwrap()),
            Just(Rational::try_new(i128::MIN + 1, 1).unwrap()),
            Just(Rational::try_new(1, i128::MAX).unwrap()),
            Just(Rational::try_new(1, i128::MAX - 1).unwrap()),
            Just(Rational::try_new(0, 1).unwrap()),
        ]
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(5000))]

    #[test]
    fn prop_test_checked_add_oracle(r1 in rational_strategy(), r2 in rational_strategy()) {
        let got = r1.checked_add(&r2);
        let expected = oracle_add(&r1, &r2);
        prop_assert_eq!(got, expected);
    }

    #[test]
    fn prop_test_checked_sub_oracle(r1 in rational_strategy(), r2 in rational_strategy()) {
        let got = r1.checked_sub(&r2);
        let expected = oracle_sub(&r1, &r2);
        prop_assert_eq!(got, expected);
    }

    #[test]
    fn prop_test_commutativity(r1 in rational_strategy(), r2 in rational_strategy()) {
        let got1 = r1.checked_add(&r2);
        let got2 = r2.checked_add(&r1);
        prop_assert_eq!(got1, got2);
    }

    #[test]
    fn prop_test_sub_negation(r1 in rational_strategy(), r2 in rational_strategy()) {
        if r2.num() != i128::MIN {
            let r2_neg = Rational::try_new(-r2.num(), r2.den());
            if let Some(negated) = r2_neg {
                let got_sub = r1.checked_sub(&r2);
                let got_add_neg = r1.checked_add(&negated);
                prop_assert_eq!(got_sub, got_add_neg);
            }
        }
    }

    #[test]
    fn prop_test_identity(r in rational_strategy()) {
        let zero = Rational::try_new(0, 1).unwrap();
        prop_assert_eq!(r.checked_add(&zero), Some(r.clone()));
        prop_assert_eq!(r.checked_sub(&zero), Some(r.clone()));
        prop_assert_eq!(zero.checked_add(&r), Some(r.clone()));
    }

    #[test]
    fn prop_test_self_sub(r in rational_strategy()) {
        let zero = Rational::try_new(0, 1).unwrap();
        prop_assert_eq!(r.checked_sub(&r), Some(zero));
    }
}
