#!/usr/bin/env sh
set -eu

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
