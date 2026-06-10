# Epic 2 Native Numeric Evidence

Date: 2026-06-11

This note records fallback-disabled Rust-vs-upstream evidence for the current
Epic 2 numeric slice. It is evidence for the cases listed here only; it does not
claim full `Number.cc` parity.

## Oracle Configuration

- Upstream oracle: `../libqalculate/src/qalc`
- Upstream arguments: `-defaults -terse -set "decimal_comma 0" -set "curconv 0"`
- Definitions: `../libqalculate/data`
- Locale/timezone: `LC_ALL=C.UTF-8`, `TZ=UTC`
- Rust fallback state: `QALCULATE_DISABLE_FALLBACK=1`,
  `QALCULATE_REPORT_FALLBACK=1`

## Native-Pass Batch Rows

`docs/batch_manifest.md` now marks 26 rows as `native-pass`.

- `parser.batch`: lines 1, 3, 5, 7, 9, 18, 20, 22, 24, 28, 32, 34, 36, 41,
  43, 45, 47, 49, 53.
- `operators.batch`: lines 1, 10, 12, 14, 21, 30, 34.

The oracle runner disables C++ fallback for these rows and verifies
`fallback=native`.

## Focused Native Oracle Cases

`tests/oracle.rs::focused_epic2_native_numeric_oracle_cases` compares these
non-batch expressions with fallback disabled:

- `i`
- `5i`
- `(1 + 2i) + (3 + 4i)`
- `(1 + 2i) * (3 + 4i)`
- `(1 + 2i) / (3 + 4i)`
- `1/3`
- `1e10`
- `2+/-0.002`
- `100+/-5%`
- `100+/-5 + 200+/-10%`
- `100+/-5% + 200+/-10%`
- `100+/-5% * 2`
- `20+/-3 + 10+/-4`
- `3+/-0.2 * 4+/-0.1`
- `12+/-0.5 / 3+/-0.2`
- `10 +/- 0`

## Verified Commands

```sh
rtk cargo test --lib
rtk cargo test --test uncertainty_adversarial
rtk cargo test --test fallback_gate cli_native_expression_succeeds_when_fallback_disabled -- --nocapture
rtk cargo test --test oracle -- --nocapture
rtk cargo test --test batch_manifest_validation
rtk cargo test --test inventory_validation
rtk cargo test --test api_parity_validation
rtk timeout 600 just quality
rtk timeout 240 just test-oracle
rtk timeout 600 just coverage
```

## Remaining Gaps

- Full arbitrary-precision float semantics, precision context, and MPFR option
  parity remain incomplete.
- Complex powers and broad `explog.batch` complex cases remain incomplete.
- Interval input syntax, options, intersections, open/closed bounds, and broad
  interval oracle rows remain incomplete. Qalc bracket expressions are not used
  as interval evidence because upstream default qalc treats them with vector-like
  semantics in several operations.
- Uncertainty function examples from `explog.batch`, ASCII/Unicode print-option
  toggles, and session-setting-dependent behavior remain incomplete.
- `Calculator` expression evaluation remains fallback-first outside the vetted
  fallback-disabled native numeric subset.
