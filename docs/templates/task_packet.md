# Task Packet: [TASK_ID] — [Short Title]

## Metadata
- **Issue**: #[N]
- **Epic**: [Epic Name]
- **Size**: XS / S / M
- **Owner**: [agent or human]
- **Branch**: `port/[feature-name]`
- **Upstream Version**: 5.11.0

## Upstream References
- **Headers**: [list of .h files inspected]
- **Implementation**: [list of .cc files inspected]
- **Data files**: [list of .xml.in or .json files used]
- **Batch fixtures**: [list of .batch files used for verification]

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
