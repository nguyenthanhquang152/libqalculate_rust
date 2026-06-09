set shell := ["sh", "-cu"]

export PATH := justfile_directory() + "/.tools/bin:" + env("PATH")

# List available recipes.
default:
    @just --list

# Run the normal pre-merge gate.
quality:
    scripts/quality.sh

# Run compile checks, Clippy, and dependency policy.
static:
    scripts/static.sh

# Check Rust formatting.
fmt:
    cargo fmt --check

# Apply Rust formatting.
fmt-fix:
    cargo fmt

# Compile all targets.
check:
    cargo check --all-targets --all-features

# Run Clippy with warnings denied.
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# Build docs with warnings denied.
doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features

# Run all tests.
test:
    cargo test --all-targets --all-features

# Run library unit tests.
test-unit:
    cargo test --lib

# Run integration smoke tests.
test-smoke:
    cargo test --test integration_smoke

# Run CLI e2e tests.
test-e2e:
    cargo test --test e2e_cli

# Run regression fixture tests.
test-regression:
    cargo test --test regression

# Run property-based tests.
test-property:
    cargo test --test property

# Run oracle tests against upstream fixtures and qalc when available.
test-oracle:
    scripts/oracle.sh

# Validate the checked-in batch manifest against upstream fixture case IDs.
manifest-check:
    sh scripts/check-batch-manifest.sh

# Generate LCOV coverage.
coverage:
    scripts/coverage.sh

# Run a bounded fuzz campaign. Override with `just fuzz 100000`.
fuzz runs="10000":
    FUZZ_RUNS={{runs}} scripts/fuzz.sh

# Run mutation testing. Override with `just mutation 180 4`.
mutation timeout="120" jobs="2":
    MUTATION_TIMEOUT={{timeout}} MUTATION_JOBS={{jobs}} scripts/mutation.sh

# Install optional local quality tools into .tools/bin.
install-tools:
    scripts/install-quality-tools.sh

# Run deeper local checks after quality tools are installed.
deep: quality static coverage test-oracle
