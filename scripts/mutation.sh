#!/usr/bin/env sh
set -eu

PROJECT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
PATH="$PROJECT_DIR/.tools/bin:$PATH"

if ! cargo mutants --version >/dev/null 2>&1; then
  echo "cargo-mutants is required: scripts/install-quality-tools.sh" >&2
  exit 127
fi

cargo mutants --timeout "${MUTATION_TIMEOUT:-120}" --jobs "${MUTATION_JOBS:-2}"
