#include "ffi_bridge.h"

namespace {

class CalculatorStateGuard {
public:
    explicit CalculatorStateGuard(Calculator &calculator)
        : calc(calculator),
          precision(calculator.getPrecision()),
          interval_arithmetic(calculator.usesIntervalArithmetic()),
          temperature_mode(calculator.getTemperatureCalculationMode()),
          decimal_point(calculator.getDecimalPoint()),
          comma(calculator.getComma()) {}

    ~CalculatorStateGuard() noexcept {
        restore_decimal_mode();
        calc.setPrecision(precision);
        calc.useIntervalArithmetic(interval_arithmetic);
        calc.setTemperatureCalculationMode(temperature_mode);
    }

    CalculatorStateGuard(const CalculatorStateGuard&) = delete;
    CalculatorStateGuard& operator=(const CalculatorStateGuard&) = delete;

private:
    void restore_decimal_mode() noexcept {
        if(decimal_point == "," && comma == ";") {
            calc.useDecimalComma();
        } else if(decimal_point == "." && comma == ";") {
            calc.useDecimalPoint(true);
        } else {
            calc.useDecimalPoint(false);
        }
    }

    Calculator &calc;
    int precision;
    bool interval_arithmetic;
    TemperatureCalculationMode temperature_mode;
    std::string decimal_point;
    std::string comma;
};

} // namespace

std::unique_ptr<Calculator> new_calculator() {
    return std::make_unique<Calculator>();
}

bool load_exchange_rates(Calculator &calc) {
    try {
        return calc.loadExchangeRates();
    } catch (...) {
        return false;
    }
}

bool load_global_definitions(Calculator &calc) {
    try {
        return calc.loadGlobalDefinitions();
    } catch (...) {
        return false;
    }
}

bool load_local_definitions(Calculator &calc) {
    try {
        return calc.loadLocalDefinitions();
    } catch (...) {
        return false;
    }
}

// Calls Calculator::calculateAndPrint(const std::string&, int) which uses
// default EvaluationOptions and PrintOptions from upstream libqalculate.
// Any C++ exception is caught and re-thrown as std::runtime_error so that
// CXX can translate it into a Rust Result::Err.
rust::String calculate_and_print(
    Calculator &calc,
    rust::Str expr,
    int32_t timeout_ms
) {
    try {
        std::string expr_str(expr.data(), expr.size());
        return rust::String(calc.calculateAndPrint(expr_str, timeout_ms));
    } catch (const std::exception&) {
        throw;  // CXX handles std::exception natively
    } catch (...) {
        throw std::runtime_error("unknown C++ exception in calculateAndPrint");
    }
}

rust::String calculate_and_print_qalc(
    Calculator &calc,
    rust::Str expr,
    int32_t timeout_ms
) {
    try {
        std::string expr_str(expr.data(), expr.size());
        CalculatorStateGuard state_guard(calc);

        bool is_approximate = false;
        PrintOptions po;
        po.is_approximate = &is_approximate;
        po.use_min_decimals = false;
        po.use_denominator_prefix = true;
        po.min_decimals = 0;
        po.use_max_decimals = false;
        po.max_decimals = 2;
        po.base = 10;
        po.min_exp = EXP_PRECISION;
        po.negative_exponents = false;
        po.sort_options.minus_last = true;
        po.indicate_infinite_series = false;
        po.show_ending_zeroes = true;
        po.digit_grouping = DIGIT_GROUPING_NONE;
        po.rounding = ROUNDING_HALF_AWAY_FROM_ZERO;
        po.number_fraction_format = FRACTION_DECIMAL;
        po.restrict_fraction_length = false;
        po.abbreviate_names = true;
        po.use_unicode_signs = true;
        po.use_unit_prefixes = true;
        po.spacious = true;
        po.short_multiplication = true;
        po.limit_implicit_multiplication = false;
        po.place_units_separately = true;
        po.use_all_prefixes = false;
        po.excessive_parenthesis = false;
        po.allow_non_usable = false;
        po.lower_case_numbers = false;
        po.duodecimal_symbols = false;
        po.exp_display = EXP_UPPERCASE_E;
        po.base_display = BASE_DISPLAY_NORMAL;
        po.twos_complement = true;
        po.hexadecimal_twos_complement = false;
        po.division_sign = DIVISION_SIGN_SLASH;
        po.multiplication_sign = MULTIPLICATION_SIGN_X;
        po.allow_factorization = false;
        po.spell_out_logical_operators = true;
        po.interval_display = INTERVAL_DISPLAY_SIGNIFICANT_DIGITS;

        EvaluationOptions eo;
        eo.approximation = APPROXIMATION_TRY_EXACT;
        eo.sync_units = true;
        eo.structuring = STRUCTURING_SIMPLIFY;
        eo.parse_options.unknowns_enabled = false;
        eo.parse_options.read_precision = DONT_READ_PRECISION;
        eo.parse_options.base = BASE_DECIMAL;
        eo.allow_complex = true;
        eo.allow_infinite = true;
        eo.auto_post_conversion = POST_CONVERSION_OPTIMAL;
        eo.assume_denominators_nonzero = true;
        eo.warn_about_denominators_assumed_nonzero = true;
        eo.parse_options.angle_unit = ANGLE_UNIT_RADIANS;
        eo.parse_options.dot_as_separator = calc.default_dot_as_separator;
        eo.parse_options.comma_as_separator = false;
        eo.mixed_units_conversion = MIXED_UNITS_CONVERSION_DEFAULT;
        eo.complex_number_form = COMPLEX_NUMBER_FORM_RECTANGULAR;
        eo.local_currency_conversion = false;
        eo.interval_calculation = INTERVAL_CALCULATION_VARIANCE_FORMULA;
        eo.parse_options.twos_complement = false;
        eo.parse_options.hexadecimal_twos_complement = false;

        calc.useDecimalPoint(false);
        calc.setPrecision(10);
        calc.useIntervalArithmetic(true);
        calc.setTemperatureCalculationMode(TEMPERATURE_CALCULATION_HYBRID);

        return rust::String(calc.calculateAndPrint(
            expr_str,
            timeout_ms,
            eo,
            po,
            AUTOMATIC_FRACTION_OFF,
            AUTOMATIC_APPROXIMATION_OFF
        ));
    } catch (const std::exception&) {
        throw;
    } catch (...) {
        throw std::runtime_error("unknown C++ exception in qalc-compatible calculateAndPrint");
    }
}
