# ⚡ EchoGrid — Decentralized NegaWatt Marketplace

[![Stellar](https://img.shields.io/badge/Stellar-Testnet-blue.svg)](https://stellar.org)
[![Soroban](https://img.shields.io/badge/Soroban-Smart_Contracts-purple.svg)](https://soroban.stellar.org)
[![Rust](https://img.shields.io/badge/Rust-1.80+-orange.svg)](https://www.rust-lang.org)
[![React](https://img.shields.io/badge/React-18-61dafb.svg)](https://react.dev)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.5-blue.svg)](https://www.typescriptlang.org)
[![License](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-green.svg)](LICENSE)

> *"To balance the grid in Ghana, we don't need millions of dollars in imported smart meters. We just need to listen."*

EchoGrid replaces expensive IoT smart meters with the microphone on ubiquitous smartphones. Local Edge AI detects the distinct low-frequency acoustic signature of heavy appliances (air conditioners, water pumps, refrigeration compressors). When the grid is strained, a municipal authority funds a Soroban smart contract bounty on Stellar. Users are challenged to power down. If the local client detects the sudden absence of the appliance hum, it generates a cryptographic **Proof of Power-Down**, which a Rust oracle verifies before triggering an autonomous Stellar micropayment.

---

## 📑 Table of Contents

- [Live Deployments & Network Details](#-live-deployments--network-details)
- [Monorepo Architecture](#-monorepo-architecture)
- [System Architecture Flow](#-system-architecture-flow)
- [Privacy & Security Model](#-privacy--security-model)
- [Quick Start Guide](#-quick-start-guide)
  - [Prerequisites](#prerequisites)
  - [1. Soroban Smart Contract (Phase 1)](#1-soroban-smart-contract-phase-1)
  - [2. Rust Oracle Service (Phase 2)](#2-rust-oracle-service-phase-2)
  - [3. Mobile Web PWA Client (Phase 3)](#3-mobile-web-pwa-client-phase-3)
  - [4. Acoustic Hum Research & DSP (Phase 0)](#4-acoustic-hum-research--dsp-phase-0)
  - [5. Acoustic CLI Prototype](#5-acoustic-cli-prototype)
- [Testing & Quality Assurance](#-testing--quality-assurance)
- [Documentation & Runbooks](#-documentation--runbooks)
- [License](#-license)

---

## 🌐 Live Deployments & Network Details

EchoGrid is deployed and operational on the **Stellar Testnet**:

| Component | Target / Environment | URL / Identifier |
| :--- | :--- | :--- |
| **PWA Web Client** | Vercel Production | [https://echogrid-pwa.vercel.app](https://echogrid-pwa.vercel.app) |
| **Soroban Smart Contract** | Stellar Testnet | [`CDFOK4NF4CY27N4G5PYA6SXTJA5MEKPVU7DQRMJUS74LIXHOKYNRHOPF`](https://stellar.expert/explorer/testnet/contract/CDFOK4NF4CY27N4G5PYA6SXTJA5MEKPVU7DQRMJUS74LIXHOKYNRHOPF) |
| **Testnet Token (USDC)** | Stellar Testnet Asset | [`CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC`](https://stellar.expert/explorer/testnet/contract/CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC) |
| **Oracle Service** | Fly.io / Docker | REST API with Axum + SQLite WAL + Prometheus |
| **OpenAPI Specification** | Oracle Documentation | [docs/api/openapi.yaml](docs/api/openapi.yaml) |

Contract details, deployment transaction hashes, and public keys are tracked in [`deployments/testnet.json`](deployments/testnet.json).

---

## 📁 Monorepo Architecture

```
EchoGrid/
├── contracts/
│   └── negawatt_escrow/       # Phase 1: Soroban Smart Contract (Rust / no_std)
│       ├── src/lib.rs          # Escrow logic, AgentGuard RBAC, replay protection
│       └── Cargo.toml
├── oracle/                    # Phase 2: Rust Oracle & Settlement Daemon (Axum)
│   ├── src/                   # REST routes, Stellar Horizon client, submission queue
│   ├── Cargo.toml
│   └── .env.example
├── app/                       # Phase 3: Mobile PWA Client (React + Vite + TypeScript)
│   ├── src/                   # Web Audio DSP, streaming detector, wallet adapter
│   ├── scripts/smoke-test.js  # Automated production health and asset verifier
│   ├── vercel.json            # Strict CSP, security headers & SPA routing
│   ├── package.json
│   └── README.md              # Detailed PWA user and operator guide
├── research/                  # Phase 0: Acoustic Hum Detection & Research
│   └── hum_detection/         # Python DSP models, STFT analysis, golden vectors
├── acoustic_cli/              # Prototype: Standalone Edge AI CLI (Rust)
│   └── src/main.rs
├── deployments/               # Network addresses, contract IDs, and deployment hashes
│   └── testnet.json           # Live Stellar Testnet configuration
├── docs/                      # Architectural specs, phase exit reports & runbooks
│   ├── api/openapi.yaml       # Oracle OpenAPI v3 specification
│   ├── phase0/                # Recording protocols, ADRs, dataset decisions
│   ├── phase1/                # Smart contract design and interface specs
│   ├── phase2/                # Oracle design, security audit, operator runbook
│   └── phase3/                # PWA design, device matrix, Phase 3 Exit Report
├── justfile                   # Task runner for research and operational commands
├── fly.toml                   # Fly.io deployment manifest for Oracle
├── vercel.json                # Root Vercel deployment configuration
└── Cargo.toml                 # Rust workspace root (oracle + acoustic_cli)
```

---

## 🔄 System Architecture Flow

```
   +-------------------------------------------------------+
   |                  Mobile Smartphone                    |
   |                                                       |
   |  [Microphone]                                         |
   |        │                                              |
   |        ▼ (Web Audio API / RingBuffer - No Audio Sent) |
   |  [In-Browser Web Worker FFT]                          |
   |        │                                              |
   |        ▼ (50/60 Hz Mains Hum & Harmonic Drop Ratio)   |
   |  [StreamingPowerDownDetector]                         |
   |        │                                              |
   |        ▼ (appliance powered down during grid strain)  |
   |  [Proof Builder]                                      |
   +--------│----------------------------------------------+
            │
            │ HTTPS POST /api/v1/proof
            │ (Canonical JSON pre-image + challenge nonce, ~440 bytes)
            ▼
   +-------------------------------------------------------+
   |                EchoGrid Rust Oracle                   |
   |                                                       |
   |  1. Validate challenge window, nonce, & rate-limits   |
   |  2. Verify spectral drop latency and energy ratios    |
   |  3. Deduplicate SHA-256 pre-image hash (replay guard) |
   |  4. Enqueue atomic payout transaction                 |
   +--------│----------------------------------------------+
            │
            │ Ed25519 Signed Soroban Invocation (`payout`)
            ▼
   +-------------------------------------------------------+
   |            Soroban Negawatt Escrow Contract           |
   |                                                       |
   |  1. require_auth() — Stellar Ed25519 protocol check   |
   |  2. Replay check — writes proof hash to storage       |
   |  3. Rate check — verifies recipient & global quotas   |
   |  4. Releases USDC bounty from municipal escrow pool   |
   +--------│----------------------------------------------+
            │
            │ Autonomous Token Transfer
            ▼
   +───────────────────────────────────────────────────────+
   |              User Stellar Wallet Address              |
   |            (Freighter Extension / Keypair)            |
   +───────────────────────────────────────────────────────+
```

---

## 🔒 Privacy & Security Model

EchoGrid is designed around strict trust-minimization and privacy guarantees:

1. **Zero Raw Audio Buffers Sent**:
   - Audio captured via the browser's Web Audio API is processed strictly in-memory inside client Web Workers.
   - Proof payloads consist exclusively of scalar feature metrics, timing latencies, detector version hashes, and cryptographic nonces (~440 bytes).
   - Zero raw audio buffers or recordings are ever transmitted across the network or stored.
2. **Untrusted Client Architecture**:
   - The Oracle and Soroban smart contract treat all client inputs as untrusted.
   - Payout transactions are authenticated using protocol-level `Address.require_auth()` signed by the Oracle keypair.
3. **AgentGuard On-Chain RBAC**:
   - `ROLE_MUNICIPALITY`: Authorized to initialize escrow, deposit bounty liquidity, and update bounty parameters.
   - `ROLE_ORACLE`: Restricted exclusively to invoking the `payout` endpoint with valid proofs.
4. **Replay & Rate-Limiting Protection**:
   - Every submitted Proof-of-Power-Down hash is written to persistent contract storage. Re-submitting an existing hash panics on-chain.
   - Sliding-window rate limiters cap maximum payouts per user address and per hour to safeguard municipality funds.
5. **Strict Testnet Quarantine**:
   - Hard-coded checks verify the network passphrase (`Test SDF Network ; September 2015`). Payouts and testnet faucets are disabled if connected to mainnet.

---

## 🚀 Quick Start Guide

### Prerequisites

- **Rust**: `1.80+` with `wasm32-unknown-unknown` target:
  ```bash
  rustup target add wasm32-unknown-unknown
  ```
- **Soroban CLI**: `cargo install --locked soroban-cli`
- **Node.js**: `v20+` and `npm`
- **Python**: `3.11+` with `uv` (for Phase 0 research tools)

---

### 1. Soroban Smart Contract (Phase 1)

```bash
cd contracts/negawatt_escrow

# Run the in-process unit tests
cargo test

# Build WASM bytecode
cargo build --target wasm32-unknown-unknown --release

# Optimize binary size for deployment
soroban contract optimize --wasm target/wasm32-unknown-unknown/release/negawatt_escrow.wasm
```

---

### 2. Rust Oracle Service (Phase 2)

```bash
cd oracle

# Copy environment template
cp .env.example .env

# Run unit and integration tests
cargo test

# Start the local development server (binds to http://localhost:8080)
cargo run
```

Production deployment to Fly.io:
```bash
# Provision volume & deploy
fly volumes create echogrid_data --size 1
fly secrets set ORACLE_SECRET_KEY="S..."
just deploy-fly
```
For complete production operations, see the [Phase 2 Operator Runbook](docs/phase2/operator-runbook.md).

---

### 3. Mobile Web PWA Client (Phase 3)

```bash
cd app

# Install dependencies
npm install

# Start local Vite development server (http://localhost:5173)
npm run dev

# Run comprehensive test suite
npm run typecheck
npm run lint
npm test

# Build production bundle and inspect bundle budgets
npm run build
npm run size

# Run automated smoke test against local or remote origin
node scripts/smoke-test.js http://localhost:5173 --skip-oracle
```

For mobile home-screen installation instructions (iOS & Android) and the audio simulator walkthrough, see the [EchoGrid App Guide](app/README.md).

---

### 4. Acoustic Hum Research & DSP (Phase 0)

The research harness verifies frequency extraction algorithms, golden test vectors, and acoustic recordings of appliances:

```bash
# Run pytest on the signal processing suite
just test

# Generate spectral analysis reports
just report

# Run detector evaluation benchmark
just eval
```

---

### 5. Acoustic CLI Prototype

A standalone command-line prototype for running the detector directly against local hardware audio inputs:

```bash
cd acoustic_cli
cargo run -- --wallet GABC1234567890ABCDEF1234567890ABCDEF1234567890ABCDEF1234 --challenge-id ch_001
```

---

## 🧪 Testing & Quality Assurance

EchoGrid enforces multi-layer verification across every tier:

- **100% Golden Parity**: The TypeScript detector (`app/src/detector/detector.ts`) achieves identical classification outputs to the Phase 0 Python reference implementation against `research/hum_detection/golden_vectors.json`.
- **E2E Playwright Suite**: Chromium tests run with `--use-file-for-fake-audio-capture` to simulate real acoustic appliance power-down transitions in end-to-end tests.
- **Accessibility**: 100/100 automated axe-core audits and WCAG 2.1 AA compliance across all application views.
- **Production Smoke Verification**: `app/scripts/smoke-test.js` continuously validates live web assets, service worker manifests, CSP headers, and CORS preflight responses.

---

## 📚 Documentation & Runbooks

- **PWA Guide & Device Setup**: [`app/README.md`](app/README.md)
- **Phase 3 Exit Report**: [`docs/phase3/EXIT.md`](docs/phase3/EXIT.md)
- **Phase 3 Physical Device Matrix**: [`docs/phase3/device-test-matrix.md`](docs/phase3/device-test-matrix.md)
- **Phase 2 Operator Runbook**: [`docs/phase2/operator-runbook.md`](docs/phase2/operator-runbook.md)
- **Phase 2 Security Findings & Audit**: [`docs/phase2/security-findings.md`](docs/phase2/security-findings.md)
- **Oracle REST OpenAPI Spec**: [`docs/api/openapi.yaml`](docs/api/openapi.yaml)
- **Phase 1 Smart Contract Interface**: [`docs/phase1/contract-interface.md`](docs/phase1/contract-interface.md)
- **Phase 0 Acoustic Feasibility Report**: [`docs/phase0/README.md`](docs/phase0/README.md)

---

## 📄 License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE) or http://opensource.org/licenses/MIT)

at your option.
