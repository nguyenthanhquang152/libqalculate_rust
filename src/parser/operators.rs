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
    ast::{ComparisonOperator, Expression, FunctionRef, NaryChildren, Symbol},
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
    // Filter out comment tokens — qalc treats `# ...` as trailing comments.
    let tokens: Vec<Token> = tokens
        .into_iter()
        .filter(|t| !matches!(t.kind, TokenKind::Comment(_)))
        .collect();

    // Pass 1: Parse with no parallel spans. Detect where parallel sums occur.
    let mut parser = Parser {
        tokens: tokens.clone(),
        position: 0,
        input_len: input.len(),
        parallel_spans: std::collections::HashSet::new(),
        detected_parallel_spans: std::collections::HashSet::new(),
    };
    let expression = parser.parse_expression(0)?;
    if let Some(token) = parser.peek() {
        return Err(parser.unexpected_token(token));
    }

    if !parser.detected_parallel_spans.is_empty() {
        // Pass 2: Parse again, treating only the detected spans as Parallel.
        let mut parser2 = Parser {
            tokens,
            position: 0,
            input_len: input.len(),
            parallel_spans: parser.detected_parallel_spans,
            detected_parallel_spans: std::collections::HashSet::new(),
        };
        let expression = parser2.parse_expression(0)?;
        if let Some(token) = parser2.peek() {
            return Err(parser2.unexpected_token(token));
        }
        Ok(expression)
    } else {
        Ok(expression)
    }
}

fn is_parallel_unit_expression(expr: &Expression) -> bool {
    match expr {
        Expression::Symbolic(symbol) => is_unit_symbol_name(symbol.name()),
        Expression::Comparison { .. }
        | Expression::LogicalAnd(_)
        | Expression::LogicalOr(_)
        | Expression::LogicalXor { .. }
        | Expression::LogicalNot(_) => false,
        _ => {
            let count = expr.child_count();
            for i in 0..count {
                if let Some(child) = expr.child(i) {
                    if is_parallel_unit_expression(child) {
                        return true;
                    }
                }
            }
            false
        }
    }
}

fn is_unit_symbol_name(name: &str) -> bool {
    // List of base unit symbol/name suffixes.
    // If a name matches one of these directly, or ends with one of these
    // preceded by a valid SI prefix, it is treated as a unit.
    const UNITS: &[&str] = &[
        "Ω",
        "ohm",
        "ohms",
        "m",
        "meter",
        "meters",
        "metre",
        "metres",
        "g",
        "gram",
        "grams",
        "s",
        "second",
        "seconds",
        "sec",
        "secs",
        "A",
        "ampere",
        "amperes",
        "amp",
        "amps",
        "K",
        "kelvin",
        "kelvins",
        "mol",
        "mole",
        "moles",
        "cd",
        "candela",
        "candelas",
        "Hz",
        "hertz",
        "N",
        "newton",
        "newtons",
        "Pa",
        "pascal",
        "pascals",
        "J",
        "joule",
        "joules",
        "W",
        "watt",
        "watts",
        "C",
        "coulomb",
        "coulombs",
        "V",
        "volt",
        "volts",
        "F",
        "farad",
        "farads",
        "S",
        "siemens",
        "Wb",
        "weber",
        "webers",
        "T",
        "tesla",
        "teslas",
        "H",
        "henry",
        "henries",
        "lm",
        "lumen",
        "lumens",
        "lx",
        "lux",
        "Bq",
        "becquerel",
        "becquerels",
        "Gy",
        "gray",
        "grays",
        "Sv",
        "sievert",
        "sieverts",
        "kat",
        "katal",
        "katals",
        "L",
        "l",
        "liter",
        "liters",
        "litre",
        "litres",
        "min",
        "minute",
        "minutes",
        "h",
        "hr",
        "hrs",
        "hour",
        "hours",
        "d",
        "day",
        "days",
        "t",
        "tonne",
        "tonnes",
        "ton",
        "tons",
        "au",
        "astronomical_unit",
        "pc",
        "parsec",
        "parsecs",
        "ly",
        "light_year",
        "light_years",
        "eV",
        "electronvolt",
        "electronvolts",
        "Da",
        "dalton",
        "daltons",
        "bar",
        "bars",
        "atm",
        "atmosphere",
        "atmospheres",
        "cal",
        "calorie",
        "calories",
        "deg",
        "degree",
        "degrees",
        "rad",
        "radian",
        "radians",
        "sr",
        "steradian",
        "steradians",
        "ft",
        "foot",
        "feet",
        "in",
        "inch",
        "inches",
        "yd",
        "yard",
        "yards",
        "mi",
        "mile",
        "miles",
        "lb",
        "pound",
        "pounds",
        "oz",
        "ounce",
        "ounces",
        "gal",
        "gallon",
        "gallons",
        "qt",
        "quart",
        "quarts",
        "pt",
        "pint",
        "pints",
        "cup",
        "cups",
        "floz",
        "fl_oz",
        "tbsp",
        "tablespoon",
        "tablespoons",
        "tsp",
        "teaspoon",
        "teaspoons",
        "psi",
        "hp",
        "horsepower",
        "B",
        "byte",
        "bytes",
        "bit",
        "bits",
    ];

    if UNITS.contains(&name) {
        return true;
    }

    // Check SI prefixes
    const SI_PREFIXES: &[&str] = &[
        "y", "z", "a", "f", "p", "n", "μ", "u", "m", "c", "d", "da", "h", "k", "M", "G", "T", "P",
        "E", "Z", "Y",
    ];

    for prefix in SI_PREFIXES {
        if let Some(rest) = name.strip_prefix(prefix) {
            if UNITS.contains(&rest) {
                return true;
            }
        }
    }

    false
}

struct Parser {
    tokens: Vec<Token>,
    position: usize,
    input_len: usize,
    parallel_spans: std::collections::HashSet<Span>,
    detected_parallel_spans: std::collections::HashSet<Span>,
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
    LogicalXor,
    LogicalNand,
    LogicalNor,
    /// Parallel sum: `a || b` when units are involved.
    /// Deferred to later unit resolution; falls back to logical OR
    /// only when no units are present.
    Parallel,
    ParallelOr,
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
                // Count consecutive `!` tokens for multifactorial.
                // Upstream: 1 `!` = factorial, 2 `!!` = double factorial,
                // 3+ `!!!...` = multifactorial(n, count).
                let mut factorial_count = 0u32;
                while self
                    .peek()
                    .is_some_and(|t| matches!(t.kind, TokenKind::Operator(Operator::Factorial)))
                {
                    self.advance();
                    factorial_count += 1;
                }
                lhs = match factorial_count {
                    1 => Expression::Factorial(Box::new(lhs)),
                    2 => Expression::DoubleFactorial(Box::new(lhs)),
                    n => Expression::MultiFactorial {
                        expr: Box::new(lhs),
                        count: n,
                    },
                };
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
                let token = self.peek().cloned().unwrap();
                self.advance();
                let rhs = self.parse_infix_rhs(precedence, associativity)?;
                if infix == InfixOperator::ParallelOr
                    && is_parallel_unit_expression(&lhs)
                    && is_parallel_unit_expression(&rhs)
                {
                    self.detected_parallel_spans.insert(token.span);
                }
                lhs = build_infix_expression(infix, lhs, rhs);
                continue;
            }

            // Standalone `E` operator: `5 E 3` → `5 × 10^3`.
            // Qalculate treats uppercase `E` between expressions as
            // multiplication by a power of ten. Lowercase `e` remains
            // an identifier (Euler's number / variable name).
            if let Some(token) = self.peek() {
                if matches!(token.kind, TokenKind::Identifier(ref name) if name == "E") {
                    // The E ten-power operator binds tighter than exponentiation,
                    // so `2E3^2` is `(2E3)^2`, not `2 * 10^(3^2)`.
                    let bp = e_operator_precedence();
                    if bp >= minimum_precedence {
                        self.advance(); // consume the `E`
                        let rhs = self.parse_infix_rhs(bp, Associativity::Left)?;
                        let ten_power = Expression::Power {
                            base: Box::new(Expression::Number(
                                Number::from_str("10").expect("10 is valid"),
                            )),
                            exponent: Box::new(rhs),
                        };
                        lhs = merge_nary(NaryOperator::Multiplication, lhs, ten_power);
                        continue;
                    }
                }
            }

            if let Some((precedence, associativity)) = self.peek_implicit_multiplication() {
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
            TokenKind::Identifier(name) => {
                if self.peek().is_some_and(|t| {
                    matches!(t.kind, TokenKind::OpenParen) && token.span.end() == t.span.start()
                }) {
                    self.advance(); // consume OpenParen
                    let args = self.parse_function_arguments()?;
                    Ok(Expression::FunctionCall {
                        function: FunctionRef::new(name),
                        args,
                    })
                } else {
                    Ok(Expression::Symbolic(Symbol::new(name)))
                }
            }
            TokenKind::EscapedIdentifier(name) => {
                let escaped_name = format!("\\{name}");
                if self.peek().is_some_and(|t| {
                    matches!(t.kind, TokenKind::OpenParen) && token.span.end() == t.span.start()
                }) {
                    self.advance(); // consume OpenParen
                    let args = self.parse_function_arguments()?;
                    Ok(Expression::FunctionCall {
                        function: FunctionRef::new(escaped_name),
                        args,
                    })
                } else {
                    Ok(Expression::Symbolic(Symbol::new(escaped_name)))
                }
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

    fn parse_function_arguments(&mut self) -> Result<Vec<Expression>, ParseError> {
        let mut args = Vec::new();
        if let Some(token) = self.peek() {
            if matches!(token.kind, TokenKind::CloseParen) {
                self.advance();
                return Ok(args);
            }
        }
        loop {
            let arg = self.parse_expression(0)?;
            args.push(arg);
            let next = self
                .advance()
                .ok_or_else(|| ParseError::new(ParseErrorKind::UnexpectedEnd, self.end_span()))?;
            match next.kind {
                TokenKind::CloseParen => break,
                TokenKind::Comma | TokenKind::Semicolon => {
                    if let Some(token) = self.peek() {
                        if matches!(token.kind, TokenKind::CloseParen) {
                            self.advance();
                            break;
                        }
                    }
                }
                _ => return Err(self.unexpected_token(&next)),
            }
        }
        Ok(args)
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
            Operator::Remainder => InfixOperator::Remainder,
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
            Operator::LogicalXor => InfixOperator::LogicalXor,
            Operator::LogicalNand => InfixOperator::LogicalNand,
            Operator::LogicalNor => InfixOperator::LogicalNor,
            // `||` is lexed as Parallel; preserve the parallel-sum
            // semantics for unit expressions like `10 Ω || 6 Ω`.
            // Logical-OR fallback is deferred to later unit resolution.
            Operator::Parallel => InfixOperator::Parallel,
            Operator::ParallelOr => {
                if self.parallel_spans.contains(&token.span) {
                    InfixOperator::Parallel
                } else {
                    InfixOperator::ParallelOr
                }
            }
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

    fn peek_implicit_multiplication(&self) -> Option<(u8, Associativity)> {
        let token = self.peek()?;
        if !token_starts_primary(token) {
            return None;
        }

        let is_adjacent = self
            .tokens
            .get(self.position.checked_sub(1)?)
            .is_some_and(|previous| previous.span.end() == token.span.start());
        Some((
            if is_adjacent {
                tight_implicit_multiplication_precedence()
            } else {
                infix_binding_power(InfixOperator::Multiply).0
            },
            Associativity::Left,
        ))
    }

    fn percent_starts_remainder_rhs(&self) -> bool {
        let percent_token = self.tokens.get(self.position).expect("called on %");
        let next_index = self.position + 1;
        let Some(next) = self.tokens.get(next_index) else {
            return false;
        };

        let adjacent = percent_token.span.end() == next.span.start();

        if token_starts_primary(next) {
            // Both `6%2` (adjacent) and `6 % 2` (spaced) are remainder.
            // Qalculate treats `%` followed by a primary operand as
            // remainder regardless of whitespace.
            return true;
        }

        if !matches!(
            next.kind,
            TokenKind::Operator(Operator::Plus | Operator::Minus)
        ) {
            return false;
        }

        // `%` followed by `+`/`-` — only treat as remainder if adjacent or if % is not attached to the left operand,
        // so `10% + 100` stays as postfix percent plus 100, but `6 % -2` becomes remainder.
        let prev_adjacent = if self.position > 0 {
            self.tokens
                .get(self.position - 1)
                .map(|prev| prev.span.end() == percent_token.span.start())
                .unwrap_or(false)
        } else {
            false
        };

        if prev_adjacent && !adjacent {
            return false;
        }

        let primary_index = next_index + 1;
        let Some(primary) = self.tokens.get(primary_index) else {
            return false;
        };
        if !token_starts_primary(primary) {
            return false;
        }

        // Adjacent `%` + sign + primary, but if the primary itself ends with
        // `%` (as in `10%-6%`), this is percentage subtraction, not remainder.
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
            let compact: String = text
                .chars()
                .filter(|ch| !ch.is_ascii_whitespace())
                .collect();
            Number::from_str(&compact)
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
    15
}

fn prefix_precedence() -> u8 {
    15
}

fn tight_implicit_multiplication_precedence() -> u8 {
    14
}

/// The standalone `E` ten-power operator binds tighter than exponentiation
/// so `2E3^2` is `(2E3)^2`. Placed above Power (15) in the precedence table.
fn e_operator_precedence() -> u8 {
    16
}

fn infix_binding_power(operator: InfixOperator) -> (u8, Associativity) {
    // Qalculate precedence (https://qalculate.github.io/manual/qalculate-expressions.html):
    // Logical XOR (loosest) < OR < NOR < NAND < AND (tightest among logicals)
    // Then: bitwise OR < XOR < AND < comparison < shift < add < parallel < mul < power
    match operator {
        InfixOperator::LogicalXor => (1, Associativity::Left),
        InfixOperator::LogicalOr => (2, Associativity::Left),
        InfixOperator::LogicalNor => (3, Associativity::Left),
        InfixOperator::LogicalNand => (4, Associativity::Left),
        InfixOperator::LogicalAnd => (5, Associativity::Left),
        InfixOperator::BitwiseOr => (6, Associativity::Left),
        InfixOperator::BitwiseXor => (7, Associativity::Left),
        InfixOperator::BitwiseAnd => (8, Associativity::Left),
        InfixOperator::Comparison(_) => (9, Associativity::Left),
        InfixOperator::ShiftLeft | InfixOperator::ShiftRight => (10, Associativity::Left),
        InfixOperator::Add | InfixOperator::Subtract => (11, Associativity::Left),
        InfixOperator::Parallel => (12, Associativity::Left),
        InfixOperator::ParallelOr => (2, Associativity::Left),
        InfixOperator::Multiply
        | InfixOperator::Divide
        | InfixOperator::Remainder
        | InfixOperator::Modulo
        | InfixOperator::IntegerDivision => (13, Associativity::Left),
        InfixOperator::Power => (15, Associativity::Right),
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
        InfixOperator::LogicalXor => Expression::LogicalXor {
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        },
        InfixOperator::LogicalNand => {
            Expression::LogicalNot(Box::new(merge_nary(NaryOperator::LogicalAnd, lhs, rhs)))
        }
        InfixOperator::LogicalNor => {
            Expression::LogicalNot(Box::new(merge_nary(NaryOperator::LogicalOr, lhs, rhs)))
        }
        InfixOperator::Parallel => Expression::Parallel {
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        },
        InfixOperator::ParallelOr => merge_nary(NaryOperator::LogicalOr, lhs, rhs),
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
