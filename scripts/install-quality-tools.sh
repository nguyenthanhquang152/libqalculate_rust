#!/usr/bin/env sh
set -eu

PROJECT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
TOOL_ROOT="${TOOL_ROOT:-$PROJECT_DIR/.tools}"

rustup install nightly
cargo install --root "$TOOL_ROOT" cargo-fuzz
cargo install --root "$TOOL_ROOT" cargo-mutants
cargo install --root "$TOOL_ROOT" cargo-llvm-cov
cargo install --root "$TOOL_ROOT" cargo-deny

echo "Installed quality tools into $TOOL_ROOT/bin"
