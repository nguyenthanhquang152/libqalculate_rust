# PR: [Title]

Closes #[N]

## Summary
[One paragraph describing what was ported and why]

## Changes
- [file1.rs]: [what changed]
- [file2.rs]: [what changed]

## Testing Evidence
- Unit tests: `cargo test --lib` ✅
- Integration: `just test-smoke` ✅
- Oracle: `just test-oracle` ✅
- Quality: `just quality` ✅

## Oracle Evidence
[Link to oracle_evidence.md or inline summary]

## Compatibility Impact
- Status changes in `docs/compatibility_inventory.md`: [list]
- New deviations: [DEV-NNNN or None]
- Retired deviations: [DEV-NNNN or None]

## Review Checklist
- [ ] No unsafe without SAFETY comment
- [ ] No C++ fallback marked as native-pass
- [ ] Inventory updated
- [ ] Deviations documented
