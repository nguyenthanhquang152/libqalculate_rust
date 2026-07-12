# Public API Parity Matrix

> Upstream version: libqalculate 5.11.0
> Scope: public header inventory plus the Task 12.5 native Rust API mapping.

## Extraction Method

The Task 0.2 parity matrix classified public API declarations from the required upstream headers by header and API family. Task 12.5 adds a stable native `Calculator` façade and a more specific category mapping. Each public symbol in a listed header inherits the most specific category row below; declarations not split into a category inherit their header/family row. This keeps every required public declaration classified without treating a representative method as completion of an entire C++ class.

`native-pass` means the named Rust-owned category is implemented and tested with no C++ object or fallback. FFI-only rows and `fallback-only` categories name the actual transitional wrapper and cannot be counted as `native-pass`. `unstarted` and the pending portion of `mixed:` rows remain explicit gaps. Rust ownership, `Result`, and per-session state are the accepted Task 12.5 API mapping; they do not approve a user-visible behavior mismatch.

## Required Header Matrix

| # | Upstream Header | Public Symbol Scope | Rust Owner | Status | FFI-only Native? | Follow-up |
|---|---|---|---|---|---|---|
| 1 | `qalculate.h` | Umbrella include surface | crate root and public modules | mixed: `native-pass`, `fallback-only`, `unstarted` | no | category rows below |
| 2 | `Calculator.h` | Calculator, messages, options, plotting, expression evaluation entrypoints | `src/calculator.rs`; transitional `src/ffi.rs` | mixed: `native-pass`, `fallback-only`, `unstarted` | no | category rows below |
| 3 | `MathStructure.h` | Expression tree construction, mutation, evaluation, formatting, algebra helpers | `src/ast.rs`, `eval.rs`, `simplify.rs`, `symbolic.rs` | mixed: `native-pass`, `unstarted` | no | #16-#24 |
| 4 | `Number.h` | Number constructors, arithmetic, predicates, conversion, intervals, uncertainty, precision | `src/number.rs` | mixed: `native-pass`, `unstarted` | no | #10-#15 |
| 5 | `ExpressionItem.h` | Shared name, registration, metadata, lifecycle APIs | `src/definitions.rs`, typed catalogs | mixed: `native-pass`, `unstarted` | no | #42 |
| 6 | `Variable.h` | Known/unknown variables, assumptions, value APIs | `src/definitions_catalog.rs`, `context.rs` | mixed: `native-pass`, `unstarted` | no | #45 |
| 7 | `Function.h` | MathFunction and Argument APIs | `src/functions/**`, `definitions_catalog.rs` | mixed: `native-pass`, `unstarted` | no | #30-#35 |
| 8 | `DataSet.h` | DataSet, DataProperty, DataObject APIs | `src/datasets.rs`, `calculator::Calculator::datasets` | mixed: `native-pass`, `unstarted` | no | dataset category below |
| 9 | `Unit.h` | Unit, AliasUnit, CompositeUnit APIs | `src/units.rs`, `unit_conversion.rs` | mixed: `native-pass`, `unstarted` | no | #38-#40; conversion category below |
| 10 | `Prefix.h` | Decimal, binary, and number prefix APIs | `src/units.rs` | mixed: `native-pass`, `unstarted` | no | #37 |
| 11 | `QalculateDateTime.h` | Date and time APIs | `src/datetime.rs` | mixed: `native-pass`, `unstarted` | no | #52-#54 |
| 12 | `includes.h` | Public enums, option structs, and constants used by calculator/parser/printing APIs | `src/options.rs`, `messages.rs`, crate root | mixed: `native-pass`, `unstarted` | no | options category below |

## API Family Matrix

| Family | Header | Public API Categories | Rust Owner | Status | Follow-up |
|---|---|---|---|---|---|
| Calculator | `Calculator.h` | construction/destruction; definition loading; parsing; evaluation; conversion; settings/options; messages; plot support | `src/calculator.rs`, transitional `src/ffi.rs` | mixed: `native-pass`, `fallback-only`, `unstarted` | category rows below |
| MathStructure | `MathStructure.h` | node types; construction; tree manipulation; evaluation; simplification; factoring; integration; differentiation; limits; polynomial ops; matrix/vector ops; printing; conversion; comparison | `ast`, `eval`, `simplify`, `symbolic` | mixed: `native-pass`, `unstarted` | #16-#24 |
| Number | `Number.h` | number type; construction; setters; arithmetic; comparison; predicates; conversion; interval ops; uncertainty; precision; format/print | `src/number.rs` | mixed: `native-pass`, `unstarted` | #10-#15 |
| ExpressionItem | `ExpressionItem.h` | name management; registration; type identification; properties; lifecycle | typed definition catalogs | mixed: `native-pass`, `unstarted` | #42 |
| Variable | `Variable.h` | construction; value get/set; assumptions; known/unknown discrimination | definition catalog and session context | mixed: `native-pass`, `unstarted` | #45 |
| Function | `Function.h` | construction; argument management; evaluation; condition checks; subfunction support | function modules and definition catalog | mixed: `native-pass`, `unstarted` | #30-#35 |
| DataSet | `DataSet.h` | construction; property management; object lookup; data loading | `src/datasets.rs` | mixed: `native-pass`, `unstarted` | dataset category below |
| Unit | `Unit.h` | construction; conversion; composite units; prefix management; base-unit relations | `src/units.rs`, `unit_conversion.rs` | mixed: `native-pass`, `unstarted` | #38-#40 |
| Prefix | `Prefix.h` | construction; value; name management; type identification | `src/units.rs` | mixed: `native-pass`, `unstarted` | #37 |
| Date/time | `QalculateDateTime.h` | construction; calendar conversion; arithmetic; formatting | `src/datetime.rs` | mixed: `native-pass`, `unstarted` | #52-#54 |
| Options and enums | `includes.h` | parse/evaluation/print options; enum constants; settings structures | `src/options.rs`, `messages.rs` | mixed: `native-pass`, `unstarted` | options category below |

## Native `Calculator` Surface

The crate-root `Calculator` (`calculator::Calculator`) is Rust-owned and contains no C++ pointer. `ffi::Calculator` remains a separately qualified compatibility/oracle wrapper. The following rows are the most specific classification for corresponding `Calculator.h` declarations.

| Category | Upstream surface | Rust surface | Status and boundary |
|---|---|---|---|
| Construction and ownership | `Calculator()`, destructor, global `CALCULATOR` usage | `calculator::Calculator::{new, default}` | `native-pass`: ordinary Rust ownership and drop; hidden mutable global state is intentionally not exposed. |
| Parsing and structured evaluation | `parse`, `calculate`, `calculateAndPrint` | `parse`, `evaluate`, `calculate`, `calculate_and_print` | `native-pass` for implemented native AST/evaluator families; unsupported upstream semantics remain `unstarted`, never implicit fallback. |
| Result formatting | `print`, `calculateAndPrint`, `MathStructure::print` | `print`, `calculate_and_print`, public `PrintOptions` | `native-pass` for plain native expression/number formatting; HTML/LaTeX, auto-format modes, and time-limited printing remain pending or `fallback-only`. |
| Definition loading and lookup | global definition loaders and item lookup methods | `load_definitions_from_dir`, `load_global_definitions`, `definitions`, `units` | `native-pass` for atomic upstream XML function/variable/prefix/unit/currency metadata loading and lookup; local save/load and mutable item lifecycle remain `unstarted`. |
| Dataset access | dataset loading and `getDataSet`/object/property lookup | `load_definitions_from_dir`, `datasets` and typed `DatasetCatalog` APIs | `native-pass` for XML catalog/object/property lookup; arbitrary mutation and persistence remain `unstarted`. |
| Unit conversion | `convert`, `convertToBestUnit`, conversion through calculation | `convert_and_print`, `calculate_and_print` with loaded unit catalog | `native-pass` for the focused native conversion engine; optimal/mixed-unit breadth and every upstream conversion overload remain pending. |
| Options and session commands | parse/evaluation/print option structs and setters | option accessors, `precision`, `set_precision`, `apply_command` | `native-pass` for represented option fields and typed commands; unrepresented option interactions remain `unstarted`. |
| Structured messages | message count/read/remove APIs | `messages`, `next_message`, `take_messages`, `clear_messages` | `native-pass`: ordered Rust-owned `CalculatorMessage` records. |
| Timeout and abort control | millisecond arguments, abort/terminate controls | no native root-API method | `fallback-only` in `ffi::Calculator` where available; native cancellation remains `unstarted`. |
| Plotting | plot parameters, data series, gnuplot helpers | no native root-API method | `unstarted`; no FFI-only plotting call is presented as native parity. |

## Status Summary

| Metric | Count |
|---|---:|
| Required headers classified | 12 |
| API families classified | 11 |
| Fully native-pass public API headers | 0 |
| Mixed native/pending public API headers | 12 |
| Native root `Calculator` categories | 8 |
| Explicit fallback-only `Calculator` categories | 1 |
| Explicit unstarted `Calculator` categories | 1 |

## Closure Notes

- `src/calculator.rs` is the native public session API; `src/ffi.rs` remains explicitly fallback/oracle infrastructure.
- No header is claimed fully complete. Each declaration inherits a mixed header/family status unless the native `Calculator` category table or a named owner task classifies it more specifically.
- Task #64 establishes stable ownership and access boundaries; it does not turn pending semantic breadth into native parity.
- #176 closes the Epic 8 matrix/vector diagnostics follow-up by documenting fail-closed behavior for unsupported diagnostic families; it does not change the `MathStructure.h` public API status.
- #49 adds `src/rates.rs` and focused fallback-disabled explicit currency conversion evidence, but it does not expose or complete the upstream `Unit.h`/`Calculator.h` public conversion APIs; those remain tracked by #50 and #64.
- There are no approved public API deviations in `docs/deviations.md` for this matrix. Rust ownership, `Result`, and the absence of hidden global state are accepted API mappings from the task packet rather than user-visible deviations.
