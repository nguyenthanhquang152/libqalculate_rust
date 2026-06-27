use crate::ast::{Expression, NaryChildren};
use crate::context::CalculatorContext;
use crate::number::Number;

fn is_expr_zero(expr: &Expression) -> bool {
    matches!(expr, Expression::Number(num) if num.is_zero())
}

fn contains_var(expr: &Expression, x: &Expression) -> bool {
    if expr == x {
        return true;
    }
    match expr {
        Expression::Negate(inner) => contains_var(inner, x),
        Expression::Addition(nary) | Expression::Multiplication(nary) => {
            nary.as_slice().iter().any(|child| contains_var(child, x))
        }
        Expression::Power { base, exponent } => contains_var(base, x) || contains_var(exponent, x),
        Expression::FunctionCall { args, .. } => args.iter().any(|arg| contains_var(arg, x)),
        _ => false,
    }
}

/// Extracts the main variable of a polynomial expression. If none is specified,
/// it finds the first variable symbol in the expression.
pub fn get_polynomial_variable(expr: &Expression, var_arg: &Expression) -> Expression {
    match var_arg {
        Expression::Symbolic(sym) if sym.name() != "undefined" => var_arg.clone(),
        Expression::Variable(_) => var_arg.clone(),
        _ => {
            let mut vars = Vec::new();
            find_variables(expr, &mut vars);
            if let Some(first_var) = vars.first() {
                first_var.clone()
            } else {
                Expression::Symbolic(crate::ast::Symbol::new("x"))
            }
        }
    }
}

fn find_variables(expr: &Expression, vars: &mut Vec<Expression>) {
    match expr {
        Expression::Symbolic(_) | Expression::Variable(_) => {
            if !vars.contains(expr) {
                vars.push(expr.clone());
            }
        }
        Expression::Negate(inner) => find_variables(inner, vars),
        Expression::Addition(nary) | Expression::Multiplication(nary) => {
            for child in nary.as_slice() {
                find_variables(child, vars);
            }
        }
        Expression::Power { base, exponent } => {
            find_variables(base, vars);
            find_variables(exponent, vars);
        }
        Expression::FunctionCall { args, .. } => {
            for arg in args {
                find_variables(arg, vars);
            }
        }
        _ => {}
    }
}

/// Computes the highest exponent of the variable x in a polynomial expression.
pub fn compute_degree(expr: &Expression, x: &Expression) -> Number {
    if expr == x {
        return Number::from_i32(1);
    }
    match expr {
        Expression::Number(_) | Expression::Symbolic(_) | Expression::Variable(_) => {
            Number::from_i32(0)
        }
        Expression::Negate(inner) => compute_degree(inner, x),
        Expression::Addition(nary) => {
            let mut max_deg = Number::from_i32(0);
            for child in nary.as_slice() {
                let deg = compute_degree(child, x);
                if deg.gt(&max_deg) {
                    max_deg = deg;
                }
            }
            max_deg
        }
        Expression::Multiplication(nary) => {
            let mut sum_deg = Number::from_i32(0);
            for child in nary.as_slice() {
                sum_deg = sum_deg.add(&compute_degree(child, x));
            }
            sum_deg
        }
        Expression::Power { base, exponent } => {
            if let Expression::Number(ref exp_num) = **exponent {
                compute_degree(base, x).mul(exp_num)
            } else {
                Number::from_i32(0)
            }
        }
        Expression::FunctionCall { function, args } => {
            if function.id() == "sqrt" && args.len() == 1 {
                compute_degree(&args[0], x)
                    .mul(&Number::from_rational(crate::number::Rational::new(1, 2)))
            } else {
                Number::from_i32(0)
            }
        }
        _ => Number::from_i32(0),
    }
}

/// Computes the lowest exponent of the variable x in a polynomial expression.
pub fn compute_ldegree(expr: &Expression, x: &Expression) -> Number {
    if !contains_var(expr, x) {
        return Number::from_i32(0);
    }
    if expr == x {
        return Number::from_i32(1);
    }
    match expr {
        Expression::Negate(inner) => compute_ldegree(inner, x),
        Expression::Addition(nary) => {
            let mut min_deg: Option<Number> = None;
            for child in nary.as_slice() {
                let deg = compute_ldegree(child, x);
                if let Some(ref m) = min_deg {
                    if deg.lt(m) {
                        min_deg = Some(deg);
                    }
                } else {
                    min_deg = Some(deg);
                }
            }
            min_deg.unwrap_or_else(|| Number::from_i32(0))
        }
        Expression::Multiplication(nary) => {
            let mut sum_deg = Number::from_i32(0);
            for child in nary.as_slice() {
                sum_deg = sum_deg.add(&compute_ldegree(child, x));
            }
            sum_deg
        }
        Expression::Power { base, exponent } => {
            if let Expression::Number(ref exp_num) = **exponent {
                compute_ldegree(base, x).mul(exp_num)
            } else {
                Number::from_i32(0)
            }
        }
        Expression::FunctionCall { function, args } => {
            if function.id() == "sqrt" && args.len() == 1 {
                compute_ldegree(&args[0], x)
                    .mul(&Number::from_rational(crate::number::Rational::new(1, 2)))
            } else {
                Number::from_i32(0)
            }
        }
        _ => Number::from_i32(0),
    }
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

fn is_expr_equivalent(a: &Expression, b: &Expression, context: &mut CalculatorContext) -> bool {
    if a == b {
        return true;
    }
    let diff = Expression::Addition(
        NaryChildren::new(vec![a.clone(), Expression::Negate(Box::new(b.clone()))]).unwrap(),
    );
    if let Ok(evaluated) = crate::eval::evaluate_ast(&diff, context) {
        if is_expr_zero(&evaluated) {
            return true;
        }
    }
    false
}

fn extract_term_coeff(
    term: &Expression,
    x: &Expression,
    pownr: &Number,
    context: &mut CalculatorContext,
) -> Expression {
    match term {
        Expression::Negate(inner) => {
            let c = extract_term_coeff(inner, x, pownr, context);
            if is_expr_zero(&c) {
                c
            } else {
                Expression::Negate(Box::new(c))
            }
        }
        Expression::Multiplication(nary) => {
            let mut x_factors = Vec::new();
            let mut coeff_factors = Vec::new();
            for factor in nary.as_slice() {
                if contains_var(factor, x) {
                    x_factors.push(factor.clone());
                } else {
                    coeff_factors.push(factor.clone());
                }
            }
            if x_factors.is_empty() {
                if pownr.is_zero() {
                    term.clone()
                } else {
                    Expression::Number(Number::from_i32(0))
                }
            } else {
                let x_expr = if x_factors.len() == 1 {
                    x_factors[0].clone()
                } else {
                    Expression::Multiplication(NaryChildren::new(x_factors).unwrap())
                };
                let x_expr_simp = crate::simplify::simplify_ast(&x_expr, context);
                let expected = make_power(x.clone(), pownr.clone());
                let expected_simp = crate::simplify::simplify_ast(&expected, context);
                if is_expr_equivalent(&x_expr_simp, &expected_simp, context) {
                    if coeff_factors.is_empty() {
                        Expression::Number(Number::from_i32(1))
                    } else if coeff_factors.len() == 1 {
                        coeff_factors[0].clone()
                    } else {
                        Expression::Multiplication(NaryChildren::new(coeff_factors).unwrap())
                    }
                } else {
                    Expression::Number(Number::from_i32(0))
                }
            }
        }
        _ => {
            if !contains_var(term, x) {
                if pownr.is_zero() {
                    term.clone()
                } else {
                    Expression::Number(Number::from_i32(0))
                }
            } else {
                let expected = make_power(x.clone(), pownr.clone());
                let expected_simp = crate::simplify::simplify_ast(&expected, context);
                if is_expr_equivalent(term, &expected_simp, context) {
                    Expression::Number(Number::from_i32(1))
                } else {
                    Expression::Number(Number::from_i32(0))
                }
            }
        }
    }
}

/// Computes the coefficient of x^pownr in a polynomial expression.
pub fn compute_coeff(
    expr: &Expression,
    pownr: &Number,
    x: &Expression,
    context: &mut CalculatorContext,
) -> Expression {
    let expr_simp = crate::simplify::simplify_ast(expr, context);
    let terms = match &expr_simp {
        Expression::Addition(nary) => nary.as_slice(),
        other => std::slice::from_ref(other),
    };
    let mut coeff_terms = Vec::new();
    for term in terms {
        let c = extract_term_coeff(term, x, pownr, context);
        if !is_expr_zero(&c) {
            coeff_terms.push(c);
        }
    }
    let res = if coeff_terms.is_empty() {
        Expression::Number(Number::from_i32(0))
    } else if coeff_terms.len() == 1 {
        coeff_terms[0].clone()
    } else {
        Expression::Addition(NaryChildren::new(coeff_terms).unwrap())
    };
    crate::simplify::simplify_ast(&res, context)
}

/// Helper to determine if an expression has a negative leading coefficient sign.
pub fn has_negative_sign(expr: &Expression) -> bool {
    match expr {
        Expression::Negate(_) => true,
        Expression::Number(num) => num.is_negative(),
        Expression::Multiplication(nary) => {
            let mut negative = false;
            for factor in nary.as_slice() {
                if let Expression::Number(num) = factor {
                    if num.is_negative() {
                        negative = !negative;
                    }
                } else if let Expression::Negate(_) = factor {
                    negative = !negative;
                }
            }
            negative
        }
        _ => false,
    }
}

/// Computes the unit sign of the leading coefficient of a polynomial.
pub fn polynomial_unit(
    expr: &Expression,
    x: &Expression,
    context: &mut CalculatorContext,
) -> Expression {
    let lcoeff = compute_coeff(expr, &compute_degree(expr, x), x, context);
    Expression::Number(Number::from_i32(if has_negative_sign(&lcoeff) {
        -1
    } else {
        1
    }))
}

/// Extracts the overall numerical coefficient from a term multiplication.
pub fn overall_coefficient(expr: &Expression) -> Number {
    match expr {
        Expression::Number(num) => num.clone(),
        Expression::Multiplication(nary) => {
            let mut coeff = Number::from_i32(1);
            for factor in nary.as_slice() {
                coeff = coeff.mul(&overall_coefficient(factor));
            }
            coeff
        }
        Expression::Negate(inner) => overall_coefficient(inner).negate(),
        _ => Number::from_i32(1),
    }
}

/// Computes the overall integer GCD of the coefficients of a polynomial.
pub fn get_integer_content(mpoly: &Expression) -> Number {
    if let Expression::Number(num) = mpoly {
        num.abs()
    } else if let Expression::Addition(nary) = mpoly {
        let mut icontent = rug::Integer::from(0);
        let mut l = rug::Integer::from(1);
        let mut first = true;
        for term in nary.as_slice() {
            let coeff = overall_coefficient(term);
            let num_val = get_rug_integer_numerator(&coeff);
            let den_val = get_rug_integer_denominator(&coeff);
            if first {
                icontent = num_val;
                l = den_val;
                first = false;
            } else {
                icontent = rug::Integer::from(icontent.gcd_ref(&num_val));
                l = rug::Integer::from(l.lcm_ref(&den_val));
            }
        }
        Number::from_rational(crate::number::Rational {
            value: rug::Rational::from((icontent, l)),
        })
    } else if let Expression::Multiplication(_) = mpoly {
        overall_coefficient(mpoly).abs()
    } else if let Expression::Negate(inner) = mpoly {
        overall_coefficient(inner).abs()
    } else {
        Number::from_i32(1)
    }
}

/// Extracts the numerator as a rug Integer from a Rational or Float number value.
pub fn get_rug_integer_numerator(num: &Number) -> rug::Integer {
    let (real, _) = num.to_canonical_ref();
    match &*real {
        crate::number::NumberValue::Rational(r) => r.value.numer().clone(),
        crate::number::NumberValue::Float(f) => {
            if let Some(rational) = rug::Rational::from_f64(f.value()) {
                rational.numer().clone()
            } else {
                rug::Integer::from(0)
            }
        }
        _ => rug::Integer::from(0),
    }
}

/// Extracts the denominator as a rug Integer from a Rational or Float number value.
pub fn get_rug_integer_denominator(num: &Number) -> rug::Integer {
    let (real, _) = num.to_canonical_ref();
    match &*real {
        crate::number::NumberValue::Rational(r) => r.value.denom().clone(),
        crate::number::NumberValue::Float(f) => {
            if let Some(rational) = rug::Rational::from_f64(f.value()) {
                rational.denom().clone()
            } else {
                rug::Integer::from(1)
            }
        }
        _ => rug::Integer::from(1),
    }
}

/// Computes the GCD of two numbers.
pub fn gcd_numbers(a: &Number, b: &Number) -> Number {
    if a.is_zero() {
        return b.clone();
    }
    if b.is_zero() {
        return a.clone();
    }
    let a_num = get_rug_integer_numerator(a);
    let a_den = get_rug_integer_denominator(a);
    let b_num = get_rug_integer_numerator(b);
    let b_den = get_rug_integer_denominator(b);

    let gcd_num = rug_gcd(&a_num, &b_num);
    let lcm_den = rug_lcm(&a_den, &b_den);

    Number::from_rational(crate::number::Rational {
        value: rug::Rational::from((gcd_num, lcm_den)),
    })
}

/// Computes the LCM of two numbers.
pub fn lcm_numbers(a: &Number, b: &Number) -> Number {
    if a.is_zero() || b.is_zero() {
        return Number::from_i32(0);
    }
    let a_num = get_rug_integer_numerator(a);
    let a_den = get_rug_integer_denominator(a);
    let b_num = get_rug_integer_numerator(b);
    let b_den = get_rug_integer_denominator(b);

    let lcm_num = rug_lcm(&a_num, &b_num);
    let gcd_den = rug_gcd(&a_den, &b_den);

    Number::from_rational(crate::number::Rational {
        value: rug::Rational::from((lcm_num, gcd_den)),
    })
}

fn rug_gcd(a: &rug::Integer, b: &rug::Integer) -> rug::Integer {
    rug::Integer::from(a.gcd_ref(b))
}

fn rug_lcm(a: &rug::Integer, b: &rug::Integer) -> rug::Integer {
    rug::Integer::from(a.lcm_ref(b))
}

fn extract_factors(expr: &Expression, factors: &mut Vec<Expression>) {
    match expr {
        Expression::Multiplication(nary) => {
            for factor in nary.as_slice() {
                extract_factors(factor, factors);
            }
        }
        Expression::Power { base, exponent } => {
            if let Expression::Number(ref num) = **exponent {
                if let Some(exp_val) = num.to_integer() {
                    if exp_val.is_positive() {
                        if let Some(val) = exp_val.to_i32() {
                            if val < 10 {
                                for _ in 0..val {
                                    extract_factors(base, factors);
                                }
                                return;
                            }
                        }
                    }
                }
            }
            factors.push(expr.clone());
        }
        _ => {
            factors.push(expr.clone());
        }
    }
}

fn get_symbolic_factors(expr: &Expression) -> Vec<Expression> {
    let mut factors = Vec::new();
    extract_factors(expr, &mut factors);
    factors
        .into_iter()
        .filter(|f| !matches!(f, Expression::Number(_)))
        .collect()
}

/// Computes the GCD of two expressions.
pub fn expression_gcd(
    a: &Expression,
    b: &Expression,
    _context: &mut CalculatorContext,
) -> Expression {
    if is_expr_zero(a) {
        return b.clone();
    }
    if is_expr_zero(b) {
        return a.clone();
    }
    if a == b {
        return a.clone();
    }
    if let (Expression::Number(num_a), Expression::Number(num_b)) = (a, b) {
        return Expression::Number(gcd_numbers(num_a, num_b));
    }

    let ic_a = get_integer_content(a);
    let ic_b = get_integer_content(b);
    let gcd_num = gcd_numbers(&ic_a, &ic_b);

    let sym_a = get_symbolic_factors(a);
    let mut sym_b = get_symbolic_factors(b);
    let mut common_sym = Vec::new();

    for sa in sym_a {
        if is_expr_zero(&sa) {
            continue;
        }
        if let Some(pos) = sym_b.iter().position(|sb| sb == &sa) {
            common_sym.push(sa);
            sym_b.remove(pos);
        }
    }

    if common_sym.is_empty() {
        Expression::Number(gcd_num)
    } else {
        let mut all_factors = Vec::new();
        if !gcd_num.is_one() {
            all_factors.push(Expression::Number(gcd_num));
        }
        all_factors.extend(common_sym);
        if all_factors.len() == 1 {
            all_factors[0].clone()
        } else {
            Expression::Multiplication(NaryChildren::new(all_factors).unwrap())
        }
    }
}

fn divide_expression_by_constant(expr: &Expression, c: &Number) -> Expression {
    if c.is_one() {
        return expr.clone();
    }
    match expr {
        Expression::Addition(nary) => {
            let mut new_terms = Vec::new();
            for term in nary.as_slice() {
                new_terms.push(divide_expression_by_constant(term, c));
            }
            Expression::Addition(NaryChildren::new(new_terms).unwrap())
        }
        Expression::Multiplication(nary) => {
            let mut new_factors = Vec::new();
            let mut divided = false;
            for factor in nary.as_slice() {
                if !divided {
                    if let Expression::Number(num) = factor {
                        new_factors.push(Expression::Number(num.div(c)));
                        divided = true;
                        continue;
                    }
                }
                new_factors.push(factor.clone());
            }
            if !divided {
                new_factors.push(Expression::Number(Number::from_i32(1).div(c)));
            }
            Expression::Multiplication(NaryChildren::new(new_factors).unwrap())
        }
        Expression::Number(num) => Expression::Number(num.div(c)),
        Expression::Negate(inner) => {
            Expression::Negate(Box::new(divide_expression_by_constant(inner, c)))
        }
        _ => Expression::Multiplication(
            NaryChildren::new(vec![
                expr.clone(),
                Expression::Number(Number::from_i32(1).div(c)),
            ])
            .unwrap(),
        ),
    }
}

fn collect_polynomial_degrees(expr: &Expression, x: &Expression, degrees: &mut Vec<Number>) {
    match expr {
        Expression::Addition(nary) => {
            for term in nary.as_slice() {
                let d = compute_degree(term, x);
                if !degrees.contains(&d) {
                    degrees.push(d);
                }
            }
        }
        _ => {
            let d = compute_degree(expr, x);
            degrees.push(d);
        }
    }
}

/// Computes the content (GCD of coefficients) of a polynomial.
pub fn polynomial_content(
    expr: &Expression,
    x: &Expression,
    context: &mut CalculatorContext,
) -> Expression {
    let expr_simp = crate::simplify::simplify_ast(expr, context);
    if is_expr_zero(&expr_simp) {
        return Expression::Number(Number::from_i32(0));
    }
    if !contains_var(&expr_simp, x) {
        return expr_simp.clone();
    }
    if let Expression::Number(num) = &expr_simp {
        return Expression::Number(num.abs());
    }

    let c = get_integer_content(&expr_simp);
    let c_expr = Expression::Number(c.clone());
    let r = if c.is_one() {
        expr_simp.clone()
    } else {
        divide_expression_by_constant(&expr_simp, &c)
    };

    let lcoeff = compute_coeff(&r, &compute_degree(&r, x), x, context);
    if let Expression::Number(ref num) = lcoeff {
        if num.is_integer() {
            return c_expr;
        }
    }

    let deg = compute_degree(&r, x);
    let ldeg = compute_ldegree(&r, x);
    if deg == ldeg {
        let mut final_c = c_expr.clone();
        if polynomial_unit(&lcoeff, x, context) == Expression::Number(Number::from_i32(-1)) {
            final_c = Expression::Negate(Box::new(final_c));
        }
        let prod = Expression::Multiplication(NaryChildren::new(vec![lcoeff, final_c]).unwrap());
        return crate::eval::evaluate_ast(&prod, context).unwrap_or_else(|_| prod.clone());
    }

    let mut degrees = Vec::new();
    collect_polynomial_degrees(&expr_simp, x, &mut degrees);

    let mut mcontent = Expression::Number(Number::from_i32(0));
    for deg_val in degrees {
        let coeff = compute_coeff(&expr_simp, &deg_val, x, context);
        mcontent = expression_gcd(&coeff, &mcontent, context);
        if let Expression::Number(ref num) = mcontent {
            if num.is_one() {
                break;
            }
        }
    }

    if c.is_one() {
        mcontent
    } else {
        let prod = Expression::Multiplication(NaryChildren::new(vec![mcontent, c_expr]).unwrap());
        crate::eval::evaluate_ast(&prod, context).unwrap_or_else(|_| prod.clone())
    }
}

/// Computes the primitive part of a polynomial (expression divided by content).
pub fn polynomial_primpart(
    expr: &Expression,
    x: &Expression,
    context: &mut CalculatorContext,
) -> Expression {
    let expr_simp = crate::simplify::simplify_ast(expr, context);
    if is_expr_zero(&expr_simp) {
        return Expression::Number(Number::from_i32(0));
    }
    if !contains_var(&expr_simp, x) {
        return Expression::Number(Number::from_i32(1));
    }
    if let Expression::Number(_) = expr_simp {
        return Expression::Number(Number::from_i32(1));
    }

    let c_expr = polynomial_content(&expr_simp, x, context);
    if is_expr_zero(&c_expr) {
        return Expression::Number(Number::from_i32(0));
    }
    let is_neg =
        polynomial_unit(&expr_simp, x, context) == Expression::Number(Number::from_i32(-1));
    let divisor = if is_neg {
        Expression::Negate(Box::new(c_expr))
    } else {
        c_expr
    };

    let mut div_num = Number::from_i32(1);
    let mut has_div_num = false;
    match divisor {
        Expression::Number(ref num) => {
            div_num = num.clone();
            has_div_num = true;
        }
        Expression::Negate(ref inner) => {
            if let Expression::Number(ref num) = **inner {
                div_num = num.negate();
                has_div_num = true;
            }
        }
        _ => {}
    }

    if has_div_num {
        divide_expression_by_constant(&expr_simp, &div_num)
    } else {
        let div = Expression::Multiplication(
            NaryChildren::new(vec![
                expr_simp,
                Expression::Power {
                    base: Box::new(divisor),
                    exponent: Box::new(Expression::Number(Number::from_i32(-1))),
                },
            ])
            .unwrap(),
        );
        crate::eval::evaluate_ast(&div, context).unwrap_or_else(|_| div.clone())
    }
}

fn make_sqrt(expr: &Expression) -> Expression {
    match expr {
        Expression::Number(num) => Expression::Number(num.sqrt()),
        Expression::Power { base, exponent } => {
            if let Expression::Number(ref num) = **exponent {
                if let Some(int_val) = num.to_integer() {
                    if int_val.is_even() {
                        let half = int_val / 2;
                        let half_num = Number::from_rational(crate::number::Rational {
                            value: rug::Rational::from(half),
                        });
                        return Expression::Power {
                            base: base.clone(),
                            exponent: Box::new(Expression::Number(half_num)),
                        };
                    }
                }
            }
            Expression::Power {
                base: Box::new(expr.clone()),
                exponent: Box::new(Expression::Number(Number::from_rational(
                    crate::number::Rational::new(1, 2),
                ))),
            }
        }
        _ => {
            if let Expression::Multiplication(nary) = expr {
                let mut coeff = Number::from_i32(1);
                let mut other_factors = Vec::new();
                for factor in nary.as_slice() {
                    if let Expression::Number(num) = factor {
                        coeff = coeff.mul(num);
                    } else {
                        other_factors.push(factor.clone());
                    }
                }
                if !coeff.is_one() {
                    let coeff_sqrt = coeff.sqrt();
                    if other_factors.is_empty() {
                        return Expression::Number(coeff_sqrt);
                    }
                    let rest = if other_factors.len() == 1 {
                        other_factors[0].clone()
                    } else {
                        Expression::Multiplication(NaryChildren::new(other_factors).unwrap())
                    };
                    let rest_sqrt = make_sqrt(&rest);
                    return Expression::Multiplication(
                        NaryChildren::new(vec![Expression::Number(coeff_sqrt), rest_sqrt]).unwrap(),
                    );
                }
            }
            Expression::FunctionCall {
                function: crate::ast::FunctionRef::new("sqrt"),
                args: vec![expr.clone()],
            }
        }
    }
}

/// Simplifies multiplication of radicals into a single radical.
pub fn normalize_radicals(expr: &Expression) -> Expression {
    match expr {
        Expression::Multiplication(nary) => {
            let mut non_sqrts = Vec::new();
            let mut sqrt_args = Vec::new();
            for factor in nary.as_slice() {
                if let Expression::FunctionCall { function, args } = factor {
                    if function.id() == "sqrt" && args.len() == 1 {
                        sqrt_args.push(args[0].clone());
                        continue;
                    }
                }
                if let Expression::Power { base, exponent } = factor {
                    if let Expression::Number(ref num) = **exponent {
                        if num.to_f64() == 0.5 {
                            sqrt_args.push(*base.clone());
                            continue;
                        }
                    }
                }
                non_sqrts.push(normalize_radicals(factor));
            }
            if sqrt_args.is_empty() {
                Expression::Multiplication(NaryChildren::new(non_sqrts).unwrap())
            } else {
                let merged_args = if sqrt_args.len() == 1 {
                    sqrt_args[0].clone()
                } else {
                    Expression::Multiplication(NaryChildren::new(sqrt_args).unwrap())
                };
                let merged_sqrt = Expression::FunctionCall {
                    function: crate::ast::FunctionRef::new("sqrt"),
                    args: vec![merged_args],
                };
                non_sqrts.push(merged_sqrt);
                if non_sqrts.len() == 1 {
                    non_sqrts[0].clone()
                } else {
                    Expression::Multiplication(NaryChildren::new(non_sqrts).unwrap())
                }
            }
        }
        Expression::Power { base, exponent } => Expression::Power {
            base: Box::new(normalize_radicals(base)),
            exponent: exponent.clone(),
        },
        Expression::Addition(nary) => {
            let mut children = Vec::new();
            for child in nary.as_slice() {
                children.push(normalize_radicals(child));
            }
            Expression::Addition(NaryChildren::new(children).unwrap())
        }
        _ => expr.clone(),
    }
}

/// Simplifies quadratic trinomials of the form a*u^2 +- 2ab*uv + b^2*v^2 into (au +- bv)^2.
pub fn match_perfect_square(
    expr: &Expression,
    context: &mut CalculatorContext,
) -> Option<Expression> {
    let terms = match expr {
        Expression::Addition(nary) => nary.as_slice(),
        _ => return None,
    };
    if terms.len() != 3 {
        return None;
    }
    let t0 = &terms[0];
    let t1 = &terms[1];
    let t2 = &terms[2];

    let mut check_perm =
        |s1: &Expression, s2: &Expression, mid: &Expression| -> Option<Expression> {
            if has_negative_sign(s1) || has_negative_sign(s2) {
                return None;
            }
            let a = make_sqrt(s1);
            let b = make_sqrt(s2);
            let two_a_b = Expression::Multiplication(
                NaryChildren::new(vec![
                    Expression::Number(Number::from_i32(2)),
                    a.clone(),
                    b.clone(),
                ])
                .unwrap(),
            );
            let two_a_b_eval = crate::eval::evaluate_ast(&two_a_b, context).ok()?;
            let mid_eval = crate::eval::evaluate_ast(mid, context).ok()?;

            let normalized_two_a_b = normalize_radicals(&two_a_b_eval);
            let normalized_mid = normalize_radicals(&mid_eval);

            if is_expr_equivalent(&normalized_two_a_b, &normalized_mid, context) {
                let sum = Expression::Addition(NaryChildren::new(vec![a, b]).unwrap());
                let sum_eval = crate::eval::evaluate_ast(&sum, context).ok()?;
                return Some(Expression::Power {
                    base: Box::new(sum_eval),
                    exponent: Box::new(Expression::Number(Number::from_i32(2))),
                });
            }
            if is_expr_equivalent(
                &normalized_two_a_b,
                &Expression::Negate(Box::new(normalized_mid.clone())),
                context,
            ) || is_expr_equivalent(
                &Expression::Negate(Box::new(normalized_two_a_b)),
                &normalized_mid,
                context,
            ) {
                let diff = Expression::Addition(
                    NaryChildren::new(vec![a, Expression::Negate(Box::new(b))]).unwrap(),
                );
                let diff_eval = crate::eval::evaluate_ast(&diff, context).ok()?;
                return Some(Expression::Power {
                    base: Box::new(diff_eval),
                    exponent: Box::new(Expression::Number(Number::from_i32(2))),
                });
            }
            None
        };

    if let Some(res) = check_perm(t0, t2, t1) {
        return Some(res);
    }
    if let Some(res) = check_perm(t0, t1, t2) {
        return Some(res);
    }
    if let Some(res) = check_perm(t1, t2, t0) {
        return Some(res);
    }
    None
}

/// Perform integer prime factorization on a rational number.
pub fn factorize_number(num: &Number) -> Option<Vec<Number>> {
    if !num.is_integer() || num.is_zero() {
        return None;
    }
    let mut factors = Vec::new();
    let mut n = get_rug_integer_numerator(num);
    if n == 0 {
        return None;
    }
    let is_neg = n.is_negative();
    if is_neg {
        n = -n;
    }
    if n == 1 {
        if is_neg {
            factors.push(Number::from_i32(-1));
        } else {
            factors.push(Number::from_i32(1));
        }
        return Some(factors);
    }
    if is_neg {
        factors.push(Number::from_i32(-1));
    }

    // Quick primality test to avoid long loops on large primes
    if n.is_probably_prime(25) != rug::integer::IsPrime::No {
        factors.push(Number::from_rational(crate::number::Rational {
            value: rug::Rational::from(n),
        }));
        return Some(factors);
    }

    let mut d = rug::Integer::from(2);
    // Precompute limit to avoid multiplication in loop condition
    let limit = n.clone().sqrt();

    // For safety, limit the maximum bit width of trial division to prevent hang
    let max_trial = 10_000_000;
    let mut iterations = 0;

    while d <= limit {
        iterations += 1;
        if iterations > max_trial {
            break;
        }
        while rug::Integer::from(&n % &d) == 0 {
            n /= &d;
            factors.push(Number::from_rational(crate::number::Rational {
                value: rug::Rational::from(d.clone()),
            }));
        }

        // Quick check if the remaining n is prime
        if n > 1 && n.is_probably_prime(25) != rug::integer::IsPrime::No {
            break;
        }

        d += 1;
    }
    if n > 1 {
        factors.push(Number::from_rational(crate::number::Rational {
            value: rug::Rational::from(n),
        }));
    }
    Some(factors)
}
