# List available recipes.
help:
    @just --list

# Set up Git precommit hooks for this project (recommended).
[group('setup')]
install-precommit:
    @uv run pre-commit install

# Compiles the Rust code for development.
[group('testing')]
compile:
    @uv run maturin develop

# Compiles Rust, then runs Rust and Python tests.
[group('testing')]
compile-and-test:
    @just compile
    @just test-rust
    @just test-python

# Runs the Rust tests.
[group('testing')]
[working-directory: 'rust']
test-rust $PYO3_PYTHON=`uv python find`:
    @cargo test --no-default-features

# Runs Python tests.
[group('testing')]
test-python $UV_PYTHON=env("UV_PYTHON", ""):
    @uv run pytest --benchmark-skip

# Runs tests under all supported Python versions, plus Rust.
[group('testing')]
test-all:
    just test-python 3.10
    just test-python 3.11
    just test-python 3.12
    just test-python 3.13
    just test-python 3.14
    just test-python 3.14t
    just test-python 3.15
    just test-python 3.15t
    just test-rust

# Populate missing Syrupy snapshots.
[group('testing')]
update-snapshots:
    @uv run pytest --snapshot-update --benchmark-skip


# Format the Rust code.
[group('formatting')]
[working-directory: 'rust']
format-rust:
    @cargo fmt
    @echo 'Formatted Rust code.'

# Format the Python code.
[group('formatting')]
format-python:
    @uv run ruff format
    @echo 'Formatted Python code.'

# Format all code.
[group('formatting')]
format:
    @just format-rust
    @just format-python

# Lint Python code.
[group('linting')]
lint-python:
    @echo 'Checking uv.lock is up to date...'
    @uv lock --check
    @echo 'Running ruff format...'
    @uv run ruff format --check
    @echo 'Running ruff check...'
    @uv run ruff check
    @echo 'Running mypy...'
    @uv run mypy src/grimp tests
    @echo 'Running Import Linter...'
    @uv run lint-imports

# Lint Rust code using cargo fmt and clippy.
[working-directory: 'rust']
[group('linting')]
lint-rust:
    @cargo fmt --check
    @cargo clippy --all-targets --all-features -- -D warnings

# Attempt to fix any clippy errors.
[working-directory: 'rust']
[group('linting')]
autofix-rust:
    @cargo clippy --all-targets --all-features --fix --allow-staged --allow-dirty
    @just format-rust

# Fix any ruff errors.
[group('linting')]
autofix-python:
    @uv run ruff check --fix
    @just format-python

# Fix any lint errors.
[group('linting')]
autofix:
    @just autofix-rust
    @just autofix-python

# Run linters.
[group('linting')]
lint:
    @echo 'Linting Python...'
    @just lint-python
    @echo 'Linting Rust...'
    @just lint-rust
    @echo ''
    @echo '👍 {{GREEN}} Linting all good.{{NORMAL}}'

# Build docs.
[group('docs')]
build-docs:
    @uv run --group=docs sphinx-build -b html docs dist/docs --fail-on-warning --fresh-env --quiet

# Build docs and open in browser.
[group('docs')]
build-and-open-docs:
    @just build-docs
    @open dist/docs/index.html

# Run benchmarks locally using pytest-benchmark.
[group('benchmarking')]
benchmark-local:
    @uv run pytest --benchmark-only --benchmark-autosave
    @just show-benchmark-results

# Show recent local benchmark results.
[group('benchmarking')]
show-benchmark-results:
    @uv run pytest-benchmark compare --group-by=fullname --sort=name --columns=mean

# Run benchmarks using Codspeed. This only works in CI.
[group('benchmarking')]
benchmark-ci:
    @uv run --group=benchmark-ci pytest --codspeed

# Run all linters, build docs and tests. Worth running before pushing to Github.
[group('prepush')]
full-check:
    @just lint
    @just build-docs
    @just test-all
    @echo '👍 {{GREEN}} Linting, docs and tests all good.{{NORMAL}}'

set windows-shell := ["powershell.exe", "-NoProfile", "-NoLogo", "-Command"]
