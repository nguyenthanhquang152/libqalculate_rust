//! Native vector and matrix helpers.
//!
//! Upstream oracle:
//! - `../libqalculate/libqalculate/MathStructure-matrixvector.cc`
//! - `../libqalculate/libqalculate/Calculator-parse.cc`
//! - `../libqalculate/libqalculate/MathStructure-print.cc`

use crate::ast::Expression;
use crate::number::Number;
use std::str::FromStr;

/// Parses the vector/matrix literal subset promoted by issue #41.
pub(crate) fn parse_collection_literal(input: &str) -> Option<Expression> {
    let mut parser = CollectionParser::new(input);
    let expr = parser.parse_top_level()?;
    parser.skip_ws();
    (parser.is_at_end() && parser.saw_collection && !has_ragged_nested_vectors(&expr))
        .then_some(expr)
}

/// Parses and evaluates the vector/matrix function subset promoted by issue #41.
pub(crate) fn evaluate_collection_function(input: &str) -> Option<Expression> {
    let (name, args_source) = split_function_call(input)?;
    let args = parse_arguments(args_source)?;
    if args.iter().any(has_ragged_nested_vectors) {
        return None;
    }
    match name {
        "vector" => Some(Expression::Vector(args)),
        "matrix" => build_matrix(&args),
        "matrix2vector" => matrix_to_vector(&args),
        "columns" => {
            let count = columns(args.first()?)?;
            Some(Expression::Number(Number::from_i64(count as i64)))
        }
        "elements" => {
            let count = elements(args.first()?)?;
            Some(Expression::Number(Number::from_i64(count as i64)))
        }
        "element" => element(&args),
        "dimension" if args.len() == 1 => {
            let count = dimension(args.first()?)?;
            Some(Expression::Number(Number::from_i64(count as i64)))
        }
        "rows" if args.len() == 1 => {
            let count = rows_count(args.first()?)?;
            Some(Expression::Number(Number::from_i64(count as i64)))
        }
        "row" if args.len() == 2 => {
            let collection = args.first()?;
            let row_idx = number_to_i64(args.get(1)?)?;
            row(collection, row_idx)
        }
        "column" if args.len() == 2 => {
            let collection = args.first()?;
            let col_idx = number_to_i64(args.get(1)?)?;
            column(collection, col_idx)
        }
        _ => None,
    }
}

/// Parses and evaluates the vector/matrix arithmetic subset promoted by issue #41.
pub(crate) fn evaluate_collection_arithmetic(input: &str) -> Option<Expression> {
    let mut parser = CollectionParser::new(input);
    let expr = parser.parse_add_sub_expression()?;
    parser.skip_ws();
    (parser.is_at_end() && parser.saw_collection && !has_ragged_nested_vectors(&expr))
        .then_some(expr)
}

struct CollectionParser<'a> {
    input: &'a str,
    position: usize,
    saw_collection: bool,
}

impl<'a> CollectionParser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            position: 0,
            saw_collection: false,
        }
    }

    fn parse_top_level(&mut self) -> Option<Expression> {
        self.skip_ws();
        let mut items = vec![self.parse_value()?];
        self.skip_ws();
        while self.consume_delimiter().is_some() {
            self.saw_collection = true;
            self.skip_ws();
            if self.is_at_end() {
                items.push(zero());
                break;
            }
            items.push(self.parse_value()?);
            self.skip_ws();
        }

        if items.len() == 1 {
            items.pop()
        } else {
            Some(Expression::Vector(items))
        }
    }

    fn parse_add_sub_expression(&mut self) -> Option<Expression> {
        let mut lhs = self.parse_multiplicative_expression()?;
        loop {
            self.skip_ws();
            if self.consume_operator("+") {
                let rhs = self.parse_multiplicative_expression()?;
                lhs = add_collections(lhs, rhs)?;
            } else if self.consume_operator("-") {
                let rhs = self.parse_multiplicative_expression()?;
                lhs = sub_collections(lhs, rhs)?;
            } else {
                break;
            }
        }
        Some(lhs)
    }

    fn parse_multiplicative_expression(&mut self) -> Option<Expression> {
        let mut lhs = self.parse_value()?;
        loop {
            self.skip_ws();
            if self.consume_operator(".*") {
                let rhs = self.parse_value()?;
                lhs = elementwise_mul_collections(lhs, rhs)?;
            } else if self.consume_operator("*") {
                let rhs = self.parse_value()?;
                lhs = multiply_collections(lhs, rhs)?;
            } else {
                break;
            }
        }
        Some(lhs)
    }

    fn parse_value(&mut self) -> Option<Expression> {
        self.skip_ws();
        match self.peek_char()? {
            '[' => self.parse_container('[', ']'),
            '(' => self.parse_container('(', ')'),
            ',' | ';' | ']' | ')' => Some(zero()),
            _ => self.parse_number(),
        }
    }

    fn parse_container(&mut self, open: char, close: char) -> Option<Expression> {
        self.consume_char(open)?;
        let mut is_collection = open == '[';
        if is_collection {
            self.saw_collection = true;
        }
        self.skip_ws();

        let mut rows = vec![Vec::new()];
        let mut saw_semicolon = false;
        if self.consume_char(close).is_some() {
            return Some(Expression::Vector(Vec::new()));
        }

        loop {
            self.skip_ws();
            if self.consume_char(close).is_some() {
                rows.last_mut()?.push(zero());
                break;
            }

            if self.peek_char().is_some_and(|ch| ch == ',' || ch == ';') {
                is_collection = true;
                rows.last_mut()?.push(zero());
            } else {
                rows.last_mut()?.push(self.parse_value()?);
            }

            self.skip_ws();
            if self.consume_char(close).is_some() {
                break;
            }

            if let Some(delimiter) = self.consume_delimiter() {
                is_collection = true;
                if delimiter == ';' && open == '[' {
                    saw_semicolon = true;
                    rows.push(Vec::new());
                }
            } else if self.peek_char().is_some_and(|ch| ch != close) {
                // qalc matrix literals also accept whitespace between row items.
                continue;
            } else {
                return None;
            }
        }

        if open == '(' && !is_collection && rows.len() == 1 && rows[0].len() == 1 {
            return rows.pop()?.pop();
        }

        self.saw_collection = true;
        if saw_semicolon {
            Some(Expression::Vector(
                rows.into_iter().map(Expression::Vector).collect(),
            ))
        } else {
            Some(Expression::Vector(rows.pop()?))
        }
    }

    fn parse_number(&mut self) -> Option<Expression> {
        let start = self.position;
        while let Some(ch) = self.peek_char() {
            if ch.is_whitespace() || matches!(ch, ',' | ';' | ']' | ')') {
                break;
            }
            self.position += ch.len_utf8();
        }

        let text = self.input[start..self.position].trim();
        if text.is_empty() {
            return Some(zero());
        }

        Number::from_str(text).ok().map(Expression::Number)
    }

    fn consume_delimiter(&mut self) -> Option<char> {
        self.skip_ws();
        let ch = self.peek_char()?;
        if matches!(ch, ',' | ';') {
            self.position += ch.len_utf8();
            Some(ch)
        } else {
            None
        }
    }

    fn consume_char(&mut self, expected: char) -> Option<()> {
        self.skip_ws();
        let ch = self.peek_char()?;
        if ch == expected {
            self.position += ch.len_utf8();
            Some(())
        } else {
            None
        }
    }

    fn consume_operator(&mut self, expected: &str) -> bool {
        self.skip_ws();
        if self.input[self.position..].starts_with(expected) {
            self.position += expected.len();
            true
        } else {
            false
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.position..].chars().next()
    }

    fn skip_ws(&mut self) {
        while let Some(ch) = self.peek_char() {
            if !ch.is_whitespace() {
                break;
            }
            self.position += ch.len_utf8();
        }
    }

    fn is_at_end(&self) -> bool {
        self.position >= self.input.len()
    }
}

fn zero() -> Expression {
    Expression::Number(Number::from_i32(0))
}

fn split_function_call(input: &str) -> Option<(&str, &str)> {
    let trimmed = input.trim();
    let open = trimmed.find('(')?;
    let name = trimmed[..open].trim();
    if name.is_empty()
        || !name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return None;
    }
    let close = trimmed.rfind(')')?;
    (close == trimmed.len() - 1).then_some((name, &trimmed[open + 1..close]))
}

fn parse_arguments(input: &str) -> Option<Vec<Expression>> {
    let mut parser = CollectionParser::new(input);
    let mut args = Vec::new();
    parser.skip_ws();
    if parser.is_at_end() {
        return Some(args);
    }
    loop {
        if parser.peek_char().is_some_and(|ch| ch == ',' || ch == ';') {
            args.push(zero());
        } else {
            args.push(parser.parse_value()?);
        }
        parser.skip_ws();
        if parser.is_at_end() {
            break;
        }
        parser.consume_delimiter()?;
        parser.skip_ws();
        if parser.is_at_end() {
            args.push(zero());
            break;
        }
    }
    Some(args)
}

fn build_matrix(args: &[Expression]) -> Option<Expression> {
    let rows = number_to_usize(args.first()?)?;
    let cols = number_to_usize(args.get(1)?)?;
    if rows == 0 || cols == 0 {
        return None;
    }
    let total = rows.checked_mul(cols)?;
    let mut values = args
        .iter()
        .skip(2)
        .flat_map(flatten_owned)
        .collect::<Vec<_>>();
    values.resize_with(total, zero);
    values.truncate(total);

    if total == 1 {
        return values.into_iter().next();
    }

    let matrix_rows = values
        .chunks(cols)
        .map(|row| Expression::Vector(row.to_vec()))
        .collect::<Vec<_>>();
    Some(Expression::Vector(matrix_rows))
}

fn add_collections(lhs: Expression, rhs: Expression) -> Option<Expression> {
    binary_elementwise(lhs, rhs, Number::add)
}

fn sub_collections(lhs: Expression, rhs: Expression) -> Option<Expression> {
    binary_elementwise(lhs, rhs, Number::sub)
}

fn elementwise_mul_collections(lhs: Expression, rhs: Expression) -> Option<Expression> {
    binary_elementwise(lhs, rhs, Number::mul)
}

fn multiply_collections(lhs: Expression, rhs: Expression) -> Option<Expression> {
    match (collection_shape(&lhs)?, collection_shape(&rhs)?) {
        (CollectionShape::Scalar, CollectionShape::Scalar) => {
            let lhs = as_number(&lhs)?;
            let rhs = as_number(&rhs)?;
            Some(Expression::Number(lhs.mul(rhs)))
        }
        (CollectionShape::Scalar, _) => {
            map_numbers(&rhs, &|number| Some(as_number(&lhs)?.mul(number)))
        }
        (_, CollectionShape::Scalar) => {
            map_numbers(&lhs, &|number| Some(number.mul(as_number(&rhs)?)))
        }
        (CollectionShape::Matrix { .. }, CollectionShape::Matrix { .. }) => {
            matrix_multiply(&lhs, &rhs)
        }
        _ => None,
    }
}

fn binary_elementwise(
    lhs: Expression,
    rhs: Expression,
    op: fn(&Number, &Number) -> Number,
) -> Option<Expression> {
    match (collection_shape(&lhs)?, collection_shape(&rhs)?) {
        (CollectionShape::Scalar, CollectionShape::Scalar) => {
            Some(Expression::Number(op(as_number(&lhs)?, as_number(&rhs)?)))
        }
        (CollectionShape::Scalar, _) => {
            let scalar = as_number(&lhs)?;
            map_numbers(&rhs, &|number| Some(op(scalar, number)))
        }
        (_, CollectionShape::Scalar) => {
            let scalar = as_number(&rhs)?;
            map_numbers(&lhs, &|number| Some(op(number, scalar)))
        }
        (lhs_shape, rhs_shape) if lhs_shape == rhs_shape => zip_numbers(&lhs, &rhs, op),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CollectionShape {
    Scalar,
    Vector { len: usize },
    Matrix { rows: usize, cols: usize },
}

fn collection_shape(expr: &Expression) -> Option<CollectionShape> {
    match expr {
        Expression::Number(_) => Some(CollectionShape::Scalar),
        Expression::Vector(items) if is_rectangular_matrix(expr) => {
            let cols = vector_len(items.first()?)?;
            Some(CollectionShape::Matrix {
                rows: items.len(),
                cols,
            })
        }
        Expression::Vector(items)
            if items
                .iter()
                .all(|item| matches!(item, Expression::Number(_))) =>
        {
            Some(CollectionShape::Vector { len: items.len() })
        }
        _ => None,
    }
}

fn as_number(expr: &Expression) -> Option<&Number> {
    match expr {
        Expression::Number(number) => Some(number),
        _ => None,
    }
}

fn map_numbers(expr: &Expression, op: &dyn Fn(&Number) -> Option<Number>) -> Option<Expression> {
    match expr {
        Expression::Number(number) => Some(Expression::Number(op(number)?)),
        Expression::Vector(items) => Some(Expression::Vector(
            items
                .iter()
                .map(|item| map_numbers(item, op))
                .collect::<Option<Vec<_>>>()?,
        )),
        _ => None,
    }
}

fn zip_numbers(
    lhs: &Expression,
    rhs: &Expression,
    op: fn(&Number, &Number) -> Number,
) -> Option<Expression> {
    match (lhs, rhs) {
        (Expression::Number(lhs), Expression::Number(rhs)) => {
            Some(Expression::Number(op(lhs, rhs)))
        }
        (Expression::Vector(lhs_items), Expression::Vector(rhs_items))
            if lhs_items.len() == rhs_items.len() =>
        {
            Some(Expression::Vector(
                lhs_items
                    .iter()
                    .zip(rhs_items)
                    .map(|(lhs, rhs)| zip_numbers(lhs, rhs, op))
                    .collect::<Option<Vec<_>>>()?,
            ))
        }
        _ => None,
    }
}

fn matrix_multiply(lhs: &Expression, rhs: &Expression) -> Option<Expression> {
    let lhs_rows = matrix_numbers(lhs)?;
    let rhs_rows = matrix_numbers(rhs)?;
    let lhs_cols = lhs_rows.first()?.len();
    if lhs_cols != rhs_rows.len() {
        return None;
    }
    let rhs_cols = rhs_rows.first()?.len();
    let rhs_columns = (0..rhs_cols)
        .map(|col| {
            rhs_rows
                .iter()
                .map(|row| row[col].clone())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut result = Vec::with_capacity(lhs_rows.len());
    for lhs_row in &lhs_rows {
        let mut result_row = Vec::with_capacity(rhs_columns.len());
        for rhs_col in &rhs_columns {
            let mut sum = Number::from_i32(0);
            for (lhs_value, rhs_value) in lhs_row.iter().zip(rhs_col) {
                sum = sum.add(&lhs_value.mul(rhs_value));
            }
            result_row.push(Expression::Number(sum));
        }
        result.push(Expression::Vector(result_row));
    }
    Some(Expression::Vector(result))
}

fn matrix_numbers(expr: &Expression) -> Option<Vec<Vec<Number>>> {
    let Expression::Vector(rows) = expr else {
        return None;
    };
    rows.iter()
        .map(|row| {
            let Expression::Vector(items) = row else {
                return None;
            };
            items
                .iter()
                .map(|item| as_number(item).cloned())
                .collect::<Option<Vec<_>>>()
        })
        .collect()
}

fn matrix_to_vector(args: &[Expression]) -> Option<Expression> {
    let values = flatten_owned(args.first()?).collect::<Vec<_>>();
    match values.as_slice() {
        [] => Some(Expression::Vector(Vec::new())),
        [single] => Some(single.clone()),
        _ => Some(Expression::Vector(values)),
    }
}

fn columns(expr: &Expression) -> Option<usize> {
    match expr {
        Expression::Vector(items) if items.is_empty() => Some(0),
        Expression::Vector(items) if is_rectangular_matrix(expr) => {
            let Expression::Vector(first_row) = items.first()? else {
                return None;
            };
            Some(first_row.len())
        }
        Expression::Vector(items) => Some(items.len()),
        Expression::Number(_) => Some(1),
        _ => None,
    }
}

fn elements(expr: &Expression) -> Option<usize> {
    match expr {
        Expression::Vector(items) if is_rectangular_matrix(expr) => {
            let cols = columns(expr)?;
            Some(items.len() * cols)
        }
        Expression::Vector(items) => Some(items.len()),
        Expression::Number(_) => Some(1),
        _ => None,
    }
}

fn element(args: &[Expression]) -> Option<Expression> {
    let collection = args.first()?;
    let first_index = number_to_usize(args.get(1)?)?.checked_sub(1)?;

    if is_rectangular_matrix(collection) {
        let Expression::Vector(rows) = collection else {
            return None;
        };
        if args.len() == 2 {
            return rows.get(first_index).cloned();
        }
        let col_index = number_to_usize(args.get(2)?)?.checked_sub(1)?;
        let Expression::Vector(row) = rows.get(first_index)? else {
            return None;
        };
        return row.get(col_index).cloned();
    }

    let Expression::Vector(items) = collection else {
        return None;
    };
    items.get(first_index).cloned()
}

pub(crate) fn is_rectangular_matrix(expr: &Expression) -> bool {
    let Expression::Vector(rows) = expr else {
        return false;
    };
    let Some(first_len) = rows.first().and_then(vector_len) else {
        return false;
    };
    rows.iter().all(|row| vector_len(row) == Some(first_len))
}

fn vector_len(expr: &Expression) -> Option<usize> {
    match expr {
        Expression::Vector(items) => Some(items.len()),
        _ => None,
    }
}

fn has_ragged_nested_vectors(expr: &Expression) -> bool {
    let Expression::Vector(items) = expr else {
        return false;
    };
    let nested_lengths = items.iter().filter_map(vector_len).collect::<Vec<_>>();
    if !nested_lengths.is_empty()
        && (nested_lengths.len() != items.len()
            || nested_lengths
                .iter()
                .any(|len| Some(*len) != nested_lengths.first().copied()))
    {
        return true;
    }
    items.iter().any(has_ragged_nested_vectors)
}

fn flatten_owned(expr: &Expression) -> Box<dyn Iterator<Item = Expression> + '_> {
    match expr {
        Expression::Vector(items) if is_rectangular_matrix(expr) => {
            Box::new(items.iter().flat_map(|row| {
                let Expression::Vector(row_items) = row else {
                    unreachable!("is_rectangular_matrix checked every row");
                };
                row_items.iter().cloned()
            }))
        }
        Expression::Vector(items) => Box::new(items.iter().cloned()),
        other => Box::new(std::iter::once(other.clone())),
    }
}

fn number_to_usize(expr: &Expression) -> Option<usize> {
    let Expression::Number(number) = expr else {
        return None;
    };
    let value = number.to_i64()?;
    (value >= 0).then_some(value as usize)
}

fn number_to_i64(expr: &Expression) -> Option<i64> {
    let Expression::Number(number) = expr else {
        return None;
    };
    number.to_i64()
}

fn dimension(expr: &Expression) -> Option<usize> {
    match expr {
        Expression::Vector(items) => Some(items.len()),
        _ => None,
    }
}

fn rows_count(expr: &Expression) -> Option<usize> {
    let matrix = to_matrix(expr)?;
    Some(matrix.len())
}

fn row(expr: &Expression, row_idx: i64) -> Option<Expression> {
    let matrix = to_matrix(expr)?;
    let num_rows = matrix.len();
    let idx = resolve_index(row_idx, num_rows)?;
    let r = matrix.get(idx)?;
    Some(simplify_result(r.clone()))
}

fn column(expr: &Expression, col_idx: i64) -> Option<Expression> {
    let matrix = to_matrix(expr)?;
    let num_cols = matrix.first()?.len();
    let idx = resolve_index(col_idx, num_cols)?;
    let mut col_vec = Vec::new();
    for r in matrix {
        col_vec.push(r.get(idx)?.clone());
    }
    Some(simplify_result(col_vec))
}

fn resolve_index(idx: i64, len: usize) -> Option<usize> {
    let mut resolved = idx;
    if resolved < 0 {
        resolved += len as i64 + 1;
    }
    if resolved <= 0 || resolved > len as i64 {
        None
    } else {
        Some((resolved - 1) as usize)
    }
}

fn to_matrix(expr: &Expression) -> Option<Vec<Vec<Expression>>> {
    match expr {
        Expression::Vector(items) => {
            if items.is_empty() {
                Some(Vec::new())
            } else if is_rectangular_matrix(expr) {
                let mut matrix = Vec::new();
                for row_expr in items {
                    if let Expression::Vector(row_items) = row_expr {
                        matrix.push(row_items.clone());
                    } else {
                        return None;
                    }
                }
                Some(matrix)
            } else {
                Some(vec![items.clone()])
            }
        }
        Expression::Number(_) => Some(vec![vec![expr.clone()]]),
        _ => None,
    }
}

fn simplify_result(vec: Vec<Expression>) -> Expression {
    if vec.len() == 1 {
        vec.into_iter().next().unwrap()
    } else {
        Expression::Vector(vec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::format_result_with_numbers;

    fn format(expr: &Expression) -> String {
        format_result_with_numbers(expr, &Number::to_string).expect("collection should format")
    }

    #[test]
    fn parses_omitted_vector_elements() {
        let expr = parse_collection_literal("(1,)").expect("literal should parse");
        assert_eq!(format(&expr), "[1  0]");

        let expr = parse_collection_literal("1,").expect("literal should parse");
        assert_eq!(format(&expr), "[1  0]");

        let expr = parse_collection_literal("[,,,]").expect("literal should parse");
        assert_eq!(format(&expr), "[0  0  0  0]");

        let expr = parse_collection_literal("(,,,-2)").expect("literal should parse");
        assert_eq!(format(&expr), "[0  0  0  -2]");

        let expr = parse_collection_literal("(1;;2)").expect("literal should parse");
        assert_eq!(format(&expr), "[1  0  2]");
    }

    #[test]
    fn leaves_grouped_scalars_to_the_regular_parser() {
        assert!(parse_collection_literal("(1)").is_none());
        assert!(parse_collection_literal("((1))").is_none());
    }

    #[test]
    fn parses_matrix_rows() {
        let expr = parse_collection_literal("((1, 2), (4, 5))").expect("literal should parse");
        assert_eq!(format(&expr), "[1  2; 4  5]");

        let expr =
            parse_collection_literal("((1; 2; 3); (4; 5; 6))").expect("literal should parse");
        assert_eq!(format(&expr), "[1  2  3; 4  5  6]");

        let expr = parse_collection_literal("[1 2; 4 5]").expect("literal should parse");
        assert_eq!(format(&expr), "[1  2; 4  5]");

        let expr = parse_collection_literal("[[1, 2], [4, 5]]").expect("literal should parse");
        assert_eq!(format(&expr), "[1  2; 4  5]");

        let expr = parse_collection_literal("[-0.1, 1.23, ], [.1, , -.2], [,,]")
            .expect("literal should parse");
        assert_eq!(format(&expr), "[-0.1  1.23  0; 0.1  0  -0.2; 0  0  0]");

        assert!(parse_collection_literal("( 1; 2; 3, 4, 5, 6 ); (4; 5)").is_none());
    }

    #[test]
    fn evaluates_constructor_and_accessor_functions() {
        let expr = evaluate_collection_function("vector()").expect("function should parse");
        assert_eq!(format(&expr), "[]");

        let expr = evaluate_collection_function("vector(,)").expect("function should parse");
        assert_eq!(format(&expr), "[0  0]");

        let expr = evaluate_collection_function("vector(1, 2, 3)").expect("function should parse");
        assert_eq!(format(&expr), "[1  2  3]");

        let expr =
            evaluate_collection_function("matrix(1, 1, [2])").expect("function should parse");
        assert_eq!(format(&expr), "2");

        let expr = evaluate_collection_function("matrix(1, 3, 2)").expect("function should parse");
        assert_eq!(format(&expr), "[2  0  0]");

        let expr =
            evaluate_collection_function("matrix(3, 1, [1 2])").expect("function should parse");
        assert_eq!(format(&expr), "[1; 2; 0]");

        let expr = evaluate_collection_function("matrix(3, 3, [])").expect("function should parse");
        assert_eq!(format(&expr), "[0  0  0; 0  0  0; 0  0  0]");

        let expr = evaluate_collection_function("matrix(3, 3, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10)")
            .expect("function should parse");
        assert_eq!(format(&expr), "[1  2  3; 4  5  6; 7  8  9]");

        assert!(evaluate_collection_function("matrix(0, 3, [])").is_none());
        assert!(evaluate_collection_function("matrix(3, 0, [])").is_none());

        let expr = evaluate_collection_function("columns([[,,,]])").expect("function should parse");
        assert_eq!(format(&expr), "4");

        let expr = evaluate_collection_function("columns([1])").expect("function should parse");
        assert_eq!(format(&expr), "1");

        let expr = evaluate_collection_function("matrix2vector([1 2; 4 5])")
            .expect("function should parse");
        assert_eq!(format(&expr), "[1  2  4  5]");

        let expr =
            evaluate_collection_function("matrix2vector([[0]])").expect("function should parse");
        assert_eq!(format(&expr), "0");

        let expr = evaluate_collection_function("matrix2vector([1 2 3; 4 5 6; 7 8 9])")
            .expect("function should parse");
        assert_eq!(format(&expr), "[1  2  3  4  5  6  7  8  9]");

        let expr =
            evaluate_collection_function("columns([1 2; 4 5])").expect("function should parse");
        assert_eq!(format(&expr), "2");

        assert!(evaluate_collection_function("columns([[1], [2, 3]])").is_none());

        let expr =
            evaluate_collection_function("element([1 2; 3 4], 1)").expect("function should parse");
        assert_eq!(format(&expr), "[1  2]");

        let expr = evaluate_collection_function("element([1 2 3; 4 5 6; 1 0 9], 1, 3)")
            .expect("function should parse");
        assert_eq!(format(&expr), "3");

        let expr = evaluate_collection_function("element([1 2 3; 4 5 6], 2, 1)")
            .expect("function should parse");
        assert_eq!(format(&expr), "4");

        let expr = evaluate_collection_function("elements([])").expect("function should parse");
        assert_eq!(format(&expr), "0");

        let expr = evaluate_collection_function("elements([1 2])").expect("function should parse");
        assert_eq!(format(&expr), "2");

        let expr =
            evaluate_collection_function("elements([1 2; 3 4])").expect("function should parse");
        assert_eq!(format(&expr), "4");
    }

    #[test]
    fn evaluates_basic_arithmetic() {
        let expr =
            evaluate_collection_arithmetic("(1; 2; 3) * 2 - 2").expect("expression should parse");
        assert_eq!(format(&expr), "[0  2  4]");

        let expr =
            evaluate_collection_arithmetic("[1,2] + [3,4]").expect("expression should parse");
        assert_eq!(format(&expr), "[4  6]");

        let expr =
            evaluate_collection_arithmetic("[1 2; 4 5] * 2").expect("expression should parse");
        assert_eq!(format(&expr), "[2  4; 8  10]");

        let expr =
            evaluate_collection_arithmetic("((1; 2; 3); (4; 5; 6)) * ((7; 8); (9; 10); (11; 12))")
                .expect("expression should parse");
        assert_eq!(format(&expr), "[58  64; 139  154]");

        let expr = evaluate_collection_arithmetic("[1 2; 3 4].*[1 2; 3 4]")
            .expect("expression should parse");
        assert_eq!(format(&expr), "[1  4; 9  16]");

        assert!(evaluate_collection_arithmetic("[1 2] + [3 4 5]").is_none());
    }

    #[test]
    fn evaluates_dimension_rows_row_column() {
        let expr = evaluate_collection_function("dimension([])").expect("should evaluate");
        assert_eq!(format(&expr), "0");

        let expr = evaluate_collection_function("dimension([0])").expect("should evaluate");
        assert_eq!(format(&expr), "1");

        let expr = evaluate_collection_function("dimension([1 2 3 4])").expect("should evaluate");
        assert_eq!(format(&expr), "4");

        let expr = evaluate_collection_function("rows([1])").expect("should evaluate");
        assert_eq!(format(&expr), "1");

        let expr = evaluate_collection_function("rows([1 2; 3 4])").expect("should evaluate");
        assert_eq!(format(&expr), "2");

        let expr = evaluate_collection_function("row([1], 1)").expect("should evaluate");
        assert_eq!(format(&expr), "1");

        let expr = evaluate_collection_function("row([1 2], 1)").expect("should evaluate");
        assert_eq!(format(&expr), "[1  2]");

        let expr = evaluate_collection_function("row([1 2; 3 4], 2)").expect("should evaluate");
        assert_eq!(format(&expr), "[3  4]");

        let expr = evaluate_collection_function("column([1], 1)").expect("should evaluate");
        assert_eq!(format(&expr), "1");

        let expr = evaluate_collection_function("column([1, 2], 1)").expect("should evaluate");
        assert_eq!(format(&expr), "1");

        let expr = evaluate_collection_function("column([1 2; 3 4], 2)").expect("should evaluate");
        assert_eq!(format(&expr), "[2  4]");
    }

    #[test]
    fn shape_accessors_fail_closed_on_unsupported_arity() {
        assert!(evaluate_collection_function("dimension([1], 2)").is_none());
        assert!(evaluate_collection_function("rows([1], 2)").is_none());
        assert!(evaluate_collection_function("row([1], 1, 2)").is_none());
        assert!(evaluate_collection_function("column([1], 1, 2)").is_none());
    }
}
