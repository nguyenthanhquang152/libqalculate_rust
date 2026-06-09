#!/usr/bin/env sh
set -eu

PROJECT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
PATH="$PROJECT_DIR/.tools/bin:$PATH"

cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings

if ! cargo deny --version >/dev/null 2>&1; then
  echo "cargo-deny is required for dependency policy: scripts/install-quality-tools.sh" >&2
  exit 127
fi

cargo deny check -A license-not-encountered
