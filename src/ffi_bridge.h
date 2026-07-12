#pragma once
#include "Calculator.h"
#include "rust/cxx.h"
#include <cstddef>
#include <cstdint>
#include <memory>
#include <string>

// Factory function to instantiate Calculator on the heap
std::unique_ptr<Calculator> new_calculator();
bool qalc_enable_session_answers(Calculator &calc);
bool qalc_set_session_answer(Calculator &calc, rust::Str expression);
bool qalc_set_session_variable(
    Calculator &calc,
    rust::Str name,
    rust::Str expression
);
bool qalc_define_session_variable(
    Calculator &calc,
    rust::Str name,
    rust::Str expression
);
bool qalc_set_session_function(
    Calculator &calc,
    rust::Str name,
    rust::Str expression
);
rust::String qalc_print_session_variable(Calculator &calc, rust::Str name);
void qalc_clear_session_answers(Calculator &calc);
bool qalc_delete_session_variable(Calculator &calc, rust::Str name);
rust::String qalc_print_session_answer(
    Calculator &calc,
    int32_t output_base,
    bool unicode_enabled
);

// Wrapper methods for loading definitions
bool load_exchange_rates(Calculator &calc);
bool load_global_definitions(Calculator &calc);
bool load_global_definitions_selected(
    Calculator &calc,
    bool units,
    bool currencies,
    bool functions,
    bool variables,
    bool datasets
);
bool load_local_definitions(Calculator &calc);

// Wrapper method for calculation
rust::String calculate_and_print(
    Calculator &calc,
    rust::Str expr,
    int32_t timeout_ms
);

rust::String calculate_and_print_qalc(
    Calculator &calc,
    rust::Str expr,
    int32_t timeout_ms,
    int32_t output_base,
    int32_t input_base,
    std::uint8_t assumption_mode,
    std::uint8_t mode_flags
);

bool qalc_last_result_is_approximate();
bool qalc_last_markup_output_is_complete();
rust::String qalc_last_messages();
rust::String qalc_last_parsed_expression();
std::size_t qalc_last_message_line_count();
bool qalc_last_message_had_error();
