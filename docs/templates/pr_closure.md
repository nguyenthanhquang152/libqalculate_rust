# PR: [Title]

Use this body only when the linked issue definition of done is satisfied or when a
tracking issue explicitly accepts the remaining work as follow-up/out-of-scope.

Closes #[N]

## Summary
[One paragraph describing what was ported and why]

## Upstream Scope
- **Version**: libqalculate 5.11.0
- **Headers inspected**: `Calculator.h`, `MathStructure.h`, `Number.h`, `ExpressionItem.h`
- **Implementation inspected**: `Calculator-parse.cc`, `Calculator-calculate.cc`, `MathStructure-print.cc`, `Number.cc`, `../libqalculate/src/qalc.cc`
- **Data inspected**: `functions.xml.in`, `units.xml.in`, `prefixes.xml.in`, `variables.xml.in`, `rates.json`
- **Fixtures inspected**: `parser.batch`, `operators.batch`, `numberbase.batch`, `units.batch`, `strings.batch`

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
- Fallback-disabled status: native / fallback-only / inventory-only

## Review Checklist
- [ ] No unsafe without SAFETY comment
- [ ] No C++ fallback marked as native-pass
- [ ] Inventory updated
- [ ] Deviations documented
