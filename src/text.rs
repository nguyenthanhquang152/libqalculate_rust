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

fn get_term_degree(term: &Expression) -> f64 {
    match term {
        Expression::Number(_) => 0.0,
        Expression::Symbolic(_) | Expression::Variable(_) => 1.0,
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
        Expression::Multiplication(nary) => nary.as_slice().iter().any(is_term_negative),
        _ => false,
    }
}

fn get_absolute_term(term: &Expression) -> Expression {
    match term {
        Expression::Negate(inner) => *inner.clone(),
        Expression::Number(num) => Expression::Number(num.abs()),
        Expression::Multiplication(nary) => {
            let mut new_factors = Vec::new();
            let mut negated = false;
            for factor in nary.as_slice() {
                if !negated && is_term_negative(factor) {
                    new_factors.push(get_absolute_term(factor));
                    negated = true;
                } else {
                    new_factors.push(factor.clone());
                }
            }
            Expression::Multiplication(NaryChildren::new(new_factors).unwrap())
        }
        _ => term.clone(),
    }
}

fn needs_parens_in_division(expr: &Expression) -> bool {
    matches!(
        expr,
        Expression::Addition(_) | Expression::Multiplication(_)
    )
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

            if !terms.is_empty() && is_term_negative(&terms[0]) {
                if let Some(pos) = terms.iter().position(|t| !is_term_negative(t)) {
                    let first_pos = terms.remove(pos);
                    terms.insert(0, first_pos);
                }
            }

            let mut out = String::new();
            for (index, child) in terms.iter().enumerate() {
                if index == 0 {
                    let s = format_result_with_numbers(child, format_number)?;
                    out.push_str(&s);
                } else if is_term_negative(child) {
                    let abs_child = get_absolute_term(child);
                    let s = format_result_with_numbers(&abs_child, format_number)?;
                    out.push_str(" - ");
                    out.push_str(&s);
                } else {
                    let s = format_result_with_numbers(child, format_number)?;
                    out.push_str(" + ");
                    out.push_str(&s);
                }
            }
            Some(out)
        }
        Expression::Multiplication(children) => {
            let mut parts = Vec::new();
            for child in children.as_slice() {
                let s = format_result_with_numbers(child, format_number)?;
                parts.push((child, s));
            }
            let mut out = String::new();
            for i in 0..parts.len() {
                let (expr, s) = &parts[i];
                if i > 0 {
                    let (prev_expr, prev_s) = &parts[i - 1];
                    let need_sep = match (prev_expr, expr) {
                        (Expression::Number(_), Expression::Symbolic(_)) => false,
                        (Expression::Number(_), Expression::Variable(_)) => false,
                        (Expression::Number(_), Expression::Power { .. }) => false,
                        (Expression::Number(_), Expression::FunctionCall { .. }) => false,
                        (Expression::Symbolic(_), Expression::Symbolic(_)) => false,
                        (Expression::Symbolic(_), Expression::Variable(_)) => false,
                        (Expression::Variable(_), Expression::Symbolic(_)) => false,
                        (Expression::Variable(_), Expression::Variable(_)) => false,
                        (Expression::Power { .. }, Expression::Symbolic(_)) => false,
                        (Expression::Power { .. }, Expression::Variable(_)) => false,
                        _ => {
                            !(s.chars()
                                .next()
                                .is_some_and(|c| c.is_alphabetic() || c == '(')
                                && prev_s
                                    .chars()
                                    .last()
                                    .is_some_and(|c| c.is_alphanumeric() || c == ')'))
                        }
                    };
                    if need_sep {
                        out.push('*');
                    }
                }
                out.push_str(s);
            }
            Some(out)
        }
        Expression::Division {
            numerator,
            denominator,
        } => {
            let num_str = format_result_with_numbers(numerator, format_number)?;
            let den_str = format_result_with_numbers(denominator, format_number)?;
            let num_formatted = if needs_parens_in_division(numerator) {
                format!("({num_str})")
            } else {
                num_str
            };
            let den_formatted = if needs_parens_in_division(denominator) {
                format!("({den_str})")
            } else {
                den_str
            };
            Some(format!("{num_formatted} / {den_formatted}"))
        }
        Expression::Negate(child) => {
            let formatted = format_result_with_numbers(child, format_number)?;
            Some(format!("-{formatted}"))
        }
        Expression::Power { base, exponent } => {
            let base_str = format_result_with_numbers(base, format_number)?;
            let exp_str = format_result_with_numbers(exponent, format_number)?;
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
            let formatted = items
                .iter()
                .map(|item| format_result_with_numbers(item, format_number))
                .collect::<Option<Vec<_>>>()?;
            Some(format!("[{}]", formatted.join("  ")))
        }
        Expression::Remainder { lhs, rhs } => {
            let l = format_result_with_numbers(lhs, format_number)?;
            let r = format_result_with_numbers(rhs, format_number)?;
            Some(format!("{l}%{r}"))
        }
        Expression::Modulo { lhs, rhs } => {
            let l = format_result_with_numbers(lhs, format_number)?;
            let r = format_result_with_numbers(rhs, format_number)?;
            Some(format!("{l} mod {r}"))
        }
        Expression::IntegerDivision { lhs, rhs } => {
            let l = format_result_with_numbers(lhs, format_number)?;
            let r = format_result_with_numbers(rhs, format_number)?;
            Some(format!("{l}//{r}"))
        }
        Expression::ShiftLeft { lhs, rhs } => {
            let l = format_result_with_numbers(lhs, format_number)?;
            let r = format_result_with_numbers(rhs, format_number)?;
            Some(format!("{l}<<{r}"))
        }
        Expression::ShiftRight { lhs, rhs } => {
            let l = format_result_with_numbers(lhs, format_number)?;
            let r = format_result_with_numbers(rhs, format_number)?;
            Some(format!("{l}>>{r}"))
        }
        Expression::BitwiseAnd(children) => {
            let formatted = children
                .as_slice()
                .iter()
                .map(|item| format_result_with_numbers(item, format_number))
                .collect::<Option<Vec<_>>>()?;
            Some(formatted.join("&"))
        }
        Expression::BitwiseOr(children) => {
            let formatted = children
                .as_slice()
                .iter()
                .map(|item| format_result_with_numbers(item, format_number))
                .collect::<Option<Vec<_>>>()?;
            Some(formatted.join("|"))
        }
        Expression::BitwiseXor(children) => {
            let formatted = children
                .as_slice()
                .iter()
                .map(|item| format_result_with_numbers(item, format_number))
                .collect::<Option<Vec<_>>>()?;
            Some(formatted.join(" xor "))
        }
        Expression::BitwiseNot(child) => {
            let formatted = format_result_with_numbers(child, format_number)?;
            Some(format!("~{formatted}"))
        }
        Expression::LogicalAnd(children) => {
            let formatted = children
                .as_slice()
                .iter()
                .map(|item| format_result_with_numbers(item, format_number))
                .collect::<Option<Vec<_>>>()?;
            Some(formatted.join(" and "))
        }
        Expression::LogicalOr(children) => {
            let formatted = children
                .as_slice()
                .iter()
                .map(|item| format_result_with_numbers(item, format_number))
                .collect::<Option<Vec<_>>>()?;
            Some(formatted.join(" or "))
        }
        Expression::LogicalXor { lhs, rhs } => {
            let l = format_result_with_numbers(lhs, format_number)?;
            let r = format_result_with_numbers(rhs, format_number)?;
            Some(format!("{l} xor {r}"))
        }
        Expression::LogicalNot(child) => {
            let formatted = format_result_with_numbers(child, format_number)?;
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
            let l = format_result_with_numbers(lhs, format_number)?;
            let r = format_result_with_numbers(rhs, format_number)?;
            Some(format!("{l}{op_str}{r}"))
        }
        Expression::Conversion { expr, target } => {
            let e = format_result_with_numbers(expr, format_number)?;
            let t = format_result_with_numbers(target, format_number)?;
            Some(format!("{e} to {t}"))
        }
        Expression::Assignment { variable, value } => {
            let val = format_result_with_numbers(value, format_number)?;
            Some(format!("{variable}:={val}"))
        }
        Expression::Inverse(child) => {
            let formatted = format_result_with_numbers(child, format_number)?;
            Some(format!("1/{formatted}"))
        }
        Expression::Factorial(child) => {
            let formatted = format_result_with_numbers(child, format_number)?;
            Some(format!("{formatted}!"))
        }
        Expression::DoubleFactorial(child) => {
            let formatted = format_result_with_numbers(child, format_number)?;
            Some(format!("{formatted}!!"))
        }
        Expression::MultiFactorial { expr, count } => {
            let formatted = format_result_with_numbers(expr, format_number)?;
            Some(format!("{}{}", formatted, "!".repeat(*count as usize)))
        }
        Expression::Percent(child) => {
            let formatted = format_result_with_numbers(child, format_number)?;
            Some(format!("{formatted}%"))
        }
        Expression::Parallel { lhs, rhs } => {
            let l = format_result_with_numbers(lhs, format_number)?;
            let r = format_result_with_numbers(rhs, format_number)?;
            Some(format!("{l} parallel {r}"))
        }
        Expression::Undefined => Some("undefined".to_string()),
        Expression::Aborted => Some("aborted".to_string()),
        Expression::DateTime(value) => Some(value.source().to_string()),
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
        Expression::Multiplication(children) => {
            let mut parts = Vec::new();
            for child in children.as_slice() {
                let s = format_raw_expression(child);
                parts.push((child, s));
            }
            let mut out = String::new();
            for i in 0..parts.len() {
                let (expr, s) = &parts[i];
                if i > 0 {
                    let (prev_expr, prev_s) = &parts[i - 1];
                    let need_sep = match (prev_expr, expr) {
                        (Expression::Number(_), Expression::Symbolic(_)) => false,
                        (Expression::Number(_), Expression::Variable(_)) => false,
                        (Expression::Number(_), Expression::Power { .. }) => false,
                        (Expression::Number(_), Expression::FunctionCall { .. }) => false,
                        (Expression::Symbolic(_), Expression::Symbolic(_)) => false,
                        (Expression::Symbolic(_), Expression::Variable(_)) => false,
                        (Expression::Variable(_), Expression::Symbolic(_)) => false,
                        (Expression::Variable(_), Expression::Variable(_)) => false,
                        (Expression::Power { .. }, Expression::Symbolic(_)) => false,
                        (Expression::Power { .. }, Expression::Variable(_)) => false,
                        _ => {
                            !(s.chars()
                                .next()
                                .is_some_and(|c| c.is_alphabetic() || c == '(')
                                && prev_s
                                    .chars()
                                    .last()
                                    .is_some_and(|c| c.is_alphanumeric() || c == ')'))
                        }
                    };
                    if need_sep {
                        out.push('*');
                    }
                }
                out.push_str(s);
            }
            out
        }
        Expression::Division {
            numerator,
            denominator,
        } => {
            let num_str = format_raw_expression(numerator);
            let den_str = format_raw_expression(denominator);
            let num_formatted = if needs_parens_in_division(numerator) {
                format!("({num_str})")
            } else {
                num_str
            };
            let den_formatted = if needs_parens_in_division(denominator) {
                format!("({den_str})")
            } else {
                den_str
            };
            format!("{num_formatted}/{den_formatted}")
        }
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
    let mut terms = children.as_slice().to_vec();
    terms.sort_by(|a, b| {
        let deg_a = get_term_degree(a);
        let deg_b = get_term_degree(b);
        deg_b
            .partial_cmp(&deg_a)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| format_raw_expression(a).cmp(&format_raw_expression(b)))
    });

    if !terms.is_empty() && is_term_negative(&terms[0]) {
        if let Some(pos) = terms.iter().position(|t| !is_term_negative(t)) {
            let first_pos = terms.remove(pos);
            terms.insert(0, first_pos);
        }
    }

    let mut out = String::new();
    for (index, child) in terms.iter().enumerate() {
        if index == 0 {
            out.push_str(&format_raw_expression(child));
        } else if is_term_negative(child) {
            let abs_child = get_absolute_term(child);
            out.push_str(" - ");
            out.push_str(&format_raw_expression(&abs_child));
        } else {
            out.push_str(" + ");
            out.push_str(&format_raw_expression(child));
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
