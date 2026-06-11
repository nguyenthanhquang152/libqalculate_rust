//! Lexer for qalc expression and session-command input.
//!
//! This module only tokenizes source text. It does not parse expressions,
//! resolve names, evaluate values, or format output.
//! Active-input-base interpretation remains deferred to the parser/session
//! layers; this lexer keeps source spans and token fragments for those layers.
//!
//! Upstream oracle files:
//! - `../libqalculate/libqalculate/Calculator-parse.cc` for lexical operator,
//!   command, string, and name boundaries.
//! - `../libqalculate/libqalculate/Number.cc` for numeric literal seeds.
//! - `../libqalculate/libqalculate/util.cc` for string/Unicode handling.

use std::{error::Error, fmt, ops::Range};

/// Byte span for a token or lexical error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    start: usize,
    end: usize,
}

impl Span {
    /// Creates a byte span.
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Returns the start byte offset.
    pub fn start(&self) -> usize {
        self.start
    }

    /// Returns the end byte offset.
    pub fn end(&self) -> usize {
        self.end
    }

    /// Returns this span as a Rust range.
    pub fn range(&self) -> Range<usize> {
        self.start..self.end
    }
}

/// Whether a line is an expression or a qalc session command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineKind {
    /// Ordinary expression input.
    Expression,
    /// Session command input, such as `/set unicode 1`.
    Command,
}

/// Tokenized input line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexedLine {
    /// Line classification.
    pub kind: LineKind,
    /// Original line content.
    pub source: String,
    /// Tokens retained from the line.
    pub tokens: Vec<Token>,
}

/// Token with byte span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// Token category and payload.
    pub kind: TokenKind,
    /// Byte span within the source string.
    pub span: Span,
}

impl Token {
    fn new(kind: TokenKind, start: usize, end: usize) -> Self {
        Self {
            kind,
            span: Span::new(start, end),
        }
    }
}

/// Token category emitted by the lexer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    /// Numeric literal with original text and coarse lexical category.
    Number {
        /// Source text for the literal.
        text: String,
        /// Numeric literal category.
        kind: NumberLiteralKind,
    },
    /// Name, symbol, unit-like text, or unresolved identifier.
    Identifier(String),
    /// Quoted string literal content without surrounding quotes.
    StringLiteral(String),
    /// Operator token.
    Operator(Operator),
    /// `(`.
    OpenParen,
    /// `)`.
    CloseParen,
    /// `[`.
    OpenBracket,
    /// `]`.
    CloseBracket,
    /// `,`.
    Comma,
    /// `;`.
    Semicolon,
    /// `.`.
    Dot,
    /// `:`.
    Colon,
    /// `...`.
    Ellipsis,
    /// Comment text after `#`, excluding the `#`.
    Comment(String),
    /// Leading `/` that marks a slash-prefixed session command.
    CommandPrefix,
    /// Escaped unknown/function-argument name, such as `\x`.
    EscapedIdentifier(String),
}

/// Coarse numeric literal category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NumberLiteralKind {
    /// Decimal-free integer.
    Integer,
    /// Decimal literal without exponent.
    Decimal,
    /// Decimal/scientific literal with exponent marker.
    Scientific,
    /// Literal with a base prefix.
    BasePrefixed(BasePrefix),
}

/// Base prefix kind for prefixed numeric literals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BasePrefix {
    /// `0x` or `0X`.
    Hexadecimal,
    /// `0b` or `0B`.
    Binary,
    /// `0o` or `0O`.
    Octal,
    /// `0d` or `0D`.
    Duodecimal,
}

/// Operators recognized by the lexer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Operator {
    /// Addition.
    Plus,
    /// Subtraction or unary negation.
    Minus,
    /// Multiplication.
    Multiply,
    /// Division.
    Divide,
    /// Unit or expression conversion, written `to`, `->`, or `→`.
    Conversion,
    /// Exponentiation.
    Power,
    /// Remainder or percent-like operator depending on parser context.
    Percent,
    /// Modulo operator, written `%%` or `mod`.
    Modulo,
    /// Integer division, written `//`, `\`, or `div`.
    IntegerDivide,
    /// Factorial.
    Factorial,
    /// Bitwise left shift.
    ShiftLeft,
    /// Bitwise right shift.
    ShiftRight,
    /// Equality.
    Equal,
    /// Less-than.
    Less,
    /// Greater-than.
    Greater,
    /// Less-than-or-equal.
    LessOrEqual,
    /// Greater-than-or-equal.
    GreaterOrEqual,
    /// Not-equal.
    NotEqual,
    /// Logical and.
    LogicalAnd,
    /// Logical or.
    LogicalOr,
    /// Logical xor.
    LogicalXor,
    /// Logical nand.
    LogicalNand,
    /// Logical nor.
    LogicalNor,
    /// Logical not.
    LogicalNot,
    /// Bitwise and.
    BitwiseAnd,
    /// Bitwise or.
    BitwiseOr,
    /// Bitwise xor.
    BitwiseXor,
    /// Bitwise not.
    BitwiseNot,
    /// Parallel sum operator, written `||` or `∥`.
    Parallel,
    /// Set union.
    SetUnion,
    /// Set intersection.
    SetIntersection,
    /// Set difference.
    SetDifference,
    /// Set symmetric difference.
    SetSymmetricDifference,
    /// Set membership.
    SetMembership,
    /// Set non-membership.
    SetNotMembership,
    /// Set contains.
    SetContains,
    /// Set does not contain.
    SetNotContains,
    /// Proper subset.
    ProperSubset,
    /// Subset.
    Subset,
    /// Proper superset.
    ProperSuperset,
    /// Superset.
    Superset,
    /// Variable assignment, written `:=` or `=:`.
    Assignment,
    /// Uncertainty plus/minus token, written `+/-` or `±`.
    Uncertainty,
}

/// Lexical error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    /// Error category.
    pub kind: LexErrorKind,
    /// Byte span where the error was detected.
    pub span: Span,
}

impl LexError {
    fn new(kind: LexErrorKind, start: usize, end: usize) -> Self {
        Self {
            kind,
            span: Span::new(start, end),
        }
    }
}

/// Lexical error category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LexErrorKind {
    /// Source contains an interior NUL byte.
    InteriorNul,
    /// A string literal reaches end-of-line before its closing quote.
    UnterminatedString,
    /// A character was not recognized by the lexer.
    UnexpectedCharacter(char),
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            LexErrorKind::InteriorNul => write!(
                f,
                "interior NUL byte at byte range {}..{}",
                self.span.start(),
                self.span.end()
            ),
            LexErrorKind::UnterminatedString => write!(
                f,
                "unterminated string starting at byte {}",
                self.span.start()
            ),
            LexErrorKind::UnexpectedCharacter(ch) => write!(
                f,
                "unexpected character {ch:?} at byte range {}..{}",
                self.span.start(),
                self.span.end()
            ),
        }
    }
}

impl Error for LexError {}

/// Tokenizes one line and classifies it as expression or session command.
pub fn lex_line(input: &str) -> Result<LexedLine, LexError> {
    reject_nul(input)?;
    let kind = classify_line(input);
    let tokens = match kind {
        LineKind::Expression => lex_expression_after_preflight(input)?,
        LineKind::Command => lex_command_after_preflight(input)?,
    };
    Ok(LexedLine {
        kind,
        source: input.to_string(),
        tokens,
    })
}

/// Tokenizes expression text.
pub fn lex_expression(input: &str) -> Result<Vec<Token>, LexError> {
    reject_nul(input)?;
    lex_expression_after_preflight(input)
}

fn lex_expression_after_preflight(input: &str) -> Result<Vec<Token>, LexError> {
    let mut lexer = Lexer::expression(input);
    lexer.lex()
}

/// Tokenizes session command text without applying expression-only command
/// prefix semantics to the leading slash.
pub fn lex_command(input: &str) -> Result<Vec<Token>, LexError> {
    reject_nul(input)?;
    lex_command_after_preflight(input)
}

fn lex_command_after_preflight(input: &str) -> Result<Vec<Token>, LexError> {
    let mut lexer = Lexer::command(input);
    lexer.lex()
}

struct Lexer<'a> {
    input: &'a str,
    index: usize,
    mode: LexerMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LexerMode {
    Expression,
    Command,
}

impl Lexer<'_> {
    fn expression(input: &str) -> Lexer<'_> {
        Lexer {
            input,
            index: 0,
            mode: LexerMode::Expression,
        }
    }

    fn command(input: &str) -> Lexer<'_> {
        Lexer {
            input,
            index: 0,
            mode: LexerMode::Command,
        }
    }

    fn lex(&mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();
        while let Some((start, ch)) = self.peek_char() {
            if ch == '\0' {
                return Err(LexError::new(LexErrorKind::InteriorNul, start, start + 1));
            }
            if ch.is_whitespace() {
                self.bump_char();
                continue;
            }
            if self.mode == LexerMode::Command && tokens.is_empty() && ch == '/' {
                self.bump_char();
                tokens.push(Token::new(TokenKind::CommandPrefix, start, self.index));
                continue;
            }
            if ch == '#' {
                self.bump_char();
                tokens.push(Token::new(
                    TokenKind::Comment(self.input[self.index..].to_string()),
                    start,
                    self.input.len(),
                ));
                self.index = self.input.len();
                break;
            }
            if ch == '"' || ch == '\'' {
                tokens.push(self.lex_string(start, ch)?);
                continue;
            }
            if ch.is_ascii_digit() || starts_decimal_literal(self.input, self.index) {
                tokens.push(self.lex_number(start));
                continue;
            }
            if ch == '\\' && self.escaped_identifier_follows() {
                tokens.push(self.lex_escaped_identifier(start));
                continue;
            }
            if let Some(token) = self.lex_punctuation_or_symbol(start, ch, tokens.last()) {
                tokens.push(token);
                continue;
            }
            if is_identifier_start(ch) {
                tokens.push(self.lex_word_or_identifier(start));
                continue;
            }

            let end = start + ch.len_utf8();
            return Err(LexError::new(
                LexErrorKind::UnexpectedCharacter(ch),
                start,
                end,
            ));
        }

        Ok(tokens)
    }

    fn lex_string(&mut self, start: usize, quote: char) -> Result<Token, LexError> {
        self.bump_char();
        let mut value = String::new();
        while let Some((index, ch)) = self.peek_char() {
            match ch {
                '\0' => return Err(LexError::new(LexErrorKind::InteriorNul, index, index + 1)),
                ch if ch == quote => {
                    self.bump_char();
                    return Ok(Token::new(
                        TokenKind::StringLiteral(value),
                        start,
                        self.index,
                    ));
                }
                '\\' => {
                    self.bump_char();
                    if let Some((escaped_index, escaped)) = self.peek_char() {
                        if escaped == '\0' {
                            return Err(LexError::new(
                                LexErrorKind::InteriorNul,
                                escaped_index,
                                escaped_index + 1,
                            ));
                        }
                        value.push(escaped);
                        self.bump_char();
                    } else {
                        value.push('\\');
                    }
                }
                _ => {
                    value.push(ch);
                    self.bump_char();
                }
            }
        }

        Err(LexError::new(
            LexErrorKind::UnterminatedString,
            start,
            self.input.len(),
        ))
    }

    fn lex_number(&mut self, start: usize) -> Token {
        if prefixed_number_starts(self.remaining(), "0x", is_hex_digit)
            || prefixed_number_starts(self.remaining(), "0X", is_hex_digit)
        {
            self.consume_prefixed_digits(is_hex_digit);
            return Token::new(
                TokenKind::Number {
                    text: self.input[start..self.index].to_string(),
                    kind: NumberLiteralKind::BasePrefixed(BasePrefix::Hexadecimal),
                },
                start,
                self.index,
            );
        }
        if prefixed_number_starts(self.remaining(), "0b", is_binary_digit)
            || prefixed_number_starts(self.remaining(), "0B", is_binary_digit)
        {
            self.consume_prefixed_digits(is_binary_digit);
            return Token::new(
                TokenKind::Number {
                    text: self.input[start..self.index].to_string(),
                    kind: NumberLiteralKind::BasePrefixed(BasePrefix::Binary),
                },
                start,
                self.index,
            );
        }
        if prefixed_number_starts(self.remaining(), "0o", is_octal_digit)
            || prefixed_number_starts(self.remaining(), "0O", is_octal_digit)
        {
            self.consume_prefixed_digits(is_octal_digit);
            return Token::new(
                TokenKind::Number {
                    text: self.input[start..self.index].to_string(),
                    kind: NumberLiteralKind::BasePrefixed(BasePrefix::Octal),
                },
                start,
                self.index,
            );
        }
        if prefixed_number_starts(self.remaining(), "0d", is_duodecimal_digit)
            || prefixed_number_starts(self.remaining(), "0D", is_duodecimal_digit)
        {
            self.consume_prefixed_digits(is_duodecimal_digit);
            return Token::new(
                TokenKind::Number {
                    text: self.input[start..self.index].to_string(),
                    kind: NumberLiteralKind::BasePrefixed(BasePrefix::Duodecimal),
                },
                start,
                self.index,
            );
        }

        let mut saw_dot = false;
        let mut saw_exp = false;

        self.consume_grouped_digits();
        if !self.remaining().starts_with("...") && self.consume_spaces_before('.') {
            saw_dot = true;
            self.bump_char();
            self.consume_grouped_digits();
        }
        if matches!(self.peek_char().map(|(_, ch)| ch), Some('e' | 'E')) {
            let save = self.index;
            self.bump_char();
            if matches!(self.peek_char().map(|(_, ch)| ch), Some('+' | '-')) {
                self.bump_char();
            }
            let before_digits = self.index;
            self.consume_grouped_digits();
            if self.index > before_digits {
                saw_exp = true;
            } else {
                self.index = save;
            }
        }

        let kind = if saw_exp {
            NumberLiteralKind::Scientific
        } else if saw_dot {
            NumberLiteralKind::Decimal
        } else {
            NumberLiteralKind::Integer
        };

        Token::new(
            TokenKind::Number {
                text: self.input[start..self.index].to_string(),
                kind,
            },
            start,
            self.index,
        )
    }

    fn lex_punctuation_or_symbol(
        &mut self,
        start: usize,
        ch: char,
        previous: Option<&Token>,
    ) -> Option<Token> {
        let rest = self.remaining();
        let (kind, width) = if rest.starts_with("+/-") {
            (TokenKind::Operator(Operator::Uncertainty), 3)
        } else if rest.starts_with("...") {
            (TokenKind::Ellipsis, 3)
        } else if rest.starts_with("->") {
            (TokenKind::Operator(Operator::Conversion), 2)
        } else if rest.starts_with("<<") {
            (TokenKind::Operator(Operator::ShiftLeft), 2)
        } else if rest.starts_with(">>") {
            (TokenKind::Operator(Operator::ShiftRight), 2)
        } else if rest.starts_with("%%") {
            (TokenKind::Operator(Operator::Modulo), 2)
        } else if rest.starts_with("//") {
            (TokenKind::Operator(Operator::IntegerDivide), 2)
        } else if rest.starts_with("<=") {
            (TokenKind::Operator(Operator::LessOrEqual), 2)
        } else if rest.starts_with(">=") {
            (TokenKind::Operator(Operator::GreaterOrEqual), 2)
        } else if rest.starts_with("!=") {
            (TokenKind::Operator(Operator::NotEqual), 2)
        } else if rest.starts_with("≠") {
            (
                TokenKind::Operator(Operator::NotEqual),
                rest_char_width(rest),
            )
        } else if rest.starts_with(":=") || rest.starts_with("=:") {
            (TokenKind::Operator(Operator::Assignment), 2)
        } else if rest.starts_with("==") {
            (TokenKind::Operator(Operator::Equal), 2)
        } else if rest.starts_with("&&") {
            (TokenKind::Operator(Operator::LogicalAnd), 2)
        } else if rest.starts_with("||") {
            (TokenKind::Operator(Operator::Parallel), 2)
        } else if rest.starts_with("^^") {
            (TokenKind::Operator(Operator::BitwiseXor), 2)
        } else if rest.starts_with("**") {
            (TokenKind::Operator(Operator::Power), 2)
        } else {
            match ch {
                '+' => (TokenKind::Operator(Operator::Plus), 1),
                '-' | '−' => (TokenKind::Operator(Operator::Minus), ch.len_utf8()),
                '*' | '×' | '·' | '⋅' => {
                    (TokenKind::Operator(Operator::Multiply), ch.len_utf8())
                }
                '/' | '÷' | '∕' => (TokenKind::Operator(Operator::Divide), ch.len_utf8()),
                '\\' => (TokenKind::Operator(Operator::IntegerDivide), 1),
                '^' => (TokenKind::Operator(Operator::Power), 1),
                '%' => (TokenKind::Operator(Operator::Percent), 1),
                '!' if is_prefix_operator_position(previous) => {
                    (TokenKind::Operator(Operator::LogicalNot), 1)
                }
                '!' => (TokenKind::Operator(Operator::Factorial), 1),
                '=' => (TokenKind::Operator(Operator::Equal), 1),
                '<' => (TokenKind::Operator(Operator::Less), 1),
                '>' => (TokenKind::Operator(Operator::Greater), 1),
                '→' => (TokenKind::Operator(Operator::Conversion), ch.len_utf8()),
                '≤' => (TokenKind::Operator(Operator::LessOrEqual), ch.len_utf8()),
                '≥' => (TokenKind::Operator(Operator::GreaterOrEqual), ch.len_utf8()),
                '±' => (TokenKind::Operator(Operator::Uncertainty), ch.len_utf8()),
                '&' | '∧' => (TokenKind::Operator(Operator::BitwiseAnd), ch.len_utf8()),
                '|' | '∨' => (TokenKind::Operator(Operator::BitwiseOr), ch.len_utf8()),
                '⊻' => (TokenKind::Operator(Operator::BitwiseXor), ch.len_utf8()),
                '~' => (TokenKind::Operator(Operator::BitwiseNot), 1),
                '¬' => (TokenKind::Operator(Operator::LogicalNot), ch.len_utf8()),
                '∥' => (TokenKind::Operator(Operator::Parallel), ch.len_utf8()),
                '∪' => (TokenKind::Operator(Operator::SetUnion), ch.len_utf8()),
                '∩' => (
                    TokenKind::Operator(Operator::SetIntersection),
                    ch.len_utf8(),
                ),
                '∖' => (TokenKind::Operator(Operator::SetDifference), ch.len_utf8()),
                '⊖' => (
                    TokenKind::Operator(Operator::SetSymmetricDifference),
                    ch.len_utf8(),
                ),
                '∈' => (TokenKind::Operator(Operator::SetMembership), ch.len_utf8()),
                '∉' => (
                    TokenKind::Operator(Operator::SetNotMembership),
                    ch.len_utf8(),
                ),
                '∋' => (TokenKind::Operator(Operator::SetContains), ch.len_utf8()),
                '∌' => (TokenKind::Operator(Operator::SetNotContains), ch.len_utf8()),
                '⊊' => (TokenKind::Operator(Operator::ProperSubset), ch.len_utf8()),
                '⊆' => (TokenKind::Operator(Operator::Subset), ch.len_utf8()),
                '⊋' => (TokenKind::Operator(Operator::ProperSuperset), ch.len_utf8()),
                '⊇' => (TokenKind::Operator(Operator::Superset), ch.len_utf8()),
                '(' => (TokenKind::OpenParen, 1),
                ')' => (TokenKind::CloseParen, 1),
                '[' => (TokenKind::OpenBracket, 1),
                ']' => (TokenKind::CloseBracket, 1),
                ',' => (TokenKind::Comma, 1),
                ';' => (TokenKind::Semicolon, 1),
                '.' => (TokenKind::Dot, 1),
                ':' => (TokenKind::Colon, 1),
                _ => return None,
            }
        };

        self.index += width;
        Some(Token::new(kind, start, self.index))
    }

    fn lex_word_or_identifier(&mut self, start: usize) -> Token {
        self.bump_char();
        self.consume_while(is_identifier_continue);
        let text = &self.input[start..self.index];
        let lower = text.to_ascii_lowercase();
        if let Some(operator) = word_operator(&lower) {
            Token::new(TokenKind::Operator(operator), start, self.index)
        } else {
            Token::new(TokenKind::Identifier(text.to_string()), start, self.index)
        }
    }

    fn escaped_identifier_follows(&self) -> bool {
        self.remaining()
            .strip_prefix('\\')
            .and_then(|rest| rest.chars().next())
            .is_some_and(is_identifier_start)
    }

    fn lex_escaped_identifier(&mut self, start: usize) -> Token {
        self.bump_char();
        self.bump_char();
        self.consume_while(is_identifier_continue);
        Token::new(
            TokenKind::EscapedIdentifier(self.input[start + 1..self.index].to_string()),
            start,
            self.index,
        )
    }

    fn remaining(&self) -> &str {
        &self.input[self.index..]
    }

    fn peek_char(&self) -> Option<(usize, char)> {
        self.remaining().chars().next().map(|ch| (self.index, ch))
    }

    fn peek_is(&self, expected: char) -> bool {
        self.peek_char().map(|(_, ch)| ch) == Some(expected)
    }

    fn bump_char(&mut self) -> Option<char> {
        let ch = self.remaining().chars().next()?;
        self.index += ch.len_utf8();
        Some(ch)
    }

    fn consume_while(&mut self, mut predicate: impl FnMut(char) -> bool) {
        while let Some((_, ch)) = self.peek_char() {
            if !predicate(ch) {
                break;
            }
            self.bump_char();
        }
    }

    fn consume_grouped_digits(&mut self) {
        self.consume_while(|ch| ch.is_ascii_digit());

        loop {
            let before_spaces = self.index;
            self.consume_while(|ch| ch == ' ');
            if matches!(self.peek_char(), Some((_, ch)) if ch.is_ascii_digit()) {
                self.consume_while(|ch| ch.is_ascii_digit());
            } else {
                self.index = before_spaces;
                break;
            }
        }
    }

    fn consume_spaces_before(&mut self, expected: char) -> bool {
        let before_spaces = self.index;
        self.consume_while(|ch| ch == ' ');
        if self.peek_is(expected) {
            true
        } else {
            self.index = before_spaces;
            false
        }
    }

    fn consume_prefixed_digits(&mut self, is_digit: fn(char) -> bool) {
        self.index += 2;
        self.consume_while(is_digit);

        loop {
            let before_spaces = self.index;
            self.consume_while(|ch| ch == ' ');
            if matches!(self.peek_char(), Some((_, ch)) if is_digit(ch)) {
                self.consume_while(is_digit);
            } else {
                self.index = before_spaces;
                break;
            }
        }
    }
}

fn classify_line(input: &str) -> LineKind {
    let trimmed = input.trim_start();
    if trimmed.starts_with('/') {
        return LineKind::Command;
    }
    let lower_trimmed = trimmed.to_ascii_lowercase();
    if matches!(lower_trimmed.as_str(), "mc" | "ms" | "m+" | "m-")
        || lower_trimmed == "partial fraction"
        || lower_trimmed.starts_with("partial fraction ")
    {
        return LineKind::Command;
    }

    let (first_word, rest) = match trimmed.find(char::is_whitespace) {
        Some(index) => (&trimmed[..index], trimmed[index..].trim()),
        None => (trimmed, ""),
    };
    let first_word = first_word.to_ascii_lowercase();

    match session_command_arity(first_word.as_str()) {
        Some(CommandArity::NoArgs) if rest.is_empty() => LineKind::Command,
        Some(CommandArity::NoArgs) => LineKind::Expression,
        Some(CommandArity::Args | CommandArity::OptionalArgs) => LineKind::Command,
        None => LineKind::Expression,
    }
}

fn reject_nul(input: &str) -> Result<(), LexError> {
    if let Some(index) = input.find('\0') {
        return Err(LexError::new(LexErrorKind::InteriorNul, index, index + 1));
    }
    Ok(())
}

fn starts_decimal_literal(input: &str, index: usize) -> bool {
    let rest = &input[index..];
    if rest.starts_with("...") {
        return false;
    }
    let mut chars = rest.chars();
    if chars.next() != Some('.') {
        return false;
    }
    match chars.find(|ch| *ch != ' ') {
        Some(ch) => ch.is_ascii_digit(),
        None => true,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommandArity {
    NoArgs,
    Args,
    OptionalArgs,
}

fn session_command_arity(word: &str) -> Option<CommandArity> {
    Some(match word {
        "exrates" | "stack" | "exact" | "approximate" | "approx" | "factor" | "simplify"
        | "expand" | "mode" | "exit" | "quit" | "history" => CommandArity::NoArgs,
        "set" | "save" | "variable" | "function" | "delete" | "keep" | "unkeep" | "assume"
        | "base" | "rpn" | "move" | "convert" | "to" | "find" | "info" => CommandArity::Args,
        "store" | "clear" | "swap" | "copy" | "rotate" | "pop" | "list" | "help" => {
            CommandArity::OptionalArgs
        }
        _ => return None,
    })
}

fn prefixed_number_starts(input: &str, prefix: &str, is_digit: fn(char) -> bool) -> bool {
    input
        .strip_prefix(prefix)
        .and_then(|rest| rest.chars().next())
        .is_some_and(is_digit)
}

fn is_hex_digit(ch: char) -> bool {
    ch.is_ascii_hexdigit()
}

fn is_binary_digit(ch: char) -> bool {
    matches!(ch, '0' | '1')
}

fn is_octal_digit(ch: char) -> bool {
    matches!(ch, '0'..='7')
}

fn is_duodecimal_digit(ch: char) -> bool {
    ch.is_ascii_digit() || matches!(ch, 'E' | 'X' | 'A' | 'B' | 'e' | 'x' | 'a' | 'b')
}

fn is_prefix_operator_position(previous: Option<&Token>) -> bool {
    previous.is_none_or(|token| {
        matches!(
            token.kind,
            TokenKind::Operator(_)
                | TokenKind::OpenParen
                | TokenKind::OpenBracket
                | TokenKind::Comma
                | TokenKind::Semicolon
                | TokenKind::Colon
                | TokenKind::CommandPrefix
        )
    })
}

fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_alphabetic() || is_unit_symbol(ch)
}

fn is_identifier_continue(ch: char) -> bool {
    ch == '_' || ch == '\'' || ch.is_alphanumeric() || is_unit_symbol(ch)
}

fn is_unit_symbol(ch: char) -> bool {
    !ch.is_ascii()
        && !ch.is_whitespace()
        && !matches!(
            ch,
            '±' | '−'
                | '×'
                | '÷'
                | '⋅'
                | '∕'
                | '→'
                | '≤'
                | '≥'
                | '≠'
                | '∧'
                | '∨'
                | '⊻'
                | '¬'
                | '∥'
                | '∪'
                | '∩'
                | '∖'
                | '⊖'
                | '∈'
                | '∉'
                | '∋'
                | '∌'
                | '⊊'
                | '⊆'
                | '⊋'
                | '⊇'
                | '('
                | ')'
                | '['
                | ']'
                | ','
                | ';'
                | '#'
                | '"'
        )
}

fn word_operator(word: &str) -> Option<Operator> {
    Some(match word {
        "plus" => Operator::Plus,
        "minus" => Operator::Minus,
        "times" => Operator::Multiply,
        "per" => Operator::Divide,
        "to" => Operator::Conversion,
        "rem" => Operator::Percent,
        "mod" => Operator::Modulo,
        "div" => Operator::IntegerDivide,
        "and" => Operator::LogicalAnd,
        "or" => Operator::LogicalOr,
        "xor" => Operator::LogicalXor,
        "nand" => Operator::LogicalNand,
        "nor" => Operator::LogicalNor,
        "not" => Operator::LogicalNot,
        "bitand" => Operator::BitwiseAnd,
        "bitor" => Operator::BitwiseOr,
        "bitxor" => Operator::BitwiseXor,
        "bitnot" => Operator::BitwiseNot,
        _ => return None,
    })
}

fn rest_char_width(rest: &str) -> usize {
    rest.chars().next().map_or(0, char::len_utf8)
}
