use libqalculate_rust::ast::{ComparisonOperator, Expression};
use libqalculate_rust::parser::operators::{parse_expression, ParseErrorKind};

fn number_text(expr: &Expression) -> String {
    match expr {
        Expression::Number(number) => number.to_string(),
        other => panic!("expected number, got {other:?}"),
    }
}

fn symbol_name(expr: &Expression) -> &str {
    match expr {
        Expression::Symbolic(symbol) => symbol.name(),
        other => panic!("expected symbol, got {other:?}"),
    }
}

fn children(expr: &Expression) -> &[Expression] {
    match expr {
        Expression::Addition(children)
        | Expression::Multiplication(children)
        | Expression::BitwiseAnd(children)
        | Expression::BitwiseOr(children)
        | Expression::BitwiseXor(children)
        | Expression::LogicalAnd(children)
        | Expression::LogicalOr(children) => children.as_slice(),
        other => panic!("expected n-ary expression, got {other:?}"),
    }
}

#[test]
fn parses_arithmetic_precedence_and_right_associative_power() {
    let expr = parse_expression("1 + 2 * 3 ^ 4 ^ 5").expect("parse expression");
    let add = children(&expr);
    assert_eq!(number_text(&add[0]), "1");

    let mul = children(&add[1]);
    assert_eq!(number_text(&mul[0]), "2");

    let Expression::Power { base, exponent } = &mul[1] else {
        panic!("expected outer power, got {:?}", mul[1]);
    };
    assert_eq!(number_text(base), "3");
    let Expression::Power {
        base: inner_base,
        exponent: inner_exponent,
    } = exponent.as_ref()
    else {
        panic!("expected right-associated inner power, got {exponent:?}");
    };
    assert_eq!(number_text(inner_base), "4");
    assert_eq!(number_text(inner_exponent), "5");
}

#[test]
fn parses_prefix_postfix_and_implicit_multiplication() {
    let expr = parse_expression("-2x(3 + 4)!").expect("parse expression");
    let factors = children(&expr);
    assert_eq!(factors.len(), 3);

    let Expression::Negate(negated) = &factors[0] else {
        panic!("expected leading negation, got {:?}", factors[0]);
    };
    assert_eq!(number_text(negated), "2");
    assert_eq!(symbol_name(&factors[1]), "x");

    let Expression::Factorial(grouped) = &factors[2] else {
        panic!("expected postfix factorial, got {:?}", factors[2]);
    };
    let grouped_terms = children(grouped);
    assert_eq!(number_text(&grouped_terms[0]), "3");
    assert_eq!(number_text(&grouped_terms[1]), "4");

    let plus = parse_expression("+2").expect("parse unary plus");
    assert_eq!(number_text(&plus), "2");
}

#[test]
fn preserves_adaptive_implicit_multiplication_spacing() {
    let tight = parse_expression("1/5x").expect("parse tight implicit product");
    let Expression::Division {
        numerator,
        denominator,
    } = tight
    else {
        panic!("expected division root, got {tight:?}");
    };
    assert_eq!(number_text(&numerator), "1");
    let denominator_factors = children(&denominator);
    assert_eq!(number_text(&denominator_factors[0]), "5");
    assert_eq!(symbol_name(&denominator_factors[1]), "x");

    let spaced = parse_expression("1/5 x").expect("parse spaced implicit product");
    let factors = children(&spaced);
    assert!(matches!(factors[0], Expression::Division { .. }));
    assert_eq!(symbol_name(&factors[1]), "x");
}

#[test]
fn parses_remainder_modulo_integer_division_and_percent_nodes() {
    let rem = parse_expression("6%2").expect("parse remainder");
    let Expression::Remainder { lhs, rhs } = rem else {
        panic!("expected remainder node, got {rem:?}");
    };
    assert_eq!(number_text(&lhs), "6");
    assert_eq!(number_text(&rhs), "2");

    let signed_rem = parse_expression("6%-2").expect("parse signed remainder");
    let Expression::Remainder { lhs, rhs } = signed_rem else {
        panic!("expected signed remainder node, got {signed_rem:?}");
    };
    assert_eq!(number_text(&lhs), "6");
    assert!(matches!(rhs.as_ref(), Expression::Negate(_)));

    let positive_rem = parse_expression("6%+2").expect("parse positive remainder");
    let Expression::Remainder { lhs, rhs } = positive_rem else {
        panic!("expected positive remainder node, got {positive_rem:?}");
    };
    assert_eq!(number_text(&lhs), "6");
    assert_eq!(number_text(&rhs), "2");

    let modulo = parse_expression("3 mod -2").expect("parse modulo");
    let Expression::Modulo { lhs, rhs } = modulo else {
        panic!("expected modulo node, got {modulo:?}");
    };
    assert_eq!(number_text(&lhs), "3");
    assert!(matches!(rhs.as_ref(), Expression::Negate(_)));

    let int_div = parse_expression(r"5\2").expect("parse integer division");
    let Expression::IntegerDivision { lhs, rhs } = int_div else {
        panic!("expected integer division node, got {int_div:?}");
    };
    assert_eq!(number_text(&lhs), "5");
    assert_eq!(number_text(&rhs), "2");

    let percentage = parse_expression("100 + 10%").expect("parse percentage");
    let terms = children(&percentage);
    assert_eq!(number_text(&terms[0]), "100");
    assert!(matches!(terms[1], Expression::Percent(_)));

    let percentage_difference = parse_expression("10%-6%").expect("parse percentage subtraction");
    let terms = children(&percentage_difference);
    assert!(matches!(terms[0], Expression::Percent(_)));
    let Expression::Negate(rhs) = &terms[1] else {
        panic!("expected subtraction term, got {:?}", terms[1]);
    };
    assert!(matches!(rhs.as_ref(), Expression::Percent(_)));

    let grouped_percentage_difference =
        parse_expression("10%-((6))%").expect("parse grouped percentage subtraction");
    let terms = children(&grouped_percentage_difference);
    assert!(matches!(terms[0], Expression::Percent(_)));
    let Expression::Negate(rhs) = &terms[1] else {
        panic!("expected grouped subtraction term, got {:?}", terms[1]);
    };
    assert!(matches!(rhs.as_ref(), Expression::Percent(_)));

    let grouped_remainder = parse_expression("6%-((2))").expect("parse grouped signed remainder");
    let Expression::Remainder { lhs, rhs } = grouped_remainder else {
        panic!("expected grouped signed remainder, got {grouped_remainder:?}");
    };
    assert_eq!(number_text(&lhs), "6");
    assert!(matches!(rhs.as_ref(), Expression::Negate(_)));
}

#[test]
fn parses_comparison_logical_and_bitwise_precedence() {
    let comparison = parse_expression("1 + 2 <= 3 * 4").expect("parse comparison");
    let Expression::Comparison { op, lhs, rhs } = comparison else {
        panic!("expected comparison, got {comparison:?}");
    };
    assert_eq!(op, ComparisonOperator::LessOrEqual);
    assert!(matches!(lhs.as_ref(), Expression::Addition(_)));
    assert!(matches!(rhs.as_ref(), Expression::Multiplication(_)));

    let logical = parse_expression("not a or b and c").expect("parse logical");
    let and_terms = children(&logical);
    assert_eq!(and_terms.len(), 2);
    let or_terms = children(&and_terms[0]);
    assert!(matches!(or_terms[0], Expression::LogicalNot(_)));
    assert_eq!(symbol_name(&or_terms[1]), "b");
    assert_eq!(symbol_name(&and_terms[1]), "c");

    let bitwise_xor_word =
        parse_expression("0b1011 0010 xor 0b0111 0001").expect("parse word xor as bitwise xor");
    let xor_terms = children(&bitwise_xor_word);
    assert_eq!(number_text(&xor_terms[0]), "178");
    assert_eq!(number_text(&xor_terms[1]), "113");

    let bitwise_not_unicode = parse_expression("¬1").expect("parse unicode bitwise not");
    let Expression::BitwiseNot(not_operand) = bitwise_not_unicode else {
        panic!("expected bitwise-not root, got {bitwise_not_unicode:?}");
    };
    assert_eq!(number_text(&not_operand), "1");

    let bitwise =
        parse_expression("0b0101 | 0b1001 ^^ 0b0111 & 0b0011").expect("parse bitwise precedence");
    let bitwise_or = children(&bitwise);
    assert_eq!(bitwise_or.len(), 2);
    assert!(matches!(bitwise_or[1], Expression::BitwiseXor(_)));

    let shifted = parse_expression("18 << 1 >> 2").expect("parse bitwise shifts");
    let Expression::ShiftRight { lhs, rhs } = shifted else {
        panic!("expected shift-right root, got {shifted:?}");
    };
    assert!(matches!(lhs.as_ref(), Expression::ShiftLeft { .. }));
    assert_eq!(number_text(&rhs), "2");

    let shift_vs_comparison = parse_expression("1 << 2 < 3").expect("parse shift comparison");
    let Expression::Comparison { op, lhs, rhs } = shift_vs_comparison else {
        panic!("expected comparison root, got {shift_vs_comparison:?}");
    };
    assert_eq!(op, ComparisonOperator::Less);
    assert!(matches!(lhs.as_ref(), Expression::ShiftLeft { .. }));
    assert_eq!(number_text(&rhs), "3");

    let shift_vs_bitwise = parse_expression("1 & 2 << 3").expect("parse bitwise shift precedence");
    let bitwise_terms = children(&shift_vs_bitwise);
    assert_eq!(number_text(&bitwise_terms[0]), "1");
    assert!(matches!(bitwise_terms[1], Expression::ShiftLeft { .. }));
}

#[test]
fn preserves_power_boundaries_around_postfix_and_lower_precedence_operators() {
    let factorial_exponent = parse_expression("2 ^ 3!").expect("parse factorial exponent");
    let Expression::Power { base, exponent } = factorial_exponent else {
        panic!("expected power root, got {factorial_exponent:?}");
    };
    assert_eq!(number_text(&base), "2");
    assert!(matches!(exponent.as_ref(), Expression::Factorial(_)));

    let percent_exponent = parse_expression("2 ^ 10%").expect("parse percent exponent");
    let Expression::Power { base, exponent } = percent_exponent else {
        panic!("expected power root, got {percent_exponent:?}");
    };
    assert_eq!(number_text(&base), "2");
    assert!(matches!(exponent.as_ref(), Expression::Percent(_)));

    let remainder_after_power = parse_expression("2 ^ 6%2").expect("parse remainder after power");
    let Expression::Remainder { lhs, rhs } = remainder_after_power else {
        panic!("expected remainder root, got {remainder_after_power:?}");
    };
    assert!(matches!(lhs.as_ref(), Expression::Power { .. }));
    assert_eq!(number_text(&rhs), "2");

    let remainder_inside_sum = parse_expression("1 + 6%2").expect("parse remainder inside sum");
    let terms = children(&remainder_inside_sum);
    assert_eq!(number_text(&terms[0]), "1");
    assert!(matches!(terms[1], Expression::Remainder { .. }));

    let implicit_product_after_power =
        parse_expression("2 ^ 3x").expect("parse implicit product after power");
    let factors = children(&implicit_product_after_power);
    assert_eq!(factors.len(), 2);
    assert!(matches!(factors[0], Expression::Power { .. }));
    assert_eq!(symbol_name(&factors[1]), "x");

    let implicit_product_inside_sum =
        parse_expression("1 + 2x").expect("parse implicit product inside sum");
    let terms = children(&implicit_product_inside_sum);
    assert_eq!(number_text(&terms[0]), "1");
    assert!(matches!(terms[1], Expression::Multiplication(_)));
}

#[test]
fn parses_base_prefixed_literals_visible_to_operator_parser() {
    let duodecimal = parse_expression("0de + 0dx").expect("parse duodecimal aliases");
    let terms = children(&duodecimal);
    assert_eq!(number_text(&terms[0]), "10");
    assert_eq!(number_text(&terms[1]), "11");

    let large_hex = parse_expression("0x100000000000000000000000000000000")
        .expect("parse base-prefixed integer larger than i128");
    assert_eq!(
        number_text(&large_hex),
        "340282366920938463463374607431768211456"
    );

    let grouped_integer = parse_expression("123 456").expect("parse grouped integer");
    assert_eq!(number_text(&grouped_integer), "123456");

    let grouped_decimal = parse_expression("0 . 001").expect("parse grouped decimal");
    assert_eq!(number_text(&grouped_decimal), "0.001");
}

#[test]
fn parses_reduced_operator_fixture_rows_without_evaluating() {
    // Reduced rows from:
    // ../libqalculate/tests/operators.batch:1-67
    // ../libqalculate/tests/bitwise.batch:11-64
    // ../libqalculate/tests/percentages.batch:1-61
    for source in [
        "1 plus 2",
        "5--2",
        "5---2",
        "-5-2",
        "3 × 4",
        "4 times 5",
        "12 per 3",
        "7 rem 2",
        "3 %% -2",
        "5 div 2",
        "5 ** 3",
        "4 ** 3 ** 2",
        "5!",
        "18 >> 2",
        "18 << 1",
        "~ -812",
        "0b0101 | 0b1001",
        "0b1011 0010 ∧ 0b0111 0001",
        "0b1011 0010 xor 0b0111 0001",
        "0b1011 0010 ⊻ 0b0111 0001",
        "10% + 5%",
        "10%-6%",
        "100 + 10%",
        "100 + (10 + 10)%",
        "100 - 10% - 10%",
        "10 - x% = 8",
    ] {
        parse_expression(source).unwrap_or_else(|err| panic!("{source}: {err}"));
    }
}

#[test]
fn returns_structured_errors_for_invalid_operator_syntax() {
    let dangling = parse_expression("1 +").expect_err("dangling operator is invalid");
    assert_eq!(dangling.kind(), ParseErrorKind::UnexpectedEnd);
    assert_eq!(dangling.span().range(), 3..3);
    let display = dangling.to_string();
    assert!(display.contains("UnexpectedEnd"));
    assert!(display.contains("3..3"));

    let grouping = parse_expression("(1 + 2").expect_err("missing close paren is invalid");
    assert_eq!(grouping.kind(), ParseErrorKind::UnclosedGroup);
    assert_eq!(grouping.span().range(), 0..1);

    let wrong_close = parse_expression("(1 + 2]").expect_err("wrong close delimiter is invalid");
    assert_eq!(wrong_close.kind(), ParseErrorKind::UnclosedGroup);
    assert_eq!(wrong_close.span().range(), 0..1);

    let unsupported = parse_expression("1 -> m").expect_err("conversion is not task 3.3");
    assert!(matches!(
        unsupported.kind(),
        ParseErrorKind::UnsupportedOperator(_)
    ));
}
