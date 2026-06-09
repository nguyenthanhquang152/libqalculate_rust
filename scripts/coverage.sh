#!/usr/bin/env sh
set -eu

PROJECT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
PATH="$PROJECT_DIR/.tools/bin:$PATH"

if ! cargo llvm-cov --version >/dev/null 2>&1; then
  echo "cargo-llvm-cov is required: scripts/install-quality-tools.sh" >&2
  exit 127
fi

mkdir -p target/llvm-cov
cargo llvm-cov --all-features --workspace --lcov --output-path target/llvm-cov/lcov.info
