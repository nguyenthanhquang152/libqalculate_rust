//! Native AST evaluator for primitive nodes.
//!
//! Evaluates numbers, variables, arithmetic, comparisons, logical/bitwise
//! operators, and simple unitless functions recursively.

use crate::ast::{ComparisonOperator, Expression, NaryChildren, Symbol, VariableRef};
use crate::context::CalculatorContext;
use crate::number::{ComparisonResult, Number, NumberValue};
use crate::parser::names::{NameMatch, NameRegistry};

/// Helper to convert a `Number` to `rug::Integer` by truncating the real part.
fn to_integer(num: &Number) -> Option<rug::Integer> {
    let (real, imag) = num.to_canonical_ref();
    if !imag.is_real_zero() {
        return None;
    }
    match &*real {
        NumberValue::Rational(r) => Some(r.value.clone().trunc().numer().clone()),
        NumberValue::Float(f) => f.rug_float().clone().to_integer(),
        _ => None,
    }
}

/// Helper to log a division by zero warning and return NaN.
fn handle_division_by_zero(context: &mut CalculatorContext) -> Expression {
    let msg = crate::messages::CalculatorMessage::new(
        "Division by zero".to_string(),
        crate::messages::MessageType::Warning,
        crate::messages::MessageCategory::None,
        crate::messages::MessageStage::Calculation,
    );
    context.messages.push(msg);
    Expression::Number(Number::nan())
}

/// Helper to determine the truthiness of an expression.
///
/// Returns `Some(true)` if truthy, `Some(false)` if falsy, and `None` if unknown.
fn is_truthy(expr: &Expression) -> Option<bool> {
    match expr {
        Expression::Number(num) => {
            if num.is_zero() || num.is_nan() {
                Some(false)
            } else {
                Some(true)
            }
        }
        _ => None,
    }
}

/// Evaluates nested addition terms by flattening and folding numerical constants.
fn evaluate_addition(
    nary: &NaryChildren,
    context: &mut CalculatorContext,
) -> Result<Expression, String> {
    let mut numbers = Vec::new();
    let mut other_terms = Vec::new();
    for child in nary.as_slice() {
        let eval_child = evaluate_ast_rec(child, context)?;
        match eval_child {
            Expression::Number(num) => {
                numbers.push(num);
            }
            Expression::Addition(sub_nary) => {
                for sub_child in sub_nary.as_slice() {
                    match sub_child {
                        Expression::Number(num) => numbers.push(num.clone()),
                        other => other_terms.push(other.clone()),
                    }
                }
            }
            other => {
                other_terms.push(other);
            }
        }
    }

    if numbers.is_empty() {
        if other_terms.len() == 1 {
            return Ok(other_terms.remove(0));
        }
        return Ok(Expression::Addition(
            NaryChildren::new(other_terms).map_err(|e| e.to_string())?,
        ));
    }

    let mut sum = numbers.remove(0);
    for num in numbers {
        sum = sum.add(&num);
    }

    if other_terms.is_empty() {
        return Ok(Expression::Number(sum));
    }

    if sum.is_zero() {
        if other_terms.len() == 1 {
            return Ok(other_terms.remove(0));
        }
        return Ok(Expression::Addition(
            NaryChildren::new(other_terms).map_err(|e| e.to_string())?,
        ));
    }

    let mut all_terms = vec![Expression::Number(sum)];
    all_terms.extend(other_terms);
    Ok(Expression::Addition(
        NaryChildren::new(all_terms).map_err(|e| e.to_string())?,
    ))
}

/// Evaluates nested multiplication terms by flattening and folding numerical constants.
fn evaluate_multiplication(
    nary: &NaryChildren,
    context: &mut CalculatorContext,
) -> Result<Expression, String> {
    let mut numbers = Vec::new();
    let mut other_terms = Vec::new();
    for child in nary.as_slice() {
        let eval_child = evaluate_ast_rec(child, context)?;
        match eval_child {
            Expression::Number(num) => {
                numbers.push(num);
            }
            Expression::Multiplication(sub_nary) => {
                for sub_child in sub_nary.as_slice() {
                    match sub_child {
                        Expression::Number(num) => numbers.push(num.clone()),
                        other => other_terms.push(other.clone()),
                    }
                }
            }
            other => {
                other_terms.push(other);
            }
        }
    }

    if numbers.is_empty() {
        if other_terms.len() == 1 {
            return Ok(other_terms.remove(0));
        }
        return Ok(Expression::Multiplication(
            NaryChildren::new(other_terms).map_err(|e| e.to_string())?,
        ));
    }

    let mut product = numbers.remove(0);
    for num in numbers {
        product = product.mul(&num);
    }

    if product.is_zero() {
        return Ok(Expression::Number(product));
    }

    if other_terms.is_empty() {
        return Ok(Expression::Number(product));
    }

    if product.is_one() {
        if other_terms.len() == 1 {
            return Ok(other_terms.remove(0));
        }
        return Ok(Expression::Multiplication(
            NaryChildren::new(other_terms).map_err(|e| e.to_string())?,
        ));
    }

    let mut all_terms = vec![Expression::Number(product)];
    all_terms.extend(other_terms);
    Ok(Expression::Multiplication(
        NaryChildren::new(all_terms).map_err(|e| e.to_string())?,
    ))
}

fn evaluate_symbolic(sym: &Symbol, context: &mut CalculatorContext) -> Result<Expression, String> {
    let name = sym.name();
    if let Some(val) = context.variables.get(name) {
        return evaluate_ast_rec(&val.clone(), context);
    }
    if let Some(match_result) = context.definitions.lookup(name, false) {
        match match_result {
            NameMatch::Variable { .. } => {
                return Ok(Expression::Variable(VariableRef::new(name.to_owned())));
            }
            NameMatch::Unit { definition, prefix } => {
                return Ok(Expression::Unit {
                    unit: crate::ast::UnitRef::new(definition.id().to_owned()),
                    prefix: prefix.map(|p| crate::ast::PrefixRef::new(p.id().to_owned())),
                    plural: false,
                });
            }
            _ => {}
        }
    }
    Ok(Expression::Symbolic(sym.clone()))
}

fn evaluate_variable(
    var_ref: &VariableRef,
    context: &mut CalculatorContext,
) -> Result<Expression, String> {
    let name = var_ref.id();
    if let Some(val) = context.variables.get(name) {
        return evaluate_ast_rec(&val.clone(), context);
    }
    Ok(Expression::Variable(var_ref.clone()))
}

fn evaluate_ast_rec(
    expr: &Expression,
    context: &mut CalculatorContext,
) -> Result<Expression, String> {
    match expr {
        Expression::Number(num) => Ok(Expression::Number(num.clone())),
        Expression::Negate(child) => {
            let child_eval = evaluate_ast_rec(child, context)?;
            match child_eval {
                Expression::Number(num) => Ok(Expression::Number(num.negate())),
                other => Ok(Expression::Negate(Box::new(other))),
            }
        }
        Expression::Inverse(child) => {
            let child_eval = evaluate_ast_rec(child, context)?;
            match child_eval {
                Expression::Number(num) => {
                    if num.is_zero() {
                        Ok(handle_division_by_zero(context))
                    } else {
                        Ok(Expression::Number(Number::from_i32(1).div(&num)))
                    }
                }
                other => Ok(Expression::Inverse(Box::new(other))),
            }
        }
        Expression::Division {
            numerator,
            denominator,
        } => {
            let num_eval = evaluate_ast_rec(numerator, context)?;
            let den_eval = evaluate_ast_rec(denominator, context)?;
            match (num_eval, den_eval) {
                (Expression::Number(n), Expression::Number(d)) => {
                    if d.is_zero() {
                        Ok(handle_division_by_zero(context))
                    } else {
                        Ok(Expression::Number(n.div(&d)))
                    }
                }
                (num, den) => Ok(Expression::Division {
                    numerator: Box::new(num),
                    denominator: Box::new(den),
                }),
            }
        }
        Expression::Addition(nary) => evaluate_addition(nary, context),
        Expression::Multiplication(nary) => evaluate_multiplication(nary, context),
        Expression::Power { base, exponent } => {
            let base_eval = evaluate_ast_rec(base, context)?;
            let exp_eval = evaluate_ast_rec(exponent, context)?;
            match (base_eval, exp_eval) {
                (Expression::Number(b), Expression::Number(e)) => Ok(Expression::Number(b.pow(&e))),
                (b, e) => Ok(Expression::Power {
                    base: Box::new(b),
                    exponent: Box::new(e),
                }),
            }
        }
        Expression::Remainder { lhs, rhs } => {
            let lhs_eval = evaluate_ast_rec(lhs, context)?;
            let rhs_eval = evaluate_ast_rec(rhs, context)?;
            match (lhs_eval, rhs_eval) {
                (Expression::Number(l), Expression::Number(r)) => {
                    if r.is_zero() {
                        Ok(handle_division_by_zero(context))
                    } else {
                        Ok(Expression::Number(l.rem(&r)))
                    }
                }
                (l, r) => Ok(Expression::Remainder {
                    lhs: Box::new(l),
                    rhs: Box::new(r),
                }),
            }
        }
        Expression::Modulo { lhs, rhs } => {
            let lhs_eval = evaluate_ast_rec(lhs, context)?;
            let rhs_eval = evaluate_ast_rec(rhs, context)?;
            match (lhs_eval, rhs_eval) {
                (Expression::Number(l), Expression::Number(r)) => {
                    if r.is_zero() {
                        Ok(handle_division_by_zero(context))
                    } else {
                        Ok(Expression::Number(l.modulo(&r)))
                    }
                }
                (l, r) => Ok(Expression::Modulo {
                    lhs: Box::new(l),
                    rhs: Box::new(r),
                }),
            }
        }
        Expression::IntegerDivision { lhs, rhs } => {
            let lhs_eval = evaluate_ast_rec(lhs, context)?;
            let rhs_eval = evaluate_ast_rec(rhs, context)?;
            match (lhs_eval, rhs_eval) {
                (Expression::Number(l), Expression::Number(r)) => {
                    if r.is_zero() {
                        Ok(handle_division_by_zero(context))
                    } else {
                        Ok(Expression::Number(l.int_div(&r)))
                    }
                }
                (l, r) => Ok(Expression::IntegerDivision {
                    lhs: Box::new(l),
                    rhs: Box::new(r),
                }),
            }
        }
        Expression::Percent(child) => {
            let child_eval = evaluate_ast_rec(child, context)?;
            match child_eval {
                Expression::Number(n) => Ok(Expression::Number(n.div(&Number::from_i32(100)))),
                other => Ok(Expression::Percent(Box::new(other))),
            }
        }
        Expression::ShiftLeft { lhs, rhs } => {
            let lhs_eval = evaluate_ast_rec(lhs, context)?;
            let rhs_eval = evaluate_ast_rec(rhs, context)?;
            match (lhs_eval, rhs_eval) {
                (Expression::Number(l), Expression::Number(r)) => {
                    if let (Some(l_int), Some(r_int)) = (to_integer(&l), to_integer(&r)) {
                        if r_int >= 0 && r_int <= 100_000 {
                            let shift_amount = r_int.to_u32().unwrap();
                            let res = l_int << shift_amount;
                            Ok(Expression::Number(Number::from_rational(
                                crate::number::Rational {
                                    value: rug::Rational::from(res),
                                },
                            )))
                        } else {
                            Ok(Expression::ShiftLeft {
                                lhs: Box::new(Expression::Number(l)),
                                rhs: Box::new(Expression::Number(r)),
                            })
                        }
                    } else {
                        Ok(Expression::ShiftLeft {
                            lhs: Box::new(Expression::Number(l)),
                            rhs: Box::new(Expression::Number(r)),
                        })
                    }
                }
                (l, r) => Ok(Expression::ShiftLeft {
                    lhs: Box::new(l),
                    rhs: Box::new(r),
                }),
            }
        }
        Expression::ShiftRight { lhs, rhs } => {
            let lhs_eval = evaluate_ast_rec(lhs, context)?;
            let rhs_eval = evaluate_ast_rec(rhs, context)?;
            match (lhs_eval, rhs_eval) {
                (Expression::Number(l), Expression::Number(r)) => {
                    if let (Some(l_int), Some(r_int)) = (to_integer(&l), to_integer(&r)) {
                        if r_int >= 0 && r_int <= 100_000 {
                            let shift_amount = r_int.to_u32().unwrap();
                            let res = l_int >> shift_amount;
                            Ok(Expression::Number(Number::from_rational(
                                crate::number::Rational {
                                    value: rug::Rational::from(res),
                                },
                            )))
                        } else {
                            Ok(Expression::ShiftRight {
                                lhs: Box::new(Expression::Number(l)),
                                rhs: Box::new(Expression::Number(r)),
                            })
                        }
                    } else {
                        Ok(Expression::ShiftRight {
                            lhs: Box::new(Expression::Number(l)),
                            rhs: Box::new(Expression::Number(r)),
                        })
                    }
                }
                (l, r) => Ok(Expression::ShiftRight {
                    lhs: Box::new(l),
                    rhs: Box::new(r),
                }),
            }
        }
        Expression::BitwiseAnd(nary) => {
            let mut eval_children = Vec::new();
            for child in nary.as_slice() {
                eval_children.push(evaluate_ast_rec(child, context)?);
            }
            if eval_children
                .iter()
                .all(|c| matches!(c, Expression::Number(_)))
            {
                let mut ints = Vec::new();
                for c in &eval_children {
                    if let Expression::Number(num) = c {
                        if let Some(i) = to_integer(num) {
                            ints.push(i);
                        } else {
                            return Ok(Expression::BitwiseAnd(
                                NaryChildren::new(eval_children).unwrap(),
                            ));
                        }
                    }
                }
                if ints.is_empty() {
                    return Ok(Expression::Number(Number::from_i32(0)));
                }
                let mut res = ints.remove(0);
                for i in ints {
                    res &= i;
                }
                Ok(Expression::Number(Number::from_rational(
                    crate::number::Rational {
                        value: rug::Rational::from(res),
                    },
                )))
            } else {
                Ok(Expression::BitwiseAnd(
                    NaryChildren::new(eval_children).unwrap(),
                ))
            }
        }
        Expression::BitwiseOr(nary) => {
            let mut eval_children = Vec::new();
            for child in nary.as_slice() {
                eval_children.push(evaluate_ast_rec(child, context)?);
            }
            if eval_children
                .iter()
                .all(|c| matches!(c, Expression::Number(_)))
            {
                let mut ints = Vec::new();
                for c in &eval_children {
                    if let Expression::Number(num) = c {
                        if let Some(i) = to_integer(num) {
                            ints.push(i);
                        } else {
                            return Ok(Expression::BitwiseOr(
                                NaryChildren::new(eval_children).unwrap(),
                            ));
                        }
                    }
                }
                if ints.is_empty() {
                    return Ok(Expression::Number(Number::from_i32(0)));
                }
                let mut res = ints.remove(0);
                for i in ints {
                    res |= i;
                }
                Ok(Expression::Number(Number::from_rational(
                    crate::number::Rational {
                        value: rug::Rational::from(res),
                    },
                )))
            } else {
                Ok(Expression::BitwiseOr(
                    NaryChildren::new(eval_children).unwrap(),
                ))
            }
        }
        Expression::BitwiseXor(nary) => {
            let mut eval_children = Vec::new();
            for child in nary.as_slice() {
                eval_children.push(evaluate_ast_rec(child, context)?);
            }
            if eval_children
                .iter()
                .all(|c| matches!(c, Expression::Number(_)))
            {
                let mut ints = Vec::new();
                for c in &eval_children {
                    if let Expression::Number(num) = c {
                        if let Some(i) = to_integer(num) {
                            ints.push(i);
                        } else {
                            return Ok(Expression::BitwiseXor(
                                NaryChildren::new(eval_children).unwrap(),
                            ));
                        }
                    }
                }
                if ints.is_empty() {
                    return Ok(Expression::Number(Number::from_i32(0)));
                }
                let mut res = ints.remove(0);
                for i in ints {
                    res ^= i;
                }
                Ok(Expression::Number(Number::from_rational(
                    crate::number::Rational {
                        value: rug::Rational::from(res),
                    },
                )))
            } else {
                Ok(Expression::BitwiseXor(
                    NaryChildren::new(eval_children).unwrap(),
                ))
            }
        }
        Expression::BitwiseNot(child) => {
            let child_eval = evaluate_ast_rec(child, context)?;
            match child_eval {
                Expression::Number(num) => {
                    if let Some(i) = to_integer(&num) {
                        let res = !i;
                        Ok(Expression::Number(Number::from_rational(
                            crate::number::Rational {
                                value: rug::Rational::from(res),
                            },
                        )))
                    } else {
                        Ok(Expression::BitwiseNot(Box::new(Expression::Number(num))))
                    }
                }
                other => Ok(Expression::BitwiseNot(Box::new(other))),
            }
        }
        Expression::LogicalAnd(nary) => {
            let mut eval_children = Vec::new();
            for child in nary.as_slice() {
                eval_children.push(evaluate_ast_rec(child, context)?);
            }
            for child in &eval_children {
                if let Some(false) = is_truthy(child) {
                    return Ok(Expression::Number(Number::from_i32(0)));
                }
            }
            if eval_children
                .iter()
                .all(|c| matches!(is_truthy(c), Some(true)))
            {
                return Ok(Expression::Number(Number::from_i32(1)));
            }
            let mut remaining = Vec::new();
            for child in eval_children {
                if !matches!(is_truthy(&child), Some(true)) {
                    remaining.push(child);
                }
            }
            if remaining.is_empty() {
                Ok(Expression::Number(Number::from_i32(1)))
            } else if remaining.len() == 1 {
                Ok(remaining.remove(0))
            } else {
                Ok(Expression::LogicalAnd(
                    NaryChildren::new(remaining).unwrap(),
                ))
            }
        }
        Expression::LogicalOr(nary) => {
            let mut eval_children = Vec::new();
            for child in nary.as_slice() {
                eval_children.push(evaluate_ast_rec(child, context)?);
            }
            for child in &eval_children {
                if let Some(true) = is_truthy(child) {
                    return Ok(Expression::Number(Number::from_i32(1)));
                }
            }
            if eval_children
                .iter()
                .all(|c| matches!(is_truthy(c), Some(false)))
            {
                return Ok(Expression::Number(Number::from_i32(0)));
            }
            let mut remaining = Vec::new();
            for child in eval_children {
                if !matches!(is_truthy(&child), Some(false)) {
                    remaining.push(child);
                }
            }
            if remaining.is_empty() {
                Ok(Expression::Number(Number::from_i32(0)))
            } else if remaining.len() == 1 {
                Ok(remaining.remove(0))
            } else {
                Ok(Expression::LogicalOr(NaryChildren::new(remaining).unwrap()))
            }
        }
        Expression::LogicalXor { lhs, rhs } => {
            let lhs_eval = evaluate_ast_rec(lhs, context)?;
            let rhs_eval = evaluate_ast_rec(rhs, context)?;
            match (is_truthy(&lhs_eval), is_truthy(&rhs_eval)) {
                (Some(l), Some(r)) => Ok(Expression::Number(Number::from_i32(if l != r {
                    1
                } else {
                    0
                }))),
                _ => Ok(Expression::LogicalXor {
                    lhs: Box::new(lhs_eval),
                    rhs: Box::new(rhs_eval),
                }),
            }
        }
        Expression::LogicalNot(child) => {
            let child_eval = evaluate_ast_rec(child, context)?;
            match is_truthy(&child_eval) {
                Some(true) => Ok(Expression::Number(Number::from_i32(0))),
                Some(false) => Ok(Expression::Number(Number::from_i32(1))),
                None => Ok(Expression::LogicalNot(Box::new(child_eval))),
            }
        }
        Expression::Comparison { op, lhs, rhs } => {
            let lhs_eval = evaluate_ast_rec(lhs, context)?;
            let rhs_eval = evaluate_ast_rec(rhs, context)?;
            match (&lhs_eval, &rhs_eval) {
                (Expression::Number(l), Expression::Number(r)) => {
                    let cmp = l.compare(r);
                    let result = match op {
                        ComparisonOperator::Equal => {
                            cmp == ComparisonResult::Equal || cmp == ComparisonResult::EqualLimits
                        }
                        ComparisonOperator::NotEqual => cmp == ComparisonResult::NotEqual,
                        ComparisonOperator::Less => cmp == ComparisonResult::Greater,
                        ComparisonOperator::Greater => cmp == ComparisonResult::Less,
                        ComparisonOperator::LessOrEqual => {
                            cmp == ComparisonResult::EqualOrGreater
                                || cmp == ComparisonResult::Equal
                                || cmp == ComparisonResult::EqualLimits
                        }
                        ComparisonOperator::GreaterOrEqual => {
                            cmp == ComparisonResult::EqualOrLess
                                || cmp == ComparisonResult::Equal
                                || cmp == ComparisonResult::EqualLimits
                        }
                    };
                    Ok(Expression::Number(Number::from_i32(if result {
                        1
                    } else {
                        0
                    })))
                }
                _ => Ok(Expression::Comparison {
                    op: *op,
                    lhs: Box::new(lhs_eval),
                    rhs: Box::new(rhs_eval),
                }),
            }
        }
        Expression::Assignment { variable, value } => {
            let val_eval = evaluate_ast_rec(value, context)?;
            context.variables.insert(variable.clone(), val_eval.clone());
            Ok(val_eval)
        }
        Expression::Variable(var_ref) => evaluate_variable(var_ref, context),
        Expression::Symbolic(sym) => evaluate_symbolic(sym, context),
        Expression::FunctionCall { function, args } => {
            let mut args_eval = Vec::new();
            for arg in args {
                args_eval.push(evaluate_ast_rec(arg, context)?);
            }
            let fid = function.id();
            if fid == "abs" && args_eval.len() == 1 {
                if let Expression::Number(num) = &args_eval[0] {
                    return Ok(Expression::Number(num.abs()));
                }
            } else if fid == "sqrt" && args_eval.len() == 1 {
                if let Expression::Number(num) = &args_eval[0] {
                    return Ok(Expression::Number(num.sqrt()));
                }
            } else if fid == "ln" && args_eval.len() == 1 {
                if let Expression::Number(num) = &args_eval[0] {
                    return Ok(Expression::Number(num.ln()));
                }
            }

            if let Some(NameMatch::Function {
                min_args, max_args, ..
            }) = context.definitions.lookup(fid, true)
            {
                if args_eval.len() < min_args || max_args.is_some_and(|max| args_eval.len() > max) {
                    let msg = crate::messages::CalculatorMessage::new(
                        format!("Invalid number of arguments for function '{}'", fid),
                        crate::messages::MessageType::Error,
                        crate::messages::MessageCategory::None,
                        crate::messages::MessageStage::Calculation,
                    );
                    context.messages.push(msg);
                    return Err(format!(
                        "Invalid number of arguments for function '{}'",
                        fid
                    ));
                }
            }
            Ok(Expression::FunctionCall {
                function: function.clone(),
                args: args_eval,
            })
        }
        Expression::Vector(elems) => {
            let mut eval_elems = Vec::new();
            for elem in elems {
                eval_elems.push(evaluate_ast_rec(elem, context)?);
            }
            Ok(Expression::Vector(eval_elems))
        }
        other => Ok(other.clone()),
    }
}

/// Evaluates a parsed AST expression using the given context.
pub fn evaluate_ast(
    expr: &Expression,
    context: &mut CalculatorContext,
) -> Result<Expression, String> {
    evaluate_ast_rec(expr, context)
}
