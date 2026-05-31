"""Structured logging configuration using structlog."""

from __future__ import annotations

import os
import sys

import structlog


def setup_logging() -> None:
    """Configure structlog with sensible defaults.

    Uses JSON output when ``LOG_FORMAT=json`` (e.g. production),
    otherwise uses colored console output for development.
    """
    shared_processors: list[structlog.types.Processor] = [
        structlog.contextvars.merge_contextvars,
        structlog.stdlib.add_log_level,
        structlog.stdlib.add_logger_name,
        structlog.processors.TimeStamper(fmt="iso"),
        structlog.processors.StackInfoRenderer(),
        structlog.processors.UnicodeDecoder(),
    ]

    if os.environ.get("LOG_FORMAT") == "json":
        renderer: structlog.types.Processor = structlog.processors.JSONRenderer()
    else:
        renderer = structlog.dev.ConsoleRenderer()

    structlog.configure(
        processors=[
            *shared_processors,
            renderer,
        ],
        wrapper_class=structlog.make_filtering_bound_logger(0),
        context_class=dict,
        logger_factory=structlog.PrintLoggerFactory(file=sys.stderr),
        cache_logger_on_first_use=True,
    )


_configured = False


def get_logger(*args: object, **kwargs: object) -> structlog.stdlib.BoundLogger:
    """Return a structlog logger, configuring on first call.

    This avoids running ``setup_logging()`` at import time, which would
    interfere with tests and multi-process setups.
    """
    global _configured  # noqa: PLW0603
    if not _configured:
        setup_logging()
        _configured = True
    return structlog.get_logger(*args, **kwargs)
