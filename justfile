# Task runner for vermieterme. CI calls the same recipes (local = CI).
# Install: https://github.com/casey/just   Overview: `just --list`

set shell := ["bash", "-euo", "pipefail", "-c"]

# Directory of the Rust workspace.
backend := "backend"

# List all recipes.
default:
    @just --list

# Full quality gate: everything CI runs (Rust, web, line sizes). The Rust release
# build (`just build`) is left out on purpose: clippy already compiles all targets and
# the release build with LTO is slow; the Docker build covers it.
check: fmt-check lint test deny web-check lines

# Format Rust sources.
fmt:
    cd {{backend}} && cargo fmt --all

# Verify Rust formatting without changing files.
fmt-check:
    cd {{backend}} && cargo fmt --all --check

# Clippy with warnings as errors (all targets).
lint:
    cd {{backend}} && cargo clippy --locked --workspace --all-targets -- -D warnings

# Rust unit and integration tests (SQLite; tests without a service print [skip]).
test:
    cd {{backend}} && cargo test --locked --workspace

# MariaDB/MySQL tests only. Convention for P2+: MySQL-specific tests are named with the
# prefix `mysql_` (module or test name) and skip visibly with [skip] without TEST_MYSQL_URL;
# the SQLite suite runs in `just test` and is not repeated here. Fails if TEST_MYSQL_URL is empty.
test-mysql:
    @test -n "${TEST_MYSQL_URL:-}" || { echo "TEST_MYSQL_URL fehlt (z. B. mysql://root:passwort@127.0.0.1:3306)" >&2; exit 1; }
    cd {{backend}} && cargo test --locked --workspace mysql_

# Release build of the backend binary.
build:
    cd {{backend}} && cargo build --locked --release

# Advisories, licenses and sources of Rust dependencies.
deny:
    cd {{backend}} && cargo deny check

# Source file sizes: warning above 300 lines, error above 400 (allowlist: scripts/lines-allowlist.txt).
lines:
    bash scripts/check-lines.sh

# Web lint (ESLint).
web-lint:
    pnpm exec eslint . --ignore-pattern '.claude/**'

# Web type check.
web-types:
    pnpm exec tsc --noEmit

# Web unit tests (Vitest).
web-test:
    pnpm exec vitest run --exclude '.claude/**' --exclude 'node_modules/**'

# Web production build.
web-build:
    pnpm build

# Whole web gate: lint, types, tests, build.
web-check: web-lint web-types web-test web-build
