# aggregator

Aggregator for multiple Raspberry Pi Picos for a dumb smart home

## Setup

```bash
# Install uv
curl -LsSf https://astral.sh/uv/install.sh | sh

# Install just (task runner)
# macOS: brew install just
# Linux: cargo install just (or see https://github.com/casey/just#installation)

# First-time setup (install deps + pre-commit hooks)
just setup
```

## Development

```bash
just check       # Run all checks (lint, format, typecheck, test)
just lint-fix    # Auto-fix linting issues
just format      # Auto-format code
just test-cov    # Run tests with coverage
```

Run `just` to see all available commands.

## Tools

| Tool | Purpose |
|------|---------|
| [uv](https://docs.astral.sh/uv/) | Package & environment management |
| [Ruff](https://docs.astral.sh/ruff/) | Linting & formatting |
| [Pyrefly](https://pyrefly.org/) | Type checking |
| [Pytest](https://docs.pytest.org/) | Testing |
| [Just](https://github.com/casey/just) | Task runner |
| [Pre-commit](https://pre-commit.com/) | Git hooks |

