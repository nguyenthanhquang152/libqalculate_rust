use libqalculate_rust::context::CalculatorContext;
use libqalculate_rust::ffi::{Calculator, FallbackState};

struct EnvGuard {
    name: &'static str,
    old_value: Option<String>,
}

impl EnvGuard {
    fn set(name: &'static str, value: &str) -> Self {
        let old_value = std::env::var(name).ok();
        #[allow(unused_unsafe)]
        unsafe {
            std::env::set_var(name, value);
        }
        Self { name, old_value }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        #[allow(unused_unsafe)]
        unsafe {
            match &self.old_value {
                Some(val) => std::env::set_var(self.name, val),
                None => std::env::remove_var(self.name),
            }
        }
    }
}

fn eval(ctx: &mut CalculatorContext, expr: &str) -> String {
    ctx.parse_and_evaluate_to_string(expr)
        .unwrap_or_else(|err| panic!("failed to evaluate {expr:?}: {err}"))
}

fn eval_err(ctx: &mut CalculatorContext, expr: &str) -> String {
    ctx.parse_and_evaluate_to_string(expr)
        .expect_err(&format!("expected {expr:?} to fail"))
}

#[test]
fn quoted_text_literals_match_strings_batch() {
    let mut ctx = CalculatorContext::new();

    assert_eq!(eval(&mut ctx, r#""""#), r#""""#);
    assert_eq!(eval(&mut ctx, r#""x""#), "'x'");
    assert_eq!(eval(&mut ctx, r#""xx""#), r#""xx""#);
    assert_eq!(eval(&mut ctx, r#""meters""#), r#""meters""#);
    assert_eq!(eval(&mut ctx, "'12'"), r#""12""#);
    assert_eq!(eval(&mut ctx, r#""12""#), r#""12""#);
}

#[test]
fn concatenate_len_and_decimal_text_arguments_match_strings_batch() {
    let mut ctx = CalculatorContext::new();

    assert_eq!(
        eval(&mut ctx, r#"concatenate("a", "bc", 'defg')"#),
        r#""abcdefg""#
    );
    assert_eq!(
        eval(&mut ctx, r#"concatenate("", "c", '', 'd')"#),
        r#""cd""#
    );
    assert_eq!(eval(&mut ctx, "concatenate(1,2)"), r#""12""#);
    assert_eq!(eval(&mut ctx, "concatenate(1*2, 5)"), r#""1*25""#);
    assert_eq!(eval(&mut ctx, "dec(concatenate(4*2, 5))"), "100");

    assert_eq!(eval(&mut ctx, r#"alpha:="c""#), "'c'");
    assert_eq!(eval(&mut ctx, "beta:=2"), "2");
    assert_eq!(
        eval(
            &mut ctx,
            "concatenate(concatenate(a, b), alpha, d, dec(123, 1), beta)"
        ),
        r#""abcd123beta""#
    );

    assert_eq!(eval(&mut ctx, r#"len("")"#), "0");
    assert_eq!(eval(&mut ctx, r#"len(" ")"#), "1");
    assert_eq!(eval(&mut ctx, "len(5)"), "1");
    assert_eq!(eval(&mut ctx, "len(5/6)"), "3");
    assert_eq!(eval(&mut ctx, r#"len(concatenate("a", "bc"))"#), "3");
}

#[test]
fn unicode_char_and_code_match_strings_batch() {
    let mut ctx = CalculatorContext::new();
    ctx.apply_command("/set unicode 1").unwrap();

    assert_eq!(eval(&mut ctx, "0xD8 to unicode"), "Ø");
    assert_eq!(eval(&mut ctx, "char(0xD8)"), "'Ø'");
    assert_eq!(eval(&mut ctx, "char([0xD8, 0x61])"), "['Ø'  'a']");
    assert_eq!(eval(&mut ctx, "code(Ø) to hex"), "D8");
    assert_eq!(eval(&mut ctx, "code(😀) to hex"), "1F600");
    assert_eq!(eval(&mut ctx, "code(🍉, utf-8, 0) to hex"), "F09F8D89");
    assert_eq!(eval(&mut ctx, "code(abc)"), "[97  98  99]");
}

#[test]
fn qalc_profile_prefixes_hex_unicode_code_output() {
    let _guard = EnvGuard::set("QALCULATE_DISABLE_FALLBACK", "1");
    let mut calc = Calculator::new();

    for (expr, expected) in [
        ("code(Ø) to hex", "0xD8"),
        ("code(😀) to hex", "0x1F600"),
        ("code(🍉, utf-8, 0) to hex", "0xF09F8D89"),
    ] {
        let result = calc
            .calculate_and_print_qalc_with_settings_and_fallback_state(
                expr,
                &["/set unicode 1"],
                1000,
            )
            .unwrap_or_else(|err| panic!("failed to evaluate {expr:?}: {err}"));
        assert_eq!(result.output, expected);
        assert_eq!(result.fallback_state, FallbackState::Native);
    }
}

#[test]
fn invalid_text_and_codepoints_are_rejected() {
    let mut ctx = CalculatorContext::new();

    assert!(eval_err(&mut ctx, r#""unterminated"#).contains("Unterminated"));
    assert!(eval_err(&mut ctx, "\"a\0b\"").contains("InteriorNul"));
    assert!(eval_err(&mut ctx, "char(0)").contains("Invalid Unicode code point"));
    assert!(eval_err(&mut ctx, "char(0xD800)").contains("Invalid Unicode code point"));
    assert!(eval_err(&mut ctx, r#"code("")"#).contains("Expected non-empty text"));
}

fn eval_to_expr(ctx: &mut CalculatorContext, expr: &str) -> libqalculate_rust::ast::Expression {
    let parsed = libqalculate_rust::parser::operators::parse_expression(expr)
        .unwrap_or_else(|err| panic!("failed to parse {expr:?}: {err}"));
    libqalculate_rust::eval::evaluate_ast(&parsed, ctx)
        .unwrap_or_else(|err| panic!("failed to evaluate {expr:?}: {err}"))
}

fn assert_eval_eq(ctx: &mut CalculatorContext, expr: &str, expected_expr: &str) {
    let evaluated = eval_to_expr(ctx, expr);
    let simplified_eval = libqalculate_rust::simplify::simplify_ast(&evaluated, ctx);
    let expected = libqalculate_rust::parser::operators::parse_expression(expected_expr).unwrap();
    let simplified_expected = libqalculate_rust::simplify::simplify_ast(&expected, ctx);
    assert_eq!(simplified_eval, simplified_expected);
}

#[test]
fn replace_nounit_title_represents_batch() {
    let mut ctx = CalculatorContext::new();
    ctx.definitions.add_unit("m");
    ctx.definitions.add_unit("s");

    // replace
    assert_eval_eq(&mut ctx, "replace(x + 1, x, y)", "y + 1");
    assert_eval_eq(&mut ctx, "replace(2 * x + y, x, 3)", "2 * 3 + y");
    assert_eval_eq(&mut ctx, "replace(x + y, [x, y], [a, b])", "a + b");

    // replace with precalc
    ctx.variables.insert(
        "v".to_string(),
        libqalculate_rust::parser::operators::parse_expression("x + 1").unwrap(),
    );
    assert_eval_eq(&mut ctx, "replace(v, x, y, 0)", "v");
    assert_eval_eq(&mut ctx, "replace(v, x, y, 1)", "y + 1");

    // replace not found warning
    let len_before = ctx.messages.len();
    let _ = eval_to_expr(&mut ctx, "replace(x + 1, z, y)");
    assert!(ctx.messages.len() > len_before);
    assert!(ctx
        .messages
        .get_messages()
        .iter()
        .any(|msg| msg.message().contains("was not found")));

    // nounit
    assert_eq!(eval(&mut ctx, "nounit(5 m)"), "5");
    assert_eq!(eval(&mut ctx, "nounit(5 m/s)"), "5");
    assert_eval_eq(&mut ctx, "nounit(x m + y s)", "x + y");
    assert_eval_eq(&mut ctx, "nounit((5 m)^2)", "25");
    assert_eval_eq(&mut ctx, "nounit(5 m / (2 s))", "5 / 2");

    // title
    assert_eq!(eval(&mut ctx, "title(sgn)"), r#""Signum""#);
    assert_eq!(eval(&mut ctx, "title(m)"), r#""meter""#);
    assert!(eval_err(&mut ctx, "title(nonexistent)").contains("does not exist"));

    // representsInteger
    assert_eq!(eval(&mut ctx, "representsInteger(5)"), "1");
    assert_eq!(eval(&mut ctx, "representsInteger(5.0)"), "1");
    assert_eq!(eval(&mut ctx, "representsInteger(5/1)"), "1");
    assert_eq!(eval(&mut ctx, "representsInteger(5.5)"), "0");
    assert_eq!(eval(&mut ctx, "representsInteger(x)"), "0");

    // representsRational
    assert_eq!(eval(&mut ctx, "representsRational(2/3)"), "1");
    assert_eq!(eval(&mut ctx, "representsRational(2.5)"), "1");

    // representsReal
    assert_eq!(eval(&mut ctx, "representsReal(5.5)"), "1");
    assert_eq!(eval(&mut ctx, "representsReal(x)"), "0");

    // representsNumber
    assert_eq!(eval(&mut ctx, "representsNumber(5.5)"), "1");
    assert_eq!(eval(&mut ctx, "representsNumber(3.14)"), "1");
}
