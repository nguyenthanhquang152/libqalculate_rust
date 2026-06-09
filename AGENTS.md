@/home/nt-quang/.codex/RTK.md

# Project Context

This repository is the Rust port workspace for `libqalculate`, the C++ Qalculate library and CLI.

The upstream C++ checkout is expected at:

```text
../libqalculate
```

Treat `../libqalculate` as the reference implementation and oracle. Do not edit the upstream checkout unless the user explicitly asks for that.

# Upstream Snapshot

The adjacent upstream checkout currently reports `libqalculate` version `5.11.0` in `configure.ac` and `includes.h`.

Key upstream areas:

- Public API headers: `../libqalculate/libqalculate/*.h`
- Core implementation: `../libqalculate/libqalculate/*.cc`
- CLI and test runners: `../libqalculate/src/qalc.cc`, `../libqalculate/src/test.cc`, `../libqalculate/src/unittest.cc`
- Golden-style batch tests: `../libqalculate/tests/*.batch`
- Definition data: `../libqalculate/data/*.xml.in`, `../libqalculate/data/rates.json`

The major C++ concepts to preserve in Rust are:

- `Calculator`: global/session state, parsing, evaluation, printing, definitions, conversion, messages
- `MathStructure`: expression tree for numbers, units, variables, functions, vectors, comparisons, bitwise/logical forms, and datetimes
- `Number`: arbitrary precision rational/float/complex/infinite values with intervals and uncertainty
- `ExpressionItem` subclasses: functions, units, variables, prefixes, datasets

# Porting Guidance

Prefer a staged port with small, testable Rust modules over transliterating the full C++ system at once.

Recommended order:

1. Create the Rust crate structure and a test harness that can consume upstream `.batch` files or a reduced subset.
2. Port the numeric core enough to match parser and arithmetic expectations.
3. Port the expression AST and parser surface.
4. Add evaluation, simplification, formatting, units, definitions, and higher-level functions incrementally.

Keep compatibility behavior explicit. When Rust code intentionally diverges from C++ internals, document the user-visible behavior being preserved.

# Verification

Use upstream tests as the main compatibility target:

- Start with small focused fixtures copied or referenced from `../libqalculate/tests/*.batch`.
- Add Rust unit tests around every ported semantic area.
- When comparing behavior, use upstream `qalc` or the upstream test runner as the oracle if it is built and available.

Before claiming a ported area is complete, run the relevant Rust tests and, when feasible, compare against the corresponding upstream batch cases.

# Rust Conventions

Follow the repository's established Rust style once a crate exists. Until then, prefer:

- Library-first layout with a CLI wrapper added only when needed.
- Clear domain modules such as `number`, `ast`, `parser`, `eval`, `format`, `units`, and `definitions`.
- Rust ownership and error handling instead of emulating C++ reference counting.
- Exact arithmetic crates or FFI choices only after validating they can represent Qalculate behavior, especially MPFR-style intervals, precision, complex values, infinities, and uncertainty.

## graphify

This project has a knowledge graph at graphify-out/ with god nodes, community structure, and cross-file relationships.

When the user types `/graphify`, invoke the `skill` tool with `skill: "graphify"` before doing anything else.

Rules:
- For codebase questions, first run `graphify query "<question>"` when graphify-out/graph.json exists. Use `graphify path "<A>" "<B>"` for relationships and `graphify explain "<concept>"` for focused concepts. These return a scoped subgraph, usually much smaller than GRAPH_REPORT.md or raw grep output.
- Dirty graphify-out/ files are expected after hooks or incremental updates; dirty graph files are not a reason to skip graphify. Only skip graphify if the task is about stale or incorrect graph output, or the user explicitly says not to use it.
- If graphify-out/wiki/index.md exists, use it for broad navigation instead of raw source browsing.
- Read graphify-out/GRAPH_REPORT.md only for broad architecture review or when query/path/explain do not surface enough context.
- After modifying code, run `graphify update .` to keep the graph current (AST-only, no API cost).
