---
name: code-review
description: Use when running a final code review on a Rust libqalculate port change, branch, or pull request before merge or handoff.
---

# Code Review Orchestrator

Run a final review focused on correctness, compatibility with upstream `../libqalculate`, Rust API quality, and test evidence.

## Workflow

1. Inspect the diff and current project instructions.
2. Run each repository-local `code-review-*` skill other than this orchestrator. Use one independent subagent per skill when subagents are available; otherwise run the passes manually and state the fallback.
3. Preserve every evidence-backed finding from each pass. Do not collapse distinct issues into a vague summary.
4. Add any additional correctness, safety, maintainability, or missing-test findings visible from the diff.

## Finding Standard

Report findings first, ordered by severity:

- `P0`: unsoundness, data loss, or broad compatibility break.
- `P1`: likely correctness regression, externally visible behavior break, missing required compatibility test, or review blocker.
- `P2`: narrow bug, maintainability risk, incomplete context, or useful staging concern.

Every finding must include a file path and line number, the violated behavior or invariant, and a concrete fix direction. Do not leave GitHub comments or mutate labels unless explicitly asked.

If no findings remain, state the review passes run and any residual risk.
