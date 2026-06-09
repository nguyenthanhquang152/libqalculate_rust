# Slop Cleaner Report: [TASK_ID]

Use this when narrowing or replacing scaffold/fallback code. Name the upstream behavior
that defines the cleanup target before editing Rust code.

## Scope
- **Module**: [src/module.rs]
- **Previous status**: scaffold / fallback-only
- **New status**: native-pass / scaffold (narrowed)
- **Upstream headers**: `Calculator.h`, `MathStructure.h`, `Number.h`, or task-specific replacement
- **Upstream implementation**: `Calculator-calculate.cc`, `MathStructure-print.cc`, `Number.cc`, or task-specific replacement
- **Upstream fixtures**: `parser.batch`, `operators.batch`, `numberbase.batch`, or task-specific replacement

## Changes Made
- [ ] Removed FFI fallback for [feature]
- [ ] Replaced placeholder with native implementation
- [ ] Added missing error handling
- [ ] Updated `docs/compatibility_inventory.md`

## Before/After Comparison
| Metric | Before | After |
|---|---|---|
| Lines of unsafe code | [N] | [N] |
| FFI calls | [N] | [N] |
| Test coverage | [N]% | [N]% |
| Oracle pass rate | [N]% | [N]% |

## Verification
- [ ] `just quality` passes
- [ ] `just test-oracle` passes
- [ ] No regressions in existing tests
- [ ] No C++ fallback output is counted as `native-pass`
