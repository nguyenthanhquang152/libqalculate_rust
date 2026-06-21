use libqalculate_rust::ast::{ComparisonOperator, Expression};
use libqalculate_rust::parser::lexer::Operator;
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
    let expr = parse_expression("-2x (3 + 4)!").expect("parse expression");
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
    let xor_bitwise = parse_expression("a xor b").expect("parse word xor as bitwise xor");
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

    // Arrow `->` is now a supported conversion operator (Issue #19).
    let arrow = parse_expression("1 -> m").expect("arrow conversion should parse");
    assert!(
        matches!(arrow, Expression::Conversion { .. }),
        "expected Conversion, got {arrow:?}"
    );
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
fn pipe_pipe_between_unit_comparisons_stays_logical_or() {
    let expr = parse_expression("1 m = 1 m || 2 m = 2 m").expect("unit comparisons joined by ||");
    let Expression::LogicalOr(terms) = expr else {
        panic!("expected LogicalOr, got {expr:?}");
    };
    assert_eq!(terms.as_slice().len(), 2);
    assert!(matches!(terms.as_slice()[0], Expression::Comparison { .. }));
    assert!(matches!(terms.as_slice()[1], Expression::Comparison { .. }));
}

#[test]
fn chained_unit_pipe_pipe_detects_every_parallel_operator() {
    let expr = parse_expression("1 ohm || 2 ohm || 3 ohm").expect("unit parallel chain");
    let Expression::Parallel { lhs, rhs } = expr else {
        panic!("expected outer Parallel, got {expr:?}");
    };
    assert!(matches!(lhs.as_ref(), Expression::Parallel { .. }));
    let rhs_terms = children(rhs.as_ref());
    assert_eq!(number_text(&rhs_terms[0]), "3");
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
    let spaced_signed =
        parse_expression("6 % -2").expect("spaced percent with signed operand is remainder");
    let Expression::Remainder {
        lhs: lhs_signed,
        rhs: rhs_signed,
    } = spaced_signed
    else {
        panic!("expected Remainder, got {spaced_signed:?}");
    };
    assert_eq!(number_text(&lhs_signed), "6");
    assert!(matches!(*rhs_signed, Expression::Negate(_)));

    let function_percent = parse_expression("10%-sin(6)%").expect("function percent subtraction");
    let terms = children(&function_percent);
    assert!(matches!(terms[0], Expression::Percent(_)));
    let Expression::Negate(rhs) = &terms[1] else {
        panic!("expected subtraction term, got {:?}", terms[1]);
    };
    let Expression::Percent(percent_rhs) = rhs.as_ref() else {
        panic!("expected percent function RHS, got {rhs:?}");
    };
    assert!(matches!(
        percent_rhs.as_ref(),
        Expression::FunctionCall { .. }
    ));
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
    let expr = parse_expression("e + 4").expect("leading lowercase e remains symbolic");
    let terms = children(&expr);
    assert_eq!(symbol_name(&terms[0]), "e");
    assert_eq!(number_text(&terms[1]), "4");
}

#[test]
fn lowercase_e_between_operands_is_power_of_ten() {
    let expr = parse_expression("5 e 3").expect("lowercase e parses as ten-power operator");
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
    let Expression::Power {
        base: ten,
        exponent: three,
    } = &mul_terms[1]
    else {
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
        "5!!",             // double factorial
        "5!!!",            // triple factorial (multifactorial)
        "7 rem -2",        // rem with signed RHS
        "1 + 2 # comment", // trailing comment
        "a || b",          // parallel operator
        "a xor b",         // word xor as bitwise XOR
        "a or b and c",    // correct logical precedence
        "10% + 100",       // spaced percent as postfix
        "6 % 2",           // spaced percent with primary as remainder
        "5 E 3",           // standalone E operator
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
    let expr_pipe =
        parse_expression("1 + 2 ohm || 3 ohm").expect("parse || parallel sum with addition");
    let Expression::Addition(terms_pipe) = expr_pipe else {
        panic!("expected Addition, got {expr_pipe:?}");
    };
    assert_eq!(terms_pipe.as_slice().len(), 2);
    assert_eq!(number_text(&terms_pipe.as_slice()[0]), "1");
    let Expression::Parallel {
        lhs: lhs_pipe,
        rhs: rhs_pipe,
    } = &terms_pipe.as_slice()[1]
    else {
        panic!("expected Parallel, got {:?}", terms_pipe.as_slice()[1]);
    };
    let lhs_terms = children(lhs_pipe.as_ref());
    assert_eq!(number_text(&lhs_terms[0]), "2");
    assert_eq!(symbol_name(&lhs_terms[1]), "ohm");
    let rhs_terms = children(rhs_pipe.as_ref());
    assert_eq!(number_text(&rhs_terms[0]), "3");
    assert_eq!(symbol_name(&rhs_terms[1]), "ohm");
}

#[test]
fn test_code_review_issue_25_independent_parallel_resolution() {
    // Comment 25: (10 Ω || 6 Ω) + (a || b)
    // The first `||` must parse as Parallel, and the second as LogicalOr.
    let expr =
        parse_expression("(10 Ω || 6 Ω) + (a || b)").expect("parse (10 Ω || 6 Ω) + (a || b)");
    let Expression::Addition(terms) = expr else {
        panic!("expected Addition, got {expr:?}");
    };
    assert_eq!(terms.as_slice().len(), 2);

    // First term: 10 Ω || 6 Ω should be Expression::Parallel
    let Expression::Parallel { lhs, rhs } = &terms.as_slice()[0] else {
        panic!(
            "expected first term to be Parallel, got {:?}",
            terms.as_slice()[0]
        );
    };
    let lhs_terms = children(lhs.as_ref());
    assert_eq!(number_text(&lhs_terms[0]), "10");
    assert_eq!(symbol_name(&lhs_terms[1]), "Ω");
    let rhs_terms = children(rhs.as_ref());
    assert_eq!(number_text(&rhs_terms[0]), "6");
    assert_eq!(symbol_name(&rhs_terms[1]), "Ω");

    // Second term: a || b should be Expression::LogicalOr
    let logical_or = &terms.as_slice()[1];
    let logical_or_children = children(logical_or);
    assert_eq!(logical_or_children.len(), 2);
    assert_eq!(symbol_name(&logical_or_children[0]), "a");
    assert_eq!(symbol_name(&logical_or_children[1]), "b");
}

#[test]
fn test_code_review_issue_26_whitelist_free_unit_detection() {
    // Comment 26: 1 L || 2 L and 1 ft || 2 ft must parse as Parallel.
    for source in &["1 L || 2 L", "1 ft || 2 ft"] {
        let expr =
            parse_expression(source).unwrap_or_else(|e| panic!("failed to parse {source}: {e}"));
        let Expression::Parallel { lhs, rhs } = expr else {
            panic!("expected Parallel for {source}, got {expr:?}");
        };
        let lhs_terms = children(lhs.as_ref());
        assert_eq!(number_text(&lhs_terms[0]), "1");
        let rhs_terms = children(rhs.as_ref());
        assert_eq!(number_text(&rhs_terms[0]), "2");
    }
}

#[test]
fn test_code_review_issue_27_function_calls() {
    // Adjacent identifier calls are unresolved FunctionCall placeholders.
    let sin_expr = parse_expression("sin(2)").expect("parse sin(2)");
    let Expression::FunctionCall { function, args } = sin_expr else {
        panic!("expected FunctionCall, got {sin_expr:?}");
    };
    assert_eq!(function.id(), "sin");
    assert_eq!(args.len(), 1);
    assert_eq!(number_text(&args[0]), "2");

    let sqrt_expr = parse_expression("sqrt(32)").expect("parse sqrt(32)");
    let Expression::FunctionCall {
        function: sqrt_fn,
        args: sqrt_args,
    } = sqrt_expr
    else {
        panic!("expected FunctionCall, got {sqrt_expr:?}");
    };
    assert_eq!(sqrt_fn.id(), "sqrt");
    assert_eq!(sqrt_args.len(), 1);
    assert_eq!(number_text(&sqrt_args[0]), "32");

    // Check escaped identifier as function name: \sin(2)
    let escaped_expr = parse_expression("\\sin(2)").expect("parse \\sin(2)");
    let Expression::FunctionCall {
        function: esc_fn,
        args: esc_args,
    } = escaped_expr
    else {
        panic!("expected FunctionCall, got {escaped_expr:?}");
    };
    assert_eq!(esc_fn.id(), "\\sin");
    assert_eq!(esc_args.len(), 1);
    assert_eq!(number_text(&esc_args[0]), "2");

    let circle_expr = parse_expression("circle(3)").expect("parse circle(3)");
    let Expression::FunctionCall {
        function: circle_fn,
        args: circle_args,
    } = circle_expr
    else {
        panic!("expected FunctionCall, got {circle_expr:?}");
    };
    assert_eq!(circle_fn.id(), "circle");
    assert_eq!(circle_args.len(), 1);
    assert_eq!(number_text(&circle_args[0]), "3");

    let diff_expr = parse_expression("diff(6x^2)").expect("parse diff(6x^2)");
    let Expression::FunctionCall {
        function: diff_fn,
        args: diff_args,
    } = diff_expr
    else {
        panic!("expected FunctionCall, got {diff_expr:?}");
    };
    assert_eq!(diff_fn.id(), "diff");
    assert_eq!(diff_args.len(), 1);
    assert!(matches!(diff_args[0], Expression::Multiplication(_)));

    let coeff_expr = parse_expression("coeff(3x + 4, 0)").expect("parse coeff call");
    let Expression::FunctionCall {
        function: coeff_fn,
        args: coeff_args,
    } = coeff_expr
    else {
        panic!("expected FunctionCall, got {coeff_expr:?}");
    };
    assert_eq!(coeff_fn.id(), "coeff");
    assert_eq!(coeff_args.len(), 2);
    assert!(matches!(coeff_args[0], Expression::Addition(_)));
    assert_eq!(number_text(&coeff_args[1]), "0");

    let spaced_group =
        parse_expression("x (3 + 4)").expect("spaced group remains implicit multiplication");
    let spaced_terms = children(&spaced_group);
    assert_eq!(symbol_name(&spaced_terms[0]), "x");
    assert!(matches!(spaced_terms[1], Expression::Addition(_)));
}

#[test]
fn bare_and_postfix_single_argument_functions_parse_as_calls() {
    let sqrt_expr = parse_expression("sqrt 2x").expect("parse bare sqrt argument");
    let Expression::FunctionCall {
        function: sqrt_fn,
        args: sqrt_args,
    } = sqrt_expr
    else {
        panic!("expected FunctionCall, got {sqrt_expr:?}");
    };
    assert_eq!(sqrt_fn.id(), "sqrt");
    assert_eq!(sqrt_args.len(), 1);
    assert!(matches!(sqrt_args[0], Expression::Multiplication(_)));

    let sum_expr = parse_expression("sqrt 2x + 1").expect("bare function stops before addition");
    let sum_terms = children(&sum_expr);
    assert!(matches!(sum_terms[0], Expression::FunctionCall { .. }));
    assert_eq!(number_text(&sum_terms[1]), "1");

    let sin_expr = parse_expression("sin 0").expect("parse bare sin argument");
    let Expression::FunctionCall {
        function: sin_fn,
        args: sin_args,
    } = sin_expr
    else {
        panic!("expected FunctionCall, got {sin_expr:?}");
    };
    assert_eq!(sin_fn.id(), "sin");
    assert_eq!(number_text(&sin_args[0]), "0");

    let postfix_expr = parse_expression("1 + 2 sin").expect("parse postfix sin");
    let terms = children(&postfix_expr);
    assert_eq!(number_text(&terms[0]), "1");
    let Expression::FunctionCall {
        function: postfix_fn,
        args: postfix_args,
    } = &terms[1]
    else {
        panic!("expected postfix FunctionCall, got {:?}", terms[1]);
    };
    assert_eq!(postfix_fn.id(), "sin");
    assert_eq!(number_text(&postfix_args[0]), "2");
}

#[test]
fn parallel_sum_with_comparison_tail_keeps_parallel_precedence() {
    let expr = parse_expression("1 Ω || 2 Ω = 3 Ω")
        .expect("parallel unit expression followed by comparison parses");
    let Expression::Comparison {
        op: ComparisonOperator::Equal,
        lhs,
        rhs,
    } = expr
    else {
        panic!("expected comparison root, got {expr:?}");
    };

    let Expression::Parallel {
        lhs: parallel_lhs,
        rhs: parallel_rhs,
    } = lhs.as_ref()
    else {
        panic!("expected comparison lhs to be Parallel, got {lhs:?}");
    };

    let left_terms = children(parallel_lhs);
    assert_eq!(number_text(&left_terms[0]), "1");
    assert_eq!(symbol_name(&left_terms[1]), "Ω");
    let right_terms = children(parallel_rhs);
    assert_eq!(number_text(&right_terms[0]), "2");
    assert_eq!(symbol_name(&right_terms[1]), "Ω");

    let rhs_terms = children(&rhs);
    assert_eq!(number_text(&rhs_terms[0]), "3");
    assert_eq!(symbol_name(&rhs_terms[1]), "Ω");
}

#[test]
fn postfix_function_binds_below_power() {
    let expr = parse_expression("2^3 sin").expect("postfix function after power parses");
    let Expression::FunctionCall { function, args } = expr else {
        panic!("expected FunctionCall root, got {expr:?}");
    };
    assert_eq!(function.id(), "sin");
    assert_eq!(args.len(), 1);
    let Expression::Power { base, exponent } = &args[0] else {
        panic!(
            "expected postfix function argument to be Power, got {:?}",
            args[0]
        );
    };
    assert_eq!(number_text(base), "2");
    assert_eq!(number_text(exponent), "3");
}

#[test]
fn upstream_unit_aliases_can_drive_parallel_detection() {
    // `angstrom` is an upstream built-in alias from ../libqalculate/data/units.xml.in.
    let expr = parse_expression("1 angstrom || 2 angstrom")
        .expect("upstream unit alias drives parallel detection");
    let Expression::Parallel { lhs, rhs } = expr else {
        panic!("expected Parallel for upstream unit alias, got {expr:?}");
    };
    let lhs_terms = children(&lhs);
    assert_eq!(number_text(&lhs_terms[0]), "1");
    assert_eq!(symbol_name(&lhs_terms[1]), "angstrom");
    let rhs_terms = children(&rhs);
    assert_eq!(number_text(&rhs_terms[0]), "2");
    assert_eq!(symbol_name(&rhs_terms[1]), "angstrom");

    let unresolved_unit = parse_expression("1 widget || 2 widget")
        .expect("numeric symbol products are potential unit quantities");
    let Expression::Parallel { lhs, rhs } = unresolved_unit else {
        panic!("expected Parallel for unresolved unit-like products, got {unresolved_unit:?}");
    };
    let lhs_terms = children(&lhs);
    assert_eq!(number_text(&lhs_terms[0]), "1");
    assert_eq!(symbol_name(&lhs_terms[1]), "widget");
    let rhs_terms = children(&rhs);
    assert_eq!(number_text(&rhs_terms[0]), "2");
    assert_eq!(symbol_name(&rhs_terms[1]), "widget");
}

#[test]
fn bare_function_argument_stops_before_explicit_division() {
    let expr = parse_expression("sqrt 2/2").expect("bare function before division parses");
    let Expression::Division {
        numerator,
        denominator,
    } = expr
    else {
        panic!("expected division root, got {expr:?}");
    };
    let Expression::FunctionCall { function, args } = numerator.as_ref() else {
        panic!("expected numerator to be FunctionCall, got {numerator:?}");
    };
    assert_eq!(function.id(), "sqrt");
    assert_eq!(number_text(&args[0]), "2");
    assert_eq!(number_text(&denominator), "2");
}

#[test]
fn exp10_parses_as_bare_single_argument_function() {
    // `exp10` is an upstream built-in function from ../libqalculate/data/functions.xml.in.
    let expr = parse_expression("exp10 3").expect("bare exp10 parses");
    let Expression::FunctionCall { function, args } = expr else {
        panic!("expected FunctionCall root, got {expr:?}");
    };
    assert_eq!(function.id(), "exp10");
    assert_eq!(args.len(), 1);
    assert_eq!(number_text(&args[0]), "3");
}

#[test]
fn bare_function_tail_participates_in_percent_subtraction() {
    let expr = parse_expression("10%-sqrt 4%").expect("bare function percent subtraction parses");
    let terms = children(&expr);
    assert_eq!(terms.len(), 2);
    assert!(matches!(terms[0], Expression::Percent(_)));

    let Expression::Negate(rhs) = &terms[1] else {
        panic!("expected subtraction term, got {:?}", terms[1]);
    };
    let Expression::FunctionCall { function, args } = rhs.as_ref() else {
        panic!("expected RHS function, got {rhs:?}");
    };
    assert_eq!(function.id(), "sqrt");
    let Expression::Percent(arg) = &args[0] else {
        panic!(
            "expected percent inside bare function argument, got {:?}",
            args[0]
        );
    };
    assert_eq!(number_text(arg), "4");
}

#[test]
fn parallel_sum_after_comparison_tail_keeps_parallel_precedence() {
    let expr =
        parse_expression("x = 1 Ω || 2 Ω").expect("comparison RHS parallel unit expression parses");
    let Expression::Comparison {
        op: ComparisonOperator::Equal,
        lhs,
        rhs,
    } = expr
    else {
        panic!("expected comparison root, got {expr:?}");
    };
    assert_eq!(symbol_name(&lhs), "x");
    let Expression::Parallel {
        lhs: parallel_lhs,
        rhs: parallel_rhs,
    } = rhs.as_ref()
    else {
        panic!("expected comparison rhs to be Parallel, got {rhs:?}");
    };
    let left_terms = children(parallel_lhs);
    assert_eq!(number_text(&left_terms[0]), "1");
    assert_eq!(symbol_name(&left_terms[1]), "Ω");
    let right_terms = children(parallel_rhs);
    assert_eq!(number_text(&right_terms[0]), "2");
    assert_eq!(symbol_name(&right_terms[1]), "Ω");
}

#[test]
fn postfix_function_binds_inside_logical_and_bitwise_prefix_operands() {
    let logical = parse_expression("not 2 sin").expect("logical prefix with postfix function");
    let Expression::LogicalNot(logical_operand) = logical else {
        panic!("expected LogicalNot root, got {logical:?}");
    };
    let Expression::FunctionCall { function, args } = logical_operand.as_ref() else {
        panic!("expected LogicalNot operand to be FunctionCall, got {logical_operand:?}");
    };
    assert_eq!(function.id(), "sin");
    assert_eq!(number_text(&args[0]), "2");

    let bitwise = parse_expression("~2 sin").expect("bitwise prefix with postfix function");
    let Expression::BitwiseNot(bitwise_operand) = bitwise else {
        panic!("expected BitwiseNot root, got {bitwise:?}");
    };
    let Expression::FunctionCall { function, args } = bitwise_operand.as_ref() else {
        panic!("expected BitwiseNot operand to be FunctionCall, got {bitwise_operand:?}");
    };
    assert_eq!(function.id(), "sin");
    assert_eq!(number_text(&args[0]), "2");
}

#[test]
fn adaptive_unit_grouping_is_preserved_around_divisions() {
    let expr = parse_expression("5 m/5 m/s").expect("spaced unit division parses");
    let Expression::Division {
        numerator,
        denominator,
    } = expr
    else {
        panic!("expected top-level Division, got {expr:?}");
    };

    let numerator_terms = children(&numerator);
    assert_eq!(number_text(&numerator_terms[0]), "5");
    assert_eq!(symbol_name(&numerator_terms[1]), "m");

    let denominator_terms = children(&denominator);
    assert_eq!(number_text(&denominator_terms[0]), "5");
    let Expression::Division {
        numerator: unit_numerator,
        denominator: unit_denominator,
    } = &denominator_terms[1]
    else {
        panic!(
            "expected denominator unit factor to be Division, got {:?}",
            denominator_terms[1]
        );
    };
    assert_eq!(symbol_name(unit_numerator), "m");
    assert_eq!(symbol_name(unit_denominator), "s");
}

#[test]
fn spaced_parenthesized_calls_do_not_become_postfix_functions() {
    let expr = parse_expression("2 sin (3)").expect("spaced parenthesized call parses");
    let factors = children(&expr);
    assert_eq!(number_text(&factors[0]), "2");
    let Expression::FunctionCall { function, args } = &factors[1] else {
        panic!(
            "expected second factor to be FunctionCall, got {:?}",
            factors[1]
        );
    };
    assert_eq!(function.id(), "sin");
    assert_eq!(number_text(&args[0]), "3");
}

#[test]
fn percent_subtraction_scans_power_and_factorial_rhs_tails() {
    let power = parse_expression("10%-6^2%").expect("power percent subtraction parses");
    let terms = children(&power);
    assert!(matches!(terms[0], Expression::Percent(_)));
    let Expression::Negate(power_rhs) = &terms[1] else {
        panic!("expected subtraction term, got {:?}", terms[1]);
    };
    let Expression::Power { exponent, .. } = power_rhs.as_ref() else {
        panic!("expected RHS power expression, got {power_rhs:?}");
    };
    assert!(matches!(exponent.as_ref(), Expression::Percent(_)));

    let factorial = parse_expression("10%-6!%").expect("factorial percent subtraction parses");
    let terms = children(&factorial);
    assert!(matches!(terms[0], Expression::Percent(_)));
    let Expression::Negate(factorial_rhs) = &terms[1] else {
        panic!("expected subtraction term, got {:?}", terms[1]);
    };
    let Expression::Percent(factorial_percent) = factorial_rhs.as_ref() else {
        panic!("expected RHS percent, got {factorial_rhs:?}");
    };
    assert!(matches!(
        factorial_percent.as_ref(),
        Expression::Factorial(_)
    ));
}

#[test]
fn bare_function_arguments_include_spaced_variable_and_unit_products() {
    for source in ["sqrt 2 x", "sqrt 2 m"] {
        let expr = parse_expression(source)
            .unwrap_or_else(|err| panic!("{source} should parse as bare function: {err}"));
        let Expression::FunctionCall { function, args } = expr else {
            panic!("expected FunctionCall for {source}, got {expr:?}");
        };
        assert_eq!(function.id(), "sqrt");
        let factors = children(&args[0]);
        assert_eq!(number_text(&factors[0]), "2");
        assert!(matches!(factors[1], Expression::Symbolic(_)));
    }
}

#[test]
fn powered_denominator_units_stay_inside_adaptive_division() {
    let expr = parse_expression("5 m/5 m^2/s").expect("powered unit denominator parses");
    let Expression::Division {
        numerator,
        denominator,
    } = expr
    else {
        panic!("expected top-level Division, got {expr:?}");
    };
    let numerator_terms = children(&numerator);
    assert_eq!(number_text(&numerator_terms[0]), "5");
    assert_eq!(symbol_name(&numerator_terms[1]), "m");

    let denominator_terms = children(&denominator);
    assert_eq!(number_text(&denominator_terms[0]), "5");
    let Expression::Division {
        numerator: unit_numerator,
        denominator: unit_denominator,
    } = &denominator_terms[1]
    else {
        panic!(
            "expected denominator unit factor to be Division, got {:?}",
            denominator_terms[1]
        );
    };
    let Expression::Power { base, exponent } = unit_numerator.as_ref() else {
        panic!("expected powered denominator unit, got {unit_numerator:?}");
    };
    assert_eq!(symbol_name(base), "m");
    assert_eq!(number_text(exponent), "2");
    assert_eq!(symbol_name(unit_denominator), "s");
}

#[test]
fn unit_power_shorthand_expands_outside_adaptive_divisors() {
    let product = parse_expression("5 m2").expect("unit shorthand product parses");
    let factors = children(&product);
    assert_eq!(number_text(&factors[0]), "5");
    let Expression::Power { base, exponent } = &factors[1] else {
        panic!("expected shorthand powered unit, got {:?}", factors[1]);
    };
    assert_eq!(symbol_name(base), "m");
    assert_eq!(number_text(exponent), "2");

    let quotient = parse_expression("5 m2/s").expect("unit shorthand quotient parses");
    let Expression::Division {
        numerator,
        denominator,
    } = quotient
    else {
        panic!("expected Division, got {quotient:?}");
    };
    let numerator_terms = children(&numerator);
    assert_eq!(number_text(&numerator_terms[0]), "5");
    let Expression::Power { base, exponent } = &numerator_terms[1] else {
        panic!(
            "expected shorthand powered numerator unit, got {:?}",
            numerator_terms[1]
        );
    };
    assert_eq!(symbol_name(base), "m");
    assert_eq!(number_text(exponent), "2");
    assert_eq!(symbol_name(&denominator), "s");
}

#[test]
fn unit_power_shorthand_stays_inside_adaptive_division() {
    let expr = parse_expression("5 m/5 m2/s").expect("unit shorthand denominator parses");
    let Expression::Division { denominator, .. } = expr else {
        panic!("expected top-level Division, got {expr:?}");
    };

    let denominator_terms = children(&denominator);
    assert_eq!(number_text(&denominator_terms[0]), "5");
    let Expression::Division {
        numerator: unit_numerator,
        denominator: unit_denominator,
    } = &denominator_terms[1]
    else {
        panic!(
            "expected denominator unit factor to be Division, got {:?}",
            denominator_terms[1]
        );
    };
    let Expression::Power { base, exponent } = unit_numerator.as_ref() else {
        panic!("expected shorthand powered denominator unit, got {unit_numerator:?}");
    };
    assert_eq!(symbol_name(base), "m");
    assert_eq!(number_text(exponent), "2");
    assert_eq!(symbol_name(unit_denominator), "s");
}

#[test]
fn spaced_unit_products_stay_inside_adaptive_denominators() {
    let expr = parse_expression("5 m/5 m m/s").expect("spaced unit product denominator parses");
    let Expression::Division { denominator, .. } = expr else {
        panic!("expected top-level Division, got {expr:?}");
    };

    let denominator_terms = children(&denominator);
    assert_eq!(number_text(&denominator_terms[0]), "5");
    let Expression::Division {
        numerator: unit_numerator,
        denominator: unit_denominator,
    } = &denominator_terms[1]
    else {
        panic!(
            "expected denominator unit factor to be Division, got {:?}",
            denominator_terms[1]
        );
    };
    let unit_product = children(unit_numerator);
    assert_eq!(symbol_name(&unit_product[0]), "m");
    assert_eq!(symbol_name(&unit_product[1]), "m");
    assert_eq!(symbol_name(unit_denominator), "s");
}

#[test]
fn negated_numeric_user_unit_products_drive_parallel_detection() {
    let expr = parse_expression("-1 widget || -2 widget")
        .expect("signed possible user-unit quantities parse as parallel");
    let Expression::Parallel { lhs, rhs } = expr else {
        panic!("expected Parallel, got {expr:?}");
    };
    let lhs_terms = children(&lhs);
    assert!(matches!(lhs_terms[0], Expression::Negate(_)));
    assert_eq!(symbol_name(&lhs_terms[1]), "widget");
    let rhs_terms = children(&rhs);
    assert!(matches!(rhs_terms[0], Expression::Negate(_)));
    assert_eq!(symbol_name(&rhs_terms[1]), "widget");
}

#[test]
fn grouped_rhs_percent_stays_percentage_subtraction() {
    let expr = parse_expression("10%-(6%)").expect("grouped percent subtraction parses");
    let terms = children(&expr);
    assert_eq!(terms.len(), 2);
    assert!(matches!(terms[0], Expression::Percent(_)));
    let Expression::Negate(rhs) = &terms[1] else {
        panic!("expected subtraction term, got {:?}", terms[1]);
    };
    assert!(matches!(rhs.as_ref(), Expression::Percent(_)));
}

#[test]
fn nested_grouped_rhs_percent_stays_percentage_subtraction() {
    let expr = parse_expression("10%-((6%))").expect("nested grouped percent subtraction parses");
    let terms = children(&expr);
    assert_eq!(terms.len(), 2);
    assert!(matches!(terms[0], Expression::Percent(_)));
    let Expression::Negate(rhs) = &terms[1] else {
        panic!("expected subtraction term, got {:?}", terms[1]);
    };
    assert!(matches!(rhs.as_ref(), Expression::Percent(_)));
}

#[test]
fn sgn_alias_parses_as_bare_and_postfix_single_argument_function() {
    let bare = parse_expression("sgn -2").expect("bare sgn alias parses");
    let Expression::FunctionCall { function, args } = bare else {
        panic!("expected FunctionCall, got {bare:?}");
    };
    assert_eq!(function.id(), "sgn");
    assert!(matches!(args[0], Expression::Negate(_)));

    let postfix = parse_expression("2 sgn").expect("postfix sgn alias parses");
    let Expression::FunctionCall { function, args } = postfix else {
        panic!("expected FunctionCall, got {postfix:?}");
    };
    assert_eq!(function.id(), "sgn");
    assert_eq!(number_text(&args[0]), "2");
}

#[test]
fn logical_and_tail_can_drive_parallel_detection() {
    let expr = parse_expression("a and 1 Ω || 2 Ω")
        .expect("logical and with RHS parallel unit expression parses");
    let terms = children(&expr);
    assert_eq!(symbol_name(&terms[0]), "a");
    let Expression::Parallel { lhs, rhs } = &terms[1] else {
        panic!(
            "expected logical-and rhs to be Parallel, got {:?}",
            terms[1]
        );
    };
    let lhs_terms = children(lhs);
    assert_eq!(number_text(&lhs_terms[0]), "1");
    assert_eq!(symbol_name(&lhs_terms[1]), "Ω");
    let rhs_terms = children(rhs);
    assert_eq!(number_text(&rhs_terms[0]), "2");
    assert_eq!(symbol_name(&rhs_terms[1]), "Ω");
}

#[test]
fn tight_identifier_function_suffix_splits_into_implicit_product() {
    let expr = parse_expression("2xsin(2)").expect("tight variable/function suffix parses");
    let factors = children(&expr);
    assert_eq!(number_text(&factors[0]), "2");
    let suffix_terms = children(&factors[1]);
    assert_eq!(symbol_name(&suffix_terms[0]), "x");
    let Expression::FunctionCall { function, args } = &suffix_terms[1] else {
        panic!("expected function factor, got {:?}", suffix_terms[1]);
    };
    assert_eq!(function.id(), "sin");
    assert_eq!(number_text(&args[0]), "2");
}

#[test]
fn multi_letter_unknown_function_suffix_is_preserved() {
    let expr = parse_expression("myasin(2)").expect("unknown suffix function parses");
    let Expression::FunctionCall { function, args } = expr else {
        panic!("expected FunctionCall, got {expr:?}");
    };
    assert_eq!(function.id(), "myasin");
    assert_eq!(number_text(&args[0]), "2");
}

#[test]
fn spaced_remainder_keeps_signed_percent_rhs() {
    let expr =
        parse_expression("6 % -2%").expect("spaced remainder with signed percent rhs parses");
    let Expression::Remainder { lhs, rhs } = expr else {
        panic!("expected Remainder, got {expr:?}");
    };
    assert_eq!(number_text(&lhs), "6");
    let Expression::Negate(negated) = rhs.as_ref() else {
        panic!("expected signed remainder rhs, got {rhs:?}");
    };
    let Expression::Percent(percent_rhs) = negated.as_ref() else {
        panic!("expected percent in signed rhs, got {negated:?}");
    };
    assert_eq!(number_text(percent_rhs), "2");
}

#[test]
fn unit_only_divisor_stays_unit_chain() {
    let expr = parse_expression("5 m/m/s").expect("unit-only divisor chain parses");
    let Expression::Division {
        numerator,
        denominator,
    } = expr
    else {
        panic!("expected top-level Division, got {expr:?}");
    };
    let numerator_terms = children(&numerator);
    assert_eq!(number_text(&numerator_terms[0]), "5");
    assert_eq!(symbol_name(&numerator_terms[1]), "m");

    let Expression::Division {
        numerator: unit_numerator,
        denominator: unit_denominator,
    } = denominator.as_ref()
    else {
        panic!("expected denominator unit chain, got {denominator:?}");
    };
    assert_eq!(symbol_name(unit_numerator), "m");
    assert_eq!(symbol_name(unit_denominator), "s");
}

#[test]
fn tight_e_identifier_is_implicit_product_not_ten_power() {
    let expr = parse_expression("2e").expect("tight e parses as implicit product");
    let factors = children(&expr);
    assert_eq!(number_text(&factors[0]), "2");
    assert_eq!(symbol_name(&factors[1]), "e");

    let reciprocal = parse_expression("1/(2e)").expect("tight e in denominator parses");
    let Expression::Division {
        numerator,
        denominator,
    } = reciprocal
    else {
        panic!("expected Division, got {reciprocal:?}");
    };
    assert_eq!(number_text(&numerator), "1");
    let denominator_factors = children(&denominator);
    assert_eq!(number_text(&denominator_factors[0]), "2");
    assert_eq!(symbol_name(&denominator_factors[1]), "e");
}

#[test]
fn bare_function_argument_includes_postfix_tails() {
    let factorial = parse_expression("sqrt 4!").expect("bare factorial argument parses");
    let Expression::FunctionCall { function, args } = factorial else {
        panic!("expected FunctionCall, got {factorial:?}");
    };
    assert_eq!(function.id(), "sqrt");
    let Expression::Factorial(arg) = &args[0] else {
        panic!("expected factorial argument, got {:?}", args[0]);
    };
    assert_eq!(number_text(arg), "4");

    let percent = parse_expression("sqrt 10%").expect("bare percent argument parses");
    let Expression::FunctionCall { function, args } = percent else {
        panic!("expected FunctionCall, got {percent:?}");
    };
    assert_eq!(function.id(), "sqrt");
    let Expression::Percent(arg) = &args[0] else {
        panic!("expected percent argument, got {:?}", args[0]);
    };
    assert_eq!(number_text(arg), "10");
}

#[test]
fn postfix_function_binds_after_postfix_percent() {
    let expr = parse_expression("10% sin").expect("postfix function after percent parses");
    let Expression::FunctionCall { function, args } = expr else {
        panic!("expected FunctionCall, got {expr:?}");
    };
    assert_eq!(function.id(), "sin");
    let Expression::Percent(arg) = &args[0] else {
        panic!("expected percent function argument, got {:?}", args[0]);
    };
    assert_eq!(number_text(arg), "10");
}

#[test]
fn signed_spaced_unit_denominator_stays_quantity_unit_chain() {
    let expr = parse_expression("5 m/-5 m/s").expect("signed spaced unit denominator parses");
    let Expression::Division {
        numerator,
        denominator,
    } = expr
    else {
        panic!("expected top-level Division, got {expr:?}");
    };
    let numerator_terms = children(&numerator);
    assert_eq!(number_text(&numerator_terms[0]), "5");
    assert_eq!(symbol_name(&numerator_terms[1]), "m");

    let denominator_terms = children(&denominator);
    let Expression::Negate(quantity) = &denominator_terms[0] else {
        panic!(
            "expected signed denominator quantity, got {:?}",
            denominator_terms[0]
        );
    };
    assert_eq!(number_text(quantity), "5");
    let Expression::Division {
        numerator: unit_numerator,
        denominator: unit_denominator,
    } = &denominator_terms[1]
    else {
        panic!(
            "expected denominator unit chain, got {:?}",
            denominator_terms[1]
        );
    };
    assert_eq!(symbol_name(unit_numerator), "m");
    assert_eq!(symbol_name(unit_denominator), "s");
}

#[test]
fn compact_unit_denominators_stay_inside_adaptive_division() {
    let expr = parse_expression("5 m/5m/s").expect("compact unit denominator parses");
    let Expression::Division { denominator, .. } = expr else {
        panic!("expected top-level Division, got {expr:?}");
    };
    let denominator_terms = children(&denominator);
    assert_eq!(number_text(&denominator_terms[0]), "5");
    let Expression::Division {
        numerator: unit_numerator,
        denominator: unit_denominator,
    } = &denominator_terms[1]
    else {
        panic!(
            "expected denominator unit factor to be Division, got {:?}",
            denominator_terms[1]
        );
    };
    assert_eq!(symbol_name(unit_numerator), "m");
    assert_eq!(symbol_name(unit_denominator), "s");

    let signed = parse_expression("5 m/-5m/s").expect("signed compact unit denominator parses");
    let Expression::Division { denominator, .. } = signed else {
        panic!("expected top-level Division, got {signed:?}");
    };
    let denominator_terms = children(&denominator);
    let Expression::Negate(quantity) = &denominator_terms[0] else {
        panic!(
            "expected signed denominator quantity, got {:?}",
            denominator_terms[0]
        );
    };
    assert_eq!(number_text(quantity), "5");
    let Expression::Division {
        numerator: unit_numerator,
        denominator: unit_denominator,
    } = &denominator_terms[1]
    else {
        panic!(
            "expected signed denominator unit factor to be Division, got {:?}",
            denominator_terms[1]
        );
    };
    assert_eq!(symbol_name(unit_numerator), "m");
    assert_eq!(symbol_name(unit_denominator), "s");
}

#[test]
fn nonadjacent_units_do_not_drive_parallel_detection() {
    let expr = parse_expression("1 Ω + a || b + 2 Ω").expect("nonadjacent units parse");
    let operands = children(&expr);
    assert_eq!(operands.len(), 2);
    assert!(matches!(expr, Expression::LogicalOr(_)));
}

#[test]
fn unsupported_word_operators_are_rejected() {
    for (source, operator) in [
        ("5 comb 2", Operator::Combination),
        ("5 perm 2", Operator::Permutation),
    ] {
        let err = match parse_expression(source) {
            Ok(expr) => panic!("{source} should not parse, got {expr:?}"),
            Err(err) => err,
        };
        assert_eq!(err.kind(), ParseErrorKind::UnsupportedOperator(operator));
    }
}

#[test]
fn bare_function_can_be_remainder_rhs() {
    let expr = parse_expression("6 % sqrt 4").expect("bare function remainder rhs parses");
    let Expression::Remainder { lhs, rhs } = expr else {
        panic!("expected Remainder, got {expr:?}");
    };
    assert_eq!(number_text(&lhs), "6");
    let Expression::FunctionCall { function, args } = rhs.as_ref() else {
        panic!("expected bare function RHS, got {rhs:?}");
    };
    assert_eq!(function.id(), "sqrt");
    assert_eq!(number_text(&args[0]), "4");
}

// ===== Conversion parsing tests =====

#[test]
fn parses_simple_conversion_with_to() {
    let expr = parse_expression("5 m to ft").expect("parse conversion");
    let Expression::Conversion { expr: lhs, target } = expr else {
        panic!("expected Conversion, got {expr:?}");
    };
    let factors = children(&lhs);
    assert_eq!(number_text(&factors[0]), "5");
    assert_eq!(symbol_name(&factors[1]), "m");
    assert_eq!(symbol_name(&target), "ft");
}

#[test]
fn parses_conversion_with_arrow_operator() {
    // The lexer emits Conversion for both `to` and `->`
    let expr = parse_expression("100 USD -> EUR").expect("parse arrow conversion");
    let Expression::Conversion { expr: lhs, target } = expr else {
        panic!("expected Conversion, got {expr:?}");
    };
    let factors = children(&lhs);
    assert_eq!(number_text(&factors[0]), "100");
    assert_eq!(symbol_name(&factors[1]), "USD");
    assert_eq!(symbol_name(&target), "EUR");
}

#[test]
fn conversion_has_lower_precedence_than_addition() {
    // `2 + 3 to m` should parse as `(2 + 3) to m`, not `2 + (3 to m)`
    let expr = parse_expression("2 + 3 to m").expect("parse conversion precedence");
    let Expression::Conversion { expr: lhs, target } = expr else {
        panic!("expected Conversion at top, got {expr:?}");
    };
    let terms = children(&lhs);
    assert_eq!(number_text(&terms[0]), "2");
    assert_eq!(number_text(&terms[1]), "3");
    assert_eq!(symbol_name(&target), "m");
}

#[test]
fn conversion_has_lower_precedence_than_multiplication() {
    // `5 m to ft` should parse as `(5 * m) to ft`
    let expr = parse_expression("5 m to ft").expect("parse mul-conversion");
    let Expression::Conversion { expr: lhs, target } = expr else {
        panic!("expected Conversion, got {expr:?}");
    };
    let factors = children(&lhs);
    assert_eq!(factors.len(), 2);
    assert_eq!(number_text(&factors[0]), "5");
    assert_eq!(symbol_name(&factors[1]), "m");
    assert_eq!(symbol_name(&target), "ft");
}

#[test]
fn conversion_chains_left_to_right() {
    // `5 m to ft to in` should parse as `(5 m to ft) to in`
    let expr = parse_expression("5 m to ft to in").expect("parse chained conversion");
    let Expression::Conversion {
        expr: inner,
        target: outer_target,
    } = expr
    else {
        panic!("expected outer Conversion, got {expr:?}");
    };
    assert_eq!(symbol_name(&outer_target), "in");

    let Expression::Conversion {
        expr: lhs,
        target: inner_target,
    } = inner.as_ref()
    else {
        panic!("expected inner Conversion, got {inner:?}");
    };
    let factors = children(lhs);
    assert_eq!(number_text(&factors[0]), "5");
    assert_eq!(symbol_name(&factors[1]), "m");
    assert_eq!(symbol_name(inner_target), "ft");
}

#[test]
fn conversion_with_complex_expression() {
    // `2^3 + 1 to m` should parse as `(2^3 + 1) to m`
    let expr = parse_expression("2^3 + 1 to m").expect("parse complex conversion");
    let Expression::Conversion { expr: lhs, target } = expr else {
        panic!("expected Conversion, got {expr:?}");
    };
    assert_eq!(symbol_name(&target), "m");
    let terms = children(&lhs);
    assert_eq!(terms.len(), 2);
    assert!(matches!(&terms[0], Expression::Power { .. }));
    assert_eq!(number_text(&terms[1]), "1");
}

// ===== Assignment parsing tests =====

#[test]
fn parses_simple_assignment() {
    let expr = parse_expression("x := 5").expect("parse assignment");
    let Expression::Assignment { variable, value } = expr else {
        panic!("expected Assignment, got {expr:?}");
    };
    assert_eq!(variable, "x");
    assert_eq!(number_text(&value), "5");
}

#[test]
fn assignment_captures_full_expression_rhs() {
    // `x := 2 + 3` should parse as `x := (2 + 3)`
    let expr = parse_expression("x := 2 + 3").expect("parse assignment with expression");
    let Expression::Assignment { variable, value } = expr else {
        panic!("expected Assignment, got {expr:?}");
    };
    assert_eq!(variable, "x");
    let terms = children(&value);
    assert_eq!(number_text(&terms[0]), "2");
    assert_eq!(number_text(&terms[1]), "3");
}

#[test]
fn assignment_with_conversion_rhs() {
    // `x := 5 m to ft` should parse as `x := (5 m to ft)`
    let expr = parse_expression("x := 5 m to ft").expect("parse assignment with conversion");
    let Expression::Assignment { variable, value } = expr else {
        panic!("expected Assignment, got {expr:?}");
    };
    assert_eq!(variable, "x");
    let Expression::Conversion {
        expr: conv_lhs,
        target,
    } = value.as_ref()
    else {
        panic!("expected Conversion in RHS, got {value:?}");
    };
    let factors = children(conv_lhs);
    assert_eq!(number_text(&factors[0]), "5");
    assert_eq!(symbol_name(&factors[1]), "m");
    assert_eq!(symbol_name(target), "ft");
}

#[test]
fn chained_assignment_is_right_associative() {
    // `x := y := 5` should parse as `x := (y := 5)`
    let expr = parse_expression("x := y := 5").expect("parse chained assignment");
    let Expression::Assignment { variable, value } = expr else {
        panic!("expected outer Assignment, got {expr:?}");
    };
    assert_eq!(variable, "x");
    let Expression::Assignment {
        variable: inner_var,
        value: inner_val,
    } = value.as_ref()
    else {
        panic!("expected inner Assignment, got {value:?}");
    };
    assert_eq!(inner_var, "y");
    assert_eq!(number_text(inner_val), "5");
}

#[test]
fn conversion_has_lower_precedence_than_logical_operators() {
    // `a && b to m` should parse as `(a && b) to m`
    let expr = parse_expression("a && b to m").expect("parse logical conversion");
    let Expression::Conversion { expr: lhs, target } = expr else {
        panic!("expected Conversion at top, got {expr:?}");
    };
    assert_eq!(symbol_name(&target), "m");
    assert!(matches!(lhs.as_ref(), Expression::LogicalAnd(_)));
}
