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

    // With corrected qalc precedence, AND binds tighter than OR:
    // `not a or b and c` → `(not a) or (b and c)`
    let logical = parse_expression("not a or b and c").expect("parse logical");
    let or_terms = children(&logical);
    assert_eq!(or_terms.len(), 2);
    assert!(matches!(or_terms[0], Expression::LogicalNot(_)));
    let and_terms = children(&or_terms[1]);
    assert_eq!(symbol_name(&and_terms[0]), "b");
    assert_eq!(symbol_name(&and_terms[1]), "c");

    // `xor` word operator produces bitwise XOR at parse time.
    // Promotion to logical XOR is deferred to evaluation.
    let xor_bitwise =
        parse_expression("a xor b").expect("parse word xor as bitwise xor");
    let xor_terms = children(&xor_bitwise);
    assert_eq!(xor_terms.len(), 2);
    assert!(matches!(xor_bitwise, Expression::BitwiseXor(_)));
    assert_eq!(symbol_name(&xor_terms[0]), "a");
    assert_eq!(symbol_name(&xor_terms[1]), "b");

    // `||` without units is parsed as LogicalOr.
    let logical_or = parse_expression("1>2 || 2>1").expect("parse || as logical or");
    let Expression::LogicalOr(terms) = logical_or else {
        panic!("expected LogicalOr, got {logical_or:?}");
    };
    assert!(matches!(terms[0], Expression::Comparison { .. }));
    assert!(matches!(terms[1], Expression::Comparison { .. }));

    // `||` with units is parsed as Parallel.
    let parallel = parse_expression("10 Ω || 6 Ω").expect("parse || as parallel");
    let Expression::Parallel { lhs, rhs } = parallel else {
        panic!("expected Parallel, got {parallel:?}");
    };
    let lhs_terms = children(lhs.as_ref());
    assert_eq!(number_text(&lhs_terms[0]), "10");
    assert_eq!(symbol_name(&lhs_terms[1]), "Ω");
    let rhs_terms = children(rhs.as_ref());
    assert_eq!(number_text(&rhs_terms[0]), "6");
    assert_eq!(symbol_name(&rhs_terms[1]), "Ω");

    // Bitwise `xor` via `bitxor` or `^^` still uses BitwiseXor.
    let bitwise_xor_word =
        parse_expression("0b1011 0010 bitxor 0b0111 0001").expect("parse bitxor as bitwise xor");
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

// ============================================================================
// PR #117 review fix tests
// ============================================================================

#[test]
fn trailing_comments_are_ignored() {
    // Comment 14: `1 + 2 # note` must parse as `1 + 2`, ignoring the comment.
    let expr = parse_expression("1 + 2 # note").expect("trailing comment should be ignored");
    let terms = children(&expr);
    assert_eq!(terms.len(), 2);
    assert_eq!(number_text(&terms[0]), "1");
    assert_eq!(number_text(&terms[1]), "2");

    // Comment at end of number expression.
    let expr2 = parse_expression("42 # the answer").expect("comment after single value");
    assert_eq!(number_text(&expr2), "42");
}

#[test]
fn pipe_pipe_parses_as_logical_or_or_parallel() {
    // Comment 20 / Comment 3446611880 / Comment 3446611881:
    // `||` represents logical OR when operands do not contain unit symbols.
    // If they contain units (like `ohm`), it parses as parallel sum.
    let expr_logical = parse_expression("a || b").expect("|| should parse as logical or");
    let Expression::LogicalOr(terms) = expr_logical else {
        panic!("expected LogicalOr, got {expr_logical:?}");
    };
    assert_eq!(symbol_name(&terms.as_slice()[0]), "a");
    assert_eq!(symbol_name(&terms.as_slice()[1]), "b");

    let expr_parallel = parse_expression("a ohm || b ohm").expect("|| should parse as parallel");
    let Expression::Parallel { lhs, rhs } = expr_parallel else {
        panic!("expected Parallel, got {expr_parallel:?}");
    };
    let lhs_terms = children(lhs.as_ref());
    assert_eq!(symbol_name(&lhs_terms[0]), "a");
    assert_eq!(symbol_name(&lhs_terms[1]), "ohm");
    let rhs_terms = children(rhs.as_ref());
    assert_eq!(symbol_name(&rhs_terms[0]), "b");
    assert_eq!(symbol_name(&rhs_terms[1]), "ohm");
}

#[test]
fn xor_word_is_bitwise_xor() {
    // Comment 17: `xor` word operator should produce BitwiseXor.
    // Upstream treats `xor` as bitwise XOR at parse time,
    // with promotion to logical XOR deferred to evaluation.
    let expr = parse_expression("a xor b").expect("xor should be bitwise");
    let terms = children(&expr);
    assert_eq!(terms.len(), 2);
    assert!(matches!(expr, Expression::BitwiseXor(_)));
    assert_eq!(symbol_name(&terms[0]), "a");
    assert_eq!(symbol_name(&terms[1]), "b");

    // `bitxor` also remains BitwiseXor.
    let bitwise = parse_expression("a bitxor b").expect("bitxor should be bitwise");
    let terms = children(&bitwise);
    assert_eq!(terms.len(), 2);
    assert!(matches!(bitwise, Expression::BitwiseXor(_)));
}

#[test]
fn logical_precedence_matches_qalc_manual() {
    // Comment 6, 10: AND binds tighter than OR, XOR is loosest.
    // `a or b and c` → `a or (b and c)` (AND tighter)
    let expr = parse_expression("a or b and c").expect("precedence test");
    let or_terms = children(&expr);
    assert_eq!(or_terms.len(), 2);
    assert_eq!(symbol_name(&or_terms[0]), "a");
    let and_terms = children(&or_terms[1]);
    assert_eq!(symbol_name(&and_terms[0]), "b");
    assert_eq!(symbol_name(&and_terms[1]), "c");

    // `a xor b or c` → LogicalOr(BitwiseXor(a, b), c)
    // xor (BitwiseXor) at precedence 7 is tighter than LogicalOr at 2,
    // so xor binds a and b first.
    let xor_expr = parse_expression("a xor b or c").expect("xor tighter than or");
    let or_terms = children(&xor_expr);
    assert_eq!(or_terms.len(), 2);
    // First child is BitwiseXor(a, b)
    let inner_xor = children(&or_terms[0]);
    assert_eq!(inner_xor.len(), 2);
    assert_eq!(symbol_name(&inner_xor[0]), "a");
    assert_eq!(symbol_name(&inner_xor[1]), "b");
    // Second child is c
    assert_eq!(symbol_name(&or_terms[1]), "c");
}

#[test]
fn double_factorial_is_double_factorial_node() {
    // Comment 21: `5!!` should parse as DoubleFactorial(5), not nested Factorial.
    let expr = parse_expression("5!!").expect("double factorial should parse");
    let Expression::DoubleFactorial(inner) = expr else {
        panic!("expected DoubleFactorial, got {expr:?}");
    };
    assert_eq!(number_text(&inner), "5");
}

#[test]
fn triple_factorial_is_multifactorial() {
    // Comment 21: `5!!!` should parse as MultiFactorial { expr: 5, count: 3 }.
    let expr = parse_expression("5!!!").expect("triple factorial should parse");
    let Expression::MultiFactorial { expr: inner, count } = expr else {
        panic!("expected MultiFactorial, got {expr:?}");
    };
    assert_eq!(number_text(&inner), "5");
    assert_eq!(count, 3);
}

#[test]
fn rem_word_is_always_remainder() {
    // Comment 4, 13: `rem` word operator should always be a remainder,
    // never postfix percent.

    // `7 rem 2` → Remainder(7, 2)
    let expr = parse_expression("7 rem 2").expect("rem is remainder");
    let Expression::Remainder { lhs, rhs } = expr else {
        panic!("expected Remainder, got {expr:?}");
    };
    assert_eq!(number_text(&lhs), "7");
    assert_eq!(number_text(&rhs), "2");

    // `7 rem -2` → Remainder(7, Negation(-2))
    let expr2 = parse_expression("7 rem -2").expect("rem with negative rhs");
    let Expression::Remainder { lhs, rhs } = expr2 else {
        panic!("expected Remainder, got {expr2:?}");
    };
    assert_eq!(number_text(&lhs), "7");
    assert!(matches!(*rhs, Expression::Negate(_)));
}

#[test]
fn percent_spacing_disambiguates_postfix_from_remainder() {
    // Comment 7: `10% + 100` (spaced) is Percent(10) + 100.
    let expr = parse_expression("10% + 100").expect("spaced percent is postfix");
    let terms = children(&expr);
    assert_eq!(terms.len(), 2);
    assert!(matches!(terms[0], Expression::Percent(_)));
    assert_eq!(number_text(&terms[1]), "100");

    // `6%2` (adjacent) is Remainder(6, 2).
    let tight = parse_expression("6%2").expect("tight percent is remainder");
    let Expression::Remainder { lhs, rhs } = tight else {
        panic!("expected Remainder, got {tight:?}");
    };
    assert_eq!(number_text(&lhs), "6");
    assert_eq!(number_text(&rhs), "2");

    // Comment 18: `6 % 2` (spaced) is also Remainder(6, 2).
    let spaced = parse_expression("6 % 2").expect("spaced percent with operand is remainder");
    let Expression::Remainder { lhs, rhs } = spaced else {
        panic!("expected Remainder, got {spaced:?}");
    };
    assert_eq!(number_text(&lhs), "6");
    assert_eq!(number_text(&rhs), "2");

    // Comment 3446611878: `6 % -2` (spaced with signed RHS) is Remainder(6, -2).
    let spaced_signed = parse_expression("6 % -2").expect("spaced percent with signed operand is remainder");
    let Expression::Remainder { lhs: lhs_signed, rhs: rhs_signed } = spaced_signed else {
        panic!("expected Remainder, got {spaced_signed:?}");
    };
    assert_eq!(number_text(&lhs_signed), "6");
    assert!(matches!(*rhs_signed, Expression::Negate(_)));
}

#[test]
fn standalone_e_operator_is_power_of_ten() {
    // Comment 15: `5 E 3` → Multiplication(5, Power(10, 3))
    let expr = parse_expression("5 E 3").expect("standalone E parses");
    let terms = children(&expr);
    assert_eq!(terms.len(), 2);
    assert_eq!(number_text(&terms[0]), "5");
    let Expression::Power { base, exponent } = &terms[1] else {
        panic!("expected Power, got {:?}", terms[1]);
    };
    assert_eq!(number_text(base), "10");
    assert_eq!(number_text(exponent), "3");
}

#[test]
fn lowercase_e_remains_identifier() {
    // Comment 16: Lowercase `e` should be an identifier (Euler's number),
    // not the ten-power operator.
    let expr = parse_expression("2 e 4").expect("lowercase e is implicit multiply");
    let terms = children(&expr);
    // `2 e 4` is implicit multiplication: Mul(2, e, 4)
    assert!(terms.len() >= 2);
    // Check that `e` appears as a symbolic identifier, not consumed as operator
    assert!(terms.iter().any(|t| matches!(t, Expression::Symbolic(s) if s.name() == "e")));
}

#[test]
fn e_operator_binds_tighter_than_power() {
    // Comment 19: Standalone `E` operator binds tighter than exponentiation.
    // `2 E 3^2` → `(2 × 10^3) ^ 2`, i.e. Power(Mul(2, Power(10, 3)), 2).
    // NOTE: `2E3` without spaces is a single scientific notation number literal
    // (= 2000), so only the spaced form triggers the standalone E operator.
    let expr = parse_expression("2 E 3^2").expect("E binds tighter than ^");
    let Expression::Power { base, exponent } = expr else {
        panic!("expected Power at top level, got {expr:?}");
    };
    assert_eq!(number_text(&exponent), "2");
    // base should be Multiplication(2, Power(10, 3))
    let mul_terms = children(&base);
    assert_eq!(mul_terms.len(), 2);
    assert_eq!(number_text(&mul_terms[0]), "2");
    let Expression::Power { base: ten, exponent: three } = &mul_terms[1] else {
        panic!("expected inner Power(10, 3), got {:?}", mul_terms[1]);
    };
    assert_eq!(number_text(ten), "10");
    assert_eq!(number_text(three), "3");

    // Without spaces, `2E3` is parsed as a single number literal by the lexer.
    let literal = parse_expression("2E3").expect("scientific notation");
    assert_eq!(number_text(&literal), "2000");
}

#[test]
fn pr117_fixture_rows_parse_without_evaluating() {
    // Additional fixture rows from PR #117 review comments.
    for source in [
        "5!!",                // double factorial
        "5!!!",               // triple factorial (multifactorial)
        "7 rem -2",           // rem with signed RHS
        "1 + 2 # comment",   // trailing comment
        "a || b",             // parallel operator
        "a xor b",            // word xor as bitwise XOR
        "a or b and c",       // correct logical precedence
        "10% + 100",          // spaced percent as postfix
        "6 % 2",              // spaced percent with primary as remainder
        "5 E 3",              // standalone E operator
    ] {
        parse_expression(source).unwrap_or_else(|err| panic!("{source}: {err}"));
    }
}

#[test]
fn logical_xor_symbol_is_logical_xor_node() {
    // Comment 3446611881: `⊕` should map to LogicalXor, not unit symbol or bitwise XOR.
    let expr = parse_expression("1>2 ⊕ 2>1").expect("⊕ should parse as logical xor");
    let Expression::LogicalXor { lhs, rhs } = expr else {
        panic!("expected LogicalXor, got {expr:?}");
    };
    assert!(matches!(lhs.as_ref(), Expression::Comparison { .. }));
    assert!(matches!(rhs.as_ref(), Expression::Comparison { .. }));
}

#[test]
fn parallel_sum_precedence() {
    // Comment 3446611880: Parallel sum `∥` groups between addition (11) and multiplication (13).
    // So `1 + 2 ∥ 3` parses as `1 + (2 ∥ 3)`.
    let expr = parse_expression("1 + 2 ∥ 3").expect("parse parallel sum with addition");
    let Expression::Addition(terms) = expr else {
        panic!("expected Addition, got {expr:?}");
    };
    assert_eq!(terms.as_slice().len(), 2);
    assert_eq!(number_text(&terms.as_slice()[0]), "1");
    let Expression::Parallel { lhs, rhs } = &terms.as_slice()[1] else {
        panic!("expected Parallel, got {:?}", terms.as_slice()[1]);
    };
    assert_eq!(number_text(lhs), "2");
    assert_eq!(number_text(rhs), "3");

    // Test with `||` and unit symbols (which triggers the parallel-sum precedence path)
    let expr_pipe = parse_expression("1 + 2 ohm || 3 ohm").expect("parse || parallel sum with addition");
    let Expression::Addition(terms_pipe) = expr_pipe else {
        panic!("expected Addition, got {expr_pipe:?}");
    };
    assert_eq!(terms_pipe.as_slice().len(), 2);
    assert_eq!(number_text(&terms_pipe.as_slice()[0]), "1");
    let Expression::Parallel { lhs: lhs_pipe, rhs: rhs_pipe } = &terms_pipe.as_slice()[1] else {
        panic!("expected Parallel, got {:?}", terms_pipe.as_slice()[1]);
    };
    let lhs_terms = children(lhs_pipe.as_ref());
    assert_eq!(number_text(&lhs_terms[0]), "2");
    assert_eq!(symbol_name(&lhs_terms[1]), "ohm");
    let rhs_terms = children(rhs_pipe.as_ref());
    assert_eq!(number_text(&rhs_terms[0]), "3");
    assert_eq!(symbol_name(&rhs_terms[1]), "ohm");
}
