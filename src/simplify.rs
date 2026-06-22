//! Native AST simplifier.
//!
//! Simplifies expressions by constant folding, applying identity rules,
//! normalizing signs, collecting terms, and canonicalizing order.

use crate::ast::{Expression, NaryChildren};
use crate::context::CalculatorContext;
use crate::number::Number;

fn is_constant(expr: &Expression) -> bool {
    match expr {
        Expression::Number(_) => true,
        Expression::Symbolic(_) | Expression::Variable(_) => false,
        Expression::Addition(nary)
        | Expression::Multiplication(nary)
        | Expression::LogicalAnd(nary)
        | Expression::LogicalOr(nary)
        | Expression::BitwiseAnd(nary)
        | Expression::BitwiseOr(nary)
        | Expression::BitwiseXor(nary) => nary.as_slice().iter().all(is_constant),
        Expression::Negate(child)
        | Expression::Inverse(child)
        | Expression::Percent(child)
        | Expression::LogicalNot(child)
        | Expression::BitwiseNot(child) => is_constant(child),
        Expression::Division {
            numerator,
            denominator,
        } => is_constant(numerator) && is_constant(denominator),
        Expression::Power { base, exponent } => is_constant(base) && is_constant(exponent),
        Expression::Remainder { lhs, rhs }
        | Expression::Modulo { lhs, rhs }
        | Expression::IntegerDivision { lhs, rhs }
        | Expression::ShiftLeft { lhs, rhs }
        | Expression::ShiftRight { lhs, rhs }
        | Expression::LogicalXor { lhs, rhs }
        | Expression::Comparison { lhs, rhs, .. } => is_constant(lhs) && is_constant(rhs),
        Expression::FunctionCall { args, .. } => args.iter().all(is_constant),
        Expression::Vector(elems) => elems.iter().all(is_constant),
        _ => false,
    }
}

fn kind_priority(kind: crate::ast::StructureKind) -> u32 {
    use crate::ast::StructureKind;
    match kind {
        StructureKind::Number => 0,
        StructureKind::Symbolic => 1,
        StructureKind::Variable => 2,
        StructureKind::Unit => 3,
        StructureKind::DateTime => 4,
        StructureKind::Addition => 5,
        StructureKind::Multiplication => 6,
        StructureKind::Division => 7,
        StructureKind::Inverse => 8,
        StructureKind::Power => 9,
        StructureKind::Negate => 10,
        StructureKind::Function => 11,
        StructureKind::Vector => 12,
        StructureKind::Remainder => 13,
        StructureKind::Modulo => 14,
        StructureKind::IntegerDivision => 15,
        StructureKind::ShiftLeft => 16,
        StructureKind::ShiftRight => 17,
        StructureKind::Factorial => 18,
        StructureKind::DoubleFactorial => 19,
        StructureKind::MultiFactorial => 20,
        StructureKind::Percent => 21,
        StructureKind::BitwiseAnd => 22,
        StructureKind::BitwiseOr => 23,
        StructureKind::BitwiseXor => 24,
        StructureKind::BitwiseNot => 25,
        StructureKind::LogicalAnd => 26,
        StructureKind::LogicalOr => 27,
        StructureKind::LogicalXor => 28,
        StructureKind::LogicalNot => 29,
        StructureKind::Comparison => 30,
        StructureKind::Parallel => 31,
        StructureKind::Conversion => 32,
        StructureKind::Assignment => 33,
        StructureKind::Undefined => 34,
        StructureKind::Aborted => 35,
    }
}

fn compare_expressions(a: &Expression, b: &Expression) -> std::cmp::Ordering {
    match (a, b) {
        (Expression::Number(_), Expression::Number(_)) => {}
        (Expression::Number(_), _) => return std::cmp::Ordering::Less,
        (_, Expression::Number(_)) => return std::cmp::Ordering::Greater,
        _ => {}
    }
    let kind_a = a.structure_kind();
    let kind_b = b.structure_kind();
    if kind_a != kind_b {
        return kind_priority(kind_a).cmp(&kind_priority(kind_b));
    }
    
    fn compare_slices(sa: &[Expression], sb: &[Expression]) -> std::cmp::Ordering {
        let len_cmp = sa.len().cmp(&sb.len());
        if len_cmp != std::cmp::Ordering::Equal {
            return len_cmp;
        }
        for (ca, cb) in sa.iter().zip(sb.iter()) {
            let c_cmp = compare_expressions(ca, cb);
            if c_cmp != std::cmp::Ordering::Equal {
                return c_cmp;
            }
        }
        std::cmp::Ordering::Equal
    }

    match (a, b) {
        (Expression::Number(na), Expression::Number(nb)) => {
            if let Some(ord) = na.partial_cmp(nb) {
                ord
            } else {
                na.to_string().cmp(&nb.to_string())
            }
        }
        (Expression::Symbolic(sa), Expression::Symbolic(sb)) => sa.name().cmp(sb.name()),
        (Expression::Variable(va), Expression::Variable(vb)) => va.id().cmp(vb.id()),
        (Expression::Negate(ia), Expression::Negate(ib))
        | (Expression::Inverse(ia), Expression::Inverse(ib))
        | (Expression::Percent(ia), Expression::Percent(ib))
        | (Expression::BitwiseNot(ia), Expression::BitwiseNot(ib))
        | (Expression::LogicalNot(ia), Expression::LogicalNot(ib))
        | (Expression::Factorial(ia), Expression::Factorial(ib))
        | (Expression::DoubleFactorial(ia), Expression::DoubleFactorial(ib)) => {
            compare_expressions(ia, ib)
        }
        (Expression::Addition(na), Expression::Addition(nb))
        | (Expression::Multiplication(na), Expression::Multiplication(nb))
        | (Expression::BitwiseAnd(na), Expression::BitwiseAnd(nb))
        | (Expression::BitwiseOr(na), Expression::BitwiseOr(nb))
        | (Expression::BitwiseXor(na), Expression::BitwiseXor(nb))
        | (Expression::LogicalAnd(na), Expression::LogicalAnd(nb))
        | (Expression::LogicalOr(na), Expression::LogicalOr(nb)) => {
            compare_slices(na.as_slice(), nb.as_slice())
        }
        (
            Expression::Division { numerator: na, denominator: da },
            Expression::Division { numerator: nb, denominator: db },
        ) => {
            let n_cmp = compare_expressions(na, nb);
            if n_cmp != std::cmp::Ordering::Equal {
                return n_cmp;
            }
            compare_expressions(da, db)
        }
        (
            Expression::Power { base: ba, exponent: ea },
            Expression::Power { base: bb, exponent: eb },
        ) => {
            let b_cmp = compare_expressions(ba, bb);
            if b_cmp != std::cmp::Ordering::Equal {
                return b_cmp;
            }
            compare_expressions(ea, eb)
        }
        (
            Expression::Remainder { lhs: la, rhs: ra },
            Expression::Remainder { lhs: lb, rhs: rb },
        )
        | (
            Expression::Modulo { lhs: la, rhs: ra },
            Expression::Modulo { lhs: lb, rhs: rb },
        )
        | (
            Expression::IntegerDivision { lhs: la, rhs: ra },
            Expression::IntegerDivision { lhs: lb, rhs: rb },
        )
        | (
            Expression::ShiftLeft { lhs: la, rhs: ra },
            Expression::ShiftLeft { lhs: lb, rhs: rb },
        )
        | (
            Expression::ShiftRight { lhs: la, rhs: ra },
            Expression::ShiftRight { lhs: lb, rhs: rb },
        )
        | (
            Expression::LogicalXor { lhs: la, rhs: ra },
            Expression::LogicalXor { lhs: lb, rhs: rb },
        )
        | (
            Expression::Parallel { lhs: la, rhs: ra },
            Expression::Parallel { lhs: lb, rhs: rb },
        ) => {
            let l_cmp = compare_expressions(la, lb);
            if l_cmp != std::cmp::Ordering::Equal {
                return l_cmp;
            }
            compare_expressions(ra, rb)
        }
        (
            Expression::MultiFactorial { expr: ea, count: ca },
            Expression::MultiFactorial { expr: eb, count: cb },
        ) => {
            let e_cmp = compare_expressions(ea, eb);
            if e_cmp != std::cmp::Ordering::Equal {
                return e_cmp;
            }
            ca.cmp(cb)
        }
        (
            Expression::Unit { unit: ua, prefix: pa, plural: pla },
            Expression::Unit { unit: ub, prefix: pb, plural: plb },
        ) => {
            let u_cmp = ua.id().cmp(ub.id());
            if u_cmp != std::cmp::Ordering::Equal {
                return u_cmp;
            }
            let p_cmp = match (pa, pb) {
                (Some(prefix_a), Some(prefix_b)) => prefix_a.id().cmp(prefix_b.id()),
                (None, None) => std::cmp::Ordering::Equal,
                (None, Some(_)) => std::cmp::Ordering::Less,
                (Some(_), None) => std::cmp::Ordering::Greater,
            };
            if p_cmp != std::cmp::Ordering::Equal {
                return p_cmp;
            }
            pla.cmp(plb)
        }
        (
            Expression::FunctionCall { function: fa, args: aa },
            Expression::FunctionCall { function: fb, args: ab },
        ) => {
            let f_cmp = fa.id().cmp(fb.id());
            if f_cmp != std::cmp::Ordering::Equal {
                return f_cmp;
            }
            compare_slices(aa, ab)
        }
        (Expression::Vector(va), Expression::Vector(vb)) => {
            compare_slices(va, vb)
        }
        (
            Expression::Comparison { op: oa, lhs: la, rhs: ra },
            Expression::Comparison { op: ob, lhs: lb, rhs: rb },
        ) => {
            let op_val = |o: &crate::ast::ComparisonOperator| match o {
                crate::ast::ComparisonOperator::Less => 0,
                crate::ast::ComparisonOperator::Greater => 1,
                crate::ast::ComparisonOperator::LessOrEqual => 2,
                crate::ast::ComparisonOperator::GreaterOrEqual => 3,
                crate::ast::ComparisonOperator::Equal => 4,
                crate::ast::ComparisonOperator::NotEqual => 5,
            };
            let op_cmp = op_val(oa).cmp(&op_val(ob));
            if op_cmp != std::cmp::Ordering::Equal {
                return op_cmp;
            }
            let l_cmp = compare_expressions(la, lb);
            if l_cmp != std::cmp::Ordering::Equal {
                return l_cmp;
            }
            compare_expressions(ra, rb)
        }
        (
            Expression::Conversion { expr: ea, target: ta },
            Expression::Conversion { expr: eb, target: tb },
        ) => {
            let e_cmp = compare_expressions(ea, eb);
            if e_cmp != std::cmp::Ordering::Equal {
                return e_cmp;
            }
            compare_expressions(ta, tb)
        }
        (
            Expression::Assignment { variable: va, value: val_a },
            Expression::Assignment { variable: vb, value: val_b },
        ) => {
            let v_cmp = va.cmp(vb);
            if v_cmp != std::cmp::Ordering::Equal {
                return v_cmp;
            }
            compare_expressions(val_a, val_b)
        }
        (Expression::DateTime(da), Expression::DateTime(db)) => {
            da.source().cmp(db.source())
        }
        (Expression::Undefined, Expression::Undefined) => std::cmp::Ordering::Equal,
        (Expression::Aborted, Expression::Aborted) => std::cmp::Ordering::Equal,
        _ => std::cmp::Ordering::Equal,
    }
}

fn simplify_negate(expr: &Expression) -> Expression {
    match expr {
        Expression::Negate(inner) => *inner.clone(),
        Expression::Number(num) => Expression::Number(num.negate()),
        Expression::Addition(nary) => {
            let mut negated_terms = nary.as_slice().iter().map(simplify_negate).collect::<Vec<_>>();
            negated_terms.sort_by(compare_expressions);
            Expression::Addition(NaryChildren::new(negated_terms).unwrap())
        }
        Expression::Multiplication(nary) => {
            let mut terms = nary.as_slice().to_vec();
            if let Some(pos) = terms.iter().position(|t| matches!(t, Expression::Number(_))) {
                if let Expression::Number(num) = &terms[pos] {
                    let negated_num = num.negate();
                    if negated_num.is_one() {
                        terms.remove(pos);
                    } else {
                        terms[pos] = Expression::Number(negated_num);
                    }
                }
            } else {
                terms.insert(0, Expression::Number(Number::from_i32(-1)));
            }
            if terms.is_empty() {
                Expression::Number(Number::from_i32(1))
            } else if terms.len() == 1 {
                terms.remove(0)
            } else {
                terms.sort_by(compare_expressions);
                Expression::Multiplication(NaryChildren::new(terms).unwrap())
            }
        }
        other => make_product(Number::from_i32(-1), other.clone()),
    }
}

fn is_negative_of(a: &Expression, b: &Expression) -> bool {
    simplify_negate(a) == *b || simplify_negate(b) == *a
}

fn extract_coeff_and_base(expr: &Expression) -> (Number, Expression) {
    match expr {
        Expression::Negate(inner) => {
            let (coeff, base) = extract_coeff_and_base(inner);
            (coeff.negate(), base)
        }
        Expression::Multiplication(nary) => {
            let mut coeff = Number::from_i32(1);
            let mut rest = Vec::new();
            for child in nary.as_slice() {
                match child {
                    Expression::Number(num) => {
                        coeff = coeff.mul(num);
                    }
                    other => {
                        rest.push(other.clone());
                    }
                }
            }
            if rest.is_empty() {
                (coeff, Expression::Number(Number::from_i32(1)))
            } else if rest.len() == 1 {
                (coeff, rest.remove(0))
            } else {
                (coeff, Expression::Multiplication(NaryChildren::new(rest).unwrap()))
            }
        }
        Expression::Number(num) => (num.clone(), Expression::Number(Number::from_i32(1))),
        other => (Number::from_i32(1), other.clone()),
    }
}

fn add_symbolic_term(terms: &mut Vec<(Expression, Number)>, base: Expression, coeff: Number) {
    for (b, c) in terms.iter_mut() {
        if *b == base {
            *c = c.add(&coeff);
            return;
        }
    }
    terms.push((base, coeff));
}

fn make_product(coeff: Number, base: Expression) -> Expression {
    if coeff.is_one() {
        base
    } else if coeff.is_zero() {
        Expression::Number(coeff)
    } else {
        match base {
            Expression::Multiplication(nary) => {
                let mut terms = vec![Expression::Number(coeff)];
                terms.extend(nary.as_slice().to_vec());
                Expression::Multiplication(NaryChildren::new(terms).unwrap())
            }
            other => {
                Expression::Multiplication(NaryChildren::new(vec![Expression::Number(coeff), other]).unwrap())
            }
        }
    }
}

fn extract_base_and_exponent(expr: &Expression) -> (Expression, Number) {
    match expr {
        Expression::Power { base, exponent } => {
            if let Expression::Number(num) = &**exponent {
                (*base.clone(), num.clone())
            } else {
                (expr.clone(), Number::from_i32(1))
            }
        }
        other => (other.clone(), Number::from_i32(1)),
    }
}

fn add_multiplication_term(terms: &mut Vec<(Expression, Number)>, base: Expression, exp: Number) {
    for (b, e) in terms.iter_mut() {
        if *b == base {
            *e = e.add(&exp);
            return;
        }
    }
    terms.push((base, exp));
}

fn make_power(base: Expression, exponent: Number) -> Expression {
    if exponent.is_one() {
        base
    } else if exponent.is_zero() {
        Expression::Number(Number::from_i32(1))
    } else {
        Expression::Power {
            base: Box::new(base),
            exponent: Box::new(Expression::Number(exponent)),
        }
    }
}

fn simplify_rec(expr: &Expression, context: &mut CalculatorContext) -> Expression {
    if is_constant(expr) {
        let messages_backup = context.messages.clone();
        match crate::eval::evaluate_ast(expr, context) {
            Ok(res_expr) => return res_expr,
            Err(_) => {
                context.messages = messages_backup;
            }
        }
    }

    match expr {
        Expression::Negate(child) => {
            let child_simplified = simplify_rec(child, context);
            match child_simplified {
                Expression::Negate(inner) => *inner,
                Expression::Number(num) => Expression::Number(num.negate()),
                other => simplify_negate(&other),
            }
        }
        Expression::Inverse(child) => {
            let child_simplified = simplify_rec(child, context);
            match child_simplified {
                Expression::Inverse(inner) => *inner,
                Expression::Number(num) => {
                    if num.is_zero() {
                        Expression::Inverse(Box::new(Expression::Number(num)))
                    } else {
                        Expression::Number(Number::from_i32(1).div(&num))
                    }
                }
                other => Expression::Inverse(Box::new(other)),
            }
        }
        Expression::Addition(nary) => {
            let mut num_accum = Number::from_i32(0);
            let mut sym_terms = Vec::new();

            for child in nary.as_slice() {
                let child_simplified = simplify_rec(child, context);
                match child_simplified {
                    Expression::Addition(sub_nary) => {
                        for sub_child in sub_nary.as_slice() {
                            let (coeff, base) = extract_coeff_and_base(sub_child);
                            if let Expression::Number(num) = base {
                                num_accum = num_accum.add(&coeff.mul(&num));
                            } else {
                                add_symbolic_term(&mut sym_terms, base, coeff);
                            }
                        }
                    }
                    other => {
                        let (coeff, base) = extract_coeff_and_base(&other);
                        if let Expression::Number(num) = base {
                            num_accum = num_accum.add(&coeff.mul(&num));
                        } else {
                            add_symbolic_term(&mut sym_terms, base, coeff);
                        }
                    }
                }
            }

            let mut final_terms = Vec::new();
            if !num_accum.is_zero() {
                final_terms.push(Expression::Number(num_accum));
            }

            for (base, coeff) in sym_terms {
                if !coeff.is_zero() {
                    if is_negative_of(&base, &Expression::Number(Number::from_i32(0))) {
                        // skip
                    } else {
                        final_terms.push(make_product(coeff, base));
                    }
                }
            }

            if final_terms.is_empty() {
                Expression::Number(Number::from_i32(0))
            } else if final_terms.len() == 1 {
                final_terms.remove(0)
            } else {
                final_terms.sort_by(compare_expressions);
                Expression::Addition(NaryChildren::new(final_terms).unwrap())
            }
        }
        Expression::Multiplication(nary) => {
            let mut num_accum = Number::from_i32(1);
            let mut sym_terms = Vec::new();

            for child in nary.as_slice() {
                let child_simplified = simplify_rec(child, context);
                match child_simplified {
                    Expression::Multiplication(sub_nary) => {
                        for sub_child in sub_nary.as_slice() {
                            match sub_child {
                                Expression::Number(num) => {
                                    num_accum = num_accum.mul(num);
                                }
                                other => {
                                    let (base, exp) = extract_base_and_exponent(&other);
                                    add_multiplication_term(&mut sym_terms, base, exp);
                                }
                            }
                        }
                    }
                    Expression::Number(num) => {
                        num_accum = num_accum.mul(&num);
                    }
                    other => {
                        let (base, exp) = extract_base_and_exponent(&other);
                        add_multiplication_term(&mut sym_terms, base, exp);
                    }
                }
            }

            if num_accum.is_zero() {
                return Expression::Number(num_accum);
            }

            let mut final_terms = Vec::new();
            for (base, exp) in sym_terms {
                if !exp.is_zero() {
                    final_terms.push(make_power(base, exp));
                }
            }

            final_terms.sort_by(compare_expressions);

            if !num_accum.is_one() {
                final_terms.insert(0, Expression::Number(num_accum));
            }

            if final_terms.is_empty() {
                Expression::Number(Number::from_i32(1))
            } else if final_terms.len() == 1 {
                final_terms.remove(0)
            } else {
                Expression::Multiplication(NaryChildren::new(final_terms).unwrap())
            }
        }
        Expression::Division {
            numerator,
            denominator,
        } => {
            let num_simplified = simplify_rec(numerator, context);
            let den_simplified = simplify_rec(denominator, context);
            match (&num_simplified, &den_simplified) {
                (Expression::Number(n), Expression::Number(d)) => {
                    if d.is_zero() {
                        Expression::Division {
                            numerator: Box::new(num_simplified),
                            denominator: Box::new(den_simplified),
                        }
                    } else {
                        Expression::Number(n.div(d))
                    }
                }
                (n, d) => {
                    if let Expression::Number(d_num) = d {
                        if d_num.is_one() {
                            return n.clone();
                        }
                    }
                    if let Expression::Number(n_num) = n {
                        if n_num.is_zero() {
                            return Expression::Number(n_num.clone());
                        }
                    }
                    if let Expression::FunctionCall { function, args } = d {
                        if function.id() == "abs" && args.len() == 1 {
                            if args[0] == *n {
                                return Expression::FunctionCall {
                                    function: crate::ast::FunctionRef::new("sgn".to_owned()),
                                    args: vec![n.clone()],
                                };
                            } else if is_negative_of(&args[0], n) {
                                return Expression::Negate(Box::new(Expression::FunctionCall {
                                    function: crate::ast::FunctionRef::new("sgn".to_owned()),
                                    args: vec![args[0].clone()],
                                }));
                            }
                        }
                    }
                    Expression::Division {
                        numerator: Box::new(num_simplified),
                        denominator: Box::new(den_simplified),
                    }
                }
            }
        }
        Expression::Power { base, exponent } => {
            let base_simplified = simplify_rec(base, context);
            let exp_simplified = simplify_rec(exponent, context);
            match (&base_simplified, &exp_simplified) {
                (Expression::Number(b), Expression::Number(e)) => Expression::Number(b.pow(e)),
                (b, e) => {
                    if let Expression::Number(e_num) = e {
                        if e_num.is_zero() {
                            return Expression::Number(Number::from_i32(1));
                        }
                        if e_num.is_one() {
                            return b.clone();
                        }
                    }
                    if let Expression::Number(b_num) = b {
                        if b_num.is_one() {
                            return Expression::Number(Number::from_i32(1));
                        }
                        if b_num.is_zero() {
                            return Expression::Number(Number::from_i32(0));
                        }
                    }
                    Expression::Power {
                        base: Box::new(base_simplified),
                        exponent: Box::new(exp_simplified),
                    }
                }
            }
        }
        Expression::FunctionCall { function, args } => {
            let mut simplified_args = Vec::new();
            for arg in args {
                simplified_args.push(simplify_rec(arg, context));
            }
            let fid = function.id();
            if fid == "abs" && simplified_args.len() == 1 {
                let arg = &simplified_args[0];
                match arg {
                    Expression::Negate(inner) => {
                        return Expression::FunctionCall {
                            function: function.clone(),
                            args: vec![*inner.clone()],
                        };
                    }
                    Expression::Number(num) => {
                        return Expression::Number(num.abs());
                    }
                    Expression::Addition(nary) => {
                        let terms = nary.as_slice();
                        if terms.len() == 2 {
                            let (t1, t2) = (&terms[0], &terms[1]);
                            let is_abs_t1 = |e: &Expression, possible_abs: &Expression| -> bool {
                                if let Expression::FunctionCall { function: f, args: a } = possible_abs {
                                    f.id() == "abs" && a.len() == 1 && a[0] == *e
                                } else {
                                    false
                                }
                            };
                            let check_pair = |e1: &Expression, e2: &Expression| -> Option<Expression> {
                                let (coeff, base) = extract_coeff_and_base(e2);
                                if coeff == Number::from_i32(-1) && is_abs_t1(e1, &base) {
                                    return Some(Expression::Addition(NaryChildren::new(vec![
                                        Expression::FunctionCall {
                                            function: crate::ast::FunctionRef::new("abs".to_owned()),
                                            args: vec![e1.clone()],
                                        },
                                        simplify_negate(e1),
                                    ]).unwrap()));
                                }
                                None
                            };
                            if let Some(res) = check_pair(t1, t2) {
                                return simplify_rec(&res, context);
                            }
                            if let Some(res) = check_pair(t2, t1) {
                                return simplify_rec(&res, context);
                            }
                        }
                    }
                    _ => {}
                }
                let neg_arg = simplify_negate(arg);
                let cmp_res = compare_expressions(&neg_arg, arg);
                if cmp_res == std::cmp::Ordering::Less {
                    return Expression::FunctionCall {
                        function: function.clone(),
                        args: vec![neg_arg],
                    };
                }
            }
            Expression::FunctionCall {
                function: function.clone(),
                args: simplified_args,
            }
        }
        Expression::Vector(elems) => {
            let mut simplified_elems = Vec::new();
            for elem in elems {
                simplified_elems.push(simplify_rec(elem, context));
            }
            Expression::Vector(simplified_elems)
        }
        other => other.clone(),
    }
}

/// Simplifies a parsed AST expression using the given context.
pub fn simplify_ast(expr: &Expression, context: &mut CalculatorContext) -> Expression {
    simplify_rec(expr, context)
}
