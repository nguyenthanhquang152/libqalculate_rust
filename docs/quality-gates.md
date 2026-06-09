# Quality Gates

This Rust port uses executable gates to keep C++ to Rust porting work reviewable and
compatible with upstream `../libqalculate`. Gates are evidence, not ceremony: every handoff
must state what was run, what was skipped, and what the result proves.

## Current Gate Semantics

`just quality` is the normal local scaffold gate:

```sh
just quality
```

As currently implemented by `scripts/quality.sh`, it runs:

- `cargo fmt --check`
- `cargo check --all-targets --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`
- `cargo test --all-targets --all-features`

It does not currently run `just static`, `just test-oracle`, `just coverage`, `just fuzz`, or
`just mutation`. Agents must run the extra gates below when the change type requires them.

## Gate Map

| Gate | Command | Purpose |
| --- | --- | --- |
| Format | `just fmt` | Enforce stable Rust formatting. |
| Compile | `just check` | Compile all targets and features. |
| Lint | `just lint` | Catch correctness, API, and maintainability issues. |
| Static/dependency policy | `just static` | Run check, Clippy, and `cargo-deny` advisories/licenses/sources/bans. |
| Rustdoc | `just doc` | Keep public docs warning-free. |
| Unit tests | `just test-unit` | Test deterministic module behavior. |
| Integration smoke | `just test-smoke` | Confirm crate-level assumptions and upstream fixture availability. |
| CLI e2e | `just test-e2e` | Exercise the CLI binary as a user would. |
| Regression | `just test-regression` | Lock reduced fixtures and previously fixed behavior. |
| Oracle | `just test-oracle` | Run oracle tests. This must be differential before it can prove parity. |
| Property-based | `just test-property` | Stress generated invariants. |
| Fuzz | `just fuzz` | Run a bounded fuzz campaign. |
| Mutation | `just mutation` | Check whether tests kill semantic mutations. |
| Coverage | `just coverage` | Produce coverage data; threshold enforcement must be added before using this as a hard gate. |
| Deep local suite | `just deep` | Run `quality`, `static`, `coverage`, and `test-oracle`. |

## Gate Matrix by Change Type

| Change type | Minimum gates | Extra gates |
| --- | --- | --- |
| Docs only | Manual doc review; run commands only if docs changed command semantics. | Link/path check with `rg` if references changed. |
| Build scripts or dependencies | `just quality`, `just static` | Platform/link notes, upstream configure-feature mapping. |
| Parser, formatter, evaluator | `just quality`, focused tests, `just test-oracle` for affected cases | `just fuzz`, `just mutation`, `just coverage`. |
| Numeric core | `just quality`, unit tests, oracle cases | Mutation on changed module, property tests for identities. |
| Units, definitions, datasets, rates | `just quality`, loader tests, oracle cases | XML/JSON fuzz targets, provenance check. |
| CLI/session behavior | `just quality`, `just test-e2e`, oracle batch/session cases | Stdin/exit-status/platform tests. |
| FFI or unsafe | `just quality`, `just static`, focused wrapper tests, `unsafe-checker` | ABI/layout notes, sanitizer or valgrind run when practical. |
| Public API change | `just quality`, API tests, `code-review-breaking-changes` | Semver/breaking-change rationale. |

## Oracle Gate Policy

The upstream checkout at `../libqalculate` is the oracle. If `../libqalculate/src/qalc`
exists, `scripts/oracle.sh` uses it. Otherwise set:

```sh
QALCULATE_ORACLE=/path/to/qalc just test-oracle
```

Current oracle tests may pass while only proving upstream fixture availability. Do not report
native Rust parity unless the test executed Rust and C++ for the same case with C++ fallback
disabled.

Final parity CI must fail when upstream `qalc` is unavailable. Local inventory jobs may skip
with an explicit message, but skipped oracle execution means the feature remains unproven.

## Optional Tool Installs

```sh
just install-tools
```

`rustfmt` and Clippy are pinned through `rust-toolchain.toml`. Optional Cargo tools are
installed under `.tools/bin` by default, and helper scripts prepend that directory to `PATH`.

## Fuzzing Policy

Run short fuzz campaigns before merging parser, formatter, evaluator, CLI command, or
definition-loader changes. Keep crash artifacts under `fuzz/artifacts/` until reduced into
regression tests.

## Mutation Policy

Run mutation testing on stable semantic modules after adding meaningful tests. Scope
campaigns when needed:

```sh
just mutation
```

Surviving mutants should either become new tests or be documented as equivalent mutants in
the handoff.

## Coverage Policy

The target minimum for ported semantic modules is 80% line coverage. The current coverage
script emits LCOV data; threshold enforcement must be added before coverage can be treated
as a hard pass/fail gate.
