# Upstream Documentation Example Manifest

This manifest tracks the upstream documentation examples selected for the Rust
port's executable parity evidence. Source anchors refer to the adjacent
libqalculate 5.11.0 checkout. `native-pass` requires fallback-disabled Rust
evidence; `pending` rows are explicit family-level boundaries and do not count
as parity.

The CLI differentials add deterministic `-c0`/`-t` flags where needed to make
the documented behavior byte-comparable. The user-facing expression, option,
or API call remains the one shown by the upstream source.

| ID | Upstream source | Source context | Rust command/API | Required data/settings | Owner task | Status | Evidence |
|---|---|---|---|---|---|---|---|
| `README-CLI-001` | `../libqalculate/README.md:40` | `qalc 5+2` | `qalc-rs 5+2` | exact non-terse equation; normal definitions; fallback disabled; pinned locale/home | [#65] | `native-pass` | `tests/e2e_cli.rs::docs_example_readme_cli_arithmetic_matches_upstream` |
| `README-CLI-002` | `../libqalculate/README.md:42` | `qalc --help` | `qalc-rs --help` | behavioral help invariant; evaluation/fallback path unreachable | [#60], [#65] | `native-pass` | `tests/e2e_cli.rs::cli_help_and_version_aliases_match_upstream` |
| `MAN-CLI-SET-001` | `../libqalculate/man/qalc.1:103` | `\-\-set "base 16"` | `qalc-rs -t -s "base 16" -- 52` | output base 16; normal definitions; fallback disabled | [#60], [#65] | `native-pass` | `tests/e2e_cli.rs::docs_example_man_set_base_16_matches_upstream` |
| `README-NUMBASE-001` | `../libqalculate/README.md:346` | `52 to bin` | `qalc-rs -t -- "52 to bin"` | normal definitions; fallback disabled | [#26], [#65] | `native-pass` | `tests/e2e_cli.rs::docs_example_readme_number_base_matches_upstream` |
| `CALCULATOR-API-001` | `../libqalculate/libqalculate/Calculator.h:39` | `calculateAndPrint("1 + 1"` | `Calculator::calculate_and_print("1 + 1")` | Rust-owned session; no FFI fallback | [#64], [#65] | `native-pass` | `src/calculator.rs::calculate_and_print("1 + 1")` |
| `README-SYMBOLIC-001` | `../libqalculate/README.md:81` | `Symbolic calculations` | representative symbolic, function, unit, and plot examples | feature-specific data and options | [#25]-[#50], [#83] | `pending` | `pending` |
| `README-ADVANCED-001` | `../libqalculate/README.md:120` | `Basic functions and operators` | remaining function, algebra, calculus, matrix, statistics, and uncertainty examples | feature-specific oracle promotion | [#15], [#25]-[#44], [#83] | `pending` | `pending` |
| `README-DATETIME-001` | `../libqalculate/README.md:318` | `Time and date` | dynamic and calendar examples outside the pinned deterministic slice | timezone/clock policy and feature-specific fixtures | [#51]-[#54], [#83] | `pending` | `pending` |
| `MAN-SETTINGS-001` | `../libqalculate/man/qalc.1:310` | `These settings are changed` | remaining interactive and CLI setting combinations | setting-specific native evidence | [#22], [#60]-[#61], [#83] | `pending` | `pending` |
| `CALCULATOR-API-002` | `../libqalculate/libqalculate/Calculator.h:473` | `calculateAndPrint` | timeout, broad option, and parsed-expression overload families | public API category implementation | [#64], [#83] | `pending` | `pending` |

## Status policy

- `native-pass`: the evidence path names a checked test or doctest. CLI
  calculation cases explicitly disable the C++ fallback; control-flow-only
  flags such as `--help` cannot enter the evaluation/fallback path.
- `pending`: the upstream example family remains owned by the linked task or
  Epic #83 follow-up; no easier replacement example is substituted.
- `out-of-scope`: reserved for an approved rationale. No selected example uses
  this status.

No intentional deviation is recorded by this manifest.

[#15]: https://github.com/nguyenthanhquang152/libqalculate_rust/issues/15
[#22]: https://github.com/nguyenthanhquang152/libqalculate_rust/issues/22
[#25]: https://github.com/nguyenthanhquang152/libqalculate_rust/issues/25
[#26]: https://github.com/nguyenthanhquang152/libqalculate_rust/issues/26
[#44]: https://github.com/nguyenthanhquang152/libqalculate_rust/issues/44
[#50]: https://github.com/nguyenthanhquang152/libqalculate_rust/issues/50
[#51]: https://github.com/nguyenthanhquang152/libqalculate_rust/issues/51
[#54]: https://github.com/nguyenthanhquang152/libqalculate_rust/issues/54
[#60]: https://github.com/nguyenthanhquang152/libqalculate_rust/issues/60
[#61]: https://github.com/nguyenthanhquang152/libqalculate_rust/issues/61
[#64]: https://github.com/nguyenthanhquang152/libqalculate_rust/issues/64
[#65]: https://github.com/nguyenthanhquang152/libqalculate_rust/issues/65
[#83]: https://github.com/nguyenthanhquang152/libqalculate_rust/issues/83
