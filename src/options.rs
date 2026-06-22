//! Parsed options for printing, parsing, and evaluating mathematical expressions.
//!
//! Seeded from C++ libqalculate defaults in `includes.h` and `Calculator.h`.

/// How number base prefix/suffix is displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum BaseDisplay {
    /// Do not show number base.
    None = 0,
    /// Normal display (e.g. 0x for hex).
    Normal = 1,
    /// Alternative representation.
    Alternative = 2,
    /// Suffix style (e.g. _16).
    Suffix = 3,
}

/// Format of fractions and decimals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum NumberFractionFormat {
    /// Decimal format (e.g. 0.333333).
    Decimal = 0,
    /// Decimal if exact, else fraction.
    DecimalExact = 1,
    /// Fractional format (e.g. 1/3).
    Fractional = 2,
    /// Combined integer and fraction (e.g. 1 + 1/3).
    Combined = 3,
    /// Fractional with fixed denominator.
    FractionalFixedDenominator = 4,
    /// Combined with fixed denominator.
    CombinedFixedDenominator = 5,
    /// Percentage (e.g. 33.3%).
    Percent = 6,
    /// Permille (e.g. 333‰).
    Permille = 7,
    /// Permyriad (‱).
    Permyriad = 8,
}

/// Sign used to display multiplication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum MultiplicationSign {
    /// Dot sign (·).
    Dot = 0,
    /// Cross sign (×).
    X = 1,
    /// Alternative dot sign.
    AltDot = 2,
}

/// Sign used to display division.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum DivisionSign {
    /// Forward slash (/).
    Slash = 0,
    /// Division slash (∕).
    DivisionSlash = 1,
    /// Division sign (÷).
    Division = 2,
}

/// How number intervals are displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum IntervalDisplay {
    /// Significant digits.
    SignificantDigits = 0,
    /// Bounded interval (e.g. [1, 2]).
    Interval = 1,
    /// Plus-minus representation (e.g. 1.5 ± 0.5).
    PlusMinus = 2,
    /// Midpoint representation.
    Midpoint = 3,
    /// Lower bound only.
    Lower = 4,
    /// Upper bound only.
    Upper = 5,
    /// Concise representation.
    Concise = 6,
    /// Relative uncertainty.
    Relative = 7,
}

/// Digit grouping style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum DigitGrouping {
    /// No grouping.
    None = 0,
    /// Standard grouping.
    Standard = 1,
    /// Locale-specific grouping.
    Locale = 2,
}

/// Format for date and time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum DateTimeFormat {
    /// ISO format.
    Iso = 0,
    /// Locale-specific format.
    Locale = 1,
}

/// Time zone used for date/time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum TimeZone {
    /// UTC time.
    Utc = 0,
    /// Local time.
    Local = 1,
    /// Custom offset timezone.
    Custom = 2,
}

/// Mode for exponent display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ExpDisplay {
    /// Default style.
    Default = 0,
    /// Uppercase E.
    UppercaseE = 1,
    /// Lowercase e.
    LowercaseE = 2,
    /// Power of 10 style.
    PowerOf10 = 3,
}

/// Rounding mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum RoundingMode {
    /// Half away from zero (default).
    HalfAwayFromZero = 0,
    /// Half to even.
    HalfToEven = 1,
    /// Half to odd.
    HalfToOdd = 2,
    /// Half toward zero.
    HalfTowardZero = 3,
    /// Half up.
    HalfUp = 4,
    /// Half down.
    HalfDown = 5,
    /// Half random.
    HalfRandom = 6,
    /// Toward zero.
    TowardZero = 7,
    /// Away from zero.
    AwayFromZero = 8,
    /// Up.
    Up = 9,
    /// Down.
    Down = 10,
}

/// Approximation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ApproximationMode {
    /// Allow only exact results.
    Exact = 0,
    /// Try to make the result as exact as possible.
    TryExact = 1,
    /// Calculate the result approximately directly.
    Approximate = 2,
    /// Exact variables only.
    ExactVariables = 3,
}

/// Structuring/simplification mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum StructuringMode {
    /// No structuring.
    None = 0,
    /// Simplify and expand.
    Expand = 1,
    /// Factorize the result.
    Factorize = 2,
    /// Hybrid mode.
    Hybrid = 3,
}

/// Post-calculation unit conversion mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum AutoPostConversion {
    /// No post conversion.
    None = 0,
    /// Optimal SI conversion.
    OptimalSi = 1,
    /// Base units.
    Base = 2,
    /// Optimal conversion.
    Optimal = 3,
}

/// Mixed units conversion style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum MixedUnitsConversion {
    /// No mixed units.
    None = 0,
    /// Downwards conversion, keep units.
    DownwardsKeep = 1,
    /// Downwards conversion.
    Downwards = 2,
    /// Default mixed units conversion.
    Default = 3,
    /// Force integer values.
    ForceInteger = 4,
    /// Force all.
    ForceAll = 5,
}

/// Reading precision mode for parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ReadPrecisionMode {
    /// Do not read precision from digit length.
    DontReadPrecision = 0,
    /// Always read precision.
    AlwaysReadPrecision = 1,
    /// Read precision when decimals are present.
    ReadPrecisionWhenDecimals = 2,
}

/// Angle unit mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum AngleUnit {
    /// No default angle unit.
    None = 0,
    /// Radians.
    Radians = 1,
    /// Degrees.
    Degrees = 2,
    /// Gradians.
    Gradians = 3,
    /// Custom.
    Custom = 4,
}

/// Form of complex numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ComplexNumberForm {
    /// Rectangular form (a + bi).
    Rectangular = 0,
    /// Exponential form (r * e^(i*theta)).
    Exponential = 1,
    /// Polar form (r ∠ theta).
    Polar = 2,
    /// Cis form (r * cis(theta)).
    Cis = 3,
}

/// Parsing mode style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ParsingMode {
    /// Adaptive style.
    Adaptive = 0,
    /// Parse implicit multiplication first.
    ImplicitMultiplicationFirst = 1,
    /// Conventional multiplication ordering.
    Conventional = 2,
    /// Chain style execution.
    Chain = 3,
    /// Reverse Polish Notation.
    Rpn = 4,
}

/// Method for uncertainty interval calculation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum IntervalCalculation {
    /// Ignore intervals/uncertainties.
    None = 0,
    /// Variance formula propagation.
    VarianceFormula = 1,
    /// Rigorous interval arithmetic.
    IntervalArithmetic = 2,
    /// Simple interval arithmetic (uncorrelated).
    SimpleIntervalArithmetic = 3,
}

/// Options for Unicode formatting usage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum UnicodeSigns {
    /// Unicode signs off.
    Off = 0,
    /// Unicode signs on.
    On = 1,
    /// Unicode signs only for unit exponents.
    OnlyUnitExponents = 2,
    /// Unicode signs without exponents.
    WithoutExponents = 3,
}

/// Options for formatting and displaying expressions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintOptions {
    /// Minimum exponent before switching to scientific notation.
    pub min_exp: i32,
    /// Number base for displaying numbers.
    pub base: i32,
    /// Prefix/suffix display style for the number base.
    pub base_display: BaseDisplay,
    /// Lower case for non-numeric characters for bases > 10.
    pub lower_case_numbers: bool,
    /// If rational numbers will be displayed with decimals, fractions, or combined.
    pub number_fraction_format: NumberFractionFormat,
    /// Show three dots for infinite digit series instead of rounding.
    pub indicate_infinite_series: bool,
    /// Show ending zeroes to indicate precision.
    pub show_ending_zeroes: bool,
    /// Prefer abbreviated names of variables, units, etc.
    pub abbreviate_names: bool,
    /// Prefer reference names.
    pub use_reference_names: bool,
    /// Isolate units at the end of the expression.
    pub place_units_separately: bool,
    /// Use prefixes for units when appropriate.
    pub use_unit_prefixes: bool,
    /// Use SI prefixes for all units.
    pub use_prefixes_for_all_units: bool,
    /// Use prefixes for currencies.
    pub use_prefixes_for_currencies: bool,
    /// Use all prefixes.
    pub use_all_prefixes: bool,
    /// Split prefixes between numerator and denominator.
    pub use_denominator_prefix: bool,
    /// Use negative exponents instead of division.
    pub negative_exponents: bool,
    /// Avoid using multiplication signs when appropriate.
    pub short_multiplication: bool,
    /// Limit implicit multiplication format.
    pub limit_implicit_multiplication: bool,
    /// Allow output that cannot be parsed back.
    pub allow_non_usable: bool,
    /// Level of Unicode signs allowed.
    pub use_unicode_signs: UnicodeSigns,
    /// Sign used for multiplication.
    pub multiplication_sign: MultiplicationSign,
    /// Sign used for division.
    pub division_sign: DivisionSign,
    /// Space formatting.
    pub spacious: bool,
    /// Excessive parentheses.
    pub excessive_parenthesis: bool,
    /// Transform raised to 1/2 to square root.
    pub halfexp_to_sqrt: bool,
    /// Minimum decimals.
    pub min_decimals: i32,
    /// Maximum decimals.
    pub max_decimals: i32,
    /// Use minimum decimals limit.
    pub use_min_decimals: bool,
    /// Use maximum decimals limit.
    pub use_max_decimals: bool,
    /// Round halfway to even.
    pub round_halfway_to_even: bool,
    /// Improve division multipliers.
    pub improve_division_multipliers: bool,
    /// Comma sign string.
    pub comma_sign: String,
    /// Decimal point sign string.
    pub decimalpoint_sign: String,
    /// Hide underscore spaces.
    pub hide_underscore_spaces: bool,
    /// Preserve formatting exactly.
    pub preserve_format: bool,
    /// Allow factorization in output.
    pub allow_factorization: bool,
    /// Spell out logical operators.
    pub spell_out_logical_operators: bool,
    /// Restrict to parent precision.
    pub restrict_to_parent_precision: bool,
    /// Restrict fraction length.
    pub restrict_fraction_length: bool,
    /// Exponent to root.
    pub exp_to_root: bool,
    /// Preserve individual precision of numbers.
    pub preserve_precision: bool,
    /// Interval display style.
    pub interval_display: IntervalDisplay,
    /// Digit grouping style.
    pub digit_grouping: DigitGrouping,
    /// Format for date/time.
    pub date_time_format: DateTimeFormat,
    /// Timezone.
    pub time_zone: TimeZone,
    /// Custom timezone offset in minutes.
    pub custom_time_zone: i32,
    /// Twos complement for binary numbers.
    pub twos_complement: bool,
    /// Twos complement for hexadecimal numbers.
    pub hexadecimal_twos_complement: bool,
    /// Number of bits for twos complement.
    pub binary_bits: u32,
    /// Exponent display style.
    pub exp_display: ExpDisplay,
    /// Duodecimal symbols.
    pub duodecimal_symbols: bool,
    /// Rounding method.
    pub rounding: RoundingMode,
}

impl Default for PrintOptions {
    fn default() -> Self {
        Self {
            min_exp: -1, // EXP_PRECISION
            base: 10,
            base_display: BaseDisplay::None,
            lower_case_numbers: false,
            number_fraction_format: NumberFractionFormat::Decimal,
            indicate_infinite_series: false,
            show_ending_zeroes: true,
            abbreviate_names: true,
            use_reference_names: false,
            place_units_separately: true,
            use_unit_prefixes: true,
            use_prefixes_for_all_units: false,
            use_prefixes_for_currencies: false,
            use_all_prefixes: false,
            use_denominator_prefix: true,
            negative_exponents: false,
            short_multiplication: true,
            limit_implicit_multiplication: false,
            allow_non_usable: false,
            use_unicode_signs: UnicodeSigns::Off,
            multiplication_sign: MultiplicationSign::Dot,
            division_sign: DivisionSign::DivisionSlash,
            spacious: true,
            excessive_parenthesis: false,
            halfexp_to_sqrt: true,
            min_decimals: 0,
            max_decimals: -1,
            use_min_decimals: true,
            use_max_decimals: true,
            round_halfway_to_even: false,
            improve_division_multipliers: true,
            comma_sign: String::new(),
            decimalpoint_sign: String::new(),
            hide_underscore_spaces: false,
            preserve_format: false,
            allow_factorization: false,
            spell_out_logical_operators: false,
            restrict_to_parent_precision: true,
            restrict_fraction_length: false,
            exp_to_root: false,
            preserve_precision: false,
            interval_display: IntervalDisplay::Interval,
            digit_grouping: DigitGrouping::None,
            date_time_format: DateTimeFormat::Iso,
            time_zone: TimeZone::Local,
            custom_time_zone: 0,
            twos_complement: true,
            hexadecimal_twos_complement: false,
            binary_bits: 0,
            exp_display: ExpDisplay::Default,
            duodecimal_symbols: false,
            rounding: RoundingMode::HalfAwayFromZero,
        }
    }
}

/// Options for parsing expressions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseOptions {
    /// Enable variable parsing.
    pub variables_enabled: bool,
    /// Enable function parsing.
    pub functions_enabled: bool,
    /// Parse unknowns as symbolic variables.
    pub unknowns_enabled: bool,
    /// Enable unit parsing.
    pub units_enabled: bool,
    /// Parse with Reverse Polish Notation syntax.
    pub rpn: bool,
    /// Base of parsed numbers.
    pub base: i32,
    /// Limit implicit multiplication syntax.
    pub limit_implicit_multiplication: bool,
    /// Method of reading precision from digits.
    pub read_precision: ReadPrecisionMode,
    /// Dot as thousands separator.
    pub dot_as_separator: bool,
    /// Comma as thousands separator.
    pub comma_as_separator: bool,
    /// Interpret square brackets as parentheses.
    pub brackets_as_parentheses: bool,
    /// Default angle unit for trig functions.
    pub angle_unit: AngleUnit,
    /// Preserve parsed expression structure.
    pub preserve_format: bool,
    /// Parsing mode style.
    pub parsing_mode: ParsingMode,
    /// Binary twos complement assumption.
    pub twos_complement: bool,
    /// Hexadecimal twos complement assumption.
    pub hexadecimal_twos_complement: bool,
    /// Bits used for binary/hexadecimal numbers.
    pub binary_bits: u32,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            variables_enabled: true,
            functions_enabled: true,
            unknowns_enabled: true,
            units_enabled: true,
            rpn: false,
            base: 10,
            limit_implicit_multiplication: false,
            read_precision: ReadPrecisionMode::DontReadPrecision,
            dot_as_separator: false,
            comma_as_separator: false,
            brackets_as_parentheses: false,
            angle_unit: AngleUnit::None,
            preserve_format: false,
            parsing_mode: ParsingMode::Adaptive,
            twos_complement: false,
            hexadecimal_twos_complement: false,
            binary_bits: 0,
        }
    }
}

/// Options for evaluating expressions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluationOptions {
    /// Control approximation level.
    pub approximation: ApproximationMode,
    /// Synchronize compatible units during math operations.
    pub sync_units: bool,
    /// Sync non-linear unit relations (e.g. Celsius).
    pub sync_nonlinear_unit_relations: bool,
    /// Keep unit prefixes in original expression.
    pub keep_prefixes: bool,
    /// Replace variables with values.
    pub calculate_variables: bool,
    /// Calculate functions.
    pub calculate_functions: bool,
    /// Evaluate comparisons.
    pub test_comparisons: bool,
    /// Isolate a variable in equations.
    pub isolate_x: bool,
    /// Expand factorization/parentheses.
    pub expand: bool,
    /// Combine divisions.
    pub combine_divisions: bool,
    /// Reduce non-numerical parts of division.
    pub reduce_divisions: bool,
    /// Allow complex numbers in evaluation.
    pub allow_complex: bool,
    /// Allow infinity in evaluation.
    pub allow_infinite: bool,
    /// Assume denominators are non-zero.
    pub assume_denominators_nonzero: bool,
    /// Warn when denominator is assumed non-zero.
    pub warn_about_denominators_assumed_nonzero: bool,
    /// Split squares to least base.
    pub split_squares: bool,
    /// Keep zero units.
    pub keep_zero_units: bool,
    /// Post-calculation unit conversion behavior.
    pub auto_post_conversion: AutoPostConversion,
    /// Mixed units display style.
    pub mixed_units_conversion: MixedUnitsConversion,
    /// Simplification structure style.
    pub structuring: StructuringMode,
    /// Use polynomial division for simplification.
    pub do_polynomial_division: bool,
    /// Complex number form.
    pub complex_number_form: ComplexNumberForm,
    /// Convert currency to local settings.
    pub local_currency_conversion: bool,
    /// Transform trig functions (e.g. sin to cos).
    pub transform_trigonometric_functions: bool,
    /// Calculation style for uncertainties.
    pub interval_calculation: IntervalCalculation,
    /// Parse options nested inside evaluation options.
    pub parse_options: ParseOptions,
}

impl Default for EvaluationOptions {
    fn default() -> Self {
        Self {
            approximation: ApproximationMode::TryExact,
            sync_units: true,
            sync_nonlinear_unit_relations: true,
            keep_prefixes: false,
            calculate_variables: true,
            calculate_functions: true,
            test_comparisons: true,
            isolate_x: true,
            expand: true,
            combine_divisions: false,
            reduce_divisions: true,
            allow_complex: true,
            allow_infinite: true,
            assume_denominators_nonzero: true,
            warn_about_denominators_assumed_nonzero: false,
            split_squares: true,
            keep_zero_units: true,
            auto_post_conversion: AutoPostConversion::Optimal,
            mixed_units_conversion: MixedUnitsConversion::Default,
            structuring: StructuringMode::Expand, // default: simplify = expand
            do_polynomial_division: true,
            complex_number_form: ComplexNumberForm::Rectangular,
            local_currency_conversion: true,
            transform_trigonometric_functions: true,
            interval_calculation: IntervalCalculation::VarianceFormula,
            parse_options: ParseOptions::default(),
        }
    }
}
