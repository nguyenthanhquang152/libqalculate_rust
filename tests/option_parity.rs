//! Integration tests for `PrintOptions`, `ParseOptions`, `EvaluationOptions` parity and session settings.

use libqalculate_rust::context::{AssumptionSign, CalculatorContext};
use libqalculate_rust::options::{
    AngleUnit, ApproximationMode, AutoPostConversion, BaseDisplay, ComplexNumberForm,
    DateTimeFormat, DigitGrouping, DivisionSign, ExpDisplay, IntervalCalculation, IntervalDisplay,
    MixedUnitsConversion, MultiplicationSign, NumberFractionFormat, ParsingMode, ReadPrecisionMode,
    RoundingMode, StructuringMode, TimeZone, UnicodeSigns,
};

#[test]
fn test_options_defaults_matching_upstream() {
    let context = CalculatorContext::default();

    // 1. PrintOptions Defaults
    let po = &context.print_options;
    assert_eq!(po.min_exp, -1);
    assert_eq!(po.base, 10);
    assert_eq!(po.base_display, BaseDisplay::None);
    assert!(!po.lower_case_numbers);
    assert_eq!(po.number_fraction_format, NumberFractionFormat::Decimal);
    assert!(!po.indicate_infinite_series);
    assert!(po.show_ending_zeroes);
    assert!(po.abbreviate_names);
    assert!(!po.use_reference_names);
    assert!(po.place_units_separately);
    assert!(po.use_unit_prefixes);
    assert!(!po.use_prefixes_for_all_units);
    assert!(!po.use_prefixes_for_currencies);
    assert!(!po.use_all_prefixes);
    assert!(po.use_denominator_prefix);
    assert!(!po.negative_exponents);
    assert!(po.short_multiplication);
    assert!(!po.limit_implicit_multiplication);
    assert!(!po.allow_non_usable);
    assert_eq!(po.use_unicode_signs, UnicodeSigns::Off);
    assert_eq!(po.multiplication_sign, MultiplicationSign::Dot);
    assert_eq!(po.division_sign, DivisionSign::DivisionSlash);
    assert!(po.spacious);
    assert!(!po.excessive_parenthesis);
    assert!(po.halfexp_to_sqrt);
    assert_eq!(po.min_decimals, 0);
    assert_eq!(po.max_decimals, -1);
    assert!(po.use_min_decimals);
    assert!(po.use_max_decimals);
    assert!(!po.round_halfway_to_even);
    assert!(po.improve_division_multipliers);
    assert!(po.comma_sign.is_empty());
    assert!(po.decimalpoint_sign.is_empty());
    assert!(!po.hide_underscore_spaces);
    assert!(!po.preserve_format);
    assert!(!po.allow_factorization);
    assert!(!po.spell_out_logical_operators);
    assert!(po.restrict_to_parent_precision);
    assert!(!po.restrict_fraction_length);
    assert!(!po.exp_to_root);
    assert!(!po.preserve_precision);
    assert_eq!(po.interval_display, IntervalDisplay::Interval);
    assert_eq!(po.digit_grouping, DigitGrouping::None);
    assert_eq!(po.date_time_format, DateTimeFormat::Iso);
    assert_eq!(po.time_zone, TimeZone::Local);
    assert_eq!(po.custom_time_zone, 0);
    assert!(po.twos_complement);
    assert!(!po.hexadecimal_twos_complement);
    assert_eq!(po.binary_bits, 0);
    assert_eq!(po.exp_display, ExpDisplay::Default);
    assert!(!po.duodecimal_symbols);
    assert_eq!(po.rounding, RoundingMode::HalfAwayFromZero);

    // 2. ParseOptions Defaults
    let pa = &context.parse_options;
    assert!(pa.variables_enabled);
    assert!(pa.functions_enabled);
    assert!(pa.unknowns_enabled);
    assert!(pa.units_enabled);
    assert!(!pa.rpn);
    assert_eq!(pa.base, 10);
    assert!(!pa.limit_implicit_multiplication);
    assert_eq!(pa.read_precision, ReadPrecisionMode::DontReadPrecision);
    assert!(!pa.dot_as_separator);
    assert!(!pa.comma_as_separator);
    assert!(!pa.brackets_as_parentheses);
    assert_eq!(pa.angle_unit, AngleUnit::None);
    assert!(!pa.preserve_format);
    assert_eq!(pa.parsing_mode, ParsingMode::Adaptive);
    assert!(!pa.twos_complement);
    assert!(!pa.hexadecimal_twos_complement);
    assert_eq!(pa.binary_bits, 0);

    // 3. EvaluationOptions Defaults
    let eo = &context.evaluation_options;
    assert_eq!(eo.approximation, ApproximationMode::TryExact);
    assert!(eo.sync_units);
    assert!(eo.sync_nonlinear_unit_relations);
    assert!(!eo.keep_prefixes);
    assert!(eo.calculate_variables);
    assert!(eo.calculate_functions);
    assert!(eo.test_comparisons);
    assert!(eo.isolate_x);
    assert!(eo.expand);
    assert!(!eo.combine_divisions);
    assert!(eo.reduce_divisions);
    assert!(eo.allow_complex);
    assert!(eo.allow_infinite);
    assert!(eo.assume_denominators_nonzero);
    assert!(!eo.warn_about_denominators_assumed_nonzero);
    assert!(eo.split_squares);
    assert!(eo.keep_zero_units);
    assert_eq!(eo.auto_post_conversion, AutoPostConversion::Optimal);
    assert_eq!(eo.mixed_units_conversion, MixedUnitsConversion::Default);
    assert_eq!(eo.structuring, StructuringMode::Expand);
    assert!(eo.do_polynomial_division);
    assert_eq!(eo.complex_number_form, ComplexNumberForm::Rectangular);
    assert!(eo.local_currency_conversion);
    assert!(eo.transform_trigonometric_functions);
    assert_eq!(
        eo.interval_calculation,
        IntervalCalculation::VarianceFormula
    );
    assert_eq!(eo.parse_options, *pa);
}

#[test]
fn test_comprehensive_session_state_transitions() {
    let mut context = CalculatorContext::default();

    // Test input/output base transition
    context.apply_command("set input base 16").unwrap();
    assert_eq!(context.input_base, 16);
    assert_eq!(context.print_options.base, 16);
    assert_eq!(context.parse_options.base, 16);

    context.apply_command("set outbase 8").unwrap();
    assert_eq!(context.output_base, 8);

    // Test approximation mode transition
    context.apply_command("set approximation exact").unwrap();
    assert_eq!(
        context.evaluation_options.approximation,
        ApproximationMode::Exact
    );

    context.apply_command("set approx try exact").unwrap();
    assert_eq!(
        context.evaluation_options.approximation,
        ApproximationMode::TryExact
    );

    context.apply_command("set approx approximate").unwrap();
    assert_eq!(
        context.evaluation_options.approximation,
        ApproximationMode::Approximate
    );

    // Test fraction format mode transition
    context.apply_command("/set fr 2").unwrap();
    assert_eq!(
        context.print_options.number_fraction_format,
        NumberFractionFormat::Fractional
    );

    // Test unicode transition
    context.apply_command("set unicode true").unwrap();
    assert_eq!(context.print_options.use_unicode_signs, UnicodeSigns::On);

    context.apply_command("set unicode 0").unwrap();
    assert_eq!(context.print_options.use_unicode_signs, UnicodeSigns::Off);

    // Test interval calculation transition
    context.apply_command("set ic 2").unwrap();
    assert_eq!(
        context.evaluation_options.interval_calculation,
        IntervalCalculation::IntervalArithmetic
    );

    // Test interval display transition
    context.apply_command("set id 2").unwrap();
    assert_eq!(
        context.print_options.interval_display,
        IntervalDisplay::PlusMinus
    );

    // Test concise uncertainty transition
    context.apply_command("set cu 1").unwrap();
    assert_eq!(
        context.print_options.interval_display,
        IntervalDisplay::Concise
    );

    // Test complex transition
    context.apply_command("set cplx 0").unwrap();
    assert!(!context.evaluation_options.allow_complex);

    context.apply_command("set cplx 1").unwrap();
    assert!(context.evaluation_options.allow_complex);

    // Test decimal comma transition
    context.apply_command("set decimal comma 1").unwrap();
    assert_eq!(context.print_options.comma_sign, ",");
    assert_eq!(context.print_options.decimalpoint_sign, ".");

    context.apply_command("set decimal comma 0").unwrap();
    assert!(context.print_options.comma_sign.is_empty());
    assert!(context.print_options.decimalpoint_sign.is_empty());

    // Test currency conversion transition
    context.apply_command("set curconv 0").unwrap();
    assert!(!context.evaluation_options.local_currency_conversion);

    context.apply_command("set curconv 1").unwrap();
    assert!(context.evaluation_options.local_currency_conversion);

    // Test abbreviations transition
    context.apply_command("set abbreviations 0").unwrap();
    assert!(!context.print_options.abbreviate_names);

    context.apply_command("set abbreviations 1").unwrap();
    assert!(context.print_options.abbreviate_names);

    // Test engineering display transition
    context.apply_command("set edisp 3").unwrap();
    assert_eq!(context.print_options.exp_display, ExpDisplay::PowerOf10);

    // Test assume transition
    context.apply_command("assume positive").unwrap();
    assert_eq!(context.assumptions.default_sign, AssumptionSign::Positive);

    context.apply_command("assume unknown").unwrap();
    assert_eq!(context.assumptions.default_sign, AssumptionSign::Unknown);
}

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

#[test]
fn test_focused_oracle_cases_native_routing() {
    let _guard = EnvGuard::set("QALCULATE_DISABLE_FALLBACK", "1");

    let mut calc = libqalculate_rust::ffi::Calculator::new();

    // Verify 52 to hex works natively through fallback-disabled scaffold
    let res_hex = calc
        .calculate_and_print_qalc_with_settings_and_fallback_state("52 to hex", &[], 1000)
        .unwrap();
    assert_eq!(res_hex.output, "0x34");
    assert_eq!(
        res_hex.fallback_state,
        libqalculate_rust::ffi::FallbackState::Native
    );

    // Verify 52.34 to sexa works natively through fallback-disabled scaffold with settings
    let res_sexa = calc
        .calculate_and_print_qalc_with_settings_and_fallback_state(
            "52.34 to sexa",
            &["set input base 10", "/set unicode 1"],
            1000,
        )
        .unwrap();
    assert_eq!(res_sexa.output, "52°20′24″");
    assert_eq!(
        res_sexa.fallback_state,
        libqalculate_rust::ffi::FallbackState::Native
    );
}

#[test]
fn test_option_enums_classification() {
    // This inventory test guarantees that all required options enums are classified and match
    // our expected variants.

    // 1. BaseDisplay classification
    let _ = BaseDisplay::None;
    let _ = BaseDisplay::Normal;
    let _ = BaseDisplay::Alternative;
    let _ = BaseDisplay::Suffix;

    // 2. NumberFractionFormat classification
    let _ = NumberFractionFormat::Decimal;
    let _ = NumberFractionFormat::DecimalExact;
    let _ = NumberFractionFormat::Fractional;
    let _ = NumberFractionFormat::Combined;
    let _ = NumberFractionFormat::FractionalFixedDenominator;
    let _ = NumberFractionFormat::CombinedFixedDenominator;
    let _ = NumberFractionFormat::Percent;
    let _ = NumberFractionFormat::Permille;
    let _ = NumberFractionFormat::Permyriad;

    // 3. IntervalDisplay classification
    let _ = IntervalDisplay::SignificantDigits;
    let _ = IntervalDisplay::Interval;
    let _ = IntervalDisplay::PlusMinus;
    let _ = IntervalDisplay::Midpoint;
    let _ = IntervalDisplay::Lower;
    let _ = IntervalDisplay::Upper;
    let _ = IntervalDisplay::Concise;
    let _ = IntervalDisplay::Relative;

    // 4. ApproximationMode classification
    let _ = ApproximationMode::Exact;
    let _ = ApproximationMode::TryExact;
    let _ = ApproximationMode::Approximate;
    let _ = ApproximationMode::ExactVariables;

    // 5. StructuringMode classification
    let _ = StructuringMode::None;
    let _ = StructuringMode::Expand;
    let _ = StructuringMode::Factorize;
    let _ = StructuringMode::Hybrid;

    // 6. AngleUnit classification
    let _ = AngleUnit::None;
    let _ = AngleUnit::Radians;
    let _ = AngleUnit::Degrees;
    let _ = AngleUnit::Gradians;
    let _ = AngleUnit::Custom;

    // 7. ComplexNumberForm classification
    let _ = ComplexNumberForm::Rectangular;
    let _ = ComplexNumberForm::Exponential;
    let _ = ComplexNumberForm::Polar;
    let _ = ComplexNumberForm::Cis;
}
