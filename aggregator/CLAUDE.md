# CLAUDE.md

This file provides guidance for Claude (or any AI assistant) working on this project.

## Project Overview

**aggregator** — Aggregator for multiple Raspberry Pi Picos for a dumb smart home

## Tech Stack

- **Python 3.14+** with **uv** for package/environment management
- **Ruff** for linting and formatting (ALL rules enabled)
- **Pyrefly** for static type checking (strict mode)
- **Pytest** for testing (with coverage via pytest-cov)
- **Just** as task runner
- **Pre-commit** for git hooks

## Key Commands

```bash
just setup         # First-time: install deps + pre-commit hooks
just check         # Run ALL checks (lint + format + typecheck + test)
just lint-fix      # Ruff linter with auto-fix
just format        # Ruff formatter
just typecheck     # Pyrefly type checker
just test          # Pytest
just test-cov      # Pytest with coverage
```

## Code Conventions

- **Ruff** with `select = ["ALL"]` — every rule enabled, explicit ignores in pyproject.toml.
- **Google-style** docstrings.
- **Pyrefly strict** — all parameters must have type annotations.
- **Coverage threshold: 80%**.
- Always run `just check` before committing.
- Pre-commit hooks enforce formatting automatically.

## Project Layout

```
src/aggregator/    # Source code
tests/                    # Tests (no docstrings/annotations required)
pyproject.toml            # All tool configuration
justfile                  # Developer commands
```

