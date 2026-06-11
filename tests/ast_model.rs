use libqalculate_rust::ast::{
    Associativity, ComparisonOperator, DatasetRef, DateTimeLiteral, DefinitionKind, DefinitionRef,
    Expression, FunctionRef, NaryChildren, OperatorArity, PrecedenceClass, PrefixRef,
    StructureKind, Symbol, UnitRef, VariableRef,
};
use libqalculate_rust::number::Number;

fn n(value: i32) -> Expression {
    Expression::Number(Number::from_i32(value))
}

fn operands(children: Vec<Expression>) -> NaryChildren {
    NaryChildren::new(children).expect("valid n-ary operands")
}

#[test]
fn constructs_every_upstream_mathstructure_variant_with_shape_metadata() {
    let function = FunctionRef::new("functions:sin");
    let meter = UnitRef::new("units:meter");
    let kilo = PrefixRef::new("prefixes:kilo");
    let variable = VariableRef::new("variables:x");

    let cases = vec![
        (
            Expression::Multiplication(operands(vec![n(2), n(3), n(4)])),
            StructureKind::Multiplication,
            3,
            Some(OperatorArity::AtLeast(2)),
            Some(Associativity::Associative),
            Some(PrecedenceClass::Multiplicative),
        ),
        (
            Expression::Inverse(Box::new(n(5))),
            StructureKind::Inverse,
            1,
            Some(OperatorArity::Exact(1)),
            Some(Associativity::Prefix),
            Some(PrecedenceClass::Prefix),
        ),
        (
            Expression::Division {
                numerator: Box::new(n(6)),
                denominator: Box::new(n(2)),
            },
            StructureKind::Division,
            2,
            Some(OperatorArity::Exact(2)),
            Some(Associativity::Left),
            Some(PrecedenceClass::Multiplicative),
        ),
        (
            Expression::Addition(operands(vec![n(1), n(2), n(3)])),
            StructureKind::Addition,
            3,
            Some(OperatorArity::AtLeast(2)),
            Some(Associativity::Associative),
            Some(PrecedenceClass::Additive),
        ),
        (
            Expression::Negate(Box::new(n(7))),
            StructureKind::Negate,
            1,
            Some(OperatorArity::Exact(1)),
            Some(Associativity::Prefix),
            Some(PrecedenceClass::Prefix),
        ),
        (
            Expression::Power {
                base: Box::new(n(2)),
                exponent: Box::new(n(8)),
            },
            StructureKind::Power,
            2,
            Some(OperatorArity::Exact(2)),
            Some(Associativity::Right),
            Some(PrecedenceClass::Power),
        ),
        (n(42), StructureKind::Number, 0, None, None, None),
        (
            Expression::Unit {
                unit: meter,
                prefix: Some(kilo),
                plural: false,
            },
            StructureKind::Unit,
            0,
            None,
            None,
            None,
        ),
        (
            Expression::Symbolic(Symbol::new("alpha")),
            StructureKind::Symbolic,
            0,
            None,
            None,
            None,
        ),
        (
            Expression::FunctionCall {
                function,
                args: vec![n(90)],
            },
            StructureKind::Function,
            1,
            Some(OperatorArity::Any),
            Some(Associativity::None),
            Some(PrecedenceClass::Primary),
        ),
        (
            Expression::Variable(variable),
            StructureKind::Variable,
            0,
            None,
            None,
            None,
        ),
        (
            Expression::Vector(vec![n(1), n(2)]),
            StructureKind::Vector,
            2,
            None,
            None,
            None,
        ),
        (
            Expression::BitwiseAnd(operands(vec![n(7), n(3)])),
            StructureKind::BitwiseAnd,
            2,
            Some(OperatorArity::AtLeast(2)),
            Some(Associativity::Associative),
            Some(PrecedenceClass::BitwiseAnd),
        ),
        (
            Expression::BitwiseOr(operands(vec![n(4), n(1)])),
            StructureKind::BitwiseOr,
            2,
            Some(OperatorArity::AtLeast(2)),
            Some(Associativity::Associative),
            Some(PrecedenceClass::BitwiseOr),
        ),
        (
            Expression::BitwiseXor(operands(vec![n(6), n(2), n(4)])),
            StructureKind::BitwiseXor,
            3,
            Some(OperatorArity::AtLeast(2)),
            Some(Associativity::Associative),
            Some(PrecedenceClass::BitwiseXor),
        ),
        (
            Expression::BitwiseNot(Box::new(n(1))),
            StructureKind::BitwiseNot,
            1,
            Some(OperatorArity::Exact(1)),
            Some(Associativity::Prefix),
            Some(PrecedenceClass::Prefix),
        ),
        (
            Expression::LogicalAnd(operands(vec![n(1), n(0)])),
            StructureKind::LogicalAnd,
            2,
            Some(OperatorArity::AtLeast(2)),
            Some(Associativity::Associative),
            Some(PrecedenceClass::LogicalAnd),
        ),
        (
            Expression::LogicalOr(operands(vec![n(1), n(0)])),
            StructureKind::LogicalOr,
            2,
            Some(OperatorArity::AtLeast(2)),
            Some(Associativity::Associative),
            Some(PrecedenceClass::LogicalOr),
        ),
        (
            Expression::LogicalXor {
                lhs: Box::new(n(1)),
                rhs: Box::new(n(0)),
            },
            StructureKind::LogicalXor,
            2,
            Some(OperatorArity::Exact(2)),
            Some(Associativity::Left),
            Some(PrecedenceClass::LogicalXor),
        ),
        (
            Expression::LogicalNot(Box::new(n(0))),
            StructureKind::LogicalNot,
            1,
            Some(OperatorArity::Exact(1)),
            Some(Associativity::Prefix),
            Some(PrecedenceClass::Prefix),
        ),
        (
            Expression::Comparison {
                op: ComparisonOperator::LessOrEqual,
                lhs: Box::new(n(2)),
                rhs: Box::new(n(3)),
            },
            StructureKind::Comparison,
            2,
            Some(OperatorArity::Exact(2)),
            Some(Associativity::None),
            Some(PrecedenceClass::Comparison),
        ),
        (
            Expression::Undefined,
            StructureKind::Undefined,
            0,
            None,
            None,
            None,
        ),
        (
            Expression::Aborted,
            StructureKind::Aborted,
            0,
            None,
            None,
            None,
        ),
        (
            Expression::DateTime(DateTimeLiteral::new("2026-06-12T00:00:00+07:00")),
            StructureKind::DateTime,
            0,
            None,
            None,
            None,
        ),
    ];

    for (expr, kind, child_count, arity, associativity, precedence) in cases {
        assert_eq!(expr.structure_kind(), kind);
        assert_eq!(expr.child_count(), child_count);
        assert_eq!(expr.operator_metadata().map(|meta| meta.arity), arity);
        assert_eq!(
            expr.operator_metadata().map(|meta| meta.associativity),
            associativity
        );
        assert_eq!(
            expr.operator_metadata().map(|meta| meta.precedence),
            precedence
        );
    }
}

#[test]
fn preserves_child_order_for_precedence_sensitive_shapes() {
    let multiplication = Expression::Multiplication(operands(vec![n(2), n(3), n(4)]));
    assert_eq!(multiplication.child(0), Some(&n(2)));
    assert_eq!(multiplication.child(1), Some(&n(3)));
    assert_eq!(multiplication.child(2), Some(&n(4)));
    assert_eq!(multiplication.child(3), None);

    let addition = Expression::Addition(operands(vec![n(1), n(2), n(3)]));
    assert_eq!(addition.child(0), Some(&n(1)));
    assert_eq!(addition.child(1), Some(&n(2)));
    assert_eq!(addition.child(2), Some(&n(3)));

    let function = Expression::FunctionCall {
        function: FunctionRef::new("functions:max"),
        args: vec![n(5), n(9)],
    };
    assert_eq!(function.child(0), Some(&n(5)));
    assert_eq!(function.child(1), Some(&n(9)));

    let vector = Expression::Vector(vec![n(8), n(13)]);
    assert_eq!(vector.child(0), Some(&n(8)));
    assert_eq!(vector.child(1), Some(&n(13)));

    let power = Expression::Power {
        base: Box::new(n(2)),
        exponent: Box::new(n(8)),
    };
    assert_eq!(power.child(0), Some(&n(2)));
    assert_eq!(power.child(1), Some(&n(8)));
    assert_eq!(power.child(2), None);

    let division = Expression::Division {
        numerator: Box::new(n(9)),
        denominator: Box::new(n(3)),
    };
    assert_eq!(division.child(0), Some(&n(9)));
    assert_eq!(division.child(1), Some(&n(3)));

    let comparison = Expression::Comparison {
        op: ComparisonOperator::Greater,
        lhs: Box::new(Expression::Symbolic(Symbol::new("x"))),
        rhs: Box::new(n(0)),
    };
    assert_eq!(
        comparison.child(0),
        Some(&Expression::Symbolic(Symbol::new("x")))
    );
    assert_eq!(comparison.child(1), Some(&n(0)));

    let bitwise_and = Expression::BitwiseAnd(operands(vec![n(7), n(3), n(1)]));
    assert_eq!(bitwise_and.child(0), Some(&n(7)));
    assert_eq!(bitwise_and.child(1), Some(&n(3)));
    assert_eq!(bitwise_and.child(2), Some(&n(1)));
    assert_eq!(bitwise_and.child(3), None);

    let bitwise_or = Expression::BitwiseOr(operands(vec![n(4), n(2), n(1)]));
    assert_eq!(bitwise_or.child(0), Some(&n(4)));
    assert_eq!(bitwise_or.child(1), Some(&n(2)));
    assert_eq!(bitwise_or.child(2), Some(&n(1)));
    assert_eq!(bitwise_or.child(3), None);

    let bitwise_xor = Expression::BitwiseXor(operands(vec![n(1), n(2), n(3)]));
    assert_eq!(bitwise_xor.child(0), Some(&n(1)));
    assert_eq!(bitwise_xor.child(1), Some(&n(2)));
    assert_eq!(bitwise_xor.child(2), Some(&n(3)));
    assert_eq!(bitwise_xor.child(3), None);

    let logical_and = Expression::LogicalAnd(operands(vec![n(1), n(0), n(1)]));
    assert_eq!(logical_and.child(0), Some(&n(1)));
    assert_eq!(logical_and.child(1), Some(&n(0)));
    assert_eq!(logical_and.child(2), Some(&n(1)));
    assert_eq!(logical_and.child(3), None);

    let logical = Expression::LogicalOr(operands(vec![n(0), n(1), n(0)]));
    assert_eq!(logical.child(0), Some(&n(0)));
    assert_eq!(logical.child(1), Some(&n(1)));
    assert_eq!(logical.child(2), Some(&n(0)));

    let logical_xor = Expression::LogicalXor {
        lhs: Box::new(n(1)),
        rhs: Box::new(n(0)),
    };
    assert_eq!(logical_xor.child(0), Some(&n(1)));
    assert_eq!(logical_xor.child(1), Some(&n(0)));
    assert_eq!(logical_xor.child(2), None);

    let unary_nodes = [
        (Expression::Inverse(Box::new(n(11))), n(11)),
        (Expression::Negate(Box::new(n(12))), n(12)),
        (Expression::BitwiseNot(Box::new(n(13))), n(13)),
        (Expression::LogicalNot(Box::new(n(14))), n(14)),
    ];
    for (expr, expected_child) in unary_nodes {
        assert_eq!(expr.child_count(), 1);
        assert_eq!(expr.child(0), Some(&expected_child));
        assert_eq!(expr.child(1), None);
    }
}

#[test]
fn nary_operator_children_reject_empty_and_unary_shapes() {
    let empty = NaryChildren::new(vec![]).expect_err("empty n-ary operands");
    assert_eq!(empty.minimum(), 2);
    assert_eq!(empty.actual(), 0);

    let unary = NaryChildren::new(vec![n(1)]).expect_err("unary n-ary operands");
    assert_eq!(unary.minimum(), 2);
    assert_eq!(unary.actual(), 1);
}

#[test]
fn deep_clone_owns_children_independently() {
    let original = Expression::Addition(operands(vec![n(1), n(2)]));
    let mut cloned = original.clone();

    match &mut cloned {
        Expression::Addition(children) => children.as_mut_slice()[0] = n(99),
        other => panic!("unexpected expression shape: {other:?}"),
    }

    assert_eq!(original.child(0), Some(&n(1)));
    assert_eq!(cloned.child(0), Some(&n(99)));
}

#[test]
fn stable_definition_handles_are_value_based_not_pointer_identity() {
    let first_id = String::from("variables:x");
    let second_id = format!("variables:{}", "x");
    let first = DefinitionRef::new(DefinitionKind::Variable, first_id);
    let second = DefinitionRef::new(DefinitionKind::Variable, second_id);

    assert_eq!(first, second);
    assert_ne!(first.id().as_ptr(), second.id().as_ptr());
    assert_ne!(
        first,
        DefinitionRef::new(DefinitionKind::Unit, "variables:x")
    );
    assert_eq!(first.kind(), DefinitionKind::Variable);
    assert_eq!(first.id(), "variables:x");
}

#[test]
fn vectors_model_matrices_as_vectors_of_vectors() {
    let matrix =
        Expression::matrix(vec![vec![n(1), n(2)], vec![n(3), n(4)]]).expect("valid matrix");

    assert_eq!(matrix.structure_kind(), StructureKind::Vector);
    assert_eq!(matrix.child_count(), 2);
    assert_eq!(
        matrix.child(0).map(Expression::structure_kind),
        Some(StructureKind::Vector)
    );
    assert_eq!(
        matrix.child(1).map(Expression::structure_kind),
        Some(StructureKind::Vector)
    );

    let rows = matrix.as_matrix_rows().expect("matrix rows");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0], &[n(1), n(2)]);
    assert_eq!(rows[1], &[n(3), n(4)]);

    assert!(Expression::matrix(vec![]).is_err());
    assert!(Expression::matrix(vec![vec![n(1)], vec![n(2), n(3)]]).is_err());

    let ragged_vector = Expression::Vector(vec![
        Expression::Vector(vec![n(1)]),
        Expression::Vector(vec![n(2), n(3)]),
    ]);
    assert!(ragged_vector.as_matrix_rows().is_none());
}

#[test]
fn fixture_shape_seeds_cover_later_parser_targets() {
    let x = VariableRef::new("variables:x");
    let sin = FunctionRef::new("functions:sin");
    let meter = UnitRef::new("units:meter");
    let kilo = PrefixRef::new("prefixes:kilo");

    // These are synthetic reduced shape seeds from the fixture families named
    // in #16. They are not copied batch lines; when parser fixtures are ported,
    // refresh this table from exact upstream cases.
    let seeds = vec![
        (
            "parser.batch",
            "5x + 2",
            Expression::Addition(operands(vec![
                Expression::Multiplication(operands(vec![n(5), Expression::Variable(x.clone())])),
                n(2),
            ])),
            StructureKind::Addition,
            2,
        ),
        (
            "operators.batch",
            "2^8",
            Expression::Power {
                base: Box::new(n(2)),
                exponent: Box::new(n(8)),
            },
            StructureKind::Power,
            2,
        ),
        (
            "bitwise.batch",
            "7 bitand 3",
            Expression::BitwiseAnd(operands(vec![n(7), n(3)])),
            StructureKind::BitwiseAnd,
            2,
        ),
        (
            "units.batch",
            "5 km",
            Expression::Multiplication(operands(vec![
                n(5),
                Expression::Unit {
                    unit: meter,
                    prefix: Some(kilo),
                    plural: false,
                },
            ])),
            StructureKind::Multiplication,
            2,
        ),
        (
            "variables.batch",
            "x > 0",
            Expression::Comparison {
                op: ComparisonOperator::Greater,
                lhs: Box::new(Expression::Variable(x)),
                rhs: Box::new(n(0)),
            },
            StructureKind::Comparison,
            2,
        ),
        (
            "matrixvector.batch",
            "[1 2; 3 4]",
            Expression::matrix(vec![vec![n(1), n(2)], vec![n(3), n(4)]]).expect("valid matrix"),
            StructureKind::Vector,
            2,
        ),
        (
            "dates.batch",
            "\"2020-05-20\"",
            Expression::DateTime(DateTimeLiteral::new("2020-05-20")),
            StructureKind::DateTime,
            0,
        ),
        (
            "strings.batch",
            "sin(90)",
            Expression::FunctionCall {
                function: sin,
                args: vec![n(90)],
            },
            StructureKind::Function,
            1,
        ),
        (
            "operators.batch",
            "not 0",
            Expression::LogicalNot(Box::new(n(0))),
            StructureKind::LogicalNot,
            1,
        ),
    ];

    for (fixture, source, expr, kind, child_count) in seeds {
        assert_eq!(expr.structure_kind(), kind, "{fixture}: {source}");
        assert_eq!(expr.child_count(), child_count, "{fixture}: {source}");
        assert_fixture_seed_payload(fixture, source, &expr);
    }
}

fn assert_fixture_seed_payload(fixture: &str, source: &str, expr: &Expression) {
    match (fixture, source, expr) {
        ("parser.batch", "5x + 2", Expression::Addition(children)) => {
            assert_eq!(children.len(), 2);
            assert_eq!(
                children[0],
                Expression::Multiplication(operands(vec![
                    n(5),
                    Expression::Variable(VariableRef::new("variables:x")),
                ]))
            );
            assert_eq!(children[1], n(2));
        }
        ("operators.batch", "2^8", Expression::Power { base, exponent }) => {
            assert_eq!(base.as_ref(), &n(2));
            assert_eq!(exponent.as_ref(), &n(8));
        }
        ("bitwise.batch", "7 bitand 3", Expression::BitwiseAnd(children)) => {
            assert_eq!(children.as_slice(), &[n(7), n(3)]);
        }
        ("units.batch", "5 km", Expression::Multiplication(children)) => {
            assert_eq!(children.len(), 2);
            assert_eq!(children[0], n(5));
            assert_eq!(
                children[1],
                Expression::Unit {
                    unit: UnitRef::new("units:meter"),
                    prefix: Some(PrefixRef::new("prefixes:kilo")),
                    plural: false,
                }
            );
        }
        ("variables.batch", "x > 0", Expression::Comparison { op, lhs, rhs }) => {
            assert_eq!(*op, ComparisonOperator::Greater);
            assert_eq!(
                lhs.as_ref(),
                &Expression::Variable(VariableRef::new("variables:x"))
            );
            assert_eq!(rhs.as_ref(), &n(0));
        }
        ("matrixvector.batch", "[1 2; 3 4]", Expression::Vector(rows)) => {
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0], Expression::Vector(vec![n(1), n(2)]));
            assert_eq!(rows[1], Expression::Vector(vec![n(3), n(4)]));
        }
        ("dates.batch", "\"2020-05-20\"", Expression::DateTime(literal)) => {
            assert_eq!(literal.source(), "2020-05-20");
        }
        ("strings.batch", "sin(90)", Expression::FunctionCall { function, args }) => {
            assert_eq!(function, &FunctionRef::new("functions:sin"));
            assert_eq!(args, &[n(90)]);
        }
        ("operators.batch", "not 0", Expression::LogicalNot(child)) => {
            assert_eq!(child.as_ref(), &n(0));
        }
        _ => panic!("unhandled fixture seed shape: {fixture}: {source}: {expr:?}"),
    }
}

#[test]
fn definition_handles_cover_all_definition_families() {
    let handles: Vec<DefinitionRef> = vec![
        FunctionRef::new("functions:sin").into(),
        UnitRef::new("units:meter").into(),
        PrefixRef::new("prefixes:kilo").into(),
        VariableRef::new("variables:x").into(),
        DatasetRef::new("datasets:planets").into(),
    ];

    for handle in handles {
        assert!(!handle.id().is_empty());
    }

    assert_eq!(
        FunctionRef::new("functions:sin").as_definition_ref().kind(),
        DefinitionKind::Function
    );
    assert_eq!(
        UnitRef::new("units:meter").as_definition_ref().kind(),
        DefinitionKind::Unit
    );
    assert_eq!(
        PrefixRef::new("prefixes:kilo").as_definition_ref().kind(),
        DefinitionKind::Prefix
    );
    assert_eq!(
        VariableRef::new("variables:x").as_definition_ref().kind(),
        DefinitionKind::Variable
    );
    assert_eq!(
        DatasetRef::new("datasets:planets")
            .as_definition_ref()
            .kind(),
        DefinitionKind::Dataset
    );
}

#[test]
fn comparison_operator_model_matches_upstream_comparison_type_family() {
    let operators = [
        ComparisonOperator::Less,
        ComparisonOperator::Greater,
        ComparisonOperator::LessOrEqual,
        ComparisonOperator::GreaterOrEqual,
        ComparisonOperator::Equal,
        ComparisonOperator::NotEqual,
    ];

    for op in operators {
        let expr = Expression::Comparison {
            op,
            lhs: Box::new(n(1)),
            rhs: Box::new(n(2)),
        };
        assert_eq!(expr.structure_kind(), StructureKind::Comparison);
        assert_eq!(expr.child_count(), 2);
        assert_eq!(
            expr.operator_metadata().map(|metadata| metadata.arity),
            Some(OperatorArity::Exact(2))
        );
    }
}
