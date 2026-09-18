"""
Unit tests for time_utils.py
=============================

Covers:
  - Duration decomposition into days, hours, minutes, seconds
  - Negative duration handling and zero-second inputs
  - Boundary transitions (59s, 60s, 3599s, 3600s, 86399s, 86400s)
  - Floating-point duration inputs
  - Formatting as [D days, ]HH:MM:SS strings
  - Custom timestamp formatting strings
"""

import os
import sys
import time

# Ensure ualbf-project root is on sys.path
project_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if project_root not in sys.path:
    sys.path.insert(0, project_root)

from time_utils import (
    decompose_duration,
    format_hhmmss,
    get_current_timestamp,
)  # noqa: E402


def test_decompose_duration_zero_and_negative():
    assert decompose_duration(0) == (0, 0, 0, 0)
    assert decompose_duration(-1) == (0, 0, 0, 0)
    assert decompose_duration(-100.5) == (0, 0, 0, 0)


def test_decompose_duration_boundary_transitions():
    # 59 seconds
    assert decompose_duration(59) == (0, 0, 0, 59)
    # 60 seconds (1 minute)
    assert decompose_duration(60) == (0, 0, 1, 0)
    # 3599 seconds (59m 59s)
    assert decompose_duration(3599) == (0, 0, 59, 59)
    # 3600 seconds (1 hour)
    assert decompose_duration(3600) == (0, 1, 0, 0)
    # 86399 seconds (23h 59m 59s)
    assert decompose_duration(86399) == (0, 23, 59, 59)
    # 86400 seconds (1 day)
    assert decompose_duration(86400) == (1, 0, 0, 0)
    # 90061 seconds (1d 1h 1m 1s)
    assert decompose_duration(90061) == (1, 1, 1, 1)


def test_decompose_duration_float_inputs():
    assert decompose_duration(0.9) == (0, 0, 0, 0)
    assert decompose_duration(59.999) == (0, 0, 0, 59)
    assert decompose_duration(3661.75) == (0, 1, 1, 1)


def test_format_hhmmss_durations():
    assert format_hhmmss(-10) == "00:00:00"
    assert format_hhmmss(0) == "00:00:00"
    assert format_hhmmss(59) == "00:00:59"
    assert format_hhmmss(60) == "00:01:00"
    assert format_hhmmss(3600) == "01:00:00"
    assert format_hhmmss(3661) == "01:01:01"
    assert format_hhmmss(86399) == "23:59:59"
    assert format_hhmmss(86400) == "1 days, 00:00:00"
    assert format_hhmmss(90061) == "1 days, 01:01:01"
    assert format_hhmmss(172800) == "2 days, 00:00:00"


def test_format_hhmmss_float_inputs():
    assert format_hhmmss(3661.8) == "01:01:01"
    assert format_hhmmss(86400.99) == "1 days, 00:00:00"


def test_get_current_timestamp(monkeypatch):
    # Test default format
    ts_default = get_current_timestamp()
    assert isinstance(ts_default, str)
    assert len(ts_default.split(":")) == 3

    # Test custom formatting string
    custom_fmt = "%Y-%m-%d %H:%M:%S"
    ts_custom = get_current_timestamp(fmt=custom_fmt)
    assert len(ts_custom.split(" ")) == 2

    # Mock time.strftime to verify custom format passing
    called_fmt = []

    def mock_strftime(fmt):
        called_fmt.append(fmt)
        return "MOCKED_TIME"

    monkeypatch.setattr(time, "strftime", mock_strftime)

    res = get_current_timestamp("%S")
    assert res == "MOCKED_TIME"
    assert called_fmt == ["%S"]
