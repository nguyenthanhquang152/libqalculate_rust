//! Rust domain model for upstream `MathStructure` expression trees.
//!
//! This module models tree shape only. Parsing, evaluation, simplification,
//! definition loading, and formatting are intentionally owned by later porting
//! tasks.
//!
//! Upstream oracle files:
//! - `../libqalculate/libqalculate/MathStructure.h` for `StructureType`,
//!   documented container arity, child order, and matrix-as-vector shape.
//! - `../libqalculate/libqalculate/MathStructure.cc` for tree mutation and
//!   child ordering behavior to preserve in later parser/evaluator tasks.
//! - `../libqalculate/libqalculate/includes.h` for `ComparisonType`.

use crate::number::Number;
use std::ops::Index;

/// Stable kind tag for Rust expression categories used by the staged port.
///
/// Most variants mirror upstream `MathStructure::StructureType`. Parser-stage
/// operator variants such as remainder, shifts, factorial, and percent are
/// Rust-side placeholders for upstream function-backed forms until function
/// definitions and evaluation are ported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StructureKind {
    /// Multiplication node with ordered factors.
    Multiplication,
    /// Formatted inverse node with one child.
    Inverse,
    /// Formatted division node with numerator then denominator.
    Division,
    /// Addition node with ordered terms.
    Addition,
    /// Formatted negation node with one child.
    Negate,
    /// Power node with base then exponent.
    Power,
    /// Remainder operation node.
    Remainder,
    /// Modulo operation node.
    Modulo,
    /// Integer division operation node.
    IntegerDivision,
    /// Bitwise left-shift operation node.
    ShiftLeft,
    /// Bitwise right-shift operation node.
    ShiftRight,
    /// Factorial operation node.
    Factorial,
    /// Percent operation node.
    Percent,
    /// Numeric value node.
    Number,
    /// Unit reference node.
    Unit,
    /// Symbolic text node.
    Symbolic,
    /// Function call node.
    Function,
    /// Variable reference node.
    Variable,
    /// Vector node, including matrix rows represented as nested vectors.
    Vector,
    /// Bitwise-and operation node.
    BitwiseAnd,
    /// Bitwise-or operation node.
    BitwiseOr,
    /// Bitwise-xor operation node.
    BitwiseXor,
    /// Bitwise-not operation node.
    BitwiseNot,
    /// Logical-and operation node.
    LogicalAnd,
    /// Logical-or operation node.
    LogicalOr,
    /// Logical-xor operation node.
    LogicalXor,
    /// Logical-not operation node.
    LogicalNot,
    /// Comparison operation node.
    Comparison,
    /// Undefined value node.
    Undefined,
    /// Aborted calculation sentinel node.
    Aborted,
    /// Date/time value node.
    DateTime,
}

/// Arity contract for an operator-shaped expression node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperatorArity {
    /// The node must have exactly this many children.
    Exact(usize),
    /// The node must have at least this many children.
    AtLeast(usize),
    /// The node accepts any number of children.
    Any,
}

/// Associativity contract for precedence-sensitive expression nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Associativity {
    /// Grouping does not affect the operator result.
    Associative,
    /// Binary operator groups left-to-right.
    Left,
    /// Binary operator groups right-to-left.
    Right,
    /// Prefix unary operator.
    Prefix,
    /// Operator has no associative grouping rule.
    None,
}

/// Precedence bucket used by parser and formatter tasks to preserve tree shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrecedenceClass {
    /// Primary expression such as a literal or function call.
    Primary,
    /// Prefix unary operation.
    Prefix,
    /// Exponentiation.
    Power,
    /// Multiplication, division, and inverse.
    Multiplicative,
    /// Addition and negation.
    Additive,
    /// Bitwise shifts.
    Shift,
    /// Comparison operators.
    Comparison,
    /// Bitwise and.
    BitwiseAnd,
    /// Bitwise xor.
    BitwiseXor,
    /// Bitwise or.
    BitwiseOr,
    /// Logical and.
    LogicalAnd,
    /// Logical xor.
    LogicalXor,
    /// Logical or.
    LogicalOr,
}

/// Shape metadata for an operator-backed expression node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OperatorMetadata {
    /// Number of children accepted by the operator.
    pub arity: OperatorArity,
    /// Associativity used when parsing or formatting adjacent operators.
    pub associativity: Associativity,
    /// Precedence bucket used when nesting this node under another operator.
    pub precedence: PrecedenceClass,
}

impl OperatorMetadata {
    const fn new(
        arity: OperatorArity,
        associativity: Associativity,
        precedence: PrecedenceClass,
    ) -> Self {
        Self {
            arity,
            associativity,
            precedence,
        }
    }
}

/// Error returned when an n-ary operator is built with too few children.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArityError {
    minimum: usize,
    actual: usize,
}

impl ArityError {
    /// Returns the minimum number of children required.
    pub fn minimum(&self) -> usize {
        self.minimum
    }

    /// Returns the number of children provided.
    pub fn actual(&self) -> usize {
        self.actual
    }
}

/// Owned children for operators that require two or more operands.
#[derive(Debug, Clone, PartialEq)]
pub struct NaryChildren {
    children: Vec<Expression>,
}

impl NaryChildren {
    /// Creates n-ary children, rejecting empty and unary operator shapes.
    pub fn new(children: Vec<Expression>) -> Result<Self, ArityError> {
        if children.len() < 2 {
            return Err(ArityError {
                minimum: 2,
                actual: children.len(),
            });
        }

        Ok(Self { children })
    }

    /// Creates n-ary children from two required operands and optional trailing operands.
    pub fn from_two(first: Expression, second: Expression, rest: Vec<Expression>) -> Self {
        let mut children = Vec::with_capacity(2 + rest.len());
        children.push(first);
        children.push(second);
        children.extend(rest);
        Self { children }
    }

    /// Returns the number of children.
    pub fn len(&self) -> usize {
        self.children.len()
    }

    /// Returns whether there are no children.
    ///
    /// This always returns `false`; it exists to pair with `len` for generic callers.
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    /// Returns a child by zero-based index.
    pub fn get(&self, index: usize) -> Option<&Expression> {
        self.children.get(index)
    }

    /// Returns all children in upstream child order.
    pub fn as_slice(&self) -> &[Expression] {
        &self.children
    }

    /// Returns mutable children in upstream child order without allowing length changes.
    pub fn as_mut_slice(&mut self) -> &mut [Expression] {
        &mut self.children
    }
}

impl Index<usize> for NaryChildren {
    type Output = Expression;

    fn index(&self, index: usize) -> &Self::Output {
        &self.children[index]
    }
}

/// Type of upstream definition referenced by an expression node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DefinitionKind {
    /// Math function definition.
    Function,
    /// Unit definition.
    Unit,
    /// Unit prefix definition.
    Prefix,
    /// Variable definition.
    Variable,
    /// Dataset definition.
    Dataset,
}

/// Stable reference to a definition-backed upstream item.
///
/// This is intentionally value-based. It must not encode raw pointer identity
/// from the C++ implementation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DefinitionRef {
    kind: DefinitionKind,
    id: String,
}

impl DefinitionRef {
    /// Creates a stable definition reference.
    pub fn new(kind: DefinitionKind, id: impl Into<String>) -> Self {
        Self {
            kind,
            id: id.into(),
        }
    }

    /// Returns the referenced definition category.
    pub fn kind(&self) -> DefinitionKind {
        self.kind
    }

    /// Returns the stable definition identifier.
    pub fn id(&self) -> &str {
        &self.id
    }
}

macro_rules! typed_definition_ref {
    ($(#[$meta:meta])* $name:ident, $kind:expr) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $name(DefinitionRef);

        impl $name {
            /// Creates a stable typed definition reference.
            pub fn new(id: impl Into<String>) -> Self {
                Self(DefinitionRef::new($kind, id))
            }

            /// Returns the stable definition identifier.
            pub fn id(&self) -> &str {
                self.0.id()
            }

            /// Returns the generic definition reference.
            pub fn as_definition_ref(&self) -> &DefinitionRef {
                &self.0
            }
        }

        impl From<$name> for DefinitionRef {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

typed_definition_ref!(
    /// Stable handle for a math function definition.
    FunctionRef,
    DefinitionKind::Function
);
typed_definition_ref!(
    /// Stable handle for a unit definition.
    UnitRef,
    DefinitionKind::Unit
);
typed_definition_ref!(
    /// Stable handle for a unit prefix definition.
    PrefixRef,
    DefinitionKind::Prefix
);
typed_definition_ref!(
    /// Stable handle for a variable definition.
    VariableRef,
    DefinitionKind::Variable
);
typed_definition_ref!(
    /// Stable handle for a dataset definition.
    DatasetRef,
    DefinitionKind::Dataset
);

/// Symbolic leaf text used for unknown names and symbolic values.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Symbol {
    name: String,
}

impl Symbol {
    /// Creates a symbolic leaf from its source text.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    /// Returns the symbolic source text.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Date/time literal carried by an AST leaf before full date semantics are ported.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DateTimeLiteral {
    source: String,
}

impl DateTimeLiteral {
    /// Creates a date/time literal from its source representation.
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
        }
    }

    /// Returns the source representation for this date/time literal.
    pub fn source(&self) -> &str {
        &self.source
    }
}

/// Comparison sign stored by comparison expression nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComparisonOperator {
    /// Less-than comparison.
    Less,
    /// Greater-than comparison.
    Greater,
    /// Less-than-or-equal comparison.
    LessOrEqual,
    /// Greater-than-or-equal comparison.
    GreaterOrEqual,
    /// Equality comparison.
    Equal,
    /// Inequality comparison.
    NotEqual,
}

/// Error returned when a vector-of-vectors cannot be treated as a matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MatrixShapeError {
    /// A matrix needs at least one row.
    Empty,
    /// A row has a different width than the first row.
    Ragged {
        /// Zero-based row index with the mismatched width.
        row: usize,
        /// Width required by the first row.
        expected: usize,
        /// Width found in this row.
        actual: usize,
    },
}

/// Owned expression tree node for the Rust `MathStructure` port.
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// Multiplication with ordered factors.
    Multiplication(NaryChildren),
    /// Formatted inverse with one child.
    Inverse(Box<Expression>),
    /// Formatted division with numerator then denominator.
    Division {
        /// Numerator expression.
        numerator: Box<Expression>,
        /// Denominator expression.
        denominator: Box<Expression>,
    },
    /// Addition with ordered terms.
    Addition(NaryChildren),
    /// Formatted negation with one child.
    Negate(Box<Expression>),
    /// Power with base then exponent.
    Power {
        /// Base expression.
        base: Box<Expression>,
        /// Exponent expression.
        exponent: Box<Expression>,
    },
    /// Remainder operation with left then right child.
    Remainder {
        /// Left-hand side expression.
        lhs: Box<Expression>,
        /// Right-hand side expression.
        rhs: Box<Expression>,
    },
    /// Modulo operation with left then right child.
    Modulo {
        /// Left-hand side expression.
        lhs: Box<Expression>,
        /// Right-hand side expression.
        rhs: Box<Expression>,
    },
    /// Integer division operation with left then right child.
    IntegerDivision {
        /// Left-hand side expression.
        lhs: Box<Expression>,
        /// Right-hand side expression.
        rhs: Box<Expression>,
    },
    /// Bitwise left shift with left then right child.
    ShiftLeft {
        /// Left-hand side expression.
        lhs: Box<Expression>,
        /// Right-hand side expression.
        rhs: Box<Expression>,
    },
    /// Bitwise right shift with left then right child.
    ShiftRight {
        /// Left-hand side expression.
        lhs: Box<Expression>,
        /// Right-hand side expression.
        rhs: Box<Expression>,
    },
    /// Factorial operation with one child.
    Factorial(Box<Expression>),
    /// Percent operation with one child.
    Percent(Box<Expression>),
    /// Numeric value.
    Number(Number),
    /// Unit reference and optional formatted prefix.
    Unit {
        /// Stable unit definition handle.
        unit: UnitRef,
        /// Optional stable prefix definition handle.
        prefix: Option<PrefixRef>,
        /// Whether the formatted unit is plural.
        plural: bool,
    },
    /// Symbolic text leaf.
    Symbolic(Symbol),
    /// Function call with stable function handle and ordered arguments.
    FunctionCall {
        /// Stable function definition handle.
        function: FunctionRef,
        /// Ordered call arguments.
        args: Vec<Expression>,
    },
    /// Variable reference.
    Variable(VariableRef),
    /// Vector value. Matrices are represented as vectors of vector rows.
    Vector(Vec<Expression>),
    /// Bitwise-and expression.
    BitwiseAnd(NaryChildren),
    /// Bitwise-or expression.
    BitwiseOr(NaryChildren),
    /// Bitwise-xor expression.
    BitwiseXor(NaryChildren),
    /// Bitwise-not expression.
    BitwiseNot(Box<Expression>),
    /// Logical-and expression.
    LogicalAnd(NaryChildren),
    /// Logical-or expression.
    LogicalOr(NaryChildren),
    /// Logical-xor expression with left then right child.
    LogicalXor {
        /// Left-hand side expression.
        lhs: Box<Expression>,
        /// Right-hand side expression.
        rhs: Box<Expression>,
    },
    /// Logical-not expression.
    LogicalNot(Box<Expression>),
    /// Comparison with left-hand side then right-hand side children.
    Comparison {
        /// Comparison sign.
        op: ComparisonOperator,
        /// Left-hand side expression.
        lhs: Box<Expression>,
        /// Right-hand side expression.
        rhs: Box<Expression>,
    },
    /// Undefined value sentinel.
    Undefined,
    /// Aborted calculation sentinel.
    Aborted,
    /// Date/time literal.
    DateTime(DateTimeLiteral),
}

impl Expression {
    /// Creates a matrix using upstream's vector-of-vectors representation.
    pub fn matrix(rows: Vec<Vec<Expression>>) -> Result<Self, MatrixShapeError> {
        validate_matrix_rows(&rows)?;
        Ok(Self::Vector(rows.into_iter().map(Self::Vector).collect()))
    }

    /// Returns the upstream-compatible structure kind for this expression node.
    pub fn structure_kind(&self) -> StructureKind {
        match self {
            Self::Multiplication(_) => StructureKind::Multiplication,
            Self::Inverse(_) => StructureKind::Inverse,
            Self::Division { .. } => StructureKind::Division,
            Self::Addition(_) => StructureKind::Addition,
            Self::Negate(_) => StructureKind::Negate,
            Self::Power { .. } => StructureKind::Power,
            Self::Remainder { .. } => StructureKind::Remainder,
            Self::Modulo { .. } => StructureKind::Modulo,
            Self::IntegerDivision { .. } => StructureKind::IntegerDivision,
            Self::ShiftLeft { .. } => StructureKind::ShiftLeft,
            Self::ShiftRight { .. } => StructureKind::ShiftRight,
            Self::Factorial(_) => StructureKind::Factorial,
            Self::Percent(_) => StructureKind::Percent,
            Self::Number(_) => StructureKind::Number,
            Self::Unit { .. } => StructureKind::Unit,
            Self::Symbolic(_) => StructureKind::Symbolic,
            Self::FunctionCall { .. } => StructureKind::Function,
            Self::Variable(_) => StructureKind::Variable,
            Self::Vector(_) => StructureKind::Vector,
            Self::BitwiseAnd(_) => StructureKind::BitwiseAnd,
            Self::BitwiseOr(_) => StructureKind::BitwiseOr,
            Self::BitwiseXor(_) => StructureKind::BitwiseXor,
            Self::BitwiseNot(_) => StructureKind::BitwiseNot,
            Self::LogicalAnd(_) => StructureKind::LogicalAnd,
            Self::LogicalOr(_) => StructureKind::LogicalOr,
            Self::LogicalXor { .. } => StructureKind::LogicalXor,
            Self::LogicalNot(_) => StructureKind::LogicalNot,
            Self::Comparison { .. } => StructureKind::Comparison,
            Self::Undefined => StructureKind::Undefined,
            Self::Aborted => StructureKind::Aborted,
            Self::DateTime(_) => StructureKind::DateTime,
        }
    }

    /// Returns shape metadata for operator-backed nodes.
    pub fn operator_metadata(&self) -> Option<OperatorMetadata> {
        match self {
            Self::Multiplication(_) => Some(OperatorMetadata::new(
                OperatorArity::AtLeast(2),
                Associativity::Associative,
                PrecedenceClass::Multiplicative,
            )),
            Self::Inverse(_) => Some(OperatorMetadata::new(
                OperatorArity::Exact(1),
                Associativity::Prefix,
                PrecedenceClass::Prefix,
            )),
            Self::Division { .. } => Some(OperatorMetadata::new(
                OperatorArity::Exact(2),
                Associativity::Left,
                PrecedenceClass::Multiplicative,
            )),
            Self::Addition(_) => Some(OperatorMetadata::new(
                OperatorArity::AtLeast(2),
                Associativity::Associative,
                PrecedenceClass::Additive,
            )),
            Self::Negate(_) => Some(OperatorMetadata::new(
                OperatorArity::Exact(1),
                Associativity::Prefix,
                PrecedenceClass::Prefix,
            )),
            Self::Power { .. } => Some(OperatorMetadata::new(
                OperatorArity::Exact(2),
                Associativity::Right,
                PrecedenceClass::Power,
            )),
            Self::Remainder { .. } | Self::Modulo { .. } | Self::IntegerDivision { .. } => {
                Some(OperatorMetadata::new(
                    OperatorArity::Exact(2),
                    Associativity::Left,
                    PrecedenceClass::Multiplicative,
                ))
            }
            Self::ShiftLeft { .. } | Self::ShiftRight { .. } => Some(OperatorMetadata::new(
                OperatorArity::Exact(2),
                Associativity::Left,
                PrecedenceClass::Shift,
            )),
            Self::Factorial(_) | Self::Percent(_) => Some(OperatorMetadata::new(
                OperatorArity::Exact(1),
                Associativity::None,
                PrecedenceClass::Primary,
            )),
            Self::FunctionCall { .. } => Some(OperatorMetadata::new(
                OperatorArity::Any,
                Associativity::None,
                PrecedenceClass::Primary,
            )),
            Self::BitwiseAnd(_) => Some(OperatorMetadata::new(
                OperatorArity::AtLeast(2),
                Associativity::Associative,
                PrecedenceClass::BitwiseAnd,
            )),
            Self::BitwiseOr(_) => Some(OperatorMetadata::new(
                OperatorArity::AtLeast(2),
                Associativity::Associative,
                PrecedenceClass::BitwiseOr,
            )),
            // Upstream keeps bitwise XOR in the merge/sort paths with AND/OR,
            // while logical XOR is built as a binary transform.
            Self::BitwiseXor(_) => Some(OperatorMetadata::new(
                OperatorArity::AtLeast(2),
                Associativity::Associative,
                PrecedenceClass::BitwiseXor,
            )),
            Self::BitwiseNot(_) => Some(OperatorMetadata::new(
                OperatorArity::Exact(1),
                Associativity::Prefix,
                PrecedenceClass::Prefix,
            )),
            Self::LogicalAnd(_) => Some(OperatorMetadata::new(
                OperatorArity::AtLeast(2),
                Associativity::Associative,
                PrecedenceClass::LogicalAnd,
            )),
            Self::LogicalOr(_) => Some(OperatorMetadata::new(
                OperatorArity::AtLeast(2),
                Associativity::Associative,
                PrecedenceClass::LogicalOr,
            )),
            Self::LogicalXor { .. } => Some(OperatorMetadata::new(
                OperatorArity::Exact(2),
                Associativity::Left,
                PrecedenceClass::LogicalXor,
            )),
            Self::LogicalNot(_) => Some(OperatorMetadata::new(
                OperatorArity::Exact(1),
                Associativity::Prefix,
                PrecedenceClass::Prefix,
            )),
            Self::Comparison { .. } => Some(OperatorMetadata::new(
                OperatorArity::Exact(2),
                Associativity::None,
                PrecedenceClass::Comparison,
            )),
            Self::Number(_)
            | Self::Unit { .. }
            | Self::Symbolic(_)
            | Self::Variable(_)
            | Self::Vector(_)
            | Self::Undefined
            | Self::Aborted
            | Self::DateTime(_) => None,
        }
    }

    /// Returns the number of direct children in upstream child order.
    pub fn child_count(&self) -> usize {
        match self {
            Self::Multiplication(children)
            | Self::Addition(children)
            | Self::BitwiseAnd(children)
            | Self::BitwiseOr(children)
            | Self::BitwiseXor(children)
            | Self::LogicalAnd(children)
            | Self::LogicalOr(children) => children.len(),
            Self::Vector(children) => children.len(),
            Self::FunctionCall { args, .. } => args.len(),
            Self::Inverse(_) | Self::Negate(_) | Self::BitwiseNot(_) | Self::LogicalNot(_) => 1,
            Self::Factorial(_) | Self::Percent(_) => 1,
            Self::Division { .. }
            | Self::Power { .. }
            | Self::Remainder { .. }
            | Self::Modulo { .. }
            | Self::IntegerDivision { .. }
            | Self::ShiftLeft { .. }
            | Self::ShiftRight { .. }
            | Self::LogicalXor { .. }
            | Self::Comparison { .. } => 2,
            Self::Number(_)
            | Self::Unit { .. }
            | Self::Symbolic(_)
            | Self::Variable(_)
            | Self::Undefined
            | Self::Aborted
            | Self::DateTime(_) => 0,
        }
    }

    /// Returns a direct child by zero-based upstream child order.
    pub fn child(&self, index: usize) -> Option<&Expression> {
        match self {
            Self::Multiplication(children)
            | Self::Addition(children)
            | Self::BitwiseAnd(children)
            | Self::BitwiseOr(children)
            | Self::BitwiseXor(children)
            | Self::LogicalAnd(children)
            | Self::LogicalOr(children) => children.get(index),
            Self::Vector(children) => children.get(index),
            Self::FunctionCall { args, .. } => args.get(index),
            Self::Inverse(child)
            | Self::Negate(child)
            | Self::Factorial(child)
            | Self::Percent(child)
            | Self::BitwiseNot(child)
            | Self::LogicalNot(child) => (index == 0).then_some(child.as_ref()),
            Self::Division {
                numerator,
                denominator,
            } => match index {
                0 => Some(numerator.as_ref()),
                1 => Some(denominator.as_ref()),
                _ => None,
            },
            Self::Power { base, exponent } => match index {
                0 => Some(base.as_ref()),
                1 => Some(exponent.as_ref()),
                _ => None,
            },
            Self::Remainder { lhs, rhs }
            | Self::Modulo { lhs, rhs }
            | Self::IntegerDivision { lhs, rhs }
            | Self::ShiftLeft { lhs, rhs }
            | Self::ShiftRight { lhs, rhs } => match index {
                0 => Some(lhs.as_ref()),
                1 => Some(rhs.as_ref()),
                _ => None,
            },
            Self::LogicalXor { lhs, rhs } => match index {
                0 => Some(lhs.as_ref()),
                1 => Some(rhs.as_ref()),
                _ => None,
            },
            Self::Comparison { lhs, rhs, .. } => match index {
                0 => Some(lhs.as_ref()),
                1 => Some(rhs.as_ref()),
                _ => None,
            },
            Self::Number(_)
            | Self::Unit { .. }
            | Self::Symbolic(_)
            | Self::Variable(_)
            | Self::Undefined
            | Self::Aborted
            | Self::DateTime(_) => None,
        }
    }

    /// Views this expression as matrix rows when it is a vector of vector rows.
    pub fn as_matrix_rows(&self) -> Option<Vec<&[Expression]>> {
        let Self::Vector(rows) = self else {
            return None;
        };

        let matrix_rows = rows
            .iter()
            .map(|row| match row {
                Self::Vector(children) => Some(children.as_slice()),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()?;

        validate_matrix_row_slices(&matrix_rows).ok()?;

        Some(matrix_rows)
    }
}

fn validate_matrix_rows(rows: &[Vec<Expression>]) -> Result<(), MatrixShapeError> {
    let Some(first) = rows.first() else {
        return Err(MatrixShapeError::Empty);
    };

    let expected = first.len();
    for (row, children) in rows.iter().enumerate().skip(1) {
        if children.len() != expected {
            return Err(MatrixShapeError::Ragged {
                row,
                expected,
                actual: children.len(),
            });
        }
    }

    Ok(())
}

fn validate_matrix_row_slices(rows: &[&[Expression]]) -> Result<(), MatrixShapeError> {
    let Some(first) = rows.first() else {
        return Err(MatrixShapeError::Empty);
    };

    let expected = first.len();
    for (row, children) in rows.iter().enumerate().skip(1) {
        if children.len() != expected {
            return Err(MatrixShapeError::Ragged {
                row,
                expected,
                actual: children.len(),
            });
        }
    }

    Ok(())
}
