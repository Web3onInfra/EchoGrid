# ADR 001: Soroban RPC Client Architecture & Interaction Model

- **Status:** Accepted
- **Date:** 2026-10-28
- **Context:** EchoGrid Phase 0 (Track B: Stellar Testnet Soroban Payout Spike)

---

## 1. Context & Problem Statement

EchoGrid requires an off-chain oracle service (written in Rust / Axum) that listens for verified acoustic power-down claims, evaluates Proof-of-Power-Down proofs, and initiates on-chain bounty payouts to users via the `negawatt_escrow` smart contract on Stellar Testnet.

To execute payouts on Stellar/Soroban from Rust, the oracle must:
1. Query node health and ledger status.
2. Build and simulate the contract invocation (`payout` entrypoint with `proof_hash` and `recipient`).
3. Handle auth entries, resource footprints, and fee estimation.
4. Sign transactions using the oracle's Ed25519 keypair.
5. Submit the signed transaction envelope via Soroban RPC.
6. Poll transaction status (`getTransaction`) until finalized on a ledger.

We evaluated four architecture options for the Rust client interaction.

---

## 2. Decision Drivers

- **Reliability:** Zero silent transaction serialization/footprint failures.
- **Protocol Currency:** Seamless compatibility with Soroban Protocol 22+ on Testnet.
- **Maintainability:** Avoiding thousands of lines of fragile manual XDR packing and byte-level manipulation.
- **Observability:** Explicit transaction hashes, error codes, and explorer links for monitoring.

---

## 3. Considered Options

### Option A: Official Contract SDK (`soroban-sdk`)
- **Description:** Attempting to build and submit client transactions using `soroban-sdk`.
- **Finding:** `soroban-sdk` is explicitly scoped for on-chain WebAssembly contract execution (`#![no_std]`). It lacks off-chain transaction envelope builders, RPC transport clients, and keystore management for native host binaries.

### Option B: Community / Legacy Rust SDKs (`stellar-sdk`, `stellar-client`)
- **Description:** Using existing community Rust SDKs from crates.io.
- **Finding:** Most available crates are either unmaintained, pinned to legacy Horizon REST endpoints (pre-Soroban), or do not implement Soroban RPC Protocol 20+ simulation and footprint resolution.

### Option C: Pure Raw JSON-RPC + Handcrafted XDR (`stellar-xdr` + `ed25519-dalek`)
- **Description:** Hand-crafting `InvokeHostFunctionOp` XDR structs, posting to `simulateTransaction`, parsing returned footprint/auth data, assembling `TransactionEnvelope`, signing with Ed25519, and posting to `sendTransaction`.
- **Finding:** While possible, this requires ~1,500+ lines of low-level XDR encoding, is highly susceptible to protocol specification drifts between Soroban releases, and requires manual sequence number fetching and fee calculation.

### Option D: Hybrid Stellar Engine Delegation + Direct Soroban JSON-RPC (Selected)
- **Description:**
  - **Transaction Construction & Signing:** Delegate invocation building, resource simulation, auth entry signing, and network broadcast to the official SDF `stellar` CLI tool.
  - **RPC Monitoring & Ledger Queries:** Implement a lightweight, asynchronous JSON-RPC client (`reqwest` + `tokio`) in Rust to query `getHealth`, `getLatestLedger`, and poll `getTransaction` directly from the Soroban RPC node.
- **Finding:** Provides the highest stability and zero protocol drift risk while retaining full programmatic control, async polling, and error taxonomy mapping in the Rust service.

---

## 4. Decision

We chose **Option D (Hybrid Delegation & Direct JSON-RPC)** for Phase 0 and the initial Phase 1 oracle integration.

1. **RPC Client (`spikes/soroban_rpc`):**
   - Connects to `https://soroban-testnet.stellar.org`.
   - Sends typed JSON-RPC 2.0 requests with custom `User-Agent: EchoGrid-Spike/0.1`.
   - Implements `getHealth`, `getLatestLedger`, `getTransaction`, and `poll_transaction`.
2. **Transaction Dispatch:**
   - Invokes the `stellar contract invoke` engine with `--source-account`, `--network testnet`, and explicit contract arguments.
   - Extracts the transaction hash and passes it to `poll_transaction` for ledger confirmation.

---

## 5. Consequences & Roadmap

- **Positive:**
  - Payout transactions execute reliably end-to-end on testnet with verified on-chain SAC transfers.
  - Transaction hashes are validated via JSON-RPC polling and link directly to StellarExpert.
  - Zero dependencies on unmaintained third-party SDK crates.
- **Negative / Risks:**
  - Requires `stellar` CLI binary to be present in the host environment or Docker container.
- **Phase 1 Production Strategy:**
  - Package the `stellar` CLI in the oracle container image.
  - Encapsulate the invocation in a dedicated Rust worker queue with mutex-locked sequence management to support concurrent claims.
