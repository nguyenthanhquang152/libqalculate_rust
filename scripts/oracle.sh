#!/usr/bin/env sh
set -eu

export TZ=UTC
export LC_ALL=C
export LANG=C
export QALCULATE_DEFINITIONS_DIR="${QALCULATE_DEFINITIONS_DIR:-../libqalculate/data}"

if [ -n "${QALCULATE_ORACLE:-}" ]; then
  cargo test --test oracle -- --nocapture
  exit 0
fi

if [ -x ../libqalculate/src/qalc ]; then
  QALCULATE_ORACLE=../libqalculate/src/qalc cargo test --test oracle -- --nocapture
  exit 0
fi

echo "No upstream qalc oracle found; running fixture-inventory oracle tests only." >&2
cargo test --test oracle -- --nocapture
