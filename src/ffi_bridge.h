#pragma once
#include "Calculator.h"
#include "rust/cxx.h"
#include <memory>
#include <string>

// Factory function to instantiate Calculator on the heap
std::unique_ptr<Calculator> new_calculator();

// Wrapper methods for loading definitions
bool load_exchange_rates(Calculator &calc);
bool load_global_definitions(Calculator &calc);
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
    int32_t timeout_ms
);
