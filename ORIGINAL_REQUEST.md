# Original User Request

## Initial Request — 2026-06-09T01:33:45Z

Implement the `libqalculate` Rust port by executing the progressive stages defined in the docs. The team will operate in parallel/worktree-based mode, consuming the master plan, guidelines, testing strategy, and agent skills mapping.

Working directory: /home/nt-quang/Workspaces/personal/experimentals/porting_qalculator/libqalculate_rust
Integrity mode: development

## Requirements

### R1. Staged Execution of the Master Porting Plan
Implement the components according to the epic and task ordering defined in docs/porting_master_plan.md.

### R2. Strict Adherence to Porting Guidelines
All code must satisfy safety comments, FFI wrapping, memory ownership, and error/string representations described in docs/porting_guidelines.md.

### R3. Comprehensive Testing & Oracle Verification
Perform unit testing, property tests, mutation checks, and C++ oracle comparisons as specified in docs/testing_strategy.md.

### R4. Skill Mapping Utilization
Leverage local LSP tools, navigate and refactor helpers, and code reviewers mapped out in docs/agent_skills_mapping.md.

## Acceptance Criteria

### Core Porting Correctness
- [ ] Ported arbitrary-precision Float and Rational arithmetic matches C++ output.
- [ ] Lexer and Parser parse mathematical expressions with correct precedence.
- [ ] Evaluator substituted variables and functions correctly.

### Quality and Safety Gates
- [ ] No `unsafe` blocks are added without a preceding `// SAFETY` invariant comment.
- [ ] Total code coverage remains at or above 80% (`just coverage`).
- [ ] Hybrid tests and oracle comparisons pass successfully (`just quality`).

## Follow-up — 2026-06-09T08:41:15Z

Complete Task 1.4 (no-cpp-fallback-gate) of Epic 1 in the Rust port of `libqalculate`. Specifically, implement a test/configuration path proving selected feature slices run natively, and ensure a feature cannot be marked `native-pass` when its output came from the C++ fallback.

Working directory: /home/nt-quang/Workspaces/personal/experimentals/porting_qalculator/libqalculate_rust
Integrity mode: development

## Requirements

### R1. Disable Fallback Switch via Environment Variable
- Use the environment variable `QALCULATE_DISABLE_FALLBACK` (e.g. `QALCULATE_DISABLE_FALLBACK=1`) to globally control the C++ FFI fallback.
- When `QALCULATE_DISABLE_FALLBACK=1` is set:
  - Any evaluation of non-native/unimplemented features must fail loudly (return an error or panic) instead of calling C++.
  - For mock/scaffold native features (e.g. when expression is exactly "1 + 1" or "native-scaffold-test"), run natively in Rust.
- When the environment variable is not set (default), allow C++ FFI fallback to operate normally.

### R2. Fallback State Tracking in Test Runner
- The differential oracle runner (`tests/oracle.rs`) and any status checking logic must record whether a test case used the C++ fallback or executed natively.
- Prevent a test case or feature from being classified as `native-pass` if it depended on the C++ fallback.
- Make the fallback state visible in the differential output (`fallback=disabled` or `fallback=cpp-fallback-enabled` or `fallback=native`).

### R3. Parallel Branching / Worktree Strategy
- Spawn a separate git branch and/or worktree if needed for individual components of the task, then merge them cleanly.

### R4. Codebase Hygiene & AI Deslop
- Clean up any unused files, duplicate code, or temporary files introduced.
- Perform a deslop review to remove AI-generated boilerplate or low-quality patterns.
- Keep C++ FFI fallback capabilities available for oracle comparisons when enabled.

## Acceptance Criteria

### Verification & Testing
- [ ] Implement a test proving fallback-allowed runs behave normally (using C++ FFI wrapper).
- [ ] Implement a test proving fallback-disabled runs for unimplemented features fail loudly.
- [ ] Implement a test proving fallback-disabled runs for natively implemented features succeed.
- [ ] The oracle/manifest output fails the `native-pass` check if fallback was active for that case.
- [ ] All workspace tests pass successfully (`cargo test`).
- [ ] Hygiene check: `just quality` and `just static` pass without any warnings.
- [ ] A final PR body is generated using the `codex-pr-body` skill.
