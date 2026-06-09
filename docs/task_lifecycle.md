# Task Lifecycle for Porting Agents

This lifecycle is the required path from selecting a GitHub issue to handing its pull
request off for review. It ties together the master plan, coding rules, testing strategy,
quality gates, GitHub workflow, and agent skills.

## 1. Select and Bound the Task

Start from a GitHub issue created from `docs/porting_master_plan.md` and
`docs/github_workflow.md`. If a task is larger than M, split it before implementation. A task
must have a disjoint owner module and a clear upstream oracle.

Do not start coding until the task packet below is filled in.

```md
GitHub issue:
Linked tracking issue or milestone:
Task ID / epic / size:
Dependencies:
Rust owner modules:
Architecture boundary:
Upstream oracle files:
  - Headers:
  - Implementation:
  - Data:
  - Tests:
User-visible behavior to preserve:
Intentional divergences:
Prior lessons consulted:
Tests to add first:
Hygiene checkpoints:
Branch:
PR plan:
Required gates:
Required review skills:
Completion evidence:
```

An AI coding agent must comment on the issue with the branch name and first verification
target before implementation begins. The branch should be named
`issue/<number>-<short-slug>` or `agent/<number>-<short-slug>`.

## 2. Research Upstream Behavior

Use the skills in `docs/agent_skills_mapping.md`, `rg`, and focused source reads. The output
of research must be specific:

- C++ header names and relevant types/methods.
- C++ implementation files and behavior entry points.
- Batch/data fixtures that prove behavior.
- CLI command or API call used as oracle.
- Option, locale, timezone, data-path, and session settings required by the behavior.

Avoid broad claims like "matches upstream" without file and fixture evidence.

## 3. Write Tests First

Before implementation, add the smallest useful failing test or oracle-manifest entry.

Test choices:

- Unit test for deterministic internals.
- Regression fixture copied or reduced from upstream behavior.
- Differential oracle case for user-visible output.
- Property test for invariant-heavy logic.
- Fuzz target for parsers, loaders, evaluators, or command handlers.

If the current scaffold cannot execute the final test yet, add the manifest entry and a
scaffold test that makes the missing runner capability explicit. Do not count that as
parity.

## 4. Implement Native Rust Behavior

Implement the smallest native slice that satisfies the test.

Rules:

- Follow `docs/porting_guidelines.md`.
- Keep C++ fallback disabled for the feature evidence used to mark native parity.
- Keep definition data provenance and refresh paths explicit.
- Preserve diagnostics and message ordering.
- Update inventories, manifests, and deviations in the same task when behavior status
  changes.

## 5. Run the AI Slop Cleaner

AI coding agents tend to add duplicated helpers, speculative abstractions, scattered flags,
and logic in the wrong layer. Run this hygiene checkpoint frequently:

- after each coherent implementation slice,
- before expanding a file or module boundary,
- after `cargo check` first succeeds,
- after tests pass but before final review,
- whenever review feedback says the code is correct but hard to maintain.

Use `karpathy-guidelines` for surgical-change discipline, `rust-refactor-helper` for safe
renames/moves/extractions, `rust-symbol-analyzer` or `rust-call-graph` for complexity and
dependency hotspots, and `thermo-nuclear-code-quality-review` for M-sized, risky, or
architecture-sensitive changes.

The cleanup rule is strict: remove the mess created by the current task, but do not perform
unrelated drive-by refactors. If a broader cleanup is needed, create a separate hygiene task
packet with its own tests and review evidence.

AI Slop Cleaner checklist:

- The change still lives inside the task's declared Rust owner modules.
- No feature logic leaked into parser, formatter, CLI, or FFI layers that do not own it.
- No new helper, wrapper, enum variant, option flag, or trait exists only for one call site
  unless it clarifies an invariant.
- No C++ fallback path can be mistaken for native parity.
- No duplicated upstream lookup, formatting, conversion, or diagnostic logic bypasses a
  canonical helper.
- No file grew toward a large-module threshold without a decomposition note. Treat 600 lines
  as a review warning and 1000 lines as a blocker unless explicitly justified.
- No TODO, placeholder, panic, unwrap, broad clone, or lossy numeric conversion remains in
  user-visible logic without a tracked issue or deviation.
- The public API and module boundaries still match `docs/porting_master_plan.md` and
  `docs/porting_guidelines.md`.

Record the result in the handoff:

```md
Hygiene/refactor evidence:
- Checkpoint run:
- Tools/skills used:
- Cleanup performed:
- Architecture drift found:
- Follow-up hygiene tasks:
```

## 6. Verify

Run gates according to `docs/quality-gates.md`. At minimum, code changes require
`just quality`. Semantic changes also require focused tests and oracle evidence.

Use this evidence block in the handoff:

```md
Verification:
- Command:
  Result:
  What it proves:
- Command:
  Result:
  What it proves:
Skipped:
- Gate:
  Reason:
  Residual risk:
```

## 7. Oracle Evidence Template

Every compatibility claim needs this block:

```md
Oracle Evidence:
- Upstream version:
- qalc path and availability:
- C++ source/fixture checked:
- Rust command run:
- Upstream command run:
- Fallback disabled: yes/no
- Output comparison:
- Normalization/deviation id:
- If skipped, why and why the feature is not complete yet:
```

## 8. Review and Resolve

Run the review skills required by the task:

- `code-review-change-size`
- `code-review-context`
- `code-review-testing`
- `code-review-breaking-changes` when API/CLI behavior differs
- `unsafe-checker` for FFI/unsafe

Resolve findings and rerun affected gates. If a finding is intentionally not resolved, record
the reason and the residual risk.

## 9. Open or Update the Pull Request

Each implementation issue must produce a PR. The PR is the review vehicle and the only path
to `main`.

PR rules:

- Link exactly one primary issue.
- Use `Closes #N` only when the PR fully satisfies the issue definition of done.
- Use `Refs #N` for partial infrastructure or preparatory work.
- Keep the PR as draft until required gates, oracle evidence, hygiene checks, and review
  skills are complete or explicitly documented.
- Push review fixes to the same PR branch and rerun affected gates.
- Do not merge with open P0/P1 findings.

Update the linked issue whenever scope changes, blockers appear, acceptance criteria are
split, or follow-up issues are created.

## 10. Extract Lessons Learned

Every task should decide whether it produced a reusable lesson for future agents. Lessons are
required when an agent:

- misunderstood upstream behavior,
- added code in the wrong module or layer,
- relied on C++ fallback while claiming native behavior,
- wrote tests that only checked scaffolding instead of parity,
- introduced duplicated or speculative abstractions,
- missed diagnostics, option state, locale/timezone, data provenance, or message ordering,
- hit a compiler, borrow-checker, FFI, or numeric precision issue that is likely to recur.

Use this template in the handoff:

```md
Lessons learned:
- Mistake pattern:
- Root cause:
- Prevention rule:
- Docs or task template to update:
- Follow-up issue:
```

If a lesson is reusable across tasks, update this lifecycle document or a dedicated project
lessons registry in the same PR. Keep lessons concrete and actionable; avoid vague notes like
"be careful" or "write better tests".

## 11. Final Handoff Template

Use this format for the final task response or PR body:

```md
Issue:
PR:

Summary:
-

Upstream evidence:
-

Rust changes:
-

Hygiene/refactor evidence:
-

Tests and gates:
-

Oracle evidence:
-

Deviations:
-

Lessons learned:
-

Review skills:
-

Residual risks:
-
```

## 12. Completion Rules

A task is incomplete if any of these are true:

- There is no GitHub issue or the issue lacks a complete task packet.
- The implementation was pushed directly to `main` instead of through a PR.
- The PR does not link the issue or uses `Closes #N` before the issue is actually complete.
- The upstream oracle files are not named.
- Tests only prove fixture parsing or upstream availability.
- The feature still uses C++ fallback but is claimed as native.
- The AI Slop Cleaner checkpoint was skipped for a code task or found architecture drift that
  remains unresolved.
- A reusable agent mistake was found but no lesson was recorded or extracted.
- Required gates or review skills were skipped without explanation.
- A behavior mismatch exists without an approved deviation.
- Public API differences were not reviewed as breaking changes.
