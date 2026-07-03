# Compatibility Inventory — libqalculate → libqalculate_rust

> **Upstream version**: libqalculate 5.11.0
> **Inventory date**: 2026-07-03
> **Epics**: 0 — Project Bootstrap & Inventory; 1 — Workspace Foundation and Optional C++ Oracle/FFI; 2 — Numeric Core (Number); 3 — AST, Parser, and Session Commands; 8 — Vectors, Matrices, Statistics, and CSV Data
> **Tasks**: 0.1/0.2 inventory baseline; 1.1 (hybrid-build-inventory), 1.2 (ffi-sys-bindings), 1.3 (safe-ffi-calculator-wrapper), 1.4 (no-cpp-fallback-gate), 2.1-2.6 numeric-core slices; 3.1-3.5 AST, parser, name resolution, and command parsing slices; 8.1 (vector-matrix-ast-eval) literal/constructor/accessor scaffold

---

## Summary Statistics

| Category | Total | native-pass | tooling-pass | scaffold | fallback-only | unstarted | out-of-scope |
|---|---|---|---|---|---|---|---|
| Public Headers | 22 | 0 | 0 | 3 | 1 | 14 | 4 |
| Implementation Files | 41 | 0 | 0 | 4 | 3 | 34 | 0 |
| Definition Data Files | 9 | 0 | 0 | 0 | 0 | 9 | 0 |
| Batch Test Files | 17 | 1 | 0 | 4 | 0 | 12 | 0 |
| Batch Test Cases | 656 | 237 | 0 | 0 | 0 | 419 | 0 |
| CLI Behaviors | 10 | 2 | 3 | 1 | 1 | 3 | 0 |
| Core Class API Groups | 59 | 0 | 0 | 12 | 1 | 46 | 0 |

**Overall porting progress**: The workspace has an FFI fallback wrapper, build inventory, sys bindings, and a no-fallback gate for native evidence. The `Number` type now has native Rust slices for representation, exact rational storage, MPFR-backed floats, complex values, interval storage, uncertainty, selected arithmetic, formatting, and a small fallback-disabled expression evaluator. Full upstream `Number.cc` parity is not complete: setters, full conversion/format APIs, all edge-case arithmetic, base conversion display, and broad native oracle coverage remain incomplete. `Calculator` expression evaluation is still fallback-first, with native fallback-disabled routing only for oracle-proven subsets that the Rust scaffold can parse and evaluate successfully, including focused precision-context float arithmetic/comparison evidence, complex zero-part collapse, component metadata evidence, equality/inequality and equal-operand ordering evidence, finite interval arithmetic, infinity interval endpoint evidence, endpoint extraction, a narrow disjoint interval intersection row, alphabetic infinity literal/arithmetic evidence, one focused real-valued uncertainty `ln` propagation case, and a focused vector/matrix literal, constructor/accessor, shape/accessor, and arithmetic subset including selected `hadamard`, `identity`, and `magnitude` rows. The batch manifest currently has 237 `native-pass` rows across selected batch rows; every other batch case remains inventory-only until proven with fallback disabled. Focused numeric native oracle evidence is recorded in `docs/epic2_native_evidence.md`; vector/matrix evidence is recorded by `tests/oracle.rs::focused_issue41_vector_matrix_literal_oracle_cases`.

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
| 9 | `Function.h` | — | `unstarted` | `MathFunction` and argument classes |
| 10 | `BuiltinFunctions.h` | — | `unstarted` | ~200 built-in function declarations |
| 11 | `Variable.h` | — | `unstarted` | `Variable`, `KnownVariable`, `UnknownVariable` |
| 12 | `Unit.h` | — | `unstarted` | `Unit`, `AliasUnit`, `CompositeUnit` |
| 13 | `Prefix.h` | — | `unstarted` | `DecimalPrefix`, `BinaryPrefix`, `NumberPrefix` |
| 14 | `DataSet.h` | — | `unstarted` | `DataSet`, `DataProperty`, `DataObject` |
| 15 | `QalculateDateTime.h` | — | `unstarted` | Date/time arithmetic types |
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
- **scaffold (3)**: `Number.h`, `includes.h`, `qalculate.h`
- **unstarted (14)**: `MathStructure.h`, `MathStructure-support.h`, `ExpressionItem.h`, `Function.h`, `BuiltinFunctions.h`, `Variable.h`, `Unit.h`, `Prefix.h`, `DataSet.h`, `QalculateDateTime.h`, `definitions.h`, `util.h`, `bernoulli_numbers.h`, `primes.h`
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
| 4 | `Calculator-definitions.cc` | Definition loading from XML | `fallback-only` via `src/ffi.rs` |
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
| 12 | `MathStructure-matrixvector.cc` | Matrix and vector operations | `scaffold` via `src/matrix.rs`; native-pass coverage is limited to vector/matrix literal construction, selected identity construction, selected shape/accessors, selected magnitude rows, scalar scaling/subtraction, selected multiply/divide arithmetic, selected `hadamard` entrywise multiplication rows, row/column elementwise broadcasting, one rectangular matrix multiplication row, and same-shape elementwise multiplication rows |
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
| 5 | `BuiltinFunctions-matrixvector.cc` | Matrix/vector functions | `scaffold` via `src/matrix.rs`; selected `vector`, `matrix`, `matrix2vector`, `columns`, `dimension`, `rows`, `row`, `column`, `element`, `elements`, `multiply`, `hadamard`, `identity`, `magnitude`, `divide`, and `rdivide` rows are native-pass |
| 6 | `BuiltinFunctions-number.cc` | Number theory functions | `unstarted` |
| 7 | `BuiltinFunctions-combinatorics.cc` | Combinatorics functions | `unstarted` |
| 8 | `BuiltinFunctions-logical.cc` | Logical/comparison functions | `unstarted` |
| 9 | `BuiltinFunctions-statistics.cc` | Statistical functions | `unstarted` |
| 10 | `BuiltinFunctions-trigonometry.cc` | Trigonometric functions | `unstarted` |
| 11 | `BuiltinFunctions-special.cc` | Special functions (gamma, beta, zeta, erf, etc.) | `unstarted` |
| 12 | `BuiltinFunctions-util.cc` | Utility functions (string, base, etc.) | `unstarted` |

### Other Families (8 files)

| # | C++ File | Family | Rust Status |
|---|---|---|---|
| 1 | `ExpressionItem.cc` | ExpressionItem | `unstarted` |
| 2 | `Function.cc` | Function | `unstarted` |
| 3 | `Variable.cc` | Variable | `unstarted` |
| 4 | `Unit.cc` | Unit | `unstarted` |
| 5 | `Prefix.cc` | Prefix | `unstarted` |
| 6 | `DataSet.cc` | DataSet | `unstarted` |
| 7 | `QalculateDateTime.cc` | DateTime | `unstarted` |
| 8 | `util.cc` | Utility | `unstarted` |

### Families by Status

| Status | Families | File Count |
|---|---|---|
| `scaffold` | Number | 1 |
| `fallback-only` | Calculator construction, calculation, definitions | 3 |
| `unstarted` | Calculator conversion/parsing/plot APIs, MathStructure, BuiltinFunctions, ExpressionItem, Function, Variable, Unit, Prefix, DataSet, DateTime, Utility | 37 |

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
| 1 | `currencies.xml.in` | XML | Currency definitions and exchange rate metadata | `unstarted` | ~170 currencies; names use `@TRANSLATORS` markers |
| 2 | `datasets.xml.in` | XML | Dataset definitions (elements, planets) | `unstarted` | References `elements.xml.in` and `planets.xml.in` |
| 3 | `elements.xml.in` | XML | Periodic table element properties | `unstarted` | 118 elements with physical/chemical properties |
| 4 | `functions.xml.in` | XML | Math/science function definitions | `unstarted` | Builtin function metadata and argument specs |
| 5 | `planets.xml.in` | XML | Solar system planet/body data | `unstarted` | Orbital and physical parameters |
| 6 | `prefixes.xml.in` | XML | SI, binary, and other prefix definitions | `unstarted` | Decimal (yocto–quetta), binary (kibi–exbi), number prefixes |
| 7 | `units.xml.in` | XML | Unit definitions and conversion relations | `unstarted` | ~600 units across all domains |
| 8 | `variables.xml.in` | XML | Built-in variable definitions | `unstarted` | Mathematical/physical constants |
| 9 | `rates.json` | JSON | Exchange rate data (updated externally) | `unstarted` | ECB-sourced rates; fetched at runtime in upstream |

### Data Loading Pipeline Status

```
XML/JSON files → Parser → Definition Registry → Calculator State
     ↑                        ↑                        ↑
  Present in           No Rust XML            No Rust registry
  upstream data/       parser yet             (ffi.rs delegates
  directory                                    to C++ loadDefs)
```

---

## 4. Batch Test Fixtures Inventory

Lists all 17 upstream `.batch` files from `../libqalculate/tests/` with case counts, session setting commands, and required external assets.

| # | Batch File | Cases | Settings | Required Assets | Rust Status |
|---|---|---|---|---|---|
| 1 | `bitwise.batch` | 24 | 0 | — | `unstarted` |
| 2 | `calculus.batch` | 11 | 0 | — | `unstarted` |
| 3 | `dates.batch` | 11 | 0 | — | `unstarted` |
| 4 | `explog.batch` | 10 | 1 | — | `scaffold` |
| 5 | `geometry.batch` | 30 | 0 | — | `native-pass` |
| 6 | `limits.batch` | 181 | 4 | — | `unstarted` |
| 7 | `matrixvector.batch` | 130 | 0 | — | `scaffold` |
| 8 | `numberbase.batch` | 15 | 3 | — | `native-pass` |
| 9 | `operators.batch` | 30 | 0 | — | `scaffold` |
| 10 | `parser.batch` | 27 | 0 | — | `scaffold` |
| 11 | `percentages.batch` | 26 | 0 | — | `unstarted` |
| 12 | `polynomial.batch` | 49 | 4 | — | `unstarted` |
| 13 | `solver.batch` | 25 | 4 | — | `unstarted` |
| 14 | `stats.batch` | 39 | 0 | `vectordata.csv`, `vectordata2.csv` | `unstarted` |
| 15 | `strings.batch` | 24 | 1 | — | `unstarted` |
| 16 | `units.batch` | 13 | 0 | — | `unstarted` |
| 17 | `variables.batch` | 11 | 0 | — | `unstarted` |

### Totals

| Metric | Value |
|---|---|
| Total batch files | 17 |
| Total test cases | 656 |
| Native-pass batch cases | 237 |
| Inventory-only batch cases | 419 |
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
| 7 | `-defaults` | Reset to default settings | — | `unstarted` | |
| 8 | `-set <option> <value>` | Set calculator option | Limited fallback-disabled native-evidence settings for promoted rows and focused probes | `scaffold` | Current native settings are whitelisted for input base/Unicode numberbase evidence, precision evidence, interval-display plus `/set ic 2` interval evidence, and Refs #15 concise uncertainty probes; generic qalc setting support remains unstarted, and fallback-enabled settings are rejected rather than silently ignored |
| 9 | `--test-file <path>` | Run batch test file | — | `unstarted` | `qalc-rs` has `--parse-batch` but no evaluation |
| 10 | Interactive REPL | Line-editing REPL | — | `unstarted` | |

### CLI Status Summary

| Status | Count |
|---|---|
| `native-pass` | 2 |
| `tooling-pass` | 3 |
| `scaffold` | 1 |
| `fallback-only` | 1 |
| `unstarted` | 3 |

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
| 5 | Conversion | 6 | `unstarted` | No direct `convert()` / `convertToBaseUnits()` wrapper yet |
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
| 1 | Construction | 4 | `unstarted` | `KnownVariable`, `UnknownVariable` constructors |
| 2 | Value get / set | 5 | `unstarted` | `get()`, `set()` for known variables |
| 3 | Assumption management | 6 | `unstarted` | `assumptions()`, `setAssumptions()` (sign, type) |
| 4 | Known / unknown | 3 | `unstarted` | `isKnown()`, type discrimination |

### 6.6 Function (`Function.h`)

| # | API Category | Est. Methods | Rust Status | Notes |
|---|---|---|---|---|
| 1 | Construction | 4 | `unstarted` | `MathFunction`, `UserFunction` constructors |
| 2 | Argument management | 8 | `unstarted` | Arg types, counts, defaults, conditions |
| 3 | Evaluation | 3 | `unstarted` | `calculate()`, `representsNumber()` |
| 4 | Condition checks | 4 | `unstarted` | `testCondition()`, `testArgumentCount()` |
| 5 | Subfunction support | 3 | `unstarted` | `setSubfunction()`, expression-based functions |

### 6.7 DataSet (`DataSet.h`)

| # | API Category | Est. Methods | Rust Status | Notes |
|---|---|---|---|---|
| 1 | Construction | 3 | `unstarted` | DataSet, DataProperty, DataObject |
| 2 | Property management | 6 | `unstarted` | Add/get/list properties and their types |
| 3 | Object lookup | 4 | `unstarted` | Find objects by property values |
| 4 | Data loading | 3 | `unstarted` | Load from XML data files |

### 6.8 Unit (`Unit.h`)

| # | API Category | Est. Methods | Rust Status | Notes |
|---|---|---|---|---|
| 1 | Construction | 5 | `unstarted` | `Unit`, `AliasUnit`, `CompositeUnit` constructors |
| 2 | Conversion | 6 | `unstarted` | `convert()`, `convertToBaseUnit()`, `convertFromBaseUnit()` |
| 3 | Composite units | 4 | `unstarted` | Add/get components of composite units |
| 4 | Prefix management | 4 | `unstarted` | `setPrefix()`, `usesPrefix()` |
| 5 | Base unit relations | 5 | `unstarted` | `baseUnit()`, `baseExponent()`, expression/relation |

### 6.9 Prefix (`Prefix.h`)

| # | API Category | Est. Methods | Rust Status | Notes |
|---|---|---|---|---|
| 1 | Construction | 3 | `unstarted` | `DecimalPrefix`, `BinaryPrefix`, `NumberPrefix` |
| 2 | Value | 2 | `unstarted` | `value()` — the multiplier as a Number |
| 3 | Name management | 4 | `unstarted` | `name()`, `abbreviation()`, Unicode names |
| 4 | Type identification | 2 | `unstarted` | `type()` — decimal, binary, or number |

### API Parity Summary

| Class | Total Categories | native-pass | scaffold | fallback-only | unstarted |
|---|---|---|---|---|---|
| Calculator | 8 | 0 | 2 | 1 | 5 |
| MathStructure | 14 | 0 | 0 | 0 | 14 |
| Number | 11 | 0 | 10 | 0 | 1 |
| ExpressionItem | 4 | 0 | 0 | 0 | 4 |
| Variable | 4 | 0 | 0 | 0 | 4 |
| Function | 5 | 0 | 0 | 0 | 5 |
| DataSet | 4 | 0 | 0 | 0 | 4 |
| Unit | 5 | 0 | 0 | 0 | 5 |
| Prefix | 4 | 0 | 0 | 0 | 4 |
| **Total** | **59** | **0** | **12** | **1** | **46** |

---

## Appendix A: File Cross-Reference

### Rust Source Files → Upstream Mappings

| Rust File | Upstream Coverage | Status |
|---|---|---|
| `src/lib.rs` | `includes.h` (partial), crate root | `scaffold` |
| `src/ffi.rs` | `Calculator.h`, `Calculator.cc`, `Calculator-calculate.cc`, `Calculator-definitions.cc` (partial) | `fallback-only` |
| `src/number.rs` | `Number.h`, `Number.cc` | mixed: focused native evidence, scaffold, unstarted |
| `src/batch.rs` | — (no upstream equivalent; native batch parser) | `tooling-pass` |
| `src/main.rs` | `../src/qalc.cc` (partial CLI parity) | mixed |

### Upstream Files with No Rust Counterpart

37 of 41 `.cc` implementation responsibilities remain unstarted (Calculator conversion/parsing/plot APIs, all MathStructure files, all BuiltinFunctions files, ExpressionItem, Function, Variable, Unit, Prefix, DataSet, DateTime, and util families).

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
    scaffold: 3
    fallback_only: 1
    unstarted: 14
    out_of_scope: 4
  implementation_files:
    native_pass: 0
    tooling_pass: 0
    scaffold: 4
    fallback_only: 3
    unstarted: 34
    out_of_scope: 0
  definition_data:
    native_pass: 0
    tooling_pass: 0
    scaffold: 0
    fallback_only: 0
    unstarted: 9
    out_of_scope: 0
  batch_tests:
    native_pass: 1
    tooling_pass: 0
    scaffold: 4
    fallback_only: 0
    unstarted: 12
    out_of_scope: 0
  batch_test_cases:
    native_pass: 237
    tooling_pass: 0
    scaffold: 0
    fallback_only: 0
    unstarted: 419
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
