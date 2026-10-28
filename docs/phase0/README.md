# Phase 0: De-risk Implementation Checklist

Phase 0 answers two primary risks before any product or frontend work:
- **Track A (Python):** Which frequency bands from AC / fridge / pump / fan hum survive a phone mic through the browser audio pipeline, and what simple detector reliably flags "appliance turned OFF"?
- **Track B (Rust):** Can the Rust oracle build, sign, and submit a Soroban `payout` invocation to Stellar testnet via Soroban RPC (not Horizon alone)?

---

## 15 PR Checklist

### Foundation
- [x] **PR 1: chore: phase-0 scaffolding and CI**
  - Branch: `phase0/01-scaffolding-and-ci`
  - Scope: Checklist (`docs/phase0/README.md`), `research/`, `spikes/`, `.gitignore` entries (`data/`, `*.wav`, `*.env`), path-filtered CI for Rust & Python.

### Track A: Hum Detection (Python)
- [x] **PR 2: feat(research): python project setup**
  - Branch: `phase0/02-python-setup`
  - Scope: `pyproject.toml` (numpy, scipy, soundfile, matplotlib, pytest, ruff), package skeleton (`research/hum_detection/echogrid_hum`), task runner commands (`test`, `lint`, `report`).
- [x] **PR 3: feat(research): dataset fetch scripts and manifest**
  - Branch: `phase0/03-dataset-fetch`
  - Scope: `fetch_datasets.py` with checksummed downloads into `data/public/` (UrbanSound8K AC + negatives, MIMII pump/fan, `--sample` CI flag), `data/MANIFEST.md`, and `NOTICE.md`.
- [x] **PR 4: feat(research): browser capture page and recording protocol**
  - Branch: `phase0/04-capture-page`
  - Scope: Single-file `research/capture/index.html` (raw getUserMedia, audio pipeline diagnostics, ON/OFF sidecar marking), `docs/phase0/recording_protocol.md`, loader and `validate_recordings.py`.
  - **HUMAN GATE 1**: Record real appliance audio with capture page (Completed).
- [x] **PR 5: feat(dsp): band-energy and harmonic features**
  - Branch: `phase0/05-dsp-features`
  - Scope: Pure functions for STFT/Welch band energy (dB), noise floor estimator, mains-harmonic comb energy (50/60 Hz), band SNR helpers, deterministic synthetic tests.
- [x] **PR 6: feat(report): spectral survey report**
  - Branch: `phase0/06-spectral-survey`
  - Scope: Survey script generating PSD and spectrograms (20–500 Hz) into `reports/spectra/`, summary table of detectable bands in `reports/spectra/README.md`.
- [x] **PR 7: feat(analysis): ON/OFF band-SNR analysis**
  - Branch: `phase0/07-band-snr`
  - Scope: Per-band SNR calculation (ON vs OFF energy), `reports/band_snr.md`, definition of usable bands (≥6 dB SNR on ≥80% recordings).
- [x] **PR 8: feat(detector): streaming power-down detector v0**
  - Branch: `phase0/08-streaming-detector`
  - Scope: Frame-based streaming detector (adaptive noise floor, hysteresis, configurable minimum sustained drop), frozen configuration, property tests.
- [x] **PR 9: feat(eval): evaluation harness and report**
  - Branch: `phase0/09-eval-harness`
  - Scope: Evaluation harness across synthetic & real ON/OFF transitions + negative classes, reporting detection rate, false triggers/hr, latency into `reports/eval.md`.
- [x] **PR 10: feat(detector): freeze parameters and golden vectors**
  - Branch: `phase0/10-golden-vectors`
  - Scope: Export `detector_params.json` and golden test vectors (JSON frame series -> expected detector states), CI replay test.

### Track B: Soroban RPC Spike (Rust)
- [x] **PR 11: feat(spike): soroban-rpc spike crate**
  - Branch: `phase0/11-soroban-rpc-spike`
  - Scope: `spikes/soroban_rpc/` workspace binary crate with `health` subcommand calling `getHealth` & `getLatestLedger` on Stellar testnet RPC, documented endpoint & passphrase.
- [x] **PR 12: chore(deploy): testnet deployment scripts**
  - Branch: `phase0/12-testnet-deploy`
  - Scope: `scripts/deploy_testnet.sh` using `stellar` CLI, building/optimizing/deploying `negawatt_escrow`, funding escrow, committing non-secrets to `deployments/testnet.json`.
  - **HUMAN GATE 2**: Funded testnet identity via `stellar` CLI keystore (Completed).
- [x] **PR 13: feat(spike): hardcoded payout invocation**
  - Branch: `phase0/13-hardcoded-payout`
  - Scope: Build Soroban invoke tx, `simulateTransaction`, sign with oracle key, `sendTransaction`, poll `getTransaction`, print explorer link, ADR `docs/phase0/adr-rpc-client.md`.
- [x] **PR 14: test(spike): negative paths and error taxonomy**
  - Branch: `phase0/14-negative-paths`
  - Scope: `#[ignore]`d live integration tests for replayed proof hash, non-oracle signer, insufficient balance; error taxonomy enum for `oracle/src/types.rs`.

### Wrap-up
- [x] **PR 15: docs: Phase 0 decisions and exit report**
  - Branch: `phase0/15-exit-report`
  - Scope: `docs/phase0/DECISIONS.md` go/no-go table, measured metrics, browser audio settings findings, RPC client decision, contract notes, Phase 1/2 recommendations.
