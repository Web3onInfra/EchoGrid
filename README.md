# ⚡ EchoGrid — Decentralized NegaWatt Marketplace

[![CI](https://github.com/Web3onInfra/EchoGrid/actions/workflows/ci.yml/badge.svg)](https://github.com/Web3onInfra/EchoGrid/actions/workflows/ci.yml)
[![Stellar](https://img.shields.io/badge/Stellar-Testnet-blue.svg)](https://stellar.org)
[![Soroban](https://img.shields.io/badge/Soroban-Smart_Contracts-purple.svg)](https://soroban.stellar.org)
[![Rust](https://img.shields.io/badge/Rust-1.80+-orange.svg)](https://www.rust-lang.org)
[![React](https://img.shields.io/badge/React-18-61dafb.svg)](https://react.dev)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.5-blue.svg)](https://www.typescriptlang.org)
[![License](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-green.svg)](LICENSE)

> *"To balance the grid in Ghana, we don't need millions of dollars in imported smart meters. We just need to listen."*

---

> [!IMPORTANT]
> **Honest Framing & Trust Model**
>
> The EchoGrid MVP trusts the client's claim of an appliance power-down. While cryptographic challenge nonces, recipient cooldowns, and on-chain spending caps strictly limit potential abuse, they do not prove that an appliance physically turned off. This live demonstration showcases the end-to-end micropayment loop and real-time acoustic detection user experience on Stellar Testnet; physical and cryptographic verification remains post-MVP work.

---

## 🌐 Live Deployments & Network Details

EchoGrid is deployed and operational on the **Stellar Testnet**:

| Component | Target / Environment | URL / Identifier |
| :--- | :--- | :--- |
| **PWA Web Client** | Vercel Production | [https://echogrid-pwa.vercel.app](https://echogrid-pwa.vercel.app) |
| **Live Oracle Health** | Fly.io Microservice | [`GET https://echogrid-oracle.fly.dev/api/v1/health`](https://echogrid-oracle.fly.dev/api/v1/health) |
| **Live Oracle Readiness** | Fly.io Self-Check Probe | [`GET https://echogrid-oracle.fly.dev/api/v1/ready`](https://echogrid-oracle.fly.dev/api/v1/ready) |
| **Soroban Smart Contract** | Stellar Testnet Escrow | [`CDFOK4NF4CY27N4G5PYA6SXTJA5MEKPVU7DQRMJUS74LIXHOKYNRHOPF`](https://stellar.expert/explorer/testnet/contract/CDFOK4NF4CY27N4G5PYA6SXTJA5MEKPVU7DQRMJUS74LIXHOKYNRHOPF) |
| **Testnet Token (USDC)** | Stellar SAC Asset | [`CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC`](https://stellar.expert/explorer/testnet/contract/CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC) |
| **Confirmed Payout Tx** | Testnet Settlement Example | [`4e8e6154ba3cae2c8e815b7067750cdb4178ab9ba70ad57aa834846185936506`](https://stellar.expert/explorer/testnet/tx/4e8e6154ba3cae2c8e815b7067750cdb4178ab9ba70ad57aa834846185936506) |
| **OpenAPI Specification** | Oracle Documentation | [docs/api/openapi.yaml](docs/api/openapi.yaml) |

Contract details, deployment transaction hashes, and public keys are tracked in [`deployments/testnet.json`](deployments/testnet.json).

---

## 🏗️ Architecture & Data Flow

```mermaid
sequenceDiagram
    autonumber
    participant App as Mobile PWA Client<br/>(Web Audio DSP)
    participant Oracle as Rust Oracle Service<br/>(Axum + SQLite WAL)
    participant Contract as Soroban Escrow<br/>(Stellar Testnet)
    participant Ledger as Stellar Ledger

    Note over App: 1. User listens to appliance mains hum (50/60 Hz harmonics)
    App->>Oracle: GET /api/v1/challenge?wallet=G...
    Oracle-->>App: { challenge_id, nonce, expires_at }
    
    Note over App: 2. Appliance shuts down; Web Worker FFT detects hum drop
    Note over App: 3. ProofBuilder hashes pre-image with challenge nonce
    App->>Oracle: POST /api/v1/proof<br/>{ wallet, challenge_id, proof_v1 }
    
    Note over Oracle: 4. Oracle verifies nonce, rate limits, and cooldowns
    Oracle->>Contract: Invoke payout(recipient, proof_hash)
    Contract->>Ledger: Transfer 1.0 USDC from escrow pool to recipient
    Ledger-->>Contract: Transaction finalized
    Contract-->>Oracle: Emit 'negawatt_payout' event + tx_hash
    Oracle-->>App: { status: "confirmed", tx_hash: "0x..." }
    Note over App: 5. Displays receipt with StellarExpert link
```

---

## 🔍 What's Real vs. Mocked

Per our canonical trust specification in [`docs/trust-model.md`](docs/trust-model.md):

| Category | Component / Feature | Current Implementation Status |
| :--- | :--- | :--- |
| **Real** | **Soroban Escrow on Testnet** | Contract [`CDFOK4NF4CY...`](https://stellar.expert/explorer/testnet/contract/CDFOK4NF4CY27N4G5PYA6SXTJA5MEKPVU7DQRMJUS74LIXHOKYNRHOPF) enforces caps, daily budgets, replay guards, and issues token payouts. |
| **Real** | **Live Rust Oracle Daemon** | Axum service on Fly.io ([`echogrid-oracle.fly.dev`](https://echogrid-oracle.fly.dev)) with SQLite WAL store, challenge nonces, and serialized queue submission. |
| **Real** | **Testnet Payouts** | Automated token transfers on Stellar Testnet for valid proof submissions. |
| **Real** | **Microphone Capture & DSP** | Web Audio API + Web Worker Cooley-Tukey Radix-2 FFT detects 50/60 Hz appliance hum drop-offs in real time. |
| **Mocked** | **Grid Strain Data** | Grid conditions (Green / Amber / Red / Emergency) are simulated client-side (`GridView.tsx`) without live utility SCADA integration. |
| **Mocked** | **Simulator-Mode Claims** | A toggleable demo simulator injects synthetic hum waveforms; claims carry `simulation: true` per the Phase 2 oracle policy. |
| **Not Built Yet** | **Hardware / Watermark Verification** | Smart-plug telemetry, acoustic watermarking, or physical shutdown proofs. |
| **Not Built Yet** | **Coarse Geofencing** | IP or cellular proximity attestation verifying claimant location within the strained substation zone. |
| **Not Built Yet** | **Native On-Device ML** | Quantized models (TFLite/ONNX) for multi-appliance acoustic fingerprinting. |
| **Not Built Yet** | **Decentralized Oracles** | Multi-party consensus, threshold signatures, or slashing mechanisms. |
| **Not Built Yet** | **Utility SCADA / OpenADR** | Automated OpenADR 2.0b integration with electric utility dispatch centers. |
| **Not Built Yet** | **Mainnet Deployment** | Production tokenomics, real fiat collateral, and third-party security audits. |

---

## 🔒 Acoustic Privacy Statement

**Audio never leaves your device.** 

EchoGrid operates with a strict zero-knowledge acoustic privacy policy:
- Audio captured via the smartphone microphone is processed entirely in volatile memory within client Web Workers using standard Web Audio API interfaces.
- No raw audio waveforms, PCM audio buffers, spectrogram images, or audio recordings are ever written to disk, buffered in cookies/local storage, or transmitted across the network.
- Outgoing proof payloads (`Proof of Power-Down v1`) consist exclusively of scalar mathematical features: band signal-to-noise ratios, timestamp delta measurements, detector version hashes, and cryptographic challenge nonces.

---

## 🚀 Quick Start Guide

### 1. Web App PWA (Mock / Local Mode)

Run the client application locally in mock mode without needing a microphone or external backend:

```bash
# 1. Install dependencies
cd app
npm ci

# 2. Start Vite development server
npm run dev

# 3. Open browser at http://localhost:5173
# In the Monitor tab, toggle "Simulator Mode" to run synthetic hum drop-off claims.
```

### 2. Rust Oracle Service (Local Development)

```bash
# 1. Run all workspace tests (75 offline unit & security tests)
cargo test --workspace

# 2. Launch oracle service in local mock mode
cargo run -p echogrid-oracle
```

---

## 📡 API Quickstart

### 1. Request a Challenge Nonce

```bash
curl -s "https://echogrid-oracle.fly.dev/api/v1/challenge?wallet=GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT"
```

**Success Response (HTTP 200):**
```json
{
  "challenge_id": "ch_5f8a1b2c3d4e",
  "nonce": "a1b2c3d4e5f60718293a4b5c6d7e8f90",
  "expires_at": 1730352000,
  "ttl_seconds": 120
}
```

### 2. Submit Proof-of-Power-Down

```bash
curl -s -X POST "https://echogrid-oracle.fly.dev/api/v1/proof" \
  -H "Content-Type: application/json" \
  -d '{
    "wallet_address": "GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT",
    "challenge_id": "ch_5f8a1b2c3d4e",
    "simulation": true,
    "proof": {
      "version": "v1",
      "baseline_energy_db": -24.5,
      "event_energy_db": -55.2,
      "energy_drop_db": 30.7,
      "delta_ms": 1200,
      "pre_drop_snr": 18.4,
      "post_drop_snr": 1.2,
      "detector_version": "0.1.0"
    }
  }'
```

**Success Response (HTTP 200):**
```json
{
  "claim_id": "claim_9b8c7d6e5f4a",
  "status": "confirmed",
  "bounty_stroops": 10000000,
  "tx_hash": "4e8e6154ba3cae2c8e815b7067750cdb4178ab9ba70ad57aa834846185936506",
  "explorer_url": "https://stellar.expert/explorer/testnet/tx/4e8e6154ba3cae2c8e815b7067750cdb4178ab9ba70ad57aa834846185936506",
  "simulated": true
}
```

**Cooldown Active Error (HTTP 429):**
```json
{
  "error": "COOLDOWN_ACTIVE",
  "message": "Wallet must wait 600 seconds between successive claims",
  "retry_after_seconds": 450
}
```

---

## 🔬 Research & DSP Reproducibility

Phase 0 acoustic hum research and Python DSP model prototypes are located under `research/hum_detection`.

### Reproducing Acoustic Research Tests

```bash
# 1. Navigate to research workspace
cd research/hum_detection

# 2. Run test suite using uv or the pre-configured virtual environment
uv run pytest
# Alternatively with Python:
# python -m pytest tests/
```

Expected output: `22 passed` verifying SciPy STFT, band SNR ratios, streaming detector thresholds, and golden parity vectors against TypeScript ports.

---

## 🗺️ Roadmap & Milestone Status

| Phase | Milestone | Description | Status |
| :--- | :--- | :--- | :---: |
| **Phase 0** | **Research Spike** | Acoustic STFT research, golden vectors, Soroban RPC client spike | `COMPLETE` |
| **Phase 1** | **Smart Contract** | Hardened `negawatt_escrow` Soroban contract on Stellar Testnet | `COMPLETE` |
| **Phase 2** | **Live Oracle** | Rust Axum daemon, SQLite WAL, cooldowns, caps, and Fly.io deployment | `COMPLETE` |
| **Phase 3** | **Mobile PWA** | Client PWA, Web Audio DSP, Freighter/demo wallet, Vercel hosting | `COMPLETE` |
| **Phase 4** | **Ship & Polish** | Unified CI, uptime monitor, trust disclosures, demo video, README rewrite | `IN PROGRESS` |
| **Phase 5** | **Post-MVP** | Hardware watermarking, coarse geofencing, multi-oracle consensus, Mainnet | `PLANNED` |

---

## 📜 License & Contributions

- Dual-licensed under [MIT](LICENSE) or [Apache 2.0](LICENSE).
- Contributions follow [Conventional Commits](https://www.conventionalcommits.org) and require all unified CI checks (`ci-ok`) to pass.
