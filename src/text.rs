//! Text formatting and Unicode helpers for qalc-compatible string values.

use crate::ast::{Expression, NaryChildren, PrecedenceClass};
use crate::number::Number;

pub(crate) fn quote_text_for_qalc(text: &str) -> String {
    if text.chars().count() == 1 {
        format!("'{}'", escape_quoted_text(text, '\''))
    } else {
        format!("\"{}\"", escape_quoted_text(text, '"'))
    }
}

fn quote_datetime_for_qalc(text: &str) -> String {
    format!("\"{}\"", escape_quoted_text(text, '"'))
}

fn get_term_degree(term: &Expression) -> f64 {
    match term {
        Expression::Number(_) => 0.0,
        Expression::Power { base, exponent } => {
            if let Expression::Number(ref num) = **exponent {
                num.to_f64() * get_term_degree(base)
            } else {
                get_term_degree(base)
            }
        }
        Expression::Multiplication(nary) => nary.as_slice().iter().map(get_term_degree).sum(),
        Expression::Division {
            numerator,
            denominator,
        } => get_term_degree(numerator) - get_term_degree(denominator),
        Expression::Negate(inner) => get_term_degree(inner),
        _ => 1.0,
    }
}

fn is_term_negative(term: &Expression) -> bool {
    match term {
        Expression::Number(num) => num.is_negative(),
        Expression::Negate(_) => true,
        Expression::Multiplication(nary) => {
            nary.as_slice()
                .iter()
                .filter(|term| is_term_negative(term))
                .count()
                % 2
                == 1
        }
        _ => false,
    }
}

fn get_absolute_term(term: &Expression) -> Expression {
    match term {
        Expression::Negate(inner) => *inner.clone(),
        Expression::Number(num) => Expression::Number(num.abs()),
        Expression::Multiplication(nary) => {
            let mut new_factors = Vec::new();
            for factor in nary.as_slice() {
                if is_term_negative(factor) {
                    new_factors.push(get_absolute_term(factor));
                } else {
                    new_factors.push(factor.clone());
                }
            }
            Expression::Multiplication(NaryChildren::new(new_factors).unwrap())
        }
        _ => term.clone(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChildPosition {
    Nary,
    Subtrahend,
    Left,
    Right,
    PrefixOperand,
    PostfixOperand,
    PowerBase,
    PowerExponent,
    AssignmentValue,
}

fn precedence_rank(expr: &Expression) -> u8 {
    match expr.operator_metadata().map(|metadata| metadata.precedence) {
        Some(PrecedenceClass::Assignment) => 0,
        Some(PrecedenceClass::Conversion) => 1,
        Some(PrecedenceClass::LogicalXor) => 2,
        Some(PrecedenceClass::LogicalOr) => 3,
        Some(PrecedenceClass::LogicalAnd) => 6,
        Some(PrecedenceClass::BitwiseOr) => 7,
        Some(PrecedenceClass::BitwiseXor) => 8,
        Some(PrecedenceClass::BitwiseAnd) => 9,
        Some(PrecedenceClass::Comparison) => 10,
        Some(PrecedenceClass::Shift) => 11,
        Some(PrecedenceClass::Additive) => 12,
        Some(PrecedenceClass::Parallel) => 13,
        Some(PrecedenceClass::Multiplicative) => 14,
        Some(PrecedenceClass::Prefix) => 16,
        Some(PrecedenceClass::Power) => 16,
        Some(PrecedenceClass::Primary) | None => 18,
    }
}

fn has_operator_shape(expr: &Expression) -> bool {
    expr.operator_metadata().is_some()
}

fn needs_parentheses(parent: &Expression, child: &Expression, position: ChildPosition) -> bool {
    if !has_operator_shape(child) {
        return false;
    }

    let child_precedence = precedence_rank(child);
    let parent_precedence = precedence_rank(parent);
    if child_precedence < parent_precedence {
        return true;
    }
    if child_precedence > parent_precedence {
        return false;
    }

    match (parent, position) {
        (Expression::Addition(_), ChildPosition::Subtrahend) => true,
        (Expression::Addition(_), ChildPosition::Nary) => matches!(child, Expression::Addition(_)),
        (Expression::Multiplication(_), ChildPosition::Nary) => matches!(
            child,
            Expression::Division { .. }
                | Expression::Remainder { .. }
                | Expression::Modulo { .. }
                | Expression::IntegerDivision { .. }
                | Expression::Multiplication(_)
        ),
        (Expression::Division { .. }, ChildPosition::Right) => true,
        (Expression::Power { .. }, ChildPosition::PowerBase) => true,
        (Expression::Factorial(_), ChildPosition::PostfixOperand)
        | (Expression::DoubleFactorial(_), ChildPosition::PostfixOperand)
        | (Expression::MultiFactorial { .. }, ChildPosition::PostfixOperand)
        | (Expression::Percent(_), ChildPosition::PostfixOperand) => {
            !matches!(child, Expression::Power { .. })
        }
        (
            Expression::Remainder { .. }
            | Expression::Modulo { .. }
            | Expression::IntegerDivision { .. }
            | Expression::ShiftLeft { .. }
            | Expression::ShiftRight { .. }
            | Expression::LogicalXor { .. }
            | Expression::Parallel { .. }
            | Expression::Conversion { .. },
            ChildPosition::Right,
        ) => true,
        (Expression::Comparison { .. }, ChildPosition::Left | ChildPosition::Right) => true,
        _ => false,
    }
}

fn parenthesize_if_needed(
    parent: &Expression,
    child: &Expression,
    position: ChildPosition,
    formatted: String,
) -> String {
    if needs_parentheses(parent, child, position) {
        format!("({formatted})")
    } else {
        formatted
    }
}

fn format_raw_child(parent: &Expression, child: &Expression, position: ChildPosition) -> String {
    let formatted = format_raw_expression(child);
    parenthesize_if_needed(parent, child, position, formatted)
}

fn format_result_child<F>(
    parent: &Expression,
    child: &Expression,
    position: ChildPosition,
    format_number: &F,
) -> Option<String>
where
    F: Fn(&Number) -> String,
{
    let formatted = format_result_with_numbers(child, format_number)?;
    Some(parenthesize_if_needed(parent, child, position, formatted))
}

fn multiplication_separator(prev_expr: &Expression, expr: &Expression) -> &'static str {
    match (prev_expr, expr) {
        (Expression::Number(_), Expression::Unit { .. }) => " ",
        _ => "*",
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
        Expression::Variable(variable) => Some(variable.id().to_string()),
        Expression::Unit { unit, prefix, .. } => {
            let mut out = String::new();
            if let Some(prefix) = prefix {
                out.push_str(prefix.id());
            }
            out.push_str(unit.id());
            Some(out)
        }
        Expression::Addition(children) => {
            let mut terms = children.as_slice().to_vec();
            terms.sort_by(|a, b| {
                let deg_a = get_term_degree(a);
                let deg_b = get_term_degree(b);
                deg_b
                    .partial_cmp(&deg_a)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| format_raw_expression(a).cmp(&format_raw_expression(b)))
            });

            if is_term_negative(&terms[0]) {
                if let Some(pos) = terms.iter().position(|t| !is_term_negative(t)) {
                    let first_pos = terms.remove(pos);
                    terms.insert(0, first_pos);
                }
            }

            let mut out = String::new();
            for (index, child) in terms.iter().enumerate() {
                if index == 0 {
                    let s = format_result_child(expr, child, ChildPosition::Nary, format_number)?;
                    out.push_str(&s);
                } else if is_term_negative(child) {
                    let abs_child = get_absolute_term(child);
                    let s = format_result_child(
                        expr,
                        &abs_child,
                        ChildPosition::Subtrahend,
                        format_number,
                    )?;
                    out.push_str(" - ");
                    out.push_str(&s);
                } else {
                    let s = format_result_child(expr, child, ChildPosition::Nary, format_number)?;
                    out.push_str(" + ");
                    out.push_str(&s);
                }
            }
            Some(out)
        }
        Expression::Multiplication(children) => {
            let mut parts = Vec::new();
            for child in children.as_slice() {
                let s = format_result_child(expr, child, ChildPosition::Nary, format_number)?;
                parts.push((child, s));
            }
            let mut out = String::new();
            for i in 0..parts.len() {
                let (expr, s) = &parts[i];
                if i > 0 {
                    let (prev_expr, _) = &parts[i - 1];
                    out.push_str(multiplication_separator(prev_expr, expr));
                }
                out.push_str(s);
            }
            Some(out)
        }
        Expression::Division {
            numerator,
            denominator,
        } => {
            let num_formatted =
                format_result_child(expr, numerator, ChildPosition::Left, format_number)?;
            let den_formatted =
                format_result_child(expr, denominator, ChildPosition::Right, format_number)?;
            Some(format!("{num_formatted} / {den_formatted}"))
        }
        Expression::Negate(child) => {
            let formatted =
                format_result_child(expr, child, ChildPosition::PrefixOperand, format_number)?;
            Some(format!("-{formatted}"))
        }
        Expression::Power { base, exponent } => {
            let base_str =
                format_result_child(expr, base, ChildPosition::PowerBase, format_number)?;
            let exp_str =
                format_result_child(expr, exponent, ChildPosition::PowerExponent, format_number)?;
            Some(format!("{base_str}^{exp_str}"))
        }
        Expression::FunctionCall { function, args } => {
            let formatted_args = args
                .iter()
                .map(|item| format_result_with_numbers(item, format_number))
                .collect::<Option<Vec<_>>>()?;
            Some(format!("{}({})", function.id(), formatted_args.join(", ")))
        }
        Expression::Vector(items) => {
            if crate::matrix::is_rectangular_matrix(expr) {
                let rows = items
                    .iter()
                    .map(|item| {
                        let Expression::Vector(row_items) = item else {
                            return None;
                        };
                        row_items
                            .iter()
                            .map(|cell| format_result_with_numbers(cell, format_number))
                            .collect::<Option<Vec<_>>>()
                            .map(|row| row.join("  "))
                    })
                    .collect::<Option<Vec<_>>>()?;
                Some(format!("[{}]", rows.join("; ")))
            } else {
                let formatted = items
                    .iter()
                    .map(|item| format_result_with_numbers(item, format_number))
                    .collect::<Option<Vec<_>>>()?;
                Some(format!("[{}]", formatted.join("  ")))
            }
        }
        Expression::Remainder { lhs, rhs } => {
            let l = format_result_child(expr, lhs, ChildPosition::Left, format_number)?;
            let r = format_result_child(expr, rhs, ChildPosition::Right, format_number)?;
            Some(format!("{l}%{r}"))
        }
        Expression::Modulo { lhs, rhs } => {
            let l = format_result_child(expr, lhs, ChildPosition::Left, format_number)?;
            let r = format_result_child(expr, rhs, ChildPosition::Right, format_number)?;
            Some(format!("{l} mod {r}"))
        }
        Expression::IntegerDivision { lhs, rhs } => {
            let l = format_result_child(expr, lhs, ChildPosition::Left, format_number)?;
            let r = format_result_child(expr, rhs, ChildPosition::Right, format_number)?;
            Some(format!("{l}//{r}"))
        }
        Expression::ShiftLeft { lhs, rhs } => {
            let l = format_result_child(expr, lhs, ChildPosition::Left, format_number)?;
            let r = format_result_child(expr, rhs, ChildPosition::Right, format_number)?;
            Some(format!("{l}<<{r}"))
        }
        Expression::ShiftRight { lhs, rhs } => {
            let l = format_result_child(expr, lhs, ChildPosition::Left, format_number)?;
            let r = format_result_child(expr, rhs, ChildPosition::Right, format_number)?;
            Some(format!("{l}>>{r}"))
        }
        Expression::BitwiseAnd(children) => {
            let formatted = children
                .as_slice()
                .iter()
                .map(|item| format_result_child(expr, item, ChildPosition::Nary, format_number))
                .collect::<Option<Vec<_>>>()?;
            Some(formatted.join("&"))
        }
        Expression::BitwiseOr(children) => {
            let formatted = children
                .as_slice()
                .iter()
                .map(|item| format_result_child(expr, item, ChildPosition::Nary, format_number))
                .collect::<Option<Vec<_>>>()?;
            Some(formatted.join("|"))
        }
        Expression::BitwiseXor(children) => {
            let formatted = children
                .as_slice()
                .iter()
                .map(|item| format_result_child(expr, item, ChildPosition::Nary, format_number))
                .collect::<Option<Vec<_>>>()?;
            Some(formatted.join(" xor "))
        }
        Expression::BitwiseNot(child) => {
            let formatted =
                format_result_child(expr, child, ChildPosition::PrefixOperand, format_number)?;
            Some(format!("~{formatted}"))
        }
        Expression::LogicalAnd(children) => {
            let formatted = children
                .as_slice()
                .iter()
                .map(|item| format_result_child(expr, item, ChildPosition::Nary, format_number))
                .collect::<Option<Vec<_>>>()?;
            Some(formatted.join(" and "))
        }
        Expression::LogicalOr(children) => {
            let formatted = children
                .as_slice()
                .iter()
                .map(|item| format_result_child(expr, item, ChildPosition::Nary, format_number))
                .collect::<Option<Vec<_>>>()?;
            Some(formatted.join(" or "))
        }
        Expression::LogicalXor { lhs, rhs } => {
            let l = format_result_child(expr, lhs, ChildPosition::Left, format_number)?;
            let r = format_result_child(expr, rhs, ChildPosition::Right, format_number)?;
            Some(format!("{l} xor {r}"))
        }
        Expression::LogicalNot(child) => {
            let formatted =
                format_result_child(expr, child, ChildPosition::PrefixOperand, format_number)?;
            Some(format!("not {formatted}"))
        }
        Expression::Comparison { op, lhs, rhs } => {
            let op_str = match op {
                crate::ast::ComparisonOperator::Equal => "=",
                crate::ast::ComparisonOperator::NotEqual => "!=",
                crate::ast::ComparisonOperator::Less => "<",
                crate::ast::ComparisonOperator::LessOrEqual => "<=",
                crate::ast::ComparisonOperator::Greater => ">",
                crate::ast::ComparisonOperator::GreaterOrEqual => ">=",
            };
            let l = format_result_child(expr, lhs, ChildPosition::Left, format_number)?;
            let r = format_result_child(expr, rhs, ChildPosition::Right, format_number)?;
            Some(format!("{l} {op_str} {r}"))
        }
        Expression::Conversion { expr, target } => {
            let e = format_result_child(
                &Expression::Conversion {
                    expr: expr.clone(),
                    target: target.clone(),
                },
                expr,
                ChildPosition::Left,
                format_number,
            )?;
            let t = format_result_child(
                &Expression::Conversion {
                    expr: expr.clone(),
                    target: target.clone(),
                },
                target,
                ChildPosition::Right,
                format_number,
            )?;
            Some(format!("{e} to {t}"))
        }
        Expression::Assignment { variable, value } => {
            let val =
                format_result_child(expr, value, ChildPosition::AssignmentValue, format_number)?;
            Some(format!("{variable}:={val}"))
        }
        Expression::Inverse(child) => {
            let formatted = format_result_child(expr, child, ChildPosition::Right, format_number)?;
            Some(format!("1/{formatted}"))
        }
        Expression::Factorial(child) => {
            let formatted =
                format_result_child(expr, child, ChildPosition::PostfixOperand, format_number)?;
            Some(format!("{formatted}!"))
        }
        Expression::DoubleFactorial(child) => {
            let formatted =
                format_result_child(expr, child, ChildPosition::PostfixOperand, format_number)?;
            Some(format!("{formatted}!!"))
        }
        Expression::MultiFactorial { expr, count } => {
            let formatted = format_result_child(
                &Expression::MultiFactorial {
                    expr: expr.clone(),
                    count: *count,
                },
                expr,
                ChildPosition::PostfixOperand,
                format_number,
            )?;
            Some(format!("{}{}", formatted, "!".repeat(*count as usize)))
        }
        Expression::Percent(child) => {
            let formatted =
                format_result_child(expr, child, ChildPosition::PostfixOperand, format_number)?;
            Some(format!("{formatted}%"))
        }
        Expression::Parallel { lhs, rhs } => {
            let l = format_result_child(expr, lhs, ChildPosition::Left, format_number)?;
            let r = format_result_child(expr, rhs, ChildPosition::Right, format_number)?;
            Some(format!("{l} parallel {r}"))
        }
        Expression::Undefined => Some("undefined".to_string()),
        Expression::Aborted => Some("aborted".to_string()),
        Expression::DateTime(value) => Some(quote_datetime_for_qalc(value.source())),
    }
}

pub(crate) fn format_raw_expression(expr: &Expression) -> String {
    match expr {
        Expression::Number(num) => num.to_string(),
        Expression::Text(text) => quote_text_for_qalc(text),
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
        Expression::Addition(children) => format_addition(expr, children),
        Expression::Multiplication(children) => {
            let mut parts = Vec::new();
            for child in children.as_slice() {
                let s = format_raw_child(expr, child, ChildPosition::Nary);
                parts.push((child, s));
            }
            let mut out = String::new();
            for i in 0..parts.len() {
                let (expr, s) = &parts[i];
                if i > 0 {
                    let (prev_expr, _) = &parts[i - 1];
                    out.push_str(multiplication_separator(prev_expr, expr));
                }
                out.push_str(s);
            }
            out
        }
        Expression::Division {
            numerator,
            denominator,
        } => {
            let num_formatted = format_raw_child(expr, numerator, ChildPosition::Left);
            let den_formatted = format_raw_child(expr, denominator, ChildPosition::Right);
            format!("{num_formatted}/{den_formatted}")
        }
        Expression::Negate(child) => {
            format!(
                "-{}",
                format_raw_child(expr, child, ChildPosition::PrefixOperand)
            )
        }
        Expression::Power { base, exponent } => format!(
            "{}^{}",
            format_raw_child(expr, base, ChildPosition::PowerBase),
            format_raw_child(expr, exponent, ChildPosition::PowerExponent)
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
            if let Some(rows) = expr.as_matrix_rows() {
                let rows = rows
                    .iter()
                    .map(|row| {
                        row.iter()
                            .map(format_raw_expression)
                            .collect::<Vec<_>>()
                            .join("  ")
                    })
                    .collect::<Vec<_>>();
                format!("[{}]", rows.join("; "))
            } else {
                let formatted = items.iter().map(format_raw_expression).collect::<Vec<_>>();
                format!("[{}]", formatted.join("  "))
            }
        }
        Expression::Remainder { lhs, rhs } => format_binary_raw(expr, lhs, "%", rhs),
        Expression::Modulo { lhs, rhs } => format_binary_raw(expr, lhs, " mod ", rhs),
        Expression::IntegerDivision { lhs, rhs } => format_binary_raw(expr, lhs, "//", rhs),
        Expression::ShiftLeft { lhs, rhs } => format_binary_raw(expr, lhs, "<<", rhs),
        Expression::ShiftRight { lhs, rhs } => format_binary_raw(expr, lhs, ">>", rhs),
        Expression::BitwiseAnd(children) => format_nary_raw(expr, children, "&"),
        Expression::BitwiseOr(children) => format_nary_raw(expr, children, "|"),
        Expression::BitwiseXor(children) => format_nary_raw(expr, children, " xor "),
        Expression::BitwiseNot(child) => {
            format!(
                "~{}",
                format_raw_child(expr, child, ChildPosition::PrefixOperand)
            )
        }
        Expression::LogicalAnd(children) => format_nary_raw(expr, children, " and "),
        Expression::LogicalOr(children) => format_nary_raw(expr, children, " or "),
        Expression::LogicalXor { lhs, rhs } => format_binary_raw(expr, lhs, " xor ", rhs),
        Expression::LogicalNot(child) => {
            format!(
                "not {}",
                format_raw_child(expr, child, ChildPosition::PrefixOperand)
            )
        }
        Expression::Comparison { op, lhs, rhs } => {
            let op = match op {
                crate::ast::ComparisonOperator::Equal => "=",
                crate::ast::ComparisonOperator::NotEqual => "!=",
                crate::ast::ComparisonOperator::Less => "<",
                crate::ast::ComparisonOperator::LessOrEqual => "<=",
                crate::ast::ComparisonOperator::Greater => ">",
                crate::ast::ComparisonOperator::GreaterOrEqual => ">=",
            };
            format_binary_raw(expr, lhs, &format!(" {op} "), rhs)
        }
        Expression::Conversion { expr, target } => format!(
            "{} to {}",
            format_raw_child(
                &Expression::Conversion {
                    expr: expr.clone(),
                    target: target.clone(),
                },
                expr,
                ChildPosition::Left
            ),
            format_raw_child(
                &Expression::Conversion {
                    expr: expr.clone(),
                    target: target.clone(),
                },
                target,
                ChildPosition::Right
            )
        ),
        Expression::Assignment { variable, value } => {
            format!(
                "{variable}:={}",
                format_raw_child(expr, value, ChildPosition::AssignmentValue)
            )
        }
        Expression::Inverse(child) => {
            format!("1/{}", format_raw_child(expr, child, ChildPosition::Right))
        }
        Expression::Factorial(child) => {
            format!(
                "{}!",
                format_raw_child(expr, child, ChildPosition::PostfixOperand)
            )
        }
        Expression::DoubleFactorial(child) => {
            format!(
                "{}!!",
                format_raw_child(expr, child, ChildPosition::PostfixOperand)
            )
        }
        Expression::MultiFactorial { expr, count } => {
            format!(
                "{}{}",
                format_raw_child(
                    &Expression::MultiFactorial {
                        expr: expr.clone(),
                        count: *count,
                    },
                    expr,
                    ChildPosition::PostfixOperand
                ),
                "!".repeat(*count as usize)
            )
        }
        Expression::Percent(child) => {
            format!(
                "{}%",
                format_raw_child(expr, child, ChildPosition::PostfixOperand)
            )
        }
        Expression::Parallel { lhs, rhs } => format_binary_raw(expr, lhs, " parallel ", rhs),
        Expression::Undefined => "undefined".to_string(),
        Expression::Aborted => "aborted".to_string(),
        Expression::DateTime(value) => quote_datetime_for_qalc(value.source()),
    }
}

pub(crate) fn format_qalc_equation(
    input: &str,
    output: &str,
    approximate: bool,
    unicode_enabled: bool,
    message_line_count: usize,
) -> String {
    let trimmed_input = input.trim();
    let explicit_radix_literal = trimmed_input
        .strip_prefix(['+', '-'])
        .unwrap_or(trimmed_input)
        .get(..2)
        .is_some_and(|prefix| matches!(prefix, "0x" | "0X" | "0b" | "0B" | "0o" | "0O"));
    let formatted_input = match crate::parser::operators::parse_expression(input) {
        Ok(_) if explicit_radix_literal => trimmed_input.to_string(),
        Ok(expression) => {
            if let Expression::Conversion { target, .. } = &expression {
                if let Expression::Symbolic(symbol) = target.as_ref() {
                    if symbol.name().eq_ignore_ascii_case("latex")
                        || symbol.name().eq_ignore_ascii_case("html")
                    {
                        return output.to_string();
                    }
                }
            }
            if let Expression::Division {
                numerator,
                denominator,
            } = &expression
            {
                format!(
                    "{} / {}",
                    format_raw_child(&expression, numerator, ChildPosition::Left),
                    format_raw_child(&expression, denominator, ChildPosition::Right)
                )
            } else {
                format_raw_expression(&expression)
            }
        }
        Err(_) => input.trim().to_string(),
    };

    let formatted_input = style_qalc_input_operators(&formatted_input, unicode_enabled);

    let relation = if approximate && unicode_enabled {
        " ≈ "
    } else if approximate {
        " = approx. "
    } else {
        " = "
    };
    if message_line_count == 0 {
        return format!("{formatted_input}{relation}{output}");
    }

    match output
        .match_indices('\n')
        .nth(message_line_count.saturating_sub(1))
    {
        Some((split_at, _)) => format!(
            "{}\n{formatted_input}{relation}{}",
            &output[..split_at],
            &output[split_at + 1..]
        ),
        None => format!("{formatted_input}{relation}{output}"),
    }
}

fn style_qalc_input_operators(input: &str, unicode_enabled: bool) -> String {
    let chars = input.chars().collect::<Vec<_>>();
    let mut styled = String::with_capacity(input.len());
    let mut quote = None;
    let mut escaped = false;
    let mut index = 0;

    while index < chars.len() {
        let ch = chars[index];
        if let Some(active_quote) = quote {
            styled.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == active_quote {
                quote = None;
            }
            index += 1;
            continue;
        }
        if matches!(ch, '\'' | '"') {
            quote = Some(ch);
            styled.push(ch);
            index += 1;
            continue;
        }
        if ch == '*' {
            while styled.ends_with(char::is_whitespace) {
                styled.pop();
            }
            styled.push_str(if unicode_enabled { " × " } else { " * " });
            index += 1;
            while index < chars.len() && chars[index].is_whitespace() {
                index += 1;
            }
            continue;
        }
        if unicode_enabled && ch == '-' {
            styled.push('−');
            index += 1;
            continue;
        }
        if unicode_enabled && ch == '^' && index + 1 < chars.len() {
            let exponent = chars[index + 1];
            let has_token_boundary = index + 2 == chars.len()
                || !(chars[index + 2].is_ascii_alphanumeric()
                    || matches!(chars[index + 2], '.' | '_'));
            if has_token_boundary && matches!(exponent, '2' | '3') {
                styled.push(if exponent == '2' { '²' } else { '³' });
                index += 2;
                continue;
            }
        }
        styled.push(ch);
        index += 1;
    }

    styled
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

fn format_addition(parent: &Expression, children: &NaryChildren) -> String {
    let mut terms = children.as_slice().to_vec();
    terms.sort_by(|a, b| {
        let deg_a = get_term_degree(a);
        let deg_b = get_term_degree(b);
        deg_b
            .partial_cmp(&deg_a)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| format_raw_expression(a).cmp(&format_raw_expression(b)))
    });

    if is_term_negative(&terms[0]) {
        if let Some(pos) = terms.iter().position(|t| !is_term_negative(t)) {
            let first_pos = terms.remove(pos);
            terms.insert(0, first_pos);
        }
    }

    let mut out = String::new();
    for (index, child) in terms.iter().enumerate() {
        if index == 0 {
            out.push_str(&format_raw_child(parent, child, ChildPosition::Nary));
        } else if is_term_negative(child) {
            let abs_child = get_absolute_term(child);
            out.push_str(" - ");
            out.push_str(&format_raw_child(
                parent,
                &abs_child,
                ChildPosition::Subtrahend,
            ));
        } else {
            out.push_str(" + ");
            out.push_str(&format_raw_child(parent, child, ChildPosition::Nary));
        }
    }
    out
}

fn format_binary_raw(parent: &Expression, lhs: &Expression, op: &str, rhs: &Expression) -> String {
    format!(
        "{}{}{}",
        format_raw_child(parent, lhs, ChildPosition::Left),
        op,
        format_raw_child(parent, rhs, ChildPosition::Right)
    )
}

fn format_nary_raw(parent: &Expression, children: &NaryChildren, op: &str) -> String {
    children
        .as_slice()
        .iter()
        .map(|child| format_raw_child(parent, child, ChildPosition::Nary))
        .collect::<Vec<_>>()
        .join(op)
}

#[cfg(test)]
mod tests {
    use super::{format_qalc_equation, format_raw_expression, format_result_with_numbers};
    use crate::ast::{
        ComparisonOperator, Expression, FunctionRef, NaryChildren, PrefixRef, Symbol, UnitRef,
        VariableRef,
    };
    use crate::number::Number;

    #[test]
    fn qalc_equation_formats_input_and_preserves_message_lines() {
        assert_eq!(
            format_qalc_equation("1+1", "2", false, true, 0),
            "1 + 1 = 2"
        );
        assert_eq!(
            format_qalc_equation("warning(1)", "warning: first\n0", false, true, 1),
            "warning: first\nwarning(1) = 0"
        );
        assert_eq!(
            format_qalc_equation("matrix()", "[1  2]\n[3  4]", false, true, 0),
            "matrix() = [1  2]\n[3  4]"
        );
        assert_eq!(
            format_qalc_equation(
                "1/2 to latex",
                "$\\displaystyle \\frac{1}{2}$",
                false,
                true,
                0,
            ),
            "$\\displaystyle \\frac{1}{2}$"
        );
        assert_eq!(
            format_qalc_equation("1/3", "0.3333333333", true, false, 0),
            "1 / 3 = approx. 0.3333333333"
        );
        assert_eq!(
            format_qalc_equation("x^20", "1", false, true, 0),
            "x^20 = 1"
        );
        assert_eq!(format_qalc_equation("x^2", "1", false, true, 0), "x² = 1");
        assert_eq!(
            format_qalc_equation("\"a*b\"", "\"a*b\"", false, true, 0),
            "\"a*b\" = \"a*b\""
        );
        assert_eq!(
            format_qalc_equation("sqrt(-1)", "i", false, true, 0),
            "sqrt(−1) = i"
        );
    }
    use crate::parser::operators::parse_expression;
    use proptest::prelude::*;

    fn n(value: i32) -> Expression {
        Expression::Number(Number::from_i32(value))
    }

    fn sym(name: &str) -> Expression {
        Expression::Symbolic(Symbol::new(name))
    }

    fn operands(children: Vec<Expression>) -> NaryChildren {
        NaryChildren::new(children).expect("valid n-ary expression")
    }

    fn add(children: Vec<Expression>) -> Expression {
        Expression::Addition(operands(children))
    }

    fn mul(children: Vec<Expression>) -> Expression {
        Expression::Multiplication(operands(children))
    }

    fn div(numerator: Expression, denominator: Expression) -> Expression {
        Expression::Division {
            numerator: Box::new(numerator),
            denominator: Box::new(denominator),
        }
    }

    fn pow(base: Expression, exponent: Expression) -> Expression {
        Expression::Power {
            base: Box::new(base),
            exponent: Box::new(exponent),
        }
    }

    fn neg(expr: Expression) -> Expression {
        Expression::Negate(Box::new(expr))
    }

    fn rem(lhs: Expression, rhs: Expression) -> Expression {
        Expression::Remainder {
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
    }

    fn lt(lhs: Expression, rhs: Expression) -> Expression {
        Expression::Comparison {
            op: ComparisonOperator::Less,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
    }

    fn raw(input: &str) -> String {
        let expr = parse_expression(input).unwrap_or_else(|err| panic!("parse {input:?}: {err}"));
        format_raw_expression(&expr)
    }

    fn result(input: &str) -> String {
        let expr = parse_expression(input).unwrap_or_else(|err| panic!("parse {input:?}: {err}"));
        format_result_with_numbers(&expr, &Number::to_string).expect("expression should format")
    }

    #[test]
    fn datetime_literals_keep_quotes_when_formatted() {
        let expr = parse_expression("\"2020-05-20\"").expect("valid date literal");
        assert!(matches!(expr, Expression::DateTime(_)));

        assert_eq!(format_raw_expression(&expr), "\"2020-05-20\"");
        assert_eq!(
            format_result_with_numbers(&expr, &ToString::to_string),
            Some("\"2020-05-20\"".to_string())
        );
    }

    #[test]
    fn datetime_literal_format_round_trips_as_datetime_not_arithmetic() {
        let expr = parse_expression("\"2020-05-20\"").expect("valid date literal");
        let formatted = format_raw_expression(&expr);
        let reparsed = parse_expression(&formatted).expect("formatted date literal reparses");

        assert!(matches!(reparsed, Expression::DateTime(_)));
    }

    #[test]
    fn raw_formatter_preserves_precedence_boundaries() {
        let cases = [
            ("x * (y + z)", "x*(y + z)"),
            ("(x + y) * z", "(x + y)*z"),
            ("(x + y)^2", "(x + y)^2"),
            ("1/(x + y)", "1/(x + y)"),
            ("-(x + y)", "-(x + y)"),
            ("-x^2", "-x^2"),
            ("x^y^z", "x^y^z"),
            ("(x + y)!", "(x + y)!"),
            ("sqrt(x + 1)", "sqrt(x + 1)"),
            ("x < y + z", "x < y + z"),
            ("x and (y or z)", "x and (y or z)"),
        ];

        for (input, expected) in cases {
            assert_eq!(raw(input), expected, "raw formatting {input:?}");
        }
    }

    #[test]
    fn result_formatter_preserves_precedence_boundaries() {
        let cases = [
            ("x * (y + z)", "x*(y + z)"),
            ("(x + y)^2", "(x + y)^2"),
            ("1/(x + y)", "1 / (x + y)"),
            ("-(x + y)", "-(x + y)"),
            ("sqrt(x + 1)", "sqrt(x + 1)"),
        ];

        for (input, expected) in cases {
            assert_eq!(result(input), expected, "result formatting {input:?}");
        }
    }

    #[test]
    fn addition_orders_by_degree_and_formats_negative_terms() {
        let expr = add(vec![
            n(1),
            sym("x"),
            neg(pow(sym("x"), n(4))),
            div(pow(sym("x"), n(3)), sym("y")),
            mul(vec![sym("x"), sym("y")]),
        ]);
        assert_eq!(format_raw_expression(&expr), "x*y - x^4 + x^3/y + x + 1");
        assert_eq!(
            format_result_with_numbers(&expr, &Number::to_string),
            Some("x*y - x^4 + x^3 / y + x + 1".to_string())
        );

        let power_base_degree = add(vec![
            pow(mul(vec![sym("x"), sym("y")]), n(3)),
            pow(sym("z"), n(2)),
        ]);
        assert_eq!(format_raw_expression(&power_base_degree), "(x*y)^3 + z^2");

        let negative_factor = add(vec![sym("x"), mul(vec![n(-2), sym("x")])]);
        assert_eq!(format_raw_expression(&negative_factor), "x - 2*x");
        assert_eq!(
            format_result_with_numbers(&negative_factor, &Number::to_string),
            Some("x - 2*x".to_string())
        );

        let even_negative_product = add(vec![sym("a"), mul(vec![n(-2), neg(sym("x"))])]);
        assert_eq!(format_raw_expression(&even_negative_product), "-2*-x + a");

        let odd_negative_product = add(vec![
            sym("a"),
            mul(vec![n(-2), neg(sym("x")), neg(sym("y"))]),
        ]);
        assert_eq!(format_raw_expression(&odd_negative_product), "a - 2*x*y");
    }

    #[test]
    fn addition_subtrahend_keeps_grouping() {
        let grouped = add(vec![sym("y"), sym("z")]);
        let expr = add(vec![sym("x"), neg(grouped)]);

        assert_eq!(format_raw_expression(&expr), "x - (y + z)");
        assert_eq!(
            format_result_with_numbers(&expr, &Number::to_string),
            Some("x - (y + z)".to_string())
        );
    }

    #[test]
    fn equal_precedence_raw_parentheses_preserve_tree_shape() {
        let cases = [
            (
                add(vec![add(vec![sym("x"), sym("y")]), sym("z")]),
                "(x + y) + z",
            ),
            (
                mul(vec![sym("x"), mul(vec![sym("y"), sym("z")])]),
                "x*(y*z)",
            ),
            (mul(vec![sym("x"), div(sym("y"), sym("z"))]), "x*(y/z)"),
            (div(sym("x"), div(sym("y"), sym("z"))), "x/(y/z)"),
            (pow(pow(sym("x"), sym("y")), sym("z")), "(x^y)^z"),
            (pow(sym("x"), pow(sym("y"), sym("z"))), "x^y^z"),
            (neg(pow(sym("x"), n(2))), "-x^2"),
            (Expression::Factorial(Box::new(neg(sym("x")))), "(-x)!"),
            (
                Expression::Factorial(Box::new(pow(sym("x"), n(2)))),
                "(x^2)!",
            ),
            (
                Expression::Factorial(Box::new(Expression::Factorial(Box::new(sym("x"))))),
                "(x!)!",
            ),
            (rem(sym("x"), rem(sym("y"), sym("z"))), "x%(y%z)"),
            (lt(sym("x"), lt(sym("y"), sym("z"))), "x < (y < z)"),
        ];

        for (expr, expected) in cases {
            assert_eq!(format_raw_expression(&expr), expected);
        }
    }

    #[test]
    fn equal_precedence_result_parentheses_preserve_tree_shape() {
        let cases = [
            (
                add(vec![add(vec![sym("x"), sym("y")]), sym("z")]),
                "(x + y) + z",
            ),
            (
                mul(vec![sym("x"), mul(vec![sym("y"), sym("z")])]),
                "x*(y*z)",
            ),
            (mul(vec![sym("x"), div(sym("y"), sym("z"))]), "x*(y / z)"),
            (div(sym("x"), div(sym("y"), sym("z"))), "x / (y / z)"),
            (pow(pow(sym("x"), sym("y")), sym("z")), "(x^y)^z"),
            (pow(sym("x"), pow(sym("y"), sym("z"))), "x^y^z"),
            (neg(pow(sym("x"), n(2))), "-x^2"),
            (Expression::Factorial(Box::new(neg(sym("x")))), "(-x)!"),
            (
                Expression::Factorial(Box::new(pow(sym("x"), n(2)))),
                "(x^2)!",
            ),
            (
                Expression::Factorial(Box::new(Expression::Factorial(Box::new(sym("x"))))),
                "(x!)!",
            ),
            (rem(sym("x"), rem(sym("y"), sym("z"))), "x%(y%z)"),
            (lt(sym("x"), lt(sym("y"), sym("z"))), "x < (y < z)"),
        ];

        for (expr, expected) in cases {
            assert_eq!(
                format_result_with_numbers(&expr, &Number::to_string),
                Some(expected.to_string())
            );
        }
    }

    #[test]
    fn vectors_matrices_functions_units_and_variables_format_by_name() {
        let vector = Expression::Vector(vec![sym("x"), sym("y")]);
        assert_eq!(format_raw_expression(&vector), "[x  y]");
        assert_eq!(
            format_result_with_numbers(&vector, &Number::to_string),
            Some("[x  y]".to_string())
        );

        let matrix = Expression::matrix(vec![vec![sym("x"), sym("y")], vec![n(1), n(2)]])
            .expect("valid matrix");
        assert_eq!(format_raw_expression(&matrix), "[x  y; 1  2]");
        assert_eq!(
            format_result_with_numbers(&matrix, &Number::to_string),
            Some("[x  y; 1  2]".to_string())
        );

        let function = Expression::FunctionCall {
            function: FunctionRef::new("sqrt"),
            args: vec![Expression::Addition(operands(vec![sym("x"), n(1)]))],
        };
        assert_eq!(format_raw_expression(&function), "sqrt(x + 1)");

        let unit = Expression::Multiplication(operands(vec![
            n(5),
            Expression::Unit {
                unit: UnitRef::new("m"),
                prefix: None,
                plural: false,
            },
        ]));
        assert_eq!(
            format_result_with_numbers(&unit, &Number::to_string),
            Some("5 m".to_string())
        );
        assert_eq!(format_raw_expression(&unit), "5 m");

        let prefixed_unit = Expression::Unit {
            unit: UnitRef::new("m"),
            prefix: Some(PrefixRef::new("k")),
            plural: false,
        };
        assert_eq!(format_raw_expression(&prefixed_unit), "km");

        let variable = Expression::Variable(VariableRef::new("stored_value"));
        assert_eq!(format_raw_expression(&variable), "stored_value");
    }

    fn atom_text() -> impl Strategy<Value = String> {
        prop_oneof![
            (0i32..=9).prop_map(|value| value.to_string()),
            prop::sample::select(&["a", "b", "c", "x", "y", "z"]).prop_map(str::to_string),
        ]
    }

    fn formatted_subset_text() -> impl Strategy<Value = String> {
        prop_oneof![
            atom_text(),
            (atom_text(), atom_text()).prop_map(|(a, b)| format!("{a} + {b}")),
            (atom_text(), atom_text()).prop_map(|(a, b)| format!("{a}*{b}")),
            (atom_text(), atom_text(), atom_text())
                .prop_map(|(a, b, c)| format!("{a}*({b} + {c})")),
            (atom_text(), atom_text()).prop_map(|(a, b)| format!("{a}/({b} + 1)")),
            (atom_text(), atom_text()).prop_map(|(a, b)| format!("({a} + {b})^2")),
            atom_text().prop_map(|a| format!("sqrt({a} + 1)")),
            (atom_text(), atom_text()).prop_map(|(a, b)| format!("{a} < {b} + 1")),
        ]
    }

    proptest! {
        #[test]
        fn raw_formatter_is_idempotent_for_constrained_precedence_subset(input in formatted_subset_text()) {
            let parsed = parse_expression(&input).expect("generated expression should parse");
            let formatted = format_raw_expression(&parsed);
            let reparsed = parse_expression(&formatted)
                .unwrap_or_else(|err| panic!("formatted expression {formatted:?} should parse: {err}"));
            prop_assert_eq!(format_raw_expression(&reparsed), formatted);
        }
    }
}
