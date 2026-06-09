# Task Packet: [TASK_ID] — [Short Title]

Use this template for one XS/S/M porting task. Replace bracketed placeholders with
task-specific values before assigning work; do not leave blank placeholder values in an issue body.

## Metadata
- **Issue**: #[N]
- **Epic**: [Epic Name]
- **Size**: XS / S / M
- **Owner**: [agent or human]
- **Branch**: `port/[feature-name]`
- **Upstream Version**: 5.11.0

## Upstream References
- **Headers**: replace with task-specific headers; examples include `../libqalculate/libqalculate/Calculator.h`, `MathStructure.h`, `Number.h`, and `ExpressionItem.h`.
- **Implementation**: replace with task-specific sources; examples include `../libqalculate/libqalculate/Calculator-parse.cc`, `Calculator-calculate.cc`, `MathStructure-print.cc`, `Number.cc`, and `../libqalculate/src/qalc.cc`.
- **Data files**: replace with task-specific data; examples include `../libqalculate/data/functions.xml.in`, `units.xml.in`, `prefixes.xml.in`, `variables.xml.in`, and `rates.json`.
- **Batch fixtures**: replace with task-specific fixtures; examples include `../libqalculate/tests/parser.batch`, `operators.batch`, `numberbase.batch`, `units.batch`, and `strings.batch`.

## Scope
- [ ] [Specific deliverable 1]
- [ ] [Specific deliverable 2]
- [ ] [Specific deliverable 3]

## Acceptance Criteria
- [ ] All new code has `// SAFETY` comments for any unsafe blocks
- [ ] Unit tests pass: `cargo test --lib`
- [ ] Integration tests pass: `just test-smoke`
- [ ] Oracle comparison passes: `just test-oracle`
- [ ] Quality gates pass: `just quality`
- [ ] No C++ fallback marked as native-pass
- [ ] `docs/compatibility_inventory.md` updated with new status

## Implementation Notes
[Design decisions, C++ behavior quirks, Rust idiom choices]

## Deviation Records
[Reference any DEV-NNNN entries from docs/deviations.md, or "None"]

## Completion Evidence
- **Merged PR**: #[PR]
- **Fallback-disabled status**: native / fallback-only / inventory-only
- **Commands run**: [exact commands and results]
- **Review skills run**: `code-review-change-size`, `code-review-context`, `code-review-testing`; add `code-review-breaking-changes` for public API changes.
