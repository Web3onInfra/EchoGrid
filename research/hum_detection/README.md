# echogrid_hum

Acoustic hum detection and spectral analysis research package for EchoGrid Phase 0.

## Commands
Run tasks using `just` or `uv`:
- `just test` — run pytest test suite
- `just lint` — run ruff linter and formatting checks
- `just report` — run spectral report generation
- `uv run python export_feature_fixtures.py` — regenerate STFT band-energy parity fixtures in `research/fixtures/feature_parity.json` for TypeScript and Rust ports
