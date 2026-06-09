# GitHub Issue and PR Workflow

GitHub is the system of record for porting progress. The docs define the porting plan, but
GitHub issues define the executable work units that AI coding agents pick up. Pull requests
are the only path for changing `main`.

## Work Tracking Model

- Create one GitHub issue for each XS/S/M task packet.
- Use tracking issues for epics, milestones, and broad inventories. Do not assign broad
  tracking issues directly to an implementation agent.
- Each implementation issue must map back to `docs/porting_master_plan.md` and include the
  task packet from `docs/task_lifecycle.md`.
- Each issue should be small enough for one AI coding agent session plus review. If the work
  is larger, split the issue before implementation starts.
- Progress is measured by issue state, linked PRs, and completed acceptance criteria, not by
  informal chat history.

## Issue Requirements

Every porting issue must include:

```md
Title:
Epic / task id:
Size: XS | S | M
Priority:
Rust owner modules:
Architecture boundary:
Upstream oracle files:
  - Headers:
  - Implementation:
  - Data:
  - Tests:
User-visible behavior:
Acceptance criteria:
Tests to add first:
Required gates:
Required review skills:
AI Slop Cleaner checkpoints:
Lessons already known:
Definition of done:
```

Use `docs/templates/task_packet.md` as the starting point for implementation issues. Use
`docs/templates/oracle_evidence.md` for oracle results, `docs/templates/pr_closure.md` for
pull request bodies and issue closure evidence, and `docs/templates/slop_cleaner_report.md`
when removing scaffolding or fallback code.

Required labels:

- `type:port`
- `size:xs`, `size:s`, or `size:m`
- `priority:high`, `priority:medium`, or `priority:low`
- one area label such as `area:number`, `area:parser`, `area:ffi`, `area:units`,
  `area:cli`, `area:testing`, or `area:docs`
- one status label such as `status:ready`, `status:in-progress`, `status:blocked`,
  `status:review`, or `status:done`

Optional labels:

- `risk:unsafe`
- `risk:breaking-api`
- `needs:oracle`
- `needs:upstream-research`
- `needs:hygiene`
- `has:approved-deviation`

## Agent Assignment

When an AI coding agent starts work:

1. Confirm the issue has a complete task packet.
2. Comment on the issue with the planned branch name and first verification target.
3. Create a branch named `issue/<number>-<short-slug>` or `agent/<number>-<short-slug>`.
4. Keep the issue updated when scope changes, blockers appear, or acceptance criteria need
   splitting.
5. Do not push directly to `main`.

If the agent discovers that the issue is too broad, it must stop implementation, propose
smaller child issues, and keep the original issue as a tracking issue.

## Pull Request Requirements

Every PR must link exactly one primary implementation issue. Use `Closes #N` only when the
PR fully satisfies the issue definition of done. Use `Refs #N` for partial or preparatory
work.

The PR description must include:

```md
Issue:
Summary:
Upstream evidence:
Rust changes:
Tests and gates:
Oracle evidence:
Hygiene/refactor evidence:
Lessons learned:
Review skills:
Deviations:
Residual risks:
```

PRs start as draft when:

- the oracle runner or required fixture is still being built,
- acceptance criteria are incomplete,
- review findings are unresolved,
- the agent is asking for design feedback before final gates.

Move the PR out of draft only after required tests, gates, AI Slop Cleaner checks, and
review-skill passes have been run or explicitly documented as skipped with residual risk.

## Review and Improve Loop

The PR review loop is mandatory:

1. Run the repository-local review skills listed in `docs/agent_skills_mapping.md`.
2. Request human or agent review.
3. Convert findings into concrete commits on the PR branch.
4. Rerun affected gates after fixes.
5. Update the PR description and linked issue with the final evidence.

Do not merge while any P0/P1 review finding remains open. P2 findings must be fixed,
converted into follow-up issues, or explicitly accepted by the maintainer.

## Merge Requirements

Merge to `main` only when:

- the linked issue acceptance criteria are complete,
- the PR includes current verification evidence,
- no required review skill was skipped without explanation,
- no native parity claim depends on C++ fallback,
- no architecture drift or hygiene finding remains unresolved,
- every reusable agent mistake has a lesson recorded or follow-up issue,
- `main` is not bypassed by direct pushes.

After merge:

- Confirm the issue closes automatically or manually close it with the merged PR link.
- Update tracking issues and milestone progress.
- Move any follow-up work into new issues instead of leaving it buried in PR comments.

## Milestones and Tracking Issues

Use milestones for checkpoint-level progress:

- C0: Inventory
- C1: Oracle Harness
- C2: Native Arithmetic
- C3: Parser and Commands
- C4: Evaluation
- C5: Data and CLI
- C6: Full Parity

Tracking issues should summarize child issue status, remaining risks, and checkpoint gates.
They should not contain implementation-only acceptance criteria that belong in child issues.

## GitHub CLI Notes

Prefer authenticated `gh` commands for GitHub issue and PR operations:

```sh
gh issue create --repo nguyenthanhquang152/libqalculate_rust
gh issue view <number> --repo nguyenthanhquang152/libqalculate_rust
gh pr create --repo nguyenthanhquang152/libqalculate_rust
gh pr view <number> --repo nguyenthanhquang152/libqalculate_rust
```

Agents must not use unauthenticated raw GitHub API calls when `gh` is available.
