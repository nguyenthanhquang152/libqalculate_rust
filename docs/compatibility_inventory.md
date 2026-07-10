# Compatibility Inventory — libqalculate → libqalculate_rust

> **Upstream version**: libqalculate 5.11.0
> **Inventory date**: 2026-07-03
> **Epics**: 0 — Project Bootstrap & Inventory; 1 — Workspace Foundation and Optional C++ Oracle/FFI; 2 — Numeric Core (Number); 3 — AST, Parser, and Session Commands; 8 — Vectors, Matrices, Statistics, and CSV Data; 9 — Definitions, Units, Datasets, Currencies, and Rates
> **Tasks**: 0.1/0.2 inventory baseline; 1.1 (hybrid-build-inventory), 1.2 (ffi-sys-bindings), 1.3 (safe-ffi-calculator-wrapper), 1.4 (no-cpp-fallback-gate), 2.1-2.6 numeric-core slices; 3.1-3.5 AST, parser, name resolution, and command parsing slices; 8.1 (vector-matrix-ast-eval) and 8.2 (matrix-functions) native `matrixvector.batch` coverage; 9.1 (xml-loader-core), 9.2 (prefix-unit-loader), 9.3 (function-variable-loader), 9.4 (datasets-elements-planets), and 9.5 (currency-rate-loader) loader/lookup scaffolds

---

## Summary Statistics

| Category | Total | native-pass | tooling-pass | scaffold | fallback-only | unstarted | out-of-scope |
|---|---|---|---|---|---|---|---|
| Public Headers | 22 | 0 | 0 | 8 | 1 | 9 | 4 |
| Implementation Files | 41 | 0 | 0 | 11 | 2 | 28 | 0 |
| Definition Data Files | 9 | 0 | 0 | 9 | 0 | 0 | 0 |
| Batch Test Files | 17 | 5 | 0 | 3 | 0 | 9 | 0 |
| Batch Test Cases | 656 | 352 | 0 | 0 | 0 | 304 | 0 |
| CLI Behaviors | 14 | 3 | 3 | 5 | 1 | 2 | 0 |
| Core Class API Groups | 59 | 0 | 0 | 30 | 1 | 28 | 0 |

**Overall porting progress**: The workspace has an FFI fallback wrapper, build inventory, sys bindings, and a no-fallback gate for native evidence. The `Number` type now has native Rust slices for representation, exact rational storage, MPFR-backed floats, complex values, interval storage, uncertainty, selected arithmetic, formatting, and a small fallback-disabled expression evaluator. Full upstream `Number.cc` parity is not complete: setters, full conversion/format APIs, all edge-case arithmetic, base conversion display, and broad native oracle coverage remain incomplete. `Calculator` expression evaluation is still fallback-first, with native fallback-disabled routing only for oracle-proven subsets that the Rust scaffold can parse and evaluate successfully, including focused precision-context float arithmetic/comparison evidence, complex zero-part collapse, component metadata evidence, equality/inequality and equal-operand ordering evidence, finite interval arithmetic, infinity interval endpoint evidence, endpoint extraction, a narrow disjoint interval intersection row, alphabetic infinity literal/arithmetic evidence, one focused real-valued uncertainty `ln` propagation case, all 130 `matrixvector.batch` rows covering vector/matrix literals, constructors/accessors, arithmetic, shape helpers, and matrix functions, plus a narrow CSV loader count proof for `vectordata.csv` and `vectordata2.csv`, direct CSV-backed `mean(load(...))`/`stdev(load(...))`/`min(load(...))`/`max(load(...))`/`total(load(...))`/`range(load(...))`/`median(load(...))`, `geomean(abs(load(...)))`/`harmmean(abs(load(...)))`/`rms(load(...))`/`trimmean(load(...), 10)`/`winsormean(load(...), 10)`/`weighmean(load(...), genvector(2;1;100))`/`stderr(load(...))`/`meandev(load(...))`/`quartile(load(...), 1, 7)`/`percentile(load(...), 25, 7)`/`decile(load(...), 9, 7)`/`iqr(load(...))` proof for `vectordata.csv`, direct paired CSV `pearson`/`spearman`/`covar`/`poolvar`/`ttest`/`pttest` proof for `vectordata.csv` and `vectordata2.csv`, quoted-path forms for those direct CSV consumers, fallback-disabled native session-variable execution for the original `stats.batch` `name=load(...)` setup/delete rows, focused fallback-disabled native session-variable execution for all `variables.batch` rows in one context, selected literal statistics `mean`/`stdev`/`quartile`/`percentile`/`normdist`/`normdistinv`/`quadraticfit`/`cubicfit`/`fdist`/`chisqdistinv`/`mode`/`median` evidence, focused native dataset lookup evidence for `atom`/`planet` element and planet properties, and all 11 fallback-disabled `dates.batch` rows covering time arithmetic, CET-to-UTC-offset formatting, date arithmetic, `addDays`, timestamp/stamptodate, and focused lunar phase helpers. Epic 9 now has a Rust XML definition loader scaffold in `src/definitions.rs`, a typed prefix/unit catalog in `src/units.rs`, a typed function/variable catalog in `src/definitions_catalog.rs`, and a typed dataset catalog in `src/datasets.rs` that loads upstream dataset metadata, property aliases and flags, object data rows, provenance, and focused `atom`/`planet` lookup behavior; it does not claim unit conversion, currency rate, function-body execution, full variable-evaluation, or full DataSet public API parity. The batch manifest currently has 352 `native-pass` rows across selected batch rows; every other batch case remains inventory-only until proven through the manifest runner with fallback disabled. Focused numeric native oracle evidence is recorded in `docs/epic2_native_evidence.md`; vector/matrix evidence is recorded by `tests/oracle.rs::focused_issue41_vector_matrix_literal_oracle_cases`; CSV loader/statistics evidence is recorded by `tests/oracle.rs::focused_issue44_csv_load_oracle_cases`; literal statistics evidence is recorded by `tests/oracle.rs::focused_issue43_literal_statistics_oracle_cases`; XML loader scaffold evidence is recorded by `tests/definition_loader.rs`; prefix/unit loader evidence is recorded by `tests/prefix_unit_loader.rs`; function/variable loader evidence is recorded by `tests/function_variable_loader.rs`; dataset loader and lookup evidence is recorded by `tests/dataset_loader.rs`, `tests/dataset_lookup.rs`, and `tests/oracle.rs::focused_issue48_dataset_lookup_oracle_cases`; session-variable statistics evidence is recorded by `src/ffi.rs::tests::fallback_disabled_preserves_csv_loaded_statistics_session_variables`; focused `variables.batch` session evidence is recorded by `tests/oracle.rs::focused_issue47_variables_batch_session_oracle_cases`; focused date/time parser/formatter evidence is recorded by `tests/oracle.rs::focused_issue52_datetime_parser_formatter_oracle_cases`; and focused date/time function evidence is recorded by `tests/oracle.rs::focused_issue53_datetime_function_oracle_cases`.

**Issue #49 update**: `src/rates.rs` now parses `rates.json` snapshot dates and raw per-currency rates with provenance, applies offline `eurofxref-daily.xml` precedence for the focused fiat conversion oracle cases, preserves the upstream built-in BTC relation from `Calculator.cc`, and routes only explicit currency conversions through fallback-disabled native evaluation. This does not claim network exchange-rate refresh or general unit conversion parity; those remain staged follow-ups.

**Issue #60 update**: CLI listing (`-l`, `--list`, and the typed `--list-*` variants) is rendered from the existing Rust XML catalogs with exact upstream-derived search examples. Definition-disable flags gate only expressions that use the disabled family, unrelated native work remains available, and the C++ bridge loads selected global catalogs in upstream startup order. `-defaults` preserves the current built-in-only configuration behavior; no persistent user-configuration reader exists yet. `-exrates` validates the configured local snapshot and effective offline catalog; network refresh is tracked in #199. Color-off is effective, while forced-on color fails explicitly and links to #198.

---

## Status Legend

| Status | Meaning |
|---|---|
| `native-pass` | Fully ported to Rust; passes relevant upstream tests natively |
| `tooling-pass` | Rust-only tooling or inventory checks pass their own tests; no upstream native parity is claimed |
| `scaffold` | Rust types/module exist but functionality is incomplete |
| `fallback-only` | Functionality provided via FFI call to upstream C++ |
| `approved-deviation` | Intentionally differs from C++; user-visible behavior preserved |
| `unstarted` | No Rust code exists for this area |
| `out-of-scope` | Not needed in Rust (private impl detail, autoconf macro, etc.) |

---

## 1. Public Headers Inventory

Maps all 22 C++ public headers from `../libqalculate/libqalculate/*.h` to their Rust owner modules and implementation status.

| # | C++ Header | Rust Module | Status | Notes |
|---|---|---|---|---|
| 1 | `Calculator.h` | `src/ffi.rs` | `fallback-only` | FFI wrapper to C++ `Calculator`; exposes `calculate_and_print()`, `calculate_and_print_qalc()`, and definition-loading helpers via `cxx` bridge |
| 2 | `Calculator_p.h` | — | `out-of-scope` | Private implementation detail of Calculator |
| 3 | `MathStructure.h` | — | `unstarted` | Core expression tree; ~120 public methods |
| 4 | `MathStructure_p.h` | — | `out-of-scope` | Private implementation detail of MathStructure |
| 5 | `MathStructure-support.h` | — | `unstarted` | Internal support macros for MathStructure operations |
| 6 | `Number.h` | `src/number.rs` | `scaffold` | Mixed native slices for `NumberValue`, `Rational`, `Float`, intervals, complex values, and uncertainty; not full upstream `Number.h` parity |
| 7 | `ExpressionItem.h` | — | `unstarted` | Base class for all definition items |
| 8 | `ExpressionItem_p.h` | — | `out-of-scope` | Private implementation detail of ExpressionItem |
| 9 | `Function.h` | `src/definitions_catalog.rs` | `scaffold` | Loaded `FunctionDefinition` and `FunctionArgument` metadata exist, including raw user-function expressions/subfunctions; function body execution remains incomplete |
| 10 | `BuiltinFunctions.h` | — | `unstarted` | ~200 built-in function declarations |
| 11 | `Variable.h` | `src/definitions_catalog.rs` | `scaffold` | Loaded builtin/known/unknown variable metadata exists; full variable API remains incomplete |
| 12 | `Unit.h` | `src/units.rs` | `scaffold` | Typed loaded definitions for base, alias, composite, and builtin units; conversion APIs remain unstarted |
| 13 | `Prefix.h` | `src/units.rs` | `scaffold` | Typed loaded decimal/binary prefix definitions with names and exponents; full multiplier API remains incomplete |
| 14 | `DataSet.h` | `src/datasets.rs` | `scaffold` | Typed loaded `DatasetDefinition`, `DatasetPropertyDefinition`, and `DatasetObject` records exist with focused lookup; full public API remains incomplete |
| 15 | `QalculateDateTime.h` | `src/datetime.rs` | `scaffold` | Validated Gregorian date/time value model with exact seconds, UTC timestamps/differences, ordering, and exact day/month/year arithmetic; parsing, localized formatting, current-time functions, and calendar conversions remain incomplete |
| 16 | `includes.h` | `src/lib.rs` (partial) | `scaffold` | Enums, options structs referenced but not fully ported |
| 17 | `definitions.h` | — | `unstarted` | Definition loading constants and version macros |
| 18 | `qalculate.h` | — | `scaffold` | Umbrella `#include` header; not directly needed in Rust module system |
| 19 | `support.h` | — | `out-of-scope` | Autoconf portability macros (`HAVE_*`, platform detection) |
| 20 | `util.h` | — | `unstarted` | String utilities, file I/O helpers, locale support |
| 21 | `bernoulli_numbers.h` | — | `unstarted` | Precomputed Bernoulli number data table |
| 22 | `primes.h` | — | `unstarted` | Precomputed prime number table |

### Headers by Status

- **native-pass (0)**: —
- **fallback-only (1)**: `Calculator.h`
- **scaffold (9)**: `Number.h`, `Function.h`, `Variable.h`, `Unit.h`, `Prefix.h`, `DataSet.h`, `QalculateDateTime.h`, `includes.h`, `qalculate.h`
- **unstarted (8)**: `MathStructure.h`, `MathStructure-support.h`, `ExpressionItem.h`, `BuiltinFunctions.h`, `definitions.h`, `util.h`, `bernoulli_numbers.h`, `primes.h`
- **out-of-scope (4)**: `Calculator_p.h`, `MathStructure_p.h`, `ExpressionItem_p.h`, `support.h`

---

## 2. Implementation Families

Maps all 41 C++ `.cc` implementation files from `../libqalculate/libqalculate/*.cc` to their functional families and Rust status.

### Calculator Family (6 files)

| # | C++ File | Responsibility | Rust Status |
|---|---|---|---|
| 1 | `Calculator.cc` | Core calculator state, construction, messages | `fallback-only` for construction only via `src/ffi.rs` |
| 2 | `Calculator-calculate.cc` | Expression evaluation engine | `fallback-only` via `src/ffi.rs` |
| 3 | `Calculator-convert.cc` | Unit/base conversion | `unstarted` |
| 4 | `Calculator-definitions.cc` | Definition loading from XML | `scaffold` via `src/definitions.rs` and typed prefix/unit loading in `src/units.rs`; calculator state loading remains `fallback-only` via `src/ffi.rs` |
| 5 | `Calculator-parse.cc` | Expression parsing | `unstarted` for direct parser APIs; used internally by fallback evaluation |
| 6 | `Calculator-plot.cc` | Gnuplot integration | `unstarted` |

### MathStructure Family (14 files)

| # | C++ File | Responsibility | Rust Status |
|---|---|---|---|
| 1 | `MathStructure.cc` | Core AST node types, construction, basic ops | `unstarted` |
| 2 | `MathStructure-calculate.cc` | Expression calculation/evaluation | `unstarted` |
| 3 | `MathStructure-convert.cc` | Unit/type conversion within AST | `unstarted` |
| 4 | `MathStructure-decompose.cc` | Expression decomposition | `unstarted` |
| 5 | `MathStructure-differentiate.cc` | Symbolic differentiation | `unstarted` |
| 6 | `MathStructure-eval.cc` | Advanced evaluation strategies | `unstarted` |
| 7 | `MathStructure-factor.cc` | Polynomial/expression factoring | `unstarted` |
| 8 | `MathStructure-gcd.cc` | GCD/LCM of expressions | `unstarted` |
| 9 | `MathStructure-integrate.cc` | Symbolic integration | `unstarted` |
| 10 | `MathStructure-isolatex.cc` | Variable isolation / solving | `unstarted` |
| 11 | `MathStructure-limit.cc` | Limit computation | `unstarted` |
| 12 | `MathStructure-matrixvector.cc` | Matrix and vector operations | `scaffold` via `src/matrix.rs`; every visible `matrixvector.batch` row is native-pass, covering vector/matrix literal construction, top-level list output, identity construction, shape/accessors, `adj`, `cofactor`, `combine`, `cross`, `det`, `dot` function/operator rows, `entrywise`, `genvector`, `horzcat`, `vertcat`, `inverse`, magnitude, `norm`, `part`, `permanent`, `pow`/entrywise power, `rank`, `rk`, `rref`, `slice`, `sort`, and `transpose` rows, scalar scaling/subtraction, multiply/divide arithmetic, `hadamard` entrywise multiplication rows, row/column elementwise broadcasting, rectangular matrix multiplication, same-shape elementwise multiplication rows, and entrywise power broadcasting rows; broader direct upstream `MathStructure` API parity remains incomplete, and #176 keeps unsupported arity/shape/singular/non-square/dimension-mismatch diagnostics fail-closed under fallback-disabled mode |
| 13 | `MathStructure-polynomial.cc` | Polynomial arithmetic | `unstarted` |
| 14 | `MathStructure-print.cc` | Expression formatting/printing | `scaffold` via `src/text.rs`; rectangular vector/matrix output is covered for selected native-pass rows |

### Number Family (1 file)

| # | C++ File | Responsibility | Rust Status |
|---|---|---|---|
| 1 | `Number.cc` | Arbitrary-precision arithmetic (GMP/MPFR), intervals, uncertainty, complex values, formatting | `scaffold` via `src/number.rs`; selected native slices covered, full upstream parity incomplete |

### BuiltinFunctions Family (12 files)

| # | C++ File | Responsibility | Rust Status |
|---|---|---|---|
| 1 | `BuiltinFunctions-algebra.cc` | Algebraic functions (solve, simplify) | `unstarted` |
| 2 | `BuiltinFunctions-calculus.cc` | Calculus functions (diff, integrate, limit) | `unstarted` |
| 3 | `BuiltinFunctions-datetime.cc` | Date/time functions | `unstarted` |
| 4 | `BuiltinFunctions-explog.cc` | Exponential and logarithmic functions | `unstarted` |
| 5 | `BuiltinFunctions-matrixvector.cc` | Matrix/vector functions | `scaffold` via `src/matrix.rs` and `src/data.rs`; all `matrixvector.batch` vector/matrix function rows are native-pass, including `vector`, `matrix`, `matrix2vector`, `columns`, `dimension`, `rows`, `row`, `column`, `element`, `elements`, `multiply`, `adj`, `cofactor`, `combine`, `cross`, `det`, `dot` function/operator rows, `entrywise`, `hadamard`, `horzcat`, `identity`, `magnitude`, `norm`, `part`, `permanent`, collection `pow`, `rank`, `rk`, `rref`, `slice`, `sort`, `transpose`, `vertcat`, `divide`, and `rdivide`; `load(...)` has a narrow CSV count proof for `vectordata.csv`/`vectordata2.csv`, direct and session-variable CSV statistics consumption for those fixtures, and quoted-path forms for the direct consumers; #176 keeps unsupported matrix/vector diagnostics fail-closed rather than widening native handling |
| 6 | `BuiltinFunctions-number.cc` | Number theory functions | `unstarted` |
| 7 | `BuiltinFunctions-combinatorics.cc` | Combinatorics functions | `unstarted` |
| 8 | `BuiltinFunctions-logical.cc` | Logical/comparison functions | `unstarted` |
| 9 | `BuiltinFunctions-statistics.cc` | Statistical functions | `scaffold` via `src/statistics.rs`; all `stats.batch` rows are native-pass, including selected literal `mean`/`stdev`/`quartile`/`percentile`/`normdist`/`normdistinv`/`quadraticfit`/`cubicfit`/`fdist`/`chisqdistinv`/`mode`/`median`, direct CSV-backed one-vector statistics over `load(tests/vectordata.csv)`, session-variable one-vector statistics over `libqalculate_tests_vector`, and paired `pearson`/`spearman`/`covar`/`poolvar`/`ttest`/`pttest` over both upstream vector fixtures |
| 10 | `BuiltinFunctions-trigonometry.cc` | Trigonometric functions | `unstarted` |
| 11 | `BuiltinFunctions-special.cc` | Special functions (gamma, beta, zeta, erf, etc.) | `unstarted` |
| 12 | `BuiltinFunctions-util.cc` | Utility functions (string, base, etc.) | `unstarted` |

### Other Families (8 files)

| # | C++ File | Family | Rust Status |
|---|---|---|---|
| 1 | `ExpressionItem.cc` | ExpressionItem | `unstarted` |
| 2 | `Function.cc` | Function | `scaffold` via `src/definitions_catalog.rs` typed loaded function metadata, raw expression/subfunction metadata, and argument records; calculation remains unstarted |
| 3 | `Variable.cc` | Variable | `scaffold` via `src/definitions_catalog.rs` typed loaded variable metadata, values, units, uncertainty, precision, and approximation flags; full mutation/evaluation API remains incomplete |
| 4 | `Unit.cc` | Unit | `scaffold` via `src/units.rs` typed loaded unit metadata, descriptions, hidden flags, currency countries, parts, and base relations; conversion remains unstarted |
| 5 | `Prefix.cc` | Prefix | `scaffold` via `src/units.rs` typed loaded prefix names/exponents; full prefix value semantics remain incomplete |
| 6 | `DataSet.cc` | DataSet | `scaffold` via `src/datasets.rs` typed dataset/property/object loading and focused `atom`/`planet` lookup; mutation/save/full API parity remains incomplete |
| 7 | `QalculateDateTime.cc` | DateTime | `scaffold` via `src/datetime.rs` validated Gregorian value model, exact `Number` seconds, UTC timestamp/difference helpers, ordering, and exact day/month/year arithmetic; date string parsing/printing, timezone/localtime behavior, calendar conversions, and datetime built-in functions remain unstarted |
| 8 | `util.cc` | Utility | `unstarted` |

### Families by Status

| Status | Families | File Count |
|---|---|---|
| `scaffold` | MathStructure matrix/vector, MathStructure print, Number, BuiltinFunctions matrix/vector, selected statistical functions, Definition XML loader, Function typed loader, Variable typed loader, Unit typed loader, Prefix typed loader, DataSet typed loader/lookup, DateTime value model | 12 |
| `fallback-only` | Calculator construction, calculation | 2 |
| `unstarted` | Calculator conversion/parsing/plot APIs, remaining MathStructure, remaining BuiltinFunctions, ExpressionItem, Utility | 27 |

---

## 2.1 Upstream CLI and Test Harness Files

Maps the upstream command-line and test harness sources inspected for Epic 0. These files are not public library modules, but they define the user-facing batch/oracle behavior used by the Rust harness.

| # | Upstream File | Rust Owner | Status | Deviation | Next Task |
|---|---|---|---|---|---|
| 1 | `../libqalculate/src/qalc.cc` | `src/main.rs`, `src/ffi.rs`, `tests/oracle.rs` | `fallback-only` | none | #4, #20, #83 |
| 2 | `../libqalculate/src/test.cc` | `tests/oracle.rs`, `tests/batch_manifest_validation.rs` | `scaffold` | none | #4 |
| 3 | `../libqalculate/src/unittest.cc` | `tests/oracle.rs`, `tests/e2e_batch_runner.rs` | `scaffold` | none | #4 |
| 4 | `../libqalculate/libqalculate/Makefile.am` | `.github/workflows/rust.yml`, `build.rs` | `scaffold` | none | #6 |
| 5 | `../libqalculate/src/Makefile.am` | `.github/workflows/rust.yml`, `scripts/oracle.sh` | `scaffold` | none | #66 |
| 6 | `../libqalculate/tests/Makefile.am` | `docs/batch_manifest.md`, `tests/batch_manifest_validation.rs` | `tooling-pass` | none | #3 |
| 7 | `../libqalculate/data/Makefile.am` | `docs/compatibility_inventory.md` | `unstarted` | none | #41 |

All rows use exact upstream paths, an owner artifact in this repository, a status, a deviation value, and a next-task reference. No broad native parity is claimed for CLI expression evaluation: the default path still routes through the C++ fallback bridge, while fallback-disabled native evidence is limited to the oracle-proven numeric subset that `number::evaluate_expr()` can parse and evaluate successfully.

## 2.2 Build and Configure Feature Mapping

Maps upstream `configure.ac` / `Makefile.am` dependencies to the current Rust build script. This section is oracle/fallback infrastructure evidence only; it does not prove native Rust parity.

| Upstream feature or dependency | Upstream evidence | Rust build behavior | Status | Notes |
|---|---|---|---|---|
| Core C++ source list | `libqalculate_la_SOURCES` in `../libqalculate/libqalculate/Makefile.am` | `build.rs` compiles the 41 current `.cc` files into the static `qalculate` archive | `tooling-pass` | `tests/inventory_validation.rs` asserts the upstream `.cc` count and inventory coverage |
| GMP | `AC_CHECK_HEADER(gmp.h)`, `AC_CHECK_LIB(gmp, __gmpz_init)` | Links `-lgmp` | `fallback-only` | Required for the C++ fallback/oracle build |
| MPFR | `AC_CHECK_HEADERS(mpfr.h)`, `AC_CHECK_LIB(mpfr, mpfr_get_version)` | Links `-lmpfr` | `fallback-only` | Required for the C++ fallback/oracle build |
| libxml2 | `PKG_CHECK_MODULES(LIBXML, libxml-2.0 >= 2.3.8)` | Uses `pkg-config` for include paths and link metadata | `fallback-only` | Required for C++ definition XML loading |
| pthread/threading | `AC_CHECK_LIB(pthread, pthread_create)`, `HAVE_PTHREADS` | Not explicitly linked or defined by `build.rs` | `scaffold` | Current Rust tests serialize in-process FFI access with mutexes; cross-thread fallback/oracle behavior remains platform-sensitive |
| libcurl exchange-rate fetch | `--without-libcurl`, `PKG_CHECK_MODULES(LIBCURL, libcurl)`, `HAVE_LIBCURL` | Not probed; `HAVE_LIBCURL` is not defined | `unstarted` | Network exchange-rate fetching is not supported by the Rust build; local/static rate data remains data-directory dependent |
| ICU localization | `--without-icu`, `PKG_CHECK_MODULES(ICU, icu-uc)`, `HAVE_ICU` | Not probed; `HAVE_ICU` is not defined | `unstarted` | Case-insensitive Unicode/localization support is not enabled in the Rust fallback build |
| iconv / gettext / NLS | `AM_ICONV`, `LTLIBINTL`, `LTLIBICONV` | Not probed or linked directly | `unstarted` | Localized message catalogs are not part of the Rust build evidence |
| Platform C++ runtime | Autotools/libtool platform link | Links `c++` on Apple/FreeBSD, `stdc++` on non-MSVC targets | `fallback-only` | MSVC is not supported by this build script |
| Compiled definitions | `--enable-compiled-definitions`, `COMPILED_DEFINITIONS` | Not defined | `unstarted` | Rust fallback uses external data files instead of embedding definitions |
| Insecure functions switch | `--disable-insecure`, `DISABLE_INSECURE` | Not defined | `unstarted` | No Rust-side feature flag exists yet |
| Gnuplot call support | `--without-gnuplot-call`, `HAVE_GNUPLOT_CALL`, `HAVE_BYO_GNUPLOT` | Not defined | `unstarted` | Plot support is not exposed through the Rust wrapper |
| Textport/readline/test programs/docs generators | `--disable-textport`, `--enable-tests`, `--enable-defs2doc`, readline checks, doxygen checks | Out of the Rust library build | `out-of-scope` | Rust owns its own CLI/test harness; upstream helper binaries are not built by `build.rs` |

### Data and Locale Directories

`build.rs` defines `PACKAGE_DATA_DIR` as `/usr/share` and `PACKAGE_LOCALE_DIR` as `/usr/share/locale` to satisfy upstream compile-time constants. The local test/oracle path sets `QALCULATE_DEFINITIONS_DIR` to the adjacent `../libqalculate/data` checkout so definition loading is deterministic. A missing or invalid definitions directory is an expression-evaluation failure when C++ fallback is enabled; fallback-disabled native scaffold cases do not require C++ definitions.

---

## 3. Definition Data Files Inventory

Maps all 9 definition data files from `../libqalculate/data/` to Rust loading status.

| # | Data File | Format | Content | Rust Status | Notes |
|---|---|---|---|---|---|
| 1 | `currencies.xml.in` | XML | Currency definitions and exchange rate metadata | `scaffold` | Typed unit catalog loads builtin currency units, names, categories, countries, hidden flags, and provenance; focused explicit currency conversion uses this metadata plus `src/rates.rs`, while broader conversion remains #50 |
| 2 | `datasets.xml.in` | XML | Dataset definitions (elements, planets) | `scaffold` | Typed dataset catalog loads element/planet metadata, property aliases, key/hidden flags, units, argument titles, and provenance; focused `atom`/`planet` lookup cases are native |
| 3 | `elements.xml.in` | XML | Periodic table element properties | `scaffold` | Typed dataset catalog loads element object rows and key lookup by symbol, number, and name; focused `atom(...)` lookup cases are native |
| 4 | `functions.xml.in` | XML | Math/science function definitions | `scaffold` | Typed function catalog loads builtin/user function metadata, names/aliases, categories, active/hidden flags, arguments and constraint flags, raw expressions/subfunctions, examples/descriptions, provenance, diagnostics, and parser registry names; native function bodies remain feature-specific tasks |
| 5 | `planets.xml.in` | XML | Solar system planet/body data | `scaffold` | Typed dataset catalog loads planet object rows and key lookup by name; focused `planet(...)` lookup cases are native |
| 6 | `prefixes.xml.in` | XML | SI, binary, and other prefix definitions | `scaffold` | Typed prefix catalog loads decimal/binary kind, exponent, active state, name flags, provenance, and parser registry names; multiplier/conversion use remains #50 |
| 7 | `units.xml.in` | XML | Unit definitions and conversion relations | `scaffold` | Typed unit catalog loads base/alias/composite/builtin units, category paths, systems, name flags, parts, base relations, provenance, and parser registry names; conversion semantics remain #50 |
| 8 | `variables.xml.in` | XML | Built-in variable definitions | `scaffold` | Typed variable catalog loads builtin/known/unknown variable metadata, names/aliases, categories, active/hidden flags, values, units, uncertainty, precision, approximation flags, provenance, diagnostics, and parser registry names; broad variable evaluation semantics remain incomplete |
| 9 | `rates.json` | JSON | Exchange rate data (updated externally) | `scaffold` | `src/rates.rs` parses the local snapshot date and raw per-currency rates with provenance; focused native conversions use offline source precedence and do not implement network refresh |

### Data Loading Pipeline Status

```
XML/JSON files → Parser → Definition Registry → Calculator State
     ↑                        ↑                        ↑
  Present in           Rust XML scaffold      Prefix/unit catalog
  upstream data/       preserves raw          populates parser names;
  directory            provenance             calculator conversion
                                              remains future work
```

---

## 4. Batch Test Fixtures Inventory

Lists all 17 upstream `.batch` files from `../libqalculate/tests/` with case counts, session setting commands, and required external assets.

| # | Batch File | Cases | Settings | Required Assets | Rust Status |
|---|---|---|---|---|---|
| 1 | `bitwise.batch` | 24 | 0 | — | `unstarted` |
| 2 | `calculus.batch` | 11 | 0 | — | `unstarted` |
| 3 | `dates.batch` | 11 | 0 | — | `native-pass` |
| 4 | `explog.batch` | 10 | 1 | — | `scaffold` |
| 5 | `geometry.batch` | 30 | 0 | — | `native-pass` |
| 6 | `limits.batch` | 181 | 4 | — | `unstarted` |
| 7 | `matrixvector.batch` | 130 | 0 | — | `native-pass` |
| 8 | `numberbase.batch` | 15 | 3 | — | `native-pass` |
| 9 | `operators.batch` | 30 | 0 | — | `scaffold` |
| 10 | `parser.batch` | 27 | 0 | — | `scaffold` |
| 11 | `percentages.batch` | 26 | 0 | — | `unstarted` |
| 12 | `polynomial.batch` | 49 | 4 | — | `unstarted` |
| 13 | `solver.batch` | 25 | 4 | — | `unstarted` |
| 14 | `stats.batch` | 39 | 0 | `vectordata.csv`, `vectordata2.csv` | `native-pass` |
| 15 | `strings.batch` | 24 | 1 | — | `unstarted` |
| 16 | `units.batch` | 13 | 0 | — | `unstarted` |
| 17 | `variables.batch` | 11 | 0 | — | `unstarted` |

`variables.batch` remains `unstarted` in the manifest-oriented summary because
its rows are still `inventory-only` in `docs/batch_manifest.md`; focused #47
evidence for the full session sequence lives in
`tests/oracle.rs::focused_issue47_variables_batch_session_oracle_cases`.

### Totals

| Metric | Value |
|---|---|
| Total batch files | 17 |
| Total test cases | 656 |
| Native-pass batch cases | 352 |
| Inventory-only batch cases | 304 |
| Files with session settings | 6 |
| Files requiring CSV assets | 1 |
| Unique CSV assets | 2 (`vectordata.csv`, `vectordata2.csv`) |

### Session Settings Detail

| Setting Command | Used In | Meaning |
|---|---|---|
| `/set ic 2` | `explog.batch` | Set interval calculation mode to 2 |
| `/set approximation exact` | `limits.batch`, `polynomial.batch`, `solver.batch` | Force exact (symbolic) results |
| `/set fr 2` | `limits.batch`, `polynomial.batch`, `solver.batch` | Set fraction display mode to 2 |
| `/assume positive` | `limits.batch`, `polynomial.batch` | Assume variables are positive |
| `/assume unknown` | `limits.batch`, `polynomial.batch` | Reset variable assumptions to unknown |
| `set input base 16` | `numberbase.batch` | Parse input as hexadecimal |
| `set input base 10` | `numberbase.batch` | Parse input as decimal (reset) |
| `/set unicode 1` | `numberbase.batch`, `solver.batch`, `strings.batch` | Enable Unicode output |
| `/set approximation try exact` | `solver.batch` | Try exact, fall back to approximate |

### CSV Assets

| Asset | Used By | Content |
|---|---|---|
| `vectordata.csv` | `stats.batch` | Numeric vector/matrix test data |
| `vectordata2.csv` | `stats.batch` | Additional numeric vector/matrix test data |

`src/data.rs` now loads these upstream CSV assets as numeric vectors and has
fallback-disabled oracle coverage for `number(load(tests/vectordata.csv))` and
`number(load(tests/vectordata2.csv))`, including quoted-path forms. It also
resolves the upstream `tests/*.csv` fixture paths from the Rust crate root.
`src/statistics.rs` has fallback-disabled oracle coverage for direct
one-vector consumers over `load(tests/vectordata.csv)`: `mean`, `stdev`,
`min`, `max`, `total`, `range`, `median`, `geomean(abs(...))`,
`harmmean(abs(...))`, `rms`, `trimmean(..., 10)`, `winsormean(..., 10)`,
`weighmean(..., genvector(2;1;100))`, `stderr`, `meandev`, `quartile(..., 1,
7)`, `percentile(..., 25, 7)`, `decile(..., 9, 7)`, and `iqr`, including
quoted-path forms, plus direct `pearson`, `spearman`, `covar`, `poolvar`,
`ttest`, and `pttest` consumers over `load(tests/vectordata.csv)` and
`load(tests/vectordata2.csv)`, including quoted-path forms. The FFI wrapper
now preserves fallback-disabled native session variables for the original
`stats.batch` setup spelling, so `libqalculate_tests_vector=load(...)`,
`libqalculate_tests_vector2=load(...)`, the variable-backed one-vector and
paired statistics rows, and cleanup via `delete ...` are native-pass.

---

## 5. CLI Behaviors Inventory

Maps upstream `qalc` CLI flags and behaviors to `qalc-rs` implementation status.

| # | CLI Behavior | Upstream (`qalc`) | Rust (`qalc-rs`) | Status | Notes |
|---|---|---|---|---|---|
| 1 | `--version` | Prints version | Prints version | `native-pass` | |
| 2 | `--help` | Prints usage | Prints usage | `native-pass` | |
| 3 | `--self-check` | — | Runs self-diagnostics | `tooling-pass` | Rust-only addition; no upstream parity claim |
| 4 | `--list-upstream-tests` | — | Lists upstream batch files | `tooling-pass` | Rust-only addition; no upstream parity claim |
| 5 | `--parse-batch` | — | Parses batch file structure | `tooling-pass` | Native batch parser tooling; no expression evaluation |
| 6 | Expression evaluation | Evaluates via Calculator | Evaluates via FFI bridge or fallback-disabled native scaffold for an oracle-proven numeric subset | `fallback-only` | Default path delegates to C++ `Calculator::calculateAndPrint()` through `calculate_and_print_qalc()` for qalc-style output; `QALCULATE_DISABLE_FALLBACK=1` attempts `number::evaluate_expr()` only for expressions explicitly covered by native oracle evidence and reports `fallback=native` only for successful non-NaN native results |
| 7 | `-defaults` | Reset to default settings | Preserves the current built-in-only configuration state | `scaffold` | No persistent user-configuration reader exists yet; a poisoned config-directory test locks the current isolation behavior |
| 8 | `-set <option> <value>` | Set calculator option | Limited fallback-disabled native-evidence settings for promoted rows and focused probes | `scaffold` | Current native settings are whitelisted for input base/Unicode numberbase evidence, precision evidence, interval-display plus `/set ic 2` interval evidence, and Refs #15 concise uncertainty probes; generic qalc setting support remains unstarted, and fallback-enabled settings are rejected rather than silently ignored |
| 9 | `-l`, `--list`, `--list-*` | Search/list definitions | Renders selected typed Rust catalogs | `native-pass` | Exact C-locale examples cover functions, variables, units, prefixes, datasets, sorting, columns, footer, and no-match output |
| 10 | `-n`, `-no*` definition selection | Disable definition families | Gates expressions by the specific disabled family and loads selected C++ catalogs in qalc startup order | `scaffold` | `-nodefs` permits definition-free native evidence; the unit path preserves upstream's coupled currency loading |
| 11 | `-e`, `--exrates` | Refresh exchange rates, including network sources when available | Validates configured `rates.json` and the effective offline catalog | `scaffold` | Offline behavior lands in #60; network refresh is tracked in #199 |
| 12 | `-c`, `--color` | Select terminal colorization | `-c0` is uncolored; forced-on mode is an explicit error | `scaffold` | Exact token-aware colorization is tracked in #198 rather than silently ignored |
| 13 | `--test-file <path>` | Run batch test file | Explicit handoff to #63 | `unstarted` | `qalc-rs` has `--parse-batch` but no test-file execution |
| 14 | Interactive REPL | Line-editing REPL | Explicit handoff to #61 | `unstarted` | |

### CLI Status Summary

| Status | Count |
|---|---|
| `native-pass` | 3 |
| `tooling-pass` | 3 |
| `scaffold` | 5 |
| `fallback-only` | 1 |
| `unstarted` | 2 |

---

## 6. Public API Parity Matrix

For each of the 9 core C++ classes, maps major public API categories to Rust implementation status with method-count estimates.

### 6.1 Calculator (`Calculator.h` → `src/ffi.rs`)

| # | API Category | Est. Methods | Rust Status | Notes |
|---|---|---|---|---|
| 1 | Construction / destruction | 2 | `scaffold` | `Calculator::new()` creates C++ instance via FFI |
| 2 | Definition loading | 3 | `scaffold` | `load_global_definitions()`, `load_local_definitions()`, and `load_exchange_rates()` exposed via FFI |
| 3 | Expression parsing | 5 | `unstarted` | No direct `parse()` / `parseNumber()` wrapper yet |
| 4 | Expression evaluation | 8 | `fallback-only` | `calculate_and_print()` and qalc-style `calculate_and_print_qalc()` exposed via FFI; fallback-disabled mode can route a small oracle-proven native numeric expression subset through `number::evaluate_expr()` and tracked variants report fallback state for oracle evidence |
| 5 | Conversion | 6 | `scaffold` | No direct `convert()` / `convertToBaseUnits()` wrapper yet; fallback-disabled qalc-style evaluation has a focused native explicit currency conversion slice for #49 |
| 6 | Settings / options | 15+ | `unstarted` | No public settings/options wrapper yet; fallback-disabled CLI evidence accepts a narrow qalc setting subset (`input base`, `unicode`, `precision`) |
| 7 | Messages | 4 | `unstarted` | No `message()` / `nextMessage()` wrapper yet |
| 8 | Plot support | 3 | `unstarted` | No gnuplot wrapper yet |

### 6.2 MathStructure (`MathStructure.h`)

| # | API Category | Est. Methods | Rust Status | Notes |
|---|---|---|---|---|
| 1 | Node types enum | 1 | `unstarted` | `StructureType` enum (~30 variants) |
| 2 | Construction | 15+ | `unstarted` | Constructors for each node type |
| 3 | Tree manipulation | 20+ | `unstarted` | `addChild()`, `setChild()`, iterators |
| 4 | Evaluation | 5 | `unstarted` | `eval()`, `calculateFunctions()` |
| 5 | Simplification | 8 | `unstarted` | `simplify()`, `expandParentheses()` |
| 6 | Factoring | 4 | `unstarted` | `factorize()`, `structure()` |
| 7 | Integration | 3 | `unstarted` | `integrate()` |
| 8 | Differentiation | 2 | `unstarted` | `differentiate()` |
| 9 | Limits | 2 | `unstarted` | `limit()` |
| 10 | Polynomial ops | 6 | `unstarted` | `gcd()`, `polynomialDivide()` |
| 11 | Matrix / vector ops | 10 | `unstarted` | `determinant()`, `inverse()`, `transpose()` |
| 12 | Printing | 4 | `unstarted` | `print()`, `format()` |
| 13 | Conversion | 4 | `unstarted` | `convert()`, `toNumber()` |
| 14 | Comparison | 5 | `unstarted` | `equals()`, `contains()`, `isNumber()` |

### 6.3 Number (`Number.h` → `src/number.rs`)

| # | API Category | Est. Methods | Rust Status | Notes |
|---|---|---|---|---|
| 1 | NumberType enum | 1 | `scaffold` | `NumberValue` includes current Rust numeric variants, but this is not a complete upstream `Number.h` `NumberType` parity claim |
| 2 | Construction | 9 | `scaffold` | Basic constructors exist (`new`, `from_i32`, `from_rational`, `from_float`, `from_f64`, `new_interval`, `try_new_interval`, `new_uncertainty`, `new_complex`); interval construction orders finite reversed bounds like upstream `setInterval`, and `new_complex` drops zero imaginary metadata while preserving exact/approx state; the full upstream construction/setter surface remains incomplete |
| 3 | Setters | 6 | `unstarted` | `set()`, `setFloat()`, `setInterval()` |
| 4 | Arithmetic | 15 | `scaffold` | `add`, `sub`, `mul`, `div`, `pow`, `sqrt`, `ln`, `negate`, `conjugate`, `norm`, and `abs` exist for selected variants; fallback-disabled oracle evidence covers selected exact complex arithmetic, pure-real/pure-imaginary zero-part simplification, `conj`, `norm`, exact `i^2`, focused `ln`/`sqrt` real-function cases including precision-context scalar rows and `ln(5+/-0.3)` uncertainty propagation, focused infinity literal/arithmetic cases including signed infinity division, focused precision-context real float add/sub/mul/div and decimal/scientific input rows, and the focused `explog.batch:7` complex-power expression; float `ln`, `sqrt`, non-integer real `pow`, and promoted real float arithmetic use MPFR-backed arithmetic for the covered slice, non-integer/complex exponent complex power evidence currently uses the qalc-profile approximate complex branch, exact division-by-zero and indeterminate infinity forms remain outside native success because upstream keeps them symbolic, and full upstream edge-case parity remains incomplete |
| 5 | Comparison | 6 | `scaffold` | `PartialEq` for value equality; focused fallback-disabled complex `=`, `==`, `!=`, `≠`, and equal-operand `<`, `>`, `<=`, `>=`, `≤`, and `≥` output constraints now match upstream for vetted exact complex expressions; focused precision-context real float comparisons match upstream booleans for promoted square-root/rational rows; non-equal complex ordering remains outside native evidence because upstream keeps those expressions symbolic |
| 6 | Predicates | 12 | `scaffold` | Current predicates include zero/one, complex, real/imaginary part, interval, infinity, NaN, approximation, and precision state used by the scaffold; full upstream predicate parity remains incomplete |
| 7 | Conversion | 5 | `scaffold` | Bounded `num()`/`den()` accessors remain `i128`; internal exact rationals can exceed that range and display through `rug` strings |
| 8 | Interval operations | 4 | `scaffold` | Native interval storage, comparison categories, selected outward-rounded finite arithmetic, negative-only interval display, selected endpoint extraction functions, a narrow disjoint `intersect(...) -> []` row, infinity endpoint construction/display, and selected infinity endpoint arithmetic are covered by focused tests and fallback-disabled oracle probes under the existing interval-display/`ic 2` gate; qalc-compatible interval input syntax, open bounds, broad infinity interval division, overlapping interval intersection semantics, precision conversion, and broad oracle coverage remain incomplete |
| 9 | Uncertainty | 3 | `scaffold` | Native absolute/relative uncertainty representation, parsing, formatting, selected arithmetic propagation, the promoted uncertainty power row, spaced ASCII/Unicode/concise input-form evidence, scalar `uncertainty(...)` construction, `errorPart`, `valuePart`/`midpoint`, uncertainty endpoint extraction, and one real-valued `ln(5+/-0.3)` function propagation case are covered by focused tests and fallback-disabled oracle evidence; relative `errorPart(value;1)`, complex uncertainty, interval-calculation uncertainty behavior, special functions, and broad uncertainty function coverage remain incomplete |
| 10 | Precision | 4 | `scaffold` | `rug::Float` precision is tracked, and fallback-disabled qalc output honors `/set precision N` for promoted exact-rational output, native non-integer power `2 ^ 0.5`, scalar `ln`/`sqrt` function rows, focused finite real float arithmetic composed from precision-context powers at 64/128 digits, exact decimal/scientific input arithmetic at 64/128 digits, and setting-gated real float comparison rows at 64/128 digits; the full upstream precision-setting API is not ported |
| 11 | Format / print | 3 | `scaffold` | Display exists for current Rust variants; upstream base/localized formatting and full print options are not ported |

### 6.4 ExpressionItem (`ExpressionItem.h`)

| # | API Category | Est. Methods | Rust Status | Notes |
|---|---|---|---|---|
| 1 | Name management | 8 | `unstarted` | Names, abbreviations, plural forms, Unicode |
| 2 | Registration | 4 | `unstarted` | `setRegistered()`, category, `id()` |
| 3 | Type identification | 3 | `unstarted` | `type()`, `subtype()` |
| 4 | Properties | 6 | `unstarted` | `isActive()`, `isLocal()`, `isBuiltin()`, `isHidden()` |

### 6.5 Variable (`Variable.h`)

| # | API Category | Est. Methods | Rust Status | Notes |
|---|---|---|---|---|
| 1 | Construction | 4 | `scaffold` | `VariableDefinition` models loaded builtin, known, and unknown variable records; runtime constructors remain incomplete |
| 2 | Value get / set | 5 | `scaffold` | Loaded XML values, units, uncertainty, precision, and approximation flags are preserved; mutation APIs remain unstarted |
| 3 | Assumption management | 6 | `unstarted` | `assumptions()`, `setAssumptions()` (sign, type) |
| 4 | Known / unknown | 3 | `scaffold` | `VariableKind` distinguishes builtin, known, and unknown loaded records |

### 6.6 Function (`Function.h`)

| # | API Category | Est. Methods | Rust Status | Notes |
|---|---|---|---|---|
| 1 | Construction | 4 | `scaffold` | `FunctionDefinition` models loaded builtin and user function records; runtime constructors remain incomplete |
| 2 | Argument management | 8 | `scaffold` | Loaded argument index, type, title, min/max constraints, and arity metadata are preserved |
| 3 | Evaluation | 3 | `unstarted` | `calculate()`, `representsNumber()` |
| 4 | Condition checks | 4 | `scaffold` | Raw condition metadata is preserved; validation/execution semantics remain incomplete |
| 5 | Subfunction support | 3 | `unstarted` | `setSubfunction()`, expression-based functions |

### 6.7 DataSet (`DataSet.h`)

| # | API Category | Est. Methods | Rust Status | Notes |
|---|---|---|---|---|
| 1 | Construction | 3 | `scaffold` | `DatasetDefinition`, `DatasetPropertyDefinition`, and `DatasetObject` model loaded records; runtime mutation constructors remain incomplete |
| 2 | Property management | 6 | `scaffold` | Loaded property names/aliases, reference names, type, key/hidden/case flags, approximate flags, units, and provenance are preserved |
| 3 | Object lookup | 4 | `scaffold` | Loaded objects can be found by key property values; focused `atom`/`planet` native lookup is covered |
| 4 | Data loading | 3 | `scaffold` | `datasets.xml.in`, `elements.xml.in`, and `planets.xml.in` are loaded through the typed catalog; save/local override behavior remains incomplete |

### 6.8 Unit (`Unit.h`)

| # | API Category | Est. Methods | Rust Status | Notes |
|---|---|---|---|---|
| 1 | Construction | 5 | `scaffold` | `UnitDefinition` models loaded base, alias, composite, and builtin unit records; runtime `Unit` object construction remains incomplete |
| 2 | Conversion | 6 | `unstarted` | `convert()`, `convertToBaseUnit()`, `convertFromBaseUnit()` |
| 3 | Composite units | 4 | `scaffold` | Loaded `UnitPart` entries preserve unit, prefix exponent, and exponent metadata; mutation APIs remain unstarted |
| 4 | Prefix management | 4 | `scaffold` | Loaded `use_with_prefixes` metadata and parser prefix+unit name lookup exist; runtime `setPrefix()` behavior remains unstarted |
| 5 | Base unit relations | 5 | `scaffold` | Loaded `UnitBase` entries preserve base unit, relation, exponent, and mix strings; conversion evaluation remains unstarted |

### 6.9 Prefix (`Prefix.h`)

| # | API Category | Est. Methods | Rust Status | Notes |
|---|---|---|---|---|
| 1 | Construction | 3 | `scaffold` | `PrefixDefinition` models loaded decimal and binary prefix records; number prefixes remain incomplete |
| 2 | Value | 2 | `scaffold` | Decimal/binary exponent is loaded; full `Number` multiplier construction remains #50 |
| 3 | Name management | 4 | `scaffold` | Loaded names preserve upstream flags, abbreviations, Unicode aliases, and provenance |
| 4 | Type identification | 2 | `scaffold` | Loaded `PrefixType` distinguishes decimal and binary prefixes |

### API Parity Summary

| Class | Total Categories | native-pass | scaffold | fallback-only | unstarted |
|---|---|---|---|---|---|
| Calculator | 8 | 0 | 2 | 1 | 5 |
| MathStructure | 14 | 0 | 0 | 0 | 14 |
| Number | 11 | 0 | 10 | 0 | 1 |
| ExpressionItem | 4 | 0 | 0 | 0 | 4 |
| Variable | 4 | 0 | 3 | 0 | 1 |
| Function | 5 | 0 | 3 | 0 | 2 |
| DataSet | 4 | 0 | 4 | 0 | 0 |
| Unit | 5 | 0 | 4 | 0 | 1 |
| Prefix | 4 | 0 | 4 | 0 | 0 |
| **Total** | **59** | **0** | **30** | **1** | **28** |

---

## Appendix A: File Cross-Reference

### Rust Source Files → Upstream Mappings

| Rust File | Upstream Coverage | Status |
|---|---|---|
| `src/lib.rs` | `includes.h` (partial), crate root | `scaffold` |
| `src/ffi.rs` | `Calculator.h`, `Calculator.cc`, `Calculator-calculate.cc`, `Calculator-definitions.cc` (partial) | `fallback-only` |
| `src/definitions.rs` | `Calculator-definitions.cc`, upstream `data/*.xml.in` generic XML structure | `scaffold` |
| `src/units.rs` | `Prefix.h`, `Prefix.cc`, `Unit.h`, `Unit.cc`, `Calculator-definitions.cc`, `prefixes.xml.in`, `units.xml.in`, `currencies.xml.in` | `scaffold` |
| `src/rates.rs` | `Calculator-definitions.cc`, `Unit.cc`, `Calculator.cc`, `rates.json`, `eurofxref-daily.xml`, `currencies.xml.in` | `scaffold` |
| `src/number.rs` | `Number.h`, `Number.cc` | mixed: focused native evidence, scaffold, unstarted |
| `src/batch.rs` | — (no upstream equivalent; native batch parser) | `tooling-pass` |
| `src/main.rs` | `../src/qalc.cc` (partial CLI parity) | mixed |

### Upstream Files with No Rust Counterpart

30 of 41 `.cc` implementation responsibilities remain unstarted (Calculator conversion/parsing/plot APIs, remaining MathStructure files, remaining BuiltinFunctions files, ExpressionItem, Function, Variable, DataSet, and util families).

---

## Appendix B: Recommended Porting Order

Based on dependency analysis and test coverage potential:

1. **Number.cc** — Complete the scaffold; enables arithmetic test cases
2. **includes.h enums** — Port `StructureType`, `ComparisonResult`, `ApproximationMode`, etc.
3. **MathStructure.cc** — Core AST node types and construction
4. **Calculator-parse.cc** — Expression parsing (enables parser.batch)
5. **MathStructure-print.cc** — Expression formatting (enables test comparison)
6. **Calculator-calculate.cc** — Expression evaluation
7. **Definition loading** — XML parsing for units, variables, functions, prefixes
8. **BuiltinFunctions** — Incrementally, starting with arithmetic/number theory
9. **Unit.cc / Prefix.cc** — Unit conversion support
10. **Remaining families** — DataSet, DateTime, calculus, etc.

---

<!--
MACHINE-READABLE STATUS BLOCK — Do not edit manually.
Used by automated inventory validation tooling.

inventory_version: 1
upstream_version: 5.11.0
inventory_date: 2026-06-10
epic: 2
tasks: [0.1, 0.2, 1.1, 1.2, 1.3, 1.4, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6]

counts:
  public_headers: 22
  implementation_files: 41
  definition_data_files: 9
  batch_test_files: 17
  batch_test_cases: 656
  csv_assets: 2
  cli_behaviors: 10
  core_classes: 9
  api_categories: 59

status_summary:
  headers:
    native_pass: 0
    tooling_pass: 0
    scaffold: 4
    fallback_only: 1
    unstarted: 14
    out_of_scope: 4
  implementation_files:
    native_pass: 0
    tooling_pass: 0
    scaffold: 5
    fallback_only: 3
    unstarted: 33
    out_of_scope: 0
  definition_data:
    native_pass: 0
    tooling_pass: 0
    scaffold: 0
    fallback_only: 0
    unstarted: 9
    out_of_scope: 0
  batch_tests:
    native_pass: 5
    tooling_pass: 0
    scaffold: 3
    fallback_only: 0
    unstarted: 9
    out_of_scope: 0
  batch_test_cases:
    native_pass: 352
    tooling_pass: 0
    scaffold: 0
    fallback_only: 0
    unstarted: 304
    out_of_scope: 0
  cli_behaviors:
    native_pass: 2
    tooling_pass: 3
    scaffold: 1
    fallback_only: 1
    unstarted: 3
    out_of_scope: 0
  api_categories:
    native_pass: 0
    tooling_pass: 0
    scaffold: 12
    fallback_only: 1
    unstarted: 46
    out_of_scope: 0
-->
