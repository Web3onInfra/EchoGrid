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

# Run ON/OFF band SNR analysis
band-snr:
    cd research/hum_detection; uv run python -m echogrid_hum.band_snr

# Run detector evaluation benchmark
eval:
    cd research/hum_detection; uv run python -m echogrid_hum.eval

# Deploy oracle to Fly.io
deploy-fly:
    fly deploy

# View live oracle logs on Fly.io
logs-fly:
    fly logs

# Check live oracle status on Fly.io
status-fly:
    fly status
