//! Operator parser for qalc expression input.
//!
//! This module parses expression shape into the Rust `MathStructure` AST model.
//! It does not evaluate, simplify, resolve definitions, or format results.
//!
//! Upstream oracle files:
//! - `../libqalculate/libqalculate/Calculator-parse.cc` for qalc operator
//!   precedence, associativity, implicit multiplication, and percent context.
//! - `../libqalculate/tests/operators.batch`,
//!   `../libqalculate/tests/bitwise.batch`, and
//!   `../libqalculate/tests/percentages.batch` for compatibility fixtures.

use crate::{
    ast::{ComparisonOperator, Expression, NaryChildren, Symbol},
    number::Number,
    parser::lexer::{
        lex_expression, BasePrefix, LexErrorKind, NumberLiteralKind, Operator, Span, Token,
        TokenKind,
    },
};
use std::{error::Error, fmt, str::FromStr};

/// Parser error with source span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    kind: ParseErrorKind,
    span: Span,
}

impl ParseError {
    fn new(kind: ParseErrorKind, span: Span) -> Self {
        Self { kind, span }
    }

    /// Returns the parser error category.
    pub fn kind(&self) -> ParseErrorKind {
        self.kind
    }

    /// Returns the source span associated with this error.
    pub fn span(&self) -> Span {
        self.span
    }
}

/// Parser error category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParseErrorKind {
    /// Tokenization failed before parsing could start.
    Lex(LexErrorKind),
    /// Input ended where an expression was required.
    UnexpectedEnd,
    /// A grouping delimiter was not closed.
    UnclosedGroup,
    /// A token appeared where it is not valid.
    UnexpectedToken,
    /// A numeric literal could not be converted into a number leaf.
    InvalidNumber,
    /// The lexer recognizes this operator, but task 3.3 does not parse it.
    UnsupportedOperator(Operator),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} at byte range {}..{}",
            self.kind,
            self.span.start(),
            self.span.end()
        )
    }
}

impl Error for ParseError {}

/// Parses qalc expression text into an AST expression.
pub fn parse_expression(input: &str) -> Result<Expression, ParseError> {
    let tokens = lex_expression(input)
        .map_err(|err| ParseError::new(ParseErrorKind::Lex(err.kind), err.span))?;
    let mut parser = Parser {
        tokens,
        position: 0,
        input_len: input.len(),
    };
    let expression = parser.parse_expression(0)?;
    if let Some(token) = parser.peek() {
        return Err(parser.unexpected_token(token));
    }
    Ok(expression)
}

struct Parser {
    tokens: Vec<Token>,
    position: usize,
    input_len: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InfixOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    Modulo,
    IntegerDivision,
    Power,
    ShiftLeft,
    ShiftRight,
    Comparison(ComparisonOperator),
    BitwiseAnd,
    BitwiseXor,
    BitwiseOr,
    LogicalAnd,
    LogicalOr,
    LogicalNand,
    LogicalNor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Associativity {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NaryOperator {
    Addition,
    Multiplication,
    BitwiseAnd,
    BitwiseXor,
    BitwiseOr,
    LogicalAnd,
    LogicalOr,
}

impl NaryOperator {
    fn build(self, children: NaryChildren) -> Expression {
        match self {
            Self::Addition => Expression::Addition(children),
            Self::Multiplication => Expression::Multiplication(children),
            Self::BitwiseAnd => Expression::BitwiseAnd(children),
            Self::BitwiseXor => Expression::BitwiseXor(children),
            Self::BitwiseOr => Expression::BitwiseOr(children),
            Self::LogicalAnd => Expression::LogicalAnd(children),
            Self::LogicalOr => Expression::LogicalOr(children),
        }
    }
}

impl Parser {
    fn parse_expression(&mut self, minimum_precedence: u8) -> Result<Expression, ParseError> {
        let mut lhs = self.parse_prefix()?;

        loop {
            let Some(token) = self.peek() else {
                break;
            };

            if let TokenKind::Operator(Operator::Factorial) = token.kind {
                if postfix_precedence() < minimum_precedence {
                    break;
                }
                self.advance();
                lhs = Expression::Factorial(Box::new(lhs));
                continue;
            }

            if let TokenKind::Operator(Operator::Percent) = token.kind {
                if self.percent_starts_remainder_rhs() {
                    let (precedence, associativity) = infix_binding_power(InfixOperator::Remainder);
                    if precedence < minimum_precedence {
                        break;
                    }
                    self.advance();
                    let rhs = self.parse_infix_rhs(precedence, associativity)?;
                    lhs = Expression::Remainder {
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                    };
                    continue;
                }

                if postfix_precedence() < minimum_precedence {
                    break;
                }
                self.advance();
                lhs = Expression::Percent(Box::new(lhs));
                continue;
            }

            if let Some(infix) = self.peek_infix_operator()? {
                let (precedence, associativity) = infix_binding_power(infix);
                if precedence < minimum_precedence {
                    break;
                }
                self.advance();
                let rhs = self.parse_infix_rhs(precedence, associativity)?;
                lhs = build_infix_expression(infix, lhs, rhs);
                continue;
            }

            if self.peek().is_some_and(token_starts_primary) {
                let (precedence, associativity) = infix_binding_power(InfixOperator::Multiply);
                if precedence < minimum_precedence {
                    break;
                }
                let rhs = self.parse_infix_rhs(precedence, associativity)?;
                lhs = merge_nary(NaryOperator::Multiplication, lhs, rhs);
                continue;
            }

            break;
        }

        Ok(lhs)
    }

    fn parse_prefix(&mut self) -> Result<Expression, ParseError> {
        let Some(token) = self.advance() else {
            return Err(ParseError::new(
                ParseErrorKind::UnexpectedEnd,
                self.end_span(),
            ));
        };

        match token.kind {
            TokenKind::Number { text, kind } => parse_number_literal(&text, kind)
                .map(Expression::Number)
                .map_err(|_| ParseError::new(ParseErrorKind::InvalidNumber, token.span)),
            TokenKind::Identifier(name) => Ok(Expression::Symbolic(Symbol::new(name))),
            TokenKind::EscapedIdentifier(name) => {
                Ok(Expression::Symbolic(Symbol::new(format!("\\{name}"))))
            }
            TokenKind::OpenParen => {
                let expression = self.parse_expression(0)?;
                match self.peek() {
                    Some(close) if matches!(close.kind, TokenKind::CloseParen) => {
                        self.advance();
                        Ok(expression)
                    }
                    Some(_) | None => {
                        Err(ParseError::new(ParseErrorKind::UnclosedGroup, token.span))
                    }
                }
            }
            TokenKind::Operator(operator) => self.parse_prefix_operator(operator, token.span),
            TokenKind::CloseParen | TokenKind::CloseBracket => {
                Err(ParseError::new(ParseErrorKind::UnexpectedToken, token.span))
            }
            TokenKind::OpenBracket
            | TokenKind::Comma
            | TokenKind::Semicolon
            | TokenKind::Dot
            | TokenKind::Colon
            | TokenKind::Ellipsis
            | TokenKind::StringLiteral(_)
            | TokenKind::Comment(_)
            | TokenKind::CommandPrefix => {
                Err(ParseError::new(ParseErrorKind::UnexpectedToken, token.span))
            }
        }
    }

    fn parse_prefix_operator(
        &mut self,
        operator: Operator,
        span: Span,
    ) -> Result<Expression, ParseError> {
        let operand = match operator {
            Operator::Plus => self.parse_expression(prefix_precedence())?,
            Operator::Minus => {
                return Ok(Expression::Negate(Box::new(
                    self.parse_expression(prefix_precedence())?,
                )));
            }
            Operator::LogicalNot => {
                return Ok(Expression::LogicalNot(Box::new(
                    self.parse_expression(prefix_precedence())?,
                )));
            }
            Operator::BitwiseNot => {
                return Ok(Expression::BitwiseNot(Box::new(
                    self.parse_expression(prefix_precedence())?,
                )));
            }
            _ => {
                return Err(ParseError::new(
                    ParseErrorKind::UnsupportedOperator(operator),
                    span,
                ));
            }
        };
        Ok(operand)
    }

    fn parse_infix_rhs(
        &mut self,
        precedence: u8,
        associativity: Associativity,
    ) -> Result<Expression, ParseError> {
        let next_minimum = match associativity {
            Associativity::Left => precedence + 1,
            Associativity::Right => precedence,
        };
        self.parse_expression(next_minimum)
    }

    fn peek_infix_operator(&self) -> Result<Option<InfixOperator>, ParseError> {
        let Some(token) = self.peek() else {
            return Ok(None);
        };
        let TokenKind::Operator(operator) = token.kind else {
            return Ok(None);
        };

        let infix = match operator {
            Operator::Plus => InfixOperator::Add,
            Operator::Minus => InfixOperator::Subtract,
            Operator::Multiply => InfixOperator::Multiply,
            Operator::Divide => InfixOperator::Divide,
            Operator::Modulo => InfixOperator::Modulo,
            Operator::IntegerDivide => InfixOperator::IntegerDivision,
            Operator::Power => InfixOperator::Power,
            Operator::ShiftLeft => InfixOperator::ShiftLeft,
            Operator::ShiftRight => InfixOperator::ShiftRight,
            Operator::Equal => InfixOperator::Comparison(ComparisonOperator::Equal),
            Operator::Less => InfixOperator::Comparison(ComparisonOperator::Less),
            Operator::Greater => InfixOperator::Comparison(ComparisonOperator::Greater),
            Operator::LessOrEqual => InfixOperator::Comparison(ComparisonOperator::LessOrEqual),
            Operator::GreaterOrEqual => {
                InfixOperator::Comparison(ComparisonOperator::GreaterOrEqual)
            }
            Operator::NotEqual => InfixOperator::Comparison(ComparisonOperator::NotEqual),
            Operator::BitwiseAnd => InfixOperator::BitwiseAnd,
            Operator::BitwiseXor => InfixOperator::BitwiseXor,
            Operator::BitwiseOr => InfixOperator::BitwiseOr,
            Operator::LogicalAnd => InfixOperator::LogicalAnd,
            Operator::LogicalOr => InfixOperator::LogicalOr,
            Operator::LogicalNand => InfixOperator::LogicalNand,
            Operator::LogicalNor => InfixOperator::LogicalNor,
            Operator::Percent | Operator::Factorial => return Ok(None),
            unsupported => {
                return Err(ParseError::new(
                    ParseErrorKind::UnsupportedOperator(unsupported),
                    token.span,
                ));
            }
        };

        Ok(Some(infix))
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    fn advance(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.position)?.clone();
        self.position += 1;
        Some(token)
    }

    fn end_span(&self) -> Span {
        Span::new(self.input_len, self.input_len)
    }

    fn unexpected_token(&self, token: &Token) -> ParseError {
        ParseError::new(ParseErrorKind::UnexpectedToken, token.span)
    }

    fn percent_starts_remainder_rhs(&self) -> bool {
        let next_index = self.position + 1;
        let Some(next) = self.tokens.get(next_index) else {
            return false;
        };

        if token_starts_primary(next) {
            return true;
        }

        if !matches!(
            next.kind,
            TokenKind::Operator(Operator::Plus | Operator::Minus)
        ) {
            return false;
        }

        let primary_index = next_index + 1;
        let Some(primary) = self.tokens.get(primary_index) else {
            return false;
        };
        if !token_starts_primary(primary) {
            return false;
        }

        !self
            .primary_end_index(primary_index)
            .and_then(|end| self.tokens.get(end + 1))
            .is_some_and(|token| matches!(token.kind, TokenKind::Operator(Operator::Percent)))
    }

    fn primary_end_index(&self, index: usize) -> Option<usize> {
        match self.tokens.get(index)?.kind {
            TokenKind::Number { .. }
            | TokenKind::Identifier(_)
            | TokenKind::EscapedIdentifier(_) => Some(index),
            TokenKind::OpenParen => {
                let mut depth = 1usize;
                for (offset, token) in self.tokens[index + 1..].iter().enumerate() {
                    match token.kind {
                        TokenKind::OpenParen => depth += 1,
                        TokenKind::CloseParen => {
                            depth -= 1;
                            if depth == 0 {
                                return Some(index + 1 + offset);
                            }
                        }
                        _ => {}
                    }
                }
                None
            }
            _ => None,
        }
    }
}

fn parse_number_literal(text: &str, kind: NumberLiteralKind) -> Result<Number, String> {
    match kind {
        NumberLiteralKind::BasePrefixed(prefix) => parse_base_prefixed_number(text, prefix)
            .ok_or_else(|| format!("invalid base literal: {text}")),
        NumberLiteralKind::Integer | NumberLiteralKind::Decimal | NumberLiteralKind::Scientific => {
            Number::from_str(text)
        }
    }
}

fn parse_base_prefixed_number(text: &str, prefix: BasePrefix) -> Option<Number> {
    let compact: String = text
        .chars()
        .filter(|ch| !ch.is_ascii_whitespace())
        .collect();
    let digits = compact.get(2..)?;
    let radix = match prefix {
        BasePrefix::Hexadecimal => 16,
        BasePrefix::Binary => 2,
        BasePrefix::Octal => 8,
        BasePrefix::Duodecimal => 12,
    };

    let mut value = rug::Integer::from(0);
    for ch in digits.chars() {
        let digit = match prefix {
            BasePrefix::Duodecimal => duodecimal_digit(ch).or_else(|| ch.to_digit(radix))?,
            BasePrefix::Hexadecimal | BasePrefix::Binary | BasePrefix::Octal => {
                ch.to_digit(radix)?
            }
        };
        if digit >= radix {
            return None;
        }
        value *= radix;
        value += digit;
    }
    Number::from_str(&value.to_string()).ok()
}

fn duodecimal_digit(ch: char) -> Option<u32> {
    match ch {
        'a' | 'A' | 'e' | 'E' => Some(10),
        'b' | 'B' | 'x' | 'X' => Some(11),
        _ => None,
    }
}

fn postfix_precedence() -> u8 {
    // Postfix is checked before binary infix, so sharing power's binding
    // bucket still attaches `!` and postfix `%` before another `^` can bind.
    12
}

fn prefix_precedence() -> u8 {
    11
}

fn infix_binding_power(operator: InfixOperator) -> (u8, Associativity) {
    match operator {
        InfixOperator::LogicalAnd | InfixOperator::LogicalNand => (1, Associativity::Left),
        InfixOperator::LogicalOr | InfixOperator::LogicalNor => (2, Associativity::Left),
        InfixOperator::BitwiseOr => (4, Associativity::Left),
        InfixOperator::BitwiseXor => (5, Associativity::Left),
        InfixOperator::BitwiseAnd => (6, Associativity::Left),
        InfixOperator::Comparison(_) => (7, Associativity::Left),
        InfixOperator::ShiftLeft | InfixOperator::ShiftRight => (8, Associativity::Left),
        InfixOperator::Add | InfixOperator::Subtract => (9, Associativity::Left),
        InfixOperator::Multiply
        | InfixOperator::Divide
        | InfixOperator::Remainder
        | InfixOperator::Modulo
        | InfixOperator::IntegerDivision => (10, Associativity::Left),
        InfixOperator::Power => (12, Associativity::Right),
    }
}

fn build_infix_expression(operator: InfixOperator, lhs: Expression, rhs: Expression) -> Expression {
    match operator {
        InfixOperator::Add => merge_nary(NaryOperator::Addition, lhs, rhs),
        InfixOperator::Subtract => merge_nary(
            NaryOperator::Addition,
            lhs,
            Expression::Negate(Box::new(rhs)),
        ),
        InfixOperator::Multiply => merge_nary(NaryOperator::Multiplication, lhs, rhs),
        InfixOperator::Divide => Expression::Division {
            numerator: Box::new(lhs),
            denominator: Box::new(rhs),
        },
        InfixOperator::Remainder => Expression::Remainder {
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        },
        InfixOperator::Modulo => Expression::Modulo {
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        },
        InfixOperator::IntegerDivision => Expression::IntegerDivision {
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        },
        InfixOperator::Power => Expression::Power {
            base: Box::new(lhs),
            exponent: Box::new(rhs),
        },
        InfixOperator::ShiftLeft => Expression::ShiftLeft {
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        },
        InfixOperator::ShiftRight => Expression::ShiftRight {
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        },
        InfixOperator::Comparison(op) => Expression::Comparison {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        },
        InfixOperator::BitwiseAnd => merge_nary(NaryOperator::BitwiseAnd, lhs, rhs),
        InfixOperator::BitwiseXor => merge_nary(NaryOperator::BitwiseXor, lhs, rhs),
        InfixOperator::BitwiseOr => merge_nary(NaryOperator::BitwiseOr, lhs, rhs),
        InfixOperator::LogicalAnd => merge_nary(NaryOperator::LogicalAnd, lhs, rhs),
        InfixOperator::LogicalOr => merge_nary(NaryOperator::LogicalOr, lhs, rhs),
        InfixOperator::LogicalNand => {
            Expression::LogicalNot(Box::new(merge_nary(NaryOperator::LogicalAnd, lhs, rhs)))
        }
        InfixOperator::LogicalNor => {
            Expression::LogicalNot(Box::new(merge_nary(NaryOperator::LogicalOr, lhs, rhs)))
        }
    }
}

fn merge_nary(operator: NaryOperator, lhs: Expression, rhs: Expression) -> Expression {
    let mut children = match (operator, lhs) {
        (NaryOperator::Addition, Expression::Addition(children))
        | (NaryOperator::Multiplication, Expression::Multiplication(children))
        | (NaryOperator::BitwiseAnd, Expression::BitwiseAnd(children))
        | (NaryOperator::BitwiseXor, Expression::BitwiseXor(children))
        | (NaryOperator::BitwiseOr, Expression::BitwiseOr(children))
        | (NaryOperator::LogicalAnd, Expression::LogicalAnd(children))
        | (NaryOperator::LogicalOr, Expression::LogicalOr(children)) => {
            children.as_slice().to_vec()
        }
        (_, other) => vec![other],
    };
    children.push(rhs);
    operator.build(NaryChildren::new(children).expect("merge always has at least two operands"))
}

fn token_starts_primary(token: &Token) -> bool {
    matches!(
        token.kind,
        TokenKind::Number { .. }
            | TokenKind::Identifier(_)
            | TokenKind::EscapedIdentifier(_)
            | TokenKind::OpenParen
    )
}
