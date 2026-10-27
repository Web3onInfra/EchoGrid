"""Trivial test asserting package skeleton and deterministic test suite setup."""

import echogrid_hum


def test_package_version() -> None:
    """Verify package version is defined."""
    assert echogrid_hum.__version__ == "0.1.0"


def test_trivial_arithmetic() -> None:
    """Trivial deterministic test to verify test harness execution."""
    assert 1 + 1 == 2
