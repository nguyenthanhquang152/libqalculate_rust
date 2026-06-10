# Epic 2 Native Numeric Evidence

Date: 2026-06-10

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

`docs/batch_manifest.md` now marks 29 rows as `native-pass`.

- `parser.batch`: lines 1, 3, 5, 7, 9, 18, 20, 22, 24, 28, 32, 34, 36, 41,
  43, 45, 47, 49, 53.
- `operators.batch`: lines 1, 10, 12, 14, 21, 30, 34, 58, 60, 62.

The oracle runner disables C++ fallback for these rows and verifies
`fallback=native`.

## Focused Native Oracle Cases

`tests/oracle.rs::focused_epic2_native_numeric_oracle_cases` compares these
focused expressions with fallback disabled:

- `i`
- `5i`
- `(1 + 2i) + (3 + 4i)`
- `(1 + 2i) * (3 + 4i)`
- `(1 + 2i) / (3 + 4i)`
- `1/3`
- `1e10`
- `5 ^ 2`
- `2 ^ -3`
- `(-2) ^ -3`
- `(1/2) ^ -3`
- `5 ** 3`
- `4 ** 3 ** 2`
- `2+/-0.002`
- `100+/-5%`
- `100+/-5 + 200+/-10%`
- `100+/-5% + 200+/-10%`
- `100+/-5% * 2`
- `20+/-3 + 10+/-4`
- `3+/-0.2 * 4+/-0.1`
- `12+/-0.5 / 3+/-0.2`
- `10 +/- 0`

## Native Representation Invariants

- `Rational` now exposes a public lossless arbitrary-precision construction and
  inspection surface: `str::parse::<Rational>()`,
  `Rational::numerator_string()`, and `Rational::denominator_string()`.
  The older `Rational::num()` / `Rational::den()` compatibility accessors still
  return `i128` and intentionally panic when the exact value exceeds that range.
- `Rational::checked_add`, `checked_sub`, `checked_mul`, and `checked_div` now
  use the native `rug`-backed exact rational operations instead of narrowing
  through the old `i128` compatibility surface. They preserve exact
  arbitrary-precision results beyond the `i128` range; checked division still
  returns `None` for division by zero.
- qalc-profile formatting now uses scientific notation for large exact integer
  rationals beyond the upstream display threshold, including non-power-of-ten
  mantissas. Focused upstream probes covered `2e303 -> 2E303`,
  `12e303 -> 1.2E304`, `123456789012345 -> 1.234567890E14`, and
  `129999999999999 -> 1.300000000E14`.
- Exact rational comparisons now unwrap zero-uncertainty values before falling
  back to approximate interval comparison, preserving ordering for values such
  as `1e10000 +/- 0` and `2e10000 +/- 0`.
- Exact rational integer powers now stay rational when both exponent magnitude
  and estimated exact-result size are within the native guard, including
  negative exponents. Qalc `**` exponent syntax is parsed as right-associative
  power syntax. Upstream `operators.batch` rows `5 ^ 2`, `5 ** 3`, and
  `4 ** 3 ** 2` are promoted to fallback-disabled native evidence; focused
  oracle probes also cover `2 ^ -3`, `(-2) ^ -3`, and `(1/2) ^ -3`.
- Interval construction now follows upstream `Number::setInterval` in
  `../libqalculate/libqalculate/Number.cc`: finite reversed endpoints are
  accepted and stored in lower/upper order, and equal finite endpoints collapse
  to a scalar value. Focused upstream probes confirm `interval(5;2)` and
  `interval(2;5)` both print `interval(2.000000000, 5.000000000)` with
  interval-display mode, while `interval(2;2)` prints `2`.
- Native interval literal parsing is an internal `Number` parser surface, not
  qalc bracket syntax parity. It now uses the same public constructor invariant
  for finite reversed endpoints, so `"[5, 1]".parse::<Number>()` stores lower
  `1` and upper `5` instead of rejecting the literal. Exactly equal parsed
  endpoints collapse before float conversion, preserving exact scalar metadata
  for `"[2, 2]"`. Upstream default qalc still treats `[5,2]` as vector-like
  syntax (`[5  2]`), so bracket syntax remains outside native qalc oracle
  claims.
- `Number::try_new_interval` rejects NaN bounds, and `Number::new_interval`
  maps NaN-bound inputs to `NaN` instead of storing an invalid interval.
- The public safe interval constructors enforce ordered non-NaN endpoints; the
  raw `NumberValue::Interval` enum variant remains constructible for existing
  Rust API compatibility.
- `tests/number_properties.rs::interval_constructor_normalizes_reversed_bounds`
  and `tests/number_properties.rs::interval_constructor_rejects_nan_bounds`
  cover the public constructor invariant, including reversed mixed-precision
  bounds and lower/upper NaN inputs.

## Verified Commands

```sh
rtk cargo check --tests
rtk cargo test --test number_behavior rational_from_str_exposes_lossless_arbitrary_precision_surface -- --nocapture
rtk cargo test --lib test_arbitrary_precision_rationals_do_not_fall_back_to_i128_surface -- --nocapture
rtk cargo test --lib test_new_rational_arithmetic_and_comparisons -- --nocapture
rtk cargo test --lib qalc_profile_formats_nonterminating_and_large_rationals_like_upstream -- --nocapture
rtk cargo test --lib scientific_literals_with_impractical_exponents_are_rejected -- --nocapture
rtk cargo test --lib exact_large_rational_compare_does_not_collapse_to_f64_infinity -- --nocapture
rtk cargo test --lib exact_integer_powers_remain_rational_and_parse_starstar -- --nocapture
rtk cargo test --test number_challenger -- --nocapture
rtk cargo test --test number_behavior interval_literal_parsing_normalizes_reversed_bounds -- --nocapture
rtk cargo test --test number_behavior interval_literal_parsing_collapses_equal_bounds_to_scalar -- --nocapture
rtk cargo test --test number_properties interval_constructor_collapses_equal_bounds_to_scalar -- --nocapture
rtk cargo test --test number_properties interval_constructor -- --nocapture
rtk cargo test --lib
rtk env QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -t '2e303'
rtk env QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -t '12e303'
rtk env QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -t '123456789012345'
rtk env QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -t '129999999999999'
rtk env QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -t -set 'interval display 2' 'interval(5;2)'
rtk env QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -t -set 'interval display 2' 'interval(2;5)'
rtk env QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -t -set 'interval display 2' 'interval(2;2)'
rtk env QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -t '[5,2]'
rtk cargo test --test uncertainty_adversarial
rtk cargo test --test fallback_gate cli_native_expression_succeeds_when_fallback_disabled -- --nocapture
rtk cargo test --test fallback_gate cli_invalid_native_expression_fails_when_fallback_disabled -- --nocapture
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
- Division-by-zero-style power output such as `0 ^ -1 -> 1 / 0` remains outside
  the fallback-disabled native subset.
- `Calculator` expression evaluation remains fallback-first outside the vetted
  fallback-disabled native numeric subset.
