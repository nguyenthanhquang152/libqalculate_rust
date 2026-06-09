# Slop Cleaner Report: [TASK_ID]

## Scope
- **Module**: [src/module.rs]
- **Previous status**: scaffold / fallback-only
- **New status**: native-pass / scaffold (narrowed)

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
