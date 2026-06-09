# Testing and Verification Strategy: libqalculate Port

The Rust port must be proven against the upstream C++ implementation, not merely against
locally invented examples. The adjacent `../libqalculate` checkout is the oracle for
`libqalculate` 5.11.0.

The current repository contains a quality scaffold: batch parsing, fixture inventory, CLI
smoke tests, property tests for batch fixture text, and scripts for local gates. That
scaffold is useful, but it is not yet differential parity. A test that only proves upstream
`qalc` can run, or that Rust can parse a `.batch` file, must be reported as inventory
evidence, not feature parity.

## 1. Definition of Tested Parity

A behavior has tested parity only when:

1. The same input/session is executed by upstream `qalc` and native Rust `qalc-rs`.
2. C++ fallback is disabled for the Rust feature under test.
3. stdout, stderr, diagnostics/messages, exit status, and relevant session state are
   compared through an explicit normalization policy.
4. The upstream source files and fixtures are named in the test or task packet.
5. Any mismatch is fixed or recorded in `docs/deviations.md`.

Inventory-only tests, skipped oracle tests, and tests that route through C++ fallback do not
prove native parity.

## 2. TDD Loop for Porting Tasks

Agents implementing a porting task must follow this loop:

1. Write or select the smallest upstream-derived fixture that describes the behavior.
2. Add a failing Rust test or oracle-manifest entry before implementing the behavior.
3. Implement the smallest native Rust slice that makes the fixture pass without fallback.
4. Run the required local gates from `docs/quality-gates.md`.
5. Run the relevant differential oracle cases.
6. Record upstream files, command output, deviations, and review skills in the handoff.

When upstream behavior is unclear, run upstream `qalc`, inspect the matching C++ source, and
add the observation to the task packet before coding.

## 3. Differential Oracle Runner

`just test-oracle` must evolve into a true differential runner. Until then, documentation and
handoffs must clearly say when the command only checks oracle availability or fixture
inventory.

Required runner behavior:

- Locate upstream `qalc` from `QALCULATE_ORACLE` or `../libqalculate/src/qalc`.
- If the binary is missing, fail parity jobs rather than silently claiming compatibility.
  Inventory jobs may skip with an explicit message.
- Execute upstream and Rust with the same input, working directory, environment, locale,
  timezone, data directories, and session state.
- Preserve session commands such as `/set` and file-loading commands.
- Capture stdout, stderr, queued messages, exit status, and timing/abort outcomes when
  user-visible.
- Compare exact UTF-8 output by default.
- Emit a machine-readable mismatch report with case id, feature tags, command run, and
  normalization policy.

## 4. Oracle Manifest Schema

Every upstream batch case and asset must be tracked. The manifest may be TOML, JSON, CSV, or
generated Rust data, but it must carry these fields:

```text
case_id:
source_file:
source_line:
feature_tags:
input_kind: expression | session-command | file-command | cli
required_assets:
required_settings:
expected_status:
normalization: exact | approved-policy-id
deviation_id:
parity_status: unstarted | inventory-only | fallback-only | native-pass | approved-deviation | out-of-scope
owner:
last_checked_upstream_version:
```

Rules:

- Unclassified upstream cases fail the manifest check.
- `native-pass` requires Rust-vs-C++ comparison with fallback disabled.
- `approved-deviation` requires an entry in `docs/deviations.md`.
- `out-of-scope` requires a reason and reviewer approval. It cannot be used for ordinary
  upstream behavior that the final 100% port is expected to support.

## 5. Fixture and Normalization Policy

Default policy is exact UTF-8 byte-for-byte comparison after stable line-ending handling.

The test harness must model:

- Session commands and option changes.
- Locale, timezone, precision, base, Unicode, approximation, and date/time settings.
- Expected warnings/errors/messages and their order.
- CLI exit status.
- Working directory and data-file assets such as `vectordata.csv`.
- Floating or platform differences only through named policies in `docs/deviations.md`.

Do not add silent tolerances for floats, whitespace, Unicode, date/time, or path formatting.
Any tolerance is a compatibility decision and must be reviewed.

## 6. Testing Layers

| Layer | Command | Purpose |
| --- | --- | --- |
| Unit tests | `just test-unit` | Deterministic internals close to implementation. |
| Integration smoke | `just test-smoke` | Crate exports, upstream fixture availability, and scaffold assumptions. |
| CLI e2e | `just test-e2e` | User-facing binary behavior. |
| Regression fixtures | `just test-regression` | Local fixtures for fixed bugs and reduced oracle cases. |
| Differential oracle | `just test-oracle` | Target/final semantics: Rust-vs-C++ comparison for manifest cases. Current scaffold semantics are described above. |
| Property-based | `just test-property` | Parser, formatter, evaluator, and data invariant stress tests. |
| Fuzzing | `just fuzz` | Crash and panic discovery for parsers, formatters, evaluators, XML/data loaders, and CLI command parsing. |
| Mutation | `just mutation` | Check that semantic tests fail when logic is mutated. |
| Coverage | `just coverage` | Measure coverage and enforce thresholds once configured. |

## 7. Required Coverage by Feature Family

Every semantic change must have user-visible tests. The minimum coverage matrix is:

| Feature family | Required tests |
| --- | --- |
| Number core | Unit tests plus oracle cases for exact, approximate, complex, interval, uncertainty, infinity, NaN. |
| Parser and commands | Unit tests, property tests, fuzz target, `parser.batch`, command cases from all batches. |
| Evaluator and options | Oracle cases for option-dependent behavior, messages, approximation, bases, intervals, and complex forms. |
| Formatter | Round-trip property tests where valid, exact oracle output, Unicode/ASCII toggles. |
| Units and definitions | XML loader tests, fixture provenance tests, `units.batch`, conversion oracle cases. |
| Datasets/currencies/rates | Data loader tests, offline behavior, rates provenance, dataset lookup cases. |
| Symbolic algebra/calculus | Oracle cases from `polynomial.batch`, `solver.batch`, `limits.batch`, `calculus.batch`. |
| Vectors/matrices/statistics | Shape/error unit tests, CSV asset tests, `matrixvector.batch`, `stats.batch`. |
| Dates and strings | Locale/timezone-pinned tests, `dates.batch`, `strings.batch`. |
| CLI/API | `qalc-rs` e2e tests, batch `--test-file`, stdin, flags, session behavior, public API parity tests. |
| FFI/unsafe | Safe-wrapper tests, cleanup/drop tests, concurrency/global-state tests, `unsafe-checker` review. |

## 8. Property-Based Testing

Property tests must expand beyond fixture text round trips as the port grows.

Required properties:

- Parser totality: arbitrary input returns AST or structured error, never panic.
- Formatter/parser round trip for generated valid ASTs where upstream formatting is stable.
- Number identities for exact arithmetic, with domains constrained to avoid undefined cases.
- Unit dimension invariants for multiplication, division, and conversion.
- Date parsing/formatting round trips under pinned timezone/locale.
- XML/data loader totality on malformed or unknown tags.

Properties that depend on upstream output should sample generated cases and compare both
Rust and upstream `qalc` when feasible.

## 9. Fuzzing

Fuzz targets are required for:

- Expression lexer/parser.
- Session command parser.
- Formatter/parser round trip.
- Evaluator entry point with bounded context.
- XML and JSON data loaders.
- Batch/oracle manifest parser.

Crash artifacts must stay under `fuzz/artifacts/` until reduced into regression tests.
Reduced regressions should name the fuzz target and upstream behavior checked.

## 10. Mutation and Coverage

- The target coverage threshold for ported semantic modules is 80% minimum.
- Coverage scripts must fail under threshold once thresholds are configured.
- Mutation testing should be scoped to changed semantic modules when full-crate campaigns are
  too expensive.
- Surviving mutants require either stronger tests or an equivalent-mutant note in the
  handoff.

## 11. Final 100% Parity Criteria

The port is complete only when:

- The public API parity matrix is fully classified.
- Every upstream batch case is `native-pass`, `approved-deviation`, or approved
  `out-of-scope`.
- No ordinary feature relies on C++ fallback.
- All definition data files are loaded or have approved deviations.
- CLI behavior, session commands, stdin, flags, messages, and output modes are covered.
- Coverage, mutation, fuzz, static analysis, rustdoc, and differential oracle gates pass.
- `docs/deviations.md` contains every accepted behavioral difference and no stale entries.
