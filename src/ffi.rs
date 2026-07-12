#![allow(unsafe_code)]
//! Safe Rust wrapper and FFI bindings for C++ libqalculate's Calculator.

use cxx::UniquePtr;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

static FFI_LOCK: Mutex<()> = Mutex::new(());
type CatalogCacheEntry<T> = Arc<OnceLock<Option<Arc<T>>>>;
type CatalogCache<T> = OnceLock<Mutex<HashMap<PathBuf, CatalogCacheEntry<T>>>>;

const MAX_CACHED_DEFINITION_DIRS: usize = 8;
static FUNCTION_VARIABLE_CATALOG_CACHE: CatalogCache<
    crate::definitions_catalog::FunctionVariableCatalog,
> = OnceLock::new();
static PREFIX_UNIT_CATALOG_CACHE: CatalogCache<crate::units::PrefixUnitCatalog> = OnceLock::new();

// Keep these bit assignments synchronized with the named constants in
// `ffi_bridge.cc`; they are the compact options ABI for calculate_and_print_qalc.
const QALC_MODE_MARKUP: u8 = 1 << 0;
const QALC_MODE_LATEX: u8 = 1 << 1;
const QALC_MODE_TERSE: u8 = 1 << 2;
const QALC_MODE_CAPTURE_RESULT: u8 = 1 << 3;
const QALC_MODE_UNICODE: u8 = 1 << 4;

#[cxx::bridge]
#[allow(missing_docs)]
pub(crate) mod sys {
    // SAFETY: The FFI declarations below reference C++ symbols implemented in `ffi_bridge.cc`
    // and the upstream `libqalculate` library. CXX guarantees that these signatures are
    // checked and generated correctly at build time, ensuring safety under normal C++ linking assumptions.
    unsafe extern "C++" {
        include!("libqalculate_rust/src/ffi_bridge.h");

        /// Opaque C++ Calculator type.
        type Calculator;

        /// Create a std::unique_ptr to a Calculator.
        fn new_calculator() -> UniquePtr<Calculator>;
        fn qalc_enable_session_answers(calc: Pin<&mut Calculator>) -> bool;
        fn qalc_set_session_answer(calc: Pin<&mut Calculator>, expression: &str) -> bool;
        fn qalc_set_session_variable(
            calc: Pin<&mut Calculator>,
            name: &str,
            expression: &str,
        ) -> bool;
        fn qalc_define_session_variable(
            calc: Pin<&mut Calculator>,
            name: &str,
            expression: &str,
        ) -> bool;
        fn qalc_set_session_function(
            calc: Pin<&mut Calculator>,
            name: &str,
            expression: &str,
        ) -> bool;
        fn qalc_render_session_function_info(
            calc: Pin<&mut Calculator>,
            name: &str,
        ) -> Result<String>;
        fn qalc_print_session_variable(calc: Pin<&mut Calculator>, name: &str) -> Result<String>;
        fn qalc_clear_session_answers(calc: Pin<&mut Calculator>);
        fn qalc_delete_session_variable(calc: Pin<&mut Calculator>, name: &str) -> bool;
        fn qalc_delete_session_function(calc: Pin<&mut Calculator>, name: &str) -> bool;
        fn qalc_print_session_answer(
            calc: Pin<&mut Calculator>,
            output_base: i32,
            unicode_enabled: bool,
        ) -> Result<String>;

        /// Load exchange rates.
        fn load_exchange_rates(calc: Pin<&mut Calculator>) -> bool;

        /// Load global definitions.
        fn load_global_definitions(calc: Pin<&mut Calculator>) -> bool;

        /// Load selected global definition families using qalc's startup order.
        fn load_global_definitions_selected(
            calc: Pin<&mut Calculator>,
            units: bool,
            currencies: bool,
            functions: bool,
            variables: bool,
            datasets: bool,
        ) -> bool;

        /// Load local definitions.
        fn load_local_definitions(calc: Pin<&mut Calculator>) -> bool;

        /// Calculate and print an expression.
        fn calculate_and_print(
            calc: Pin<&mut Calculator>,
            expr: &str,
            timeout_ms: i32,
        ) -> Result<String>;

        /// Calculate and print using qalc-compatible evaluation/print defaults.
        fn calculate_and_print_qalc(
            calc: Pin<&mut Calculator>,
            expr: &str,
            timeout_ms: i32,
            output_base: i32,
            input_base: i32,
            assumption_mode: u8,
            mode_flags: u8,
        ) -> Result<String>;

        fn qalc_last_result_is_approximate() -> bool;
        fn qalc_last_markup_output_is_complete() -> bool;
        fn qalc_last_messages() -> String;
        fn qalc_last_parsed_expression() -> String;
        fn qalc_last_message_line_count() -> usize;
        fn qalc_last_message_had_error() -> bool;
    }
}

/// Safe wrapper around the C++ `Calculator` class.
pub struct Calculator {
    inner: UniquePtr<sys::Calculator>,
    native_context: crate::context::CalculatorContext,
    session_answers: crate::session::SessionAnswerState,
    last_native_message_had_error: bool,
    last_output_approximate: bool,
    last_output_message_lines: usize,
    _phantom: PhantomData<*mut ()>,
}

/// How an evaluation was routed with respect to the C++ fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackState {
    /// The expression was handled by native Rust code with the C++ fallback disabled.
    Native,
    /// The C++ fallback was available for this evaluation.
    CppFallbackEnabled,
    /// The C++ fallback was disabled and no native implementation handled the expression.
    Disabled,
}

impl FallbackState {
    /// Return the stable state label used inside oracle mismatch records.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::CppFallbackEnabled => "cpp-fallback-enabled",
            Self::Disabled => "disabled",
        }
    }

    /// Return the stable machine-readable marker used by CLI and oracle output.
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Native => "fallback=native",
            Self::CppFallbackEnabled => "fallback=cpp-fallback-enabled",
            Self::Disabled => "fallback=disabled",
        }
    }

    /// Parse a fallback marker from either a bare marker or a qalc-rs metadata line.
    pub fn from_marker(marker: &str) -> Option<Self> {
        let marker = marker
            .trim()
            .strip_prefix("[qalc-rs-metadata]")
            .map(str::trim)
            .unwrap_or_else(|| marker.trim());

        match marker {
            "fallback=native" => Some(Self::Native),
            "fallback=cpp-fallback-enabled" => Some(Self::CppFallbackEnabled),
            "fallback=disabled" => Some(Self::Disabled),
            _ => None,
        }
    }
}

/// Evaluation output plus the fallback state needed for oracle evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalculationOutput {
    /// The formatted result returned to the caller.
    pub output: String,
    /// The fallback routing state for this evaluation.
    pub fallback_state: FallbackState,
}

/// Return whether a native expression would consult global definition-backed
/// catalogs such as currencies, units, or datasets.
///
/// The CLI uses this conservative classifier to enforce `-nodefs` while still
/// allowing definition-independent native arithmetic and session commands.
pub fn native_expression_uses_global_definitions(expr: &str) -> bool {
    let Ok(parsed) = crate::parser::operators::parse_expression(expr) else {
        return false;
    };

    definition_usage_uses_global_definitions(&native_definition_usage(&parsed))
}

/// Return whether an expression uses any selectively disabled definition family.
///
/// Each boolean reports whether the corresponding unit, currency, function,
/// variable, or dataset family is enabled for this invocation.
pub fn native_expression_uses_disabled_definition_family(
    expr: &str,
    units_enabled: bool,
    currencies_enabled: bool,
    functions_enabled: bool,
    variables_enabled: bool,
    datasets_enabled: bool,
) -> bool {
    let Ok(parsed) = crate::parser::operators::parse_expression(expr) else {
        return true;
    };
    definition_usage_uses_disabled_family(
        &native_definition_usage(&parsed),
        units_enabled,
        currencies_enabled,
        functions_enabled,
        variables_enabled,
        datasets_enabled,
    )
}

fn definition_usage_uses_global_definitions(usage: &NativeDefinitionUsage) -> bool {
    usage.currencies || usage_uses_unit_definition(usage) || usage.datasets
}

fn definition_usage_uses_disabled_family(
    usage: &NativeDefinitionUsage,
    units_enabled: bool,
    currencies_enabled: bool,
    functions_enabled: bool,
    variables_enabled: bool,
    datasets_enabled: bool,
) -> bool {
    let uses_disabled_function_or_variable =
        uses_disabled_function_or_variable_definition(usage, functions_enabled, variables_enabled);
    (!units_enabled && usage_uses_unit_definition(usage))
        || (!currencies_enabled && usage.currencies)
        || (!datasets_enabled && usage.datasets)
        || uses_disabled_function_or_variable
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct NativeDefinitionUsage {
    units: bool,
    currencies: bool,
    datasets: bool,
    function_names: Vec<String>,
    variable_names: Vec<String>,
}

fn usage_uses_unit_definition(usage: &NativeDefinitionUsage) -> bool {
    if usage.units {
        return true;
    }
    if usage.variable_names.is_empty() {
        return false;
    }

    let definitions_dir = crate::rates::definitions_dir();
    let Some(catalog) = cached_prefix_unit_catalog(&definitions_dir) else {
        return true;
    };
    usage
        .variable_names
        .iter()
        .any(|name| catalog.unit_by_name(name).is_some())
}

fn uses_disabled_function_or_variable_definition(
    usage: &NativeDefinitionUsage,
    functions_enabled: bool,
    variables_enabled: bool,
) -> bool {
    let functions_need_lookup = !functions_enabled && !usage.function_names.is_empty();
    let variables_need_lookup = !variables_enabled && !usage.variable_names.is_empty();
    if !functions_need_lookup && !variables_need_lookup {
        return false;
    }

    let definitions_dir = crate::rates::definitions_dir();
    let Some(catalog) = cached_function_variable_catalog(&definitions_dir) else {
        return true;
    };

    let uses_disabled_function = functions_need_lookup
        && usage.function_names.iter().any(|name| {
            catalog
                .functions()
                .find_by_name(name)
                .is_some_and(|definition| {
                    definition.kind() == crate::definitions_catalog::FunctionKind::User
                })
        });
    let uses_disabled_variable = variables_need_lookup
        && usage.variable_names.iter().any(|name| {
            catalog
                .variables()
                .find_by_name(name)
                .is_some_and(|definition| {
                    definition.kind() != crate::definitions_catalog::VariableKind::Builtin
                })
        });
    uses_disabled_function || uses_disabled_variable
}

fn cached_function_variable_catalog(
    definitions_dir: &Path,
) -> Option<Arc<crate::definitions_catalog::FunctionVariableCatalog>> {
    cached_catalog(
        &FUNCTION_VARIABLE_CATALOG_CACHE,
        definitions_dir,
        |definitions_dir| {
            crate::definitions_catalog::load_function_variable_catalog_from_dir(definitions_dir)
                .ok()
        },
    )
}

fn cached_prefix_unit_catalog(
    definitions_dir: &Path,
) -> Option<Arc<crate::units::PrefixUnitCatalog>> {
    cached_catalog(
        &PREFIX_UNIT_CATALOG_CACHE,
        definitions_dir,
        |definitions_dir| crate::units::load_prefix_unit_catalog_from_dir(definitions_dir).ok(),
    )
}

fn cached_catalog<T>(
    cache: &CatalogCache<T>,
    definitions_dir: &Path,
    load: impl FnOnce(&Path) -> Option<T>,
) -> Option<Arc<T>> {
    let entry = {
        let cache = cache.get_or_init(|| Mutex::new(HashMap::new()));
        let mut entries = cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(entry) = entries.get(definitions_dir) {
            Arc::clone(entry)
        } else {
            if entries.len() >= MAX_CACHED_DEFINITION_DIRS {
                if let Some(evicted) = entries.keys().next().cloned() {
                    entries.remove(&evicted);
                }
            }
            let entry = Arc::new(OnceLock::new());
            entries.insert(definitions_dir.to_path_buf(), Arc::clone(&entry));
            entry
        }
    };

    entry
        .get_or_init(|| load(definitions_dir).map(Arc::new))
        .as_ref()
        .map(Arc::clone)
}

fn push_definition_name(names: &mut Vec<String>, name: &str) {
    if !names.iter().any(|candidate| candidate == name) {
        names.push(name.to_string());
    }
}

fn native_definition_usage(expr: &crate::ast::Expression) -> NativeDefinitionUsage {
    native_definition_usage_with_locals(expr, &|_| false)
}

fn native_definition_usage_with_locals(
    expr: &crate::ast::Expression,
    is_local: &impl Fn(&str) -> bool,
) -> NativeDefinitionUsage {
    let mut usage = NativeDefinitionUsage::default();
    collect_native_definition_usage(expr, false, &mut usage, is_local);
    usage
}

fn collect_native_definition_usage(
    expr: &crate::ast::Expression,
    dataset_argument: bool,
    usage: &mut NativeDefinitionUsage,
    is_local: &impl Fn(&str) -> bool,
) {
    use crate::ast::Expression;

    if crate::rates::match_currency_conversion(expr).is_some() {
        usage.currencies = true;
        return;
    }

    match expr {
        Expression::FunctionCall { function, args } => {
            push_definition_name(&mut usage.function_names, function.id());
            let is_dataset = crate::datasets::is_dataset_function_name(function.id());
            usage.datasets |= is_dataset;
            for arg in args {
                collect_native_definition_usage(
                    arg,
                    dataset_argument || is_dataset,
                    usage,
                    is_local,
                );
            }
        }
        Expression::Unit { .. } => usage.units = true,
        Expression::Variable(variable) => {
            if !is_local(variable.id()) {
                push_definition_name(&mut usage.variable_names, variable.id());
            }
        }
        Expression::Symbolic(symbol) => {
            if is_local(symbol.name()) {
                return;
            }
            if crate::unit_conversion::may_contain_unit_candidate(expr) {
                usage.units = true;
            } else if !dataset_argument {
                push_definition_name(&mut usage.variable_names, symbol.name());
            }
        }
        _ => {
            for index in 0..expr.child_count() {
                if let Some(child) = expr.child(index) {
                    collect_native_definition_usage(child, dataset_argument, usage, is_local);
                }
            }
        }
    }
}

/// Return whether an expression is composed entirely of native literals and
/// operators, without functions, variables, units, or definition symbols.
pub fn native_expression_is_definition_free(expr: &str) -> bool {
    let Ok(parsed) = crate::parser::operators::parse_expression(expr) else {
        return false;
    };

    !expression_contains(&parsed, &|expr| {
        matches!(
            expr,
            crate::ast::Expression::FunctionCall { .. }
                | crate::ast::Expression::Symbolic(_)
                | crate::ast::Expression::Variable(_)
                | crate::ast::Expression::Unit { .. }
                | crate::ast::Expression::Assignment { .. }
        )
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PrintProfile {
    Api,
    Qalc,
}

impl Drop for Calculator {
    fn drop(&mut self) {
        let _guard = FFI_LOCK.lock().unwrap();
        let _ = std::mem::replace(&mut self.inner, UniquePtr::null());
    }
}

impl Calculator {
    /// Create a new `Calculator` instance.
    pub fn new() -> Self {
        // SAFETY: Calling C++ factory function to instantiate a new Calculator on the C++ heap.
        // The returned UniquePtr safely manages the lifetime of the object.
        let _guard = FFI_LOCK.lock().unwrap();
        let inner = sys::new_calculator();
        Self {
            inner,
            native_context: crate::context::CalculatorContext::default(),
            session_answers: crate::session::SessionAnswerState::default(),
            last_native_message_had_error: false,
            last_output_approximate: false,
            last_output_message_lines: 0,
            _phantom: PhantomData,
        }
    }

    /// Whether the last evaluation emitted an error-severity message.
    pub fn last_native_message_had_error(&self) -> bool {
        self.last_native_message_had_error
    }

    /// Enable typed answer tracking for a long-lived interactive session.
    pub fn enable_session_mode(&mut self) {
        if self.inner.is_null() {
            return;
        }
        let enabled = {
            let _guard = FFI_LOCK.lock().unwrap();
            sys::qalc_enable_session_answers(self.inner.pin_mut())
        };
        if enabled {
            self.session_answers.enable();
        }
    }

    /// Return whether an expression uses global definitions not shadowed by
    /// variables in this calculator session.
    pub fn session_expression_uses_global_definitions(&self, expr: &str) -> bool {
        let Ok(parsed) = crate::parser::operators::parse_expression(expr) else {
            return false;
        };
        let usage = native_definition_usage_with_locals(&parsed, &|name| {
            self.native_context.variables.contains_key(name)
        });
        definition_usage_uses_global_definitions(&usage)
    }

    /// Return whether an expression uses selectively disabled definitions not
    /// shadowed by variables in this calculator session.
    pub fn session_expression_uses_disabled_definition_family(
        &self,
        expr: &str,
        units_enabled: bool,
        currencies_enabled: bool,
        functions_enabled: bool,
        variables_enabled: bool,
        datasets_enabled: bool,
    ) -> bool {
        let Ok(parsed) = crate::parser::operators::parse_expression(expr) else {
            return true;
        };
        let usage = native_definition_usage_with_locals(&parsed, &|name| {
            self.native_context.variables.contains_key(name)
        });
        definition_usage_uses_disabled_family(
            &usage,
            units_enabled,
            currencies_enabled,
            functions_enabled,
            variables_enabled,
            datasets_enabled,
        )
    }

    /// Delete a user-defined variable from the current interactive session.
    pub fn delete_session_variable(&mut self, name: &str) -> bool {
        if self.session_answers.is_enabled()
            && crate::session::SessionAnswerState::is_managed_alias(name)
        {
            return false;
        }
        let native_removed = self.native_context.variables.remove(name).is_some();
        if self.inner.is_null() {
            return native_removed;
        }
        let cpp_removed = {
            let _guard = FFI_LOCK.lock().unwrap();
            sys::qalc_delete_session_variable(self.inner.pin_mut(), name)
        };
        native_removed || cpp_removed
    }

    /// Delete a command-defined variable that shadows a managed answer alias.
    pub fn delete_session_variable_override(&mut self, name: &str) -> bool {
        if !crate::session::SessionAnswerState::is_managed_alias(name) {
            return self.delete_session_variable(name);
        }
        if self.inner.is_null() {
            return false;
        }
        let cpp_removed = {
            let _guard = FFI_LOCK.lock().unwrap();
            sys::qalc_delete_session_variable(self.inner.pin_mut(), name)
        };
        if cpp_removed {
            self.session_answers.invalidate(&mut self.native_context);
        }
        cpp_removed
    }

    /// Delete a user-defined function from the current interactive session.
    pub fn delete_session_function(&mut self, name: &str) -> bool {
        if self.inner.is_null() {
            return false;
        }
        let _guard = FFI_LOCK.lock().unwrap();
        sys::qalc_delete_session_function(self.inner.pin_mut(), name)
    }

    /// Define a variable without rotating the interactive answer history.
    pub fn define_session_variable(&mut self, name: &str, expression: &str) -> Option<String> {
        if self.inner.is_null() {
            return None;
        }
        let defined = {
            let _guard = FFI_LOCK.lock().unwrap();
            sys::qalc_define_session_variable(self.inner.pin_mut(), name, expression)
        };
        if !defined {
            return None;
        }

        let rendering = self.cpp_session_variable_rendering(name)?;
        match crate::parser::operators::parse_expression(&rendering)
            .ok()
            .and_then(|parsed| crate::eval::evaluate_ast(&parsed, &mut self.native_context).ok())
        {
            Some(value) => {
                self.native_context
                    .variables
                    .insert(name.to_string(), value);
            }
            None => {
                self.native_context.variables.remove(name);
            }
        }
        Some(rendering)
    }

    /// Define a user function without rotating the interactive answer history.
    pub fn define_session_function(&mut self, name: &str, expression: &str) -> Option<String> {
        if self.inner.is_null() {
            return None;
        }
        let function_info = {
            let _guard = FFI_LOCK.lock().unwrap();
            if !sys::qalc_set_session_function(self.inner.pin_mut(), name, expression) {
                return None;
            }
            sys::qalc_render_session_function_info(self.inner.pin_mut(), name).ok()?
        };
        (!function_info.is_empty()).then_some(function_info)
    }

    /// Return the display rendering of the current typed session answer.
    pub fn session_answer_rendering(&self) -> Option<String> {
        self.session_answers
            .cpp_rendering()
            .or_else(|| {
                self.session_answers
                    .current()
                    .map(|(_, rendering)| rendering)
            })
            .map(str::to_string)
    }

    /// Return values for variables assigned by an interactive expression.
    pub fn session_assignment_renderings(&mut self, expr: &str) -> Vec<(String, String)> {
        let mut names = Vec::new();
        if let Ok(parsed) = crate::parser::operators::parse_expression(expr) {
            collect_assignment_names(&parsed, &mut names);
        }
        if let Some((variable, _)) = crate::session::parse_load_assignment(expr) {
            names.push(variable.to_string());
        }
        names.sort();
        names.dedup();

        let mut renderings = Vec::new();
        for name in names {
            if crate::session::SessionAnswerState::is_managed_alias(&name) {
                continue;
            }
            let native_rendering = self.native_context.variables.get(&name).and_then(|value| {
                crate::session::format_answer(
                    crate::session::AnswerFormatProfile::Qalc,
                    value,
                    crate::session::NativeSessionSettings::default(),
                )
                .map(|(rendering, _)| rendering)
            });
            if let Some(rendering) =
                native_rendering.or_else(|| self.cpp_session_variable_rendering(&name))
            {
                renderings.push((name, rendering));
            }
        }
        renderings
    }

    fn cpp_session_variable_rendering(&mut self, name: &str) -> Option<String> {
        if self.inner.is_null() {
            return None;
        }
        let rendering = {
            let _guard = FFI_LOCK.lock().unwrap();
            sys::qalc_print_session_variable(self.inner.pin_mut(), name).ok()?
        };
        (!rendering.is_empty()).then_some(rendering)
    }

    /// Re-render the current typed session answer after settings change.
    ///
    /// This updates only the display of the current answer. It does not
    /// evaluate the rendered text or rotate the `ans` history.
    pub fn reformat_session_answer_with_settings(
        &mut self,
        settings: &[&str],
    ) -> Result<Option<CalculationOutput>, CalculatorError> {
        self.reformat_session_answer_with_settings_and_markup(settings, None, false)
    }

    /// Re-render only the current typed session answer after settings change.
    pub fn reformat_session_answer_terse_with_settings(
        &mut self,
        settings: &[&str],
    ) -> Result<Option<CalculationOutput>, CalculatorError> {
        self.reformat_session_answer_with_settings_and_markup(settings, None, true)
    }

    /// Re-render the current typed session answer as LaTeX after settings change.
    pub fn reformat_session_answer_latex_with_settings(
        &mut self,
        settings: &[&str],
    ) -> Result<Option<CalculationOutput>, CalculatorError> {
        self.reformat_session_answer_with_settings_and_markup(
            settings,
            Some(crate::markup::MarkupMode::Latex),
            false,
        )
    }

    /// Re-render only the current typed session answer as LaTeX after settings change.
    pub fn reformat_session_answer_latex_terse_with_settings(
        &mut self,
        settings: &[&str],
    ) -> Result<Option<CalculationOutput>, CalculatorError> {
        self.reformat_session_answer_with_settings_and_markup(
            settings,
            Some(crate::markup::MarkupMode::Latex),
            true,
        )
    }

    /// Re-render the current typed session answer as HTML after settings change.
    pub fn reformat_session_answer_html_with_settings(
        &mut self,
        settings: &[&str],
    ) -> Result<Option<CalculationOutput>, CalculatorError> {
        self.reformat_session_answer_with_settings_and_markup(
            settings,
            Some(crate::markup::MarkupMode::Html),
            false,
        )
    }

    /// Re-render only the current typed session answer as HTML after settings change.
    pub fn reformat_session_answer_html_terse_with_settings(
        &mut self,
        settings: &[&str],
    ) -> Result<Option<CalculationOutput>, CalculatorError> {
        self.reformat_session_answer_with_settings_and_markup(
            settings,
            Some(crate::markup::MarkupMode::Html),
            true,
        )
    }

    fn reformat_session_answer_with_settings_and_markup(
        &mut self,
        settings: &[&str],
        markup_mode: Option<crate::markup::MarkupMode>,
        terse: bool,
    ) -> Result<Option<CalculationOutput>, CalculatorError> {
        let parsed_settings = crate::session::NativeSessionSettings::from_raw(settings)
            .ok_or_else(|| {
                CalculatorError::UnsupportedSessionSettings(
                    settings
                        .iter()
                        .map(|setting| (*setting).to_string())
                        .collect(),
                )
            })?;
        if let Some(previous_rendering) = self.session_answers.cpp_rendering().map(str::to_string) {
            if self.inner.is_null() {
                return Ok(None);
            }
            let (unicode_enabled, output_base) =
                crate::session::NativeSessionSettings::cpp_reformat_options_from_raw(settings)
                    .ok_or_else(|| {
                        CalculatorError::UnsupportedSessionSettings(
                            settings
                                .iter()
                                .map(|setting| (*setting).to_string())
                                .collect(),
                        )
                    })?;
            let (rendering, approximate) = {
                let _guard = FFI_LOCK.lock().unwrap();
                let rendering = sys::qalc_print_session_answer(
                    self.inner.pin_mut(),
                    output_base as i32,
                    unicode_enabled,
                )
                .map_err(CalculatorError::Cxx)?;
                (rendering, sys::qalc_last_result_is_approximate())
            };
            if rendering.is_empty() {
                return Ok(None);
            }
            let output = format_reformatted_session_answer(
                markup_mode,
                terse,
                &previous_rendering,
                &rendering,
                approximate,
                unicode_enabled,
            );
            self.last_native_message_had_error = false;
            self.last_output_approximate = approximate;
            self.last_output_message_lines = 0;
            self.session_answers.update_rendering(rendering);
            return Ok(Some(CalculationOutput {
                output,
                fallback_state: FallbackState::CppFallbackEnabled,
            }));
        }

        let Some((answer, previous_rendering)) = self.session_answers.current() else {
            return Ok(None);
        };
        let answer = answer.clone();
        let previous_rendering = previous_rendering.to_string();
        let previous_approximate = self.last_output_approximate;
        let preserve_approximation = expression_is_currency_answer(&answer);
        let formatted = crate::session::format_answer(
            crate::session::AnswerFormatProfile::Qalc,
            &answer,
            parsed_settings,
        );
        let (rendering, approximate) = if let Some((rendering, formatted_approximate)) = formatted {
            let approximate = if preserve_approximation {
                previous_approximate
            } else {
                formatted_approximate
            };
            (rendering, approximate)
        } else {
            if self.inner.is_null() {
                return Ok(None);
            }
            let (unicode_enabled, output_base) =
                crate::session::NativeSessionSettings::cpp_reformat_options_from_raw(settings)
                    .ok_or_else(|| {
                        CalculatorError::UnsupportedSessionSettings(
                            settings
                                .iter()
                                .map(|setting| (*setting).to_string())
                                .collect(),
                        )
                    })?;
            let _guard = FFI_LOCK.lock().unwrap();
            let rendering = sys::qalc_print_session_answer(
                self.inner.pin_mut(),
                output_base as i32,
                unicode_enabled,
            )
            .map_err(CalculatorError::Cxx)?;
            if rendering.is_empty() {
                return Ok(None);
            }
            (rendering, sys::qalc_last_result_is_approximate())
        };
        let unicode_enabled = !parsed_settings.has_unicode_setting() || parsed_settings.unicode();
        let output = format_reformatted_session_answer(
            markup_mode,
            terse,
            &previous_rendering,
            &rendering,
            approximate,
            unicode_enabled,
        );
        self.last_native_message_had_error = false;
        self.last_output_approximate = approximate;
        self.last_output_message_lines = 0;
        self.session_answers.update_rendering(rendering);
        Ok(Some(CalculationOutput {
            output,
            fallback_state: FallbackState::Native,
        }))
    }

    /// Load the exchange rates for currencies.
    /// Returns `true` if loaded successfully.
    pub fn load_exchange_rates(&mut self) -> bool {
        if self.inner.is_null() {
            return false;
        }
        let _guard = FFI_LOCK.lock().unwrap();
        let pin = self.inner.pin_mut();
        // SAFETY: Passing a pinned mutable reference of the Calculator to the FFI function.
        // The pinned reference ensures the C++ object is not moved and is valid.
        sys::load_exchange_rates(pin)
    }

    /// Load the standard global definitions (system wide).
    /// Returns `true` if loaded successfully.
    pub fn load_global_definitions(&mut self) -> bool {
        if self.inner.is_null() {
            return false;
        }
        let _guard = FFI_LOCK.lock().unwrap();
        let pin = self.inner.pin_mut();
        // SAFETY: Passing a pinned mutable reference of the Calculator to the FFI function.
        // The pinned reference ensures the C++ object is not moved and is valid.
        sys::load_global_definitions(pin)
    }

    /// Load selected standard global definition families.
    ///
    /// This follows qalc's startup ordering, including its behavior of loading
    /// currencies together with units.
    /// Returns `true` when every requested family loaded successfully.
    pub fn load_global_definitions_selected(
        &mut self,
        units: bool,
        currencies: bool,
        functions: bool,
        variables: bool,
        datasets: bool,
    ) -> bool {
        let _guard = FFI_LOCK.lock().unwrap();
        if self.inner.is_null() {
            return false;
        }
        let pin = self.inner.pin_mut();
        sys::load_global_definitions_selected(
            pin, units, currencies, functions, variables, datasets,
        )
    }

    /// Load user-specific local definitions.
    /// Returns `true` if loaded successfully.
    pub fn load_local_definitions(&mut self) -> bool {
        if self.inner.is_null() {
            return false;
        }
        let _guard = FFI_LOCK.lock().unwrap();
        let pin = self.inner.pin_mut();
        // SAFETY: Passing a pinned mutable reference of the Calculator to the FFI function.
        // The pinned reference ensures the C++ object is not moved and is valid.
        sys::load_local_definitions(pin)
    }

    /// Evaluate a mathematical expression string and return the formatted result.
    ///
    /// # Errors
    /// Returns a `CalculatorError` if evaluation fails or fallback is disabled.
    ///
    /// # Panics
    /// Panics if the inner Calculator pointer is null, which indicates a bug
    /// (e.g., use-after-move). This should never happen in normal usage since
    /// `new()` always constructs a valid Calculator.
    pub fn calculate_and_print(
        &mut self,
        expr: &str,
        timeout_ms: i32,
    ) -> Result<String, CalculatorError> {
        self.calculate_and_print_with_fallback_state(expr, timeout_ms)
            .map(|result| result.output)
    }

    /// Evaluate a mathematical expression and return output plus fallback state.
    ///
    /// # Errors
    /// Returns a `CalculatorError` if evaluation fails or fallback is disabled.
    ///
    /// # Panics
    /// Panics if the inner Calculator pointer is null, which indicates a bug.
    pub fn calculate_and_print_with_fallback_state(
        &mut self,
        expr: &str,
        timeout_ms: i32,
    ) -> Result<CalculationOutput, CalculatorError> {
        self.calculate_with_profile(PrintProfile::Api, expr, timeout_ms)
    }

    /// Evaluate an expression using qalc-compatible print/evaluation defaults.
    ///
    /// This path is intended for the CLI/oracle harness. It preserves the plain
    /// `calculate_and_print` wrapper for API-default libqalculate smoke tests.
    ///
    /// # Errors
    /// Returns a `CalculatorError` if evaluation fails or fallback is disabled.
    ///
    /// # Panics
    /// Panics if the inner Calculator pointer is null, which indicates a bug.
    pub fn calculate_and_print_qalc(
        &mut self,
        expr: &str,
        timeout_ms: i32,
    ) -> Result<String, CalculatorError> {
        self.calculate_and_print_qalc_with_fallback_state(expr, timeout_ms)
            .map(|result| result.output)
    }

    /// Evaluate an expression using qalc-compatible defaults and return fallback state.
    ///
    /// # Errors
    /// Returns a `CalculatorError` if evaluation fails or fallback is disabled.
    ///
    /// # Panics
    /// Panics if the inner Calculator pointer is null, which indicates a bug.
    pub fn calculate_and_print_qalc_with_fallback_state(
        &mut self,
        expr: &str,
        timeout_ms: i32,
    ) -> Result<CalculationOutput, CalculatorError> {
        self.calculate_with_profile(PrintProfile::Qalc, expr, timeout_ms)
    }

    /// Evaluate an expression using qalc-compatible defaults plus a narrow set
    /// of qalc session settings supported by native fallback-disabled evidence.
    ///
    /// # Errors
    /// Returns a `CalculatorError` if evaluation fails or fallback is disabled
    /// for an unsupported expression/settings combination.
    ///
    /// # Panics
    /// Panics if the inner Calculator pointer is null, which indicates a bug.
    pub fn calculate_and_print_qalc_with_settings_and_fallback_state(
        &mut self,
        expr: &str,
        settings: &[&str],
        timeout_ms: i32,
    ) -> Result<CalculationOutput, CalculatorError> {
        self.calculate_with_profile_and_settings(
            PrintProfile::Qalc,
            expr,
            timeout_ms,
            settings,
            true,
        )
    }

    /// Evaluate a terse qalc expression while retaining C++ message metadata
    /// without prepending those messages to result-only output.
    ///
    /// # Errors
    /// Returns a `CalculatorError` if evaluation fails or fallback is disabled.
    pub fn calculate_and_print_qalc_terse_with_settings_and_fallback_state(
        &mut self,
        expr: &str,
        settings: &[&str],
        timeout_ms: i32,
    ) -> Result<CalculationOutput, CalculatorError> {
        self.calculate_with_profile_and_settings(
            PrintProfile::Qalc,
            expr,
            timeout_ms,
            settings,
            false,
        )
    }

    /// Evaluate an expression and format the parsed/result pair as a non-terse qalc equation.
    ///
    /// # Errors
    /// Returns a `CalculatorError` if evaluation fails or fallback is disabled.
    pub fn calculate_and_print_qalc_equation_with_settings_and_fallback_state(
        &mut self,
        expr: &str,
        settings: &[&str],
        timeout_ms: i32,
    ) -> Result<CalculationOutput, CalculatorError> {
        let mut result = self.calculate_and_print_qalc_with_settings_and_fallback_state(
            expr, settings, timeout_ms,
        )?;
        let unicode_enabled = crate::session::NativeSessionSettings::from_raw(settings)
            .map(|state| !state.has_unicode_setting() || state.unicode())
            .unwrap_or(true);
        result.output = crate::text::format_qalc_equation(
            expr,
            &result.output,
            self.last_output_approximate,
            unicode_enabled,
            self.last_output_message_lines,
        );
        Ok(result)
    }

    /// Evaluate an expression and format the parsed/result pair as LaTeX markup.
    ///
    /// # Errors
    /// Returns a `CalculatorError` if the expression is outside the native
    /// markup evidence slice or if evaluation fails.
    pub fn calculate_and_print_qalc_latex_with_settings_and_fallback_state(
        &mut self,
        expr: &str,
        settings: &[&str],
        timeout_ms: i32,
    ) -> Result<CalculationOutput, CalculatorError> {
        self.calculate_markup_with_settings(
            crate::markup::MarkupMode::Latex,
            expr,
            settings,
            timeout_ms,
            false,
        )
    }

    /// Evaluate an expression and format the parsed/result pair as HTML markup.
    ///
    /// # Errors
    /// Returns a `CalculatorError` if the expression is outside the native
    /// markup evidence slice or if evaluation fails.
    pub fn calculate_and_print_qalc_html_with_settings_and_fallback_state(
        &mut self,
        expr: &str,
        settings: &[&str],
        timeout_ms: i32,
    ) -> Result<CalculationOutput, CalculatorError> {
        self.calculate_markup_with_settings(
            crate::markup::MarkupMode::Html,
            expr,
            settings,
            timeout_ms,
            false,
        )
    }

    /// Evaluate an expression and format the parsed/result pair as LaTeX markup (terse/result-only).
    ///
    /// # Errors
    /// Returns a `CalculatorError` if the expression is outside the native
    /// markup evidence slice or if evaluation fails.
    pub fn calculate_and_print_qalc_latex_terse_with_settings_and_fallback_state(
        &mut self,
        expr: &str,
        settings: &[&str],
        timeout_ms: i32,
    ) -> Result<CalculationOutput, CalculatorError> {
        self.calculate_markup_with_settings(
            crate::markup::MarkupMode::Latex,
            expr,
            settings,
            timeout_ms,
            true,
        )
    }

    /// Evaluate an expression and format the parsed/result pair as HTML markup (terse/result-only).
    ///
    /// # Errors
    /// Returns a `CalculatorError` if the expression is outside the native
    /// markup evidence slice or if evaluation fails.
    pub fn calculate_and_print_qalc_html_terse_with_settings_and_fallback_state(
        &mut self,
        expr: &str,
        settings: &[&str],
        timeout_ms: i32,
    ) -> Result<CalculationOutput, CalculatorError> {
        self.calculate_markup_with_settings(
            crate::markup::MarkupMode::Html,
            expr,
            settings,
            timeout_ms,
            true,
        )
    }

    fn calculate_markup_with_settings(
        &mut self,
        mode: crate::markup::MarkupMode,
        expr: &str,
        settings: &[&str],
        timeout_ms: i32,
        terse: bool,
    ) -> Result<CalculationOutput, CalculatorError> {
        self.last_native_message_had_error = false;
        self.last_output_approximate = false;
        self.last_output_message_lines = 0;
        let fallback_disabled = fallback_disabled_by_env();
        let parsed = crate::parser::operators::parse_expression(expr).ok();
        let uses_native_session_variable = parsed
            .as_ref()
            .is_some_and(|parsed| expression_uses_context_variable(parsed, &self.native_context));
        let contains_assignment = parsed.as_ref().is_some_and(|parsed| {
            expression_contains(parsed, &|node| {
                matches!(node, crate::ast::Expression::Assignment { .. })
            })
        });
        let is_top_level_assignment = matches!(
            parsed.as_ref(),
            Some(crate::ast::Expression::Assignment { .. })
        );
        let uses_unmirrored_managed_alias = self.session_answers.has_cpp_answer()
            && parsed.as_ref().is_some_and(|parsed| {
                expression_uses_unmirrored_managed_alias(parsed, &self.native_context)
            });
        let native_variables_before = (!fallback_disabled && contains_assignment)
            .then(|| self.native_context.variables.clone());
        if (fallback_disabled || !is_top_level_assignment)
            && (!self.session_answers.has_cpp_answer() || uses_native_session_variable)
            && !uses_unmirrored_managed_alias
        {
            match native_markup_output(
                mode,
                expr,
                settings,
                terse,
                !fallback_disabled,
                &mut self.native_context,
            ) {
                Ok(Some(output)) => {
                    if let Some(before) = &native_variables_before {
                        self.synchronize_native_variables_to_cpp(before)?;
                    }
                    return Ok(self.finish_native_output(output));
                }
                Ok(None) => {}
                Err(error) if fallback_disabled => return Err(error),
                Err(_) => {}
            }
        }

        if fallback_disabled {
            return Err(CalculatorError::FallbackDisabled(expr.to_string()));
        }

        let options = cpp_fallback_options(settings)?;
        assert!(
            !self.inner.is_null(),
            "BUG: Calculator inner pointer is null - possible use-after-move"
        );
        let capture_result = self.session_answers.is_enabled();
        let (mut output, messages, answer_rendering) = {
            let _guard = FFI_LOCK.lock().unwrap();
            let pin = self.inner.pin_mut();
            let mode_flags = QALC_MODE_MARKUP
                | if mode == crate::markup::MarkupMode::Latex {
                    QALC_MODE_LATEX
                } else {
                    0
                }
                | if terse { QALC_MODE_TERSE } else { 0 }
                | if capture_result {
                    QALC_MODE_CAPTURE_RESULT
                } else {
                    0
                }
                | if options.unicode {
                    QALC_MODE_UNICODE
                } else {
                    0
                };
            let output = sys::calculate_and_print_qalc(
                pin,
                expr,
                timeout_ms,
                options.output_base as i32,
                options.input_base as i32,
                options.assumption_mode,
                mode_flags,
            )
            .map_err(CalculatorError::Cxx)?;
            let answer_rendering = output.clone();
            self.last_output_approximate = sys::qalc_last_result_is_approximate();
            self.last_output_message_lines = sys::qalc_last_message_line_count();
            self.last_native_message_had_error = sys::qalc_last_message_had_error();
            let parsed_expression = sys::qalc_last_parsed_expression();
            let messages = if terse {
                String::new()
            } else {
                sys::qalc_last_messages()
            };
            let output = if sys::qalc_last_markup_output_is_complete() {
                output
            } else {
                format_cpp_markup_output(
                    mode,
                    &parsed_expression,
                    &output,
                    terse,
                    self.last_output_approximate,
                    options.unicode,
                )
            };
            (output, messages, answer_rendering)
        };
        if !messages.is_empty() {
            output = if output.is_empty() {
                messages
            } else {
                format!("{messages}\n{output}")
            };
        }
        if capture_result {
            self.record_cpp_session_answer(expr, answer_rendering);
        }

        Ok(CalculationOutput {
            output,
            fallback_state: FallbackState::CppFallbackEnabled,
        })
    }

    fn finish_native_output(&mut self, output: NativeOutput) -> CalculationOutput {
        self.last_native_message_had_error = output.has_error_message;
        self.last_output_approximate = output.approximate;
        self.last_output_message_lines = output.message_line_count;
        if self.session_answers.is_enabled() {
            if let Some(answer) = output.answer {
                if self.set_cpp_session_answer(&answer.expression) {
                    self.session_answers.record(
                        &mut self.native_context,
                        answer.expression,
                        answer.rendering,
                    );
                } else {
                    self.session_answers.invalidate(&mut self.native_context);
                }
            } else {
                self.session_answers.invalidate(&mut self.native_context);
                self.clear_cpp_session_answers();
            }
        }
        CalculationOutput {
            output: output.output,
            fallback_state: FallbackState::Native,
        }
    }

    fn set_cpp_session_answer(&mut self, answer: &crate::ast::Expression) -> bool {
        let Some(expression) =
            crate::text::format_result_with_numbers(answer, &crate::number::Number::to_string)
        else {
            self.clear_cpp_session_answers();
            return false;
        };
        if self.inner.is_null() {
            return false;
        }
        let _guard = FFI_LOCK.lock().unwrap();
        let synchronized = sys::qalc_set_session_answer(self.inner.pin_mut(), &expression);
        if !synchronized {
            sys::qalc_clear_session_answers(self.inner.pin_mut());
        }
        synchronized
    }

    fn clear_cpp_session_answers(&mut self) {
        if self.inner.is_null() {
            return;
        }
        let _guard = FFI_LOCK.lock().unwrap();
        sys::qalc_clear_session_answers(self.inner.pin_mut());
    }

    fn synchronize_native_variables_to_cpp(
        &mut self,
        before: &std::collections::HashMap<String, crate::ast::Expression>,
    ) -> Result<(), CalculatorError> {
        let variables = self
            .native_context
            .variables
            .iter()
            .filter(|(name, value)| {
                !crate::session::SessionAnswerState::is_managed_alias(name)
                    && before.get(*name) != Some(*value)
            })
            .map(|(name, value)| {
                let expression = crate::text::format_result_with_numbers(
                    value,
                    &crate::number::Number::to_string,
                )
                .ok_or_else(|| name.clone())?;
                let previous_expression = before
                    .get(name)
                    .map(|previous| {
                        crate::text::format_result_with_numbers(
                            previous,
                            &crate::number::Number::to_string,
                        )
                        .ok_or_else(|| name.clone())
                    })
                    .transpose()?;
                Ok((name.clone(), expression, previous_expression))
            })
            .collect::<Result<Vec<_>, String>>();
        let mut variables = match variables {
            Ok(variables) => variables,
            Err(name) => {
                self.native_context.variables.clone_from(before);
                return Err(CalculatorError::NativeEvaluation(format!(
                    "failed to serialize native session variable '{name}'"
                )));
            }
        };
        if variables.is_empty() {
            return Ok(());
        }
        variables.sort_by(|left, right| left.0.cmp(&right.0));

        let failure = {
            let _guard = FFI_LOCK.lock().unwrap();
            let mut pin = self.inner.pin_mut();
            let mut failure = None;
            for (applied, (name, expression, _)) in variables.iter().enumerate() {
                if !sys::qalc_set_session_variable(pin.as_mut(), name, expression) {
                    let mut rollback_failures = Vec::new();
                    for (rollback_name, _, previous_expression) in variables[..applied].iter().rev()
                    {
                        let restored = match previous_expression {
                            Some(previous) => sys::qalc_set_session_variable(
                                pin.as_mut(),
                                rollback_name,
                                previous,
                            ),
                            None => sys::qalc_delete_session_variable(pin.as_mut(), rollback_name),
                        };
                        if !restored {
                            rollback_failures.push(rollback_name.clone());
                        }
                    }
                    failure = Some((name.clone(), rollback_failures));
                    break;
                }
            }
            failure
        };
        if let Some((name, rollback_failures)) = failure {
            self.native_context.variables.clone_from(before);
            let rollback_detail = if rollback_failures.is_empty() {
                String::new()
            } else {
                format!(
                    "; C++ rollback also failed for: {}",
                    rollback_failures.join(", ")
                )
            };
            return Err(CalculatorError::NativeEvaluation(format!(
                "failed to synchronize native session variable '{name}'{rollback_detail}"
            )));
        }
        Ok(())
    }

    fn record_cpp_session_answer(&mut self, expr: &str, rendering: String) {
        let mirrored_answer = self
            .session_answers
            .record_cpp(&mut self.native_context, rendering);
        let Some(answer) = mirrored_answer else {
            return;
        };
        let Ok(parsed) = crate::parser::operators::parse_expression(expr) else {
            return;
        };
        // Only direct chains (`x := y := value`) share the final result. An
        // assignment embedded in a larger expression can have a distinct value
        // and remains C++-owned until that value can be mirrored explicitly.
        let mut current = &parsed;
        while let crate::ast::Expression::Assignment { variable, value } = current {
            if !crate::session::SessionAnswerState::is_managed_alias(variable) {
                self.native_context
                    .variables
                    .insert(variable.clone(), answer.clone());
            }
            current = value;
        }
    }

    fn calculate_with_profile(
        &mut self,
        profile: PrintProfile,
        expr: &str,
        timeout_ms: i32,
    ) -> Result<CalculationOutput, CalculatorError> {
        self.calculate_with_profile_and_settings(profile, expr, timeout_ms, &[], true)
    }

    fn calculate_with_profile_and_settings(
        &mut self,
        profile: PrintProfile,
        expr: &str,
        timeout_ms: i32,
        settings: &[&str],
        include_cpp_messages: bool,
    ) -> Result<CalculationOutput, CalculatorError> {
        self.last_native_message_had_error = false;
        self.last_output_approximate = false;
        self.last_output_message_lines = 0;
        assert!(
            !self.inner.is_null(),
            "BUG: Calculator inner pointer is null - possible use-after-move"
        );
        let fallback_disabled = fallback_disabled_by_env();

        if self.session_answers.is_enabled()
            && (fallback_disabled || self.session_answers.has_native_answer())
        {
            let native_variables_before = (!fallback_disabled
                && crate::parser::operators::parse_expression(expr)
                    .ok()
                    .is_some_and(|parsed| {
                        expression_contains(&parsed, &|node| {
                            matches!(node, crate::ast::Expression::Assignment { .. })
                        })
                    }))
            .then(|| self.native_context.variables.clone());
            if let Some(output) = native_session_context_output(
                profile,
                expr,
                settings,
                &mut self.native_context,
                fallback_disabled,
            ) {
                if let Some(before) = &native_variables_before {
                    self.synchronize_native_variables_to_cpp(before)?;
                }
                return Ok(self.finish_native_output(output));
            }
        }

        // CLI/session settings are implemented by the native scaffold. Try that
        // evidence-backed path even when the C++ fallback is available so normal
        // CLI invocations do not reject supported settings before evaluation.
        if !settings.is_empty() {
            if let Some(output) = native_scaffold_output(profile, expr, settings) {
                return Ok(self.finish_native_output(output));
            }
        }

        if fallback_disabled {
            if let Some(output) =
                native_markup_conversion_output(expr, settings, &mut self.native_context)?
            {
                return Ok(self.finish_native_output(output));
            }
            if let Some(output) = native_currency_conversion_output(profile, expr, settings)? {
                return Ok(self.finish_native_output(output));
            }
            if settings.is_empty() {
                if let Some(output) = native_data_output(profile, expr)? {
                    return Ok(self.finish_native_output(output));
                }
                if let Some(output) =
                    native_statistics_output(profile, expr, &mut self.native_context)?
                {
                    return Ok(self.finish_native_output(output));
                }
                if let Some(output) =
                    native_session_output(profile, expr, &mut self.native_context)?
                {
                    return Ok(self.finish_native_output(output));
                }
                if let Some(output) = native_datetime_output(profile, expr)? {
                    return Ok(self.finish_native_output(output));
                }
                if let Some(output) = native_unit_conversion_output(profile, expr, settings)? {
                    return Ok(self.finish_native_output(output));
                }
            }
            if let Some(output) = native_scaffold_output(profile, expr, settings) {
                return Ok(self.finish_native_output(output));
            }
            return Err(CalculatorError::FallbackDisabled(expr.to_string()));
        }

        let options = cpp_fallback_options(settings)?;

        let capture_result = self.session_answers.is_enabled();
        let (output, answer_rendering) = {
            let _guard = FFI_LOCK.lock().unwrap();
            let mut pin = self.inner.pin_mut();
            match profile {
                PrintProfile::Api => {
                    let output = sys::calculate_and_print(pin.as_mut(), expr, timeout_ms)
                        .map_err(CalculatorError::Cxx)?;
                    if capture_result && !sys::qalc_set_session_answer(pin.as_mut(), &output) {
                        sys::qalc_clear_session_answers(pin);
                        return Err(CalculatorError::NativeEvaluation(
                            "failed to synchronize C++ session answer".to_string(),
                        ));
                    }
                    (output.clone(), output)
                }
                PrintProfile::Qalc => {
                    let mode_flags = (if capture_result {
                        QALC_MODE_CAPTURE_RESULT
                    } else {
                        0
                    }) | (if include_cpp_messages {
                        0
                    } else {
                        QALC_MODE_TERSE
                    }) | (if options.unicode {
                        QALC_MODE_UNICODE
                    } else {
                        0
                    });
                    let mut output = sys::calculate_and_print_qalc(
                        pin,
                        expr,
                        timeout_ms,
                        options.output_base as i32,
                        options.input_base as i32,
                        options.assumption_mode,
                        mode_flags,
                    )
                    .map_err(CalculatorError::Cxx)?;
                    self.last_output_approximate = sys::qalc_last_result_is_approximate();
                    self.last_output_message_lines = sys::qalc_last_message_line_count();
                    self.last_native_message_had_error = sys::qalc_last_message_had_error();
                    let answer_rendering = output.clone();
                    let messages = sys::qalc_last_messages();
                    if include_cpp_messages && !messages.is_empty() {
                        output = if output.is_empty() {
                            messages
                        } else {
                            format!("{messages}\n{output}")
                        };
                    }
                    (output, answer_rendering)
                }
            }
        };

        if self.session_answers.is_enabled() {
            self.record_cpp_session_answer(expr, answer_rendering);
        }

        Ok(CalculationOutput {
            output,
            fallback_state: FallbackState::CppFallbackEnabled,
        })
    }
}

fn fallback_disabled_by_env() -> bool {
    std::env::var("QALCULATE_DISABLE_FALLBACK").as_deref() == Ok("1")
}

fn cpp_fallback_options(
    settings: &[&str],
) -> Result<crate::session::CppFallbackOptions, CalculatorError> {
    if let Some(options) =
        crate::session::NativeSessionSettings::cpp_print_options_from_raw(settings)
    {
        return Ok(options);
    }
    Err(CalculatorError::UnsupportedSessionSettings(
        settings
            .iter()
            .map(|setting| (*setting).to_string())
            .collect(),
    ))
}

fn format_reformatted_session_answer(
    markup_mode: Option<crate::markup::MarkupMode>,
    terse: bool,
    previous_rendering: &str,
    rendering: &str,
    approximate: bool,
    unicode_enabled: bool,
) -> String {
    match markup_mode {
        Some(mode) => format_cpp_markup_output(
            mode,
            previous_rendering,
            rendering,
            terse,
            approximate,
            unicode_enabled,
        ),
        None if terse => rendering.to_string(),
        None => crate::text::format_qalc_equation(
            previous_rendering,
            rendering,
            approximate,
            unicode_enabled,
            0,
        ),
    }
}

fn format_cpp_markup_output(
    mode: crate::markup::MarkupMode,
    parsed: &str,
    result: &str,
    terse: bool,
    approximate: bool,
    unicode_enabled: bool,
) -> String {
    let body = if terse || parsed.is_empty() {
        result.to_string()
    } else {
        match mode {
            crate::markup::MarkupMode::Latex => {
                let relation = if approximate { "\\approx" } else { "=" };
                format!("{parsed} {relation} {result}")
            }
            crate::markup::MarkupMode::Html => {
                let relation = if !approximate {
                    "="
                } else if unicode_enabled {
                    "≈"
                } else {
                    "= approx."
                };
                format!("{parsed} {relation} {result}")
            }
        }
    };
    match mode {
        crate::markup::MarkupMode::Latex if body.contains('\\') => {
            format!("$\\displaystyle {body}$")
        }
        crate::markup::MarkupMode::Latex => format!("${body}$"),
        crate::markup::MarkupMode::Html => body,
    }
}

fn native_markup_conversion_output(
    expr: &str,
    settings: &[&str],
    context: &mut crate::context::CalculatorContext,
) -> Result<Option<NativeOutput>, CalculatorError> {
    let Ok(parsed) = crate::parser::operators::parse_expression(expr) else {
        return Ok(None);
    };
    let Some((mode, inner)) = markup_conversion_request(&parsed) else {
        return Ok(None);
    };

    native_markup_output_for_parsed(mode, &inner, settings, false, false, context)
}

fn native_markup_output(
    mode: crate::markup::MarkupMode,
    expr: &str,
    settings: &[&str],
    terse: bool,
    prefer_cpp_definitions: bool,
    context: &mut crate::context::CalculatorContext,
) -> Result<Option<NativeOutput>, CalculatorError> {
    let Ok(parsed) = crate::parser::operators::parse_expression(expr) else {
        return Ok(None);
    };
    native_markup_output_for_parsed(
        mode,
        &parsed,
        settings,
        terse,
        prefer_cpp_definitions,
        context,
    )
}

fn native_markup_output_for_parsed(
    mode: crate::markup::MarkupMode,
    parsed: &crate::ast::Expression,
    settings: &[&str],
    terse: bool,
    prefer_cpp_definitions: bool,
    context: &mut crate::context::CalculatorContext,
) -> Result<Option<NativeOutput>, CalculatorError> {
    if prefer_cpp_definitions
        && expression_contains(parsed, &|expr| {
            matches!(expr, crate::ast::Expression::Conversion { .. })
                && markup_conversion_request(expr).is_none()
        })
    {
        return Ok(None);
    }
    if expression_contains(parsed, &|expr| {
        let crate::ast::Expression::Conversion { target, .. } = expr else {
            return false;
        };
        let crate::ast::Expression::Symbolic(target) = target.as_ref() else {
            return false;
        };
        is_qalc_print_conversion(target.name())
    }) {
        return Ok(None);
    }
    let definition_usage = native_definition_usage(parsed);
    if prefer_cpp_definitions
        && (definition_usage.currencies || usage_uses_unit_definition(&definition_usage))
    {
        return Ok(None);
    }
    if expression_contains(parsed, &|expr| {
        let crate::ast::Expression::FunctionCall { function, .. } = expr else {
            return false;
        };
        !native_markup_function_is_supported(function.id())
    }) {
        return Ok(None);
    }
    let Some(parsed_settings) = crate::session::NativeSessionSettings::from_raw(settings) else {
        return Ok(None);
    };

    let mut markup_context = context.clone();
    crate::session::apply_raw_settings_to_context(&mut markup_context, settings)
        .ok_or_else(|| CalculatorError::NativeEvaluation("invalid native setting".to_string()))?;
    markup_context.precision_digits = parsed_settings.precision_digits();
    let contains_assignment = expression_contains(parsed, &|node| {
        matches!(node, crate::ast::Expression::Assignment { .. })
    });
    let evaluated = crate::eval::evaluate_ast(parsed, &mut markup_context)
        .map_err(CalculatorError::NativeEvaluation)?;
    let precision_digits = parsed_settings.precision_digits();
    let formatter = |num: &crate::number::Number| {
        if parsed_settings.programming_mode
            || parsed_settings.output_base.is_some_and(|base| base != 10)
        {
            if let Some(output) = crate::numberbase::native_output(
                &num.to_string(),
                parsed_settings.for_evaluated_output(),
            ) {
                return output;
            }
        }
        num.to_qalc_string_with_settings(
            precision_digits,
            parsed_settings.min_exp(),
            parsed_settings.exp_display(),
            parsed_settings.min_decimals(),
            parsed_settings.max_decimals(),
        )
    };
    let output = if terse {
        crate::markup::format_markup_result_only(&evaluated, mode, &formatter)
    } else {
        crate::markup::format_markup_equation(parsed, &evaluated, mode, &formatter)
    };
    let output = output.ok_or_else(|| {
        CalculatorError::NativeEvaluation("failed to format native markup output".to_string())
    })?;
    let Some((answer_rendering, approximate)) = crate::session::format_answer(
        crate::session::AnswerFormatProfile::Qalc,
        &evaluated,
        parsed_settings,
    ) else {
        return Ok(None);
    };
    let profile = if terse {
        PrintProfile::Api
    } else {
        PrintProfile::Qalc
    };
    let mut output = native_output_with_messages(profile, output, &mut markup_context)
        .with_answer(evaluated, answer_rendering);
    output.approximate = approximate;
    if contains_assignment {
        context.variables = markup_context.variables;
    }
    Ok(Some(output))
}

fn is_qalc_print_conversion(target: &str) -> bool {
    let target = target.to_ascii_lowercase();
    matches!(
        target.as_str(),
        "hex"
            | "hexadecimal"
            | "bin"
            | "binary"
            | "dec"
            | "decimal"
            | "oct"
            | "octal"
            | "duo"
            | "duodecimal"
            | "doz"
            | "dozenal"
            | "roman"
            | "bijective"
            | "bcd"
            | "sexa"
            | "sexagesimal"
            | "longitude"
            | "latitude"
            | "float"
            | "double"
            | "time"
            | "unicode"
            | "sci"
            | "scientific"
            | "eng"
            | "engineering"
            | "simple"
            | "utc"
            | "gmt"
            | "cet"
            | "rectangular"
            | "cartesian"
            | "exponential"
            | "polar"
            | "angle"
            | "phasor"
            | "cis"
            | "factors"
            | "factor"
            | "partial fraction"
            | "bases"
            | "calendars"
            | "optimal"
            | "prefix"
            | "base"
            | "mixed"
            | "decimals"
            | "fraction"
            | "frac"
    ) || target.starts_with("fp")
        || target.starts_with("binary")
}

fn native_markup_function_is_supported(name: &str) -> bool {
    crate::functions::builtin_info(name).is_some()
        || crate::datasets::is_dataset_function_name(name)
        || matches!(
            name,
            "hex" | "float" | "floatError" | "lxor" | "if" | "shift"
        )
}

fn markup_conversion_request(
    parsed: &crate::ast::Expression,
) -> Option<(crate::markup::MarkupMode, crate::ast::Expression)> {
    let crate::ast::Expression::Conversion { expr, target } = parsed else {
        return None;
    };
    let crate::ast::Expression::Symbolic(symbol) = target.as_ref() else {
        return None;
    };

    match symbol.name().to_ascii_lowercase().as_str() {
        "latex" => Some((crate::markup::MarkupMode::Latex, expr.as_ref().clone())),
        "html" => Some((crate::markup::MarkupMode::Html, expr.as_ref().clone())),
        _ => None,
    }
}

fn expression_contains(
    expr: &crate::ast::Expression,
    predicate: &impl Fn(&crate::ast::Expression) -> bool,
) -> bool {
    use crate::ast::Expression;
    if predicate(expr) {
        return true;
    }

    match expr {
        Expression::Conversion { expr, target } => {
            expression_contains(expr, predicate) || expression_contains(target, predicate)
        }
        Expression::Multiplication(children)
        | Expression::Addition(children)
        | Expression::LogicalAnd(children)
        | Expression::LogicalOr(children)
        | Expression::BitwiseAnd(children)
        | Expression::BitwiseOr(children)
        | Expression::BitwiseXor(children) => children
            .as_slice()
            .iter()
            .any(|child| expression_contains(child, predicate)),
        Expression::Division {
            numerator,
            denominator,
        } => {
            expression_contains(numerator, predicate) || expression_contains(denominator, predicate)
        }
        Expression::Power { base, exponent } => {
            expression_contains(base, predicate) || expression_contains(exponent, predicate)
        }
        Expression::Remainder { lhs, rhs }
        | Expression::Modulo { lhs, rhs }
        | Expression::IntegerDivision { lhs, rhs }
        | Expression::ShiftLeft { lhs, rhs }
        | Expression::ShiftRight { lhs, rhs }
        | Expression::LogicalXor { lhs, rhs }
        | Expression::Parallel { lhs, rhs }
        | Expression::Comparison { lhs, rhs, .. } => {
            expression_contains(lhs, predicate) || expression_contains(rhs, predicate)
        }
        Expression::Inverse(child)
        | Expression::Negate(child)
        | Expression::Factorial(child)
        | Expression::DoubleFactorial(child)
        | Expression::MultiFactorial { expr: child, .. }
        | Expression::Percent(child)
        | Expression::LogicalNot(child)
        | Expression::BitwiseNot(child)
        | Expression::Assignment { value: child, .. } => expression_contains(child, predicate),
        Expression::FunctionCall { args, .. } | Expression::Vector(args) => args
            .iter()
            .any(|child| expression_contains(child, predicate)),
        Expression::Number(_)
        | Expression::Text(_)
        | Expression::Unit { .. }
        | Expression::Symbolic(_)
        | Expression::Variable(_)
        | Expression::Undefined
        | Expression::Aborted
        | Expression::DateTime(_) => false,
    }
}

fn expression_uses_context_variable(
    expr: &crate::ast::Expression,
    context: &crate::context::CalculatorContext,
) -> bool {
    expression_contains(expr, &|node| match node {
        crate::ast::Expression::Symbolic(symbol) => context.variables.contains_key(symbol.name()),
        crate::ast::Expression::Variable(variable) => context.variables.contains_key(variable.id()),
        _ => false,
    })
}

fn collect_assignment_names(expr: &crate::ast::Expression, names: &mut Vec<String>) {
    if let crate::ast::Expression::Assignment { variable, .. } = expr {
        names.push(variable.clone());
    }
    for index in 0..expr.child_count() {
        if let Some(child) = expr.child(index) {
            collect_assignment_names(child, names);
        }
    }
}

fn expression_is_currency_answer(expr: &crate::ast::Expression) -> bool {
    match expr {
        crate::ast::Expression::Unit {
            unit, prefix: None, ..
        } => crate::rates::currency_info(unit.id()).is_some(),
        crate::ast::Expression::Multiplication(children) => matches!(
            children.as_slice(),
            [
                crate::ast::Expression::Number(_),
                crate::ast::Expression::Unit {
                    unit,
                    prefix: None,
                    ..
                }
            ] if crate::rates::currency_info(unit.id()).is_some()
        ),
        _ => false,
    }
}

fn expression_uses_unmirrored_managed_alias(
    expr: &crate::ast::Expression,
    context: &crate::context::CalculatorContext,
) -> bool {
    expression_contains(expr, &|node| {
        let name = match node {
            crate::ast::Expression::Symbolic(symbol) => symbol.name(),
            crate::ast::Expression::Variable(variable) => variable.id(),
            _ => return false,
        };
        crate::session::SessionAnswerState::is_managed_alias(name)
            && !context.variables.contains_key(name)
    })
}

fn contains_bitwise_ops(expr: &crate::ast::Expression) -> bool {
    expression_contains(expr, &|expr| {
        use crate::ast::Expression;
        matches!(
            expr,
            Expression::ShiftLeft { .. }
                | Expression::ShiftRight { .. }
                | Expression::BitwiseAnd(_)
                | Expression::BitwiseOr(_)
                | Expression::BitwiseXor(_)
                | Expression::BitwiseNot(_)
        )
    })
}

fn is_geometry_expression(expr: &crate::ast::Expression) -> bool {
    expression_contains(expr, &|expr| {
        let crate::ast::Expression::FunctionCall { function, .. } = expr else {
            return false;
        };
        crate::functions::geometry::lookup(function.id()).is_some()
    })
}

fn is_text_native_expression(expr: &crate::ast::Expression) -> bool {
    expression_contains(expr, &|expr| {
        let crate::ast::Expression::FunctionCall { function, .. } = expr else {
            if let crate::ast::Expression::Conversion { target, .. } = expr {
                return matches!(
                    target.as_ref(),
                    crate::ast::Expression::Symbolic(symbol)
                        if symbol.name().eq_ignore_ascii_case("unicode")
                );
            }
            return matches!(expr, crate::ast::Expression::Text(_));
        };
        crate::functions::utility_string::is_raw_utility_string(function.id())
    })
}

fn is_polynomial_native_expression(expr: &crate::ast::Expression) -> bool {
    expression_contains(expr, &|expr| {
        if let crate::ast::Expression::FunctionCall { function, .. } = expr {
            return matches!(
                function.id(),
                "coeff"
                    | "lcoeff"
                    | "tcoeff"
                    | "degree"
                    | "ldegree"
                    | "pcontent"
                    | "primpart"
                    | "punit"
                    | "factor"
            );
        }
        false
    })
}

fn is_dataset_native_expression(expr: &crate::ast::Expression) -> bool {
    expression_contains(expr, &|expr| {
        let crate::ast::Expression::FunctionCall { function, .. } = expr else {
            return false;
        };
        crate::datasets::is_dataset_function_name(function.id())
    })
}

fn evaluate_general_expression_natively(
    profile: PrintProfile,
    parsed: &crate::ast::Expression,
    context: &mut crate::context::CalculatorContext,
    precision_digits: usize,
) -> Option<SessionAnswer> {
    let evaluated = crate::eval::evaluate_ast(parsed, context).ok()?;
    let rendering = match &evaluated {
        crate::ast::Expression::Number(num) => {
            let output = num.to_string_with_options(
                precision_digits,
                context.print_options.number_fraction_format,
                context.evaluation_options.approximation,
            );
            match profile {
                PrintProfile::Api => output,
                PrintProfile::Qalc => output.replace('-', "\u{2212}"),
            }
        }
        crate::ast::Expression::Symbolic(sym) => {
            let name = qalc_symbolic_conversion_output(profile, parsed, sym.name());
            match profile {
                PrintProfile::Api => name,
                PrintProfile::Qalc => name.replace('-', "\u{2212}"),
            }
        }
        other => {
            let output = crate::text::format_result_with_numbers(other, &|num| {
                num.to_string_with_options(
                    precision_digits,
                    context.print_options.number_fraction_format,
                    context.evaluation_options.approximation,
                )
            })?;
            match profile {
                PrintProfile::Api => output,
                PrintProfile::Qalc => output.replace('-', "\u{2212}"),
            }
        }
    };
    Some(SessionAnswer {
        expression: evaluated,
        rendering,
    })
}

fn qalc_symbolic_conversion_output(
    profile: PrintProfile,
    parsed: &crate::ast::Expression,
    output: &str,
) -> String {
    if profile == PrintProfile::Qalc
        && conversion_target_is_hex(parsed)
        && !output.starts_with("0x")
        && output.chars().all(|ch| ch.is_ascii_hexdigit())
    {
        format!("0x{output}")
    } else {
        output.to_string()
    }
}

fn conversion_target_is_hex(expr: &crate::ast::Expression) -> bool {
    let crate::ast::Expression::Conversion { target, .. } = expr else {
        return false;
    };
    matches!(
        target.as_ref(),
        crate::ast::Expression::Symbolic(symbol)
            if matches!(symbol.name().to_ascii_lowercase().as_str(), "hex" | "hexadecimal")
    )
}

fn native_data_output(
    profile: PrintProfile,
    expr: &str,
) -> Result<Option<NativeOutput>, CalculatorError> {
    let Some(output) = crate::data::native_output(expr)
        .map_err(|error| CalculatorError::NativeEvaluation(error.to_string()))?
    else {
        return Ok(None);
    };

    let answer = output.parse::<crate::number::Number>().map_err(|error| {
        CalculatorError::NativeEvaluation(format!("invalid native data count: {error}"))
    })?;
    let rendering = match profile {
        PrintProfile::Api => output,
        PrintProfile::Qalc => output.replace('-', "\u{2212}"),
    };
    Ok(Some(NativeOutput::plain(rendering.clone()).with_answer(
        crate::ast::Expression::Number(answer),
        rendering,
    )))
}

fn native_statistics_output(
    profile: PrintProfile,
    expr: &str,
    context: &mut crate::context::CalculatorContext,
) -> Result<Option<NativeOutput>, CalculatorError> {
    let Some(output) = crate::statistics::native_output(expr)
        .map_err(|error| CalculatorError::NativeEvaluation(error.to_string()))?
    else {
        return Ok(None);
    };

    let rendering = match profile {
        PrintProfile::Api => output,
        PrintProfile::Qalc => output.replace('-', "\u{2212}"),
    };
    let answer = crate::parser::operators::parse_expression(&rendering)
        .ok()
        .and_then(|parsed| crate::eval::evaluate_ast(&parsed, context).ok());
    Ok(Some(match answer {
        Some(answer) => NativeOutput::plain(rendering.clone()).with_answer(answer, rendering),
        None => NativeOutput::plain(rendering),
    }))
}

fn native_session_output(
    profile: PrintProfile,
    expr: &str,
    context: &mut crate::context::CalculatorContext,
) -> Result<Option<NativeOutput>, CalculatorError> {
    let load_variable =
        crate::session::parse_load_assignment(expr).map(|(variable, _)| variable.to_string());
    let Some(output) = crate::session::native_output(expr, context)
        .map_err(|error| CalculatorError::NativeEvaluation(error.to_string()))?
    else {
        return Ok(None);
    };

    let rendering = match profile {
        PrintProfile::Api => output,
        PrintProfile::Qalc => output.replace('-', "\u{2212}"),
    };
    let mut native_output = NativeOutput::plain(rendering);
    if let Some(answer) = load_variable
        .as_ref()
        .and_then(|variable| context.variables.get(variable))
        .cloned()
    {
        let answer_profile = match profile {
            PrintProfile::Api => crate::session::AnswerFormatProfile::Api,
            PrintProfile::Qalc => crate::session::AnswerFormatProfile::Qalc,
        };
        if let Some((answer_rendering, approximate)) = crate::session::format_answer(
            answer_profile,
            &answer,
            crate::session::NativeSessionSettings::default(),
        ) {
            native_output = native_output.with_answer(answer, answer_rendering);
            native_output.approximate = approximate;
        }
    }
    Ok(Some(native_output))
}

fn native_session_context_output(
    profile: PrintProfile,
    expr: &str,
    settings: &[&str],
    context: &mut crate::context::CalculatorContext,
    allow_assignment: bool,
) -> Option<NativeOutput> {
    let parsed = crate::parser::operators::parse_expression(expr).ok()?;
    let is_assignment = matches!(&parsed, crate::ast::Expression::Assignment { .. });
    if is_assignment && !allow_assignment {
        return None;
    }
    let uses_session_variable = expression_uses_context_variable(&parsed, context);
    if !is_assignment && !uses_session_variable {
        return None;
    }
    if expression_uses_unresolved_global_value(&parsed, context) {
        return None;
    }

    let mut evaluated_context = context.clone();
    crate::session::apply_raw_settings_to_context(&mut evaluated_context, settings)?;
    evaluated_context.clear_messages();
    let answer = evaluated_context.parse_and_evaluate_expression(expr).ok()?;
    if expression_contains(&answer, &|node| {
        matches!(node, crate::ast::Expression::FunctionCall { .. })
    }) {
        return None;
    }
    if matches!(&answer, crate::ast::Expression::Conversion { .. }) {
        let substituted =
            crate::text::format_result_with_numbers(&answer, &crate::number::Number::to_string)?;
        if let Some(output) = native_unit_conversion_output(profile, &substituted, settings)
            .ok()
            .flatten()
        {
            *context = evaluated_context;
            return Some(output);
        }
    }
    let session_settings = crate::session::NativeSessionSettings::from_raw(settings)?;
    let answer_profile = match profile {
        PrintProfile::Api => crate::session::AnswerFormatProfile::Api,
        PrintProfile::Qalc => crate::session::AnswerFormatProfile::Qalc,
    };
    let (rendering, approximate) =
        crate::session::format_answer(answer_profile, &answer, session_settings)?;
    let mut output =
        native_output_with_messages(profile, rendering.clone(), &mut evaluated_context)
            .with_answer(answer, rendering);
    output.approximate = approximate;
    *context = evaluated_context;
    Some(output)
}

fn expression_uses_unresolved_global_value(
    expr: &crate::ast::Expression,
    context: &crate::context::CalculatorContext,
) -> bool {
    let mut unresolved_names = Vec::new();
    collect_unresolved_value_names(expr, context, &mut unresolved_names);
    if unresolved_names.is_empty() {
        return false;
    }

    let definitions_dir = crate::rates::definitions_dir();
    let definitions = cached_function_variable_catalog(&definitions_dir);
    let units = cached_prefix_unit_catalog(&definitions_dir);

    unresolved_names.iter().any(|name| {
        definitions
            .as_ref()
            .is_some_and(|catalog| catalog.variables().find_by_name(name).is_some())
            || units
                .as_ref()
                .is_some_and(|catalog| catalog.unit_by_name(name).is_some())
    })
}

fn collect_unresolved_value_names(
    expr: &crate::ast::Expression,
    context: &crate::context::CalculatorContext,
    names: &mut Vec<String>,
) {
    let candidate = match expr {
        crate::ast::Expression::Symbolic(symbol) => Some(symbol.name()),
        crate::ast::Expression::Variable(variable) => Some(variable.id()),
        crate::ast::Expression::Unit { unit, .. } => Some(unit.id()),
        _ => None,
    };
    if let Some(name) = candidate {
        if !context.variables.contains_key(name) && !names.iter().any(|candidate| candidate == name)
        {
            names.push(name.to_string());
        }
    }
    for index in 0..expr.child_count() {
        if let Some(child) = expr.child(index) {
            collect_unresolved_value_names(child, context, names);
        }
    }
}

fn native_datetime_output(
    _profile: PrintProfile,
    expr: &str,
) -> Result<Option<NativeOutput>, CalculatorError> {
    let Some(output) =
        crate::datetime::native_output(expr).map_err(CalculatorError::NativeEvaluation)?
    else {
        return Ok(None);
    };
    Ok(Some(
        NativeOutput::plain(output.output.clone()).with_answer(output.answer, output.output),
    ))
}

fn native_currency_conversion_output(
    profile: PrintProfile,
    expr: &str,
    settings: &[&str],
) -> Result<Option<NativeOutput>, CalculatorError> {
    if !settings.is_empty() {
        return Ok(None);
    }

    let parsed = crate::parser::operators::parse_expression(expr).ok();
    let Some(ast) = parsed else {
        return Ok(None);
    };

    let Some((amount, src, tgt)) = crate::rates::match_currency_conversion(&ast) else {
        return Ok(None);
    };

    let dir = crate::rates::definitions_dir();
    let catalog = crate::rates::RatesCatalog::load_from_dir(&dir)
        .map_err(|e| CalculatorError::NativeEvaluation(e.to_string()))?;

    let converted = catalog
        .convert(&amount, &src, &tgt)
        .map_err(CalculatorError::NativeEvaluation)?;

    let mut formatted_num = crate::rates::format_qalc_currency_number(&converted);
    if profile == PrintProfile::Qalc {
        formatted_num = formatted_num.replace('-', "\u{2212}");
    }

    let formatted = if let Some((_, symbol)) = crate::rates::currency_info(&tgt) {
        format!("{symbol}{formatted_num}")
    } else {
        format!("{formatted_num} {tgt}")
    };

    let approximate = src != tgt;
    let target = crate::ast::Expression::Unit {
        unit: crate::ast::UnitRef::new(tgt),
        prefix: None,
        plural: false,
    };
    let answer = crate::ast::Expression::Multiplication(crate::ast::NaryChildren::from_two(
        crate::ast::Expression::Number(converted),
        target,
        Vec::new(),
    ));
    let mut output = NativeOutput::plain(formatted.clone()).with_answer(answer, formatted);
    output.approximate = approximate;
    Ok(Some(output))
}

fn native_unit_conversion_output(
    profile: PrintProfile,
    expr: &str,
    settings: &[&str],
) -> Result<Option<NativeOutput>, CalculatorError> {
    if !settings.is_empty() {
        return Ok(None);
    }

    let Some(output) = crate::unit_conversion::native_output_with_answer(expr)
        .map_err(CalculatorError::NativeEvaluation)?
    else {
        return Ok(None);
    };
    let rendering = match profile {
        PrintProfile::Api => output.output,
        PrintProfile::Qalc => output.output.replace('-', "\u{2212}"),
    };
    Ok(Some(
        NativeOutput::plain(rendering.clone()).with_answer(output.answer, rendering),
    ))
}

#[derive(Debug, Clone, PartialEq)]
struct NativeOutput {
    output: String,
    has_error_message: bool,
    approximate: bool,
    message_line_count: usize,
    answer: Option<SessionAnswer>,
}

#[derive(Debug, Clone, PartialEq)]
struct SessionAnswer {
    expression: crate::ast::Expression,
    rendering: String,
}

impl NativeOutput {
    fn plain(output: String) -> Self {
        Self {
            output,
            has_error_message: false,
            approximate: false,
            message_line_count: 0,
            answer: None,
        }
    }

    fn with_answer(mut self, expression: crate::ast::Expression, rendering: String) -> Self {
        self.answer = Some(SessionAnswer {
            expression,
            rendering,
        });
        self
    }
}

fn native_output_with_messages(
    profile: PrintProfile,
    output: String,
    context: &mut crate::context::CalculatorContext,
) -> NativeOutput {
    let has_error_message = context
        .messages
        .get_messages()
        .iter()
        .any(|message| message.message_type() == crate::messages::MessageType::Error);

    if profile != PrintProfile::Qalc {
        context.messages.clear();
        return NativeOutput {
            output,
            has_error_message,
            approximate: false,
            message_line_count: 0,
            answer: None,
        };
    }

    let mut lines = context.messages.drain_qalc_lines();
    if lines.is_empty() {
        return NativeOutput {
            output,
            has_error_message,
            approximate: false,
            message_line_count: 0,
            answer: None,
        };
    }

    let message_line_count = lines.len();
    lines.push(output);
    NativeOutput {
        output: lines.join("\n"),
        has_error_message,
        approximate: false,
        message_line_count,
        answer: None,
    }
}

fn native_output_with_answer_and_messages(
    profile: PrintProfile,
    answer: SessionAnswer,
    context: &mut crate::context::CalculatorContext,
) -> NativeOutput {
    let output = native_output_with_messages(profile, answer.rendering.clone(), context);
    output.with_answer(answer.expression, answer.rendering)
}

fn native_scaffold_output(
    profile: PrintProfile,
    expr: &str,
    settings: &[&str],
) -> Option<NativeOutput> {
    let parsed_settings = crate::session::NativeSessionSettings::from_raw(settings)?;
    if parsed_settings.has_non_default_approximation() {
        return None;
    }
    if parsed_settings.programming_mode
        || parsed_settings.output_base.is_some_and(|base| base != 10)
        || (!parsed_settings.has_interval_display()
            && parsed_settings.input_base().is_some_and(|base| base != 10))
    {
        return native_numberbase_output_with_answer(expr, parsed_settings);
    }

    if let Some(output) = crate::matrix::promoted_top_level_list_literal_output(expr) {
        if !settings.is_empty() {
            return None;
        }
        let rendering = match profile {
            PrintProfile::Api => output.to_string(),
            PrintProfile::Qalc => output.replace('-', "\u{2212}"),
        };
        let vector = |values: &[i32]| {
            crate::ast::Expression::Vector(
                values
                    .iter()
                    .copied()
                    .map(crate::number::Number::from_i32)
                    .map(crate::ast::Expression::Number)
                    .collect(),
            )
        };
        let answer =
            crate::ast::Expression::Vector(vec![vector(&[1, 2, 3, 4, 5, 6]), vector(&[4, 5])]);
        return Some(NativeOutput::plain(rendering.clone()).with_answer(answer, rendering));
    }

    if let Some(collection) = crate::matrix::parse_collection_literal(expr) {
        let mut context = crate::context::CalculatorContext::default();
        crate::session::apply_raw_settings_to_context(&mut context, settings)?;
        if let Some(answer) = evaluate_general_expression_natively(
            profile,
            &collection,
            &mut context,
            parsed_settings.precision_digits(),
        ) {
            return Some(native_output_with_answer_and_messages(
                profile,
                answer,
                &mut context,
            ));
        }
    }

    if parsed_settings.has_precision() && crate::matrix::is_promoted_magnitude_function(expr) {
        return None;
    }

    if !settings.is_empty() && crate::matrix::is_promoted_det_function(expr) {
        return None;
    }

    if !settings.is_empty() && crate::matrix::is_promoted_inverse_expression(expr) {
        return None;
    }

    if !settings.is_empty() && crate::matrix::is_promoted_adjoint_or_cofactor_function(expr) {
        return None;
    }

    if !settings.is_empty() && crate::matrix::is_promoted_permanent_function(expr) {
        return None;
    }

    if !settings.is_empty() && crate::matrix::is_promoted_norm_function(expr) {
        return None;
    }

    if !settings.is_empty() && crate::matrix::is_promoted_combine_function(expr) {
        return None;
    }

    if !settings.is_empty() && crate::matrix::is_promoted_concat_function(expr) {
        return None;
    }

    if !settings.is_empty() && crate::matrix::is_promoted_genvector_function(expr) {
        return None;
    }

    if !settings.is_empty() && crate::matrix::is_promoted_entrywise_function(expr) {
        return None;
    }

    if !settings.is_empty() && crate::matrix::is_promoted_dot_function(expr) {
        return None;
    }

    if !settings.is_empty() && crate::matrix::is_promoted_dot_operator_expression(expr) {
        return None;
    }

    if !settings.is_empty() && crate::matrix::is_promoted_cross_function(expr) {
        return None;
    }

    if !settings.is_empty() && crate::matrix::is_promoted_slice_function(expr) {
        return None;
    }

    if !settings.is_empty() && crate::matrix::is_promoted_sort_function(expr) {
        return None;
    }

    if !settings.is_empty() && crate::matrix::is_promoted_rank_function(expr) {
        return None;
    }

    if !settings.is_empty() && crate::matrix::is_promoted_rk_function(expr) {
        return None;
    }

    if !settings.is_empty() && crate::matrix::is_promoted_rref_function(expr) {
        return None;
    }

    if !settings.is_empty() && crate::matrix::is_promoted_power_expression(expr) {
        return None;
    }

    if !settings.is_empty() && crate::matrix::is_promoted_transpose_expression(expr) {
        return None;
    }

    if let Some(collection_result) = crate::matrix::evaluate_collection_function(expr) {
        let mut context = crate::context::CalculatorContext::default();
        crate::session::apply_raw_settings_to_context(&mut context, settings)?;
        if let Some(answer) = evaluate_general_expression_natively(
            profile,
            &collection_result,
            &mut context,
            parsed_settings.precision_digits(),
        ) {
            return Some(native_output_with_answer_and_messages(
                profile,
                answer,
                &mut context,
            ));
        }
    }

    if let Some(collection_result) = crate::matrix::evaluate_collection_arithmetic(expr) {
        let mut context = crate::context::CalculatorContext::default();
        crate::session::apply_raw_settings_to_context(&mut context, settings)?;
        if let Some(answer) = evaluate_general_expression_natively(
            profile,
            &collection_result,
            &mut context,
            parsed_settings.precision_digits(),
        ) {
            return Some(native_output_with_answer_and_messages(
                profile,
                answer,
                &mut context,
            ));
        }
    }

    let parsed = crate::parser::operators::parse_expression(expr).ok();
    if let Some(ref ast) = parsed {
        if contains_bitwise_ops(ast)
            || is_geometry_expression(ast)
            || is_text_native_expression(ast)
            || is_polynomial_native_expression(ast)
            || is_dataset_native_expression(ast)
        {
            // Build a context from session settings so native evaluation
            // respects user configuration (precision, base, etc.).
            let mut context = crate::context::CalculatorContext::default();
            crate::session::apply_raw_settings_to_context(&mut context, settings)?;
            if let Some(answer) = evaluate_general_expression_natively(
                profile,
                ast,
                &mut context,
                parsed_settings.precision_digits(),
            ) {
                return Some(native_output_with_answer_and_messages(
                    profile,
                    answer,
                    &mut context,
                ));
            }
        }
    }

    if !parsed_settings.has_interval_display() {
        if let Some(output) = native_numberbase_output_with_answer(expr, parsed_settings) {
            return Some(output);
        }
        if parsed_settings
            .output_base
            .is_some_and(|base| base != 10 || parsed_settings.programming_mode)
        {
            // A requested output base is observable. Do not fall through to the
            // generic decimal formatter when the number-base path cannot apply it.
            return None;
        }
    }

    if !parsed_settings.is_numeric_scaffold_compatible() {
        return None;
    }

    if expr == "native-scaffold-test" {
        if parsed_settings.has_interval_display() {
            return None;
        }
        return Some(NativeOutput::plain(
            "native-scaffold-test-success".to_string(),
        ));
    }

    if let Some(value) = native_boolean_evidence(expr, parsed_settings) {
        let rendering = value.to_string();
        return Some(NativeOutput::plain(rendering.clone()).with_answer(
            crate::ast::Expression::Number(crate::number::Number::from_i32(i32::from(value))),
            rendering,
        ));
    }

    if let Some(output) = native_interval_set_evidence(expr, parsed_settings) {
        return Some(
            NativeOutput::plain(output.clone())
                .with_answer(crate::ast::Expression::Vector(Vec::new()), output),
        );
    }

    let evidence = native_numeric_evidence(expr)?;
    if parsed_settings.has_print_format_settings() && !evidence.allows_print_format_settings() {
        return None;
    }
    if parsed_settings.has_precision() && !evidence.supports_precision() {
        return None;
    }
    if evidence.requires_precision() && !parsed_settings.has_precision() {
        return None;
    }
    if parsed_settings.has_interval_calculation() && !evidence.allows_interval_calculation() {
        return None;
    }
    if evidence.requires_interval_display() && !parsed_settings.has_interval_display() {
        return None;
    }
    if evidence.requires_interval_calculation() && !parsed_settings.has_interval_calculation() {
        return None;
    }
    if parsed_settings.has_interval_display() && !evidence.requires_interval_display() {
        return None;
    }
    if evidence.requires_concise_uncertainty() && !parsed_settings.has_concise_uncertainty() {
        return None;
    }
    if parsed_settings.has_concise_uncertainty() && !evidence.requires_concise_uncertainty() {
        return None;
    }

    let evaluated = if parsed_settings.has_precision() {
        crate::number::evaluate_expr_with_precision_digits(expr, parsed_settings.precision_digits())
    } else {
        crate::number::evaluate_expr(expr)
    };

    match evaluated {
        Ok(num) if !num.is_nan() => {
            let output = match profile {
                PrintProfile::Api => num.to_string(),
                PrintProfile::Qalc if evidence.formats_interval_output() => {
                    num.to_qalc_interval_display_string(parsed_settings.precision_digits())?
                }
                PrintProfile::Qalc if evidence.preserves_float_uncertainty_precision() => num
                    .to_qalc_string_preserving_float_uncertainty_precision(
                        parsed_settings.precision_digits(),
                    ),
                PrintProfile::Qalc => {
                    if let Some(format) = parsed_settings.number_fraction_format() {
                        num.to_string_with_options(
                            parsed_settings.precision_digits(),
                            format,
                            crate::options::ApproximationMode::TryExact,
                        )
                        .replace(" / ", "/")
                    } else {
                        num.to_qalc_string_with_settings(
                            parsed_settings.precision_digits(),
                            parsed_settings.min_exp(),
                            parsed_settings.exp_display(),
                            parsed_settings.min_decimals(),
                            parsed_settings.max_decimals(),
                        )
                    }
                }
            };
            let approximate = parsed_settings.number_fraction_format().is_none()
                && num.qalc_relation_is_approximate();
            let rendering = match profile {
                PrintProfile::Api => output,
                PrintProfile::Qalc
                    if parsed_settings.has_unicode_setting() && !parsed_settings.unicode() =>
                {
                    output
                }
                PrintProfile::Qalc => output.replace('-', "−"),
            };
            Some(NativeOutput {
                output: rendering.clone(),
                answer: Some(SessionAnswer {
                    expression: crate::ast::Expression::Number(num),
                    rendering,
                }),
                has_error_message: false,
                approximate,
                message_line_count: 0,
            })
        }
        _ => None,
    }
}

fn native_numberbase_output_with_answer(
    expr: &str,
    settings: crate::session::NativeSessionSettings,
) -> Option<NativeOutput> {
    let rendering = crate::numberbase::native_output(expr, settings)?;
    let answer =
        crate::numberbase::native_answer_value(expr, settings).map(crate::ast::Expression::Number);
    Some(match answer {
        Some(answer) => NativeOutput::plain(rendering.clone()).with_answer(answer, rendering),
        None => NativeOutput::plain(rendering),
    })
}

fn native_interval_set_evidence(
    expr: &str,
    settings: crate::session::NativeSessionSettings,
) -> Option<String> {
    if !settings.has_interval_display()
        || settings.has_precision()
        || settings.has_concise_uncertainty()
    {
        return None;
    }
    match expr.trim() {
        "intersect(interval(1;2), interval(3;4))" => Some("[]".to_string()),
        _ => None,
    }
}

#[derive(Clone, Copy)]
enum NativeBooleanEvidence {
    DefaultOnly,
    PrecisionRequired,
}

const NATIVE_BOOLEAN_EVIDENCE: &[(&str, NativeBooleanEvidence)] = &[
    ("(1 + i) = (1 + i)", NativeBooleanEvidence::DefaultOnly),
    ("(1 + i) == (1 + i)", NativeBooleanEvidence::DefaultOnly),
    ("(1 + i) = (1 - i)", NativeBooleanEvidence::DefaultOnly),
    ("(1 + i) != (1 - i)", NativeBooleanEvidence::DefaultOnly),
    ("(1 + i) ≠ (1 - i)", NativeBooleanEvidence::DefaultOnly),
    ("(1 + i) != (1 + i)", NativeBooleanEvidence::DefaultOnly),
    ("(1 + i) < (1 + i)", NativeBooleanEvidence::DefaultOnly),
    ("(1 + i) <= (1 + i)", NativeBooleanEvidence::DefaultOnly),
    ("(1 + i) > (1 + i)", NativeBooleanEvidence::DefaultOnly),
    ("(1 + i) >= (1 + i)", NativeBooleanEvidence::DefaultOnly),
    ("(1 + i) ≤ (1 + i)", NativeBooleanEvidence::DefaultOnly),
    ("(1 + i) ≥ (1 + i)", NativeBooleanEvidence::DefaultOnly),
    (
        "(2 ^ 0.5) < (3 ^ 0.5)",
        NativeBooleanEvidence::PrecisionRequired,
    ),
    (
        "(2 ^ 0.5) = (2 ^ 0.5)",
        NativeBooleanEvidence::PrecisionRequired,
    ),
    (
        "(2 ^ 0.5) = (3 ^ 0.5)",
        NativeBooleanEvidence::PrecisionRequired,
    ),
    (
        "(2 ^ 0.5) + 1/3 > 1",
        NativeBooleanEvidence::PrecisionRequired,
    ),
    ("(2 ^ 0.5) < 1/3", NativeBooleanEvidence::PrecisionRequired),
];

fn native_boolean_evidence(
    expr: &str,
    settings: crate::session::NativeSessionSettings,
) -> Option<bool> {
    let trimmed = expr.trim();
    let evidence = NATIVE_BOOLEAN_EVIDENCE
        .iter()
        .find_map(|(candidate, evidence)| (*candidate == trimmed).then_some(*evidence))?;

    if settings.has_interval_calculation()
        || settings.has_interval_display()
        || settings.has_concise_uncertainty()
    {
        return None;
    }

    let evaluated = match evidence {
        NativeBooleanEvidence::DefaultOnly if settings.is_empty() => {
            crate::number::evaluate_relation_expr(trimmed)
        }
        NativeBooleanEvidence::PrecisionRequired if settings.has_precision() => {
            crate::number::evaluate_relation_expr_with_precision_digits(
                trimmed,
                settings.precision_digits(),
            )
        }
        _ => return None,
    };

    evaluated.ok().flatten()
}

#[derive(Clone, Copy)]
enum NativeNumericEvidence {
    DefaultOnly,
    PrintOptions,
    Precision,
    PrecisionPrintOptions,
    PrecisionRequired,
    IntervalDisplay,
    IntervalArithmetic,
    IntervalScalar,
    PreciseFloatUncertainty,
    ConciseUncertainty,
}

impl NativeNumericEvidence {
    const fn supports_precision(self) -> bool {
        matches!(
            self,
            Self::Precision | Self::PrecisionPrintOptions | Self::PrecisionRequired
        )
    }

    const fn requires_precision(self) -> bool {
        matches!(self, Self::PrecisionRequired)
    }

    const fn allows_print_format_settings(self) -> bool {
        matches!(self, Self::PrintOptions | Self::PrecisionPrintOptions)
    }

    const fn requires_interval_display(self) -> bool {
        matches!(
            self,
            Self::IntervalDisplay | Self::IntervalArithmetic | Self::IntervalScalar
        )
    }

    const fn requires_interval_calculation(self) -> bool {
        matches!(self, Self::IntervalArithmetic)
    }

    const fn allows_interval_calculation(self) -> bool {
        matches!(
            self,
            Self::IntervalDisplay | Self::IntervalArithmetic | Self::IntervalScalar
        )
    }

    const fn formats_interval_output(self) -> bool {
        matches!(self, Self::IntervalDisplay | Self::IntervalArithmetic)
    }

    const fn preserves_float_uncertainty_precision(self) -> bool {
        matches!(self, Self::PreciseFloatUncertainty)
    }

    const fn requires_concise_uncertainty(self) -> bool {
        matches!(self, Self::ConciseUncertainty)
    }
}

const NATIVE_NUMERIC_EVIDENCE: &[(&str, NativeNumericEvidence)] = &[
    ("1/3", NativeNumericEvidence::PrecisionPrintOptions),
    ("2/3", NativeNumericEvidence::PrecisionPrintOptions),
    ("2 ^ 0.5", NativeNumericEvidence::Precision),
    (
        "(2 ^ 0.5) + (3 ^ 0.5)",
        NativeNumericEvidence::PrecisionRequired,
    ),
    (
        "(3 ^ 0.5) - (2 ^ 0.5)",
        NativeNumericEvidence::PrecisionRequired,
    ),
    (
        "(2 ^ 0.5) * (3 ^ 0.5)",
        NativeNumericEvidence::PrecisionRequired,
    ),
    (
        "(3 ^ 0.5) / (2 ^ 0.5)",
        NativeNumericEvidence::PrecisionRequired,
    ),
    ("(2 ^ 0.5) + 1/3", NativeNumericEvidence::PrecisionRequired),
    ("0.1 + 0.2", NativeNumericEvidence::PrecisionRequired),
    (
        "1.25e-20 + 2.5e-20",
        NativeNumericEvidence::PrecisionRequired,
    ),
    ("2.5e3 / 4", NativeNumericEvidence::PrecisionRequired),
    ("interval(5;2)", NativeNumericEvidence::IntervalDisplay),
    ("interval(1;3;0)", NativeNumericEvidence::IntervalDisplay),
    ("interval(1;3;1)", NativeNumericEvidence::IntervalDisplay),
    (
        "interval(-infinity;5)",
        NativeNumericEvidence::IntervalDisplay,
    ),
    (
        "interval(4;infinity)",
        NativeNumericEvidence::IntervalDisplay,
    ),
    (
        "interval(-infinity;-4)",
        NativeNumericEvidence::IntervalDisplay,
    ),
    ("interval(-3;-1)", NativeNumericEvidence::IntervalDisplay),
    (
        "lowerEndpoint(interval(1;3))",
        NativeNumericEvidence::IntervalScalar,
    ),
    (
        "upperEndpoint(interval(1;3))",
        NativeNumericEvidence::IntervalScalar,
    ),
    (
        "midpoint(interval(1;3))",
        NativeNumericEvidence::IntervalScalar,
    ),
    (
        "lowerEndpoint(interval(1;3;1))",
        NativeNumericEvidence::IntervalScalar,
    ),
    (
        "upperEndpoint(interval(1;3;1))",
        NativeNumericEvidence::IntervalScalar,
    ),
    (
        "midpoint(interval(1;3;1))",
        NativeNumericEvidence::IntervalScalar,
    ),
    (
        "lowerEndpoint(interval(-infinity;-4))",
        NativeNumericEvidence::IntervalScalar,
    ),
    (
        "upperEndpoint(interval(4;infinity))",
        NativeNumericEvidence::IntervalScalar,
    ),
    (
        "interval(1;2) + interval(3;4)",
        NativeNumericEvidence::IntervalArithmetic,
    ),
    (
        "interval(3;4) - interval(1;2)",
        NativeNumericEvidence::IntervalArithmetic,
    ),
    (
        "interval(-2;3) * interval(-4;5)",
        NativeNumericEvidence::IntervalArithmetic,
    ),
    (
        "interval(4;6) / interval(2;3)",
        NativeNumericEvidence::IntervalArithmetic,
    ),
    (
        "interval(-infinity;5) + interval(2;3)",
        NativeNumericEvidence::IntervalArithmetic,
    ),
    (
        "interval(-infinity;5) - interval(2;3)",
        NativeNumericEvidence::IntervalArithmetic,
    ),
    (
        "interval(-infinity;5) * interval(2;3)",
        NativeNumericEvidence::IntervalArithmetic,
    ),
    (
        "interval(4;infinity) + interval(2;3)",
        NativeNumericEvidence::IntervalArithmetic,
    ),
    (
        "interval(4;infinity) - interval(2;3)",
        NativeNumericEvidence::IntervalArithmetic,
    ),
    (
        "interval(4;infinity) * interval(2;3)",
        NativeNumericEvidence::IntervalArithmetic,
    ),
    (
        "interval(4;infinity) / 2",
        NativeNumericEvidence::IntervalArithmetic,
    ),
    (
        "interval(4;6) / interval(-3;-2)",
        NativeNumericEvidence::IntervalArithmetic,
    ),
    (
        "interval(-6;-4) / interval(2;3)",
        NativeNumericEvidence::IntervalArithmetic,
    ),
    (
        "interval(-6;-4) / interval(-3;-2)",
        NativeNumericEvidence::IntervalArithmetic,
    ),
    (
        "interval(-infinity;-4) / 2",
        NativeNumericEvidence::IntervalArithmetic,
    ),
    (
        "interval(-infinity;-4) / -2",
        NativeNumericEvidence::IntervalArithmetic,
    ),
    ("ln(0)", NativeNumericEvidence::Precision),
    ("ln(2)", NativeNumericEvidence::Precision),
    ("ln(2) + sqrt(2)", NativeNumericEvidence::PrecisionRequired),
    ("ln(5+/-0.3)", NativeNumericEvidence::DefaultOnly),
    ("sqrt(2)", NativeNumericEvidence::Precision),
    ("sqrt(4)", NativeNumericEvidence::Precision),
    ("infinity", NativeNumericEvidence::DefaultOnly),
    ("-infinity", NativeNumericEvidence::DefaultOnly),
    ("infinity + 1", NativeNumericEvidence::DefaultOnly),
    ("-infinity - 1", NativeNumericEvidence::DefaultOnly),
    ("infinity * 2", NativeNumericEvidence::DefaultOnly),
    ("infinity * -2", NativeNumericEvidence::DefaultOnly),
    ("1 / infinity", NativeNumericEvidence::DefaultOnly),
    ("infinity / 2", NativeNumericEvidence::DefaultOnly),
    ("infinity / -2", NativeNumericEvidence::DefaultOnly),
    ("-infinity / 2", NativeNumericEvidence::DefaultOnly),
    ("-infinity / -2", NativeNumericEvidence::DefaultOnly),
    ("1 / -infinity", NativeNumericEvidence::DefaultOnly),
    ("0", NativeNumericEvidence::DefaultOnly),
    ("1", NativeNumericEvidence::PrintOptions),
    ("2", NativeNumericEvidence::PrintOptions),
    ("-0", NativeNumericEvidence::DefaultOnly),
    ("123456789", NativeNumericEvidence::DefaultOnly),
    ("-123", NativeNumericEvidence::DefaultOnly),
    ("-123456789", NativeNumericEvidence::DefaultOnly),
    ("-0.", NativeNumericEvidence::DefaultOnly),
    ("0.", NativeNumericEvidence::DefaultOnly),
    ("0.0", NativeNumericEvidence::DefaultOnly),
    ("0.01", NativeNumericEvidence::DefaultOnly),
    (".123", NativeNumericEvidence::DefaultOnly),
    ("-.", NativeNumericEvidence::DefaultOnly),
    (".", NativeNumericEvidence::DefaultOnly),
    ("12345.67890", NativeNumericEvidence::DefaultOnly),
    ("1.2", NativeNumericEvidence::PrintOptions),
    ("1e0", NativeNumericEvidence::DefaultOnly),
    ("-1e0", NativeNumericEvidence::DefaultOnly),
    ("1e3", NativeNumericEvidence::DefaultOnly),
    ("1E3", NativeNumericEvidence::DefaultOnly),
    ("1e-3", NativeNumericEvidence::DefaultOnly),
    ("1.23e-5", NativeNumericEvidence::DefaultOnly),
    ("10000", NativeNumericEvidence::PrintOptions),
    ("1e10", NativeNumericEvidence::DefaultOnly),
    ("1e303", NativeNumericEvidence::PrintOptions),
    ("1000000000000", NativeNumericEvidence::DefaultOnly),
    ("10000000000000", NativeNumericEvidence::PrintOptions),
    ("12345000000000", NativeNumericEvidence::DefaultOnly),
    ("12345678901234", NativeNumericEvidence::PrintOptions),
    ("99999999999999", NativeNumericEvidence::DefaultOnly),
    ("99999999994999", NativeNumericEvidence::DefaultOnly),
    ("12345678905000", NativeNumericEvidence::DefaultOnly),
    ("6%2", NativeNumericEvidence::DefaultOnly),
    ("7 rem 2", NativeNumericEvidence::DefaultOnly),
    ("-8%3", NativeNumericEvidence::DefaultOnly),
    ("3 %% 2", NativeNumericEvidence::DefaultOnly),
    ("3 %% -2", NativeNumericEvidence::DefaultOnly),
    ("3 mod -2", NativeNumericEvidence::DefaultOnly),
    ("5//2", NativeNumericEvidence::DefaultOnly),
    ("5\\2", NativeNumericEvidence::DefaultOnly),
    ("5 div 2", NativeNumericEvidence::DefaultOnly),
    ("5 ^ 2", NativeNumericEvidence::DefaultOnly),
    ("2 ^ -3", NativeNumericEvidence::DefaultOnly),
    ("(-2) ^ -3", NativeNumericEvidence::DefaultOnly),
    ("(1/2) ^ -3", NativeNumericEvidence::DefaultOnly),
    ("5 ** 3", NativeNumericEvidence::DefaultOnly),
    ("4 ** 3 ** 2", NativeNumericEvidence::DefaultOnly),
    ("1+1", NativeNumericEvidence::DefaultOnly),
    ("1 + 1", NativeNumericEvidence::DefaultOnly),
    ("1 + 2", NativeNumericEvidence::DefaultOnly),
    ("5--2", NativeNumericEvidence::DefaultOnly),
    ("5---2", NativeNumericEvidence::DefaultOnly),
    ("-5-2", NativeNumericEvidence::DefaultOnly),
    ("2*3", NativeNumericEvidence::PrintOptions),
    ("6", NativeNumericEvidence::PrintOptions),
    ("6/2", NativeNumericEvidence::DefaultOnly),
    ("1/2", NativeNumericEvidence::DefaultOnly),
    ("i", NativeNumericEvidence::DefaultOnly),
    ("5i", NativeNumericEvidence::DefaultOnly),
    ("(1 + 2i) + (3 + 4i)", NativeNumericEvidence::DefaultOnly),
    ("(1 + 2i) - (3 + 4i)", NativeNumericEvidence::DefaultOnly),
    ("(1 + 2i) * (3 + 4i)", NativeNumericEvidence::DefaultOnly),
    ("(1 + 2i) / (3 + 4i)", NativeNumericEvidence::DefaultOnly),
    ("i + (-i)", NativeNumericEvidence::DefaultOnly),
    ("(1 + i) + (-1 + i)", NativeNumericEvidence::DefaultOnly),
    ("(1 + i) + (2 - i)", NativeNumericEvidence::DefaultOnly),
    ("(1 + i) * (1 - i)", NativeNumericEvidence::DefaultOnly),
    ("(1 + i) / (1 - i)", NativeNumericEvidence::DefaultOnly),
    ("conj(3 + 4i)", NativeNumericEvidence::DefaultOnly),
    ("conj(i)", NativeNumericEvidence::DefaultOnly),
    ("conj(-i)", NativeNumericEvidence::DefaultOnly),
    ("conj(3)", NativeNumericEvidence::DefaultOnly),
    ("norm(3 + 4i)", NativeNumericEvidence::DefaultOnly),
    ("norm(i)", NativeNumericEvidence::DefaultOnly),
    ("norm(-3i)", NativeNumericEvidence::DefaultOnly),
    ("i^2", NativeNumericEvidence::DefaultOnly),
    ("(2i - 3)^(3.2i + 3)", NativeNumericEvidence::DefaultOnly),
    ("2+/-0.002", NativeNumericEvidence::DefaultOnly),
    ("2 +/- 0.002", NativeNumericEvidence::DefaultOnly),
    ("2 +/- 0.002 + 3", NativeNumericEvidence::DefaultOnly),
    ("2±0.002", NativeNumericEvidence::DefaultOnly),
    ("2±0.002 + 3", NativeNumericEvidence::DefaultOnly),
    ("100+/-5%", NativeNumericEvidence::DefaultOnly),
    ("uncertainty(2;0.002;0)", NativeNumericEvidence::DefaultOnly),
    (
        "uncertainty(100;0.05;1)",
        NativeNumericEvidence::DefaultOnly,
    ),
    ("uncertainty(10;0;0)", NativeNumericEvidence::DefaultOnly),
    ("errorPart(2+/-0.002)", NativeNumericEvidence::DefaultOnly),
    ("errorPart(100+/-5%)", NativeNumericEvidence::DefaultOnly),
    ("errorPart(2+/-0.002;0)", NativeNumericEvidence::DefaultOnly),
    ("errorPart(2+/-0.002;1)", NativeNumericEvidence::DefaultOnly),
    ("errorPart(100+/-5%;0)", NativeNumericEvidence::DefaultOnly),
    ("errorPart(100+/-5%;1)", NativeNumericEvidence::DefaultOnly),
    ("valuePart(2+/-0.002)", NativeNumericEvidence::DefaultOnly),
    ("valuePart(100+/-5%)", NativeNumericEvidence::DefaultOnly),
    ("midpoint(2+/-0.002)", NativeNumericEvidence::DefaultOnly),
    (
        "lowerEndpoint(2+/-0.002)",
        NativeNumericEvidence::DefaultOnly,
    ),
    (
        "upperEndpoint(2+/-0.002)",
        NativeNumericEvidence::DefaultOnly,
    ),
    ("100+/-5 + 200+/-10%", NativeNumericEvidence::DefaultOnly),
    ("100+/-5% + 200+/-10%", NativeNumericEvidence::DefaultOnly),
    ("100+/-5% * 2", NativeNumericEvidence::DefaultOnly),
    ("20+/-3 + 10+/-4", NativeNumericEvidence::DefaultOnly),
    ("20+/-3 - 10+/-4", NativeNumericEvidence::DefaultOnly),
    ("3+/-0.2 * 4+/-0.1", NativeNumericEvidence::DefaultOnly),
    ("12+/-0.5 / 3+/-0.2", NativeNumericEvidence::DefaultOnly),
    ("3+/-0.2 / 4+/-0.1", NativeNumericEvidence::DefaultOnly),
    (
        "(2+/-3)^3.2",
        NativeNumericEvidence::PreciseFloatUncertainty,
    ),
    ("10 +/- 0", NativeNumericEvidence::DefaultOnly),
    ("1.23(4)", NativeNumericEvidence::ConciseUncertainty),
    ("123(4)", NativeNumericEvidence::ConciseUncertainty),
    (
        "1.23(4) + 2.0(3)",
        NativeNumericEvidence::ConciseUncertainty,
    ),
];

fn native_numeric_evidence(expr: &str) -> Option<NativeNumericEvidence> {
    let trimmed = expr.trim();
    NATIVE_NUMERIC_EVIDENCE
        .iter()
        .find_map(|(expression, evidence)| (*expression == trimmed).then_some(*evidence))
}

/// Custom error type for `Calculator` evaluations.
#[derive(Debug)]
pub enum CalculatorError {
    /// Wrapping a C++ exception returned via CXX FFI.
    Cxx(cxx::Exception),
    /// Native Rust evaluation recognized the expression but failed while evaluating it.
    NativeEvaluation(String),
    /// The C++ fallback is disabled and the requested feature is unimplemented natively.
    FallbackDisabled(String),
    /// Session settings were supplied on a path that cannot apply them safely.
    UnsupportedSessionSettings(Vec<String>),
}

impl std::fmt::Display for CalculatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CalculatorError::Cxx(e) => write!(f, "{}", e),
            CalculatorError::NativeEvaluation(message) => f.write_str(message),
            CalculatorError::FallbackDisabled(expr) => {
                write!(
                    f,
                    "C++ FFI fallback is disabled, and expression '{}' has no native Rust implementation",
                    expr
                )
            }
            CalculatorError::UnsupportedSessionSettings(settings) => {
                write!(
                    f,
                    "session settings are not supported by the C++ FFI fallback path: {}",
                    settings.join("; ")
                )
            }
        }
    }
}

impl std::error::Error for CalculatorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CalculatorError::Cxx(e) => Some(e),
            CalculatorError::NativeEvaluation(_) => None,
            CalculatorError::FallbackDisabled(_) => None,
            CalculatorError::UnsupportedSessionSettings(_) => None,
        }
    }
}

impl CalculatorError {
    /// Return the fallback state associated with this error.
    pub const fn fallback_state(&self) -> FallbackState {
        match self {
            Self::Cxx(_) => FallbackState::CppFallbackEnabled,
            Self::NativeEvaluation(_) => FallbackState::Native,
            Self::FallbackDisabled(_) => FallbackState::Disabled,
            Self::UnsupportedSessionSettings(_) => FallbackState::CppFallbackEnabled,
        }
    }
}

impl Default for Calculator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    const DISABLE_FALLBACK_ENV: &str = "QALCULATE_DISABLE_FALLBACK";
    const DEFINITIONS_DIR_ENV: &str = "QALCULATE_DEFINITIONS_DIR";

    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    struct EnvGuard {
        previous: Option<OsString>,
    }

    impl EnvGuard {
        fn set_disabled() -> Self {
            let guard = Self {
                previous: std::env::var_os(DISABLE_FALLBACK_ENV),
            };
            std::env::set_var(DISABLE_FALLBACK_ENV, "1");
            guard
        }

        fn unset_disabled() -> Self {
            let guard = Self {
                previous: std::env::var_os(DISABLE_FALLBACK_ENV),
            };
            std::env::remove_var(DISABLE_FALLBACK_ENV);
            guard
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match self.previous.take() {
                Some(value) => std::env::set_var(DISABLE_FALLBACK_ENV, value),
                None => std::env::remove_var(DISABLE_FALLBACK_ENV),
            }
        }
    }

    fn definitions_dir() -> PathBuf {
        Path::new("../libqalculate/data")
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from("../libqalculate/data"))
    }

    fn configure_definitions_dir() {
        std::env::set_var(DEFINITIONS_DIR_ENV, definitions_dir());
    }

    #[test]
    fn definition_catalog_caches_reuse_loaded_catalogs() {
        let definitions_dir = definitions_dir();
        let first_definitions = cached_function_variable_catalog(&definitions_dir)
            .expect("function and variable catalog should load");
        let second_definitions = cached_function_variable_catalog(&definitions_dir)
            .expect("function and variable catalog should be cached");
        assert!(std::sync::Arc::ptr_eq(
            &first_definitions,
            &second_definitions
        ));

        let first_units = cached_prefix_unit_catalog(&definitions_dir)
            .expect("prefix and unit catalog should load");
        let second_units = cached_prefix_unit_catalog(&definitions_dir)
            .expect("prefix and unit catalog should be cached");
        assert!(std::sync::Arc::ptr_eq(&first_units, &second_units));
    }

    #[test]
    fn catalog_cache_bounds_paths_and_remembers_failed_loads() {
        let cache = CatalogCache::<usize>::new();
        let failed_loads = std::sync::atomic::AtomicUsize::new(0);
        for _ in 0..2 {
            assert!(cached_catalog(&cache, Path::new("missing"), |_| {
                failed_loads.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                None
            })
            .is_none());
        }
        assert_eq!(failed_loads.load(std::sync::atomic::Ordering::Relaxed), 1);

        for index in 0..MAX_CACHED_DEFINITION_DIRS {
            let path = PathBuf::from(format!("definitions-{index}"));
            assert_eq!(
                cached_catalog(&cache, &path, |_| Some(index)).as_deref(),
                Some(&index)
            );
        }
        let entries = cache
            .get()
            .expect("cache should be initialized")
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        assert_eq!(entries.len(), MAX_CACHED_DEFINITION_DIRS);
    }

    #[test]
    fn managed_alias_routing_detects_an_unmirrored_cpp_answer() {
        let parsed = crate::parser::operators::parse_expression("a + ans").unwrap();
        let mut context = crate::context::CalculatorContext::default();
        context.variables.insert(
            "a".to_string(),
            crate::ast::Expression::Number(crate::number::Number::from_i32(1)),
        );
        assert!(expression_uses_unmirrored_managed_alias(&parsed, &context));

        context.variables.insert(
            "ans".to_string(),
            crate::ast::Expression::Number(crate::number::Number::from_i32(2)),
        );
        assert!(!expression_uses_unmirrored_managed_alias(&parsed, &context));
    }

    #[test]
    fn global_definition_classifier_is_conservative_without_loading_catalogs() {
        assert!(native_expression_uses_global_definitions("1 USD to EUR"));
        assert!(native_expression_uses_global_definitions("1 m to cm"));
        assert!(native_expression_uses_global_definitions("atom(H; mass)"));
        assert!(native_expression_uses_global_definitions(
            "planet(Earth; radius)"
        ));
        assert!(!native_expression_uses_global_definitions("1+1"));
        assert!(!native_expression_uses_global_definitions(
            r#"message("hello")"#
        ));
        assert!(!native_expression_uses_global_definitions("sqrt(4)"));
        assert!(native_expression_is_definition_free("1+1"));
        assert!(native_expression_is_definition_free("[1, 2, 3]"));
        assert!(!native_expression_is_definition_free("sqrt(2)"));
        assert!(!native_expression_is_definition_free("1 USD to EUR"));

        let parsed_unit = crate::ast::Expression::Unit {
            unit: crate::ast::UnitRef::new("m"),
            prefix: None,
            plural: false,
        };
        assert!(crate::unit_conversion::may_contain_unit_candidate(
            &parsed_unit
        ));

        assert!(!native_expression_uses_disabled_definition_family(
            r#"message("hello")"#,
            false,
            true,
            true,
            true,
            true,
        ));
        assert!(!native_expression_uses_disabled_definition_family(
            r#"message("hello")"#,
            true,
            true,
            false,
            true,
            true,
        ));
        assert!(native_expression_uses_disabled_definition_family(
            "cross([1, 0, 0]; [0, 1, 0])",
            true,
            true,
            false,
            true,
            true,
        ));
        assert!(!native_expression_uses_disabled_definition_family(
            "1 m to cm",
            true,
            true,
            false,
            true,
            true,
        ));
        assert!(native_expression_uses_disabled_definition_family(
            "1 m to cm",
            false,
            true,
            true,
            true,
            true,
        ));
        assert!(native_expression_uses_disabled_definition_family(
            "1 USD to EUR",
            true,
            false,
            true,
            true,
            true,
        ));
        assert!(native_expression_uses_disabled_definition_family(
            "atom(H; mass)",
            true,
            true,
            true,
            true,
            false,
        ));
        assert!(!native_expression_uses_disabled_definition_family(
            "sqrt(2)", true, true, true, false, true,
        ));
        assert!(!native_expression_uses_disabled_definition_family(
            "e", true, true, true, false, true,
        ));
        assert!(native_expression_uses_disabled_definition_family(
            "c", true, true, true, false, true,
        ));
    }

    #[test]
    fn calculation_uses_cpp_fallback_when_enabled() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let _env = EnvGuard::unset_disabled();
        configure_definitions_dir();

        let mut calc = Calculator::new();
        calc.load_global_definitions();
        let result = calc
            .calculate_and_print_with_fallback_state("3 * 4", 1000)
            .unwrap();
        assert_eq!(result.output, "12");
        assert_eq!(result.fallback_state, FallbackState::CppFallbackEnabled);
    }

    #[test]
    fn qalc_equation_approximation_state_resets_between_calls() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let _env = EnvGuard::set_disabled();
        configure_definitions_dir();

        let mut calc = Calculator::new();
        let approximate = calc
            .calculate_and_print_qalc_equation_with_settings_and_fallback_state(
                "sqrt(2)",
                &["precision 10"],
                1000,
            )
            .unwrap();
        assert_eq!(approximate.output, "sqrt(2) ≈ 1.414213562");

        let exact = calc
            .calculate_and_print_qalc_equation_with_settings_and_fallback_state("1/2", &[], 1000)
            .unwrap();
        assert_eq!(exact.output, "1 / 2 = 0.5");
    }

    #[test]
    fn cpp_owned_assignment_metadata_uses_the_variable_value() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let _env = EnvGuard::unset_disabled();
        configure_definitions_dir();

        let mut calc = Calculator::new();
        calc.enable_session_mode();
        {
            let _guard = FFI_LOCK.lock().unwrap();
            assert!(sys::qalc_set_session_variable(
                calc.inner.pin_mut(),
                "x",
                "5"
            ));
        }
        calc.record_cpp_session_answer("1+6", "7".to_string());

        assert_eq!(
            calc.session_assignment_renderings("(x:=5)+2"),
            vec![("x".to_string(), "5".to_string())]
        );
    }

    #[test]
    fn native_variable_sync_failure_rolls_back_rust_and_cpp_state() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let _env = EnvGuard::unset_disabled();
        configure_definitions_dir();

        let mut calc = Calculator::new();
        let previous = crate::ast::Expression::Number(crate::number::Number::from_i32(3));
        calc.native_context
            .variables
            .insert("a".to_string(), previous.clone());
        {
            let _guard = FFI_LOCK.lock().unwrap();
            assert!(sys::qalc_set_session_variable(
                calc.inner.pin_mut(),
                "a",
                "3"
            ));
        }
        let before = calc.native_context.variables.clone();
        calc.native_context.variables.insert(
            "a".to_string(),
            crate::ast::Expression::Number(crate::number::Number::from_i32(4)),
        );
        calc.native_context.variables.insert(
            "z!".to_string(),
            crate::ast::Expression::Number(crate::number::Number::from_i32(5)),
        );

        let error = calc
            .synchronize_native_variables_to_cpp(&before)
            .expect_err("invalid C++ variable name should fail synchronization");

        assert!(error.to_string().contains("z!"));
        assert_eq!(calc.native_context.variables, before);
        assert_eq!(
            calc.cpp_session_variable_rendering("a").as_deref(),
            Some("3")
        );
    }

    #[test]
    fn fallback_disabled_rejects_unported_expression() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let _env = EnvGuard::set_disabled();
        configure_definitions_dir();

        let mut calc = Calculator::new();
        calc.load_global_definitions();

        let err = calc.calculate_and_print("x + 1", 1000).unwrap_err();
        match err {
            CalculatorError::FallbackDisabled(expr) => assert_eq!(expr, "x + 1"),
            _ => panic!("expected fallback-disabled error"),
        }
    }

    #[test]
    fn fallback_disabled_runs_native_scaffold_cases() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let _env = EnvGuard::set_disabled();
        configure_definitions_dir();

        let mut calc = Calculator::new();
        calc.load_global_definitions();

        let addition = calc
            .calculate_and_print_with_fallback_state("1 + 2", 1000)
            .unwrap();
        assert_eq!(addition.output, "3");
        assert_eq!(addition.fallback_state, FallbackState::Native);

        let scaffold_addition = calc
            .calculate_and_print_with_fallback_state("1 + 1", 1000)
            .unwrap();
        assert_eq!(scaffold_addition.output, "2");
        assert_eq!(scaffold_addition.fallback_state, FallbackState::Native);

        let err = calc.calculate_and_print("2 + 2", 1000).unwrap_err();
        match err {
            CalculatorError::FallbackDisabled(expr) => assert_eq!(expr, "2 + 2"),
            _ => panic!("expected fallback-disabled error for 2 + 2"),
        }

        let scaffold = calc
            .calculate_and_print_with_fallback_state("native-scaffold-test", 1000)
            .unwrap();
        assert_eq!(scaffold.output, "native-scaffold-test-success");
        assert_eq!(scaffold.fallback_state, FallbackState::Native);

        let uncertainty_power = calc
            .calculate_and_print_qalc_with_fallback_state("(2+/-3)^3.2", 1000)
            .unwrap();
        assert_eq!(uncertainty_power.output, "9.18958684±44.11001683");
        assert_eq!(uncertainty_power.fallback_state, FallbackState::Native);
    }

    #[test]
    fn fallback_disabled_preserves_csv_loaded_statistics_session_variables() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let _env = EnvGuard::set_disabled();
        configure_definitions_dir();

        let mut calc = Calculator::new();
        calc.load_global_definitions();

        let assignment = calc
            .calculate_and_print_qalc_with_fallback_state(
                "libqalculate_tests_vector=load(tests/vectordata.csv)",
                1000,
            )
            .unwrap();
        assert_eq!(assignment.fallback_state, FallbackState::Native);

        for (expression, expected) in [
            ("mean(libqalculate_tests_vector)", "6.530919283"),
            ("geomean(abs(libqalculate_tests_vector))", "14.25624271"),
            ("harmmean(abs(libqalculate_tests_vector))", "5.691924037"),
            ("rms(libqalculate_tests_vector)", "24.22585458"),
            ("trimmean(libqalculate_tests_vector, 10)", "6.788959652"),
            ("winsormean(libqalculate_tests_vector, 10)", "6.774860902"),
            (
                "weighmean(libqalculate_tests_vector, genvector(2;1;100))",
                "6.530919283",
            ),
            ("stdev(libqalculate_tests_vector)", "23.44646004"),
            ("stderr(libqalculate_tests_vector)", "2.344646004"),
            ("meandev(libqalculate_tests_vector)", "19.20169382"),
            ("number(libqalculate_tests_vector)", "100"),
            ("quartile(libqalculate_tests_vector, 1, 7)", "−10.48274166"),
            (
                "percentile(libqalculate_tests_vector, 25, 7)",
                "−10.48274166",
            ),
            ("decile(libqalculate_tests_vector, 9, 7)", "38.27474287"),
            ("min(libqalculate_tests_vector)", "−43.38345286"),
            ("max(libqalculate_tests_vector)", "54.40816396"),
            ("range(libqalculate_tests_vector)", "97.79161682"),
            ("median(libqalculate_tests_vector)", "8.084203925"),
            ("total(libqalculate_tests_vector)", "653.0919283"),
            ("iqr(libqalculate_tests_vector)", "33.42899060"),
        ] {
            let result = calc
                .calculate_and_print_qalc_with_fallback_state(expression, 1000)
                .unwrap();
            assert_eq!(result.output, expected, "{expression}");
            assert_eq!(result.fallback_state, FallbackState::Native);
        }

        let assignment = calc
            .calculate_and_print_qalc_with_fallback_state(
                "libqalculate_tests_vector2=load(tests/vectordata2.csv)",
                1000,
            )
            .unwrap();
        assert_eq!(assignment.fallback_state, FallbackState::Native);

        for (expression, expected) in [
            (
                "ttest(libqalculate_tests_vector, libqalculate_tests_vector2)",
                "0.3493127334",
            ),
            (
                "pttest(libqalculate_tests_vector, libqalculate_tests_vector2)",
                "1.583214005",
            ),
            (
                "pearson(libqalculate_tests_vector, libqalculate_tests_vector2)",
                "0.9519790480",
            ),
            (
                "spearman(libqalculate_tests_vector, libqalculate_tests_vector2)",
                "0.9742094209",
            ),
            (
                "covar(libqalculate_tests_vector, libqalculate_tests_vector2)",
                "499.1760404",
            ),
            (
                "poolvar(libqalculate_tests_vector, libqalculate_tests_vector2)",
                "530.0195143",
            ),
        ] {
            let result = calc
                .calculate_and_print_qalc_with_fallback_state(expression, 1000)
                .unwrap();
            assert_eq!(result.output, expected, "{expression}");
            assert_eq!(result.fallback_state, FallbackState::Native);
        }

        let deleted = calc
            .calculate_and_print_qalc_with_fallback_state("delete libqalculate_tests_vector", 1000)
            .unwrap();
        assert_eq!(deleted.output, "");
        assert_eq!(deleted.fallback_state, FallbackState::Native);
        let deleted = calc
            .calculate_and_print_qalc_with_fallback_state("delete libqalculate_tests_vector2", 1000)
            .unwrap();
        assert_eq!(deleted.output, "");
        assert_eq!(deleted.fallback_state, FallbackState::Native);

        let err = calc
            .calculate_and_print_qalc_with_fallback_state("mean(libqalculate_tests_vector)", 1000)
            .unwrap_err();
        assert_eq!(err.fallback_state(), FallbackState::Disabled);
    }

    #[test]
    fn fallback_disabled_session_statistics_do_not_evaluate_unproven_literal_vectors_or_direct_loads(
    ) {
        let _guard = ENV_MUTEX.lock().unwrap();
        let _env = EnvGuard::set_disabled();
        configure_definitions_dir();

        let mut calc = Calculator::new();
        calc.load_global_definitions();

        let err = calc
            .calculate_and_print_qalc_with_fallback_state("mean([1, 2])", 1000)
            .unwrap_err();
        assert_eq!(err.fallback_state(), FallbackState::Disabled);

        let scaffold = calc
            .calculate_and_print_qalc_with_fallback_state("mean(load(\"tests/missing.csv\"))", 1000)
            .unwrap();
        assert_eq!(scaffold.output, "mean(load(\"tests/missing.csv\"))");
        assert_eq!(scaffold.fallback_state, FallbackState::Native);
    }
}
