---
name: code-review-change-size
description: Use when reviewing whether a Rust libqalculate port diff is small, staged, and coherent enough for reliable human or AI review.
---

# Change Size Review

Evaluate whether the diff is reviewable without hiding semantic risk.

## Thresholds

- Prefer under 800 changed lines for non-mechanical changes.
- Prefer under 500 changed lines for parser, evaluator, formatter, numeric, units, or definition-loading logic.
- Treat generated data, vendored upstream fixtures, and mechanical renames separately only if the PR clearly isolates them.

## Review Steps

1. Classify the diff: mechanical, scaffolding, numeric core, parser/AST, evaluation, formatting, definitions, units, CLI, tests, or mixed.
2. Check whether unrelated domains are bundled together. Flag combinations that force reviewers to reason about multiple compatibility boundaries at once.
3. Identify the smallest coherent stage that could land first while keeping tests meaningful.
4. Require explicit reviewer guidance for large diffs: what is generated, what is copied from upstream, what is semantic, and which files deserve line-by-line review.

## Findings

Report a finding when size or staging creates review risk. Include the changed-line scope, affected files, why the bundle is risky, and the proposed split. If the size is acceptable, state why the current grouping is coherent.
