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
