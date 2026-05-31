# List available recipes (default when running `just`)
default:
    @just --list

### Setup ###############################################################

# Install dependencies (dev included)
install:
    uv sync

# Install pre-commit hooks and sync dependencies
setup: install
    uv run pre-commit install

### Code Quality ########################################################

# Run Ruff linter
lint:
    uv run ruff check .

# Run Ruff linter with auto-fix
lint-fix:
    uv run ruff check --fix .

# Format code with Ruff
format:
    uv run ruff format .

# Check formatting without changes
format-check:
    uv run ruff format --check .

# Run Pyrefly type checker
typecheck:
    uv run pyrefly check

### Testing #############################################################

# Run tests
test:
    uv run pytest

# Run tests with coverage
test-cov:
    uv run pytest --cov=src/aggregator --cov-report=term-missing --cov-report=html

# Run tests with verbose output
test-verbose:
    uv run pytest -v

### Combined ############################################################

# Run all checks (lint + format-check + typecheck + test)
check: lint format-check typecheck test

### Docs ###############################################################

# Build documentation
docs:
    uv run --group docs mkdocs build

# Serve documentation with live reload
docs-serve:
    uv run --group docs mkdocs serve -a localhost:8001

### Maintenance ########################################################

# Update project from template (keeps current settings)
update:
    copier update --defaults

### Cleanup #############################################################

# Remove build artifacts and caches
clean:
    rm -rf .ruff_cache .pytest_cache htmlcov .coverage dist build site
    find . -type d -name __pycache__ -exec rm -rf {} + 2>/dev/null || true
    find . -type d -name "*.egg-info" -exec rm -rf {} + 2>/dev/null || true
