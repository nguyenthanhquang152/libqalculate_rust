#include "ffi_bridge.h"
#include "Variable.h"

#include <algorithm>
#include <array>
#include <chrono>
#include <stdexcept>

namespace {

thread_local bool last_qalc_result_is_approximate = false;
thread_local bool last_qalc_markup_output_complete = false;
thread_local std::string last_qalc_messages_text;
thread_local std::string last_qalc_parsed_expression;
thread_local std::size_t last_qalc_messages_line_count = 0;
thread_local bool last_qalc_message_had_error = false;

// Keep these bit assignments synchronized with the named Rust constants in
// `ffi.rs`; they are the compact options ABI for calculate_and_print_qalc.
constexpr std::uint8_t QALC_MODE_MARKUP = 1 << 0;
constexpr std::uint8_t QALC_MODE_LATEX = 1 << 1;
constexpr std::uint8_t QALC_MODE_TERSE = 1 << 2;
constexpr std::uint8_t QALC_MODE_CAPTURE_RESULT = 1 << 3;
constexpr std::uint8_t QALC_MODE_UNICODE = 1 << 4;

void replace_all(std::string &text, const std::string &from, const std::string &to) {
    std::size_t offset = 0;
    while((offset = text.find(from, offset)) != std::string::npos) {
        text.replace(offset, from.size(), to);
        offset += to.size();
    }
}

std::size_t utf8_length(const std::string &text) {
    return static_cast<std::size_t>(std::count_if(
        text.begin(),
        text.end(),
        [](unsigned char byte) { return (byte & 0xC0) != 0x80; }
    ));
}

std::string qalc_table_tabs(const std::string &label) {
    const std::size_t length = utf8_length(label);
    const std::size_t count = length >= 32 ? 1
        : length >= 24            ? 2
        : length >= 16            ? 3
        : length >= 8             ? 4
                                  : 5;
    return std::string(count, '\t');
}

std::string format_qalc_calendar_table(const std::string &calendar_lines) {
    std::string output = "Calendar" + qalc_table_tabs("Calendar") +
        "Day, Month, Year\n";
    std::size_t offset = 0;
    while(offset < calendar_lines.size()) {
        const std::size_t end = calendar_lines.find('\n', offset);
        const std::string line = calendar_lines.substr(offset, end - offset);
        if(!line.empty()) {
            const std::size_t separator = line.find(": ");
            if(separator == std::string::npos) {
                output += line;
            } else {
                const std::string label = line.substr(0, separator + 1);
                output += label;
                output += qalc_table_tabs(label);
                output += line.substr(separator + 2);
            }
            output += '\n';
        }
        if(end == std::string::npos) break;
        offset = end + 1;
    }
    if(!output.empty() && output.back() == '\n') output.pop_back();
    return output;
}

std::string qalc_calendar_lines(const MathStructure &result) {
    if(!result.isDateTime()) return {};
    std::string output;
    const QalculateDateTime &date = *result.datetime();
    auto append = [&](const char *label, CalendarSystem calendar, bool chinese = false) {
        if(!output.empty()) output += '\n';
        output += label;
        output += ' ';
        long year = 0;
        long month = 0;
        long day = 0;
        if(!dateToCalendar(date, year, month, day, calendar)) {
            output += "failed";
            return;
        }
        output += std::to_string(day);
        output += ' ';
        output += monthName(month, calendar, true);
        output += ' ';
        output += std::to_string(year);
        if(chinese) {
            long cycle = 0;
            long year_in_cycle = 0;
            long stem = 0;
            long branch = 0;
            chineseYearInfo(year, cycle, year_in_cycle, stem, branch);
            output += " (";
            output += chineseStemName(stem);
            output += ' ';
            output += chineseBranchName(branch);
            output += ')';
        }
    };
    append("Gregorian:", CALENDAR_GREGORIAN);
    append("Hebrew:", CALENDAR_HEBREW);
    append("Islamic:", CALENDAR_ISLAMIC);
    append("Persian:", CALENDAR_PERSIAN);
    append("Indian national:", CALENDAR_INDIAN);
    append("Chinese:", CALENDAR_CHINESE, true);
    append("Julian:", CALENDAR_JULIAN);
    append("Revised julian:", CALENDAR_MILANKOVIC);
    append("Coptic:", CALENDAR_COPTIC);
    append("Ethiopian:", CALENDAR_ETHIOPIAN);
    return output;
}

void capture_qalc_messages(Calculator &calc) {
    last_qalc_messages_text.clear();
    last_qalc_messages_line_count = 0;
    last_qalc_message_had_error = false;

    CalculatorMessage *message = calc.message();
    while(message) {
        const std::string message_text = message->message();
        if(message_text.empty()) {
            message = calc.nextMessage();
            continue;
        }
        std::string rendered;
        if(message->type() == MESSAGE_ERROR) {
            rendered = "error: ";
            last_qalc_message_had_error = true;
        } else if(message->type() == MESSAGE_WARNING) {
            rendered = "warning: ";
        }
        rendered += message_text;
        if(!last_qalc_messages_text.empty()) last_qalc_messages_text += '\n';
        last_qalc_messages_text += rendered;
        last_qalc_messages_line_count +=
            static_cast<std::size_t>(std::count(rendered.begin(), rendered.end(), '\n')) + 1;
        message = calc.nextMessage();
    }
}

PrintOptions qalc_print_options(bool unicode_enabled, bool *is_approximate) {
    PrintOptions po;
    po.is_approximate = is_approximate;
    po.use_min_decimals = false;
    po.use_denominator_prefix = true;
    po.min_decimals = 0;
    po.use_max_decimals = false;
    po.max_decimals = 2;
    po.base = BASE_DECIMAL;
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
    po.use_unicode_signs = unicode_enabled;
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
    return po;
}

std::array<KnownVariable*, 5> ensure_qalc_session_answers(Calculator &calc) {
    std::array<KnownVariable*, 5> answers{};
    const std::array<std::string, 5> names = {"ans", "ans2", "ans3", "ans4", "ans5"};
    MathStructure undefined;
    undefined.setUndefined();
    for(std::size_t index = 0; index < names.size(); ++index) {
        answers[index] = dynamic_cast<KnownVariable*>(calc.getVariable(names[index]));
        if(answers[index]) continue;
        Variable *registered = calc.addVariable(new KnownVariable(
            calc.temporaryCategory(),
            names[index],
            undefined,
            index == 0 ? "Last Answer" : "Previous Answer",
            false,
            true
        ));
        answers[index] = dynamic_cast<KnownVariable*>(registered);
        if(!answers[index]) {
            throw std::runtime_error("failed to register qalc session answer variable");
        }
        if(index == 0) {
            answers[index]->addName("answer");
            answers[index]->addName("ans1");
        }
    }
    return answers;
}

void rotate_qalc_session_answers(Calculator &calc, const MathStructure &result) {
    auto answers = ensure_qalc_session_answers(calc);
    for(std::size_t index = answers.size() - 1; index > 0; --index) {
        answers[index]->set(answers[index - 1]->get());
    }
    answers[0]->set(result);
}

void clear_qalc_session_answers(Calculator &calc) {
    MathStructure undefined;
    undefined.setUndefined();
    for(KnownVariable *answer : ensure_qalc_session_answers(calc)) {
        answer->set(undefined);
    }
}

class CalculatorStateGuard {
public:
    explicit CalculatorStateGuard(Calculator &calculator)
        : calc(calculator),
          precision(calculator.getPrecision()),
          interval_arithmetic(calculator.usesIntervalArithmetic()),
          temperature_mode(calculator.getTemperatureCalculationMode()),
          decimal_point(calculator.getDecimalPoint()),
          comma(calculator.getComma()),
          binary_prefixes(calculator.usesBinaryPrefixes()),
          fixed_denominator(calculator.fixedDenominator()),
          custom_output_base(calculator.customOutputBase()),
          assumption_type(calculator.defaultAssumptions()->type()),
          assumption_sign(calculator.defaultAssumptions()->sign()) {}

    ~CalculatorStateGuard() noexcept {
        restore_decimal_mode();
        calc.setPrecision(precision);
        calc.useIntervalArithmetic(interval_arithmetic);
        calc.setTemperatureCalculationMode(temperature_mode);
        calc.useBinaryPrefixes(binary_prefixes);
        calc.setFixedDenominator(fixed_denominator);
        calc.setCustomOutputBase(custom_output_base);
        calc.defaultAssumptions()->setType(assumption_type);
        calc.defaultAssumptions()->setSign(assumption_sign);
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
    int binary_prefixes;
    long int fixed_denominator;
    Number custom_output_base;
    AssumptionType assumption_type;
    AssumptionSign assumption_sign;
};

class CalculatorFfiGuard {
public:
    explicit CalculatorFfiGuard(Calculator &calculator_ref)
        : prev_calculator(calculator) {
        calculator = &calculator_ref;
    }
    ~CalculatorFfiGuard() noexcept {
        calculator = prev_calculator;
    }
private:
    Calculator *prev_calculator;
};

class CalculatorMessageBlocker {
public:
    explicit CalculatorMessageBlocker(Calculator &calculator_ref)
        : calc(calculator_ref) {
        calc.beginTemporaryStopMessages();
    }
    ~CalculatorMessageBlocker() noexcept {
        calc.endTemporaryStopMessages();
    }

    CalculatorMessageBlocker(const CalculatorMessageBlocker&) = delete;
    CalculatorMessageBlocker& operator=(const CalculatorMessageBlocker&) = delete;

private:
    Calculator &calc;
};

std::string print_qalc_parsed_markup(
    const MathStructure &parsed_source,
    const EvaluationOptions &eo,
    const PrintOptions &po,
    bool latex
) {
    MathStructure parsed(parsed_source);

    PrintOptions parsed_options;
    parsed_options.preserve_format = true;
    parsed_options.show_ending_zeroes = false;
    parsed_options.exp_display = po.exp_display;
    parsed_options.lower_case_numbers = po.lower_case_numbers;
    parsed_options.base_display = po.base_display;
    parsed_options.twos_complement = po.twos_complement;
    parsed_options.rounding = po.rounding;
    parsed_options.hexadecimal_twos_complement = po.hexadecimal_twos_complement;
    parsed_options.base = eo.parse_options.base;
    parsed_options.allow_non_usable = true;
    parsed_options.abbreviate_names = true;
    parsed_options.digit_grouping = po.digit_grouping;
    parsed_options.use_unicode_signs = po.use_unicode_signs;
    parsed_options.multiplication_sign = po.multiplication_sign;
    parsed_options.division_sign = po.division_sign;
    parsed_options.short_multiplication = latex;
    parsed_options.excessive_parenthesis = false;
    parsed_options.improve_division_multipliers = false;
    parsed_options.restrict_to_parent_precision = false;
    parsed_options.spell_out_logical_operators = po.spell_out_logical_operators;
    parsed_options.interval_display = INTERVAL_DISPLAY_PLUSMINUS;
    parsed.format(parsed_options);
    std::string output = parsed.print(
        parsed_options,
        true,
        0,
        latex ? TAG_TYPE_LATEX : TAG_TYPE_HTML
    );
    if(parsed.isComparison() || parsed.isLogicalAnd() || parsed.isLogicalOr()) {
        if(latex) return "\\left(" + output + "\\right)";
        return "(" + output + ")";
    }
    return output;
}

std::string wrap_qalc_markup_parentheses(std::string text, bool latex) {
    if(latex) return "\\left(" + text + "\\right)";
    return "(" + text + ")";
}

std::string calculate_qalc_structured(
    Calculator &calc,
    const std::string &expression,
    int32_t timeout_ms,
    EvaluationOptions eo,
    PrintOptions &po,
    bool markup,
    bool latex,
    bool terse,
    bool update_session_answers,
    bool &is_approximate,
    std::string &parsed_output
) {
    const auto started_at = std::chrono::steady_clock::now();
    std::string calculation_expression = expression;
    bool had_to_expression = false;
    bool complex_angle_form = false;
    bool do_factors = false;
    bool do_partial_fractions = false;
    bool do_calendars = false;
    bool do_bases = false;
    std::string to_expression_text =
        calc.parseComments(calculation_expression, eo.parse_options);
    if(!to_expression_text.empty() && calculation_expression.empty()) {
        calculation_expression = "0";
    } else {
        std::string from_expression = calculation_expression;
        had_to_expression = calc.separateToExpression(
            from_expression,
            to_expression_text,
            eo,
            true
        );
        if(had_to_expression) {
            Number custom_base;
            int binary_prefixes = -1;
            const std::string unit_target = calc.parseToExpression(
                to_expression_text,
                eo,
                po,
                &custom_base,
                &binary_prefixes,
                &complex_angle_form,
                &do_factors,
                &do_partial_fractions,
                &do_calendars,
                &do_bases
            );
            if(!custom_base.isZero()) calc.setCustomOutputBase(custom_base);
            if(binary_prefixes >= 0) calc.useBinaryPrefixes(binary_prefixes);
            calculation_expression = from_expression;
            if(!unit_target.empty()) {
                calculation_expression += " to ";
                calculation_expression += unit_target;
            }
        }
        calculation_expression =
            calc.unlocalizeExpression(calculation_expression, eo.parse_options);
    }
    MathStructure result;
    MathStructure parsed;
    MathStructure to_expression;
    calc.calculate(
        &result,
        calculation_expression,
        timeout_ms,
        eo,
        &parsed,
        &to_expression
    );

    if(do_calendars && result.isDateTime()) {
        last_qalc_markup_output_complete = true;
        is_approximate = result.isApproximate();
        if(update_session_answers) rotate_qalc_session_answers(calc, result);
        return format_qalc_calendar_table(qalc_calendar_lines(result));
    }

    const bool converted = had_to_expression || !to_expression.isUndefined();
    if(!converted && eo.auto_post_conversion == POST_CONVERSION_OPTIMAL) {
        convert_unchanged_quantity_with_unit(parsed, result, eo);
    }

    MathStructure exact;
    exact.setUndefined();
    MathStructure prepend_result;
    prepend_result.setUndefined();
    const AutomaticFractionFormat auto_fraction = !terse
            && po.number_fraction_format == FRACTION_DECIMAL
        ? AUTOMATIC_FRACTION_AUTO
        : AUTOMATIC_FRACTION_OFF;
    const AutomaticApproximation auto_approximation =
        terse ? AUTOMATIC_APPROXIMATION_OFF : AUTOMATIC_APPROXIMATION_AUTO;
    if(!terse && !calc.aborted() && po.base == BASE_DECIMAL) {
        int exact_timeout_ms = 1000;
        if(timeout_ms > 0) {
            const auto elapsed = std::chrono::duration_cast<std::chrono::milliseconds>(
                std::chrono::steady_clock::now() - started_at
            ).count();
            exact_timeout_ms = timeout_ms - static_cast<int>(elapsed) - 10;
            exact_timeout_ms = std::min(exact_timeout_ms, 1000);
        }
        if(exact_timeout_ms > 0) {
            calculate_dual_exact(
                exact,
                &result,
                calculation_expression,
                &parsed,
                eo,
                auto_approximation,
                exact_timeout_ms,
                5
            );
            if(calc.aborted()) exact.setUndefined();
        }
    }

    if(do_factors) {
        if(result.isInteger() && !parsed.isNumber()) prepend_result.set(result);
        if((result.isNumber() || result.isVector())) {
            po.restrict_fraction_length = false;
            po.number_fraction_format = FRACTION_FRACTIONAL;
        }
        result.integerFactorize();
        exact.integerFactorize();
    } else if(do_partial_fractions) {
        result.expandPartialFractions(eo);
        exact.expandPartialFractions(eo);
    }

    if(!terse && markup) {
        parsed_output = print_qalc_parsed_markup(parsed, eo, po, latex);
    }

    if(do_bases) {
        std::string output;
        for(int base : {BASE_BINARY, BASE_OCTAL, BASE_DECIMAL, BASE_HEXADECIMAL}) {
            if(!output.empty()) output += " = ";
            po.base = base;
            output += calc.print(
                result,
                0,
                po,
                markup,
                0,
                latex ? TAG_TYPE_LATEX : TAG_TYPE_HTML
            );
        }
        is_approximate = result.isApproximate();
        if(update_session_answers) rotate_qalc_session_answers(calc, result);
        return output;
    }

    std::string result_output;
    std::vector<std::string> alternative_results;
    bool exact_comparison = false;
    print_dual(
        result,
        calculation_expression,
        parsed,
        exact,
        result_output,
        alternative_results,
        po,
        eo,
        auto_fraction,
        auto_approximation,
        complex_angle_form,
        &exact_comparison,
        true,
        markup,
        0,
        latex ? TAG_TYPE_LATEX : TAG_TYPE_HTML,
        -1,
        converted
    );

    if(!prepend_result.isUndefined()) {
        PrintOptions prepend_options = po;
        prepend_options.min_exp = 0;
        alternative_results.insert(
            alternative_results.begin(),
            calc.print(
                prepend_result,
                0,
                prepend_options,
                markup,
                0,
                latex ? TAG_TYPE_LATEX : TAG_TYPE_HTML
            )
        );
    }

    if(markup && !alternative_results.empty()) {
        const bool use_parentheses =
            result.isComparison() || result.isLogicalAnd() || result.isLogicalOr();
        const std::string approximate_result = result_output;
        result_output.clear();
        for(std::size_t index = 0; index < alternative_results.size(); ++index) {
            if(index > 0) result_output += " = ";
            result_output += use_parentheses
                ? wrap_qalc_markup_parentheses(alternative_results[index], latex)
                : alternative_results[index];
        }
        if(result.isApproximate() || is_approximate) {
            result_output += po.use_unicode_signs ? " ≈ " : " = approx. ";
        } else {
            result_output += " = ";
        }
        result_output += use_parentheses
            ? wrap_qalc_markup_parentheses(approximate_result, latex)
            : approximate_result;
    } else if(markup && (result.isComparison() || result.isLogicalAnd() || result.isLogicalOr())) {
        result_output = wrap_qalc_markup_parentheses(result_output, latex);
    }

    if(exact_comparison || !alternative_results.empty()) {
        is_approximate = false;
    } else if(result.isApproximate()) {
        is_approximate = true;
    }
    if(update_session_answers) rotate_qalc_session_answers(calc, result);
    return result_output;
}

} // namespace

std::unique_ptr<Calculator> new_calculator() {
    return std::make_unique<Calculator>();
}

bool qalc_enable_session_answers(Calculator &calc) {
    try {
        CalculatorFfiGuard ffi_guard(calc);
        ensure_qalc_session_answers(calc);
        return true;
    } catch (...) {
        return false;
    }
}

bool qalc_set_session_answer(Calculator &calc, rust::Str expression) {
    try {
        CalculatorFfiGuard ffi_guard(calc);
        const std::string source(expression.data(), expression.size());
        MathStructure result = calc.parse(source);
        rotate_qalc_session_answers(calc, result);
        return true;
    } catch (...) {
        return false;
    }
}

bool qalc_set_session_variable(
    Calculator &calc,
    rust::Str name,
    rust::Str expression
) {
    try {
        CalculatorFfiGuard ffi_guard(calc);
        CalculatorMessageBlocker message_blocker(calc);
        const std::string variable_name(name.data(), name.size());
        if(!calc.variableNameIsValid(variable_name)) return false;

        const std::string source(expression.data(), expression.size());
        calc.calculate(variable_name + ":=(" + source + ")");
        Variable *variable = calc.getActiveVariable(variable_name);
        return variable != nullptr && variable->isLocal();
    } catch (...) {
        return false;
    }
}

rust::String qalc_print_session_variable(Calculator &calc, rust::Str name) {
    try {
        CalculatorFfiGuard ffi_guard(calc);
        const std::string variable_name(name.data(), name.size());
        auto *variable = dynamic_cast<KnownVariable*>(calc.getActiveVariable(variable_name));
        if(variable == nullptr || !variable->isLocal()) return rust::String();

        const MathStructure &value = variable->get();
        bool is_approximate = value.isApproximate();
        PrintOptions po = qalc_print_options(true, &is_approximate);
        po.base = 10;
        return rust::String(calc.print(value, 0, po));
    } catch (const std::exception&) {
        throw;
    } catch (...) {
        throw std::runtime_error("unknown C++ exception while printing session variable");
    }
}

void qalc_clear_session_answers(Calculator &calc) {
    try {
        CalculatorFfiGuard ffi_guard(calc);
        clear_qalc_session_answers(calc);
    } catch (...) {
    }
}

bool qalc_delete_session_variable(Calculator &calc, rust::Str name) {
    try {
        CalculatorFfiGuard ffi_guard(calc);
        const std::string variable_name(name.data(), name.size());
        Variable *variable = calc.getActiveVariable(variable_name);
        return variable != nullptr && variable->isLocal() && variable->destroy();
    } catch (...) {
        return false;
    }
}

rust::String qalc_print_session_answer(
    Calculator &calc,
    int32_t output_base,
    bool unicode_enabled
) {
    try {
        CalculatorFfiGuard ffi_guard(calc);
        auto answers = ensure_qalc_session_answers(calc);
        const MathStructure &answer = answers[0]->get();
        if(answer.isUndefined()) return rust::String();

        bool is_approximate = answer.isApproximate();
        PrintOptions po = qalc_print_options(unicode_enabled, &is_approximate);
        po.base = output_base;
        const std::string rendering = calc.print(answer, 0, po);
        last_qalc_result_is_approximate = is_approximate;
        return rust::String(rendering);
    } catch (const std::exception&) {
        throw;
    } catch (...) {
        throw std::runtime_error("unknown C++ exception while printing session answer");
    }
}

bool load_exchange_rates(Calculator &calc) {
    try {
        CalculatorFfiGuard ffi_guard(calc);
        return calc.loadExchangeRates();
    } catch (...) {
        return false;
    }
}

bool load_global_definitions(Calculator &calc) {
    try {
        CalculatorFfiGuard ffi_guard(calc);
        return calc.loadGlobalDefinitions();
    } catch (...) {
        return false;
    }
}

bool load_global_definitions_selected(
    Calculator &calc,
    bool units,
    bool currencies,
    bool functions,
    bool variables,
    bool datasets
) {
    try {
        CalculatorFfiGuard ffi_guard(calc);
        bool loaded = true;
        if(units && !calc.loadGlobalPrefixes()) loaded = false;
        if(units && !calc.loadGlobalUnits()) loaded = false;
        else if(!units && currencies && !calc.loadGlobalCurrencies()) loaded = false;
        if(!units) {
            calc.beginTemporaryStopMessages();
            calc.getGraUnit();
            calc.getRadUnit();
            calc.getDegUnit();
            calc.endTemporaryStopMessages();
        }
        if(functions && !calc.loadGlobalFunctions()) loaded = false;
        if(datasets && !calc.loadGlobalDataSets()) loaded = false;
        if(variables && !calc.loadGlobalVariables()) loaded = false;
        return loaded;
    } catch (...) {
        return false;
    }
}

bool load_local_definitions(Calculator &calc) {
    try {
        CalculatorFfiGuard ffi_guard(calc);
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
        CalculatorFfiGuard ffi_guard(calc);
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
    int32_t timeout_ms,
    int32_t output_base,
    int32_t input_base,
    std::uint8_t assumption_mode,
    std::uint8_t mode_flags
) {
    const bool markup = (mode_flags & QALC_MODE_MARKUP) != 0;
    const bool latex = (mode_flags & QALC_MODE_LATEX) != 0;
    const bool terse = (mode_flags & QALC_MODE_TERSE) != 0;
    const bool capture_result = (mode_flags & QALC_MODE_CAPTURE_RESULT) != 0;
    const bool unicode_enabled = (mode_flags & QALC_MODE_UNICODE) != 0;
    last_qalc_result_is_approximate = false;
    last_qalc_markup_output_complete = false;
    last_qalc_messages_text.clear();
    last_qalc_parsed_expression.clear();
    last_qalc_messages_line_count = 0;
    last_qalc_message_had_error = false;
    try {
        CalculatorFfiGuard ffi_guard(calc);
        std::string expr_str(expr.data(), expr.size());
        CalculatorStateGuard state_guard(calc);

        switch(assumption_mode) {
            case 0:
                break;
            case 1:
                calc.defaultAssumptions()->setSign(ASSUMPTION_SIGN_POSITIVE);
                break;
            case 2:
                calc.defaultAssumptions()->setSign(ASSUMPTION_SIGN_UNKNOWN);
                break;
            default:
                throw std::invalid_argument("invalid qalc assumption mode");
        }

        bool is_approximate = false;
        PrintOptions po = qalc_print_options(unicode_enabled, &is_approximate);
        po.base = output_base;

        EvaluationOptions eo;
        eo.approximation = APPROXIMATION_TRY_EXACT;
        eo.sync_units = true;
        eo.structuring = STRUCTURING_SIMPLIFY;
        eo.parse_options.unknowns_enabled = false;
        eo.parse_options.read_precision = DONT_READ_PRECISION;
        if(input_base < 2 || input_base > 36) {
            throw std::invalid_argument("invalid qalc input base");
        }
        eo.parse_options.base = input_base;
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
        eo.local_currency_conversion = true;
        eo.interval_calculation = INTERVAL_CALCULATION_VARIANCE_FORMULA;
        eo.parse_options.twos_complement = false;
        eo.parse_options.hexadecimal_twos_complement = false;

        calc.useDecimalPoint(false);
        calc.setPrecision(10);
        calc.useIntervalArithmetic(true);
        calc.setTemperatureCalculationMode(TEMPERATURE_CALCULATION_HYBRID);

        std::string parsed_expression;
        std::string output;
        if(markup || capture_result) {
            output = calculate_qalc_structured(
                calc,
                expr_str,
                timeout_ms,
                eo,
                po,
                markup,
                latex,
                terse,
                capture_result,
                is_approximate,
                parsed_expression
            );
        } else {
            output = calc.calculateAndPrint(
                expr_str,
                timeout_ms,
                eo,
                po,
                AUTOMATIC_FRACTION_OFF,
                AUTOMATIC_APPROXIMATION_OFF
            );
        }
        if(markup && latex) {
            replace_all(output, " ≈ ", " \\approx ");
            replace_all(output, " = approx. ", " \\approx ");
        }
        last_qalc_parsed_expression = parsed_expression;
        last_qalc_result_is_approximate = is_approximate;
        capture_qalc_messages(calc);
        return rust::String(output);
    } catch (const std::exception&) {
        throw;
    } catch (...) {
        throw std::runtime_error("unknown C++ exception in qalc-compatible calculateAndPrint");
    }
}

bool qalc_last_result_is_approximate() {
    return last_qalc_result_is_approximate;
}

bool qalc_last_markup_output_is_complete() {
    return last_qalc_markup_output_complete;
}

rust::String qalc_last_messages() {
    return rust::String(last_qalc_messages_text);
}

rust::String qalc_last_parsed_expression() {
    return rust::String(last_qalc_parsed_expression);
}

std::size_t qalc_last_message_line_count() {
    return last_qalc_messages_line_count;
}

bool qalc_last_message_had_error() {
    return last_qalc_message_had_error;
}
