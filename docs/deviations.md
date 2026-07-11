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

### DEV-0001: buffered-repl-terminal-editor

Status: proposed
Owner: #201 (follow-up from #61)
Affected features: `qalc-rs` interactive terminal editing, live completion, history navigation, Ctrl-C editing semantics, and first-run calculate-as-you-type onboarding
Upstream version: 5.11.0
Upstream evidence: `src/qalc.cc` readline setup/signal handling at lines 4953–5048, completion provider/chooser at lines 683–1139, Ctrl-C handling near line 893, first-run onboarding near lines 4268 and 10025, and isolated PTY transcripts recorded on #61
Rust behavior: the issue #61 session engine uses injected buffered input/output. Prompting, evaluation, typed `ans` history, settings, XDG history persistence/clear, and command dispatch match their covered upstream transcripts. Live Tab completion, arrow-key recall, readline-style Ctrl-C editing/abort behavior, and the first-run autocalc question are not exposed.
Rationale: the repository has no line-editor or signal dependency, and adding one or extending the CXX bridge with readline/signal ownership would materially expand the M-sized session task. The bounded engine keeps evaluator semantics independent from terminal I/O and leaves the adapter replaceable.
Normalization policy: none; covered pipe and PTY outputs remain exact. The unsupported terminal interactions are explicitly excluded rather than normalized.
Tests: `tests/interactive_cli.rs::fresh_profile_uses_the_line_repl_without_autocalc_onboarding` and `tests/interactive_cli.rs::pty_smoke_covers_prompt_quit_and_answer_state`; the missing event-driven completion/history-navigation/Ctrl-C cases are tracked by #201
Review evidence: `code-review`, Thermos correctness/maintainability passes, code-review-graph impact analysis, and the final schema-validated Antigravity correctness review completed on #61; final PR-bot review remains pending
Retirement condition: #201 supplies a readline-equivalent terminal adapter with event-driven PTY coverage for Tab/selection, Unicode editing, history recall, Ctrl-C/abort semantics, and first-run onboarding

## Retired Deviations

No deviations have been retired.
