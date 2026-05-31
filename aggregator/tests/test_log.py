"""Tests for the log module."""

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    import pytest

from aggregator.log import get_logger, setup_logging


def test_setup_logging() -> None:
    """setup_logging runs without error."""
    setup_logging()


def test_get_logger_returns_logger() -> None:
    """get_logger returns a structlog BoundLogger."""
    logger = get_logger("test")
    assert logger is not None


def test_get_logger_is_idempotent() -> None:
    """Calling get_logger twice does not raise."""
    get_logger()
    get_logger()


def test_setup_logging_json(monkeypatch: pytest.MonkeyPatch) -> None:
    """LOG_FORMAT=json selects the JSON renderer."""
    monkeypatch.setenv("LOG_FORMAT", "json")
    setup_logging()


def test_setup_logging_console(monkeypatch: pytest.MonkeyPatch) -> None:
    """Default LOG_FORMAT selects the console renderer."""
    monkeypatch.delenv("LOG_FORMAT", raising=False)
    setup_logging()
