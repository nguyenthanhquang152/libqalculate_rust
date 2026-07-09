//! LaTeX and HTML markup formatting for expression output modes.

use crate::ast::{Expression, NaryChildren};
use crate::number::{Number, NumberValue};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MarkupMode {
    Latex,
    Html,
}

pub(crate) fn format_markup_expression(expr: &Expression, mode: MarkupMode) -> Option<String> {
    match expr {
        Expression::Number(num) => Some(format_markup_number(num, mode)),
        Expression::Text(text) => Some(match mode {
            MarkupMode::Latex => format!("\\text{{{}}}", escape_latex(text)),
            MarkupMode::Html => escape_html(text),
        }),
        Expression::Symbolic(symbol) => Some(format_markup_identifier(symbol.name(), mode, true)),
        Expression::Variable(variable) => Some(format_markup_identifier(variable.id(), mode, true)),
        Expression::Unit { unit, prefix, .. } => {
            let mut out = String::new();
            if let Some(prefix) = prefix {
                out.push_str(prefix.id());
            }
            out.push_str(unit.id());
            Some(format_markup_identifier(&out, mode, false))
        }
        Expression::Addition(children) => children
            .as_slice()
            .iter()
            .map(|child| format_markup_expression(child, mode))
            .collect::<Option<Vec<_>>>()
            .map(|parts| parts.join(" + ")),
        Expression::Multiplication(children) => children
            .as_slice()
            .iter()
            .map(|child| format_markup_expression(child, mode))
            .collect::<Option<Vec<_>>>()
            .map(|parts| parts.join(" ")),
        Expression::Division {
            numerator,
            denominator,
        } => {
            let numerator = format_markup_expression(numerator, mode)?;
            let denominator = format_markup_expression(denominator, mode)?;
            Some(match mode {
                MarkupMode::Latex => format!("\\frac{{{numerator}}}{{{denominator}}}"),
                MarkupMode::Html => format!("{numerator} / {denominator}"),
            })
        }
        Expression::Negate(child) => {
            let child = format_markup_expression(child, mode)?;
            Some(format!("-{child}"))
        }
        Expression::Power { base, exponent } => {
            let base = format_markup_expression(base, mode)?;
            let exponent = format_markup_expression(exponent, mode)?;
            Some(match mode {
                MarkupMode::Latex => format!("{base}^{{{exponent}}}"),
                MarkupMode::Html => format!("{base}<sup>{exponent}</sup>"),
            })
        }
        Expression::FunctionCall { function, args } => {
            if function.id() == "sqrt" && args.len() == 1 {
                let arg = format_markup_expression(&args[0], mode)?;
                return Some(match mode {
                    MarkupMode::Latex => format!("\\sqrt{{{arg}}}"),
                    MarkupMode::Html => format!("√({arg})"),
                });
            }

            let args = args
                .iter()
                .map(|arg| format_markup_expression(arg, mode))
                .collect::<Option<Vec<_>>>()?
                .join(", ");
            let name = format_markup_identifier(function.id(), mode, false);
            Some(format!("{name}({args})"))
        }
        Expression::Vector(items) => {
            if let Some(rows) = expr.as_matrix_rows() {
                let rows = rows
                    .iter()
                    .map(|row| {
                        row.iter()
                            .map(|cell| format_markup_expression(cell, mode))
                            .collect::<Option<Vec<_>>>()
                    })
                    .collect::<Option<Vec<_>>>()?;
                Some(match mode {
                    MarkupMode::Latex => {
                        let rows = rows
                            .iter()
                            .map(|row| row.join(" & "))
                            .collect::<Vec<_>>()
                            .join(" \\\\ ");
                        format!("\\begin{{bmatrix}}{rows}\\end{{bmatrix}}")
                    }
                    MarkupMode::Html => {
                        let rows = rows
                            .iter()
                            .map(|row| row.join("&nbsp; "))
                            .collect::<Vec<_>>()
                            .join("; ");
                        format!("[{rows}]")
                    }
                })
            } else {
                let items = items
                    .iter()
                    .map(|item| format_markup_expression(item, mode))
                    .collect::<Option<Vec<_>>>()?;
                Some(format!("[{}]", items.join("  ")))
            }
        }
        Expression::Remainder { lhs, rhs } => format_markup_binary(lhs, "%", rhs, mode, false),
        Expression::Modulo { lhs, rhs } => format_markup_binary(lhs, " mod ", rhs, mode, false),
        Expression::IntegerDivision { lhs, rhs } => {
            format_markup_binary(lhs, "//", rhs, mode, false)
        }
        Expression::ShiftLeft { lhs, rhs } => format_markup_binary(lhs, "<<", rhs, mode, false),
        Expression::ShiftRight { lhs, rhs } => format_markup_binary(lhs, ">>", rhs, mode, false),
        Expression::BitwiseAnd(children) => format_markup_nary(children, "&", mode),
        Expression::BitwiseOr(children) => format_markup_nary(children, "|", mode),
        Expression::BitwiseXor(children) => format_markup_nary(children, " xor ", mode),
        Expression::BitwiseNot(child) => {
            let child = format_markup_expression(child, mode)?;
            Some(format!("~{child}"))
        }
        Expression::LogicalAnd(children) => format_markup_nary(children, " and ", mode),
        Expression::LogicalOr(children) => format_markup_nary(children, " or ", mode),
        Expression::LogicalXor { lhs, rhs } => format_markup_binary(lhs, " xor ", rhs, mode, false),
        Expression::LogicalNot(child) => {
            let child = format_markup_expression(child, mode)?;
            Some(format!("not {child}"))
        }
        Expression::Comparison { op, lhs, rhs } => {
            let op = match (op, mode) {
                (crate::ast::ComparisonOperator::Equal, _) => " = ",
                (crate::ast::ComparisonOperator::NotEqual, MarkupMode::Latex) => " \\ne ",
                (crate::ast::ComparisonOperator::NotEqual, MarkupMode::Html) => " != ",
                (crate::ast::ComparisonOperator::Less, MarkupMode::Latex) => " < ",
                (crate::ast::ComparisonOperator::Less, MarkupMode::Html) => " &lt; ",
                (crate::ast::ComparisonOperator::LessOrEqual, MarkupMode::Latex) => " \\le ",
                (crate::ast::ComparisonOperator::LessOrEqual, MarkupMode::Html) => " &lt;= ",
                (crate::ast::ComparisonOperator::Greater, MarkupMode::Latex) => " > ",
                (crate::ast::ComparisonOperator::Greater, MarkupMode::Html) => " &gt; ",
                (crate::ast::ComparisonOperator::GreaterOrEqual, MarkupMode::Latex) => " \\ge ",
                (crate::ast::ComparisonOperator::GreaterOrEqual, MarkupMode::Html) => " &gt;= ",
            };
            format_markup_binary(lhs, op, rhs, mode, mode == MarkupMode::Html)
        }
        Expression::Conversion { expr, target } => {
            format_markup_binary(expr, " to ", target, mode, false)
        }
        Expression::Assignment { variable, value } => {
            let value = format_markup_expression(value, mode)?;
            Some(format!(
                "{}:={value}",
                format_markup_identifier(variable, mode, true)
            ))
        }
        Expression::Inverse(child) => {
            let one = Expression::Number(Number::from_i32(1));
            format_markup_binary(&one, "/", child, mode, false)
        }
        Expression::Factorial(child) => {
            let child = format_markup_expression(child, mode)?;
            Some(format!("{child}!"))
        }
        Expression::DoubleFactorial(child) => {
            let child = format_markup_expression(child, mode)?;
            Some(format!("{child}!!"))
        }
        Expression::MultiFactorial { expr, count } => {
            let child = format_markup_expression(expr, mode)?;
            Some(format!("{}{}", child, "!".repeat(*count as usize)))
        }
        Expression::Percent(child) => {
            let child = format_markup_expression(child, mode)?;
            Some(format!("{child}%"))
        }
        Expression::Parallel { lhs, rhs } => {
            format_markup_binary(lhs, " parallel ", rhs, mode, false)
        }
        Expression::Undefined => Some("undefined".to_string()),
        Expression::Aborted => Some("aborted".to_string()),
        Expression::DateTime(value) => Some(match mode {
            MarkupMode::Latex => format!("\\text{{{}}}", escape_latex(value.source())),
            MarkupMode::Html => escape_html(value.source()),
        }),
    }
}

pub(crate) fn format_markup_equation<F>(
    parsed: &Expression,
    evaluated: &Expression,
    mode: MarkupMode,
    format_number: &F,
) -> Option<String>
where
    F: Fn(&Number) -> String,
{
    let lhs = format_markup_expression(parsed, mode)?;
    let rhs = format_markup_result_expression(evaluated, mode, format_number)?;
    let approximate = markup_result_is_approximate(evaluated);

    Some(match mode {
        MarkupMode::Latex => {
            if approximate {
                format!("$\\displaystyle {lhs} \\approx \\num{{{rhs}}}$")
            } else {
                format!("$\\displaystyle {lhs} = {rhs}$")
            }
        }
        MarkupMode::Html => {
            if approximate {
                format!("{lhs} ≈ {rhs}")
            } else {
                format!("{lhs} = {rhs}")
            }
        }
    })
}

fn format_markup_result_expression<F>(
    expr: &Expression,
    mode: MarkupMode,
    format_number: &F,
) -> Option<String>
where
    F: Fn(&Number) -> String,
{
    match expr {
        Expression::Number(num) => Some(match mode {
            MarkupMode::Latex => escape_latex(&format_number(num)),
            MarkupMode::Html => escape_html(&format_number(num)),
        }),
        _ => format_markup_expression(expr, mode),
    }
}

fn markup_result_is_approximate(expr: &Expression) -> bool {
    matches!(expr, Expression::Number(num) if num.approximate())
}

fn format_markup_number(num: &Number, mode: MarkupMode) -> String {
    match (num.value(), mode) {
        (NumberValue::Rational(rational), MarkupMode::Latex)
            if rational.denominator_string() != "1" =>
        {
            format!(
                "\\frac{{{}}}{{{}}}",
                rational.numerator_string(),
                rational.denominator_string()
            )
        }
        _ => match mode {
            MarkupMode::Latex => escape_latex(&num.to_string()),
            MarkupMode::Html => escape_html(&num.to_string()),
        },
    }
}

fn format_markup_identifier(name: &str, mode: MarkupMode, italic: bool) -> String {
    match mode {
        MarkupMode::Latex => escape_latex_identifier(name),
        MarkupMode::Html if italic => format!("<i>{}</i>", escape_html(name)),
        MarkupMode::Html => escape_html(name),
    }
}

fn format_markup_binary(
    lhs: &Expression,
    op: &str,
    rhs: &Expression,
    mode: MarkupMode,
    wrap: bool,
) -> Option<String> {
    let lhs = format_markup_expression(lhs, mode)?;
    let rhs = format_markup_expression(rhs, mode)?;
    let text = format!("{lhs}{op}{rhs}");
    Some(if wrap { format!("({text})") } else { text })
}

fn format_markup_nary(children: &NaryChildren, op: &str, mode: MarkupMode) -> Option<String> {
    let op = if mode == MarkupMode::Html {
        escape_html(op)
    } else {
        op.to_string()
    };
    children
        .as_slice()
        .iter()
        .map(|child| format_markup_expression(child, mode))
        .collect::<Option<Vec<_>>>()
        .map(|parts| parts.join(&op))
}

fn escape_html(text: &str) -> String {
    let mut escaped = String::new();
    for ch in text.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn escape_latex_identifier(name: &str) -> String {
    let mut out = String::new();
    for (index, part) in name.split('_').enumerate() {
        if index == 0 {
            out.push_str(&escape_latex(part));
        } else {
            out.push_str("_{");
            out.push_str(&escape_latex(part));
            out.push('}');
        }
    }
    out
}

fn escape_latex(text: &str) -> String {
    let mut escaped = String::new();
    for ch in text.chars() {
        match ch {
            '\\' => escaped.push_str("\\backslash{}"),
            '{' => escaped.push_str("\\{"),
            '}' => escaped.push_str("\\}"),
            '&' => escaped.push_str("\\&"),
            '%' => escaped.push_str("\\%"),
            '$' => escaped.push_str("\\$"),
            '#' => escaped.push_str("\\#"),
            '_' => escaped.push_str("\\_"),
            '^' => escaped.push_str("\\^{}"),
            '~' => escaped.push_str("\\~{}"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::{format_markup_equation, format_markup_expression, MarkupMode};
    use crate::ast::{ComparisonOperator, Expression, FunctionRef, NaryChildren, Symbol, UnitRef};
    use crate::number::{Number, Rational};
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

    fn lt(lhs: Expression, rhs: Expression) -> Expression {
        Expression::Comparison {
            op: ComparisonOperator::Less,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
    }

    #[test]
    fn latex_markup_formats_fraction_power_root_and_matrix_forms() {
        let fraction = div(n(1), n(2));
        assert_eq!(
            format_markup_expression(&fraction, MarkupMode::Latex),
            Some("\\frac{1}{2}".to_string())
        );

        let power = pow(sym("x"), n(2));
        assert_eq!(
            format_markup_expression(&power, MarkupMode::Latex),
            Some("x^{2}".to_string())
        );

        let root = Expression::FunctionCall {
            function: FunctionRef::new("sqrt"),
            args: vec![add(vec![sym("x"), n(1)])],
        };
        assert_eq!(
            format_markup_expression(&root, MarkupMode::Latex),
            Some("\\sqrt{x + 1}".to_string())
        );

        let matrix =
            Expression::matrix(vec![vec![n(1), n(2)], vec![n(3), n(4)]]).expect("valid matrix");
        assert_eq!(
            format_markup_expression(&matrix, MarkupMode::Latex),
            Some("\\begin{bmatrix}1 & 2 \\\\ 3 & 4\\end{bmatrix}".to_string())
        );
    }

    #[test]
    fn html_markup_escapes_comparisons_variables_powers_and_matrices() {
        let comparison = lt(sym("x"), sym("y"));
        assert_eq!(
            format_markup_expression(&comparison, MarkupMode::Html),
            Some("(<i>x</i> &lt; <i>y</i>)".to_string())
        );

        let power = pow(sym("x"), n(2));
        assert_eq!(
            format_markup_expression(&power, MarkupMode::Html),
            Some("<i>x</i><sup>2</sup>".to_string())
        );

        let root = Expression::FunctionCall {
            function: FunctionRef::new("sqrt"),
            args: vec![n(2)],
        };
        assert_eq!(
            format_markup_expression(&root, MarkupMode::Html),
            Some("√(2)".to_string())
        );

        let matrix =
            Expression::matrix(vec![vec![n(1), n(2)], vec![n(3), n(4)]]).expect("valid matrix");
        assert_eq!(
            format_markup_expression(&matrix, MarkupMode::Html),
            Some("[1&nbsp; 2; 3&nbsp; 4]".to_string())
        );
    }

    #[test]
    fn markup_equation_formats_parsed_and_approximate_result_forms() {
        let parsed = parse_expression("1/2 + sqrt(2)").expect("expression parses");
        let evaluated = Expression::Number(Number::from_f64(1.914_213_562));
        let format_number = |_: &Number| "1.914213562".to_string();

        assert_eq!(
            format_markup_equation(&parsed, &evaluated, MarkupMode::Latex, &format_number),
            Some(
                "$\\displaystyle \\frac{1}{2} + \\sqrt{2} \\approx \\num{1.914213562}$".to_string()
            )
        );
        assert_eq!(
            format_markup_equation(&parsed, &evaluated, MarkupMode::Html, &format_number),
            Some("1 / 2 + √(2) ≈ 1.914213562".to_string())
        );
    }

    #[test]
    fn markup_equation_uses_equals_for_exact_results() {
        let parsed = parse_expression("1 + 1").expect("expression parses");
        let evaluated = n(2);
        let format_number = |number: &Number| number.to_string();

        assert_eq!(
            format_markup_equation(&parsed, &evaluated, MarkupMode::Latex, &format_number),
            Some("$\\displaystyle 1 + 1 = 2$".to_string())
        );
        assert_eq!(
            format_markup_equation(&parsed, &evaluated, MarkupMode::Html, &format_number),
            Some("1 + 1 = 2".to_string())
        );
    }

    #[test]
    fn markup_formats_generic_functions_units_and_nary_html_operators() {
        let generic_function = Expression::FunctionCall {
            function: FunctionRef::new("sin"),
            args: vec![n(2)],
        };
        assert_eq!(
            format_markup_expression(&generic_function, MarkupMode::Latex),
            Some("sin(2)".to_string())
        );
        assert_eq!(
            format_markup_expression(&generic_function, MarkupMode::Html),
            Some("sin(2)".to_string())
        );

        let unit = Expression::Unit {
            unit: UnitRef::new("m&<>"),
            prefix: None,
            plural: false,
        };
        assert_eq!(
            format_markup_expression(&unit, MarkupMode::Html),
            Some("m&amp;&lt;&gt;".to_string())
        );

        let bitwise_and = Expression::BitwiseAnd(operands(vec![n(1), n(2), n(3)]));
        assert_eq!(
            format_markup_expression(&bitwise_and, MarkupMode::Html),
            Some("1&amp;2&amp;3".to_string())
        );
    }

    #[test]
    fn latex_markup_formats_native_rational_numbers_and_escapes_reserved_text() {
        let rational = Expression::Number(Number::from_rational(Rational::new(1, 2)));
        assert_eq!(
            format_markup_expression(&rational, MarkupMode::Latex),
            Some("\\frac{1}{2}".to_string())
        );

        let text = Expression::Text(r"\{}&%$#_^~".to_string());
        assert_eq!(
            format_markup_expression(&text, MarkupMode::Latex),
            Some(r"\text{\backslash{}\{\}\&\%\$\#\_\^{}\~{}}".to_string())
        );
    }

    #[test]
    fn html_markup_escapes_ampersands_in_text() {
        let text = Expression::Text("a&<>\"'".to_string());
        assert_eq!(
            format_markup_expression(&text, MarkupMode::Html),
            Some("a&amp;&lt;&gt;&quot;&#39;".to_string())
        );
    }

    proptest! {
        #[test]
        fn html_text_markup_escapes_angle_brackets_and_quotes(input in ".*") {
            let expr = Expression::Text(input);
            let formatted = format_markup_expression(&expr, MarkupMode::Html)
                .expect("text markup should format");

            prop_assert!(!formatted.contains('<'));
            prop_assert!(!formatted.contains('>'));
            prop_assert!(!formatted.contains('"'));
            prop_assert!(!formatted.contains('\''));
        }
    }
}
