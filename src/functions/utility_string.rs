//! Utility and string built-in function family.
//!
//! # Upstream oracle
//! - `../libqalculate/libqalculate/BuiltinFunctions-util.cc`
//! - `../libqalculate/libqalculate/BuiltinFunctions-number.cc` (`dec`)
//! - `../libqalculate/data/functions.xml.in`
//! - `../libqalculate/tests/strings.batch`

use crate::ast::Expression;
use crate::context::CalculatorContext;
use crate::functions::{
    make_unevaluated, validate_arity, BuiltinFunction, BuiltinFunctionInfo, FunctionError,
    FunctionResult,
};
use crate::messages::{CalculatorMessage, MessageCategory, MessageStage, MessageType};
use crate::number::{Number, Rational};
use crate::parser::names::NameRegistry;

static CODE_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "code",
    aliases: &[],
    min_args: 1,
    max_args: Some(3),
    description: "Encodes a Unicode character or text string.",
};

static CHAR_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "char",
    aliases: &[],
    min_args: 1,
    max_args: Some(1),
    description: "Returns a Unicode character for a code point.",
};

static LEN_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "len",
    aliases: &[],
    min_args: 1,
    max_args: Some(1),
    description: "Length of string.",
};

static CONCATENATE_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "concatenate",
    aliases: &["strcat"],
    min_args: 1,
    max_args: None,
    description: "Concatenate strings.",
};

static STRING_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "string",
    aliases: &[],
    min_args: 1,
    max_args: None,
    description: "Convert values to strings.",
};

static CHARACTERS_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "characters",
    aliases: &[],
    min_args: 1,
    max_args: Some(1),
    description: "Convert text to a vector of characters.",
};

static DEC_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "dec",
    aliases: &[],
    min_args: 1,
    max_args: Some(2),
    description: "Returns a value from a decimal expression.",
};

static ERROR_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "error",
    aliases: &[],
    min_args: 1,
    max_args: Some(1),
    description: "Display an error message.",
};

static WARNING_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "warning",
    aliases: &[],
    min_args: 1,
    max_args: Some(1),
    description: "Display a warning message.",
};

static MESSAGE_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "message",
    aliases: &[],
    min_args: 1,
    max_args: Some(1),
    description: "Display an informational message.",
};

static REPLACE_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "replace",
    aliases: &[],
    min_args: 3,
    max_args: Some(4),
    description: "Replaces substrings or subexpressions.",
};

static NOUNIT_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "nounit",
    aliases: &[],
    min_args: 1,
    max_args: Some(1),
    description: "Strip units.",
};

static TITLE_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "title",
    aliases: &[],
    min_args: 1,
    max_args: Some(1),
    description: "Returns the title of an object.",
};

static REPRESENTS_INTEGER_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "representsInteger",
    aliases: &[],
    min_args: 1,
    max_args: Some(1),
    description: "Returns true if the expression represents an integer.",
};

static REPRESENTS_REAL_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "representsReal",
    aliases: &[],
    min_args: 1,
    max_args: Some(1),
    description: "Returns true if the expression represents a real number.",
};

static REPRESENTS_RATIONAL_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "representsRational",
    aliases: &[],
    min_args: 1,
    max_args: Some(1),
    description: "Returns true if the expression represents a rational number.",
};

static REPRESENTS_NUMBER_INFO: BuiltinFunctionInfo = BuiltinFunctionInfo {
    name: "representsNumber",
    aliases: &[],
    min_args: 1,
    max_args: Some(1),
    description: "Returns true if the expression represents a number.",
};

struct UtilityStringFunction {
    info: &'static BuiltinFunctionInfo,
}

impl BuiltinFunction for UtilityStringFunction {
    fn info(&self) -> &BuiltinFunctionInfo {
        self.info
    }

    fn evaluate(&self, args: &[Expression], context: &mut CalculatorContext) -> FunctionResult {
        evaluate_raw(self.info.name, args, context)
            .unwrap_or_else(|| Ok(make_unevaluated(self.info.name, args)))
    }
}

static CODE_FUNCTION: UtilityStringFunction = UtilityStringFunction { info: &CODE_INFO };
static CHAR_FUNCTION: UtilityStringFunction = UtilityStringFunction { info: &CHAR_INFO };
static LEN_FUNCTION: UtilityStringFunction = UtilityStringFunction { info: &LEN_INFO };
static CONCATENATE_FUNCTION: UtilityStringFunction = UtilityStringFunction {
    info: &CONCATENATE_INFO,
};
static STRING_FUNCTION: UtilityStringFunction = UtilityStringFunction { info: &STRING_INFO };
static CHARACTERS_FUNCTION: UtilityStringFunction = UtilityStringFunction {
    info: &CHARACTERS_INFO,
};
static DEC_FUNCTION: UtilityStringFunction = UtilityStringFunction { info: &DEC_INFO };
static ERROR_FUNCTION: UtilityStringFunction = UtilityStringFunction { info: &ERROR_INFO };
static WARNING_FUNCTION: UtilityStringFunction = UtilityStringFunction {
    info: &WARNING_INFO,
};
static MESSAGE_FUNCTION: UtilityStringFunction = UtilityStringFunction {
    info: &MESSAGE_INFO,
};
static REPLACE_FUNCTION: UtilityStringFunction = UtilityStringFunction {
    info: &REPLACE_INFO,
};
static NOUNIT_FUNCTION: UtilityStringFunction = UtilityStringFunction { info: &NOUNIT_INFO };
static TITLE_FUNCTION: UtilityStringFunction = UtilityStringFunction { info: &TITLE_INFO };
static REPRESENTS_INTEGER_FUNCTION: UtilityStringFunction = UtilityStringFunction {
    info: &REPRESENTS_INTEGER_INFO,
};
static REPRESENTS_REAL_FUNCTION: UtilityStringFunction = UtilityStringFunction {
    info: &REPRESENTS_REAL_INFO,
};
static REPRESENTS_RATIONAL_FUNCTION: UtilityStringFunction = UtilityStringFunction {
    info: &REPRESENTS_RATIONAL_INFO,
};
static REPRESENTS_NUMBER_FUNCTION: UtilityStringFunction = UtilityStringFunction {
    info: &REPRESENTS_NUMBER_INFO,
};

static CATALOG: [&BuiltinFunctionInfo; 17] = [
    &CODE_INFO,
    &CHAR_INFO,
    &LEN_INFO,
    &CONCATENATE_INFO,
    &STRING_INFO,
    &CHARACTERS_INFO,
    &DEC_INFO,
    &ERROR_INFO,
    &WARNING_INFO,
    &MESSAGE_INFO,
    &REPLACE_INFO,
    &NOUNIT_INFO,
    &TITLE_INFO,
    &REPRESENTS_INTEGER_INFO,
    &REPRESENTS_REAL_INFO,
    &REPRESENTS_RATIONAL_INFO,
    &REPRESENTS_NUMBER_INFO,
];

/// Looks up a utility/string built-in function by name or alias.
pub fn lookup(name: &str) -> Option<&'static dyn BuiltinFunction> {
    match canonical_name(name) {
        Some("code") => Some(&CODE_FUNCTION),
        Some("char") => Some(&CHAR_FUNCTION),
        Some("len") => Some(&LEN_FUNCTION),
        Some("concatenate") => Some(&CONCATENATE_FUNCTION),
        Some("string") => Some(&STRING_FUNCTION),
        Some("characters") => Some(&CHARACTERS_FUNCTION),
        Some("dec") => Some(&DEC_FUNCTION),
        Some("error") => Some(&ERROR_FUNCTION),
        Some("warning") => Some(&WARNING_FUNCTION),
        Some("message") => Some(&MESSAGE_FUNCTION),
        Some("replace") => Some(&REPLACE_FUNCTION),
        Some("nounit") => Some(&NOUNIT_FUNCTION),
        Some("title") => Some(&TITLE_FUNCTION),
        Some("representsInteger") => Some(&REPRESENTS_INTEGER_FUNCTION),
        Some("representsReal") => Some(&REPRESENTS_REAL_FUNCTION),
        Some("representsRational") => Some(&REPRESENTS_RATIONAL_FUNCTION),
        Some("representsNumber") => Some(&REPRESENTS_NUMBER_FUNCTION),
        _ => None,
    }
}

/// Returns metadata for all utility/string built-in functions.
pub fn catalog() -> Vec<&'static BuiltinFunctionInfo> {
    CATALOG.to_vec()
}

pub(crate) fn evaluate_raw(
    name: &str,
    args: &[Expression],
    context: &mut CalculatorContext,
) -> Option<FunctionResult> {
    let name = canonical_name(name)?;
    Some(match name {
        "code" => evaluate_code(args, context),
        "char" => evaluate_char(args, context),
        "len" => evaluate_len(args, context),
        "concatenate" => evaluate_concatenate(args, context),
        "string" => evaluate_string(args, context),
        "characters" => evaluate_characters(args, context),
        "dec" => evaluate_dec(args, context),
        "error" => evaluate_message_function(args, context, MessageType::Error),
        "warning" => evaluate_message_function(args, context, MessageType::Warning),
        "message" => evaluate_message_function(args, context, MessageType::Information),
        "replace" => evaluate_replace(args, context),
        "nounit" => evaluate_nounit(args, context),
        "title" => evaluate_title(args, context),
        "representsInteger" => evaluate_represents("representsInteger", args, context, |e| {
            e.represents_integer()
        }),
        "representsReal" => {
            evaluate_represents("representsReal", args, context, |e| e.represents_real())
        }
        "representsRational" => evaluate_represents("representsRational", args, context, |e| {
            e.represents_rational()
        }),
        "representsNumber" => {
            evaluate_represents("representsNumber", args, context, |e| e.represents_number())
        }
        _ => return None,
    })
}

pub(crate) fn is_raw_utility_string(name: &str) -> bool {
    canonical_name(name).is_some()
}

fn canonical_name(name: &str) -> Option<&'static str> {
    if name.eq_ignore_ascii_case("strcat") {
        return Some("concatenate");
    }
    [
        "code",
        "char",
        "len",
        "concatenate",
        "string",
        "characters",
        "dec",
        "error",
        "warning",
        "message",
        "replace",
        "nounit",
        "title",
        "representsInteger",
        "representsReal",
        "representsRational",
        "representsNumber",
    ]
    .into_iter()
    .find(|known| name.eq_ignore_ascii_case(known))
}

fn evaluate_code(args: &[Expression], context: &mut CalculatorContext) -> FunctionResult {
    validate_arity("code", args, 1, Some(3))?;
    let text = text_argument_to_string(&args[0], context)?;
    if text.is_empty() {
        return Err(function_error("code", "Expected non-empty text"));
    }
    let encoding = if let Some(arg) = args.get(1) {
        text_argument_to_string(arg, context)?
    } else {
        "UTF-32".to_string()
    };
    let use_vector = bool_argument(args.get(2), true, context)?;
    let encoding = match encoding_name(&encoding) {
        Some(encoding) => encoding,
        None => return Err(function_error("code", "Unknown text encoding")),
    };
    let units = match encoding {
        Encoding::Utf8 => text.bytes().map(u128::from).collect::<Vec<_>>(),
        Encoding::Utf16 => text.encode_utf16().map(u128::from).collect::<Vec<_>>(),
        Encoding::Utf32 => text.chars().map(|ch| u128::from(ch as u32)).collect(),
    };
    code_units_to_expression("code", &units, use_vector, encoding.unit_base())
}

fn evaluate_char(args: &[Expression], context: &mut CalculatorContext) -> FunctionResult {
    validate_arity("char", args, 1, Some(1))?;
    let evaluated =
        crate::eval::evaluate_ast(&args[0], context).map_err(|message| FunctionError {
            function_name: "char".to_string(),
            message,
        })?;
    match evaluated {
        Expression::Number(number) => codepoint_text(number_to_u128(&number)?, "char"),
        Expression::Vector(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                let Expression::Number(number) = item else {
                    return Err(function_error("char", "Expected integer code point"));
                };
                out.push(codepoint_text(number_to_u128(&number)?, "char")?);
            }
            Ok(Expression::Vector(out))
        }
        other => Ok(make_unevaluated("char", &[other])),
    }
}

fn evaluate_len(args: &[Expression], context: &mut CalculatorContext) -> FunctionResult {
    validate_arity("len", args, 1, Some(1))?;
    let text = text_argument_to_string(&args[0], context)?;
    integer_expression(crate::text::unicode_len(&text) as u128, "len")
}

fn evaluate_concatenate(args: &[Expression], context: &mut CalculatorContext) -> FunctionResult {
    validate_arity("concatenate", args, 1, None)?;
    let mut out = String::new();
    for arg in args {
        out.push_str(&text_argument_to_string(arg, context)?);
    }
    Ok(Expression::Text(out))
}

fn evaluate_string(args: &[Expression], context: &mut CalculatorContext) -> FunctionResult {
    validate_arity("string", args, 1, None)?;
    if args.len() == 1 {
        return evaluated_string_expression(&args[0], context);
    }
    let mut out = Vec::with_capacity(args.len());
    for arg in args {
        out.push(evaluated_string_expression(arg, context)?);
    }
    Ok(Expression::Vector(out))
}

fn evaluate_characters(args: &[Expression], context: &mut CalculatorContext) -> FunctionResult {
    validate_arity("characters", args, 1, Some(1))?;
    let text = text_argument_to_string(&args[0], context)?;
    Ok(Expression::Vector(
        text.chars()
            .map(|ch| Expression::Text(ch.to_string()))
            .collect(),
    ))
}

fn evaluate_dec(args: &[Expression], context: &mut CalculatorContext) -> FunctionResult {
    validate_arity("dec", args, 1, Some(2))?;
    let source = text_argument_to_string(&args[0], context)?;
    let reverse = bool_argument(args.get(1), false, context)?;
    let parsed = crate::parser::operators::parse_expression(&source)
        .map_err(|error| function_error("dec", &error.to_string()))?;
    let evaluated =
        crate::eval::evaluate_ast(&parsed, context).map_err(|message| FunctionError {
            function_name: "dec".to_string(),
            message,
        })?;
    if reverse {
        let text = result_to_unquoted_text(&evaluated, context)?;
        Ok(Expression::Text(text))
    } else {
        Ok(evaluated)
    }
}

fn evaluate_message_function(
    args: &[Expression],
    context: &mut CalculatorContext,
    message_type: MessageType,
) -> FunctionResult {
    let name = match message_type {
        MessageType::Error => "error",
        MessageType::Warning => "warning",
        MessageType::Information => "message",
    };
    validate_arity(name, args, 1, Some(1))?;
    let message = text_argument_to_string(&args[0], context)?;
    context.messages.push(CalculatorMessage::new(
        message.clone(),
        message_type,
        MessageCategory::None,
        MessageStage::Calculation,
    ));
    Ok(Expression::Text(message))
}

fn evaluated_string_expression(
    arg: &Expression,
    context: &mut CalculatorContext,
) -> FunctionResult {
    let evaluated = crate::eval::evaluate_ast(arg, context).map_err(|message| FunctionError {
        function_name: "string".to_string(),
        message,
    })?;
    let text = result_to_unquoted_text(&evaluated, context)?;
    Ok(Expression::Text(text))
}
fn text_argument_to_string(
    arg: &Expression,
    context: &mut CalculatorContext,
) -> Result<String, FunctionError> {
    match arg {
        Expression::Text(text) => Ok(text.clone()),
        Expression::Symbolic(symbol) => {
            if let Some(Expression::Text(text)) = context.variables.get(symbol.name()) {
                Ok(text.clone())
            } else {
                Ok(symbol.name().to_string())
            }
        }
        Expression::Variable(variable) => {
            if let Some(Expression::Text(text)) = context.variables.get(variable.id()) {
                Ok(text.clone())
            } else {
                Ok(variable.id().to_string())
            }
        }
        Expression::FunctionCall { function, args } if is_raw_utility_string(function.id()) => {
            let evaluated = evaluate_raw(function.id(), args, context)
                .ok_or_else(|| function_error(function.id(), "Unknown text function"))??;
            result_to_unquoted_text(&evaluated, context)
        }
        other => Ok(crate::text::format_raw_expression(other)),
    }
}

fn result_to_unquoted_text(
    expr: &Expression,
    context: &mut CalculatorContext,
) -> Result<String, FunctionError> {
    match expr {
        Expression::Text(text) => Ok(text.clone()),
        Expression::Symbolic(symbol) => Ok(symbol.name().to_string()),
        Expression::Number(number) => Ok(number.to_string()),
        Expression::Variable(variable) => {
            if let Some(Expression::Text(text)) = context.variables.get(variable.id()) {
                Ok(text.clone())
            } else {
                Ok(variable.id().to_string())
            }
        }
        Expression::Vector(_) => {
            crate::text::format_result_with_numbers(expr, &|num| num.to_string())
                .ok_or_else(|| function_error("string", "Cannot convert vector to text"))
        }
        other => Ok(crate::text::format_raw_expression(other)),
    }
}

fn bool_argument(
    arg: Option<&Expression>,
    default: bool,
    context: &mut CalculatorContext,
) -> Result<bool, FunctionError> {
    let Some(arg) = arg else {
        return Ok(default);
    };
    let evaluated = crate::eval::evaluate_ast(arg, context).map_err(|message| FunctionError {
        function_name: "boolean".to_string(),
        message,
    })?;
    match evaluated {
        Expression::Number(number) => match number.get_boolean() {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(function_error("boolean", "Expected boolean value")),
        },
        Expression::Text(_) | Expression::Symbolic(_) if text_or_symbol_is_false(&evaluated) => {
            Ok(false)
        }
        Expression::Text(_) | Expression::Symbolic(_) if text_or_symbol_is_true(&evaluated) => {
            Ok(true)
        }
        _ => Err(function_error("boolean", "Expected boolean value")),
    }
}

fn text_or_symbol_is_false(expr: &Expression) -> bool {
    let value = match expr {
        Expression::Text(text) => text.as_str(),
        Expression::Symbolic(symbol) => symbol.name(),
        _ => return false,
    };
    value == "0" || value.eq_ignore_ascii_case("false")
}

fn text_or_symbol_is_true(expr: &Expression) -> bool {
    let value = match expr {
        Expression::Text(text) => text.as_str(),
        Expression::Symbolic(symbol) => symbol.name(),
        _ => return false,
    };
    value == "1" || value.eq_ignore_ascii_case("true")
}

#[derive(Debug, Clone, Copy)]
enum Encoding {
    Utf8,
    Utf16,
    Utf32,
}

impl Encoding {
    fn unit_base(self) -> u128 {
        match self {
            Self::Utf8 => 0x100,
            Self::Utf16 => 0x10000,
            Self::Utf32 => 0x1_0000_0000,
        }
    }
}

fn encoding_name(name: &str) -> Option<Encoding> {
    let normalized = name
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .map(|ch| if ch == '\u{2212}' { '-' } else { ch })
        .collect::<String>()
        .to_ascii_lowercase();
    match normalized.as_str() {
        "utf-8" | "utf8" | "ascii" | "0" => Some(Encoding::Utf8),
        "utf-16" | "utf16" | "1" => Some(Encoding::Utf16),
        "utf-32" | "utf32" | "2" => Some(Encoding::Utf32),
        _ => None,
    }
}

fn code_units_to_expression(
    name: &str,
    units: &[u128],
    use_vector: bool,
    unit_base: u128,
) -> FunctionResult {
    if use_vector && units.len() > 1 {
        return units
            .iter()
            .map(|unit| integer_expression(*unit, name))
            .collect::<Result<Vec<_>, _>>()
            .map(Expression::Vector);
    }

    let mut value = 0_u128;
    for unit in units {
        value = value
            .checked_mul(unit_base)
            .and_then(|acc| acc.checked_add(*unit))
            .ok_or_else(|| function_error(name, "Encoded value is too large"))?;
    }
    integer_expression(value, name)
}

fn codepoint_text(value: u128, name: &str) -> FunctionResult {
    crate::text::codepoint_to_string(value)
        .map(Expression::Text)
        .ok_or_else(|| function_error(name, "Invalid Unicode code point"))
}

fn integer_expression(value: u128, name: &str) -> FunctionResult {
    let value =
        i128::try_from(value).map_err(|_| function_error(name, "Integer value is too large"))?;
    Ok(Expression::Number(Number::from_rational(Rational::new(
        value, 1,
    ))))
}

fn number_to_u128(number: &Number) -> Result<u128, FunctionError> {
    crate::numberbase::number_to_u128(number)
        .ok_or_else(|| function_error("number", "Expected non-negative integer"))
}

fn evaluate_replace(args: &[Expression], context: &mut CalculatorContext) -> FunctionResult {
    validate_arity("replace", args, 3, Some(4))?;
    let old_eval = crate::eval::evaluate_ast(&args[1], context)
        .map_err(|message| function_error("replace", &message))?;
    let new_eval = crate::eval::evaluate_ast(&args[2], context)
        .map_err(|message| function_error("replace", &message))?;
    let precalc = bool_argument(args.get(3), false, context)?;

    let mut expr = if precalc {
        crate::eval::evaluate_ast(&args[0], context)
            .map_err(|message| function_error("replace", &message))?
    } else {
        args[0].clone()
    };

    let mut found = false;
    match (&old_eval, &new_eval) {
        (Expression::Vector(from_items), Expression::Vector(to_items))
            if from_items.len() == to_items.len() =>
        {
            for (from_item, to_item) in from_items.iter().zip(to_items.iter()) {
                let (new_expr, ok) = replace_expression(&expr, from_item, to_item);
                if ok {
                    found = true;
                }
                expr = new_expr;
            }
        }
        (Expression::Vector(from_items), non_vector) => {
            for from_item in from_items {
                let (new_expr, ok) = replace_expression(&expr, from_item, non_vector);
                if ok {
                    found = true;
                }
                expr = new_expr;
            }
        }
        (from, to) => {
            let (new_expr, ok) = replace_expression(&expr, from, to);
            found = ok;
            expr = new_expr;
        }
    }

    if !found {
        context.messages.push(CalculatorMessage::new(
            format!(
                "Original value ({}) was not found.",
                crate::text::format_raw_expression(&old_eval)
            ),
            MessageType::Warning,
            MessageCategory::None,
            MessageStage::Calculation,
        ));
    }

    Ok(expr)
}

fn nary_children(children: Vec<Expression>) -> crate::ast::NaryChildren {
    crate::ast::NaryChildren::new(children).unwrap()
}

fn replace_expression(expr: &Expression, from: &Expression, to: &Expression) -> (Expression, bool) {
    if expr == from {
        return (to.clone(), true);
    }

    match expr {
        Expression::Addition(children) => {
            if let Expression::Addition(from_children) = from {
                let new_children = children.clone();
                let mut matched_all = true;
                let mut indices_to_remove = Vec::new();
                for f_child in from_children.as_slice() {
                    if let Some(idx) = new_children.as_slice().iter().position(|c| c == f_child) {
                        if !indices_to_remove.contains(&idx) {
                            indices_to_remove.push(idx);
                        } else {
                            matched_all = false;
                            break;
                        }
                    } else {
                        matched_all = false;
                        break;
                    }
                }
                if matched_all && !indices_to_remove.is_empty() {
                    indices_to_remove.sort_by(|a, b| b.cmp(a));
                    let mut vec = new_children.into_vec();
                    for idx in indices_to_remove {
                        vec.remove(idx);
                    }
                    vec.push(to.clone());
                    let result = if vec.len() == 1 {
                        vec.remove(0)
                    } else {
                        Expression::Addition(nary_children(vec))
                    };
                    return (result, true);
                }
            }

            let mut changed = false;
            let mut new_children = Vec::new();
            for child in children.as_slice() {
                let (new_child, replaced) = replace_expression(child, from, to);
                if replaced {
                    changed = true;
                }
                new_children.push(new_child);
            }
            if changed {
                (Expression::Addition(nary_children(new_children)), true)
            } else {
                (expr.clone(), false)
            }
        }
        Expression::Multiplication(children) => {
            if let Expression::Multiplication(from_children) = from {
                let new_children = children.clone();
                let mut matched_all = true;
                let mut indices_to_remove = Vec::new();
                for f_child in from_children.as_slice() {
                    if let Some(idx) = new_children.as_slice().iter().position(|c| c == f_child) {
                        if !indices_to_remove.contains(&idx) {
                            indices_to_remove.push(idx);
                        } else {
                            matched_all = false;
                            break;
                        }
                    } else {
                        matched_all = false;
                        break;
                    }
                }
                if matched_all && !indices_to_remove.is_empty() {
                    indices_to_remove.sort_by(|a, b| b.cmp(a));
                    let mut vec = new_children.into_vec();
                    for idx in indices_to_remove {
                        vec.remove(idx);
                    }
                    vec.push(to.clone());
                    let result = if vec.len() == 1 {
                        vec.remove(0)
                    } else {
                        Expression::Multiplication(nary_children(vec))
                    };
                    return (result, true);
                }
            }

            let mut changed = false;
            let mut new_children = Vec::new();
            for child in children.as_slice() {
                let (new_child, replaced) = replace_expression(child, from, to);
                if replaced {
                    changed = true;
                }
                new_children.push(new_child);
            }
            if changed {
                (
                    Expression::Multiplication(nary_children(new_children)),
                    true,
                )
            } else {
                (expr.clone(), false)
            }
        }
        Expression::Division {
            numerator,
            denominator,
        } => {
            let (new_num, r1) = replace_expression(numerator, from, to);
            let (new_den, r2) = replace_expression(denominator, from, to);
            if r1 || r2 {
                (
                    Expression::Division {
                        numerator: Box::new(new_num),
                        denominator: Box::new(new_den),
                    },
                    true,
                )
            } else {
                (expr.clone(), false)
            }
        }
        Expression::Power { base, exponent } => {
            let (new_base, r1) = replace_expression(base, from, to);
            let (new_exp, r2) = replace_expression(exponent, from, to);
            if r1 || r2 {
                (
                    Expression::Power {
                        base: Box::new(new_base),
                        exponent: Box::new(new_exp),
                    },
                    true,
                )
            } else {
                (expr.clone(), false)
            }
        }
        Expression::Negate(child) => {
            let (new_child, r) = replace_expression(child, from, to);
            if r {
                (Expression::Negate(Box::new(new_child)), true)
            } else {
                (expr.clone(), false)
            }
        }
        Expression::Inverse(child) => {
            let (new_child, r) = replace_expression(child, from, to);
            if r {
                (Expression::Inverse(Box::new(new_child)), true)
            } else {
                (expr.clone(), false)
            }
        }
        Expression::FunctionCall { function, args } => {
            let mut changed = false;
            let mut new_args = Vec::new();
            for arg in args {
                let (new_arg, r) = replace_expression(arg, from, to);
                if r {
                    changed = true;
                }
                new_args.push(new_arg);
            }
            if changed {
                (
                    Expression::FunctionCall {
                        function: function.clone(),
                        args: new_args,
                    },
                    true,
                )
            } else {
                (expr.clone(), false)
            }
        }
        Expression::Vector(elems) => {
            let mut changed = false;
            let mut new_elems = Vec::new();
            for elem in elems {
                let (new_elem, r) = replace_expression(elem, from, to);
                if r {
                    changed = true;
                }
                new_elems.push(new_elem);
            }
            if changed {
                (Expression::Vector(new_elems), true)
            } else {
                (expr.clone(), false)
            }
        }
        Expression::LogicalAnd(children) => {
            let mut changed = false;
            let mut new_children = Vec::new();
            for child in children.as_slice() {
                let (new_child, r) = replace_expression(child, from, to);
                if r {
                    changed = true;
                }
                new_children.push(new_child);
            }
            if changed {
                (Expression::LogicalAnd(nary_children(new_children)), true)
            } else {
                (expr.clone(), false)
            }
        }
        Expression::LogicalOr(children) => {
            let mut changed = false;
            let mut new_children = Vec::new();
            for child in children.as_slice() {
                let (new_child, r) = replace_expression(child, from, to);
                if r {
                    changed = true;
                }
                new_children.push(new_child);
            }
            if changed {
                (Expression::LogicalOr(nary_children(new_children)), true)
            } else {
                (expr.clone(), false)
            }
        }
        Expression::LogicalNot(child) => {
            let (new_child, r) = replace_expression(child, from, to);
            if r {
                (Expression::LogicalNot(Box::new(new_child)), true)
            } else {
                (expr.clone(), false)
            }
        }
        Expression::LogicalXor { lhs, rhs } => {
            let (new_lhs, r1) = replace_expression(lhs, from, to);
            let (new_rhs, r2) = replace_expression(rhs, from, to);
            if r1 || r2 {
                (
                    Expression::LogicalXor {
                        lhs: Box::new(new_lhs),
                        rhs: Box::new(new_rhs),
                    },
                    true,
                )
            } else {
                (expr.clone(), false)
            }
        }
        Expression::BitwiseAnd(children) => {
            let mut changed = false;
            let mut new_children = Vec::new();
            for child in children.as_slice() {
                let (new_child, r) = replace_expression(child, from, to);
                if r {
                    changed = true;
                }
                new_children.push(new_child);
            }
            if changed {
                (Expression::BitwiseAnd(nary_children(new_children)), true)
            } else {
                (expr.clone(), false)
            }
        }
        Expression::BitwiseOr(children) => {
            let mut changed = false;
            let mut new_children = Vec::new();
            for child in children.as_slice() {
                let (new_child, r) = replace_expression(child, from, to);
                if r {
                    changed = true;
                }
                new_children.push(new_child);
            }
            if changed {
                (Expression::BitwiseOr(nary_children(new_children)), true)
            } else {
                (expr.clone(), false)
            }
        }
        Expression::BitwiseXor(children) => {
            let mut changed = false;
            let mut new_children = Vec::new();
            for child in children.as_slice() {
                let (new_child, r) = replace_expression(child, from, to);
                if r {
                    changed = true;
                }
                new_children.push(new_child);
            }
            if changed {
                (Expression::BitwiseXor(nary_children(new_children)), true)
            } else {
                (expr.clone(), false)
            }
        }
        Expression::BitwiseNot(child) => {
            let (new_child, r) = replace_expression(child, from, to);
            if r {
                (Expression::BitwiseNot(Box::new(new_child)), true)
            } else {
                (expr.clone(), false)
            }
        }
        _ => (expr.clone(), false),
    }
}

fn evaluate_nounit(args: &[Expression], context: &mut CalculatorContext) -> FunctionResult {
    validate_arity("nounit", args, 1, Some(1))?;
    let evaluated = crate::eval::evaluate_ast(&args[0], context)
        .map_err(|message| function_error("nounit", &message))?;
    let stripped = strip_units_expr(&evaluated);
    crate::eval::evaluate_ast(&stripped, context)
        .map_err(|message| function_error("nounit", &message))
}

fn strip_units_expr(expr: &Expression) -> Expression {
    match expr {
        Expression::Unit { .. } => Expression::Number(Number::from_rational(Rational::from_i32(1))),
        Expression::Multiplication(children) => {
            let mut new_children = Vec::new();
            for child in children.as_slice() {
                let stripped = strip_units_expr(child);
                if let Expression::Number(ref num) = stripped {
                    if num.is_one() {
                        continue;
                    }
                }
                new_children.push(stripped);
            }
            if new_children.is_empty() {
                Expression::Number(Number::from_rational(Rational::from_i32(1)))
            } else if new_children.len() == 1 {
                new_children.remove(0)
            } else {
                Expression::Multiplication(nary_children(new_children))
            }
        }
        Expression::Addition(children) => {
            let mut new_children = Vec::new();
            for child in children.as_slice() {
                let stripped = strip_units_expr(child);
                new_children.push(stripped);
            }
            if new_children.is_empty() {
                Expression::Number(Number::from_rational(Rational::from_i32(0)))
            } else if new_children.len() == 1 {
                new_children.remove(0)
            } else {
                Expression::Addition(nary_children(new_children))
            }
        }
        Expression::Division {
            numerator,
            denominator,
        } => {
            let num = strip_units_expr(numerator);
            let den = strip_units_expr(denominator);
            if let Expression::Number(ref n) = den {
                if n.is_one() {
                    return num;
                }
            }
            Expression::Division {
                numerator: Box::new(num),
                denominator: Box::new(den),
            }
        }
        Expression::Power { base, exponent } => Expression::Power {
            base: Box::new(strip_units_expr(base)),
            exponent: Box::new(strip_units_expr(exponent)),
        },
        Expression::Negate(child) => Expression::Negate(Box::new(strip_units_expr(child))),
        Expression::Inverse(child) => Expression::Inverse(Box::new(strip_units_expr(child))),
        Expression::FunctionCall { function, args } => {
            let new_args: Vec<_> = args.iter().map(strip_units_expr).collect();
            Expression::FunctionCall {
                function: function.clone(),
                args: new_args,
            }
        }
        Expression::Vector(elems) => {
            let new_elems: Vec<_> = elems.iter().map(strip_units_expr).collect();
            Expression::Vector(new_elems)
        }
        other => other.clone(),
    }
}

fn evaluate_title(args: &[Expression], context: &mut CalculatorContext) -> FunctionResult {
    validate_arity("title", args, 1, Some(1))?;
    let name = text_argument_to_string(&args[0], context)?;
    if let Some(title) = get_item_title(&name, context) {
        Ok(Expression::Text(title))
    } else {
        Err(function_error(
            "title",
            &format!("Object {} does not exist.", name),
        ))
    }
}

fn get_item_title(name: &str, context: &CalculatorContext) -> Option<String> {
    let title = match name {
        "abs" => "Absolute Value",
        "sgn" => "Signum",
        "sqrt" => "Square Root",
        "cbrt" => "Cube Root",
        "ln" => "Natural Logarithm",
        "log" => "Logarithm",
        "exp" => "Exponential",
        "sin" => "Sine",
        "cos" => "Cosine",
        "tan" => "Tangent",
        "asin" => "Arc Sine",
        "acos" => "Arc Cosine",
        "atan" => "Arc Tangent",
        "sinh" => "Hyperbolic Sine",
        "cosh" => "Hyperbolic Cosine",
        "tanh" => "Hyperbolic Tangent",
        "asinh" => "Area Hyperbolic Sine",
        "acosh" => "Area Hyperbolic Cosine",
        "atanh" => "Area Hyperbolic Tangent",
        "pi" => "pi",
        "e" => "e",
        "m" | "meter" | "meters" => "meter",
        "g" | "gram" | "grams" => "gram",
        "s" | "second" | "seconds" => "second",
        _ => "",
    };
    if !title.is_empty() {
        return Some(title.to_string());
    }

    if let Some(match_result) = context.definitions.lookup(name, false) {
        match match_result {
            crate::parser::names::NameMatch::Function { definition, .. } => {
                Some(definition.id().to_string())
            }
            crate::parser::names::NameMatch::Unit { definition, .. } => {
                Some(definition.id().to_string())
            }
            crate::parser::names::NameMatch::Variable { definition } => {
                Some(definition.id().to_string())
            }
            crate::parser::names::NameMatch::Prefix { definition } => {
                Some(definition.id().to_string())
            }
        }
    } else {
        None
    }
}

fn evaluate_represents(
    name: &str,
    args: &[Expression],
    context: &mut CalculatorContext,
    check: fn(&Expression) -> bool,
) -> FunctionResult {
    validate_arity(name, args, 1, Some(1))?;
    if check(&args[0]) {
        return Ok(Expression::Number(Number::from_rational(
            Rational::from_i32(1),
        )));
    }
    let evaluated = crate::eval::evaluate_ast(&args[0], context)
        .map_err(|message| function_error(name, &message))?;
    if check(&evaluated) {
        return Ok(Expression::Number(Number::from_rational(
            Rational::from_i32(1),
        )));
    }
    Ok(Expression::Number(Number::from_rational(
        Rational::from_i32(0),
    )))
}

fn function_error(function_name: &str, message: &str) -> FunctionError {
    FunctionError {
        function_name: function_name.to_string(),
        message: message.to_string(),
    }
}
