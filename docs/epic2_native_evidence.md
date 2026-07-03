# Epic 2 Native Numeric Evidence

Date: 2026-06-12

This note records fallback-disabled Rust-vs-upstream evidence for the current
Epic 2 numeric slice and narrow boolean comparison probes. It is evidence for
the cases listed here only; it does not claim full `Number.cc` parity.

## Oracle Configuration

- Upstream oracle: `../libqalculate/src/qalc`
- Upstream arguments: `-defaults -terse -set "decimal_comma 0" -set "curconv 0"`
- Definitions: `../libqalculate/data`
- Locale/timezone: `LC_ALL=C.UTF-8`, `TZ=UTC`
- Rust fallback state: `QALCULATE_DISABLE_FALLBACK=1`,
  `QALCULATE_REPORT_FALLBACK=1`

## Native-Pass Batch Rows

This Epic 2 note covers 55 numeric/native scaffold rows that were promoted to
`native-pass` in `docs/batch_manifest.md`. Later epics may add additional
native-pass rows outside this note's scope.

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
- `(1 + 2i) - (3 + 4i)`
- `(1 + 2i) * (3 + 4i)`
- `(1 + 2i) / (3 + 4i)`
- `i + (-i)`
- `(1 + i) + (-1 + i)`
- `(1 + i) + (2 - i)`
- `(1 + i) * (1 - i)`
- `(1 + i) / (1 - i)`
- `conj(3 + 4i)`
- `conj(i)`
- `conj(-i)`
- `conj(3)`
- `norm(3 + 4i)`
- `norm(i)`
- `norm(-3i)`
- `i^2`
- `(2i - 3)^(3.2i + 3)`
- `(1 + i) = (1 + i)`
- `(1 + i) == (1 + i)`
- `(1 + i) = (1 - i)`
- `(1 + i) != (1 - i)`
- `(1 + i) ≠ (1 - i)`
- `(1 + i) != (1 + i)`
- `(1 + i) < (1 + i)`
- `(1 + i) <= (1 + i)`
- `(1 + i) > (1 + i)`
- `(1 + i) >= (1 + i)`
- `(1 + i) ≤ (1 + i)`
- `(1 + i) ≥ (1 + i)`
- `ln(0)`
- `ln(2)`
- `ln(5+/-0.3)`
- `sqrt(2)`
- `sqrt(4)`
- `infinity`
- `-infinity`
- `infinity + 1`
- `-infinity - 1`
- `infinity * 2`
- `infinity * -2`
- `1 / infinity`
- `infinity / 2`
- `infinity / -2`
- `-infinity / 2`
- `-infinity / -2`
- `1 / -infinity`
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

`tests/oracle.rs::focused_issue41_vector_matrix_literal_oracle_cases` adds
focused matrix/vector evidence outside the Epic 2 numeric row count. The
`matrixvector.batch` magnitude rows promoted in Refs #41 are:

- `matrixvector.batch:214`: `magnitude(-2) -> 2`
- `matrixvector.batch:216`: `magnitude([-2]) -> 2`
- `matrixvector.batch:218`: `magnitude([-2, 3, 4]) -> 5.385164807`

The native gate is source-exact for these three promoted spellings and remains
fallback-disabled for equivalent aliases such as `magnitude(-2.0)` and
`magnitude(-4/2)`. Precision settings remain outside this slice because the
vector path computes default-precision `sqrt(29)` before formatting.

The focused vector/matrix oracle test also records the `matrixvector.batch`
`norm` rows promoted in Refs #41:

- `matrixvector.batch:253`: `norm([2]) -> 2`
- `matrixvector.batch:255`: `norm([3, 4]) -> 5`
- `matrixvector.batch:257`: `norm([2, 3, 6]) -> 7`

The native `norm` gate is source-exact for these three promoted spellings and
remains fallback-disabled for equivalent aliases such as `norm([2.0])`,
`norm([4/2])`, and `norm([3,4])`. Explicit session settings remain outside
this default-setting slice and are rejected for the promoted norm forms.

The same focused oracle test also records the `matrixvector.batch` `part` rows
promoted in Refs #41:

- `matrixvector.batch:221`: `part([1], 1, 1, 1, 1) -> 1`
- `matrixvector.batch:223`:
  `part([1 2 3; 4 5 6; 7 8 9; 10 11 12], 2, 2, 2, 2) -> 5`
- `matrixvector.batch:225`:
  `part([1 2 3; 4 5 6; 7 8 9; 10 11 12], 1, 3, 2, 3) -> [3; 6]`
- `matrixvector.batch:227`:
  `part([1 2 3; 4 5 6; 7 8 9; 10 11 12], 1, 2, 4, 3) -> [2  3; 5  6; 8  9; 11  12]`

The native `part` gate is source-exact for these four promoted spellings and
remains fallback-disabled for equivalent aliases such as
`part([1], 1.0, 1, 1, 1)` and unrelated subranges.

`tests/oracle.rs::focused_issue15_uncertainty_input_oracle_cases` adds a
focused Refs #15 input/API slice without changing batch-manifest counts:

- `2 +/- 0.002`
- `2 +/- 0.002 + 3`
- `2±0.002`
- `2±0.002 + 3`
- `uncertainty(2;0.002;0)`
- `uncertainty(100;0.05;1)`
- `uncertainty(10;0;0)`
- `errorPart(2+/-0.002)`
- `errorPart(100+/-5%)`
- `errorPart(2+/-0.002;0)`
- `errorPart(2+/-0.002;1)`
- `errorPart(100+/-5%;0)`
- `errorPart(100+/-5%;1)`
- `valuePart(2+/-0.002)`
- `valuePart(100+/-5%)`
- `midpoint(2+/-0.002)`
- `lowerEndpoint(2+/-0.002)`
- `upperEndpoint(2+/-0.002)`
- `20+/-3 - 10+/-4`
- `3+/-0.2 / 4+/-0.1`
- `1.23(4)` under `/set concise uncertainty 1`
- `123(4)` under `/set concise uncertainty 1`
- `1.23(4) + 2.0(3)` under `/set concise uncertainty 1`

Upstream default `1.23(4)` remains a multiplication expression and prints
`4.92`; it is intentionally not accepted as native uncertainty evidence unless
`/set concise uncertainty 1` is present.

`tests/oracle.rs::focused_epic2_float_precision_oracle_cases` records focused
Refs #12 precision-context rows:

- `1/3` under `/set precision 128`
- `2 ^ 0.5` under `/set precision 128`
- `(2 ^ 0.5) + (3 ^ 0.5)` under `/set precision 64` and
  `/set precision 128`
- `(3 ^ 0.5) - (2 ^ 0.5)` under `/set precision 64` and
  `/set precision 128`
- `(2 ^ 0.5) * (3 ^ 0.5)` under `/set precision 64` and
  `/set precision 128`
- `(3 ^ 0.5) / (2 ^ 0.5)` under `/set precision 64` and
  `/set precision 128`
- `(2 ^ 0.5) + 1/3` under `/set precision 64` and
  `/set precision 128`
- `0.1 + 0.2` under `/set precision 64` and `/set precision 128`
- `1.25e-20 + 2.5e-20` under `/set precision 64` and
  `/set precision 128`
- `2.5e3 / 4` under `/set precision 64` and `/set precision 128`
- `(2 ^ 0.5) < (3 ^ 0.5)` under `/set precision 64` and
  `/set precision 128`
- `(2 ^ 0.5) = (2 ^ 0.5)` under `/set precision 64` and
  `/set precision 128`
- `(2 ^ 0.5) = (3 ^ 0.5)` under `/set precision 64` and
  `/set precision 128`
- `(2 ^ 0.5) + 1/3 > 1` under `/set precision 64` and
  `/set precision 128`
- `(2 ^ 0.5) < 1/3` under `/set precision 64` and `/set precision 128`

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
- Native float `ln`, `sqrt`, and non-integer float `pow` arithmetic now stay in
  MPFR instead of converting through `f64` for semantic arithmetic. Focused
  unit tests assert 200-bit `ln(2)` and `2^0.5` retain MPFR-scale precision
  rather than a lifted 53-bit result.
- Refs #12 finite MPFR arithmetic evidence covers add/sub/mul/div for the
  promoted non-integer power rows under both `/set precision 64` and
  `/set precision 128`, including the mixed exact/approx row
  `(2 ^ 0.5) + 1/3`. The same precision pair is covered for focused approximate
  comparison rows, so the native gate no longer has only single-precision
  evidence for these operations.
- The fallback-disabled native expression scaffold now exposes only the focused
  `ln(...)` and `sqrt(...)` function cases promoted by oracle evidence. Default
  evidence covers `ln(0) -> −∞`, `ln(2) -> 0.6931471806`,
  `ln(5+/-0.3) -> 1.609±0.060`, `sqrt(2) -> 1.414213562`, and exact-square
  `sqrt(4) -> 2`. Refs #12 precision-context evidence now also covers scalar
  function rows under `/set precision 64` and `/set precision 128`:
  `ln(0)`, `ln(2)`, `sqrt(2)`, `sqrt(4)`, and `ln(2) + sqrt(2)`; broader
  special functions, negative-domain behavior, symbolic simplifications, and
  full MPFR option parity remain outside the native gate.
- Fallback-disabled native Refs #12 special-value evidence covers alphabetic
  infinity literals and selected arithmetic: `infinity -> +∞`,
  `-infinity -> −∞`, `infinity + 1 -> +∞`, `-infinity - 1 -> −∞`,
  `infinity * 2 -> +∞`, `infinity * -2 -> −∞`, `1 / infinity -> 0`,
  `infinity / 2 -> +∞`, `infinity / -2 -> −∞`, `-infinity / 2 -> −∞`,
  `-infinity / -2 -> +∞`, and `1 / -infinity -> 0`. The native expression
  tokenizer accepts only the qalc-compatible `infinity` name as a whole literal
  before passing it to the existing `Number` parser; internal `inf`/`nan`
  `Number::from_str` roundtrips remain outside the expression evidence gate.
  Upstream probes showed exact division by zero and indeterminate infinity
  forms remain symbolic, e.g. `1 / 0 -> 1 / 0`, `0 / 0 -> 0 / 0`,
  `infinity - infinity -> (+∞) − (+∞)`, `infinity + -infinity -> (+∞) − (+∞)`,
  and `0 * infinity -> 0(+∞)`, so those forms stay fallback-disabled.
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
  `/set precision 128`, native `2 ^ 0.5` under `/set precision 128`, and
  finite real MPFR arithmetic over precision-context non-integer powers:
  `(2 ^ 0.5) + (3 ^ 0.5)`, `(3 ^ 0.5) - (2 ^ 0.5)`,
  `(2 ^ 0.5) * (3 ^ 0.5)`, `(3 ^ 0.5) / (2 ^ 0.5)`, and
  `(2 ^ 0.5) + 1/3`. Precision-context evidence now also covers exact
  decimal/scientific input rows at two settings: `0.1 + 0.2`, `1.25e-20 +
  2.5e-20`, and `2.5e3 / 4` under `/set precision 64` and `/set precision 128`.
  Precision-enabled non-integer rational powers now evaluate with a
  precision-derived MPFR context instead of the default 53-bit context, and the
  promoted add/sub/mul/div cases preserve that MPFR precision through ordinary
  real arithmetic. Refs #12 comparison evidence is also setting-gated and covers
  `true`/`false` qalc outputs for `(2 ^ 0.5) < (3 ^ 0.5)`, `(2 ^ 0.5) =
  (2 ^ 0.5)`, `(2 ^ 0.5) = (3 ^ 0.5)`, `(2 ^ 0.5) + 1/3 > 1`, and
  `(2 ^ 0.5) < 1/3` under `/set precision 128`.
- Fallback-disabled native complex evidence covers imaginary literals and
  selected exact arithmetic output shapes: addition, subtraction,
  multiplication, division, zero-collapse (`i + (-i)`), pure-real collapse,
  pure-imaginary preservation, `conj(...)`, `norm(...)`, exact `i^2`, and the
  no-session `explog.batch:7` complex-power row `(2i - 3)^(3.2i + 3)`. This
  slice now also includes focused complex equality/inequality constraints for
  `=`, `==`, `!=`, and `≠`, plus the upstream-resolved equal-operand ordering
  constraints `<`, `<=`, `>`, `>=`, `≤`, and `≥`. Non-equal complex ordering
  remains outside the native evidence gate because upstream qalc keeps those
  expressions symbolic. These cases are compared against upstream qalc with
  exact UTF-8 output, including Unicode minus signs in qalc-profile CLI output.
  Focused unit regressions keep exact integer powers of `i` out of the
  approximate complex-power branch, including upstream `i^1000000 -> 1`, assert
  that `Number::new_complex` drops internal imaginary metadata when the
  canonical imaginary component is zero while preserving exact/approx state,
  and cover mixed exact/approx real and imaginary components across native
  add/sub/mul/div, `conj`, `norm`, and promoted powers.
- Focused upstream qalc probes on 2026-06-12 confirmed the non-equal complex
  ordering boundary remains symbolic and therefore outside native success
  evidence: `(1 + i) < (1 + 2i) -> 1 + i < 1 + 2i`,
  `(1 + i) <= (1 + 2i) -> 1 + i ≤ 1 + 2i`,
  `(1 + i) > (1 + 2i) -> 1 + i > 1 + 2i`,
  `(1 + i) >= (1 + 2i) -> 1 + i ≥ 1 + 2i`,
  `(1 + i) ≤ (1 + 2i) -> 1 + i ≤ 1 + 2i`, and
  `(1 + i) ≥ (1 + 2i) -> 1 + i ≥ 1 + 2i`. The fallback-disabled Rust gate
  rejects those same expressions instead of claiming a boolean result.
- Fallback-disabled native uncertainty evidence covers the first no-session
  `explog.batch` uncertainty power row: `(2+/-3)^3.2` now evaluates natively
  and prints `9.18958684±44.11001683` against upstream qalc. The qalc-profile
  formatter keeps the existing significant-uncertainty output for ordinary
  uncertainty arithmetic, while preserving MPFR fractional digits only for this
  promoted float-valued uncertainty power case. Refs #15 function propagation
  evidence now also covers the focused real-valued natural-log row
  `ln(5+/-0.3) -> 1.609±0.060`; this is an allowlisted native evidence case,
  not broad special-function support.
- Fallback-disabled native Refs #15 uncertainty input-form evidence now covers
  spaced ASCII absolute uncertainty input `2 +/- 0.002 -> 2.0000±0.0020`,
  `2 +/- 0.002 + 3 -> 5.0000±0.0020`, Unicode absolute uncertainty input
  `2±0.002 -> 2.0000±0.0020`, and `2±0.002 + 3 -> 5.0000±0.0020`.
  Concise uncertainty notation is supported only for vetted setting-gated cases
  under `/set concise uncertainty 1`: `1.23(4) -> 1.230±0.040`,
  `123(4) -> 123.0±4.0`, and `1.23(4) + 2.0(3) -> 3.23±0.30`.
- Fallback-disabled native Refs #15 uncertainty API evidence now covers the
  scalar constructor/extraction slice: `uncertainty(2;0.002;0) ->
  2.0000±0.0020`, `uncertainty(100;0.05;1) -> 100.0±5.0`,
  `uncertainty(10;0;0) -> 10`, `errorPart(2+/-0.002) -> 0.002000000000`,
  `errorPart(100+/-5%) -> 5`, `errorPart(2+/-0.002;0) -> 0.002000000000`,
  `errorPart(2+/-0.002;1) -> 0.001000000000`,
  `errorPart(100+/-5%;0) -> 5`, `errorPart(100+/-5%;1) -> 0.05000000000`,
  `valuePart(2+/-0.002) -> 2`, `valuePart(100+/-5%) -> 100`,
  `midpoint(2+/-0.002) -> 2`, `lowerEndpoint(2+/-0.002) -> 1.998000000`,
  and `upperEndpoint(2+/-0.002) -> 2.002000000`. Propagation evidence also now
  includes `20+/-3 - 10+/-4 -> 10.0±5.0` and
  `3+/-0.2 / 4+/-0.1 -> 0.750±0.053`. Complex uncertainty, Lambert W, Ei,
  interval calculation mode, ASCII/Unicode print-option toggles, and broad
  `explog.batch` promotions remain incomplete; `Ei(3+/-0.3)` is covered only
  by a fallback-disabled rejection guard.
- Interval construction now follows upstream `Number::setInterval` in
  `../libqalculate/libqalculate/Number.cc`: finite reversed endpoints are
  accepted and stored in lower/upper order, and equal finite endpoints collapse
  to a scalar value. Focused upstream probes confirm `interval(5;2)` and
  `interval(2;5)` both print `interval(2.000000000, 5.000000000)` with
  interval-display mode, while `interval(2;2)` prints `2`. Refs #14 infinity
  endpoint construction evidence now also covers `interval(-infinity;5) ->
  interval(−∞, 5.000000000)`, `interval(4;infinity) ->
  interval(4.000000000, +∞)`, `interval(-infinity;-4) ->
  −interval(4.000000000, +∞)`, and `interval(-3;-1) ->
  −interval(1.000000000, 3.000000000)` under `/set interval display 2`, and
  the same constructor/display rows remain native when `/set ic 2` is also
  active. The negative-only interval display mirrors upstream's sign-outside
  formatting policy for interval-display mode. Refs #14 optional-bound evidence
  now covers the finite integer rows `interval(1;3;0)` and `interval(1;3;1)`,
  which both match upstream `interval(1.000000000, 3.000000000)` under interval
  display mode. Decimal optional-bound rows such as `interval(1.1;3.3;1)`
  remain rejected natively because upstream moves those endpoints according to
  precision/outward-rounding rules that this slice does not model.
- Finite closed interval arithmetic now has a fallback-disabled native evidence
  path only when both `/set interval display 2` and `/set ic 2` are active. The
  promoted endpoint-mode cases are `interval(1;2) + interval(3;4)`,
  `interval(3;4) - interval(1;2)`,
  `interval(-2;3) * interval(-4;5)`,
  `interval(4;6) / interval(2;3)`,
  `interval(4;6) / interval(-3;-2)`,
  `interval(-6;-4) / interval(2;3)`, and
  `interval(-6;-4) / interval(-3;-2)`. Native expression parsing supports
  `interval(lower; upper)` as a numeric primary for this vetted path. A focused
  negative guard compares upstream display-only variance-mode output against
  endpoint-mode output for multiplication and keeps Rust fallback-disabled
  without `/set ic 2`.
- Refs #14 infinity endpoint arithmetic evidence extends the endpoint-mode gate
  to these exact expressions only: `interval(-infinity;5) + interval(2;3) ->
  interval(−∞, 8.000000000)`, `interval(-infinity;5) - interval(2;3) ->
  interval(−∞, 3.000000000)`, `interval(-infinity;5) * interval(2;3) ->
  interval(−∞, 15.00000000)`, `interval(4;infinity) + interval(2;3) ->
  interval(6.000000000, +∞)`, `interval(4;infinity) - interval(2;3) ->
  interval(1.000000000, +∞)`, `interval(4;infinity) * interval(2;3) ->
  interval(8.000000000, +∞)`, and `interval(4;infinity) / 2 ->
  interval(2.000000000, +∞)`, `interval(-infinity;-4) / 2 ->
  −interval(2.000000000, +∞)`, and `interval(-infinity;-4) / -2 ->
  interval(2.000000000, +∞)`. Upstream probes showed `inf` input is ambiguous
  and interval divisions where the denominator contains zero or an
  infinity-bounded interval often remain symbolic/unevaluated upstream, such as
  `interval(4;6) / interval(-1;1)`,
  `interval(4;6) / interval(0;2)`,
  `interval(4;infinity) / interval(2;4)`, and
  `2 / interval(4;infinity)`, so those forms are rejected when fallback is
  disabled.
- Refs #14 endpoint extraction evidence covers upstream
  `../libqalculate/libqalculate/BuiltinFunctions-number.cc` and
  `../libqalculate/libqalculate/Number.cc` paths for
  `lowerEndpoint(interval(1;3)) -> 1.000000000`,
  `upperEndpoint(interval(1;3)) -> 3.000000000`,
  `midpoint(interval(1;3)) -> 2`,
  `lowerEndpoint(interval(1;3;1)) -> 1.000000000`,
  `upperEndpoint(interval(1;3;1)) -> 3.000000000`,
  `midpoint(interval(1;3;1)) -> 2`,
  `lowerEndpoint(interval(-infinity;-4)) -> −∞`, and
  `upperEndpoint(interval(4;infinity)) -> +∞` under the same interval-display
  evidence gate.
- Refs #14 interval intersection evidence is intentionally narrow. Upstream
  `intersect` is the vector/set function in
  `../libqalculate/libqalculate/BuiltinFunctions-matrixvector.cc`, not a broad
  interval-overlap operator. The concrete disjoint case
  `intersect(interval(1;2), interval(3;4)) -> []` is native and
  fallback-disabled. Overlapping interval inputs such as
  `intersect(interval(1;4), interval(3;6))` remain symbolic upstream and are
  guarded as unsupported native evidence.
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
rtk cargo fmt --check
rtk git diff --check
rtk cargo check --tests
rtk timeout 600 cargo test --all-targets --all-features
rtk cargo test --test number_behavior rational_from_str_exposes_lossless_arbitrary_precision_surface -- --nocapture
rtk cargo test --lib test_arbitrary_precision_rationals_do_not_fall_back_to_i128_surface -- --nocapture
rtk cargo test --lib test_new_rational_arithmetic_and_comparisons -- --nocapture
rtk cargo test --lib qalc_profile_formats_nonterminating_and_large_rationals_like_upstream -- --nocapture
rtk cargo test --lib precision_context_applies_to_noninteger_rational_power -- --nocapture
rtk cargo test --lib precision_context_applies_to_scalar_log_and_sqrt_functions -- --nocapture
rtk cargo test --lib precision_context_applies_to_real_float_arithmetic -- --nocapture
rtk cargo test --lib precision_context_decimal_and_scientific_rows_stay_exact_without_f64_shortcuts -- --nocapture
rtk cargo test --lib precision_context_real_float_comparisons_match_upstream_booleans -- --nocapture
rtk cargo test --lib native_log_and_sqrt_functions_match_qalc_profile -- --nocapture
rtk cargo test --lib qalc_profile_formats_infinities_with_upstream_signs -- --nocapture
rtk cargo test --lib special_value_literals_parse_in_expressions_with_name_boundaries -- --nocapture
rtk cargo test --test e2e_cli cli_applies_precision_setting_for_native_rational_output -- --nocapture
rtk cargo test --test e2e_cli cli_applies_precision_setting_for_native_float_power -- --nocapture
rtk cargo test --test e2e_cli cli_applies_precision_setting_for_native_log_and_sqrt_functions -- --exact --nocapture
rtk cargo test --test e2e_cli cli_applies_precision_setting_for_native_real_float_arithmetic -- --nocapture
rtk cargo test --test e2e_cli cli_rejects_native_real_float_arithmetic_without_precision_setting -- --exact --nocapture
rtk cargo test --test e2e_cli cli_applies_precision_setting_for_native_decimal_scientific_float_arithmetic -- --exact --nocapture
rtk cargo test --test e2e_cli cli_applies_precision_setting_for_native_real_float_comparisons -- --exact --nocapture
rtk cargo test --test e2e_cli cli_runs_native_float_log_and_sqrt_functions -- --nocapture
rtk cargo test --test e2e_cli cli_runs_native_infinity_arithmetic -- --exact --nocapture
rtk cargo test --test e2e_cli cli_runs_native_signed_infinity_division -- --exact --nocapture
rtk cargo test --test e2e_cli cli_rejects_unsupported_uncertainty_special_function_when_fallback_disabled -- --nocapture
rtk cargo test --test oracle focused_epic2_float_precision_oracle_cases -- --nocapture
rtk cargo test --lib scientific_literals_with_impractical_exponents_are_rejected -- --nocapture
rtk cargo test --lib exact_large_rational_compare_does_not_collapse_to_f64_infinity -- --nocapture
rtk cargo test --lib exact_integer_powers_remain_rational_and_parse_starstar -- --nocapture
rtk cargo test --lib rational_modulo_and_remainder_match_qalc_operators -- --nocapture
rtk cargo test --lib rational_integer_division_matches_qalc_operators -- --nocapture
rtk cargo test --lib complex_zero_part_metadata_collapses_without_losing_exact_or_approx_state -- --nocapture
rtk cargo test --lib complex_conjugate_and_norm_parse_natively -- --nocapture
rtk cargo test --test e2e_cli cli_runs_native_complex_subtraction_conjugate_and_norm -- --nocapture
rtk cargo test --test fallback_gate cli_native_expression_succeeds_when_fallback_disabled -- --nocapture
rtk cargo test --lib complex_powers_match_focused_qalc_output -- --nocapture
rtk cargo test --test e2e_cli cli_runs_native_complex_powers -- --nocapture
rtk cargo test --test e2e_cli cli_runs_native_complex_equality_constraints -- --nocapture
rtk cargo test --lib complex_exact_approx_components_survive_native_operations -- --nocapture
rtk cargo test --lib complex_ordering_constraints_match_upstream_symbolic_boundary -- --nocapture
rtk cargo test --test e2e_cli cli_runs_native_complex_ordering_constraints -- --exact --nocapture
rtk cargo test --test fallback_gate cli_invalid_native_expression_fails_when_fallback_disabled -- --nocapture
rtk cargo test --lib uncertainty_power_matches_focused_qalc_display -- --nocapture
rtk cargo test --lib ordinary_uncertainty_power_keeps_significant_uncertainty_display -- --nocapture
rtk cargo test --lib qalc_profile_precise_uncertainty_is_explicit_evidence_mode -- --nocapture
rtk cargo test --lib fixed_decimal_precision_helpers_distinguish_significant_fraction_digits -- --nocapture
rtk cargo test --lib fallback_disabled_runs_native_scaffold_cases -- --nocapture
rtk cargo test --test e2e_cli cli_runs_native_uncertainty_power -- --nocapture
rtk cargo test --test oracle focused_epic2_native_numeric_oracle_cases -- --nocapture
rtk cargo test --test fallback_gate cli_invalid_native_expression_fails_when_fallback_disabled -- --nocapture
rtk cargo test --test e2e_cli cli_runs_native_unicode_uncertainty_input -- --nocapture
rtk cargo test --test e2e_cli cli_runs_native_uncertainty_api_functions -- --exact --nocapture
rtk cargo test --test e2e_cli cli_applies_concise_uncertainty_setting_for_native_input -- --nocapture
rtk cargo test --test e2e_cli cli_rejects_concise_uncertainty_without_setting -- --nocapture
rtk cargo test --test oracle focused_issue15_uncertainty_input_oracle_cases -- --nocapture
rtk cargo test --lib uncertainty_constructor -- --nocapture
rtk cargo test --test uncertainty_adversarial -- --nocapture
rtk proxy env QALCULATE_DEFINITIONS_DIR=../libqalculate/data LC_ALL=C.UTF-8 TZ=UTC ../libqalculate/src/qalc -defaults -terse -set 'decimal_comma 0' -set 'curconv 0' '2±0.002'
rtk proxy env QALCULATE_DEFINITIONS_DIR=../libqalculate/data LC_ALL=C.UTF-8 TZ=UTC ../libqalculate/src/qalc -defaults -terse -set 'decimal_comma 0' -set 'curconv 0' 'errorPart(2+/-0.002)'
rtk env QALCULATE_DEFINITIONS_DIR=../libqalculate/data LC_ALL=C.UTF-8 TZ=UTC ../libqalculate/src/qalc -defaults -terse -set 'decimal_comma 0' -set 'curconv 0' 'errorPart(2+/-0.002;1)'
rtk env QALCULATE_DEFINITIONS_DIR=../libqalculate/data LC_ALL=C.UTF-8 TZ=UTC ../libqalculate/src/qalc -defaults -terse -set 'decimal_comma 0' -set 'curconv 0' 'errorPart(100+/-5%;1)'
rtk proxy env QALCULATE_DEFINITIONS_DIR=../libqalculate/data LC_ALL=C.UTF-8 TZ=UTC ../libqalculate/src/qalc -defaults -terse -set 'decimal_comma 0' -set 'curconv 0' '2±0.002 + 3'
rtk proxy env QALCULATE_DEFINITIONS_DIR=../libqalculate/data LC_ALL=C.UTF-8 TZ=UTC ../libqalculate/src/qalc -defaults -terse -set 'decimal_comma 0' -set 'curconv 0' 'ln(5+/-0.3)'
rtk proxy env QALCULATE_DEFINITIONS_DIR=../libqalculate/data LC_ALL=C.UTF-8 TZ=UTC ../libqalculate/src/qalc -defaults -terse -set 'decimal_comma 0' -set 'curconv 0' 'Ei(3+/-0.3)'
rtk proxy env QALCULATE_DEFINITIONS_DIR=../libqalculate/data LC_ALL=C.UTF-8 TZ=UTC ../libqalculate/src/qalc -defaults -terse -set 'decimal_comma 0' -set 'curconv 0' -set 'concise uncertainty 1' '1.23(4)'
rtk proxy env QALCULATE_DEFINITIONS_DIR=../libqalculate/data LC_ALL=C.UTF-8 TZ=UTC ../libqalculate/src/qalc -defaults -terse -set 'decimal_comma 0' -set 'curconv 0' -set 'concise uncertainty 1' '123(4)'
rtk proxy env QALCULATE_DEFINITIONS_DIR=../libqalculate/data LC_ALL=C.UTF-8 TZ=UTC ../libqalculate/src/qalc -defaults -terse -set 'decimal_comma 0' -set 'curconv 0' -set 'concise uncertainty 1' '1.23(4) + 2.0(3)'
rtk proxy env QALCULATE_DEFINITIONS_DIR=../libqalculate/data LC_ALL=C.UTF-8 TZ=UTC ../libqalculate/src/qalc -defaults -terse -set 'decimal_comma 0' -set 'curconv 0' '1.23(4)'
rtk env LC_ALL=C.UTF-8 TZ=UTC QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -terse -set 'decimal_comma 0' -set 'curconv 0' 'infinity'
rtk env LC_ALL=C.UTF-8 TZ=UTC QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -terse -set 'decimal_comma 0' -set 'curconv 0' '-infinity'
rtk env LC_ALL=C.UTF-8 TZ=UTC QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -terse -set 'decimal_comma 0' -set 'curconv 0' 'infinity + 1'
rtk env LC_ALL=C.UTF-8 TZ=UTC QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -terse -set 'decimal_comma 0' -set 'curconv 0' '-infinity - 1'
rtk env LC_ALL=C.UTF-8 TZ=UTC QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -terse -set 'decimal_comma 0' -set 'curconv 0' 'infinity * 2'
rtk env LC_ALL=C.UTF-8 TZ=UTC QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -terse -set 'decimal_comma 0' -set 'curconv 0' 'infinity * -2'
rtk env LC_ALL=C.UTF-8 TZ=UTC QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -terse -set 'decimal_comma 0' -set 'curconv 0' '1 / infinity'
rtk env LC_ALL=C.UTF-8 TZ=UTC QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -terse -set 'decimal_comma 0' -set 'curconv 0' '1 / 0'
cargo mutants --timeout 180 --jobs 2 --file src/number.rs --file src/ffi.rs -F 'to_qalc_string_preserving_float_uncertainty_precision|format_qalc_value_with_uncertainty_format|format_qalc_uncertainty|fixed_decimal_has_fractional_precision|trim_fixed_decimal_trailing_zeros|mantissa_and_exponent|native_numeric_evidence' -- --lib
rtk cargo test --test oracle differential_oracle_numberbase_batch -- --nocapture
rtk cargo test --test oracle focused_epic2_numberbase_no_session_oracle_cases -- --nocapture
rtk cargo test --test oracle focused_epic2_numberbase_session_oracle_cases -- --nocapture
rtk cargo test --test number_challenger -- --nocapture
rtk cargo test --test number_behavior interval_literal_parsing_normalizes_reversed_bounds -- --nocapture
rtk cargo test --test number_behavior interval_literal_parsing_collapses_equal_bounds_to_scalar -- --nocapture
rtk cargo test --test number_behavior interval_function_is_numeric_primary_in_native_expression -- --nocapture
rtk cargo test --test number_behavior interval_function_accepts_optional_excluded_endpoint_flag_for_integer_bounds -- --exact --nocapture
rtk cargo test --test number_behavior interval_function_rejects_optional_excluded_endpoint_flag_for_decimal_bounds -- --exact --nocapture
rtk cargo test --test number_behavior interval_function_rejects_unprobed_optional_excluded_endpoint_integer_bounds -- --exact --nocapture
rtk cargo test --test number_behavior interval_ -- --nocapture
rtk cargo test --test number_properties interval_constructor_collapses_equal_bounds_to_scalar -- --nocapture
rtk cargo test --test number_properties interval_constructor -- --nocapture
rtk cargo test --lib parses_supported_settings -- --nocapture
rtk cargo test --test e2e_cli cli_applies_interval_display_setting_for_native_interval_function -- --exact --nocapture
rtk cargo test --test e2e_cli cli_applies_interval_display_setting_for_native_infinity_interval_function -- --nocapture
rtk cargo test --test e2e_cli cli_allows_ic2_for_native_infinity_interval_display_function -- --nocapture
rtk cargo test --test e2e_cli cli_runs_native_interval_arithmetic_with_ic2_endpoint_mode -- --nocapture
rtk cargo test --test e2e_cli cli_runs_native_interval_endpoint_functions -- --exact --nocapture
rtk cargo test --test e2e_cli cli_runs_native_interval_non_overlap_intersection -- --exact --nocapture
rtk cargo test --test e2e_cli cli_rejects_infinity_interval_arithmetic_without_ic2_endpoint_mode -- --nocapture
rtk cargo test --test e2e_cli cli_rejects_interval_arithmetic_without_ic2_endpoint_mode -- --nocapture
rtk cargo test --test e2e_cli cli_rejects_symbolic_interval_division_and_intersection_rows -- --exact --nocapture
rtk cargo test --test e2e_cli interval_arithmetic -- --nocapture
rtk cargo test --test oracle focused_epic2_interval_display_oracle_cases -- --exact --nocapture
rtk cargo test --test oracle focused_epic2_interval_endpoint_oracle_cases -- --exact --nocapture
rtk cargo test --test oracle focused_epic2_interval_intersection_oracle_cases -- --exact --nocapture
rtk cargo test --test oracle focused_epic2_interval_arithmetic_oracle_cases -- --nocapture
rtk cargo test --test oracle focused_epic2_interval_arithmetic_requires_ic2_guard -- --nocapture
rtk cargo test --test oracle focused_epic2_interval_arithmetic -- --nocapture
rtk cargo test --lib
rtk env QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -t '2e303'
rtk env QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -t '12e303'
rtk env QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -t '123456789012345'
rtk env QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -t '129999999999999'
rtk env QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -t -set 'interval display 2' 'interval(5;2)'
rtk env QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -t -set 'interval display 2' 'interval(2;5)'
rtk env QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -t -set 'interval display 2' 'interval(2;2)'
rtk env LC_ALL=C.UTF-8 TZ=UTC QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -terse -set 'decimal_comma 0' -set 'curconv 0' -set 'interval display 2' -set 'ic 2' 'interval(-infinity;5)'
rtk env LC_ALL=C.UTF-8 TZ=UTC QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -terse -set 'decimal_comma 0' -set 'curconv 0' -set 'interval display 2' -set 'ic 2' 'interval(4;infinity)'
rtk env LC_ALL=C.UTF-8 TZ=UTC QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -terse -set 'decimal_comma 0' -set 'curconv 0' -set 'interval display 2' -set 'ic 2' 'interval(-infinity;5) + interval(2;3)'
rtk env LC_ALL=C.UTF-8 TZ=UTC QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -terse -set 'decimal_comma 0' -set 'curconv 0' -set 'interval display 2' -set 'ic 2' 'interval(-infinity;5) - interval(2;3)'
rtk env LC_ALL=C.UTF-8 TZ=UTC QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -terse -set 'decimal_comma 0' -set 'curconv 0' -set 'interval display 2' -set 'ic 2' 'interval(-infinity;5) * interval(2;3)'
rtk env LC_ALL=C.UTF-8 TZ=UTC QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -terse -set 'decimal_comma 0' -set 'curconv 0' -set 'interval display 2' -set 'ic 2' 'interval(4;infinity) + interval(2;3)'
rtk env LC_ALL=C.UTF-8 TZ=UTC QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -terse -set 'decimal_comma 0' -set 'curconv 0' -set 'interval display 2' -set 'ic 2' 'interval(4;infinity) - interval(2;3)'
rtk env LC_ALL=C.UTF-8 TZ=UTC QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -terse -set 'decimal_comma 0' -set 'curconv 0' -set 'interval display 2' -set 'ic 2' 'interval(4;infinity) * interval(2;3)'
rtk env LC_ALL=C.UTF-8 TZ=UTC QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -terse -set 'decimal_comma 0' -set 'curconv 0' -set 'interval display 2' -set 'ic 2' 'interval(4;infinity) / 2'
rtk env LC_ALL=C.UTF-8 TZ=UTC QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -terse -set 'decimal_comma 0' -set 'curconv 0' -set 'interval display 2' -set 'ic 2' 'interval(4;infinity) / interval(2;4)'
rtk env QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -t -set 'interval display 2' -set 'ic 2' 'lowerEndpoint(interval(1;3))'
rtk env QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -t -set 'interval display 2' -set 'ic 2' 'upperEndpoint(interval(1;3))'
rtk env QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -t -set 'interval display 2' -set 'ic 2' 'midpoint(interval(1;3))'
rtk env QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -t -set 'interval display 2' -set 'ic 2' 'intersect(interval(1;2), interval(3;4))'
rtk env QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -t -set 'interval display 2' -set 'ic 2' 'intersect(interval(1;4), interval(3;6))'
rtk env QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -t -set 'interval display 2' -set 'ic 2' 'interval(1;2) + interval(3;4)'
rtk env QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -t -set 'interval display 2' -set 'ic 2' 'interval(3;4) - interval(1;2)'
rtk env QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -t -set 'interval display 2' -set 'ic 2' 'interval(-2;3) * interval(-4;5)'
rtk env QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -t -set 'interval display 2' -set 'ic 2' 'interval(4;6) / interval(2;3)'
rtk env QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -t -set 'interval display 2' -set 'ic 2' 'interval(4;6) / interval(-3;-2)'
rtk env QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -t -set 'interval display 2' -set 'ic 2' 'interval(-6;-4) / interval(2;3)'
rtk env QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -t -set 'interval display 2' -set 'ic 2' 'interval(-6;-4) / interval(-3;-2)'
rtk env QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -t -set 'interval display 2' -set 'ic 2' 'interval(-infinity;-4) / 2'
rtk env QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -t -set 'interval display 2' -set 'ic 2' 'interval(-infinity;-4) / -2'
rtk env QALCULATE_DEFINITIONS_DIR=../libqalculate/data ../libqalculate/src/qalc -defaults -t -set 'interval display 2' 'interval(-2;3) * interval(-4;5)'
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
  precision-context non-integer power/arithmetic, decimal/scientific input,
  comparison, `ln`, `sqrt`, and focused infinity literal/arithmetic evidence.
- Broad complex special-function behavior remains incomplete beyond the
  promoted exact arithmetic, `conj`, `norm`, `i^2`, focused equality/inequality
  constraints, equal-operand ordering constraints, and `explog.batch:7`
  evidence. Non-equal complex ordering is intentionally not claimed by the
  native gate because upstream qalc leaves those expressions symbolic.
- Interval input syntax, interval options beyond `/set ic 2`, open/closed bound
  variants beyond the probed `interval(a;b;exclude)` no-output-change case,
  broad infinity arithmetic including symbolic denominator-containing-zero
  divisions, overlapping interval intersection semantics, uncertainty
  intervals, complex intervals, precision conversion, and broad interval oracle
  rows remain incomplete. Qalc bracket expressions are not used as interval
  evidence because upstream default qalc treats them with vector-like semantics
  in several operations.
- Remaining uncertainty function examples from `explog.batch`, complex interval
  calculation mode, ASCII/Unicode print-option toggles, and
  session-setting-dependent behavior remain incomplete.
- Division-by-zero-style output such as `1 / 0` and `0 ^ -1 -> 1 / 0` remains
  outside the fallback-disabled native subset because upstream keeps these as
  symbolic expressions rather than promoting them to infinity.
- `Calculator` expression evaluation remains fallback-first outside the vetted
  fallback-disabled native numeric subset.
