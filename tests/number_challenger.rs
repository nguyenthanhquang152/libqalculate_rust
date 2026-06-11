use libqalculate_rust::number::Rational;
use proptest::prelude::*;

fn oracle_add(r1: &Rational, r2: &Rational) -> Rational {
    let lhs = rug_rational(r1);
    let rhs = rug_rational(r2);
    rational_from_rug(rug::Rational::from(&lhs + &rhs))
}

fn oracle_sub(r1: &Rational, r2: &Rational) -> Rational {
    let lhs = rug_rational(r1);
    let rhs = rug_rational(r2);
    rational_from_rug(rug::Rational::from(&lhs - &rhs))
}

fn oracle_mul(r1: &Rational, r2: &Rational) -> Rational {
    let lhs = rug_rational(r1);
    let rhs = rug_rational(r2);
    rational_from_rug(rug::Rational::from(&lhs * &rhs))
}

fn oracle_div(r1: &Rational, r2: &Rational) -> Option<Rational> {
    if r2.is_zero() {
        None
    } else {
        let lhs = rug_rational(r1);
        let rhs = rug_rational(r2);
        Some(rational_from_rug(rug::Rational::from(&lhs / &rhs)))
    }
}

fn rug_rational(rational: &Rational) -> rug::Rational {
    let numerator = rational
        .numerator_string()
        .parse::<rug::Integer>()
        .expect("Rational numerator should be a valid integer");
    let denominator = rational
        .denominator_string()
        .parse::<rug::Integer>()
        .expect("Rational denominator should be a valid integer");
    rug::Rational::from((numerator, denominator))
}

fn rational_from_rug(value: rug::Rational) -> Rational {
    value
        .to_string()
        .parse()
        .expect("rug rational should parse through the public Rational API")
}

fn rational_strategy() -> BoxedStrategy<Rational> {
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
    .boxed()
}

fn parsed_beyond_i128_rational_strategy() -> BoxedStrategy<Rational> {
    (0_u16..2048, 2_u8..19, any::<bool>())
        .prop_map(|(offset, denominator, negative)| {
            let mut numerator = rug::Integer::from(i128::MAX) + rug::Integer::from(offset) + 1_i32;
            if negative {
                numerator = -numerator;
            }
            format!("{numerator}/{denominator}")
                .parse()
                .expect("generated beyond-i128 rational should parse")
        })
        .boxed()
}

fn arbitrary_precision_rational_strategy() -> BoxedStrategy<Rational> {
    prop_oneof![rational_strategy(), parsed_beyond_i128_rational_strategy()].boxed()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(5000))]

    #[test]
    fn prop_test_checked_add_oracle(r1 in rational_strategy(), r2 in rational_strategy()) {
        let got = r1.checked_add(&r2);
        let expected = Some(oracle_add(&r1, &r2));
        prop_assert_eq!(got, expected);
    }

    #[test]
    fn prop_test_checked_sub_oracle(r1 in rational_strategy(), r2 in rational_strategy()) {
        let got = r1.checked_sub(&r2);
        let expected = Some(oracle_sub(&r1, &r2));
        prop_assert_eq!(got, expected);
    }

    #[test]
    fn prop_test_checked_mul_oracle(
        r1 in arbitrary_precision_rational_strategy(),
        r2 in arbitrary_precision_rational_strategy(),
    ) {
        let got = r1.checked_mul(&r2);
        let expected = Some(oracle_mul(&r1, &r2));
        prop_assert_eq!(got, expected);
    }

    #[test]
    fn prop_test_checked_div_oracle(
        r1 in arbitrary_precision_rational_strategy(),
        r2 in arbitrary_precision_rational_strategy().prop_filter("divisor not zero", |r| !r.is_zero()),
    ) {
        let got = r1.checked_div(&r2);
        let expected = oracle_div(&r1, &r2);
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
