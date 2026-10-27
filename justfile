# EchoGrid Task Runner (justfile)
set shell := ["powershell", "-c"]

default: test

# Run Python research test suite
test:
    cd research/hum_detection; uv run pytest

# Run linter and code style formatting checks
lint:
    cd research/hum_detection; uv run ruff check .; uv run ruff format --check .

# Run spectral report generation
report:
    cd research/hum_detection; uv run python -m echogrid_hum.report
