# Master Porting Plan: libqalculate C++ to Rust

This plan drives a complete Rust port of `libqalculate` 5.11.0, using the adjacent
`../libqalculate` checkout as the compatibility oracle. The goal is not a subset
calculator. The final state must cover the public library surface, `qalc` CLI behavior,
definition data, batch fixtures, diagnostics, and output formatting that upstream exposes.

The plan is written for autonomous coding agents. Each task must become a GitHub issue
before implementation. Each issue must be small enough to complete with tests in one focused
change and must name the upstream files, fixtures, and Rust modules that prove
compatibility. Each implementation issue is completed through a linked pull request, review
fixes, and merge to `main`.

## Non-Negotiable Completion Contract

A feature is not complete until all of the following are true:

1. A GitHub issue exists with a complete task packet and a linked PR.
2. The relevant upstream C++ headers, implementation files, data files, and batch fixtures
   are named in the task packet.
3. Rust tests are added before or with the implementation and assert user-visible behavior.
4. The Rust behavior passes without calling the C++ fallback for that feature.
5. Differential oracle evidence compares Rust output with upstream `qalc` or a documented
   upstream fixture.
6. Any intentional divergence is recorded in `docs/deviations.md` with a test that locks it.
7. Required quality gates and review skills from `docs/quality-gates.md` and
   `docs/agent_skills_mapping.md` have been run, with evidence in the handoff.
8. Review findings have been resolved or converted into accepted follow-up issues before
   merge.

The final port is complete only when every upstream public API family and every
`../libqalculate/tests/*.batch` case is passing, or has an approved deviation or documented
out-of-scope rationale in the oracle manifest.

## Task Sizing Reference

Tasks larger than M must be split before assignment.

| Size | Files | Scope |
| --- | --- | --- |
| XS | 1 | Single function, fixture, config entry, or narrow bug fix. |
| S | 1-2 | One self-contained type, parser rule, wrapper, or fixture slice. |
| M | 3-5 | One vertical feature slice with parser/eval/format tests. |
| L | 5-8 | Too large for direct execution. Split into S/M tasks first. |

## Required Task Packet

Every task in this plan must be expanded into this packet inside a GitHub issue before an
agent starts work:

```md
GitHub issue:
Tracking issue / milestone:
Task ID / epic / size:
Dependencies:
Rust owner modules:
Architecture boundary:
Upstream oracle files:
  - Headers:
  - Implementation:
  - Data:
  - Tests:
User-visible behavior:
Intentional divergences:
Prior lessons consulted:
Tests to add first:
Hygiene checkpoints:
Required gates:
Required review skills:
PR link:
Completion evidence:
```

## Upstream Compatibility Inventory

Agents must maintain an inventory that maps upstream features to Rust owner modules,
tests, status, and deviations. This table seeds that inventory.

| Area | Upstream sources | Upstream fixtures/data | Rust owner modules |
| --- | --- | --- | --- |
| Calculator state, parsing, evaluation, conversion, messages | `Calculator*.h`, `Calculator*.cc`, `includes.h` | all `.batch` files | `calculator`, `context`, `messages` |
| Number, precision, intervals, uncertainty, complex values | `Number.h`, `Number.cc` | `parser.batch`, `operators.batch`, `numberbase.batch` | `number` |
| MathStructure AST and symbolic transforms | `MathStructure*.h`, `MathStructure*.cc` | `polynomial.batch`, `solver.batch`, `limits.batch`, `calculus.batch` | `ast`, `simplify`, `symbolic` |
| Operators, comparisons, logical, bitwise, percentages, bases | `Calculator-parse.cc`, `BuiltinFunctions-logical.cc`, `MathStructure-calculate.cc` | `operators.batch`, `bitwise.batch`, `percentages.batch`, `numberbase.batch` | `parser`, `eval`, `operators` |
| Built-in functions | `BuiltinFunctions*.cc`, `Function.h` | `explog.batch`, `calculus.batch`, `stats.batch`, `geometry.batch` | `functions` |
| Units, prefixes, conversion | `Unit.h`, `Prefix.h`, `Calculator-convert.cc`, `MathStructure-convert.cc` | `units.batch`, `units.xml.in`, `prefixes.xml.in` | `units`, `definitions` |
| Variables, assumptions, expression items | `Variable.h`, `ExpressionItem.h`, `Calculator-definitions.cc` | `variables.batch`, `variables.xml.in` | `variables`, `definitions` |
| Datasets, elements, planets, object properties | `DataSet.h`, `DataSet.cc` | `datasets.xml.in`, `elements.xml.in`, `planets.xml.in`, `stats.batch` | `datasets`, `definitions` |
| Currencies and exchange rates | `Unit.h`, `Calculator-convert.cc` | `currencies.xml.in`, `rates.json`, `units.batch` | `currencies`, `rates` |
| Vectors, matrices, statistics | `BuiltinFunctions-matrixvector.cc`, `BuiltinFunctions-statistics.cc`, `MathStructure-matrixvector.cc` | `matrixvector.batch`, `stats.batch`, `vectordata*.csv` | `vectors`, `matrix`, `statistics` |
| Dates and calendars | `QalculateDateTime.h`, `QalculateDateTime.cc`, `BuiltinFunctions-datetime.cc` | `dates.batch` | `datetime` |
| Strings and Unicode | `util.h`, `util.cc`, `BuiltinFunctions-util.cc` | `strings.batch` | `text`, `parser`, `format` |
| Output formatting and print options | `MathStructure-print.cc`, `Calculator.h`, `includes.h` | all `.batch` files | `format` |
| CLI, sessions, completion, history, stdin/stdout | `src/qalc.cc`, `src/test.cc`, `src/unittest.cc` | all `.batch` files | `cli` |

The upstream batch inventory for 5.11.0 is:

`bitwise.batch`, `calculus.batch`, `dates.batch`, `explog.batch`, `geometry.batch`,
`limits.batch`, `matrixvector.batch`, `numberbase.batch`, `operators.batch`,
`parser.batch`, `percentages.batch`, `polynomial.batch`, `solver.batch`,
`stats.batch`, `strings.batch`, `units.batch`, and `variables.batch`.

The upstream source definition-data inventory is the `*.xml.in` files plus `rates.json`.
Generated upstream `*.xml` files and `eurofxref-daily.xml` are not counted as separate
source inventory unless a later task decides to track generated artifacts explicitly:

`currencies.xml.in`, `datasets.xml.in`, `elements.xml.in`, `functions.xml.in`,
`planets.xml.in`, `prefixes.xml.in`, `rates.json`, `units.xml.in`, and
`variables.xml.in`.

## Epic 0: Compatibility Inventory and Oracle Harness

Goal: create the evidence machinery before porting semantics.

### Task 0.1: upstream-feature-matrix (Size: S | Priority: High)

Create a checked-in inventory that maps public headers, implementation files, data files,
batch fixtures, and CLI behavior to Rust owner modules and parity status.

Acceptance criteria:

- Public API families from `../libqalculate/libqalculate/qalculate.h` are listed.
- All 17 upstream `.batch` files and CSV assets are listed.
- All 9 definition data files are listed.
- Each entry has status: `unstarted`, `scaffold`, `native-pass`, `tooling-pass`, `fallback-only`,
  `approved-deviation`, or `out-of-scope`.

### Task 0.2: public-api-parity-matrix (Size: M | Priority: High)

Inventory public constructors, methods, enums, and option structs from `Calculator`,
`MathStructure`, `Number`, `ExpressionItem`, `Variable`, `Function`, `DataSet`, `Unit`,
and `Prefix`.

Acceptance criteria:

- Each public upstream symbol is mapped to a Rust API, a pending task, or an approved
  omission.
- FFI-only interim APIs are clearly marked and cannot count as native parity.
- Breaking API differences require `code-review-breaking-changes`.

### Task 0.3: batch-fixture-inventory (Size: S | Priority: High)

Build an oracle manifest for every upstream batch case and required asset.

Acceptance criteria:

- Each case has file name, stable case id, feature tags, required session settings,
  required data files, normalization policy, parity status, and deviation id if any.
- Unclassified upstream cases fail the inventory check.
- Session commands such as `/set` are represented, not discarded.

### Task 0.4: differential-oracle-runner (Size: M | Priority: High)

Implement a runner that executes the same fixture/session against upstream `qalc` and
native Rust `qalc-rs`.

Acceptance criteria:

- Preserves session state across commands in a batch file.
- Captures stdout, stderr, diagnostics/messages, exit status, and locale/timezone inputs.
- Defaults to exact UTF-8 output comparison.
- Any normalization or tolerance must reference `docs/deviations.md`.
- A passing inventory-only oracle test is never reported as parity.

### Task 0.5: agent-task-template (Size: XS | Priority: High)

Add templates for task packets, oracle evidence, and final handoff.

Acceptance criteria:

- Templates are linked from `docs/agent_skills_mapping.md`.
- Templates require upstream files, tests, gates, review skills, and completion evidence.

## Epic 1: Workspace Foundation and Optional C++ Oracle/FFI

Goal: keep the project buildable while using upstream as oracle and, where necessary,
as a temporary fallback. FFI fallback is a transition tool, not native completion.

### Task 1.1: hybrid-build-inventory (Size: S | Priority: High)

Document how `build.rs` maps to upstream configure features and libraries.

Acceptance criteria:

- GMP, MPFR, libxml2, pthread/threading, libcurl, ICU/localization, and platform C++
  runtime behavior are either supported or explicitly documented.
- Unsupported upstream configure features are tracked in the compatibility inventory.

### Task 1.2: ffi-sys-bindings (Size: S | Priority: High)

Generate or write minimal raw bindings for oracle/fallback use.

Acceptance criteria:

- Raw bindings live in `ffi::sys`.
- Bindings are allowlisted, reproducible, and not manually edited.
- C++ exceptions cannot cross the Rust FFI boundary.
- `unsafe` is confined to modules with explicit review notes.

### Task 1.3: safe-ffi-calculator-wrapper (Size: M | Priority: High)

Expose a safe wrapper around the upstream calculator for oracle comparisons and temporary
fallback execution.

Acceptance criteria:

- Wrapper owns or borrows every C++ handle with documented lifetime rules.
- Access to upstream global calculator state is serialized and restored after each run.
- Tests prove wrapper construction, calculation, message extraction, and cleanup.

### Task 1.4: no-cpp-fallback-gate (Size: S | Priority: High)

Add a test/configuration path proving selected feature slices run natively.

Acceptance criteria:

- Ported tasks can disable C++ fallback for their feature area.
- A feature cannot be marked `native-pass` when its output came from C++ fallback.

## Epic 2: Numeric Core (`Number`)

Goal: port arbitrary precision values, exact rationals, floats, complex values, intervals,
uncertainty, infinities, NaN, and precision state.

### Task 2.1: number-representation (Size: S | Priority: High)

Implement the Rust value model and invariants.

Acceptance criteria:

- Covers finite exact, arbitrary float, complex, interval, uncertainty, infinity, and NaN.
- Deep clone semantics match upstream ownership expectations.
- No safe API exposes borrowed raw GMP/MPFR internals.

### Task 2.2: rational-arithmetic (Size: S | Priority: High)

Implement exact integer and rational arithmetic.

Acceptance criteria:

- GCD reduction, sign normalization, zero handling, and exact comparison are tested.
- `operators.batch`, `parser.batch`, and focused unit fixtures cover exact cases.

### Task 2.3: arbitrary-precision-float (Size: M | Priority: High)

Implement precision-aware floating arithmetic.

Acceptance criteria:

- Precision settings from calculator context are respected.
- No lossy `f64` shortcut is used for semantic arithmetic.
- Division by zero, overflow-like behavior, NaN, and infinities match upstream.

### Task 2.4: complex-operations (Size: S | Priority: Medium)

Implement complex representation and arithmetic.

Acceptance criteria:

- Real and imaginary parts preserve exact/approximate flags.
- Addition, subtraction, multiplication, division, conjugate, norm, and formatting are tested.

### Task 2.5: interval-core (Size: M | Priority: Medium)

Implement interval storage, ordering, and outward-rounded arithmetic.

Acceptance criteria:

- Addition, subtraction, multiplication, division, intersections, open/closed bounds, and
  infinities are tested against upstream.

### Task 2.6: uncertainty-core (Size: M | Priority: Medium)

Implement uncertainty parsing, propagation, and formatting as a separate slice from
interval arithmetic.

Acceptance criteria:

- Uncertainty forms are parsed and printed.
- Propagation rules are tested against upstream fixtures and focused oracle cases.

## Epic 3: AST, Parser, and Session Commands

Goal: build the Rust `MathStructure` model and parse user input, including session commands
used by upstream batch files.

### Task 3.1: mathstructure-model (Size: S | Priority: High)

Define enum-backed AST nodes for numbers, symbols, functions, units, vectors, matrices,
comparisons, logical forms, bitwise forms, and datetimes.

Acceptance criteria:

- Child order and operator associativity are explicit.
- Recursive ownership uses `Box`, `Vec`, or interned IDs without cycles.
- Definition references use stable IDs or handles, not raw pointer identity.

### Task 3.2: lexer-core (Size: M | Priority: High)

Tokenize numbers, names, units, operators, grouping, comments, strings, and command lines.

Acceptance criteria:

- Handles Unicode input, scientific notation, base prefixes, operator combinations, and
  interior NUL rejection.
- Tests include `parser.batch`, `strings.batch`, and command examples.

### Task 3.3: parser-operators (Size: M | Priority: High)

Parse arithmetic operators, implicit multiplication, comparisons, logical operators,
bitwise operators, and percentages.

Acceptance criteria:

- Precedence and associativity match upstream.
- Invalid syntax returns structured errors and messages without panic.

### Task 3.4: parser-functions-units-variables (Size: M | Priority: High)

Parse function calls, variables, assumptions, units, prefixes, and unit exponents.

Acceptance criteria:

- Names are resolved with upstream-compatible ambiguity rules.
- `units.batch`, `variables.batch`, and `parser.batch` cases are covered.

### Task 3.5: session-command-parser (Size: S | Priority: High)

Parse qalc batch/session commands such as `/set`, mode changes, and file/data commands.

Acceptance criteria:

- Session state changes are represented in the oracle manifest.
- Command cases in upstream batches are not silently skipped.

## Epic 4: Calculator Context, Evaluation, and Options

Goal: port evaluation state, options, message queues, and primitive evaluation.

### Task 4.1: calculator-context (Size: M | Priority: High)

Implement explicit `CalculatorContext` ownership for precision, angle units, base settings,
interval options, messages, definitions, and session state.

Acceptance criteria:

- No hidden mutable global state is used in Rust APIs.
- Any C++ fallback global access is mutex-guarded with save/restore boundaries.
- Non-fatal warnings and fatal errors preserve upstream ordering and categories.

### Task 4.2: option-parity-slice (Size: M | Priority: High)

Port parse, evaluate, and print option structs from upstream `includes.h` and `Calculator.h`.

Acceptance criteria:

- Options for bases, Unicode, decimal limits, complex form, interval calculation,
  approximation, date/time format, unit conversion, and angle units are inventoried.
- Each option has tests or a pending task.

### Task 4.3: primitive-evaluator (Size: M | Priority: High)

Evaluate numbers, variables, arithmetic, comparisons, logical operators, and unitless
function calls.

Acceptance criteria:

- Variable lookup and message emission match upstream.
- Oracle fixtures prove no C++ fallback for the covered operations.

### Task 4.4: simplification-core (Size: M | Priority: Medium)

Implement constant folding, neutral elements, term collection, and basic factorization.

Acceptance criteria:

- User-visible simplification output matches upstream print behavior.
- Tests include both AST-level assertions and oracle output.

## Epic 5: Operators, Bases, Comparisons, Logic, Bitwise, and Percentages

Goal: cover core expression semantics beyond basic arithmetic.

Tasks:

- `operator-parity` (M): `operators.batch`.
- `number-base-parity` (M): `numberbase.batch`, base formatting options, two's complement.
- `comparison-logic-parity` (M): comparisons, boolean/logical functions, truthiness.
- `bitwise-parity` (M): `bitwise.batch`, shifts, masks, signedness options.
- `percentage-parity` (S): `percentages.batch`, percent-change behavior.

Each task must include parse, evaluate, format, diagnostics, and no-fallback oracle evidence.

## Epic 6: Built-In Function Catalog

Goal: port every upstream built-in function family.

Tasks:

- `explog-functions` (M): `BuiltinFunctions-explog.cc`, `explog.batch`.
- `trigonometry-functions` (M): trigonometric and angle-unit behavior.
- `algebra-number-special-functions` (M): algebra, number, and special functions.
- `combinatorics-functions` (S): factorial, binomial, permutations.
- `geometry-functions` (S): geometry fixture coverage.
- `utility-string-functions` (M): utility and string functions, `strings.batch`.

Acceptance criteria for every task:

- Function signatures, argument validation, messages, approximation behavior, and output
  formatting are tested.
- The function registry is loaded through Rust definitions, not hard-coded ad hoc names
  unless upstream does the same.

## Epic 7: Symbolic Algebra, Solver, Limits, and Calculus

Goal: port symbolic manipulation visible to users.

Tasks:

- `polynomial-factor-gcd-decompose` (M): `MathStructure-polynomial.cc`,
  `MathStructure-factor.cc`, `MathStructure-gcd.cc`, `MathStructure-decompose.cc`,
  `polynomial.batch`.
- `solver-isolate` (M): `MathStructure-isolatex.cc`, `solver.batch`.
- `limits` (M): `MathStructure-limit.cc`, `limits.batch`.
- `differentiate` (M): `MathStructure-differentiate.cc`, `calculus.batch`.
- `integrate` (M): `MathStructure-integrate.cc`, `calculus.batch`.

Each task must include unsupported-case diagnostics and exact upstream output comparisons.

## Epic 8: Vectors, Matrices, Statistics, and CSV Data

Goal: port collection types and data-driven statistical behavior.

Tasks:

- `vector-matrix-ast-eval` (M): vector/matrix AST, indexing, arithmetic,
  `matrixvector.batch`.
- `matrix-functions` (M): determinant, transpose, inverse, and shape errors.
- `statistics-functions` (M): `BuiltinFunctions-statistics.cc`, `stats.batch`.
- `csv-data-loading` (S): `vectordata.csv`, `vectordata2.csv`, working-directory rules.

Acceptance criteria:

- Shape errors and diagnostics match upstream.
- CSV asset paths and session state are modeled in the oracle manifest.

## Epic 9: Definitions, Units, Datasets, Currencies, and Rates

Goal: load and evaluate all upstream definition data.

Tasks:

- `xml-loader-core` (M): parse XML with provenance and recoverable errors.
- `prefix-unit-loader` (M): `prefixes.xml.in`, `units.xml.in`, `units.batch`.
- `function-variable-loader` (M): `functions.xml.in`, `variables.xml.in`,
  `variables.batch`.
- `datasets-elements-planets` (M): `datasets.xml.in`, `elements.xml.in`,
  `planets.xml.in`.
- `currency-rate-loader` (M): `currencies.xml.in`, `rates.json`, offline rate behavior.
- `unit-conversion-engine` (M): compound units, reductions, prefixes, and conversion modes.

Acceptance criteria:

- Copied or generated data has provenance and a refresh path.
- Unknown XML tags do not panic.
- Conversion ratios and dataset lookups match upstream.

## Epic 10: Dates, Calendars, and Time

Goal: port date/time values and functions.

Tasks:

- `datetime-value-model` (S): `QalculateDateTime` representation and invariants.
- `datetime-parser-formatter` (M): date/time input and print options.
- `datetime-functions` (M): `BuiltinFunctions-datetime.cc`, `dates.batch`.
- `timezone-locale-policy` (S): deterministic test configuration for locale/timezone.

Acceptance criteria:

- Tests pin timezone and locale inputs.
- Calendar arithmetic, invalid dates, and formatting diagnostics match upstream.

## Epic 11: Formatting, PrintOptions, and Output Modes

Goal: match upstream text output, including modes used by CLI and APIs.

Tasks:

- `number-formatting` (M): decimal, scientific, engineering, bases, precision limits.
- `ast-text-printer` (M): `MathStructure-print.cc` text output.
- `latex-html-output` (M): LaTeX/HTML modes and escaping.
- `unicode-ascii-output` (S): Unicode toggles, symbols, and character-width assumptions.
- `message-formatting` (S): warnings/errors and CLI display.

Acceptance criteria:

- Exact UTF-8 output is the default oracle policy.
- Formatting options are covered by batch and focused tests.

## Epic 12: Full CLI and Public API Parity

Goal: expose a Rust library and `qalc-rs` CLI that match upstream user workflows.

Tasks:

- `cli-flags` (M): mirror upstream flags from `src/qalc.cc`.
- `cli-repl-session` (M): interactive mode, session options, completion, and history.
- `stdin-pipeline` (S): pipeline input/output behavior.
- `batch-test-file-mode` (S): native `--test-file` execution for oracle runs.
- `public-api-surface` (M): Rust equivalents for public library methods.
- `docs-examples-parity` (S): examples from upstream docs/tests run or have deviations.

Acceptance criteria:

- CLI behavior is tested as a user would run it.
- Public API differences are reviewed as breaking changes.
- `qalc-rs` can execute upstream batch fixtures natively for completed feature areas.

## Epic 13: Hardening, CI, and Release Readiness

Goal: make parity continuously enforceable.

Tasks:

- `ci-oracle-setup` (M): build or locate upstream 5.11.0 `qalc` and run differential tests.
- `coverage-threshold` (S): enforce coverage thresholds for ported modules.
- `fuzz-campaigns` (M): parser, formatter, evaluator, XML/data loaders, CLI commands.
- `mutation-campaigns` (M): scoped mutation tests for changed semantic modules.
- `unsafe-audit` (S): review every unsafe block, generated binding, and FFI wrapper.

Acceptance criteria:

- No undocumented skips in CI.
- Fuzz crashes are reduced into regression fixtures.
- Surviving mutants are either killed by tests or documented as equivalent.

## Progress Checkpoints

| Checkpoint | Targets | Verification |
| --- | --- | --- |
| C0: Inventory | Feature/API matrix, batch manifest, deviations registry | inventory check, `just test-smoke` |
| C1: Oracle Harness | Differential runner can compare at least one native Rust feature slice | `just test-oracle` with Rust-vs-C++ comparison |
| C2: Native Arithmetic | Number core and arithmetic pass without fallback | `just test-unit`, `just test-oracle` |
| C3: Parser and Commands | Parser/session commands cover upstream command cases | `just test-property`, `just test-regression` |
| C4: Evaluation | Evaluator, options, messages, and formatting pass selected batches | `just test-oracle`, `just coverage` |
| C5: Data and CLI | Definitions, units, datasets, currencies, dates, and CLI workflows pass | `just test-e2e`, `just test-oracle` |
| C6: Full Parity | All upstream public APIs and batch cases pass or have approved deviations | `just deep`, full oracle manifest report |

The current repository is a quality scaffold. Scaffold checks are valuable, but they do not
prove feature parity until the differential oracle runner and native feature slices exist.
