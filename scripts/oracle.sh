#!/usr/bin/env sh
set -eu

export TZ=UTC
export LC_ALL=C
export LANG=C
export QALCULATE_DEFINITIONS_DIR="${QALCULATE_DEFINITIONS_DIR:-../libqalculate/data}"

if [ -n "${QALCULATE_ORACLE:-}" ]; then
  oracle=$QALCULATE_ORACLE
elif [ -x ../libqalculate/src/qalc ]; then
  oracle=../libqalculate/src/qalc
else
  echo "No executable upstream qalc oracle found; parity tests cannot run." >&2
  exit 1
fi

if [ ! -x "$oracle" ]; then
  echo "Configured upstream qalc oracle is not executable: $oracle" >&2
  exit 1
fi

export QALCULATE_ORACLE="$oracle"
cargo test --test oracle -- --nocapture
cargo test --test e2e_cli docs_example -- --test-threads=1 --nocapture
cargo test --test public_calculator_api native_calculator_matches_the_upstream_simple_api_example -- --nocapture
