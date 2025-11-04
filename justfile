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
test-rust:
    @cargo test --no-default-features

# Runs tests under the default Python version.
[group('testing')]
test-python:
    @uv run pytest --benchmark-skip

# Runs tests under all supported Python versions, plus Rust.
[group('testing')]
test-all: test-python-3-9 test-python-3-10 test-python-3-11 test-python-3-12 test-python-3-13 test-python-3-14 test-rust

# Runs tests under Python 3.9.
[group('testing')]
test-python-3-9:
    UV_PYTHON=3.9 just test-python

# Runs tests under Python 3.10.
[group('testing')]
test-python-3-10:
    UV_PYTHON=3.10 just test-python

# Runs tests under Python 3.11.
[group('testing')]
test-python-3-11:
    UV_PYTHON=3.11 just test-python

# Runs tests under Python 3.12.
[group('testing')]
test-python-3-12:
    UV_PYTHON=3.12 just test-python

# Runs tests under Python 3.13.
[group('testing')]
test-python-3-13:
    UV_PYTHON=3.13 just test-python

# Runs tests under Python 3.14.
[group('testing')]
test-python-3-14:
    UV_PYTHON=3.14 just test-python

# Populate missing Syrupy snapshots.
[group('testing')]
update-snapshots:
    @uv run pytest --snapshot-update --benchmark-skip


# Format the Rust code.
[group('formatting')]
[working-directory: 'rust']
format-rust:
    @cargo fmt
    @echo Formatted Rust code.

# Format the Python code.
[group('formatting')]
format-python:
    @uv run ruff format
    @echo Formatted Python code.

# Format all code.
[group('formatting')]
format:
    @just format-rust
    @just format-python

# Lint Python code.
[group('linting')]
lint-python:
    @echo Running ruff format...
    @uv run ruff format --check
    @echo Running ruff check...
    @uv run ruff check
    @echo Running mypy...
    @uv run mypy src/grimp tests
    @echo Running Import Linter...
    @uv run lint-imports

# Lint Rust code using cargo fmt and clippy
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

# Fix any ruff errors
[group('linting')]
autofix-python:
    @uv run ruff check --fix

# Run linters.
[group('linting')]
lint:
    @echo Linting Python...
    @just lint-python
    @echo Linting Rust...
    @just lint-rust
    @echo
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