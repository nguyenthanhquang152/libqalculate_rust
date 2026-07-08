# Public API Parity Matrix (Task 0.2)

> Upstream version: libqalculate 5.11.0
> Scope: inventory and classification only; this document does not claim native Rust API parity.

## Extraction Method

The Task 0.2 parity matrix classifies public API declarations from the required upstream headers by header and API family. Each public symbol in a listed header inherits the row status for its owning header/family unless a later implementation issue splits it more narrowly. Native Rust public API implementation remains tracked by #64 (`public-api-surface`).

FFI-only rows name the actual Rust wrapper and cannot be counted as `native-pass`. Proposed Rust API shape differences are not approved deviations unless they appear in `docs/deviations.md`.

## Required Header Matrix

| # | Upstream Header | Public Symbol Scope | Rust Owner | Status | FFI-only Native? | Follow-up |
|---|---|---|---|---|---|---|
| 1 | `qalculate.h` | Umbrella include surface | crate root docs | `scaffold` | no | #64 |
| 2 | `Calculator.h` | Calculator, messages, options, plotting, expression evaluation entrypoints | `src/ffi.rs`, future `calculator` | `fallback-only` | no | #21, #64 |
| 3 | `MathStructure.h` | Expression tree construction, mutation, evaluation, formatting, algebra helpers | future `ast`, `eval`, `format` | `unstarted` | no | #16, #64 |
| 4 | `Number.h` | Number constructors, arithmetic, predicates, conversion, intervals, uncertainty, precision | `src/number.rs` | `scaffold` | no | #10-#15 |
| 5 | `ExpressionItem.h` | Shared name, registration, metadata, lifecycle APIs | future `definitions` | `unstarted` | no | #42, #64 |
| 6 | `Variable.h` | Known/unknown variables, assumptions, value APIs | future `variables` | `unstarted` | no | #45, #64 |
| 7 | `Function.h` | MathFunction and Argument APIs | future `functions` | `unstarted` | no | #30-#35, #64 |
| 8 | `DataSet.h` | DataSet, DataProperty, DataObject APIs | `src/datasets.rs` | `scaffold` | no | #64 |
| 9 | `Unit.h` | Unit, AliasUnit, CompositeUnit APIs | future `units` | `unstarted` | no | #38-#40, #64 |
| 10 | `Prefix.h` | Decimal, binary, and number prefix APIs | future `units` | `unstarted` | no | #37, #64 |
| 11 | `QalculateDateTime.h` | Date and time APIs | `src/datetime.rs` | `scaffold` | no | #52-#54, #64 |
| 12 | `includes.h` | Public enums, option structs, and constants used by calculator/parser/printing APIs | `src/lib.rs` partial constants, future `options` | `scaffold` | no | #22, #64 |

## API Family Matrix

| Family | Header | Public API Categories | Rust Owner | Status | Follow-up |
|---|---|---|---|---|---|
| Calculator | `Calculator.h` | construction/destruction; definition loading; parsing; evaluation; conversion; settings/options; messages; plot support | `src/ffi.rs`, future `calculator` | mixed: `scaffold`, `fallback-only`, `unstarted` | #21, #22, #64 |
| MathStructure | `MathStructure.h` | node types; construction; tree manipulation; evaluation; simplification; factoring; integration; differentiation; limits; polynomial ops; matrix/vector ops; printing; conversion; comparison | future `ast`, `eval`, `format` | `unstarted` | #16-#24, #64 |
| Number | `Number.h` | number type; construction; setters; arithmetic; comparison; predicates; conversion; interval ops; uncertainty; precision; format/print | `src/number.rs` | mixed: `scaffold`, `unstarted` | #10-#15 |
| ExpressionItem | `ExpressionItem.h` | name management; registration; type identification; properties; lifecycle | future `definitions` | `unstarted` | #42, #64 |
| Variable | `Variable.h` | construction; value get/set; assumptions; known/unknown discrimination | future `variables` | `unstarted` | #45, #64 |
| Function | `Function.h` | construction; argument management; evaluation; condition checks; subfunction support | future `functions` | `unstarted` | #30-#35, #64 |
| DataSet | `DataSet.h` | construction; property management; object lookup; data loading | `src/datasets.rs` | `scaffold` | #64 |
| Unit | `Unit.h` | construction; conversion; composite units; prefix management; base-unit relations | future `units` | `unstarted` | #38-#40, #64 |
| Prefix | `Prefix.h` | construction; value; name management; type identification | future `units` | `unstarted` | #37, #64 |
| Date/time | `QalculateDateTime.h` | construction; calendar conversion; arithmetic; formatting | `src/datetime.rs` | `scaffold` | #52-#54, #64 |
| Options and enums | `includes.h` | parse/evaluation/print options; enum constants; settings structures | `src/lib.rs`, future `options` | `scaffold` | #22, #64 |

## Status Summary

| Metric | Count |
|---|---:|
| Required headers classified | 12 |
| API families classified | 11 |
| Native-pass public API families | 0 |
| FFI-only public API headers | 1 |
| Scaffold public API headers | 4 |
| Unstarted public API headers | 7 |

## Closure Notes

- `Calculator.h` is the only FFI-only public API row and is explicitly tied to `src/ffi.rs`.
- `Number.h`, `DataSet.h`, `includes.h`, and `qalculate.h` are scaffold rows; they do not imply native parity.
- Full Rust public API design and symbol-by-symbol implementation are tracked by #64 and later implementation issues.
- #176 closes the Epic 8 matrix/vector diagnostics follow-up by documenting fail-closed behavior for unsupported diagnostic families; it does not change the `MathStructure.h` public API status.
- #49 adds `src/rates.rs` and focused fallback-disabled explicit currency conversion evidence, but it does not expose or complete the upstream `Unit.h`/`Calculator.h` public conversion APIs; those remain tracked by #50 and #64.
- There are no approved public API deviations in `docs/deviations.md` for this matrix.
