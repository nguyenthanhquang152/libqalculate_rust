---
name: code-review-testing
description: Use when reviewing whether a Rust libqalculate port change has sufficient compatibility tests, regression tests, oracle coverage, and focused unit tests.
---

# Testing Review

Review tests as the main proof that the Rust port preserves upstream behavior.

## Required Coverage

- Parser, evaluator, formatter, numeric, units, functions, variables, definitions, dates, and CLI behavior changes need observable behavior tests.
- Prefer fixture or integration tests that consume upstream-style `.batch` cases from `../libqalculate/tests` or a reduced checked-in subset.
- Add focused unit tests for deterministic internals such as `Number`, AST transforms, parser tokens, formatter rules, and conversion helpers.
- Cover edge cases for exact rationals, approximate floats, intervals, infinities, complex values, uncertainty, prefixes, units, localized input, and messages.

## Review Steps

1. Map each semantic change to a test or documented oracle comparison.
2. Check that tests assert user-visible behavior, not only implementation details.
3. Verify new fixtures explain their upstream source or expected-output oracle.
4. Flag test-only hooks in production code unless there is no clean test-support alternative.

## Findings

Report missing tests as findings when a behavior change lacks proof. Include the changed code path, the missing scenario, the expected upstream oracle, and the smallest useful test to add.
