# Compatibility Deviations Registry

This registry records intentional user-visible differences between the Rust port and
upstream `libqalculate` 5.11.0. The default policy is exact compatibility. No deviation is
approved unless it appears here with tests and review evidence.

## Policy

- Exact UTF-8 output is the default oracle comparison.
- Do not normalize whitespace, Unicode symbols, floating output, date/time text, diagnostics,
  path text, or exit status unless a deviation entry explicitly allows it.
- Tooling gaps, missing implementation, and C++ fallback are not compatibility deviations.
  They are incomplete work.
- Each deviation must include a stable id, rationale, affected features, upstream evidence,
  Rust tests, owner, and review approval.
- Stale deviations must be removed when Rust behavior converges with upstream.

## Deviation Entry Template

```md
### DEV-0000: short-title

Status: proposed | approved | retired
Owner:
Affected features:
Upstream version:
Upstream evidence:
Rust behavior:
Rationale:
Normalization policy:
Tests:
Review evidence:
Retirement condition:
```

## Approved Deviations

No user-visible compatibility deviations are currently approved.

## Proposed Deviations

No proposed deviations are currently recorded.

## Retired Deviations

No deviations have been retired.
