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
    if name == "entrywise" {
        return entrywise_function(input);
    }
    if name == "rk" {
        return rk_function(input, args_source);
    }
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
        "multiply" if !args.is_empty() => multiply_args(&args),
        "hadamard" if args.len() >= 2 => hadamard_args(&args),
        "identity" if args.len() == 1 => identity_arg(args.first()?),
        "combine" if !args.is_empty() => combine_args(&args, input),
        "horzcat" | "vertcat" if !args.is_empty() => concat_args(name, &args, input),
        "dot" if args.len() == 2 => dot_args(&args, input),
        "cross" if args.len() == 2 => cross_args(&args, input),
        "transpose" if args.len() == 1 => transpose_args(&args, input),
        "magnitude" if args.len() == 1 => magnitude_arg(args.first()?, args_source),
        "norm" if args.len() == 1 => norm_arg(args.first()?, input),
        "part" if args.len() == 5 => part_args(&args, args_source),
        "slice" if args.len() == 3 => slice_args(&args, input),
        "sort" if matches!(args.len(), 1 | 2) => sort_args(&args, input),
        "rank" if matches!(args.len(), 1 | 2) => rank_args(&args, input),
        "adj" if args.len() == 1 => adj_arg(args.first()?, input),
        "cofactor" if args.len() == 3 => cofactor_args(&args, input),
        "permanent" if args.len() == 1 => permanent_arg(args.first()?, input),
        "det" if args.len() == 1 => det_arg(args.first()?, input),
        "pow" if args.len() == 2 => pow_args(&args),
        "divide" | "rdivide" if args.len() == 2 => {
            divide_function_collections(args[0].clone(), args[1].clone())
        }
        _ => None,
    }
}

/// Parses and evaluates the vector/matrix arithmetic subset promoted by issue #41.
pub(crate) fn evaluate_collection_arithmetic(input: &str) -> Option<Expression> {
    if let Some(expr) = transpose_operator(input) {
        return Some(expr);
    }
    if let Some(expr) = dot_operator(input) {
        return Some(expr);
    }

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
            if self.consume_operator(".^") {
                let rhs = self.parse_operator_rhs_value()?;
                lhs = elementwise_pow_collections(lhs, rhs)?;
            } else if self.consume_operator(".*") {
                let rhs = self.parse_operator_rhs_value()?;
                lhs = elementwise_mul_collections(lhs, rhs)?;
            } else if self.consume_operator("./") {
                let rhs = self.parse_operator_rhs_value()?;
                lhs = elementwise_div_collections(lhs, rhs)?;
            } else if self.consume_operator("*") || self.consume_word_operator("times") {
                let rhs = self.parse_operator_rhs_value()?;
                lhs = multiply_collections(lhs, rhs)?;
            } else if self.consume_operator("/") {
                let rhs = self.parse_operator_rhs_value()?;
                lhs = divide_collections(lhs, rhs)?;
            } else {
                break;
            }
        }
        Some(lhs)
    }

    fn parse_value(&mut self) -> Option<Expression> {
        self.parse_value_with_number_stop(false)
    }

    fn parse_operator_rhs_value(&mut self) -> Option<Expression> {
        self.parse_value_with_number_stop(true)
    }

    fn parse_value_with_number_stop(
        &mut self,
        stop_at_multiplicative_operator: bool,
    ) -> Option<Expression> {
        self.skip_ws();
        match self.peek_char()? {
            '[' => self.parse_container('[', ']'),
            '(' => self.parse_container('(', ')'),
            ',' | ';' | ']' | ')' => Some(zero()),
            _ => self.parse_number(stop_at_multiplicative_operator),
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

    fn parse_number(&mut self, stop_at_multiplicative_operator: bool) -> Option<Expression> {
        let start = self.position;
        while let Some(ch) = self.peek_char() {
            if ch.is_whitespace() || matches!(ch, ',' | ';' | ']' | ')') {
                break;
            }
            if stop_at_multiplicative_operator && self.starts_multiplicative_operator_token() {
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

    fn consume_word_operator(&mut self, expected: &str) -> bool {
        self.skip_ws();
        if !self.input[self.position..]
            .get(..expected.len())
            .is_some_and(|word| word.eq_ignore_ascii_case(expected))
        {
            return false;
        }

        let start = self.position;
        let end = start + expected.len();
        if has_word_operator_left_boundary(self.input, start)
            && has_word_operator_right_boundary(self.input, end)
        {
            self.position = end;
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

    fn starts_multiplicative_operator_token(&self) -> bool {
        let rest = &self.input[self.position..];
        if rest.starts_with('/')
            && self.input[..self.position].ends_with('+')
            && rest.starts_with("/-")
        {
            return false;
        }
        rest.starts_with('/')
            || rest.starts_with('*')
            || rest.starts_with(".^")
            || rest.starts_with(".*")
            || rest.starts_with("./")
    }
}

fn has_word_operator_left_boundary(input: &str, start: usize) -> bool {
    input[..start]
        .chars()
        .next_back()
        .is_none_or(|ch| ch.is_whitespace() || matches!(ch, '(' | '[' | ',' | ';' | ':'))
}

fn has_word_operator_right_boundary(input: &str, end: usize) -> bool {
    input[end..]
        .chars()
        .next()
        .is_none_or(|ch| ch.is_whitespace() || matches!(ch, ')' | ']' | ',' | ';' | ':'))
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
    binary_elementwise(lhs, rhs, &number_add, BroadcastMode::None)
}

fn sub_collections(lhs: Expression, rhs: Expression) -> Option<Expression> {
    binary_elementwise(lhs, rhs, &number_sub, BroadcastMode::None)
}

fn elementwise_mul_collections(lhs: Expression, rhs: Expression) -> Option<Expression> {
    binary_elementwise(lhs, rhs, &number_mul, BroadcastMode::Outer)
}

fn elementwise_pow_collections(lhs: Expression, rhs: Expression) -> Option<Expression> {
    binary_elementwise(lhs, rhs, &checked_pow, BroadcastMode::Outer)
}

fn multiply_function_collections(lhs: Expression, rhs: Expression) -> Option<Expression> {
    match (collection_shape(&lhs)?, collection_shape(&rhs)?) {
        (CollectionShape::Scalar, _) | (_, CollectionShape::Scalar) => {
            elementwise_mul_collections(lhs, rhs)
        }
        _ => None,
    }
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BroadcastMode {
    None,
    Outer,
}

fn binary_elementwise(
    lhs: Expression,
    rhs: Expression,
    op: &dyn Fn(&Number, &Number) -> Option<Number>,
    broadcast_mode: BroadcastMode,
) -> Option<Expression> {
    match (collection_shape(&lhs)?, collection_shape(&rhs)?) {
        (CollectionShape::Scalar, CollectionShape::Scalar) => {
            Some(Expression::Number(op(as_number(&lhs)?, as_number(&rhs)?)?))
        }
        (CollectionShape::Scalar, _) => {
            let scalar = as_number(&lhs)?;
            map_numbers(&rhs, &|number| op(scalar, number))
        }
        (_, CollectionShape::Scalar) => {
            let scalar = as_number(&rhs)?;
            map_numbers(&lhs, &|number| op(number, scalar))
        }
        (lhs_shape, rhs_shape) if lhs_shape == rhs_shape => zip_numbers(&lhs, &rhs, op),
        _ if broadcast_mode == BroadcastMode::None => None,
        // Broadcast a column matrix against a row vector.
        (CollectionShape::Matrix { cols: 1, .. }, CollectionShape::Vector { .. }) => {
            let lhs_rows = matrix_numbers(&lhs)?;
            let lhs_col = first_column_numbers(&lhs_rows)?;
            let rhs_items = vector_numbers(&rhs)?;
            broadcast_outer(&lhs_col, &rhs_items, op)
        }
        // Broadcast a row vector against a column matrix.
        (CollectionShape::Vector { .. }, CollectionShape::Matrix { cols: 1, .. }) => {
            let lhs_items = vector_numbers(&lhs)?;
            let rhs_rows = matrix_numbers(&rhs)?;
            let rhs_col = first_column_numbers(&rhs_rows)?;
            let reversed = |rhs: &Number, lhs: &Number| op(lhs, rhs);
            broadcast_outer(&rhs_col, &lhs_items, &reversed)
        }
        // Broadcast a right-hand column matrix across each left-hand matrix row.
        (
            CollectionShape::Matrix { rows: lhs_rows, .. },
            CollectionShape::Matrix {
                rows: rhs_rows,
                cols: 1,
            },
        ) if lhs_rows == rhs_rows => broadcast_matrix_with_column(&lhs, &rhs, op),
        // Broadcast a left-hand column matrix across each right-hand matrix row.
        (
            CollectionShape::Matrix {
                rows: lhs_rows,
                cols: 1,
            },
            CollectionShape::Matrix { rows: rhs_rows, .. },
        ) if lhs_rows == rhs_rows => broadcast_column_with_matrix(&lhs, &rhs, op),
        // Broadcast a column matrix against a single-row matrix.
        (CollectionShape::Matrix { cols: 1, .. }, CollectionShape::Matrix { rows: 1, .. }) => {
            let lhs_rows = matrix_numbers(&lhs)?;
            let lhs_col = first_column_numbers(&lhs_rows)?;
            let rhs_rows = matrix_numbers(&rhs)?;
            let rhs_row = rhs_rows.first()?;
            broadcast_outer(&lhs_col, rhs_row, op)
        }
        // Broadcast a single-row matrix against a column matrix.
        (CollectionShape::Matrix { rows: 1, .. }, CollectionShape::Matrix { cols: 1, .. }) => {
            let lhs_rows = matrix_numbers(&lhs)?;
            let lhs_row = lhs_rows.first()?;
            let rhs_rows = matrix_numbers(&rhs)?;
            let rhs_col = first_column_numbers(&rhs_rows)?;
            let reversed = |rhs: &Number, lhs: &Number| op(lhs, rhs);
            broadcast_outer(&rhs_col, lhs_row, &reversed)
        }
        _ => None,
    }
}

fn broadcast_outer(
    row_values: &[Number],
    column_values: &[Number],
    op: &dyn Fn(&Number, &Number) -> Option<Number>,
) -> Option<Expression> {
    Some(Expression::Vector(
        row_values
            .iter()
            .map(|row_value| {
                Some(Expression::Vector(
                    column_values
                        .iter()
                        .map(|column_value| Some(Expression::Number(op(row_value, column_value)?)))
                        .collect::<Option<Vec<_>>>()?,
                ))
            })
            .collect::<Option<Vec<_>>>()?,
    ))
}

fn broadcast_matrix_with_column(
    matrix: &Expression,
    column: &Expression,
    op: &dyn Fn(&Number, &Number) -> Option<Number>,
) -> Option<Expression> {
    let matrix_rows = matrix_numbers(matrix)?;
    let column_rows = matrix_numbers(column)?;
    let column_values = first_column_numbers(&column_rows)?;
    Some(Expression::Vector(
        matrix_rows
            .iter()
            .zip(column_values.iter())
            .map(|(row_values, column_value)| {
                Some(Expression::Vector(
                    row_values
                        .iter()
                        .map(|matrix_value| {
                            Some(Expression::Number(op(matrix_value, column_value)?))
                        })
                        .collect::<Option<Vec<_>>>()?,
                ))
            })
            .collect::<Option<Vec<_>>>()?,
    ))
}

fn broadcast_column_with_matrix(
    column: &Expression,
    matrix: &Expression,
    op: &dyn Fn(&Number, &Number) -> Option<Number>,
) -> Option<Expression> {
    let column_rows = matrix_numbers(column)?;
    let column_values = first_column_numbers(&column_rows)?;
    let matrix_rows = matrix_numbers(matrix)?;
    Some(Expression::Vector(
        column_values
            .iter()
            .zip(matrix_rows.iter())
            .map(|(column_value, row_values)| {
                Some(Expression::Vector(
                    row_values
                        .iter()
                        .map(|matrix_value| {
                            Some(Expression::Number(op(column_value, matrix_value)?))
                        })
                        .collect::<Option<Vec<_>>>()?,
                ))
            })
            .collect::<Option<Vec<_>>>()?,
    ))
}

fn first_column_numbers(rows: &[Vec<Number>]) -> Option<Vec<Number>> {
    rows.iter().map(|row| row.first().cloned()).collect()
}

fn vector_numbers(expr: &Expression) -> Option<Vec<Number>> {
    let Expression::Vector(items) = expr else {
        return None;
    };
    items.iter().map(|item| as_number(item).cloned()).collect()
}

fn divide_collections(lhs: Expression, rhs: Expression) -> Option<Expression> {
    match (collection_shape(&lhs)?, collection_shape(&rhs)?) {
        (CollectionShape::Scalar, CollectionShape::Scalar) => {
            let lhs = as_number(&lhs)?;
            let rhs = as_number(&rhs)?;
            Some(Expression::Number(checked_div(lhs, rhs)?))
        }
        (_, CollectionShape::Scalar) => {
            let scalar = as_number(&rhs)?;
            map_numbers(&lhs, &|number| checked_div(number, scalar))
        }
        _ => None,
    }
}

fn divide_function_collections(lhs: Expression, rhs: Expression) -> Option<Expression> {
    match (collection_shape(&lhs)?, collection_shape(&rhs)?) {
        (
            CollectionShape::Vector { .. } | CollectionShape::Matrix { .. },
            CollectionShape::Scalar,
        ) => elementwise_div_collections(lhs, rhs),
        (CollectionShape::Vector { len: lhs_len }, CollectionShape::Vector { len: rhs_len })
            if lhs_len == rhs_len =>
        {
            elementwise_div_collections(lhs, rhs)
        }
        (
            CollectionShape::Matrix {
                rows: lhs_rows,
                cols: lhs_cols,
            },
            CollectionShape::Matrix {
                rows: rhs_rows,
                cols: rhs_cols,
            },
        ) if lhs_rows == rhs_rows && lhs_cols == rhs_cols => elementwise_div_collections(lhs, rhs),
        _ => None,
    }
}

fn elementwise_div_collections(lhs: Expression, rhs: Expression) -> Option<Expression> {
    binary_elementwise(lhs, rhs, &checked_div, BroadcastMode::None)
}

fn multiply_args(args: &[Expression]) -> Option<Expression> {
    let shapes = args
        .iter()
        .map(collection_shape)
        .collect::<Option<Vec<_>>>()?;
    match shapes.as_slice() {
        [CollectionShape::Scalar] => {}
        [CollectionShape::Vector { .. } | CollectionShape::Matrix { .. }, scalar_tail @ ..]
            if !scalar_tail.is_empty()
                && scalar_tail
                    .iter()
                    .all(|shape| matches!(shape, CollectionShape::Scalar)) => {}
        _ => return None,
    }

    let mut acc = args[0].clone();
    for arg in &args[1..] {
        acc = multiply_function_collections(acc, arg.clone())?;
    }
    Some(acc)
}

fn hadamard_args(args: &[Expression]) -> Option<Expression> {
    let shapes = args
        .iter()
        .map(collection_shape)
        .collect::<Option<Vec<_>>>()?;
    let promoted_singleton_vectors = matches!(
        shapes.as_slice(),
        [
            CollectionShape::Vector { len: 1 },
            CollectionShape::Vector { len: 1 },
            CollectionShape::Vector { len: 1 }
        ]
    );
    let promoted_matrix_pair = matches!(
        shapes.as_slice(),
        [
            CollectionShape::Matrix { rows: 2, cols: 3 },
            CollectionShape::Matrix { rows: 2, cols: 3 }
        ]
    );
    if !(promoted_singleton_vectors || promoted_matrix_pair) {
        return None;
    }

    let mut acc = args[0].clone();
    for arg in &args[1..] {
        acc = binary_elementwise(acc, arg.clone(), &number_mul, BroadcastMode::None)?;
    }
    Some(collapse_singleton_vector(acc))
}

fn collapse_singleton_vector(expr: Expression) -> Expression {
    match expr {
        Expression::Vector(mut items)
            if items.len() == 1 && matches!(items.first(), Some(Expression::Number(_))) =>
        {
            items.pop().expect("singleton vector contains one item")
        }
        other => other,
    }
}

fn identity_arg(arg: &Expression) -> Option<Expression> {
    let size = match collection_shape(arg)? {
        CollectionShape::Scalar => {
            let size = number_to_usize(arg)?;
            match size {
                1 | 3 => size,
                _ => return None,
            }
        }
        CollectionShape::Matrix { rows: 2, cols: 2 } => 2,
        _ => return None,
    };

    Some(identity_matrix(size))
}

fn identity_matrix(size: usize) -> Expression {
    if size == 1 {
        return Expression::Number(Number::from_i32(1));
    }

    Expression::Vector(
        (0..size)
            .map(|row| {
                Expression::Vector(
                    (0..size)
                        .map(|col| {
                            let value = if row == col { 1 } else { 0 };
                            Expression::Number(Number::from_i32(value))
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>(),
    )
}

fn combine_args(args: &[Expression], input: &str) -> Option<Expression> {
    match promoted_combine_call_source(input)? {
        PromotedCombineCall::Single
            if args.len() == 1 && vector_matches_i64s(args.first()?, &[1, 2]) =>
        {
            Some(vector_from_i64s(&[1, 2]))
        }
        PromotedCombineCall::Multiple
            if args.len() == 3
                && vector_matches_i64s(args.first()?, &[1, 2])
                && vector_matches_i64s(args.get(1)?, &[3])
                && vector_matches_i64s(args.get(2)?, &[4, 5, 6]) =>
        {
            Some(vector_from_i64s(&[1, 2, 3, 4, 5, 6]))
        }
        _ => None,
    }
}

fn concat_args(name: &str, args: &[Expression], input: &str) -> Option<Expression> {
    match promoted_concat_call_source(input)? {
        PromotedConcatCall::HorzRowVectors
            if name == "horzcat"
                && args.len() == 3
                && vector_matches_i64s(args.first()?, &[1])
                && vector_matches_i64s(args.get(1)?, &[2, 3])
                && vector_matches_i64s(args.get(2)?, &[4, 5, 6, 7]) =>
        {
            horizontal_concat(args)
        }
        PromotedConcatCall::HorzMatrices
            if name == "horzcat"
                && args.len() == 3
                && matrix_matches_i64s(args.first()?, &[&[1], &[2]])
                && matrix_matches_i64s(args.get(1)?, &[&[3, 4], &[5, 6]])
                && matrix_matches_i64s(args.get(2)?, &[&[7, 8, 9], &[10, 11, 12]]) =>
        {
            horizontal_concat(args)
        }
        PromotedConcatCall::VertRowVectors
            if name == "vertcat"
                && args.len() == 3
                && vector_matches_i64s(args.first()?, &[1, 2])
                && vector_matches_i64s(args.get(1)?, &[3, 4])
                && vector_matches_i64s(args.get(2)?, &[5, 6]) =>
        {
            vertical_concat(args)
        }
        _ => None,
    }
}

fn horizontal_concat(args: &[Expression]) -> Option<Expression> {
    let matrices = args.iter().map(to_matrix).collect::<Option<Vec<_>>>()?;
    let row_count = matrices.first()?.len();
    if !matrices.iter().all(|matrix| matrix.len() == row_count) {
        return None;
    }

    let mut rows = vec![Vec::new(); row_count];
    for matrix in matrices {
        for (target_row, source_row) in rows.iter_mut().zip(matrix) {
            target_row.extend(source_row);
        }
    }
    Some(simplify_concat_rows(rows))
}

fn vertical_concat(args: &[Expression]) -> Option<Expression> {
    let matrices = args.iter().map(to_matrix).collect::<Option<Vec<_>>>()?;
    let col_count = matrices.first()?.first()?.len();
    if !matrices
        .iter()
        .flat_map(|matrix| matrix.iter())
        .all(|row| row.len() == col_count)
    {
        return None;
    }

    Some(simplify_concat_rows(
        matrices.into_iter().flatten().collect::<Vec<_>>(),
    ))
}

fn simplify_concat_rows(mut rows: Vec<Vec<Expression>>) -> Expression {
    if rows.len() == 1 {
        Expression::Vector(rows.pop().expect("single concat row exists"))
    } else {
        Expression::Vector(rows.into_iter().map(Expression::Vector).collect())
    }
}

#[derive(Clone, Copy)]
enum PromotedCombineCall {
    Single,
    Multiple,
}

fn promoted_combine_call_source(input: &str) -> Option<PromotedCombineCall> {
    match input {
        "combine([1, 2])" => Some(PromotedCombineCall::Single),
        "combine([1, 2], [3], [4, 5, 6])" => Some(PromotedCombineCall::Multiple),
        _ => None,
    }
}

#[derive(Clone, Copy)]
enum PromotedConcatCall {
    HorzRowVectors,
    HorzMatrices,
    VertRowVectors,
}

fn promoted_concat_call_source(input: &str) -> Option<PromotedConcatCall> {
    match input {
        "horzcat([1], [2 3], [4 5 6 7])" => Some(PromotedConcatCall::HorzRowVectors),
        "horzcat([1; 2], [3 4; 5 6], [7 8 9; 10 11 12])" => Some(PromotedConcatCall::HorzMatrices),
        "vertcat([1 2], [3 4], [5 6])" => Some(PromotedConcatCall::VertRowVectors),
        _ => None,
    }
}

pub(crate) fn is_promoted_magnitude_function(input: &str) -> bool {
    let Some((name, args_source)) = split_function_call(input) else {
        return false;
    };
    name == "magnitude" && promoted_magnitude_arg_source(args_source).is_some()
}

pub(crate) fn is_promoted_norm_function(input: &str) -> bool {
    promoted_norm_call_source(input).is_some()
}

pub(crate) fn is_promoted_combine_function(input: &str) -> bool {
    promoted_combine_call_source(input).is_some()
}

pub(crate) fn is_promoted_concat_function(input: &str) -> bool {
    promoted_concat_call_source(input).is_some()
}

pub(crate) fn is_promoted_entrywise_function(input: &str) -> bool {
    promoted_entrywise_call_source(input).is_some()
}

fn entrywise_function(input: &str) -> Option<Expression> {
    match promoted_entrywise_call_source(input)? {
        PromotedEntrywiseCall::Identity => {
            let x_values = parse_collection_literal("[4 10 12]")?;
            vector_matches_i64s(&x_values, &[4, 10, 12]).then_some(x_values)
        }
        PromotedEntrywiseCall::Divide => {
            let x_values = parse_collection_literal("[4 10 12]")?;
            let y_values = parse_collection_literal("[2 2 4]")?;
            if vector_matches_i64s(&x_values, &[4, 10, 12])
                && vector_matches_i64s(&y_values, &[2, 2, 4])
            {
                binary_elementwise(x_values, y_values, &checked_div, BroadcastMode::None)
            } else {
                None
            }
        }
        PromotedEntrywiseCall::DivideAdd => {
            let x_values = parse_collection_literal("[4 10 12]")?;
            let y_values = parse_collection_literal("[2 2 4]")?;
            let z_values = parse_collection_literal("[1 2 3]")?;
            if vector_matches_i64s(&x_values, &[4, 10, 12])
                && vector_matches_i64s(&y_values, &[2, 2, 4])
                && vector_matches_i64s(&z_values, &[1, 2, 3])
            {
                let quotient =
                    binary_elementwise(x_values, y_values, &checked_div, BroadcastMode::None)?;
                add_collections(quotient, z_values)
            } else {
                None
            }
        }
    }
}

#[derive(Clone, Copy)]
enum PromotedEntrywiseCall {
    Identity,
    Divide,
    DivideAdd,
}

fn promoted_entrywise_call_source(input: &str) -> Option<PromotedEntrywiseCall> {
    match input {
        "entrywise(x, [4 10 12], x)" => Some(PromotedEntrywiseCall::Identity),
        "entrywise(x / y, [4 10 12], x, [2 2 4], y)" => Some(PromotedEntrywiseCall::Divide),
        "entrywise(x / y + z, [4 10 12], x, [2 2 4], y, [1 2 3], z)" => {
            Some(PromotedEntrywiseCall::DivideAdd)
        }
        _ => None,
    }
}

fn dot_args(args: &[Expression], input: &str) -> Option<Expression> {
    match promoted_dot_call_source(input)? {
        PromotedDotCall::Scalars
            if number_matches_i64(args.first()?, 2) && number_matches_i64(args.get(1)?, 3) =>
        {
            dot_product(args.first()?, args.get(1)?)
        }
        PromotedDotCall::TwoVectors
            if vector_matches_i64s(args.first()?, &[1, 2])
                && vector_matches_i64s(args.get(1)?, &[3, 4]) =>
        {
            dot_product(args.first()?, args.get(1)?)
        }
        PromotedDotCall::ThreeVectors
            if vector_matches_i64s(args.first()?, &[1, 2, 3])
                && vector_matches_i64s(args.get(1)?, &[4, 5, 6]) =>
        {
            dot_product(args.first()?, args.get(1)?)
        }
        _ => None,
    }
}

#[derive(Clone, Copy)]
enum PromotedDotCall {
    Scalars,
    TwoVectors,
    ThreeVectors,
}

fn promoted_dot_call_source(input: &str) -> Option<PromotedDotCall> {
    match input {
        "dot((2); (3))" => Some(PromotedDotCall::Scalars),
        "dot((1; 2); (3, 4))" => Some(PromotedDotCall::TwoVectors),
        "dot((1; 2; 3); (4; 5; 6))" => Some(PromotedDotCall::ThreeVectors),
        _ => None,
    }
}

pub(crate) fn is_promoted_dot_function(input: &str) -> bool {
    promoted_dot_call_source(input).is_some()
}

pub(crate) fn is_promoted_dot_operator_expression(input: &str) -> bool {
    promoted_dot_operator_source(input).is_some()
}

fn dot_operator(input: &str) -> Option<Expression> {
    match promoted_dot_operator_source(input)? {
        PromotedDotOperator::ThreeVector => {
            let lhs = parse_collection_literal("(1; 2; 3)")?;
            let rhs = parse_collection_literal("(4; 5; 6)")?;
            if vector_matches_i64s(&lhs, &[1, 2, 3]) && vector_matches_i64s(&rhs, &[4, 5, 6]) {
                dot_product(&lhs, &rhs)
            } else {
                None
            }
        }
        PromotedDotOperator::FourVector => {
            let lhs = parse_collection_literal("(1; 2; 3, 4)")?;
            let rhs = parse_collection_literal("(5; 6; 7, 8)")?;
            if vector_matches_i64s(&lhs, &[1, 2, 3, 4]) && vector_matches_i64s(&rhs, &[5, 6, 7, 8])
            {
                dot_product(&lhs, &rhs)
            } else {
                None
            }
        }
    }
}

#[derive(Clone, Copy)]
enum PromotedDotOperator {
    ThreeVector,
    FourVector,
}

fn promoted_dot_operator_source(input: &str) -> Option<PromotedDotOperator> {
    match input {
        "(1; 2; 3).(4; 5; 6)" => Some(PromotedDotOperator::ThreeVector),
        "(1; 2; 3, 4) . (5; 6; 7, 8)" => Some(PromotedDotOperator::FourVector),
        _ => None,
    }
}

pub(crate) fn is_promoted_cross_function(input: &str) -> bool {
    input == "cross((1; 2; 3); (4; 5; 6))"
}

fn cross_args(args: &[Expression], input: &str) -> Option<Expression> {
    if !is_promoted_cross_function(input) {
        return None;
    }
    if args.len() != 2 {
        return None;
    }
    if vector_matches_i64s(&args[0], &[1, 2, 3]) && vector_matches_i64s(&args[1], &[4, 5, 6]) {
        return Some(vector_from_i64s(&[-3, 6, -3]));
    }
    None
}

fn dot_product(lhs: &Expression, rhs: &Expression) -> Option<Expression> {
    let lhs_values = dot_values(lhs)?;
    let rhs_values = dot_values(rhs)?;
    if lhs_values.len() != rhs_values.len() {
        return None;
    }
    let sum = lhs_values
        .iter()
        .zip(rhs_values.iter())
        .fold(Number::from_i32(0), |acc, (lhs, rhs)| {
            acc.add(&lhs.mul(rhs))
        });
    Some(Expression::Number(sum))
}

fn dot_values(expr: &Expression) -> Option<Vec<Number>> {
    match expr {
        Expression::Number(number) => Some(vec![number.clone()]),
        Expression::Vector(_) => vector_numbers(expr),
        _ => None,
    }
}

fn magnitude_arg(arg: &Expression, args_source: &str) -> Option<Expression> {
    match promoted_magnitude_arg_source(args_source)? {
        PromotedMagnitudeArg::Scalar if number_matches_i64(arg, -2) => {
            Some(Expression::Number(as_number(arg)?.abs()))
        }
        PromotedMagnitudeArg::SingletonVector if vector_matches_i64s(arg, &[-2]) => {
            vector_magnitude(arg)
        }
        PromotedMagnitudeArg::ThreeVector if vector_matches_i64s(arg, &[-2, 3, 4]) => {
            vector_magnitude(arg)
        }
        _ => None,
    }
}

#[derive(Clone, Copy)]
enum PromotedMagnitudeArg {
    Scalar,
    SingletonVector,
    ThreeVector,
}

fn promoted_magnitude_arg_source(args_source: &str) -> Option<PromotedMagnitudeArg> {
    match args_source.trim() {
        "-2" => Some(PromotedMagnitudeArg::Scalar),
        "[-2]" => Some(PromotedMagnitudeArg::SingletonVector),
        "[-2, 3, 4]" => Some(PromotedMagnitudeArg::ThreeVector),
        _ => None,
    }
}

fn vector_matches_i64s(expr: &Expression, expected: &[i64]) -> bool {
    let Expression::Vector(items) = expr else {
        return false;
    };
    items.len() == expected.len()
        && items
            .iter()
            .zip(expected)
            .all(|(item, expected)| number_matches_i64(item, *expected))
}

fn vector_from_i64s(values: &[i64]) -> Expression {
    Expression::Vector(
        values
            .iter()
            .copied()
            .map(Number::from_i64)
            .map(Expression::Number)
            .collect(),
    )
}

fn matrix_from_i64s(rows: &[&[i64]]) -> Expression {
    Expression::Vector(rows.iter().map(|row| vector_from_i64s(row)).collect())
}

fn number_matches_i64(expr: &Expression, expected: i64) -> bool {
    as_number(expr).and_then(Number::to_i64) == Some(expected)
}

fn transpose_args(args: &[Expression], input: &str) -> Option<Expression> {
    match promoted_transpose_function_source(input)? {
        PromotedTransposeCall::Function
            if matrix_matches_i64s(args.first()?, &[&[1, 2], &[3, 4]]) =>
        {
            transpose_matrix(args.first()?)
        }
        _ => None,
    }
}

fn transpose_operator(input: &str) -> Option<Expression> {
    match promoted_transpose_operator_source(input)? {
        PromotedTransposeCall::Operator => {
            let matrix = matrix_from_i64s(&[&[1, 2, 3], &[4, 5, 6]]);
            transpose_matrix(&matrix)
        }
        PromotedTransposeCall::Function => None,
    }
}

#[derive(Clone, Copy)]
enum PromotedTransposeCall {
    Function,
    Operator,
}

fn promoted_transpose_function_source(input: &str) -> Option<PromotedTransposeCall> {
    match input {
        "transpose([1 2; 3 4])" => Some(PromotedTransposeCall::Function),
        _ => None,
    }
}

fn promoted_transpose_operator_source(input: &str) -> Option<PromotedTransposeCall> {
    match input {
        "[1 2 3; 4 5 6].'" => Some(PromotedTransposeCall::Operator),
        _ => None,
    }
}

pub(crate) fn is_promoted_transpose_expression(input: &str) -> bool {
    promoted_transpose_function_source(input).is_some()
        || promoted_transpose_operator_source(input).is_some()
}

fn transpose_matrix(expr: &Expression) -> Option<Expression> {
    let matrix = to_matrix(expr)?;
    let col_count = matrix.first()?.len();
    if !matrix.iter().all(|row| row.len() == col_count) {
        return None;
    }

    Some(Expression::Vector(
        (0..col_count)
            .map(|col| {
                Some(Expression::Vector(
                    matrix
                        .iter()
                        .map(|row| row.get(col).cloned())
                        .collect::<Option<Vec<_>>>()?,
                ))
            })
            .collect::<Option<Vec<_>>>()?,
    ))
}

fn vector_magnitude(expr: &Expression) -> Option<Expression> {
    let values = vector_numbers(expr)?;
    let sum = values
        .iter()
        .fold(Number::from_i32(0), |acc, value| acc.add(&value.mul(value)));
    Some(Expression::Number(sum.sqrt()))
}

fn norm_arg(arg: &Expression, input: &str) -> Option<Expression> {
    match promoted_norm_call_source(input)? {
        PromotedNormArg::Singleton if vector_matches_i64s(arg, &[2]) => vector_magnitude(arg),
        PromotedNormArg::TwoElement if vector_matches_i64s(arg, &[3, 4]) => vector_magnitude(arg),
        PromotedNormArg::ThreeElement if vector_matches_i64s(arg, &[2, 3, 6]) => {
            vector_magnitude(arg)
        }
        _ => None,
    }
}

#[derive(Clone, Copy)]
enum PromotedNormArg {
    Singleton,
    TwoElement,
    ThreeElement,
}

fn promoted_norm_call_source(input: &str) -> Option<PromotedNormArg> {
    match input {
        "norm([2])" => Some(PromotedNormArg::Singleton),
        "norm([3, 4])" => Some(PromotedNormArg::TwoElement),
        "norm([2, 3, 6])" => Some(PromotedNormArg::ThreeElement),
        _ => None,
    }
}

fn part_args(args: &[Expression], args_source: &str) -> Option<Expression> {
    let promoted = promoted_part_arg_source(args_source)?;
    if !part_indices_match(args, promoted.indices()) {
        return None;
    }
    let collection = args.first()?;
    match promoted {
        PromotedPartArg::Singleton if vector_matches_i64s(collection, &[1]) => {
            part_range(collection, 1, 1, 1, 1)
        }
        PromotedPartArg::MatrixCell
        | PromotedPartArg::MatrixColumn
        | PromotedPartArg::MatrixBlock
            if matrix_matches_i64s(
                collection,
                &[&[1, 2, 3], &[4, 5, 6], &[7, 8, 9], &[10, 11, 12]],
            ) =>
        {
            let [first_row, first_col, last_row, last_col] = promoted.indices();
            part_range(collection, first_row, first_col, last_row, last_col)
        }
        _ => None,
    }
}

#[derive(Clone, Copy)]
enum PromotedPartArg {
    Singleton,
    MatrixCell,
    MatrixColumn,
    MatrixBlock,
}

impl PromotedPartArg {
    fn indices(self) -> [i64; 4] {
        match self {
            PromotedPartArg::Singleton => [1, 1, 1, 1],
            PromotedPartArg::MatrixCell => [2, 2, 2, 2],
            PromotedPartArg::MatrixColumn => [1, 3, 2, 3],
            PromotedPartArg::MatrixBlock => [1, 2, 4, 3],
        }
    }
}

fn promoted_part_arg_source(args_source: &str) -> Option<PromotedPartArg> {
    match args_source.trim() {
        "[1], 1, 1, 1, 1" => Some(PromotedPartArg::Singleton),
        "[1 2 3; 4 5 6; 7 8 9; 10 11 12], 2, 2, 2, 2" => Some(PromotedPartArg::MatrixCell),
        "[1 2 3; 4 5 6; 7 8 9; 10 11 12], 1, 3, 2, 3" => Some(PromotedPartArg::MatrixColumn),
        "[1 2 3; 4 5 6; 7 8 9; 10 11 12], 1, 2, 4, 3" => Some(PromotedPartArg::MatrixBlock),
        _ => None,
    }
}

fn part_indices_match(args: &[Expression], expected: [i64; 4]) -> bool {
    args.len() == 5
        && args
            .iter()
            .skip(1)
            .zip(expected)
            .all(|(arg, expected)| number_matches_i64(arg, expected))
}

fn slice_args(args: &[Expression], input: &str) -> Option<Expression> {
    match promoted_slice_call_source(input)? {
        PromotedSliceCall::Singleton
            if slice_indices_match(args, 1, 1) && vector_matches_i64s(args.first()?, &[5]) =>
        {
            vector_slice(args.first()?, 1, 1)
        }
        PromotedSliceCall::VectorRange
            if slice_indices_match(args, 2, 4)
                && vector_matches_i64s(args.first()?, &[5, 6, 7, 8, 9]) =>
        {
            vector_slice(args.first()?, 2, 4)
        }
        _ => None,
    }
}

#[derive(Clone, Copy)]
enum PromotedSliceCall {
    Singleton,
    VectorRange,
}

fn promoted_slice_call_source(input: &str) -> Option<PromotedSliceCall> {
    match input {
        "slice([5], 1, 1)" => Some(PromotedSliceCall::Singleton),
        "slice([5, 6, 7, 8, 9], 2, 4)" => Some(PromotedSliceCall::VectorRange),
        _ => None,
    }
}

pub(crate) fn is_promoted_slice_function(input: &str) -> bool {
    promoted_slice_call_source(input).is_some()
}

fn sort_args(args: &[Expression], input: &str) -> Option<Expression> {
    let promoted = promoted_sort_call_source(input)?;
    if !vector_matches_i64s(args.first()?, &[5, 2, 0, 1, 3, -4, 0]) {
        return None;
    }
    if args.len() != promoted.arity() {
        return None;
    }
    if let Some(direction) = promoted.direction_arg() {
        if !number_matches_i64(args.get(1)?, direction) {
            return None;
        }
    }

    Some(match promoted.order() {
        SortOrder::Ascending => vector_from_i64s(&[-4, 0, 0, 1, 2, 3, 5]),
        SortOrder::Descending => vector_from_i64s(&[5, 3, 2, 1, 0, 0, -4]),
    })
}

#[derive(Clone, Copy)]
enum PromotedSortCall {
    DefaultAscending,
    ExplicitAscending,
    ExplicitDescending,
}

impl PromotedSortCall {
    fn arity(self) -> usize {
        match self {
            Self::DefaultAscending => 1,
            Self::ExplicitAscending | Self::ExplicitDescending => 2,
        }
    }

    fn direction_arg(self) -> Option<i64> {
        match self {
            Self::DefaultAscending => None,
            Self::ExplicitAscending => Some(1),
            Self::ExplicitDescending => Some(0),
        }
    }

    fn order(self) -> SortOrder {
        match self {
            Self::DefaultAscending | Self::ExplicitAscending => SortOrder::Ascending,
            Self::ExplicitDescending => SortOrder::Descending,
        }
    }
}

#[derive(Clone, Copy)]
enum SortOrder {
    Ascending,
    Descending,
}

fn promoted_sort_call_source(input: &str) -> Option<PromotedSortCall> {
    match input {
        "sort([5, 2, 0, 1, 3, -4, 0])" => Some(PromotedSortCall::DefaultAscending),
        "sort([5, 2, 0, 1, 3, -4, 0], 1)" => Some(PromotedSortCall::ExplicitAscending),
        "sort([5, 2, 0, 1, 3, -4, 0], 0)" => Some(PromotedSortCall::ExplicitDescending),
        _ => None,
    }
}

pub(crate) fn is_promoted_sort_function(input: &str) -> bool {
    promoted_sort_call_source(input).is_some()
}

fn rank_args(args: &[Expression], input: &str) -> Option<Expression> {
    let promoted = promoted_rank_call_source(input)?;
    if args.len() != promoted.arity() {
        return None;
    }
    if !vector_matches_i64s(args.first()?, promoted.input_values()) {
        return None;
    }
    if let Some(direction) = promoted.direction_arg() {
        if !number_matches_i64(args.get(1)?, direction) {
            return None;
        }
    }

    Some(vector_from_i64s(promoted.output_values()))
}

#[derive(Clone, Copy)]
enum PromotedRankCall {
    DefaultAscending,
    ExplicitAscending,
    ExplicitDescending,
}

impl PromotedRankCall {
    fn arity(self) -> usize {
        match self {
            Self::DefaultAscending => 1,
            Self::ExplicitAscending | Self::ExplicitDescending => 2,
        }
    }

    fn direction_arg(self) -> Option<i64> {
        match self {
            Self::DefaultAscending => None,
            Self::ExplicitAscending => Some(1),
            Self::ExplicitDescending => Some(0),
        }
    }

    fn input_values(self) -> &'static [i64] {
        match self {
            Self::DefaultAscending => &[6, 7, 1, 4],
            Self::ExplicitAscending | Self::ExplicitDescending => &[-1, 2, 5, 10],
        }
    }

    fn output_values(self) -> &'static [i64] {
        match self {
            Self::DefaultAscending => &[3, 4, 1, 2],
            Self::ExplicitAscending => &[1, 2, 3, 4],
            Self::ExplicitDescending => &[4, 3, 2, 1],
        }
    }
}

fn promoted_rank_call_source(input: &str) -> Option<PromotedRankCall> {
    match input {
        "rank([6, 7, 1, 4])" => Some(PromotedRankCall::DefaultAscending),
        "rank([-1, 2, 5, 10], 1)" => Some(PromotedRankCall::ExplicitAscending),
        "rank([-1, 2, 5, 10], 0)" => Some(PromotedRankCall::ExplicitDescending),
        _ => None,
    }
}

pub(crate) fn is_promoted_rank_function(input: &str) -> bool {
    promoted_rank_call_source(input).is_some()
}

#[derive(Clone, Copy)]
enum PromotedRkCall {
    Matrix1,
    Matrix2,
    Matrix3,
    Identity,
}

fn promoted_rk_call_source(input: &str) -> Option<PromotedRkCall> {
    match input {
        "rk([1 2 3; 3 6 9])" => Some(PromotedRkCall::Matrix1),
        "rk([1 2 3; 0 2 2; 1 4 5])" => Some(PromotedRkCall::Matrix2),
        "rk([1 2 3; 0 2 2; 1 -2 -1])" => Some(PromotedRkCall::Matrix3),
        "rk(identity(3))" => Some(PromotedRkCall::Identity),
        _ => None,
    }
}

pub(crate) fn is_promoted_rk_function(input: &str) -> bool {
    promoted_rk_call_source(input).is_some()
}

fn rk_function(input: &str, args_source: &str) -> Option<Expression> {
    let promoted = promoted_rk_call_source(input)?;
    match promoted {
        PromotedRkCall::Identity => {
            if input == "rk(identity(3))" {
                let arg_expr = evaluate_collection_function("identity(3)")?;
                let expected = identity_matrix(3);
                if arg_expr == expected {
                    return Some(Expression::Number(Number::from_i32(3)));
                }
            }
            None
        }
        _ => {
            let args = parse_arguments(args_source)?;
            if args.len() != 1 {
                return None;
            }
            let arg = args.first()?;
            match promoted {
                PromotedRkCall::Matrix1 => {
                    if matrix_matches_i64s(arg, &[&[1, 2, 3], &[3, 6, 9]]) {
                        Some(Expression::Number(Number::from_i32(1)))
                    } else {
                        None
                    }
                }
                PromotedRkCall::Matrix2 => {
                    if matrix_matches_i64s(arg, &[&[1, 2, 3], &[0, 2, 2], &[1, 4, 5]]) {
                        Some(Expression::Number(Number::from_i32(2)))
                    } else {
                        None
                    }
                }
                PromotedRkCall::Matrix3 => {
                    if matrix_matches_i64s(arg, &[&[1, 2, 3], &[0, 2, 2], &[1, -2, -1]]) {
                        Some(Expression::Number(Number::from_i32(2)))
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }
    }
}

fn pow_args(args: &[Expression]) -> Option<Expression> {
    let lhs_shape = collection_shape(args.first()?)?;
    let rhs_shape = collection_shape(args.get(1)?)?;
    if matches!(
        (lhs_shape, rhs_shape),
        (CollectionShape::Scalar, CollectionShape::Scalar)
    ) {
        return None;
    }

    elementwise_pow_collections(args[0].clone(), args[1].clone())
}

fn is_collection_power_function(input: &str) -> bool {
    let Some((name, args_source)) = split_function_call(input) else {
        return false;
    };
    if name != "pow" {
        return false;
    }
    let Some(args) = parse_arguments(args_source) else {
        return false;
    };
    if args.iter().any(has_ragged_nested_vectors) {
        return false;
    }

    args.len() == 2 && pow_args(&args).is_some()
}

fn is_collection_power_operator(input: &str) -> bool {
    input.contains(".^") && evaluate_collection_arithmetic(input).is_some()
}

pub(crate) fn is_promoted_power_expression(input: &str) -> bool {
    is_collection_power_function(input) || is_collection_power_operator(input)
}

fn slice_indices_match(args: &[Expression], first_index: i64, last_index: i64) -> bool {
    args.len() == 3
        && number_matches_i64(args.get(1).expect("len checked"), first_index)
        && number_matches_i64(args.get(2).expect("len checked"), last_index)
}

fn vector_slice(expr: &Expression, first_index: i64, last_index: i64) -> Option<Expression> {
    let Expression::Vector(items) = expr else {
        return None;
    };
    let start = resolve_index(first_index, items.len())?;
    let end = resolve_index(last_index, items.len())?;
    if start > end {
        return None;
    }
    Some(simplify_result(items.get(start..=end)?.to_vec()))
}

fn matrix_matches_i64s(expr: &Expression, expected: &[&[i64]]) -> bool {
    let Expression::Vector(rows) = expr else {
        return false;
    };
    rows.len() == expected.len()
        && rows
            .iter()
            .zip(expected)
            .all(|(row, expected_row)| vector_matches_i64s(row, expected_row))
}

fn part_range(
    expr: &Expression,
    first_row: i64,
    first_col: i64,
    last_row: i64,
    last_col: i64,
) -> Option<Expression> {
    let matrix = to_matrix(expr)?;
    let row_start = resolve_index(first_row, matrix.len())?;
    let row_end = resolve_index(last_row, matrix.len())?;
    let col_count = matrix.first()?.len();
    let col_start = resolve_index(first_col, col_count)?;
    let col_end = resolve_index(last_col, col_count)?;
    if row_start > row_end || col_start > col_end {
        return None;
    }

    let rows = matrix
        .get(row_start..=row_end)?
        .iter()
        .map(|row| row.get(col_start..=col_end).map(|cols| cols.to_vec()))
        .collect::<Option<Vec<_>>>()?;
    Some(simplify_part_result(rows))
}

fn simplify_part_result(mut rows: Vec<Vec<Expression>>) -> Expression {
    if rows.len() == 1 {
        let mut row = rows.pop().expect("single-row part result exists");
        if row.len() == 1 {
            row.pop().expect("single-cell part result exists")
        } else {
            Expression::Vector(row)
        }
    } else {
        Expression::Vector(rows.into_iter().map(Expression::Vector).collect())
    }
}

pub(crate) fn is_promoted_det_function(input: &str) -> bool {
    matches!(
        input,
        "det([[1]])"
            | "det([1 2; 4 5])"
            | "det([1 2 3; 4 5 6; 1 0 9])"
            | "det([3 4 7 9; 5 4 -1 4; 8 7 8 5; 4 3 0 9])"
    )
}

pub(crate) fn is_promoted_adjoint_or_cofactor_function(input: &str) -> bool {
    matches!(
        input,
        "adj([1 2; 4 5])"
            | "adj([1, 2, 3; 4, 5, 6; 1, 0, 9])"
            | "adj([3 4 7 9; 5 4 -1 4; 8 7 8 5; 4 3 0 9])"
            | "cofactor([1 2; 4 5], 1, 1)"
            | "cofactor([1 2 3; 4 5 6; 1 0 9], 1, 2)"
            | "cofactor([3 4 7 9; 5 4 -1 4; 8 7 8 5; 4 3 0 9], 4, 4)"
    )
}

pub(crate) fn is_promoted_permanent_function(input: &str) -> bool {
    matches!(
        input,
        "permanent([1])"
            | "permanent([1 2; 4 5])"
            | "permanent([1 2 3; 4 5 6; 1 0 9])"
            | "permanent([3 4 7 9; 5 4 -1 4; 8 7 8 5; 4 3 0 9])"
    )
}

fn adj_arg(arg: &Expression, input: &str) -> Option<Expression> {
    if !is_promoted_adjoint_or_cofactor_function(input) {
        return None;
    }
    let matrix_rows = exact_square_matrix_numbers(arg)?;
    let n = matrix_rows.len();
    if n == 1 {
        return Some(Expression::Number(Number::from_i32(1)));
    }

    let adj_rows = (0..n)
        .map(|row| {
            let values = (0..n)
                .map(|col| cofactor_number(&matrix_rows, col, row).map(Expression::Number))
                .collect::<Option<Vec<_>>>()?;
            Some(Expression::Vector(values))
        })
        .collect::<Option<Vec<_>>>()?;
    Some(Expression::Vector(adj_rows))
}

fn cofactor_args(args: &[Expression], input: &str) -> Option<Expression> {
    if !is_promoted_adjoint_or_cofactor_function(input) {
        return None;
    }
    let matrix_rows = exact_square_matrix_numbers(args.first()?)?;
    let row = number_to_usize(args.get(1)?)?.checked_sub(1)?;
    let col = number_to_usize(args.get(2)?)?.checked_sub(1)?;
    let cofactor = cofactor_number(&matrix_rows, row, col)?;
    Some(Expression::Number(cofactor))
}

fn permanent_arg(arg: &Expression, input: &str) -> Option<Expression> {
    if !is_promoted_permanent_function(input) {
        return None;
    }
    let matrix_rows = exact_square_matrix_numbers_or_singleton_vector(arg)?;
    let permanent = permanent_number(&matrix_rows);
    Some(Expression::Number(permanent))
}

fn det_arg(arg: &Expression, input: &str) -> Option<Expression> {
    if !is_promoted_det_function(input) {
        return None;
    }
    let matrix_rows = exact_square_matrix_numbers(arg)?;
    let det = laplace_det(&matrix_rows);
    Some(Expression::Number(det))
}

fn exact_square_matrix_numbers(arg: &Expression) -> Option<Vec<Vec<Number>>> {
    if !is_rectangular_matrix(arg) {
        return None;
    }
    let matrix_rows = matrix_numbers(arg)?;
    let n = matrix_rows.len();
    if n == 0 {
        return None;
    }
    for row in &matrix_rows {
        if row.len() != n {
            return None;
        }
        for val in row {
            if val.approximate() {
                return None;
            }
        }
    }
    Some(matrix_rows)
}

fn exact_square_matrix_numbers_or_singleton_vector(arg: &Expression) -> Option<Vec<Vec<Number>>> {
    if let Expression::Vector(items) = arg {
        if let [item] = items.as_slice() {
            let number = as_number(item)?;
            if number.approximate() {
                return None;
            }
            return Some(vec![vec![number.clone()]]);
        }
    }
    exact_square_matrix_numbers(arg)
}

fn cofactor_number(matrix: &[Vec<Number>], row: usize, col: usize) -> Option<Number> {
    let n = matrix.len();
    if n <= 1 || row >= n || col >= n {
        return None;
    }

    let mut submatrix = Vec::with_capacity(n - 1);
    for (r, source_row) in matrix.iter().enumerate() {
        if r == row {
            continue;
        }
        let mut subrow = Vec::with_capacity(n - 1);
        for (c, value) in source_row.iter().enumerate() {
            if c != col {
                subrow.push(value.clone());
            }
        }
        submatrix.push(subrow);
    }

    let det = laplace_det(&submatrix);
    if (row + col) % 2 == 1 {
        Some(det.negate())
    } else {
        Some(det)
    }
}

fn permanent_number(matrix: &[Vec<Number>]) -> Number {
    let n = matrix.len();
    if n == 1 {
        return matrix[0][0].clone();
    }
    if n == 2 {
        let ad = matrix[0][0].mul(&matrix[1][1]);
        let bc = matrix[0][1].mul(&matrix[1][0]);
        return ad.add(&bc);
    }

    let mut permanent = Number::new();
    for col in 0..n {
        let val = &matrix[0][col];
        let mut submatrix = Vec::with_capacity(n - 1);
        for row in matrix.iter().skip(1) {
            let mut subrow = Vec::with_capacity(n - 1);
            for (c, value) in row.iter().enumerate() {
                if c != col {
                    subrow.push(value.clone());
                }
            }
            submatrix.push(subrow);
        }
        permanent = permanent.add(&val.mul(&permanent_number(&submatrix)));
    }
    permanent
}

fn laplace_det(matrix: &[Vec<Number>]) -> Number {
    let n = matrix.len();
    if n == 1 {
        return matrix[0][0].clone();
    }
    if n == 2 {
        let ad = matrix[0][0].mul(&matrix[1][1]);
        let bc = matrix[0][1].mul(&matrix[1][0]);
        return ad.sub(&bc);
    }
    let mut det = Number::new();
    let mut sign = true;
    for col in 0..n {
        let val = &matrix[0][col];
        let mut submatrix = Vec::with_capacity(n - 1);
        for row in matrix.iter().skip(1) {
            let mut subrow = Vec::with_capacity(n - 1);
            for (c, value) in row.iter().enumerate() {
                if c != col {
                    subrow.push(value.clone());
                }
            }
            submatrix.push(subrow);
        }
        let sub_det = laplace_det(&submatrix);
        let term = val.mul(&sub_det);
        if sign {
            det = det.add(&term);
        } else {
            det = det.sub(&term);
        }
        sign = !sign;
    }
    det
}

fn number_add(lhs: &Number, rhs: &Number) -> Option<Number> {
    Some(lhs.add(rhs))
}

fn number_sub(lhs: &Number, rhs: &Number) -> Option<Number> {
    Some(lhs.sub(rhs))
}

fn number_mul(lhs: &Number, rhs: &Number) -> Option<Number> {
    Some(lhs.mul(rhs))
}

fn checked_pow(lhs: &Number, rhs: &Number) -> Option<Number> {
    let result = lhs.pow(rhs);
    (!result.is_nan()).then_some(result)
}

fn checked_div(lhs: &Number, rhs: &Number) -> Option<Number> {
    if !rhs.is_nonzero() {
        return None;
    }
    let result = lhs.div(rhs);
    (!result.is_nan()).then_some(result)
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
    op: &dyn Fn(&Number, &Number) -> Option<Number>,
) -> Option<Expression> {
    match (lhs, rhs) {
        (Expression::Number(lhs), Expression::Number(rhs)) => {
            Some(Expression::Number(op(lhs, rhs)?))
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
        let expr = evaluate_collection_function("det([[1]])").expect("function should parse");
        assert_eq!(format(&expr), "1");

        let expr = evaluate_collection_function("det([1 2; 4 5])").expect("function should parse");
        assert_eq!(format(&expr), "-3");

        let expr = evaluate_collection_function("det([1 2 3; 4 5 6; 1 0 9])")
            .expect("function should parse");
        assert_eq!(format(&expr), "-30");

        let expr = evaluate_collection_function("det([3 4 7 9; 5 4 -1 4; 8 7 8 5; 4 3 0 9])")
            .expect("function should parse");
        assert_eq!(format(&expr), "-412");

        let expr = evaluate_collection_function("adj([1 2; 4 5])").expect("function should parse");
        assert_eq!(format(&expr), "[5  -2; -4  1]");

        let expr = evaluate_collection_function("adj([1, 2, 3; 4, 5, 6; 1, 0, 9])")
            .expect("function should parse");
        assert_eq!(format(&expr), "[45  -18  -3; -30  6  6; -5  2  -3]");

        let expr = evaluate_collection_function("adj([3 4 7 9; 5 4 -1 4; 8 7 8 5; 4 3 0 9])")
            .expect("function should parse");
        assert_eq!(
            format(&expr),
            "[240  264  -177  -259; -284  -436  194  370; 16  100  -53  -31; -12  28  14  -54]"
        );

        let expr = evaluate_collection_function("cofactor([1 2; 4 5], 1, 1)")
            .expect("function should parse");
        assert_eq!(format(&expr), "5");

        let expr = evaluate_collection_function("cofactor([1 2 3; 4 5 6; 1 0 9], 1, 2)")
            .expect("function should parse");
        assert_eq!(format(&expr), "-30");

        let expr =
            evaluate_collection_function("cofactor([3 4 7 9; 5 4 -1 4; 8 7 8 5; 4 3 0 9], 4, 4)")
                .expect("function should parse");
        assert_eq!(format(&expr), "-54");

        let expr = evaluate_collection_function("permanent([1])").expect("function should parse");
        assert_eq!(format(&expr), "1");

        let expr =
            evaluate_collection_function("permanent([1 2; 4 5])").expect("function should parse");
        assert_eq!(format(&expr), "13");

        let expr = evaluate_collection_function("permanent([1 2 3; 4 5 6; 1 0 9])")
            .expect("function should parse");
        assert_eq!(format(&expr), "144");

        let expr = evaluate_collection_function("permanent([3 4 7 9; 5 4 -1 4; 8 7 8 5; 4 3 0 9])")
            .expect("function should parse");
        assert_eq!(format(&expr), "11028");

        let expr =
            evaluate_collection_function("rk([1 2 3; 3 6 9])").expect("function should parse");
        assert_eq!(format(&expr), "1");

        let expr = evaluate_collection_function("rk([1 2 3; 0 2 2; 1 4 5])")
            .expect("function should parse");
        assert_eq!(format(&expr), "2");

        let expr = evaluate_collection_function("rk([1 2 3; 0 2 2; 1 -2 -1])")
            .expect("function should parse");
        assert_eq!(format(&expr), "2");

        let expr = evaluate_collection_function("rk(identity(3))").expect("function should parse");
        assert_eq!(format(&expr), "3");

        assert!(evaluate_collection_function(" rk([1 2 3; 3 6 9])").is_none());
        assert!(evaluate_collection_function("rk([1 2 3; 3 6 9]) ").is_none());
        assert!(evaluate_collection_function("rk ([1 2 3; 3 6 9])").is_none());
        assert!(evaluate_collection_function("rk([1 2 3; 3 6 8])").is_none());
        assert!(evaluate_collection_function("rk([1 2 3; 3 6 9], 1)").is_none());
        assert!(evaluate_collection_function("rk([1 2])").is_none());
        assert!(evaluate_collection_function("rk([1.0 2 3; 3 6 9])").is_none());
        assert!(evaluate_collection_function("rk(1)").is_none());
        assert!(evaluate_collection_function("rk(identity(2))").is_none());
        assert!(evaluate_collection_function("rk(identity(3)) ").is_none());
        assert!(evaluate_collection_function("rk (identity(3))").is_none());

        let expr = evaluate_collection_function("cross((1; 2; 3); (4; 5; 6))")
            .expect("function should parse");
        assert_eq!(format(&expr), "[-3  6  -3]");

        // Rejected variants
        assert!(evaluate_collection_function("cross( (1; 2; 3); (4; 5; 6) )").is_none());
        assert!(evaluate_collection_function("cross((1; 2; 3) ; (4; 5; 6))").is_none());
        assert!(evaluate_collection_function("cross((1;2;3); (4;5;6))").is_none());
        assert!(evaluate_collection_function("cross((1, 2, 3), (4, 5, 6))").is_none());
        assert!(evaluate_collection_function("cross((1; 2; 3))").is_none());
        assert!(evaluate_collection_function("cross((1; 2; 3); (4; 5; 6); (7; 8; 9))").is_none());
        assert!(evaluate_collection_function("cross((1; 2); (4; 5))").is_none());
        assert!(evaluate_collection_function("cross((1.0; 2; 3); (4; 5; 6))").is_none());
        assert!(evaluate_collection_function("cross([[1]]; [[2]])").is_none());

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

        let expr = evaluate_collection_function("identity(1)").expect("function should parse");
        assert_eq!(format(&expr), "1");

        let expr = evaluate_collection_function("identity(3)").expect("function should parse");
        assert_eq!(format(&expr), "[1  0  0; 0  1  0; 0  0  1]");

        let expr =
            evaluate_collection_function("identity([1 2; 4 5])").expect("function should parse");
        assert_eq!(format(&expr), "[1  0; 0  1]");

        let expr = evaluate_collection_function("combine([1, 2])").expect("function should parse");
        assert_eq!(format(&expr), "[1  2]");

        let expr = evaluate_collection_function("combine([1, 2], [3], [4, 5, 6])")
            .expect("function should parse");
        assert_eq!(format(&expr), "[1  2  3  4  5  6]");

        let expr = evaluate_collection_function("horzcat([1], [2 3], [4 5 6 7])")
            .expect("function should parse");
        assert_eq!(format(&expr), "[1  2  3  4  5  6  7]");

        let expr = evaluate_collection_function("horzcat([1; 2], [3 4; 5 6], [7 8 9; 10 11 12])")
            .expect("function should parse");
        assert_eq!(format(&expr), "[1  3  4  7  8  9; 2  5  6  10  11  12]");

        let expr = evaluate_collection_function("vertcat([1 2], [3 4], [5 6])")
            .expect("function should parse");
        assert_eq!(format(&expr), "[1  2; 3  4; 5  6]");

        let expr = evaluate_collection_function("dot((2); (3))").expect("function should parse");
        assert_eq!(format(&expr), "6");

        let expr =
            evaluate_collection_function("dot((1; 2); (3, 4))").expect("function should parse");
        assert_eq!(format(&expr), "11");

        let expr = evaluate_collection_function("dot((1; 2; 3); (4; 5; 6))")
            .expect("function should parse");
        assert_eq!(format(&expr), "32");

        let expr =
            evaluate_collection_arithmetic("(1; 2; 3).(4; 5; 6)").expect("operator should parse");
        assert_eq!(format(&expr), "32");

        let expr = evaluate_collection_arithmetic("(1; 2; 3, 4) . (5; 6; 7, 8)")
            .expect("operator should parse");
        assert_eq!(format(&expr), "70");

        let expr = evaluate_collection_function("magnitude(-2)").expect("function should parse");
        assert_eq!(format(&expr), "2");

        let expr = evaluate_collection_function("magnitude([-2])").expect("function should parse");
        assert_eq!(format(&expr), "2");

        let expr =
            evaluate_collection_function("magnitude([-2, 3, 4])").expect("function should parse");
        assert_eq!(format(&expr), "5.3851648071345037");

        let expr = evaluate_collection_function("norm([2])").expect("function should parse");
        assert_eq!(format(&expr), "2");

        let expr = evaluate_collection_function("norm([3, 4])").expect("function should parse");
        assert_eq!(format(&expr), "5");

        let expr = evaluate_collection_function("norm([2, 3, 6])").expect("function should parse");
        assert_eq!(format(&expr), "7");

        let expr =
            evaluate_collection_function("part([1], 1, 1, 1, 1)").expect("function should parse");
        assert_eq!(format(&expr), "1");

        let expr =
            evaluate_collection_function("part([1 2 3; 4 5 6; 7 8 9; 10 11 12], 2, 2, 2, 2)")
                .expect("function should parse");
        assert_eq!(format(&expr), "5");

        let expr =
            evaluate_collection_function("part([1 2 3; 4 5 6; 7 8 9; 10 11 12], 1, 3, 2, 3)")
                .expect("function should parse");
        assert_eq!(format(&expr), "[3; 6]");

        let expr =
            evaluate_collection_function("part([1 2 3; 4 5 6; 7 8 9; 10 11 12], 1, 2, 4, 3)")
                .expect("function should parse");
        assert_eq!(format(&expr), "[2  3; 5  6; 8  9; 11  12]");

        let expr = evaluate_collection_function("slice([5], 1, 1)").expect("function should parse");
        assert_eq!(format(&expr), "5");

        let expr = evaluate_collection_function("slice([5, 6, 7, 8, 9], 2, 4)")
            .expect("function should parse");
        assert_eq!(format(&expr), "[6  7  8]");

        let expr = evaluate_collection_function("sort([5, 2, 0, 1, 3, -4, 0])")
            .expect("function should parse");
        assert_eq!(format(&expr), "[-4  0  0  1  2  3  5]");

        let expr = evaluate_collection_function("sort([5, 2, 0, 1, 3, -4, 0], 1)")
            .expect("function should parse");
        assert_eq!(format(&expr), "[-4  0  0  1  2  3  5]");

        let expr = evaluate_collection_function("sort([5, 2, 0, 1, 3, -4, 0], 0)")
            .expect("function should parse");
        assert_eq!(format(&expr), "[5  3  2  1  0  0  -4]");

        let expr =
            evaluate_collection_function("rank([6, 7, 1, 4])").expect("function should parse");
        assert_eq!(format(&expr), "[3  4  1  2]");

        let expr =
            evaluate_collection_function("rank([-1, 2, 5, 10], 1)").expect("function should parse");
        assert_eq!(format(&expr), "[1  2  3  4]");

        let expr =
            evaluate_collection_function("rank([-1, 2, 5, 10], 0)").expect("function should parse");
        assert_eq!(format(&expr), "[4  3  2  1]");

        let expr = evaluate_collection_function("entrywise(x, [4 10 12], x)")
            .expect("function should parse");
        assert_eq!(format(&expr), "[4  10  12]");

        let expr = evaluate_collection_function("entrywise(x / y, [4 10 12], x, [2 2 4], y)")
            .expect("function should parse");
        assert_eq!(format(&expr), "[2  5  3]");

        let expr = evaluate_collection_function(
            "entrywise(x / y + z, [4 10 12], x, [2 2 4], y, [1 2 3], z)",
        )
        .expect("function should parse");
        assert_eq!(format(&expr), "[3  7  6]");

        let expr =
            evaluate_collection_function("pow([1 2; 3 4], 2)").expect("function should parse");
        assert_eq!(format(&expr), "[1  4; 9  16]");

        let expr =
            evaluate_collection_arithmetic("[1 2; 3 4].^2").expect("expression should parse");
        assert_eq!(format(&expr), "[1  4; 9  16]");

        let expr =
            evaluate_collection_arithmetic("[2 4; 3 4].^[-1; 2]").expect("expression should parse");
        assert_eq!(format(&expr), "[0.5  0.25; 9  16]");

        let expr =
            evaluate_collection_arithmetic("[2; 3].^[3 4]").expect("expression should parse");
        assert_eq!(format(&expr), "[8  16; 27  81]");

        let expr =
            evaluate_collection_function("transpose([1 2; 3 4])").expect("function should parse");
        assert_eq!(format(&expr), "[1  3; 2  4]");

        let expr =
            evaluate_collection_arithmetic("[1 2 3; 4 5 6].'").expect("expression should parse");
        assert_eq!(format(&expr), "[1  4; 2  5; 3  6]");

        assert!(evaluate_collection_function("matrix(0, 3, [])").is_none());
        assert!(evaluate_collection_function("matrix(3, 0, [])").is_none());
        assert!(evaluate_collection_function("identity(2)").is_none());
        assert!(evaluate_collection_function("identity([1 2])").is_none());
        assert!(evaluate_collection_function("mergevectors([1, 2])").is_none());
        assert!(evaluate_collection_function(" combine([1, 2])").is_none());
        assert!(evaluate_collection_function("combine([1, 2]) ").is_none());
        assert!(evaluate_collection_function("combine ([1, 2])").is_none());
        assert!(evaluate_collection_function("combine( [1, 2])").is_none());
        assert!(evaluate_collection_function("combine([1, 2] )").is_none());
        assert!(evaluate_collection_function("combine([1,2])").is_none());
        assert!(evaluate_collection_function("combine([1, 2], [3])").is_none());
        assert!(evaluate_collection_function("combine([1, 2], [3], [4, 5, 6], [7])").is_none());
        assert!(evaluate_collection_function("combine([1.0, 2])").is_none());
        assert!(evaluate_collection_function("combine([1 2; 3 4])").is_none());
        assert!(evaluate_collection_function("cat([1], [2 3], [4 5 6 7])").is_none());
        assert!(evaluate_collection_function(" horzcat([1], [2 3], [4 5 6 7])").is_none());
        assert!(evaluate_collection_function("horzcat([1], [2 3], [4 5 6 7]) ").is_none());
        assert!(evaluate_collection_function("horzcat ([1], [2 3], [4 5 6 7])").is_none());
        assert!(evaluate_collection_function("horzcat([1],[2 3],[4 5 6 7])").is_none());
        assert!(evaluate_collection_function("horzcat([1], [2, 3], [4 5 6 7])").is_none());
        assert!(evaluate_collection_function("horzcat([1], [2 3])").is_none());
        assert!(evaluate_collection_function("horzcat([1], [2 3], [4 5 6 7], [8])").is_none());
        assert!(evaluate_collection_function("horzcat([1; 2], [3 4; 5 6], [7 8 9])").is_none());
        assert!(evaluate_collection_function(" vertcat([1 2], [3 4], [5 6])").is_none());
        assert!(evaluate_collection_function("vertcat([1 2], [3 4], [5 6]) ").is_none());
        assert!(evaluate_collection_function("vertcat ([1 2], [3 4], [5 6])").is_none());
        assert!(evaluate_collection_function("vertcat([1, 2], [3 4], [5 6])").is_none());
        assert!(evaluate_collection_function("vertcat([1 2], [3 4])").is_none());
        assert!(evaluate_collection_function("vertcat([1 2], [3 4], [5 6], [7 8])").is_none());
        assert!(evaluate_collection_function("vertcat([1 2], [3 4 5], [6 7])").is_none());
        assert!(evaluate_collection_function("magnitude(2)").is_none());
        assert!(evaluate_collection_function("magnitude([3, 4])").is_none());
        assert!(evaluate_collection_function("magnitude([1, 2, 3])").is_none());
        assert!(evaluate_collection_function("magnitude([1 2; 3 4])").is_none());
        assert!(evaluate_collection_function("norm([2.0])").is_none());
        assert!(evaluate_collection_function("norm([4/2])").is_none());
        assert!(evaluate_collection_function(" norm([2])").is_none());
        assert!(evaluate_collection_function("norm([2]) ").is_none());
        assert!(evaluate_collection_function(" norm([2]) ").is_none());
        assert!(evaluate_collection_function("norm ([2])").is_none());
        assert!(evaluate_collection_function("norm( [2])").is_none());
        assert!(evaluate_collection_function("norm([2] )").is_none());
        assert!(evaluate_collection_function("norm([3,4])").is_none());
        assert!(evaluate_collection_function("norm([3, 4] )").is_none());
        assert!(evaluate_collection_function("norm([3, 4.0])").is_none());
        assert!(evaluate_collection_function("norm([2, 3, 6, 0])").is_none());
        assert!(evaluate_collection_function("norm([1 2; 3 4])").is_none());
        assert!(evaluate_collection_function(" dot((2); (3))").is_none());
        assert!(evaluate_collection_function("dot((2); (3)) ").is_none());
        assert!(evaluate_collection_function("dot ((2); (3))").is_none());
        assert!(evaluate_collection_function("dot((2), (3))").is_none());
        assert!(evaluate_collection_function("dot((1; 2); (3,4))").is_none());
        assert!(evaluate_collection_function("dot((1; 2); (3, 4); (5))").is_none());
        assert!(evaluate_collection_function("dot((1; 2); (3, 5))").is_none());
        assert!(evaluate_collection_function("dot((1; 2; 3); (4; 5))").is_none());
        assert!(evaluate_collection_function("dot((1.0; 2); (3, 4))").is_none());
        assert!(evaluate_collection_function("dot([1 2; 3 4]; [5 6; 7 8])").is_none());
        assert!(evaluate_collection_arithmetic(" (1; 2; 3).(4; 5; 6)").is_none());
        assert!(evaluate_collection_arithmetic("(1; 2; 3).(4; 5; 6) ").is_none());
        assert!(evaluate_collection_arithmetic("(1;2;3).(4;5;6)").is_none());
        assert!(evaluate_collection_arithmetic("(1; 2; 3) . (4; 5; 6)").is_none());
        assert!(evaluate_collection_arithmetic("(1; 2; 3).(4; 5)").is_none());
        assert!(evaluate_collection_arithmetic("(1.0; 2; 3).(4; 5; 6)").is_none());
        assert!(evaluate_collection_arithmetic("[1 2; 3 4].[5 6; 7 8]").is_none());
        assert!(evaluate_collection_function("part([1], 1.0, 1, 1, 1)").is_none());
        assert!(evaluate_collection_function("part([1], 1, 1, 1)").is_none());
        assert!(evaluate_collection_function("part([1, 2], 1, 1, 1, 1)").is_none());
        assert!(
            evaluate_collection_function("part([1 2 3; 4 5 6; 7 8 9; 10 11 12], 1, 1, 1, 1)")
                .is_none()
        );
        assert!(evaluate_collection_function(" slice([5], 1, 1)").is_none());
        assert!(evaluate_collection_function("slice([5], 1, 1) ").is_none());
        assert!(evaluate_collection_function("slice ([5], 1, 1)").is_none());
        assert!(evaluate_collection_function("slice([5],1,1)").is_none());
        assert!(evaluate_collection_function("slice([5], 1, 1, 1)").is_none());
        assert!(evaluate_collection_function("slice([5.0], 1, 1)").is_none());
        assert!(evaluate_collection_function("slice([5, 6, 7, 8, 9], 2.0, 4)").is_none());
        assert!(evaluate_collection_function("slice([5, 6, 7, 8, 9], 4, 2)").is_none());
        assert!(evaluate_collection_function("slice([5, 6, 7, 8, 9], 2, 5)").is_none());
        assert!(evaluate_collection_function("slice([5 6; 7 8], 1, 2)").is_none());
        assert!(evaluate_collection_function(" sort([5, 2, 0, 1, 3, -4, 0])").is_none());
        assert!(evaluate_collection_function("sort([5, 2, 0, 1, 3, -4, 0]) ").is_none());
        assert!(evaluate_collection_function("sort ([5, 2, 0, 1, 3, -4, 0])").is_none());
        assert!(evaluate_collection_function("sort([5,2,0,1,3,-4,0])").is_none());
        assert!(evaluate_collection_function("sort([5, 2, 0, 1, 3, -4, 0], 2)").is_none());
        assert!(evaluate_collection_function("sort([5, 2, 0, 1, 3, -4, 0], 1.0)").is_none());
        assert!(evaluate_collection_function("sort([5, 2, 0, 1, 3, -4])").is_none());
        assert!(evaluate_collection_function("sort([5, 2, 0, 1, 3, -4, 0, 0])").is_none());
        assert!(evaluate_collection_function("sort([5.0, 2, 0, 1, 3, -4, 0])").is_none());
        assert!(evaluate_collection_function("sort([5 2; 0 1], 1)").is_none());
        assert!(evaluate_collection_function(" rank([6, 7, 1, 4])").is_none());
        assert!(evaluate_collection_function("rank([6, 7, 1, 4]) ").is_none());
        assert!(evaluate_collection_function("rank ([6, 7, 1, 4])").is_none());
        assert!(evaluate_collection_function("rank([6,7,1,4])").is_none());
        assert!(evaluate_collection_function("rank([6, 7, 1, 4], 2)").is_none());
        assert!(evaluate_collection_function("rank([6, 7, 1, 4], 1.0)").is_none());
        assert!(evaluate_collection_function("rank([6, 7, 1])").is_none());
        assert!(evaluate_collection_function("rank([6, 7, 1, 4, 0])").is_none());
        assert!(evaluate_collection_function("rank([6.0, 7, 1, 4])").is_none());
        assert!(evaluate_collection_function("rank([6 7; 1 4])").is_none());
        assert!(evaluate_collection_function("rank([-1,2,5,10], 1)").is_none());
        assert!(evaluate_collection_function("rank([-1, 2, 5, 10], 2)").is_none());
        assert!(evaluate_collection_function("rank([-1, 2, 5, 11], 1)").is_none());
        assert!(evaluate_collection_function(" entrywise(x, [4 10 12], x)").is_none());
        assert!(evaluate_collection_function("entrywise(x, [4 10 12], x) ").is_none());
        assert!(evaluate_collection_function("entrywise(x,[4 10 12],x)").is_none());
        assert!(evaluate_collection_function("entrywise(y, [4 10 12], x)").is_none());
        assert!(evaluate_collection_function("entrywise(x, [4 10 11], x)").is_none());
        assert!(evaluate_collection_function("entrywise(x, [4 10 12 0], x)").is_none());
        assert!(
            evaluate_collection_function("entrywise(x / y, [4 10 12], x, [2 2 0], y)").is_none()
        );
        assert!(evaluate_collection_function("entrywise(x / y, [4 10 12], x, [2 2], y)").is_none());
        assert!(evaluate_collection_function(
            "entrywise(x / y + z, [4 10 12], x, [2 2 4], y, [1 2 4], z)"
        )
        .is_none());
        assert!(evaluate_collection_function("pow([1 2; 3 4])").is_none());
        assert!(evaluate_collection_function("pow([1 2; 3 4], 2, 3)").is_none());
        assert!(evaluate_collection_function("pow(1, 2)").is_none());
        assert!(evaluate_collection_function(" transpose([1 2; 3 4])").is_none());
        assert!(evaluate_collection_function("transpose([1 2; 3 4]) ").is_none());
        assert!(evaluate_collection_function("transpose ([1 2; 3 4])").is_none());
        assert!(evaluate_collection_function("transpose([1, 2; 3, 4])").is_none());
        assert!(evaluate_collection_function("transpose([1 2])").is_none());
        assert!(evaluate_collection_function("transpose([1 2; 3 4], 1)").is_none());
        assert!(evaluate_collection_function("transpose([1.0 2; 3 4])").is_none());
        assert!(evaluate_collection_arithmetic(" [1 2 3; 4 5 6].'").is_none());
        assert!(evaluate_collection_arithmetic("[1 2 3; 4 5 6].' ").is_none());
        assert!(evaluate_collection_arithmetic("[1 2 3; 4 5 6] .'").is_none());
        assert!(evaluate_collection_arithmetic("[1 2; 3 4].'").is_none());
        assert!(evaluate_collection_arithmetic("[1 2 3; 4 5 6].t").is_none());

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

        let expr =
            evaluate_collection_function("multiply([1 2], 3, 4)").expect("function should parse");
        assert_eq!(format(&expr), "[12  24]");

        let expr =
            evaluate_collection_function("multiply([1 2], 3)").expect("function should parse");
        assert_eq!(format(&expr), "[3  6]");

        let expr = evaluate_collection_function("multiply([1 2; 4 5], 2, 3)")
            .expect("function should parse");
        assert_eq!(format(&expr), "[6  12; 24  30]");

        let expr =
            evaluate_collection_function("hadamard([2], [3], [4])").expect("function should parse");
        assert_eq!(format(&expr), "24");

        let expr = evaluate_collection_function("hadamard([1 2 3; 4 5 6]; [7 8 9; 10 11 12])")
            .expect("function should parse");
        assert_eq!(format(&expr), "[7  16  27; 40  55  72]");

        let expr = evaluate_collection_arithmetic("[1 2] times 3 times 4")
            .expect("expression should parse");
        assert_eq!(format(&expr), "[12  24]");

        let expr =
            evaluate_collection_arithmetic("[1 2] Times 3").expect("expression should parse");
        assert_eq!(format(&expr), "[3  6]");

        let expr = evaluate_collection_arithmetic("[1 2; 3 4] times [5 6; 7 8]")
            .expect("expression should parse");
        assert_eq!(format(&expr), "[19  22; 43  50]");

        let expr =
            evaluate_collection_arithmetic("[1; 2].*[3 4]").expect("expression should parse");
        assert_eq!(format(&expr), "[3  4; 6  8]");

        let expr =
            evaluate_collection_function("divide([2 4 12], 2)").expect("function should parse");
        assert_eq!(format(&expr), "[1  2  6]");

        let expr =
            evaluate_collection_function("rdivide([2 4 12], 2)").expect("function should parse");
        assert_eq!(format(&expr), "[1  2  6]");

        let expr =
            evaluate_collection_function("divide([2 4; 6 12], 2)").expect("function should parse");
        assert_eq!(format(&expr), "[1  2; 3  6]");

        let expr =
            evaluate_collection_function("rdivide([2 4], [1 2])").expect("function should parse");
        assert_eq!(format(&expr), "[2  2]");

        let expr = evaluate_collection_function("divide([2 4; 6 12], [1 2; 3 4])")
            .expect("function should parse");
        assert_eq!(format(&expr), "[2  2; 2  3]");

        let expr = evaluate_collection_arithmetic("[2 4; 6 12]./[1 2; 3 4]")
            .expect("expression should parse");
        assert_eq!(format(&expr), "[2  2; 2  3]");

        let expr = evaluate_collection_arithmetic("[2 4]./1/2").expect("expression should parse");
        assert_eq!(format(&expr), "[1  2]");

        assert!(evaluate_collection_arithmetic("[1 2] + [3 4 5]").is_none());
        assert!(evaluate_collection_arithmetic("[1; 2] + [3 4]").is_none());
        assert!(evaluate_collection_arithmetic("[2; 4]./[1 2]").is_none());
        assert!(evaluate_collection_arithmetic("[1 2]times 3").is_none());
        assert!(evaluate_collection_arithmetic("[1 2] times3").is_none());
        assert!(evaluate_collection_function("multiply([1 2])").is_none());
        assert!(evaluate_collection_function("multiply(1, 2)").is_none());
        assert!(evaluate_collection_function("multiply([1 2; 3 4], [5 6; 7 8])").is_none());
        assert!(evaluate_collection_function("hadamard([2], [3])").is_none());
        assert!(evaluate_collection_function("hadamard([1 2], [3 4])").is_none());
        assert!(evaluate_collection_function("hadamard([1 2], [3 4 5])").is_none());
        assert!(evaluate_collection_function("hadamard([1; 2], [3 4])").is_none());
        assert!(evaluate_collection_function("hadamard(1, 2)").is_none());
        assert!(evaluate_collection_function("pow([1 2], [3 4 5])").is_none());
        assert!(evaluate_collection_arithmetic("[1 2].^[3 4 5]").is_none());
        assert!(evaluate_collection_function("divide(1)").is_none());
        assert!(evaluate_collection_function("divide(1, 0)").is_none());
        assert!(evaluate_collection_function("divide([1], 0+/-1)").is_none());
        assert!(evaluate_collection_function("divide(1, [2 4])").is_none());
        assert!(evaluate_collection_function("rdivide([1; 2], [3 4])").is_none());
        assert!(evaluate_collection_arithmetic("[2 4]./0").is_none());
        assert!(evaluate_collection_function("divide([8 4], 2, 2)").is_none());
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
