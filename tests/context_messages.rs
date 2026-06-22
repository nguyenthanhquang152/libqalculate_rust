//! Integration tests for `CalculatorContext` and message queue functionality.

use libqalculate_rust::context::{AssumptionSign, AssumptionType, CalculatorContext};
use libqalculate_rust::messages::{MessageCategory, MessageStage, MessageType};
use libqalculate_rust::options::{AngleUnit, ApproximationMode, IntervalDisplay, UnicodeSigns};

#[test]
fn test_default_calculator_context_values() {
    let context = CalculatorContext::default();

    // Verify root context defaults
    assert_eq!(context.precision_digits, 8);
    assert_eq!(context.input_base, 10);
    assert_eq!(context.output_base, 10);
    assert_eq!(context.angle_unit, AngleUnit::None);

    // Verify PrintOptions defaults
    assert_eq!(context.print_options.base, 10);
    assert_eq!(context.print_options.use_unicode_signs, UnicodeSigns::Off);
    assert_eq!(
        context.print_options.interval_display,
        IntervalDisplay::Interval
    );
    assert!(context.print_options.show_ending_zeroes);
    assert!(context.print_options.abbreviate_names);

    // Verify EvaluationOptions defaults
    assert_eq!(
        context.evaluation_options.approximation,
        ApproximationMode::TryExact
    );
    assert!(context.evaluation_options.allow_complex);
    assert!(context.evaluation_options.allow_infinite);

    // Verify Assumptions defaults
    assert_eq!(context.assumptions.default_type, AssumptionType::Real);
    assert_eq!(context.assumptions.default_sign, AssumptionSign::Unknown);

    // Verify MessageQueue is empty
    assert!(context.messages.is_empty());
}

#[test]
fn test_session_state_transitions() {
    let mut context = CalculatorContext::default();

    // 1. set input base 16
    context.apply_command("set input base 16").unwrap();
    assert_eq!(context.input_base, 16);
    assert_eq!(context.print_options.base, 16);
    assert_eq!(context.parse_options.base, 16);

    // 2. set input base 10
    context.apply_command("set input base 10").unwrap();
    assert_eq!(context.input_base, 10);
    assert_eq!(context.print_options.base, 10);
    assert_eq!(context.parse_options.base, 10);

    // 3. /set unicode 1
    context.apply_command("/set unicode 1").unwrap();
    assert_eq!(context.print_options.use_unicode_signs, UnicodeSigns::On);

    // 4. /set approximation exact
    context.apply_command("/set approximation exact").unwrap();
    assert_eq!(
        context.evaluation_options.approximation,
        ApproximationMode::Exact
    );
}

#[test]
fn test_message_queue_warnings_and_errors() {
    let mut context = CalculatorContext::default();

    // 1. Parse Error test: "5 := x" has an invalid assignment target.
    let parse_res = context.parse_and_evaluate_with_context("5 := x");
    assert!(parse_res.is_err());

    // Verify a parser error was logged
    assert_eq!(context.messages.len(), 1);
    let parser_msg = context.messages.message().unwrap();
    assert_eq!(parser_msg.message_type(), MessageType::Error);
    assert_eq!(parser_msg.category(), MessageCategory::Parsing);
    assert_eq!(parser_msg.stage(), MessageStage::Parsing);

    // 2. Evaluator Warning test: "0 / 0" results in NaN warning.
    let eval_res = context.parse_and_evaluate_with_context("0 / 0").unwrap();
    assert!(eval_res.is_nan());

    // Verify an evaluator warning was logged in correct order (index 1)
    assert_eq!(context.messages.len(), 2);
    let messages = context.messages.get_messages();
    let eval_msg = &messages[1];
    assert_eq!(eval_msg.message_type(), MessageType::Warning);
    assert_eq!(eval_msg.category(), MessageCategory::None);
    assert_eq!(eval_msg.stage(), MessageStage::Calculation);
    assert!(eval_msg.message().contains("NaN"));

    // Verify ordering and retrieval via next_message
    let first = context.messages.next_message().unwrap();
    assert_eq!(first.stage(), MessageStage::Parsing);

    let second = context.messages.next_message().unwrap();
    assert_eq!(second.stage(), MessageStage::Calculation);

    assert!(context.messages.is_empty());
}

#[test]
fn test_no_fallback_assertion() {
    // Assert that the native context parsing/evaluating flows do not touch C++ fallback.
    // Setting QALCULATE_DISABLE_FALLBACK=1 ensures FFI fallback errors are thrown if used.
    std::env::set_var("QALCULATE_DISABLE_FALLBACK", "1");

    let mut context = CalculatorContext::default();

    // Evaluating "0 / 0" natively should not fail due to fallback disabled,
    // since it evaluates natively without falling back.
    let res = context.parse_and_evaluate_with_context("0 / 0");
    assert!(res.is_ok());
    assert!(res.unwrap().is_nan());

    // Invalid syntax "5 := x" should also be handled natively by our parser,
    // and fail natively without falling back to C++.
    let res2 = context.parse_and_evaluate_with_context("5 := x");
    assert!(res2.is_err());
}
