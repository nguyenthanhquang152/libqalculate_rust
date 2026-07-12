//! Native unit conversion for the focused fallback-disabled compatibility slice.

use crate::ast::Expression;
use crate::context::CalculatorContext;
use crate::number::{Number, NumberValue};
use crate::parser::operators::parse_expression;
use crate::units::{PrefixKind, PrefixUnitCatalog, UnitKind};
use std::collections::{BTreeMap, HashSet};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NativeUnitOutput {
    pub(crate) output: String,
    pub(crate) answer: Expression,
}

#[derive(Debug, Clone, PartialEq)]
struct UnitProduct {
    factor: Number,
    dimensions: BTreeMap<String, i32>,
}

#[derive(Debug, Clone, PartialEq)]
struct Quantity {
    value: Number,
    units: UnitProduct,
}

#[derive(Debug, Clone)]
enum ConversionTarget {
    Base,
    BestBinaryByte,
    Unit {
        units: UnitProduct,
        display: String,
        mixed_feet: bool,
    },
}

impl UnitProduct {
    fn unitless() -> Self {
        Self {
            factor: Number::one(),
            dimensions: BTreeMap::new(),
        }
    }

    fn base_unit(name: &str, factor: Number) -> Self {
        let mut dimensions = BTreeMap::new();
        dimensions.insert(name.to_string(), 1);
        Self { factor, dimensions }
    }

    fn mul(&self, other: &Self) -> Self {
        let mut out = Self {
            factor: self.factor.mul(&other.factor),
            dimensions: self.dimensions.clone(),
        };
        for (name, exponent) in &other.dimensions {
            *out.dimensions.entry(name.clone()).or_insert(0) += exponent;
        }
        out.clean()
    }

    fn div(&self, other: &Self) -> Self {
        self.mul(&other.pow(-1))
    }

    fn pow(&self, exponent: i32) -> Self {
        let dimensions = self
            .dimensions
            .iter()
            .filter_map(|(name, power)| {
                let next = power.saturating_mul(exponent);
                (next != 0).then(|| (name.clone(), next))
            })
            .collect();
        Self {
            factor: pow_number_i32(&self.factor, exponent),
            dimensions,
        }
        .clean()
    }

    fn clean(mut self) -> Self {
        self.dimensions.retain(|_, exponent| *exponent != 0);
        self
    }

    fn is_unitless(&self) -> bool {
        self.dimensions.is_empty()
    }

    fn same_dimensions(&self, other: &Self) -> bool {
        self.dimensions == other.dimensions
    }

    fn has_inverse_dimensions_of(&self, other: &Self) -> bool {
        self.dimensions.len() == other.dimensions.len()
            && self.dimensions.iter().all(|(name, exponent)| {
                other
                    .dimensions
                    .get(name)
                    .is_some_and(|other_exponent| *other_exponent == -*exponent)
            })
    }
}

impl Quantity {
    fn number(value: Number) -> Self {
        Self {
            value,
            units: UnitProduct::unitless(),
        }
    }

    fn unit(units: UnitProduct) -> Self {
        Self {
            value: Number::one(),
            units,
        }
    }

    fn mul(&self, other: &Self) -> Self {
        Self {
            value: self.value.mul(&other.value),
            units: self.units.mul(&other.units),
        }
    }

    fn div(&self, other: &Self) -> Self {
        Self {
            value: self.value.div(&other.value),
            units: self.units.div(&other.units),
        }
    }

    fn pow(&self, exponent: i32) -> Self {
        Self {
            value: pow_number_i32(&self.value, exponent),
            units: self.units.pow(exponent),
        }
    }

    fn negate(&self) -> Self {
        Self {
            value: self.value.negate(),
            units: self.units.clone(),
        }
    }

    fn base_value(&self) -> Number {
        self.value.mul(&self.units.factor)
    }
}

/// Evaluates a qalc expression and retains its structured quantity result.
pub(crate) fn native_output_with_answer(expr: &str) -> Result<Option<NativeUnitOutput>, String> {
    let parsed = match parse_expression(expr) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };
    if !may_contain_unit_candidate(&parsed) {
        return Ok(None);
    }

    let catalog = crate::units::load_prefix_unit_catalog_from_dir(crate::rates::definitions_dir())
        .map_err(|error| error.to_string())?;

    let mut context = CalculatorContext::default();
    catalog.register_into(&mut context.definitions);

    native_output_for_parsed(parsed, &catalog, &mut context, &format_number)
}

/// Evaluates a qalc unit expression with caller-owned definitions and session state.
pub(crate) fn native_output_with_catalog(
    parsed: Expression,
    catalog: &PrefixUnitCatalog,
    context: &mut CalculatorContext,
    format_number: &dyn Fn(&Number) -> String,
) -> Result<Option<NativeUnitOutput>, String> {
    native_output_for_parsed(parsed, catalog, context, format_number)
}

fn native_output_for_parsed(
    parsed: Expression,
    catalog: &PrefixUnitCatalog,
    context: &mut CalculatorContext,
    format_number: &dyn Fn(&Number) -> String,
) -> Result<Option<NativeUnitOutput>, String> {
    match parsed {
        Expression::Conversion { expr, target } => {
            let evaluated_expr = crate::eval::evaluate_ast(&expr, context)?;
            let mut source = match reduce_quantity(&evaluated_expr, catalog) {
                Ok(source) => source,
                Err(_) => return Ok(None),
            };
            if source.units.is_unitless()
                && crate::eval::is_supported_number_conversion_target(&target)
            {
                return Ok(None);
            }
            let evaluated_target = crate::eval::evaluate_ast(&target, context)?;
            let target = conversion_target(&evaluated_target, catalog, format_number)?;
            if source.units.is_unitless() {
                let ConversionTarget::Unit { units, .. } = &target else {
                    return Ok(None);
                };
                source.units = UnitProduct {
                    factor: Number::one(),
                    dimensions: units.dimensions.clone(),
                };
            }
            Ok(Some(NativeUnitOutput {
                output: format_conversion(&source, target, catalog, format_number)?,
                answer: evaluated_expr,
            }))
        }
        other => {
            let evaluated = crate::eval::evaluate_ast(&other, context)?;
            let quantity = match reduce_quantity(&evaluated, catalog) {
                Ok(quantity) => quantity,
                Err(_) => return Ok(None),
            };
            if quantity.units.is_unitless() {
                return Ok(None);
            }
            Ok(
                format_automatic_quantity(&quantity, catalog, format_number).map(|output| {
                    NativeUnitOutput {
                        output,
                        answer: evaluated,
                    }
                }),
            )
        }
    }
}

pub(crate) fn may_contain_unit_candidate(expr: &Expression) -> bool {
    match expr {
        Expression::Symbolic(symbol) => symbol_looks_like_unit(symbol.name()),
        Expression::Multiplication(children)
        | Expression::Addition(children)
        | Expression::BitwiseAnd(children)
        | Expression::BitwiseOr(children)
        | Expression::BitwiseXor(children)
        | Expression::LogicalAnd(children)
        | Expression::LogicalOr(children) => {
            children.as_slice().iter().any(may_contain_unit_candidate)
        }
        Expression::Conversion { expr, target } => {
            may_contain_unit_candidate(expr) || may_contain_unit_candidate(target)
        }
        Expression::Division {
            numerator,
            denominator,
        } => may_contain_unit_candidate(numerator) || may_contain_unit_candidate(denominator),
        Expression::Power { base, exponent } => {
            may_contain_unit_candidate(base) || may_contain_unit_candidate(exponent)
        }
        Expression::Negate(child)
        | Expression::Inverse(child)
        | Expression::Factorial(child)
        | Expression::DoubleFactorial(child)
        | Expression::Percent(child)
        | Expression::LogicalNot(child)
        | Expression::BitwiseNot(child) => may_contain_unit_candidate(child),
        Expression::MultiFactorial { expr, .. } => may_contain_unit_candidate(expr),
        Expression::Remainder { lhs, rhs }
        | Expression::Modulo { lhs, rhs }
        | Expression::IntegerDivision { lhs, rhs }
        | Expression::ShiftLeft { lhs, rhs }
        | Expression::ShiftRight { lhs, rhs }
        | Expression::LogicalXor { lhs, rhs }
        | Expression::Parallel { lhs, rhs }
        | Expression::Comparison { lhs, rhs, .. } => {
            may_contain_unit_candidate(lhs) || may_contain_unit_candidate(rhs)
        }
        Expression::FunctionCall { args, .. } | Expression::Vector(args) => {
            args.iter().any(may_contain_unit_candidate)
        }
        Expression::Assignment { value, .. } => may_contain_unit_candidate(value),
        Expression::Unit { .. } => true,
        Expression::Number(_)
        | Expression::Text(_)
        | Expression::Variable(_)
        | Expression::Undefined
        | Expression::Aborted
        | Expression::DateTime(_) => false,
    }
}

fn symbol_looks_like_unit(name: &str) -> bool {
    const UNITS: &[&str] = &[
        "Ω",
        "ohm",
        "ohms",
        "m",
        "dm",
        "cm",
        "mm",
        "km",
        "L",
        "l",
        "ft",
        "foot",
        "feet",
        "in",
        "inch",
        "inches",
        "yd",
        "yard",
        "yards",
        "mi",
        "mile",
        "miles",
        "h",
        "hr",
        "hrs",
        "hour",
        "hours",
        "min",
        "minute",
        "minutes",
        "s",
        "N",
        "newton",
        "newtons",
        "Pa",
        "pascal",
        "pascals",
        "lbf",
        "mph",
        "hp",
        "horsepower",
        "A",
        "V",
        "W",
        "J",
        "g",
        "kg",
        "bit",
        "bits",
        "byte",
        "bytes",
        "B",
        "megabit",
        "Mbit",
        "GiB",
        "b?byte",
        "kmph",
        "kph",
    ];
    if UNITS.contains(&name) {
        return true;
    }

    const PREFIXES: &[&str] = &[
        "y", "z", "a", "f", "p", "n", "u", "μ", "m", "c", "d", "da", "h", "k", "M", "G", "T", "P",
        "E", "Z", "Y", "Ki", "Mi", "Gi", "deci", "centi", "milli", "micro", "kilo", "mega", "giga",
        "gibi",
    ];
    const PREFIXABLE_UNITS: &[&str] = &["m", "meter", "metre", "bit", "byte", "B", "L", "l"];
    PREFIXES.iter().any(|prefix| {
        name.strip_prefix(prefix)
            .is_some_and(|rest| PREFIXABLE_UNITS.contains(&rest))
    })
}

fn conversion_target(
    target: &Expression,
    catalog: &PrefixUnitCatalog,
    format_number: &dyn Fn(&Number) -> String,
) -> Result<ConversionTarget, String> {
    if let Expression::Symbolic(symbol) = target {
        match symbol.name() {
            "base" => return Ok(ConversionTarget::Base),
            "b?byte" => return Ok(ConversionTarget::BestBinaryByte),
            _ => {}
        }
    }

    let (target_expr, mixed_feet) = match target {
        Expression::Negate(inner) => (inner.as_ref(), false),
        _ => (
            target,
            target_display(target, format_number).is_some_and(|display| display == "ft"),
        ),
    };
    let quantity = reduce_quantity(target_expr, catalog)?;
    if !quantity.value.is_one() {
        return Err("unit conversion target must be unit-only".to_string());
    }
    let display = target_display(target_expr, format_number)
        .ok_or_else(|| "unsupported unit conversion target display".to_string())?;
    Ok(ConversionTarget::Unit {
        units: quantity.units,
        display,
        mixed_feet,
    })
}

fn format_conversion(
    source: &Quantity,
    target: ConversionTarget,
    catalog: &PrefixUnitCatalog,
    format_number: &dyn Fn(&Number) -> String,
) -> Result<String, String> {
    match target {
        ConversionTarget::Base => {
            let unit = format_base_dimensions(&source.units.dimensions)?;
            Ok(format!("{} {unit}", format_number(&source.base_value())))
        }
        ConversionTarget::BestBinaryByte => {
            let (target, display) = best_binary_byte_target(source, catalog)?;
            if !source.units.same_dimensions(&target) {
                return Err("incompatible unit conversion to binary byte".to_string());
            }
            let value = source.base_value().div(&target.factor);
            Ok(format!("{} {display}", format_number(&value)))
        }
        ConversionTarget::Unit {
            units,
            display,
            mixed_feet,
        } => {
            if mixed_feet && display == "ft" {
                if !source.units.same_dimensions(&units) {
                    return Err("incompatible unit conversion target".to_string());
                }
                return format_mixed_feet(source, &units, catalog, format_number);
            }
            let value = if source.units.same_dimensions(&units) {
                source.base_value().div(&units.factor)
            } else if source.units.has_inverse_dimensions_of(&units) {
                Number::one().div(&source.base_value()).div(&units.factor)
            } else {
                return Err("incompatible unit conversion target".to_string());
            };
            Ok(format!("{} {display}", format_number(&value)))
        }
    }
}

fn format_automatic_quantity(
    quantity: &Quantity,
    catalog: &PrefixUnitCatalog,
    format_number: &dyn Fn(&Number) -> String,
) -> Option<String> {
    for (unit_name, display) in [("V", "V"), ("J", "J"), ("N", "N"), ("Pa", "Pa"), ("W", "W")] {
        if let Some(output) =
            format_quantity_as_unit(quantity, catalog, unit_name, display, format_number)
        {
            return Some(output);
        }
    }

    let unit = format_base_dimensions(&quantity.units.dimensions).ok()?;
    Some(format!("{} {unit}", format_number(&quantity.base_value())))
}

fn best_binary_byte_target(
    source: &Quantity,
    catalog: &PrefixUnitCatalog,
) -> Result<(UnitProduct, &'static str), String> {
    let candidates = [
        ("Ki", "KiB"),
        ("Mi", "MiB"),
        ("Gi", "GiB"),
        ("Ti", "TiB"),
        ("Pi", "PiB"),
        ("Ei", "EiB"),
    ];
    let mut selected = None;
    for (prefix, display) in candidates {
        let target = resolve_unit_with_prefix(catalog, "byte", Some(prefix), 1)?;
        if !source.units.same_dimensions(&target) {
            return Err("incompatible unit conversion to binary byte".to_string());
        }
        let value = source.base_value().div(&target.factor);
        if value.is_greater_than(&Number::one()) || value.is_one() || selected.is_none() {
            selected = Some((target, display));
        } else {
            break;
        }
    }
    selected.ok_or_else(|| "missing binary byte target".to_string())
}

fn format_quantity_as_unit(
    quantity: &Quantity,
    catalog: &PrefixUnitCatalog,
    unit_name: &str,
    display: &str,
    format_number: &dyn Fn(&Number) -> String,
) -> Option<String> {
    let target = resolve_unit_with_prefix(catalog, unit_name, None, 1).ok()?;
    if !quantity.units.same_dimensions(&target) {
        return None;
    }
    let value = quantity.base_value().div(&target.factor);
    Some(format!("{} {display}", format_number(&value)))
}

fn format_mixed_feet(
    source: &Quantity,
    target_feet: &UnitProduct,
    catalog: &PrefixUnitCatalog,
    format_number: &dyn Fn(&Number) -> String,
) -> Result<String, String> {
    let feet = source.base_value().div(&target_feet.factor);
    if feet.is_negative() {
        return Ok(format!("{} ft", format_number(&feet)));
    }

    let whole_feet = feet.floor();
    let whole_feet_i64 = whole_feet
        .to_i64()
        .ok_or_else(|| "mixed foot conversion exceeded integer range".to_string())?;
    if whole_feet_i64 == 0 {
        return Ok(format!("{} ft", format_number(&feet)));
    }

    let inches = resolve_unit_with_prefix(catalog, "in", None, 1)?;
    let remainder_base = source
        .base_value()
        .sub(&whole_feet.mul(&target_feet.factor));
    let remainder_inches = remainder_base.div(&inches.factor);
    Ok(format!(
        "{} ft + {} in",
        format_number(&whole_feet),
        format_number(&remainder_inches)
    ))
}

fn reduce_quantity(expr: &Expression, catalog: &PrefixUnitCatalog) -> Result<Quantity, String> {
    match expr {
        Expression::Number(number) => Ok(Quantity::number(number.clone())),
        Expression::Unit { unit, prefix, .. } => Ok(Quantity::unit(resolve_unit_with_prefix(
            catalog,
            unit.id(),
            prefix.as_ref().map(|prefix| prefix.id()),
            1,
        )?)),
        Expression::Symbolic(symbol) => {
            if catalog.unit_by_name(symbol.name()).is_some() {
                return Ok(Quantity::unit(resolve_unit_with_prefix(
                    catalog,
                    symbol.name(),
                    None,
                    1,
                )?));
            }
            Err(format!("unknown unit symbol '{}'", symbol.name()))
        }
        Expression::Multiplication(children) => {
            let mut out = Quantity::number(Number::one());
            for child in children.as_slice() {
                out = out.mul(&reduce_quantity(child, catalog)?);
            }
            Ok(out)
        }
        Expression::Division {
            numerator,
            denominator,
        } => Ok(reduce_quantity(numerator, catalog)?.div(&reduce_quantity(denominator, catalog)?)),
        Expression::Inverse(child) => {
            Ok(Quantity::number(Number::one()).div(&reduce_quantity(child, catalog)?))
        }
        Expression::Power { base, exponent } => {
            let exponent = reduce_quantity(exponent, catalog)?;
            if !exponent.units.is_unitless() {
                return Err("unit exponent must be dimensionless".to_string());
            }
            let exponent = exponent
                .value
                .to_i64()
                .and_then(|value| i32::try_from(value).ok())
                .ok_or_else(|| "unit exponent must be an integer".to_string())?;
            Ok(reduce_quantity(base, catalog)?.pow(exponent))
        }
        Expression::Negate(child) => Ok(reduce_quantity(child, catalog)?.negate()),
        _ => Err("unsupported unit expression".to_string()),
    }
}

fn resolve_unit_with_prefix(
    catalog: &PrefixUnitCatalog,
    name: &str,
    prefix_name: Option<&str>,
    exponent: i32,
) -> Result<UnitProduct, String> {
    let mut visiting = HashSet::new();
    let mut unit = resolve_unit_definition(catalog, name, &mut visiting)?;
    if let Some(prefix_name) = prefix_name {
        unit.factor = unit.factor.mul(&prefix_factor(catalog, prefix_name)?);
    }
    Ok(unit.pow(exponent))
}

fn resolve_unit_part(
    catalog: &PrefixUnitCatalog,
    name: &str,
    prefix_exponent: Option<i32>,
    exponent: i32,
    visiting: &mut HashSet<String>,
) -> Result<UnitProduct, String> {
    let mut unit = resolve_unit_definition(catalog, name, visiting)?;
    if let Some(prefix_exponent) = prefix_exponent {
        unit.factor = unit.factor.mul(&pow10(prefix_exponent));
    }
    Ok(unit.pow(exponent))
}

fn resolve_unit_definition(
    catalog: &PrefixUnitCatalog,
    name: &str,
    visiting: &mut HashSet<String>,
) -> Result<UnitProduct, String> {
    let definition = catalog
        .unit_by_name(name)
        .ok_or_else(|| format!("unknown unit '{name}'"))?;
    let key = definition
        .names()
        .first()
        .map(|name| name.name().to_string())
        .unwrap_or_else(|| name.to_string());

    if !visiting.insert(key.clone()) {
        return Err(format!("recursive unit definition '{key}'"));
    }

    let resolved = match definition.kind() {
        UnitKind::Builtin => Err(format!("builtin unit '{name}' is not a physical unit")),
        UnitKind::Base => Ok(base_unit_product(name)),
        UnitKind::Composite => {
            let mut out = UnitProduct::unitless();
            for part in definition.parts() {
                out = out.mul(&resolve_unit_part(
                    catalog,
                    part.unit(),
                    part.prefix_exponent(),
                    part.exponent(),
                    visiting,
                )?);
            }
            Ok(out)
        }
        UnitKind::Alias => {
            let mut out = UnitProduct::unitless();
            for base in definition.bases() {
                let relation = match base.relation() {
                    Some(relation) => parse_relation(relation)?,
                    None => Number::one(),
                };
                let base_product =
                    resolve_unit_part(catalog, base.unit(), None, base.exponent(), visiting)?;
                out = out.mul(&UnitProduct {
                    factor: base_product.factor.mul(&relation),
                    dimensions: base_product.dimensions,
                });
            }
            Ok(out)
        }
    };

    visiting.remove(&key);
    resolved
}

fn base_unit_product(name: &str) -> UnitProduct {
    if name == "g" || matches!(name, "gram" | "grams") {
        UnitProduct::base_unit("kg", Number::from_i32(1).div(&Number::from_i32(1000)))
    } else {
        UnitProduct::base_unit(name, Number::one())
    }
}

fn prefix_factor(catalog: &PrefixUnitCatalog, name: &str) -> Result<Number, String> {
    let definition = catalog
        .prefix_by_name(name)
        .ok_or_else(|| format!("unknown unit prefix '{name}'"))?;
    Ok(match definition.kind() {
        PrefixKind::Decimal => pow10(definition.exponent()),
        PrefixKind::Binary => pow2(definition.exponent()),
    })
}

fn parse_relation(relation: &str) -> Result<Number, String> {
    let relation = relation.trim();
    if let Some((left, right)) = relation.split_once('/') {
        let left = parse_relation(left)?;
        let right = parse_relation(right)?;
        return Ok(left.div(&right));
    }
    Number::from_str(relation)
        .map_err(|error| format!("unsupported unit relation {relation:?}: {error}"))
}

fn pow_number_i32(value: &Number, exponent: i32) -> Number {
    if exponent == 0 {
        return Number::one();
    }
    let mut result = Number::one();
    for _ in 0..exponent.unsigned_abs() {
        result = result.mul(value);
    }
    if exponent < 0 {
        Number::one().div(&result)
    } else {
        result
    }
}

fn pow10(exponent: i32) -> Number {
    pow_number_i32(&Number::from_i32(10), exponent)
}

fn pow2(exponent: i32) -> Number {
    pow_number_i32(&Number::from_i32(2), exponent)
}

fn format_number(value: &Number) -> String {
    let exact = value.to_qalc_string_with_precision(10);
    if significant_digit_count(&exact) <= 10 {
        return exact;
    }

    if let NumberValue::Rational(rational) = value.value() {
        let output = rug::Float::with_val(64, &rational.value).to_string_radix(10, Some(10));
        return fixed_decimal_from_scientific(&output).unwrap_or(output);
    }

    exact
}

fn significant_digit_count(value: &str) -> usize {
    let mut seen_non_zero = false;
    let mut count = 0usize;
    for ch in value.chars() {
        if !ch.is_ascii_digit() {
            continue;
        }
        if ch == '0' && !seen_non_zero {
            continue;
        }
        seen_non_zero = true;
        count += 1;
    }
    count
}

fn fixed_decimal_from_scientific(value: &str) -> Option<String> {
    let (mantissa, exponent) = value.split_once('e').or_else(|| value.split_once('E'))?;
    let exponent = exponent.parse::<isize>().ok()?;
    let negative = mantissa.starts_with('-');
    let mantissa = mantissa.trim_start_matches(['-', '+']);
    let mut parts = mantissa.split('.');
    let whole = parts.next().unwrap_or_default();
    let fraction = parts.next().unwrap_or_default();
    if parts.next().is_some() {
        return None;
    }

    let mut digits = String::new();
    digits.push_str(whole);
    digits.push_str(fraction);
    let decimal_pos = isize::try_from(whole.len()).ok()?.checked_add(exponent)?;

    let mut out = if decimal_pos <= 0 {
        format!("0.{}{}", "0".repeat(decimal_pos.unsigned_abs()), digits)
    } else if usize::try_from(decimal_pos).ok()? >= digits.len() {
        let zeros = usize::try_from(decimal_pos).ok()? - digits.len();
        format!("{digits}{}", "0".repeat(zeros))
    } else {
        let split = usize::try_from(decimal_pos).ok()?;
        format!("{}.{}", &digits[..split], &digits[split..])
    };

    if out.contains('.') {
        while out.ends_with('0') {
            out.pop();
        }
        if out.ends_with('.') {
            out.pop();
        }
    }
    if negative && out != "0" {
        out.insert(0, '-');
    }
    Some(out)
}

fn target_display(expr: &Expression, format_number: &dyn Fn(&Number) -> String) -> Option<String> {
    match expr {
        Expression::Unit { unit, prefix, .. } => {
            let mut out = String::new();
            if let Some(prefix) = prefix {
                out.push_str(prefix.id());
            }
            out.push_str(unit.id());
            Some(out)
        }
        Expression::Symbolic(symbol) => Some(symbol.name().to_string()),
        Expression::Multiplication(children) => children
            .as_slice()
            .iter()
            .map(|child| target_display(child, format_number))
            .collect::<Option<Vec<_>>>()
            .map(|parts| parts.join("*")),
        Expression::Division {
            numerator,
            denominator,
        } => {
            let numerator = target_display(numerator, format_number)?;
            let denominator = target_display(denominator, format_number)?;
            Some(format!("{numerator}/{denominator}"))
        }
        Expression::Power { base, exponent } => {
            let base = target_display(base, format_number)?;
            let exponent = match exponent.as_ref() {
                Expression::Number(number) => format_number(number),
                _ => target_display(exponent, format_number)?,
            };
            Some(format!("{base}^{exponent}"))
        }
        Expression::Inverse(child) => Some(format!("1/{}", target_display(child, format_number)?)),
        Expression::Negate(child) => target_display(child, format_number),
        _ => None,
    }
}

fn format_base_dimensions(dimensions: &BTreeMap<String, i32>) -> Result<String, String> {
    if dimensions.is_empty() {
        return Err("unit expression is dimensionless".to_string());
    }

    let mut numerator = Vec::new();
    let mut denominator = Vec::new();
    for key in ordered_dimension_keys(dimensions) {
        let exponent = dimensions
            .get(key)
            .copied()
            .expect("ordered keys are drawn from dimensions");
        if exponent > 0 {
            numerator.push(format_dimension_factor(key, exponent));
        } else {
            denominator.push(format_dimension_factor(key, -exponent));
        }
    }

    let numerator = if numerator.is_empty() {
        "1".to_string()
    } else {
        numerator.join("*")
    };
    if denominator.is_empty() {
        Ok(numerator)
    } else {
        let denominator = denominator.join("*");
        if denominator.contains('*') {
            Ok(format!("{numerator}/({denominator})"))
        } else {
            Ok(format!("{numerator}/{denominator}"))
        }
    }
}

fn ordered_dimension_keys(dimensions: &BTreeMap<String, i32>) -> Vec<&String> {
    let order = ["kg", "m", "A", "s", "K", "mol", "cd", "bit"];
    let mut keys = dimensions.keys().collect::<Vec<_>>();
    keys.sort_by_key(|key| {
        order
            .iter()
            .position(|item| item == &key.as_str())
            .unwrap_or(order.len())
    });
    keys
}

fn format_dimension_factor(name: &str, exponent: i32) -> String {
    if exponent == 1 {
        name.to_string()
    } else {
        format!("{name}^{exponent}")
    }
}
