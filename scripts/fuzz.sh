#!/usr/bin/env sh
set -eu

PROJECT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
PATH="$PROJECT_DIR/.tools/bin:$PATH"

if ! rustup toolchain list | grep -q '^nightly'; then
  echo "nightly Rust is required: scripts/install-quality-tools.sh" >&2
  exit 127
fi

if ! cargo fuzz --version >/dev/null 2>&1; then
  echo "cargo-fuzz is required: scripts/install-quality-tools.sh" >&2
  exit 127
fi

cargo +nightly fuzz run batch_parser -- -runs="${FUZZ_RUNS:-10000}"
