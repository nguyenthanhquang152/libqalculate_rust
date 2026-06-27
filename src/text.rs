//! Text formatting and Unicode helpers for qalc-compatible string values.

use crate::ast::{Expression, NaryChildren};
use crate::number::Number;

pub(crate) fn quote_text_for_qalc(text: &str) -> String {
    if text.chars().count() == 1 {
        format!("'{}'", escape_quoted_text(text, '\''))
    } else {
        format!("\"{}\"", escape_quoted_text(text, '"'))
    }
}

pub(crate) fn format_result_with_numbers<F>(expr: &Expression, format_number: &F) -> Option<String>
where
    F: Fn(&Number) -> String,
{
    match expr {
        Expression::Number(num) => Some(format_number(num)),
        Expression::Text(text) => Some(quote_text_for_qalc(text)),
        Expression::Symbolic(symbol) => Some(symbol.name().to_string()),
        Expression::Vector(items) => {
            let formatted = items
                .iter()
                .map(|item| format_result_with_numbers(item, format_number))
                .collect::<Option<Vec<_>>>()?;
            Some(format!("[{}]", formatted.join("  ")))
        }
        _ => None,
    }
}

pub(crate) fn format_raw_expression(expr: &Expression) -> String {
    match expr {
        Expression::Number(num) => num.to_string(),
        Expression::Text(text) => text.clone(),
        Expression::Symbolic(symbol) => symbol.name().to_string(),
        Expression::Variable(variable) => variable.id().to_string(),
        Expression::Unit { unit, prefix, .. } => {
            let mut out = String::new();
            if let Some(prefix) = prefix {
                out.push_str(prefix.id());
            }
            out.push_str(unit.id());
            out
        }
        Expression::Addition(children) => format_addition(children),
        Expression::Multiplication(children) => children
            .as_slice()
            .iter()
            .map(format_raw_expression)
            .collect::<Vec<_>>()
            .join("*"),
        Expression::Division {
            numerator,
            denominator,
        } => format!(
            "{}/{}",
            format_raw_expression(numerator),
            format_raw_expression(denominator)
        ),
        Expression::Negate(child) => format!("-{}", format_raw_expression(child)),
        Expression::Power { base, exponent } => format!(
            "{}^{}",
            format_raw_expression(base),
            format_raw_expression(exponent)
        ),
        Expression::FunctionCall { function, args } => {
            let args = args
                .iter()
                .map(format_raw_expression)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}({})", function.id(), args)
        }
        Expression::Vector(items) => {
            let formatted = items.iter().map(format_raw_expression).collect::<Vec<_>>();
            format!("[{}]", formatted.join("  "))
        }
        Expression::Remainder { lhs, rhs } => format_binary_raw(lhs, "%", rhs),
        Expression::Modulo { lhs, rhs } => format_binary_raw(lhs, " mod ", rhs),
        Expression::IntegerDivision { lhs, rhs } => format_binary_raw(lhs, "//", rhs),
        Expression::ShiftLeft { lhs, rhs } => format_binary_raw(lhs, "<<", rhs),
        Expression::ShiftRight { lhs, rhs } => format_binary_raw(lhs, ">>", rhs),
        Expression::BitwiseAnd(children) => format_nary_raw(children, "&"),
        Expression::BitwiseOr(children) => format_nary_raw(children, "|"),
        Expression::BitwiseXor(children) => format_nary_raw(children, " xor "),
        Expression::BitwiseNot(child) => format!("~{}", format_raw_expression(child)),
        Expression::LogicalAnd(children) => format_nary_raw(children, " and "),
        Expression::LogicalOr(children) => format_nary_raw(children, " or "),
        Expression::LogicalXor { lhs, rhs } => format_binary_raw(lhs, " xor ", rhs),
        Expression::LogicalNot(child) => format!("not {}", format_raw_expression(child)),
        Expression::Comparison { op, lhs, rhs } => {
            let op = match op {
                crate::ast::ComparisonOperator::Equal => "=",
                crate::ast::ComparisonOperator::NotEqual => "!=",
                crate::ast::ComparisonOperator::Less => "<",
                crate::ast::ComparisonOperator::LessOrEqual => "<=",
                crate::ast::ComparisonOperator::Greater => ">",
                crate::ast::ComparisonOperator::GreaterOrEqual => ">=",
            };
            format_binary_raw(lhs, op, rhs)
        }
        Expression::Conversion { expr, target } => format!(
            "{} to {}",
            format_raw_expression(expr),
            format_raw_expression(target)
        ),
        Expression::Assignment { variable, value } => {
            format!("{variable}:={}", format_raw_expression(value))
        }
        Expression::Inverse(child) => format!("1/{}", format_raw_expression(child)),
        Expression::Factorial(child) => format!("{}!", format_raw_expression(child)),
        Expression::DoubleFactorial(child) => format!("{}!!", format_raw_expression(child)),
        Expression::MultiFactorial { expr, count } => {
            format!(
                "{}{}",
                format_raw_expression(expr),
                "!".repeat(*count as usize)
            )
        }
        Expression::Percent(child) => format!("{}%", format_raw_expression(child)),
        Expression::Parallel { lhs, rhs } => format_binary_raw(lhs, " parallel ", rhs),
        Expression::Undefined => "undefined".to_string(),
        Expression::Aborted => "aborted".to_string(),
        Expression::DateTime(value) => value.source().to_string(),
    }
}

pub(crate) fn unicode_len(text: &str) -> usize {
    text.chars().count()
}

pub(crate) fn codepoint_to_string(value: u128) -> Option<String> {
    let value = u32::try_from(value).ok()?;
    if !(32..=0x10ffff).contains(&value) {
        return None;
    }
    char::from_u32(value).map(|ch| ch.to_string())
}

fn escape_quoted_text(text: &str, quote: char) -> String {
    let mut escaped = String::new();
    for ch in text.chars() {
        if ch == quote || ch == '\\' {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped
}

fn format_addition(children: &NaryChildren) -> String {
    let mut out = String::new();
    for (index, child) in children.as_slice().iter().enumerate() {
        match child {
            Expression::Negate(inner) => {
                out.push('-');
                out.push_str(&format_raw_expression(inner));
            }
            _ => {
                if index > 0 {
                    out.push('+');
                }
                out.push_str(&format_raw_expression(child));
            }
        }
    }
    out
}

fn format_binary_raw(lhs: &Expression, op: &str, rhs: &Expression) -> String {
    format!(
        "{}{}{}",
        format_raw_expression(lhs),
        op,
        format_raw_expression(rhs)
    )
}

fn format_nary_raw(children: &NaryChildren, op: &str) -> String {
    children
        .as_slice()
        .iter()
        .map(format_raw_expression)
        .collect::<Vec<_>>()
        .join(op)
}
