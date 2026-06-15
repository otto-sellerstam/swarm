set windows-powershell := true

# List available recipes (default when running `just`)
default:
    @just --list

### Rust ################################################################

[working-directory: './embedded']
flash bin:
    cargo run --bin {{bin}} --release

### Setup ###############################################################

# Install dependencies (dev included)
[working-directory: './aggregator']
pyinstall:
    uv sync

# Install pre-commit hooks and sync dependencies
[working-directory: './aggregator']
setup: pyinstall
    uv run pre-commit install

### Code Quality ########################################################

# Run Ruff linter
[working-directory: './aggregator']
pylint:
    uv run ruff check .

# Run Ruff linter with auto-fix
[working-directory: './aggregator']
pylint-fix:
    uv run ruff check --fix .

# Format code with Ruff
[working-directory: './aggregator']
pyformat:
    uv run ruff format .

# Check formatting without changes
[working-directory: './aggregator']
pyformat-check:
    uv run ruff format --check .

# Run Pyrefly type checker
[working-directory: './aggregator']
pytypecheck:
    uv run pyrefly check

### Testing #############################################################

# Run tests
[working-directory: './aggregator']
pytest:
    uv run pytest

# Run tests with coverage
[working-directory: './aggregator']
pytest-cov:
    uv run pytest --cov=src/aggregator --cov-report=term-missing --cov-report=html

# Run tests with verbose output
[working-directory: './aggregator']
pytest-verbose:
    uv run pytest -v

### Docs ###############################################################

# Build documentation
[working-directory: './aggregator']
pydocs:
    uv run --group docs mkdocs build

# Serve documentation with live reload
[working-directory: './aggregator']
pydocs-serve:
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
