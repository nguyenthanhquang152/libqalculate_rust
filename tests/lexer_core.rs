use libqalculate_rust::parser::lexer::{
    lex_expression, lex_line, BasePrefix, LexErrorKind, LineKind, NumberLiteralKind, Operator,
    TokenKind,
};

fn kinds(input: &str) -> Vec<TokenKind> {
    lex_expression(input)
        .expect("lex expression")
        .into_iter()
        .map(|token| token.kind)
        .collect()
}

#[test]
fn tokenizes_numbers_base_prefixes_and_scientific_notation_with_spans() {
    // Fixture source for ordinary decimal/scientific forms:
    // ../libqalculate/tests/parser.batch:41-51. Whitespace-joined decimal
    // forms from parser.batch:11, :26, and :51 are parser/session work; this
    // lexer preserves fragments and spans instead of normalizing them.
    let tokens = lex_expression("0xD8 + 0b1010 + 1.25e-3").expect("lex expression");

    assert_eq!(
        tokens
            .iter()
            .map(|token| token.span.range())
            .collect::<Vec<_>>(),
        vec![0..4, 5..6, 7..13, 14..15, 16..23]
    );
    assert_eq!(
        tokens
            .into_iter()
            .map(|token| token.kind)
            .collect::<Vec<_>>(),
        vec![
            TokenKind::Number {
                text: "0xD8".into(),
                kind: NumberLiteralKind::BasePrefixed(BasePrefix::Hexadecimal),
            },
            TokenKind::Operator(Operator::Plus),
            TokenKind::Number {
                text: "0b1010".into(),
                kind: NumberLiteralKind::BasePrefixed(BasePrefix::Binary),
            },
            TokenKind::Operator(Operator::Plus),
            TokenKind::Number {
                text: "1.25e-3".into(),
                kind: NumberLiteralKind::Scientific,
            },
        ]
    );
}

#[test]
fn keeps_grouped_decimal_literals_from_parser_fixtures() {
    // Fixture source: ../libqalculate/tests/parser.batch:11, :26, and :51.
    assert_eq!(
        kinds("123 456 789"),
        vec![TokenKind::Number {
            text: "123 456 789".into(),
            kind: NumberLiteralKind::Integer,
        }]
    );

    assert_eq!(
        kinds("-  0  .  0   01"),
        vec![
            TokenKind::Operator(Operator::Minus),
            TokenKind::Number {
                text: "0  .  0   01".into(),
                kind: NumberLiteralKind::Decimal,
            },
        ]
    );

    assert_eq!(
        kinds("-12 3.2 3e3"),
        vec![
            TokenKind::Operator(Operator::Minus),
            TokenKind::Number {
                text: "12 3.2 3e3".into(),
                kind: NumberLiteralKind::Scientific,
            },
        ]
    );
}

#[test]
fn keeps_grouped_prefixed_base_literals_from_bitwise_fixtures() {
    // Fixture source: ../libqalculate/tests/bitwise.batch:54.
    assert_eq!(
        kinds("0b1011 0010 ^^ 0b0111 0001 to bin8"),
        vec![
            TokenKind::Number {
                text: "0b1011 0010".into(),
                kind: NumberLiteralKind::BasePrefixed(BasePrefix::Binary),
            },
            TokenKind::Operator(Operator::BitwiseXor),
            TokenKind::Number {
                text: "0b0111 0001".into(),
                kind: NumberLiteralKind::BasePrefixed(BasePrefix::Binary),
            },
            TokenKind::Operator(Operator::Conversion),
            TokenKind::Identifier("bin8".into()),
        ]
    );

    // Fixture source: ../libqalculate/tests/bitwise.batch:37.
    assert_eq!(
        kinds("0b1011 0010 ∨ 0b0111 0001 to bin8"),
        vec![
            TokenKind::Number {
                text: "0b1011 0010".into(),
                kind: NumberLiteralKind::BasePrefixed(BasePrefix::Binary),
            },
            TokenKind::Operator(Operator::BitwiseOr),
            TokenKind::Number {
                text: "0b0111 0001".into(),
                kind: NumberLiteralKind::BasePrefixed(BasePrefix::Binary),
            },
            TokenKind::Operator(Operator::Conversion),
            TokenKind::Identifier("bin8".into()),
        ]
    );
}

#[test]
fn preserves_codex_reviewed_lexer_boundaries() {
    assert!(kinds("5:30").contains(&TokenKind::Colon));
    assert_eq!(
        kinds("[1:4]"),
        vec![
            TokenKind::OpenBracket,
            TokenKind::Number {
                text: "1".into(),
                kind: NumberLiteralKind::Integer,
            },
            TokenKind::Colon,
            TokenKind::Number {
                text: "4".into(),
                kind: NumberLiteralKind::Integer,
            },
            TokenKind::CloseBracket,
        ]
    );

    assert_eq!(
        kinds("0o10 + 0d10"),
        vec![
            TokenKind::Number {
                text: "0o10".into(),
                kind: NumberLiteralKind::BasePrefixed(BasePrefix::Octal),
            },
            TokenKind::Operator(Operator::Plus),
            TokenKind::Number {
                text: "0d10".into(),
                kind: NumberLiteralKind::BasePrefixed(BasePrefix::Duodecimal),
            },
        ]
    );

    assert_eq!(
        kinds(r"5\a + \x"),
        vec![
            TokenKind::Number {
                text: "5".into(),
                kind: NumberLiteralKind::Integer,
            },
            TokenKind::EscapedIdentifier("a".into()),
            TokenKind::Operator(Operator::Plus),
            TokenKind::EscapedIdentifier("x".into()),
        ]
    );

    assert_eq!(
        kinds("!(1 > 2)")[0],
        TokenKind::Operator(Operator::LogicalNot)
    );
    assert_eq!(kinds("5!")[1], TokenKind::Operator(Operator::Factorial));

    assert!(kinds("10 Ω || 6 Ω").contains(&TokenKind::Operator(Operator::Parallel)));
    assert!(kinds("10 Ω ∥ 6 Ω").contains(&TokenKind::Operator(Operator::Parallel)));

    assert_eq!(
        kinds("1...4"),
        vec![
            TokenKind::Number {
                text: "1".into(),
                kind: NumberLiteralKind::Integer,
            },
            TokenKind::Ellipsis,
            TokenKind::Number {
                text: "4".into(),
                kind: NumberLiteralKind::Integer,
            },
        ]
    );

    for (source, operator) in [
        ("A ∪ B", Operator::SetUnion),
        ("A ∩ B", Operator::SetIntersection),
        ("A ∖ B", Operator::SetDifference),
        ("A ⊖ B", Operator::SetSymmetricDifference),
        ("A ∈ B", Operator::SetMembership),
        ("A ∉ B", Operator::SetNotMembership),
        ("A ∋ B", Operator::SetContains),
        ("A ∌ B", Operator::SetNotContains),
        ("A ⊊ B", Operator::ProperSubset),
        ("A ⊆ B", Operator::Subset),
        ("A ⊋ B", Operator::ProperSuperset),
        ("A ⊇ B", Operator::Superset),
    ] {
        assert!(
            kinds(source).contains(&TokenKind::Operator(operator)),
            "{source}"
        );
    }

    assert!(kinds("a NAND b").contains(&TokenKind::Operator(Operator::LogicalNand)));
    assert!(kinds("a NOR b").contains(&TokenKind::Operator(Operator::LogicalNor)));

    assert!(kinds("5 m -> ft").contains(&TokenKind::Operator(Operator::Conversion)));
    assert!(kinds("5 m → ft").contains(&TokenKind::Operator(Operator::Conversion)));
    assert!(kinds("5 m to ft").contains(&TokenKind::Operator(Operator::Conversion)));
}

#[test]
fn preserves_command_and_vector_boundaries_from_followup_review() {
    let to_command = lex_line("to hex").expect("lex to command");
    assert_eq!(to_command.kind, LineKind::Command);
    assert_eq!(
        to_command.tokens[0].kind,
        TokenKind::Identifier("to".into())
    );

    let slash_to_command = lex_line("/to m/s").expect("lex slash to command");
    assert_eq!(slash_to_command.kind, LineKind::Command);
    assert_eq!(slash_to_command.tokens[0].kind, TokenKind::CommandPrefix);
    assert_eq!(
        slash_to_command.tokens[1].kind,
        TokenKind::Identifier("to".into())
    );

    for source in ["-> m/s", "→ m/s"] {
        let line = lex_line(source).expect("lex arrow conversion command");
        assert_eq!(line.kind, LineKind::Command, "{source}");
    }

    assert_eq!(
        kinds("[1 2 3]"),
        vec![
            TokenKind::OpenBracket,
            TokenKind::Number {
                text: "1".into(),
                kind: NumberLiteralKind::Integer,
            },
            TokenKind::Number {
                text: "2".into(),
                kind: NumberLiteralKind::Integer,
            },
            TokenKind::Number {
                text: "3".into(),
                kind: NumberLiteralKind::Integer,
            },
            TokenKind::CloseBracket,
        ]
    );

    assert_eq!(
        kinds("7rem 2"),
        vec![
            TokenKind::Number {
                text: "7".into(),
                kind: NumberLiteralKind::Integer,
            },
            TokenKind::Identifier("rem".into()),
            TokenKind::Number {
                text: "2".into(),
                kind: NumberLiteralKind::Integer,
            },
        ]
    );
    assert_eq!(
        kinds("3mod -2"),
        vec![
            TokenKind::Number {
                text: "3".into(),
                kind: NumberLiteralKind::Integer,
            },
            TokenKind::Identifier("mod".into()),
            TokenKind::Operator(Operator::Minus),
            TokenKind::Number {
                text: "2".into(),
                kind: NumberLiteralKind::Integer,
            },
        ]
    );
    assert_eq!(
        kinds("5div 2"),
        vec![
            TokenKind::Number {
                text: "5".into(),
                kind: NumberLiteralKind::Integer,
            },
            TokenKind::Identifier("div".into()),
            TokenKind::Number {
                text: "2".into(),
                kind: NumberLiteralKind::Integer,
            },
        ]
    );
}

#[test]
fn tokenizes_unicode_ascii_and_word_operator_aliases() {
    assert_eq!(
        kinds("1 ± 2 <= 3 ≥ 4 != 5 × 6 ÷ 7"),
        vec![
            TokenKind::Number {
                text: "1".into(),
                kind: NumberLiteralKind::Integer,
            },
            TokenKind::Operator(Operator::Uncertainty),
            TokenKind::Number {
                text: "2".into(),
                kind: NumberLiteralKind::Integer,
            },
            TokenKind::Operator(Operator::LessOrEqual),
            TokenKind::Number {
                text: "3".into(),
                kind: NumberLiteralKind::Integer,
            },
            TokenKind::Operator(Operator::GreaterOrEqual),
            TokenKind::Number {
                text: "4".into(),
                kind: NumberLiteralKind::Integer,
            },
            TokenKind::Operator(Operator::NotEqual),
            TokenKind::Number {
                text: "5".into(),
                kind: NumberLiteralKind::Integer,
            },
            TokenKind::Operator(Operator::Multiply),
            TokenKind::Number {
                text: "6".into(),
                kind: NumberLiteralKind::Integer,
            },
            TokenKind::Operator(Operator::Divide),
            TokenKind::Number {
                text: "7".into(),
                kind: NumberLiteralKind::Integer,
            },
        ]
    );

    assert_eq!(
        kinds("not a and b xor c bitxor d"),
        vec![
            TokenKind::Operator(Operator::LogicalNot),
            TokenKind::Identifier("a".into()),
            TokenKind::Operator(Operator::LogicalAnd),
            TokenKind::Identifier("b".into()),
            TokenKind::Operator(Operator::LogicalXor),
            TokenKind::Identifier("c".into()),
            TokenKind::Operator(Operator::BitwiseXor),
            TokenKind::Identifier("d".into()),
        ]
    );
}

#[test]
fn tokenizes_operator_spellings_from_upstream_operator_fixtures() {
    // Fixture sources: ../libqalculate/tests/operators.batch:27-55 and
    // ../libqalculate/tests/bitwise.batch:11-33.
    assert_eq!(
        kinds("5 ⋅ 6 ∕ 2"),
        vec![
            TokenKind::Number {
                text: "5".into(),
                kind: NumberLiteralKind::Integer,
            },
            TokenKind::Operator(Operator::Multiply),
            TokenKind::Number {
                text: "6".into(),
                kind: NumberLiteralKind::Integer,
            },
            TokenKind::Operator(Operator::Divide),
            TokenKind::Number {
                text: "2".into(),
                kind: NumberLiteralKind::Integer,
            },
        ]
    );

    for (source, operator) in [
        ("7 rem 2", Operator::Percent),
        ("3 %% 2", Operator::Modulo),
        ("3 mod -2", Operator::Modulo),
        ("5//2", Operator::IntegerDivide),
        (r"5\2", Operator::IntegerDivide),
        ("5 div 2", Operator::IntegerDivide),
    ] {
        assert!(
            kinds(source).contains(&TokenKind::Operator(operator)),
            "{source}"
        );
    }

    assert_eq!(
        kinds("18 << 1"),
        vec![
            TokenKind::Number {
                text: "18".into(),
                kind: NumberLiteralKind::Integer,
            },
            TokenKind::Operator(Operator::ShiftLeft),
            TokenKind::Number {
                text: "1".into(),
                kind: NumberLiteralKind::Integer,
            },
        ]
    );

    assert_eq!(
        kinds("18 >> 2"),
        vec![
            TokenKind::Number {
                text: "18".into(),
                kind: NumberLiteralKind::Integer,
            },
            TokenKind::Operator(Operator::ShiftRight),
            TokenKind::Number {
                text: "2".into(),
                kind: NumberLiteralKind::Integer,
            },
        ]
    );
}

#[test]
fn tokenizes_strings_comments_grouping_and_unit_like_names() {
    // Fixture source: ../libqalculate/tests/strings.batch:14.
    assert_eq!(
        kinds(r#"concatenate("a", "bc", 'defg')"#),
        vec![
            TokenKind::Identifier("concatenate".into()),
            TokenKind::OpenParen,
            TokenKind::StringLiteral("a".into()),
            TokenKind::Comma,
            TokenKind::StringLiteral("bc".into()),
            TokenKind::Comma,
            TokenKind::StringLiteral("defg".into()),
            TokenKind::CloseParen,
        ]
    );

    assert_eq!(
        kinds("sin(\"a # b\", 90) # keep comment"),
        vec![
            TokenKind::Identifier("sin".into()),
            TokenKind::OpenParen,
            TokenKind::StringLiteral("a # b".into()),
            TokenKind::Comma,
            TokenKind::Number {
                text: "90".into(),
                kind: NumberLiteralKind::Integer,
            },
            TokenKind::CloseParen,
            TokenKind::Comment(" keep comment".into()),
        ]
    );

    assert_eq!(
        kinds("50 Ω * 2 µA"),
        vec![
            TokenKind::Number {
                text: "50".into(),
                kind: NumberLiteralKind::Integer,
            },
            TokenKind::Identifier("Ω".into()),
            TokenKind::Operator(Operator::Multiply),
            TokenKind::Number {
                text: "2".into(),
                kind: NumberLiteralKind::Integer,
            },
            TokenKind::Identifier("µA".into()),
        ]
    );

    assert_eq!(
        kinds("6 561 ft to ?m"),
        vec![
            TokenKind::Number {
                text: "6 561".into(),
                kind: NumberLiteralKind::Integer,
            },
            TokenKind::Identifier("ft".into()),
            TokenKind::Operator(Operator::Conversion),
            TokenKind::Identifier("?m".into()),
        ]
    );

    assert_eq!(
        kinds("1 $ to EUR"),
        vec![
            TokenKind::Number {
                text: "1".into(),
                kind: NumberLiteralKind::Integer,
            },
            TokenKind::Identifier("$".into()),
            TokenKind::Operator(Operator::Conversion),
            TokenKind::Identifier("EUR".into()),
        ]
    );
}

#[test]
fn tokenizes_assignment_from_string_and_variable_fixtures() {
    // Fixture sources: ../libqalculate/tests/strings.batch:24 and
    // ../libqalculate/tests/variables.batch:1.
    assert_eq!(
        kinds(r#"alpha:="c""#),
        vec![
            TokenKind::Identifier("alpha".into()),
            TokenKind::Operator(Operator::Assignment),
            TokenKind::StringLiteral("c".into()),
        ]
    );

    assert_eq!(
        kinds("alpha := 5"),
        vec![
            TokenKind::Identifier("alpha".into()),
            TokenKind::Operator(Operator::Assignment),
            TokenKind::Number {
                text: "5".into(),
                kind: NumberLiteralKind::Integer,
            },
        ]
    );
}

#[test]
fn classifies_session_commands_without_discarding_content() {
    // Fixture sources include ../libqalculate/tests/strings.batch:40 and
    // ../libqalculate/tests/numberbase.batch:27.
    for source in [
        "/set unicode 1",
        "set input base 16",
        "/assume positive",
        "/set approximation exact",
        "/set ic 2",
    ] {
        let line = lex_line(source).expect("lex command line");
        assert_eq!(line.kind, LineKind::Command);
        assert_eq!(line.source, source);
        assert!(
            !line.tokens.is_empty(),
            "command tokens retained for {source}"
        );
        if source.starts_with('/') {
            assert_eq!(line.tokens[0].kind, TokenKind::CommandPrefix);
            assert!(!line
                .tokens
                .iter()
                .any(|token| token.kind == TokenKind::Operator(Operator::Divide)));
        }
    }

    let expression = lex_line("setback + 1").expect("lex expression line");
    assert_eq!(expression.kind, LineKind::Expression);

    for source in ["exact", "help", "save definitions", "to hex", "clear"] {
        let line = lex_line(source).expect("lex command line");
        assert_eq!(line.kind, LineKind::Command, "{source}");
    }

    for source in ["partial fraction x", "MC", "MS", "M+", "M-"] {
        let line = lex_line(source).expect("lex command line");
        assert_eq!(line.kind, LineKind::Command, "{source}");
    }

    for source in ["exact 1", "mode ([1 3])"] {
        let line = lex_line(source).expect("lex expression line");
        assert_eq!(line.kind, LineKind::Expression, "{source}");
    }

    let slash_no_arg_with_payload = lex_line("/exact 1").expect("lex slash command");
    assert_eq!(slash_no_arg_with_payload.kind, LineKind::Command);
}

#[test]
fn retains_input_base_number_expression_fragments_for_session_parser() {
    // Fixture source: ../libqalculate/tests/numberbase.batch:27-28.
    let command = lex_line("set input base 16").expect("lex command");
    assert_eq!(command.kind, LineKind::Command);

    assert_eq!(
        kinds("5p10+AEp-2*p23"),
        vec![
            TokenKind::Number {
                text: "5".into(),
                kind: NumberLiteralKind::Integer,
            },
            TokenKind::Identifier("p10".into()),
            TokenKind::Operator(Operator::Plus),
            TokenKind::Identifier("AEp".into()),
            TokenKind::Operator(Operator::Minus),
            TokenKind::Number {
                text: "2".into(),
                kind: NumberLiteralKind::Integer,
            },
            TokenKind::Operator(Operator::Multiply),
            TokenKind::Identifier("p23".into()),
        ]
    );
}

#[test]
fn rejects_interior_nul_and_unterminated_strings_without_panicking() {
    let nul = lex_expression("1\0+2").expect_err("interior NUL is lexical error");
    assert_eq!(nul.kind, LexErrorKind::InteriorNul);
    assert_eq!(nul.span.range(), 1..2);

    let comment_nul = lex_expression("1 # \0").expect_err("interior NUL in comment is rejected");
    assert_eq!(comment_nul.kind, LexErrorKind::InteriorNul);
    assert_eq!(comment_nul.span.range(), 4..5);

    let string =
        lex_expression("\"unterminated").expect_err("unterminated string is lexical error");
    assert_eq!(string.kind, LexErrorKind::UnterminatedString);
    assert_eq!(string.span.range(), 0..13);

    let unexpected = lex_expression("@").expect_err("unexpected character is lexical error");
    assert_eq!(unexpected.kind, LexErrorKind::UnexpectedCharacter('@'));
    assert_eq!(unexpected.span.range(), 0..1);
    assert!(unexpected.to_string().contains("unexpected character"));
}

#[test]
fn tokenizes_uncertainty_ascii_spelling_and_percent_operators() {
    assert_eq!(
        kinds("1 +/- 0.2 + 20%"),
        vec![
            TokenKind::Number {
                text: "1".into(),
                kind: NumberLiteralKind::Integer,
            },
            TokenKind::Operator(Operator::Uncertainty),
            TokenKind::Number {
                text: "0.2".into(),
                kind: NumberLiteralKind::Decimal,
            },
            TokenKind::Operator(Operator::Plus),
            TokenKind::Number {
                text: "20".into(),
                kind: NumberLiteralKind::Integer,
            },
            TokenKind::Operator(Operator::Percent),
        ]
    );
}
