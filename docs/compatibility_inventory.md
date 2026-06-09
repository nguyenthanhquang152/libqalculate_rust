# Compatibility Inventory — libqalculate → libqalculate_rust

> **Upstream version**: libqalculate 5.11.0
> **Inventory date**: 2026-06-09
> **Epic**: 0 — Project Bootstrap & Inventory
> **Tasks**: 0.1 (Source Inventory), 0.2 (Public API Parity Matrix)

---

## Summary Statistics

| Category | Total | native-pass | scaffold | fallback-only | unstarted | out-of-scope |
|---|---|---|---|---|---|---|
| Public Headers | 22 | 0 | 3 | 1 | 14 | 4 |
| Implementation Files | 41 | 0 | 1 | 6 | 34 | 0 |
| Definition Data Files | 9 | 0 | 0 | 0 | 9 | 0 |
| Batch Test Files | 17 | 0 | 0 | 0 | 17 | 0 |
| Batch Test Cases | 656 | 0 | 0 | 0 | 656 | 0 |
| CLI Behaviors | 10 | 5 | 0 | 1 | 4 | 0 |
| Core Class API Groups | 59 | 0 | 6 | 6 | 47 | 0 |

**Overall porting progress**: Early stage. Only the `Number` type has a scaffold, `Calculator` has an FFI fallback bridge, and the CLI (`qalc-rs`) has native batch-parsing and self-check support. All core computation, definition loading, and evaluation paths are either unstarted or fallback-only.

---

## Status Legend

| Status | Meaning |
|---|---|
| `native-pass` | Fully ported to Rust; passes relevant upstream tests natively |
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
| 1 | `Calculator.h` | `src/ffi.rs` | `fallback-only` | FFI wrapper to C++ `Calculator`; exposes `calculate()`, `loadDefinitions()` via `cxx` bridge |
| 2 | `Calculator_p.h` | — | `out-of-scope` | Private implementation detail of Calculator |
| 3 | `MathStructure.h` | — | `unstarted` | Core expression tree; ~120 public methods |
| 4 | `MathStructure_p.h` | — | `out-of-scope` | Private implementation detail of MathStructure |
| 5 | `MathStructure-support.h` | — | `unstarted` | Internal support macros for MathStructure operations |
| 6 | `Number.h` | `src/number.rs` | `scaffold` | `NumberValue` enum, `Rational`, `Float` types defined; no MPFR backend |
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
| 1 | `Calculator.cc` | Core calculator state, construction, messages | `fallback-only` via `src/ffi.rs` |
| 2 | `Calculator-calculate.cc` | Expression evaluation engine | `fallback-only` via `src/ffi.rs` |
| 3 | `Calculator-convert.cc` | Unit/base conversion | `fallback-only` via `src/ffi.rs` |
| 4 | `Calculator-definitions.cc` | Definition loading from XML | `fallback-only` via `src/ffi.rs` |
| 5 | `Calculator-parse.cc` | Expression parsing | `fallback-only` via `src/ffi.rs` |
| 6 | `Calculator-plot.cc` | Gnuplot integration | `fallback-only` via `src/ffi.rs` |

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
| 12 | `MathStructure-matrixvector.cc` | Matrix and vector operations | `unstarted` |
| 13 | `MathStructure-polynomial.cc` | Polynomial arithmetic | `unstarted` |
| 14 | `MathStructure-print.cc` | Expression formatting/printing | `unstarted` |

### Number Family (1 file)

| # | C++ File | Responsibility | Rust Status |
|---|---|---|---|
| 1 | `Number.cc` | Arbitrary-precision arithmetic (MPFR) | `scaffold` via `src/number.rs` |

### BuiltinFunctions Family (12 files)

| # | C++ File | Responsibility | Rust Status |
|---|---|---|---|
| 1 | `BuiltinFunctions-algebra.cc` | Algebraic functions (solve, simplify) | `unstarted` |
| 2 | `BuiltinFunctions-calculus.cc` | Calculus functions (diff, integrate, limit) | `unstarted` |
| 3 | `BuiltinFunctions-datetime.cc` | Date/time functions | `unstarted` |
| 4 | `BuiltinFunctions-explog.cc` | Exponential and logarithmic functions | `unstarted` |
| 5 | `BuiltinFunctions-matrixvector.cc` | Matrix/vector functions | `unstarted` |
| 6 | `BuiltinFunctions-number.cc` | Number theory functions | `unstarted` |
| 7 | `BuiltinFunctions-combinatorics.cc` | Combinatorics functions | `unstarted` |
| 8 | `BuiltinFunctions-logical.cc` | Logical/comparison functions | `unstarted` |
| 9 | `BuiltinFunctions-statistics.cc` | Statistical functions | `unstarted` |
| 10 | `BuiltinFunctions-trigonometry.cc` | Trigonometric functions | `unstarted` |
| 11 | `BuiltinFunctions-unit.cc` | Unit-related functions | `unstarted` |
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
| `fallback-only` | Calculator | 6 |
| `scaffold` | Number | 1 |
| `unstarted` | MathStructure, BuiltinFunctions, ExpressionItem, Function, Variable, Unit, Prefix, DataSet, DateTime, Utility | 34 |

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
| 4 | `explog.batch` | 10 | 1 | — | `unstarted` |
| 5 | `geometry.batch` | 30 | 0 | — | `unstarted` |
| 6 | `limits.batch` | 181 | 4 | — | `unstarted` |
| 7 | `matrixvector.batch` | 130 | 0 | `vectordata.csv`, `vectordata2.csv` | `unstarted` |
| 8 | `numberbase.batch` | 15 | 3 | — | `unstarted` |
| 9 | `operators.batch` | 30 | 0 | — | `unstarted` |
| 10 | `parser.batch` | 27 | 0 | — | `unstarted` |
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
| Files with session settings | 5 |
| Files requiring CSV assets | 2 |
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
| `vectordata.csv` | `matrixvector.batch`, `stats.batch` | Numeric vector/matrix test data |
| `vectordata2.csv` | `matrixvector.batch`, `stats.batch` | Additional numeric vector/matrix test data |

---

## 5. CLI Behaviors Inventory

Maps upstream `qalc` CLI flags and behaviors to `qalc-rs` implementation status.

| # | CLI Behavior | Upstream (`qalc`) | Rust (`qalc-rs`) | Status | Notes |
|---|---|---|---|---|---|
| 1 | `--version` | Prints version | Prints version | `native-pass` | |
| 2 | `--help` | Prints usage | Prints usage | `native-pass` | |
| 3 | `--self-check` | — | Runs self-diagnostics | `native-pass` | Rust-only addition |
| 4 | `--list-upstream-tests` | — | Lists upstream batch files | `native-pass` | Rust-only addition |
| 5 | `--parse-batch` | — | Parses batch file structure | `native-pass` | Native batch parser; no evaluation |
| 6 | Expression evaluation | Evaluates via Calculator | Evaluates via FFI bridge | `fallback-only` | Delegates to C++ `Calculator::calculate()` |
| 7 | `-defaults` | Reset to default settings | — | `unstarted` | |
| 8 | `-set <option> <value>` | Set calculator option | — | `unstarted` | |
| 9 | `--test-file <path>` | Run batch test file | — | `unstarted` | `qalc-rs` has `--parse-batch` but no evaluation |
| 10 | Interactive REPL | Line-editing REPL | — | `unstarted` | |

### CLI Status Summary

| Status | Count |
|---|---|
| `native-pass` | 5 |
| `fallback-only` | 1 |
| `unstarted` | 4 |

---

## 6. Public API Parity Matrix

For each of the 9 core C++ classes, maps major public API categories to Rust implementation status with method-count estimates.

### 6.1 Calculator (`Calculator.h` → `src/ffi.rs`)

| # | API Category | Est. Methods | Rust Status | Notes |
|---|---|---|---|---|
| 1 | Construction / destruction | 2 | `scaffold` | `Calculator::new()` creates C++ instance via FFI |
| 2 | Definition loading | 3 | `scaffold` | `loadGlobalDefinitions()` exposed via FFI |
| 3 | Expression parsing | 5 | `fallback-only` | `parse()`, `parseNumber()` via FFI |
| 4 | Expression evaluation | 8 | `fallback-only` | `calculate()`, `calculateAndPrint()` via FFI |
| 5 | Conversion | 6 | `fallback-only` | `convert()`, `convertToBaseUnits()` via FFI |
| 6 | Settings / options | 15+ | `fallback-only` | Precision, angle mode, approximation, etc. |
| 7 | Messages | 4 | `fallback-only` | `message()`, `nextMessage()`, message queue |
| 8 | Plot support | 3 | `fallback-only` | Gnuplot integration |

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
| 1 | NumberType enum | 1 | `scaffold` | `NumberValue` enum maps types (Rational, Float, PlusInfinity, MinusInfinity, etc.) |
| 2 | Construction | 8 | `scaffold` | Basic constructors exist (`from_i64`, `from_rational`, etc.) |
| 3 | Setters | 6 | `unstarted` | `set()`, `setFloat()`, `setInterval()` |
| 4 | Arithmetic | 15 | `unstarted` | `add`, `subtract`, `multiply`, `divide`, `power`, `root`, `negate`, `abs`, `mod`, etc. |
| 5 | Comparison | 6 | `scaffold` | `PartialEq` for value equality; `<`, `>`, `<=`, `>=` not yet implemented |
| 6 | Predicates | 12 | `scaffold` | `is_zero()`, `is_one()`, `is_positive()`, `is_negative()`, `is_integer()` exist as stubs |
| 7 | Conversion | 5 | `unstarted` | `intValue()`, `floatValue()`, `to_string()` |
| 8 | Interval operations | 4 | `unstarted` | `setInterval()`, `isInterval()`, interval arithmetic |
| 9 | Uncertainty | 3 | `unstarted` | `uncertainty()`, `setUncertainty()` |
| 10 | Precision | 4 | `unstarted` | `precision()`, `setPrecision()`, MPFR precision control |
| 11 | Format / print | 3 | `unstarted` | `print()`, `format()`, base conversion display |

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

| Class | Total Categories | scaffold | fallback-only | unstarted |
|---|---|---|---|---|
| Calculator | 8 | 2 | 6 | 0 |
| MathStructure | 14 | 0 | 0 | 14 |
| Number | 11 | 4 | 0 | 7 |
| ExpressionItem | 4 | 0 | 0 | 4 |
| Variable | 4 | 0 | 0 | 4 |
| Function | 5 | 0 | 0 | 5 |
| DataSet | 4 | 0 | 0 | 4 |
| Unit | 5 | 0 | 0 | 5 |
| Prefix | 4 | 0 | 0 | 4 |
| **Total** | **59** | **6** | **6** | **47** |

---

## Appendix A: File Cross-Reference

### Rust Source Files → Upstream Mappings

| Rust File | Upstream Coverage | Status |
|---|---|---|
| `src/lib.rs` | `includes.h` (partial), crate root | `scaffold` |
| `src/ffi.rs` | `Calculator.h`, `Calculator*.cc` (6 files) | `fallback-only` |
| `src/number.rs` | `Number.h`, `Number.cc` | `scaffold` |
| `src/batch.rs` | — (no upstream equivalent; native batch parser) | `native-pass` |
| `src/main.rs` | `../src/qalc.cc` (partial CLI parity) | mixed |

### Upstream Files with No Rust Counterpart

34 of 41 `.cc` files have no Rust counterpart yet (all MathStructure, BuiltinFunctions, ExpressionItem, Function, Variable, Unit, Prefix, DataSet, DateTime, and util families).

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
inventory_date: 2026-06-09
epic: 0
tasks: [0.1, 0.2]

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
    scaffold: 3
    fallback_only: 1
    unstarted: 14
    out_of_scope: 4
  implementation_files:
    native_pass: 0
    scaffold: 1
    fallback_only: 6
    unstarted: 34
    out_of_scope: 0
  definition_data:
    native_pass: 0
    scaffold: 0
    fallback_only: 0
    unstarted: 9
    out_of_scope: 0
  batch_tests:
    native_pass: 0
    scaffold: 0
    fallback_only: 0
    unstarted: 17
    out_of_scope: 0
  cli_behaviors:
    native_pass: 5
    scaffold: 0
    fallback_only: 1
    unstarted: 4
    out_of_scope: 0
  api_categories:
    native_pass: 0
    scaffold: 6
    fallback_only: 6
    unstarted: 47
    out_of_scope: 0
-->
