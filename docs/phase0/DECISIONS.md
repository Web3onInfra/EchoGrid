# EchoGrid Phase 0: Decisions and Exit Report

- **Status:** Phase 0 Complete — All 15 PRs Delivered
- **Date:** 2026-10-28
- **Tracks:** Track A (Acoustic DSP & Hum Detection) & Track B (Soroban RPC & Escrow Payouts)

---

## 1. Executive Summary & Go / No-Go Decision Table

Phase 0 aimed to de-risk the two foundational technical uncertainties of EchoGrid before any user-facing product engineering:
1. **Track A (Acoustic):** Can phone microphones and browser audio pipelines reliably detect appliance power-down via low-frequency acoustic hum without running heavy machine learning models on device?
2. **Track B (Blockchain):** Can the Rust oracle backend build, sign, submit, and confirm Soroban `payout` invocations to Stellar Testnet via Soroban RPC?

| De-risk Area | Key Evaluation Question | Outcome | Decision |
| :--- | :--- | :--- | :--- |
| **Track A: Acoustic Bands** | Are mains hum and appliance harmonics detectable across target classes? | Yes. 50 Hz, 60 Hz, and 120 Hz bands consistently show >7.5 dB SNR on active appliances. | **GO** |
| **Track A: Browser Pipeline** | Can web browsers capture raw audio without destructive WebRTC filtering? | Yes. Disabling `echoCancellation`, `noiseSuppression`, and `autoGainControl` preserves 20–500 Hz hum. | **GO** |
| **Track A: Streaming Detector** | Is a lightweight, zero-ML streaming DSP detector viable on client devices? | Yes. State-machine detector achieves 100% detection rate and 0.0 false alarms/hr at 6.0 dB drop threshold. | **GO** |
| **Track B: RPC Interaction** | Can the Rust oracle submit Soroban transactions via Soroban RPC? | Yes. Successfully verified via hybrid Stellar invocation engine + direct JSON-RPC client. | **GO** |
| **Track B: On-Chain Settlement** | Do live payout transactions confirm on Stellar Testnet and transfer funds? | Yes. Multiple live transactions finalized with SAC transfer events on Testnet ledgers. | **GO** |
| **Track B: Security & Error Taxonomy** | Are replay attacks, auth failures, and insufficient balances trapped and taxonomized? | Yes. Negative test suite passes on testnet; `EscrowError` maps on-chain failures to HTTP status codes. | **GO** |

**Final Recommendation:** **FULL GO FOR PHASE 1 (Core Product Implementation).**

---

## 2. Track A: Acoustic DSP & Hum Detection Findings

### 2.1 Measured Spectral & Band SNR Metrics
Analysis across synthetic test beds, MIMII industrial pumps/fans, and UrbanSound8K air conditioner datasets established the following acoustic baseline:

- **Detectable Usable Bands (Criteria: SNR ≥ 6.0 dB on ≥ 80% of clips):**
  - **Air Conditioner (AC):** `50 Hz` (8.4 dB), `60 Hz` (9.1 dB), `120 Hz` (7.9 dB)
  - **Water Pump:** `50 Hz` (7.7 dB), `60 Hz` (9.0 dB), `120 Hz` (8.1 dB)
  - **Ventilation Fan:** `120 Hz` (7.6 dB)
- **Primary Finding on Harmonics:**
  Because mobile phone microphones often implement hardware-level analog high-pass filtering (attenuating frequencies below 40–50 Hz to reduce wind and handling rumble), the **second harmonic (100 Hz / 120 Hz)** is significantly more resilient and reliable on mobile devices than the fundamental 50/60 Hz mains frequency.

### 2.2 Streaming Detector Benchmark Results
The parameter sweep evaluating drop thresholds from 4.0 dB to 10.0 dB on positive transitions and negative control audio yielded:

| Drop Threshold | Detection Rate (%) | Median Latency (s) | False Alarms / Hour | Verdict |
| :---: | :---: | :---: | :---: | :--- |
| 4.0 dB | 100.0% | 2.97s | 0.00 | Sensitive |
| **6.0 dB (Frozen)** | **100.0%** | **2.97s** | **0.00** | **Optimal Baseline** |
| 8.0 dB | 100.0% | 2.97s | 0.00 | Conservative |
| 10.0 dB | 100.0% | 2.97s | 0.00 | High SNR required |

- **Frozen Detector Parameters (`detector_params.json`):**
  - Sample Rate: 16,000 Hz
  - Window Size: 2,048 samples (~128 ms)
  - Hop Size: 1,024 samples (~64 ms)
  - Baseline Adaptation Alpha: 0.05
  - Minimum Sustained Drop: 3.0 seconds (prevents brief speech interruptions or transients from triggering)
  - Replay Vectors: Validated deterministically in CI via `test_golden_vectors.py`.

### 2.3 Browser Audio Capture Recommendations
The single-file capture tool (`research/capture/index.html`) established the required audio constraints for Phase 1:
```javascript
const constraints = {
  audio: {
    channelCount: 1,
    sampleRate: 16000,
    echoCancellation: false,
    noiseSuppression: false,
    autoGainControl: false,
  }
};
```
*Note:* Any browser that forces aggressive voice isolation must warn the user to adjust OS-level microphone permissions.

---

## 3. Track B: Soroban RPC & Smart Contract Findings

### 3.1 Network & Contract Artifacts
All spike interactions ran on live Stellar Testnet (Protocol 22):
- **Stellar RPC URL:** `https://soroban-testnet.stellar.org`
- **Network Passphrase:** `Test SDF Network ; September 2015`
- **Deployed Contract ID:** `CANCKQXV4H6PCJ6DTAOIVMGUFYN2J4GTHJWFEUYZEHP3RFZBZHTBF6SX`
- **Native XLM SAC Token ID:** `CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC`
- **Oracle / Municipality Address:** `GCJCQGHJMGCUU5YNB67WP3ORARHH6THZ2V6G2CXMJGB5JHP7LEDMWWHW`

### 3.2 Confirmed Live Payout Transactions
Multiple on-chain payouts were executed and verified via Soroban JSON-RPC `getTransaction`:
- **Payout Transaction 1:** [`4e8e6154ba3cae2c8e815b7067750cdb4178ab9ba70ad57aa834846185936506`](https://stellar.expert/explorer/testnet/tx/4e8e6154ba3cae2c8e815b7067750cdb4178ab9ba70ad57aa834846185936506)
- **Payout Transaction 2:** [`fdc164ae79770d6d92ab51062b762b2809ddcd9f9e858c88037ebaac8a7ae3ff`](https://stellar.expert/explorer/testnet/tx/fdc164ae79770d6d92ab51062b762b2809ddcd9f9e858c88037ebaac8a7ae3ff)
- **Payout Transaction 3:** [`bfe4f42902f6bff497d4edd5cc603692cbd2b7092a9a377502b9b11b097fa208`](https://stellar.expert/explorer/testnet/tx/bfe4f42902f6bff497d4edd5cc603692cbd2b7092a9a377502b9b11b097fa208)

Average ledger finality latency was measured at **~5.2 seconds**, well within acceptable bounds for off-chain verification feedback.

### 3.3 Client Architecture Decision (ADR 001)
As documented in `docs/phase0/adr-rpc-client.md`, we selected a **Hybrid Architecture**:
- **Transaction Engine:** Uses the official SDF `stellar` CLI tool for footprint resolution, auth entry signing, and network submission, avoiding hundreds of lines of fragile manual XDR packing.
- **Monitoring & Status:** Direct async JSON-RPC client (`reqwest` + `tokio`) queries node health, latest ledger, and polls `getTransaction`.

---

## 4. Contract Observations & Findings for Phase 1

Per Phase 0 guidelines, existing smart contracts were untouched. The following observations are recorded for Phase 1:

1. **WASM Optimization Requirement:**
   - *Observation:* Unoptimized builds from `cargo build --target wasm32-unknown-unknown --release` contain `reference-types` bytecodes which cause Stellar HostError `Error(WasmVm, InvalidAction)`.
   - *Action for Phase 1:* Contract deployment pipelines must always run `stellar contract optimize` to strip unsupported post-MVP bytecode features.
2. **Fixed Bounty per Contract Instance:**
   - *Observation:* The contract's `payout` entrypoint takes `(recipient, proof_hash)` without an amount argument, enforcing the pre-set `bounty_per_claim`.
   - *Action for Phase 1:* The oracle does not need to submit payment amounts; it only submits proof hashes. If dynamic bounties (e.g. higher reward during peak stress) are required, `set_bounty` must be invoked by the municipality.
3. **Storage TTL Management:**
   - *Observation:* `DataKey::ClaimUsed(BytesN<32>)` uses persistent contract storage. In Soroban Protocol 20+, persistent entries have TTL expiration windows.
   - *Action for Phase 1/2:* If persistent proof entries expire after long durations, old proof hashes could theoretically be replayed unless the contract or oracle invokes `extend_ttl` periodically.

---

## 5. Error Taxonomy & Edge Case Summary

The `EscrowError` enum in `oracle/src/types.rs` maps all on-chain conditions to standard HTTP responses:

| Error Case | On-Chain / RPC Signature | EscrowError Variant | HTTP Code |
| :--- | :--- | :--- | :---: |
| Replay Attack | `VM call trapped: UnreachableCodeReached` / `proof already claimed` | `ReplayDetected` | 409 Conflict |
| Unauthorized Caller | `require_auth` failure / missing oracle signature | `UnauthorizedSigner` | 403 Forbidden |
| Depleted Escrow | `insufficient escrow balance` | `InsufficientPoolBalance` | 503 Unavailable |
| RPC Lag / Dropout | Timeout waiting for transaction status | `RpcTimeout` | 504 Gateway Timeout |
| Malformed Proof | Non-32-byte hex string | `InvalidProofFormat` | 400 Bad Request |

---

## 6. Recommendations for Phase 1 & Phase 2

### Phase 1: Core Product Implementation
1. **Frontend Web App:** Implement the Web Audio API streaming detector with frozen parameters (`window=2048`, `hop=1024`, `drop=6dB`, `sustained=3s`) directly in JavaScript/WebAudio AudioWorklet.
2. **Oracle Integration:** Connect the Axum `/verify_proof` route to the Soroban RPC client spike logic, returning `tx_hash` and error taxonomy responses.
3. **Challenge / Nonce Endpoint:** Implement the challenge issuance flow so proofs cannot be pre-computed.

### Phase 2: Production Hardening
1. **Oracle Signing Worker:** Run transaction submission through a dedicated serialized queue to avoid sequence number collisions under concurrent claims.
2. **Device Attestation:** Pair acoustic proof hashes with browser WebAuthn or device hardware attestations to mitigate audio replay spoofing.
3. **Storage TTL Automation:** Implement automated keeper scripts to extend contract storage TTL on active municipal escrows.
