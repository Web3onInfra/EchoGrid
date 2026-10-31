---
title: "EchoGrid Trust & Honesty Model"
canonical_targets:
  - README.md
  - app/src/components/AboutModal.tsx
  - docs/demo/captions.srt
  - docs/release-notes-v0.1.0-mvp.md
---

# EchoGrid Canonical Trust Model

## 1. Canonical Honest-Framing Statement

The block below is the single source of truth for the honest framing of the EchoGrid MVP. All public surfaces (README, app disclosure sheets, demo video narration/captions, and release notes) reuse this canonical text verbatim. Automated CI checks enforce that no target surface drifts from this exact wording.

<!-- CANONICAL_TRUST_STATEMENT_START -->
The EchoGrid MVP trusts the client's claim of an appliance power-down. While cryptographic challenge nonces, recipient cooldowns, and on-chain spending caps strictly limit potential abuse, they do not prove that an appliance physically turned off. This live demonstration showcases the end-to-end micropayment loop and real-time acoustic detection user experience on Stellar Testnet; physical and cryptographic verification remains post-MVP work.
<!-- CANONICAL_TRUST_STATEMENT_END -->

---

## 2. Real / Mocked / Not Built Yet Inventory

| Category | Component / Feature | Current Status & Implementation Details |
| :--- | :--- | :--- |
| **Real** | **Soroban Escrow on Testnet** | Hardened smart contract (`negawatt_escrow`) deployed to Stellar Testnet ([`CDFOK4NF4CY...`](https://stellar.expert/explorer/testnet/contract/CDFOK4NF4CY27N4G5PYA6SXTJA5MEKPVU7DQRMJUS74LIXHOKYNRHOPF)). Fully manages token balances, auth verification, rollover windows, and per-claim budgets. |
| **Real** | **Live Rust Oracle Daemon** | Axum-based microservice deployed on Fly.io ([`https://echogrid-oracle.fly.dev`](https://echogrid-oracle.fly.dev)) backed by SQLite WAL storage, issuing time-bound challenges and submitting serialized on-chain payouts. |
| **Real** | **Testnet Payouts** | Automated token settlement transfers from the escrow contract to user wallets on Stellar Testnet upon valid proof reception. |
| **Real** | **Microphone Capture & DSP Detector** | Browser Web Audio API captures acoustic input; an in-memory Web Worker runs Cooley-Tukey Radix-2 FFT and band-energy DSP to detect drops in mains hum (50/60 Hz harmonics). |
| **Mocked** | **Grid Strain Data** | Grid event indicators (Green / Amber / Red / Emergency) are simulated client-side (`GridView.tsx`) for interactive demonstration, without live utility SCADA integration. |
| **Mocked** | **Simulator-Mode Claims** | A toggleable demo simulator bypasses live acoustic sensing, injecting synthetic hum waveforms and generating mock-flagged claims per the Phase 2 oracle policy (`simulation: true`). |
| **Not Built Yet** | **Hardware / Watermark Verification** | Cryptographic acoustic watermarking, acoustic reflections, or smart-plug hardware attestation to prove physical appliance shutdown. |
| **Not Built Yet** | **Coarse Geofencing** | IP/cell-tower or GPS attestation verifying claimant proximity to the strained grid zone. |
| **Not Built Yet** | **Native On-Device ML** | Quantized Edge ML models (TensorFlow Lite / ONNX) running natively for multi-appliance acoustic fingerprinting. |
| **Not Built Yet** | **Decentralized Oracles** | Multi-oracle quorum, threshold signatures, or staking/slashing mechanisms. |
| **Not Built Yet** | **Utility SCADA / OpenADR** | Automated OpenADR 2.0b or municipal utility telemetry dispatch integration. |
| **Not Built Yet** | **Mainnet Deployment** | Production tokenomics, real fiat/USDC settlement, and audits required for public Mainnet release. |

---

## 3. Privacy Statement

**Audio never leaves your device.** 

EchoGrid operates with a strict zero-knowledge acoustic privacy policy:
- Audio captured via the smartphone microphone is processed entirely in volatile memory within client Web Workers using standard Web Audio API interfaces.
- No raw audio waveforms, PCM audio buffers, spectrogram images, or audio recordings are ever written to disk, buffered in cookies/local storage, or transmitted across the network.
- Outgoing proof payloads (`Proof of Power-Down v1`) consist exclusively of scalar mathematical features: band signal-to-noise ratios, timestamp delta measurements, detector version hashes, and cryptographic challenge nonces.
