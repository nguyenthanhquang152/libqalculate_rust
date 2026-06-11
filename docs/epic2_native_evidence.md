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

`docs/batch_manifest.md` now marks 55 rows as `native-pass`.

- `parser.batch`: lines 1, 3, 5, 7, 9, 18, 20, 22, 24, 28, 32, 34, 36, 41,
  43, 45, 47, 49, 53.
- `operators.batch`: lines 1, 10, 12, 14, 21, 30, 34, 37, 39, 41, 44, 46,
  48, 51, 53, 55, 58, 60, 62.
- `numberbase.batch`: lines 1, 3, 5, 7, 9, 11, 13, 15, 17, 19, 21, 23,
  25, 28, 32.
- `explog.batch`: lines 4, 7.

The focused and differential oracle commands listed below disable C++ fallback
for the covered promoted rows and verify `fallback=native`. The promoted
`explog.batch:4` and `explog.batch:7` rows are covered by focused native
oracle/e2e/lib evidence, not by a full `explog.batch` differential run; the
other `explog.batch` rows remain inventory-only.

`1 + 1` is also kept as focused native scaffold evidence because
`ORIGINAL_REQUEST.md` explicitly names it as a fallback-disabled scaffold
expression. It is not an upstream batch-manifest promotion and is not counted in
the 55 `native-pass` batch rows above.

## Focused Native Oracle Cases

`tests/oracle.rs::focused_epic2_native_numeric_oracle_cases` compares these
focused expressions with fallback disabled:

- `i`
- `5i`
- `(1 + 2i) + (3 + 4i)`
- `(1 + 2i) * (3 + 4i)`
- `(1 + 2i) / (3 + 4i)`
- `i^2`
- `(2i - 3)^(3.2i + 3)`
- `1/3`
- `1e10`
- `1 + 1`
- `5 ^ 2`
- `2 ^ -3`
- `2 ^ 0.5`
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
- `(2+/-3)^3.2`
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
  oracle probes also cover `2 ^ -3`, `(-2) ^ -3`, `(1/2) ^ -3`, and the
  default-precision non-integer float result for `2 ^ 0.5`.
- Native float `ln` and non-integer float `pow` arithmetic now stay in MPFR
  instead of converting through `f64` for semantic arithmetic. Focused unit
  tests assert 200-bit `ln(2)` and `2^0.5` retain MPFR-scale precision rather
  than a lifted 53-bit result.
- Exact rational remainder and modulo now match upstream qalc for the promoted
  operator rows. `%` and `rem` use quotient truncation toward zero, while `%%`
  and `mod` use floor-division semantics, including negative operands and
  divisors.
- Exact rational integer division now matches upstream qalc for the promoted
  operator rows. `//`, `\`, and `div` return the quotient truncated toward zero,
  including negative operands and fractional rational dividends.
- Numberbase rows now have focused native output support for the exact promoted
  expressions covering binary, octal, hexadecimal, base-32, Roman numeral,
  32-bit float bit-pattern, bitwise-shift/AND-to-binary, `float(...)`,
  `floatError(...)`, `sqrt(n) to base sqrt(m)`, the hexadecimal `p` binary
  exponent expression under `set input base 16`, and Unicode sexagesimal output
  after accumulated input-base/Unicode settings. Other numberbase-looking
  expressions and broader session-setting combinations remain outside the
  fallback-disabled native gate until promoted with oracle evidence.
- Fallback-disabled native qalc-profile output now parses a typed
  `/set precision N` session setting for the promoted numeric evidence path.
  Nonterminating exact rationals are converted with enough MPFR guard precision
  to emit the requested decimal digits. The native evidence gate accepts
  precision values from 1 through 4096 digits to avoid unbounded CLI-requested
  allocation. Focused upstream oracle evidence covers `1/3` under
  `/set precision 128` and native `2 ^ 0.5` under `/set precision 128`;
  precision-enabled non-integer rational powers now evaluate with a
  precision-derived MPFR context instead of the default 53-bit context.
- Fallback-disabled native complex evidence covers imaginary literals and
  selected exact arithmetic output shapes: addition, subtraction,
  multiplication, division, `conj(3 + 4i)`, `norm(3 + 4i)`, exact `i^2`, and
  the no-session `explog.batch:7` complex-power row
  `(2i - 3)^(3.2i + 3)`. These cases are compared against upstream qalc with
  exact UTF-8 output, including Unicode minus signs in qalc-profile CLI output.
  Focused unit regressions keep exact integer powers of `i` out of the
  approximate complex-power branch, including upstream `i^1000000 -> 1`.
- Fallback-disabled native uncertainty evidence covers the first no-session
  `explog.batch` uncertainty power row: `(2+/-3)^3.2` now evaluates natively
  and prints `9.18958684±44.11001683` against upstream qalc. The qalc-profile
  formatter keeps the existing significant-uncertainty output for ordinary
  uncertainty arithmetic, while preserving MPFR fractional digits only for this
  promoted float-valued uncertainty power case.
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
rtk cargo test --lib precision_context_applies_to_noninteger_rational_power -- --nocapture
rtk cargo test --test e2e_cli cli_applies_precision_setting_for_native_rational_output -- --nocapture
rtk cargo test --test e2e_cli cli_applies_precision_setting_for_native_float_power -- --nocapture
rtk cargo test --test oracle focused_epic2_float_precision_oracle_cases -- --nocapture
rtk cargo test --lib scientific_literals_with_impractical_exponents_are_rejected -- --nocapture
rtk cargo test --lib exact_large_rational_compare_does_not_collapse_to_f64_infinity -- --nocapture
rtk cargo test --lib exact_integer_powers_remain_rational_and_parse_starstar -- --nocapture
rtk cargo test --lib rational_modulo_and_remainder_match_qalc_operators -- --nocapture
rtk cargo test --lib rational_integer_division_matches_qalc_operators -- --nocapture
rtk cargo test --lib complex_conjugate_and_norm_parse_natively -- --nocapture
rtk cargo test --test e2e_cli cli_runs_native_complex_subtraction_conjugate_and_norm -- --nocapture
rtk cargo test --lib complex_powers_match_focused_qalc_output -- --nocapture
rtk cargo test --test e2e_cli cli_runs_native_complex_powers -- --nocapture
rtk cargo test --lib uncertainty_power_matches_focused_qalc_display -- --nocapture
rtk cargo test --lib ordinary_uncertainty_power_keeps_significant_uncertainty_display -- --nocapture
rtk cargo test --lib qalc_profile_precise_uncertainty_is_explicit_evidence_mode -- --nocapture
rtk cargo test --lib fixed_decimal_precision_helpers_distinguish_significant_fraction_digits -- --nocapture
rtk cargo test --lib fallback_disabled_runs_native_scaffold_cases -- --nocapture
rtk cargo test --test e2e_cli cli_runs_native_uncertainty_power -- --nocapture
rtk cargo test --test oracle focused_epic2_native_numeric_oracle_cases -- --nocapture
cargo mutants --timeout 180 --jobs 2 --file src/number.rs --file src/ffi.rs -F 'to_qalc_string_preserving_float_uncertainty_precision|format_qalc_value_with_uncertainty_format|format_qalc_uncertainty|fixed_decimal_has_fractional_precision|trim_fixed_decimal_trailing_zeros|mantissa_and_exponent|native_numeric_evidence' -- --lib
rtk cargo test --test oracle differential_oracle_numberbase_batch -- --nocapture
rtk cargo test --test oracle focused_epic2_numberbase_no_session_oracle_cases -- --nocapture
rtk cargo test --test oracle focused_epic2_numberbase_session_oracle_cases -- --nocapture
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

Mutation evidence for this slice:

- Scoped formatter/native-gate run after Codex and internal review fixes:
  23 mutants tested, 22 caught, 1 unviable.

## Remaining Gaps

- Full MPFR option parity and broader arbitrary-precision float oracle coverage
  remain incomplete beyond the promoted native precision-output and
  precision-context non-integer power evidence.
- Broader complex powers and broad `explog.batch` complex cases remain
  incomplete beyond the promoted exact arithmetic, `conj`, `norm`, `i^2`, and
  `explog.batch:7` evidence.
- Interval input syntax, options, intersections, open/closed bounds, and broad
  interval oracle rows remain incomplete. Qalc bracket expressions are not used
  as interval evidence because upstream default qalc treats them with vector-like
  semantics in several operations.
- Remaining uncertainty function examples from `explog.batch`, complex interval
  calculation mode, ASCII/Unicode print-option toggles, and
  session-setting-dependent behavior remain incomplete.
- Division-by-zero-style power output such as `0 ^ -1 -> 1 / 0` remains outside
  the fallback-disabled native subset.
- `Calculator` expression evaluation remains fallback-first outside the vetted
  fallback-disabled native numeric subset.
