# Agent Skills Mapping: Autonomous Porting Workflows

This document tells AI coding agents how to use the repository skills and docs while
porting `libqalculate` from C++ to Rust. It is intentionally portable across Claude Code,
Codex, Gemini, and plain shell-driven agents.

For the full task lifecycle and handoff templates, see `docs/task_lifecycle.md`. For GitHub
issue and pull request rules, see `docs/github_workflow.md`.

## Skill Invocation Rules

- If the environment provides a skill tool or slash command, invoke the named skill before
  doing that phase of work.
- If a platform cannot invoke skills directly, read the repository-local skill file under
  `.agents/skills/<skill-name>/SKILL.md` and follow its checklist.
- Do not rely on absolute `file://` paths. Use repository-relative skill names and paths.
- When a skill is unavailable, state that in the handoff and use the documented fallback.
- `graphify-out/graph.json` is optional. If it exists, follow `AGENTS.md` and query it before
  broad source browsing. If absent, use `rg`, `rust-code-navigator`, and focused upstream
  source reads.

## Phase-to-Skill Map

| Phase | Primary skills | Codex/plain-shell fallback | Evidence expected |
| --- | --- | --- | --- |
| Intent and Rust routing | `rust-router` | Read `.agents/skills/rust-router/SKILL.md`; route to ownership, error, domain, or unsafe skills. | Chosen route and why. |
| Upstream research | `rust-code-navigator`, `rust-call-graph`, `rust-symbol-analyzer`, `rust-trait-explorer` | Use `rg`, `cargo metadata`, rustdoc, and focused reads of `../libqalculate`. | Upstream headers, `.cc`, data, and fixtures named. |
| Domain modeling | `m09-domain`, `m05-type-driven`, `m01-ownership`, `m02-resource` | Read the matching skill files and document invariants. | Rust owner modules and invariants. |
| Error/message design | `m06-error-handling`, `m13-domain-error` | Design `Result` plus message queue behavior from upstream evidence. | Fatal errors and non-fatal messages mapped. |
| Concurrency/session state | `m07-concurrency`, `m12-lifecycle` | Audit `Send`/`Sync`, locks, `Drop`, and global-state access manually. | Threading and cleanup notes. |
| FFI/unsafe | `unsafe-checker`, `m11-ecosystem` | Inspect every unsafe block and generated binding with the unsafe checklist. | `SAFETY` invariants, ownership, ABI notes. |
| CLI behavior | `domain-cli` | Compare with `../libqalculate/src/qalc.cc`; test as a user. | CLI flags, stdin, exit status, session behavior. |
| Testing design | `code-review-testing`, testing handbook skills when available | Use `docs/testing_strategy.md` and `docs/quality-gates.md`. | Fixtures, oracle cases, property/fuzz/mutation scope. |
| Refactoring | `rust-refactor-helper` | Use LSP or `cargo check` after small edits. | Refactor scope and gates. |
| GitHub issue/PR work | `gh-cli`, `codex-pr-body` | Use authenticated `gh` commands; update issue and PR descriptions manually if needed. | Linked issue, branch, PR, review state. |
| Final review | `code-review` orchestrator, `code-review-context`, `code-review-change-size`, `code-review-testing`, `code-review-breaking-changes` | Manually run each repository-local review checklist. | Findings resolved or documented. |
| PR/handoff | `codex-pr-body` | Use the handoff template in `docs/task_lifecycle.md`. | Summary, tests, oracle evidence, deviations. |

## Required Task Flow

Every implementation task must follow this order:

1. Pick or create a GitHub issue using `docs/github_workflow.md`.
2. Build a task packet from `docs/task_lifecycle.md`.
3. Comment on the issue with branch name and first verification target.
4. Research upstream files and fixtures named by the packet.
5. Add failing or pending tests from upstream behavior.
6. Implement a native Rust slice without C++ fallback for the feature under test.
7. Open a draft PR linked to the issue when code or review discussion is ready.
8. Run required gates from `docs/quality-gates.md`.
9. Run review skills and resolve findings on the PR branch.
10. Record completion evidence and update any inventories, deviations, issues, and PR body.

Do not mark a task complete from scaffold checks alone. `just test-oracle` proves parity only
when it compares Rust and C++ output for the same case with fallback disabled.

## Upstream Evidence Requirements

Each task handoff must name:

- GitHub issue and PR.
- Upstream version and `qalc` path used.
- Upstream headers and `.cc` files inspected.
- Upstream data files used or affected.
- Upstream `.batch` and CSV fixtures used.
- Rust modules changed.
- Commands run and their results.
- Whether C++ fallback was disabled for the feature.
- Any deviation ids from `docs/deviations.md`.

The review skill `code-review-context` should be able to validate compatibility from this
evidence without broad source archaeology.

## Review Loop

Before final handoff for code changes:

1. Confirm the PR links the implementation issue and uses `Closes #N` only for complete work.
2. Run `code-review-change-size` to confirm task scope stayed XS/S/M.
3. Run `code-review-context` to confirm upstream evidence is sufficient.
4. Run `code-review-testing` to confirm behavior tests and oracle coverage.
5. Run `code-review-breaking-changes` when public API or CLI behavior differs.
6. Run `unsafe-checker` for any unsafe, FFI, raw pointer, ABI, or manual `Send`/`Sync` work.
7. Resolve findings, rerun affected gates, and include the resolved-findings summary in the
   PR body and issue update.

`thermos` may be used as an additional strict review pass, but it does not replace the
repository-local review skills above.

## Skill Selection Examples

| Task | Skills |
| --- | --- |
| Port `Number` interval multiplication | `rust-router`, `m09-domain`, `m05-type-driven`, `code-review-testing` |
| Add C++ calculator fallback wrapper | `unsafe-checker`, `m11-ecosystem`, `m12-lifecycle`, `code-review-context` |
| Port `/set unicode 1` session behavior | `domain-cli`, `m06-error-handling`, `code-review-testing` |
| Port XML units loader | `m09-domain`, `m13-domain-error`, `code-review-context`, fuzzing skill if available |
| Change public Rust API | `rust-router`, `code-review-breaking-changes`, `codex-pr-body` |
