---
name: code-review-breaking-changes
description: Use when reviewing Rust libqalculate port changes that may alter public APIs, CLI compatibility, parser behavior, numeric semantics, units, definitions, or upstream-compatible output.
---

# Breaking Change Review

Review the diff for compatibility breaks against the Rust crate surface and the upstream C++ oracle in `../libqalculate`.

## Review Steps

1. Identify every changed public surface: exported Rust modules, functions, types, feature flags, errors, CLI behavior, files read from disk, and generated outputs.
2. Compare user-visible behavior with the relevant upstream source or fixture under `../libqalculate/libqalculate`, `../libqalculate/src`, `../libqalculate/data`, or `../libqalculate/tests`.
3. Treat parser, evaluator, formatter, definition loading, unit conversion, and numeric behavior as compatibility-sensitive even when the Rust API did not change.
4. Check edge cases: exact rationals, approximate floats, intervals, infinities, complex numbers, uncertainty, localization-sensitive parsing, prefixes, units, dates, warnings, and errors.
5. Verify migration paths for intentional breaks: versioning, changelog/PR body note, compatibility test update, and a clear reason the break is worth it.

## Findings

Report only evidence-backed issues. Each finding must include:

- Severity: `P0` data loss, unsoundness, or broad compatibility break; `P1` externally visible regression; `P2` narrow or documented compatibility risk.
- Local file path and line number.
- Upstream oracle path, fixture, or documented behavior used for comparison.
- The exact behavior that breaks and the smallest compatible fix or mitigation.

If no issue is found, state the surfaces reviewed and the upstream evidence checked.
