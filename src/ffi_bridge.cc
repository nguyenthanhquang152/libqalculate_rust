#include "ffi_bridge.h"

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
