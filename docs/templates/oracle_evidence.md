# Oracle Evidence: [TASK_ID]

Record one exact Rust-vs-upstream comparison. Inventory-only checks and C++ fallback
agreement are useful scaffold evidence, but do not prove native parity.

## Environment
- **C++ qalc version**: 5.11.0
- **C++ qalc path**: `../libqalculate/src/qalc`
- **Rust qalc-rs version**: [version]
- **Locale**: `LC_ALL=C.UTF-8`
- **Settings**: `-defaults -set decimal_comma 0 -set curconv 0`

## Batch Files Tested
| Batch File | Total Cases | Passed | Failed | Skipped |
|---|---|---|---|---|
| `parser.batch` | [N] | [N] | [N] | [N] |
| `operators.batch` | [N] | [N] | [N] | [N] |
| `numberbase.batch` | [N] | [N] | [N] | [N] |
| `units.batch` | [N] | [N] | [N] | [N] |
| `strings.batch` | [N] | [N] | [N] | [N] |

## Mismatches
| Case ID | Expression | Field | C++ Output | Rust Output | Deviation |
|---|---|---|---|---|---|
| [file:line] | [expr] | stdout | [cpp] | [rust] | [DEV-NNNN or N/A] |

## Commands Run
```bash
QALCULATE_ORACLE=../libqalculate/src/qalc just test-oracle
ORACLE_BATCH=parser.batch cargo test --test oracle -- --ignored differential_oracle_single --nocapture
```

## Fallback Status
- **C++ fallback enabled**: yes / no
- **Fallback-disabled native run**: pass / fail / not applicable
- **Native parity claimed**: yes / no

## Conclusion
[Summary: X/Y cases pass, Z known deviations, overall parity status]
