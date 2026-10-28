# EchoGrid Phase 2: Live Oracle Architecture & Threat Model

- **Status:** Proposed (Under Review - Gate 1)
- **Target Network:** Stellar Testnet
- **Service Stack:** Rust, Axum, Tokio, SQLite (WAL mode)
- **Deployment Platform:** Fly.io (with persistent volume)

---

## 1. Executive Summary & Core Principles

The EchoGrid Oracle acts as an automated settlement gateway between edge acoustic detectors (smartphones/appliances) and the on-chain NegaWatt Escrow smart contract (`CDFOK4NF4CY27N4G5PYA6SXTJA5MEKPVU7DQRMJUS74LIXHOKYNRHOPF`).

### Fundamental Truth
> **The oracle cannot physically prove a power-down event occurred.**
> Acoustic sensor data produced on edge hardware cannot provide unforgeable physical guarantees against simulated audio playback or compromised devices. The oracle's mandate is therefore **abuse bounding, economic rate limiting, and safe transactional settlement**.

### Architectural Principles
1. **Oracle limits are strictly tighter than contract limits:** The Soroban smart contract is the absolute backstop, not the front line of defense.
2. **Two-phase reservation before submission:** Budget is reserved prior to submitting transactions to Soroban RPC, finalized on confirmation, and reconciled on startup.
3. **Strict secret isolation:** The oracle's private signing key exists exclusively in process memory and the platform secret store. It is never serialized, logged, or exposed in error messages.
4. **Offline and deterministic testability:** All chain interactions are decoupled via mockable RPC abstractions, and all time-based operations are parameterized via a testable `Clock` trait.

---

## 2. Default Limit Hierarchy

| Dimension | Oracle Operational Limit | Contract Backstop Limit | Enforcement Layer |
|---|---|---|---|
| **Challenge TTL** | `120 s` | N/A (Oracle ephemeral) | In-memory / SQLite |
| **Allowed Clock Skew** | `30 s` | N/A | Oracle validation |
| **Per-Wallet Cooldown** | `10 minutes` | None | Oracle state |
| **Per-Wallet Claims / Day** | `2 claims / day` | `3 claims / 24h window` | Oracle state + Contract temporary storage |
| **Global Daily Payout Cap** | `10 XLM` (`100,000,000` stroops) | `20 XLM` (`200,000,000` stroops) | Oracle state + Contract instance storage |
| **Per-Payout Bounty** | `1 XLM` (`10,000,000` stroops) | `2 XLM` cap (`20,000,000` stroops) | Contract configuration |
| **Per-IP Rate Limit** | `30 req / min` | N/A | Axum middleware |
| **Challenge Rate / Wallet**| `6 req / min` | N/A | Axum middleware |
| **Synchronous Wait Window**| `20 s` | RPC timeout (60 s) | Handler async polling |

---

## 3. Endpoints & API Contract

Base prefix: `/api/v1`

### 3.1. `GET /api/v1/health`
Liveness check for container orchestration and ping monitoring.
- **Status:** `200 OK`
```json
{
  "status": "ok",
  "service": "echogrid-oracle",
  "version": "0.1.0"
}
```

### 3.2. `GET /api/v1/ready`
Readiness check. Evaluates RPC reachability, oracle account balance, and contract pause state.
- **Status:** `200 OK` (healthy) or `503 Service Unavailable`
```json
{
  "status": "ready",
  "rpc_reachable": true,
  "contract_paused": false,
  "oracle_account_funded": true,
  "oracle_public_key": "GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT"
}
```

### 3.3. `GET /api/v1/version`
Build metadata and commit version.
- **Status:** `200 OK`
```json
{
  "version": "0.1.0",
  "git_commit": "abcdef0",
  "network": "testnet"
}
```

### 3.4. `GET /api/v1/config`
Public metadata for frontends and clients.
- **Status:** `200 OK`
```json
{
  "network": "testnet",
  "network_passphrase": "Test SDF Network ; September 2015",
  "contract_id": "CDFOK4NF4CY27N4G5PYA6SXTJA5MEKPVU7DQRMJUS74LIXHOKYNRHOPF",
  "token_id": "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
  "bounty_stroops": 10000000,
  "per_payout_cap_stroops": 20000000,
  "oracle_daily_cap_stroops": 100000000,
  "cooldown_seconds": 600,
  "challenge_ttl_seconds": 120,
  "simulator_mode": false,
  "explorer_base_url": "https://stellar.expert/explorer/testnet/tx"
}
```

### 3.5. `GET /api/v1/challenge?wallet={G...}`
Issues a wallet-bound single-use challenge nonce.
- **Query Params:** `wallet` (Valid Stellar StrKey G... address)
- **Response:** `200 OK`
```json
{
  "challenge_id": "ch_9f8b4c3e2a1d0f5e",
  "wallet_address": "GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT",
  "issued_at": "2026-10-28T12:00:00Z",
  "expires_at": "2026-10-28T12:02:00Z"
}
```

### 3.6. `POST /api/v1/proof`
Submit Proof-of-Power-Down for evaluation and on-chain settlement.
- **Request Body (Proof v1 Schema):**
```json
{
  "wallet_address": "GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT",
  "challenge_id": "ch_9f8b4c3e2a1d0f5e",
  "timestamp": "2026-10-28T12:01:00Z",
  "nonce": "a1b2c3d4e5f60718",
  "proof_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "detector_version": "echonet-1.2",
  "duration_seconds": 15
}
```
- **Responses:**
  - `200 OK` (Confirmed within 20 s):
  ```json
  {
    "status": "confirmed",
    "claim_id": "clm_1028_abc123",
    "tx_hash": "6edcbd9ad86343562aa13bc5e75ab57c649dd8abeb481761738f0609c2a63e81",
    "explorer_url": "https://stellar.expert/explorer/testnet/tx/6edcbd9ad86343562aa13bc5e75ab57c649dd8abeb481761738f0609c2a63e81",
    "amount_stroops": 10000000,
    "simulated": false
  }
  ```
  - `202 Accepted` (Submitted but not confirmed within 20 s):
  ```json
  {
    "status": "pending",
    "claim_id": "clm_1028_abc123",
    "poll_url": "/api/v1/claims/clm_1028_abc123"
  }
  ```
  - `400 Bad Request` / `429 Too Many Requests` / `503 Service Unavailable` on rejections.

### 3.7. `GET /api/v1/claims/{id}`
Polls status of asynchronous or slow payouts.
- **Response:** `200 OK`
```json
{
  "claim_id": "clm_1028_abc123",
  "status": "confirmed",
  "tx_hash": "6edcbd9ad86343562aa13bc5e75ab57c649dd8abeb481761738f0609c2a63e81",
  "explorer_url": "https://stellar.expert/explorer/testnet/tx/6edcbd9ad86343562aa13bc5e75ab57c649dd8abeb481761738f0609c2a63e81",
  "updated_at": "2026-10-28T12:01:25Z"
}
```

---

## 4. Proof v1 Schema & Hash Derivation

The Proof v1 canonical format guarantees that proofs are strictly bound to the user wallet, challenge session, and timestamp:
$$\text{pre\_image} = \text{wallet\_address} \parallel \text{"|"}\parallel \text{challenge\_id} \parallel \text{"|"} \parallel \text{timestamp} \parallel \text{"|"} \parallel \text{nonce}$$
$$\text{proof\_hash} = \text{SHA-256}(\text{pre\_image})$$

### Validation Rules
1. `wallet_address`: Must be a valid StrKey format starting with `G`, length 56, passing CRC16 checksum.
2. `challenge_id`: Must exist in active store, match `wallet_address`, and not be expired (`issued_at + 120s >= now`).
3. `timestamp`: Must be valid ISO-8601 UTC string within `now +/- 30s` (allowed clock skew).
4. `nonce`: Exactly 16 hexadecimal characters (8 random bytes).
5. `proof_hash`: Exactly 64 hexadecimal characters, equal to SHA-256 of pre-image.
6. `detector_version`: String matching allowed pattern (`^echonet-[0-9]+\.[0-9]+$`).
7. `duration_seconds`: Plausible window bound ($5 \le \text{duration} \le 300$ seconds).

---

## 5. Payout State Machine & Startup Reconciliation

```
[Received]
   │
   ▼
[Reserved] ──── (Limit exceeded) ───► [Rejected]
   │
   ▼
[Submitted] ─── (RPC failure) ──────► [Failed / Released]
   │
   ├── (Confirmed on ledger) ───────► [Confirmed / Finalized]
   └── (Timeout > 20s) ─────────────► [Pending Polling]
```

### Reconciliation On Boot
On server boot, all claims marked `Submitted` or `Pending` in SQLite are checked against Soroban RPC (`getTransaction`):
- If `SUCCESS`: finalize claim and record confirmed status.
- If `NOT_FOUND` and older than 5 minutes: mark `Failed` and release reserved daily budget.
- If `FAILED`: mark `Failed` and release reserved daily budget.

---

## 6. Error Catalog & Contract Taxonomy Mapping

Uniform JSON Error Schema:
```json
{
  "code": "REPLAY_CLAIM",
  "message": "This proof has already been processed and rewarded.",
  "status": 409,
  "details": null
}
```

| Contract Error Code & Variant | Oracle HTTP Status | Oracle Error Code | User-Facing Action |
|---|---|---|---|
| `1: AlreadyInitialized` | `500` | `INTERNAL_CONTRACT_ERROR` | Contact support |
| `2: NotInitialized` | `500` | `INTERNAL_CONTRACT_ERROR` | Contact support |
| `3: Unauthorized` | `403` | `ORACLE_UNAUTHORIZED` | Oracle key rotation required |
| `4: InvalidAmount` | `400` | `INVALID_PAYLOAD` | Review payload values |
| `5: InvalidBounty` | `500` | `INVALID_CONFIG` | Municipality contract config invalid |
| `6: InvalidCap` | `500` | `INVALID_CONFIG` | Municipality contract config invalid |
| `7: InvalidBudget` | `500` | `INVALID_CONFIG` | Municipality contract config invalid |
| `8: InvariantViolated` | `500` | `CONTRACT_INVARIANT_VIOLATED`| Municipality contract config invalid |
| `9: InsufficientBalance` | `503` | `POOL_DEPLETED` | Municipality treasury needs top-up |
| `10: ReplayClaim` | `409` | `REPLAY_CLAIM` | New power-down challenge required |
| `11: ExceedsPayoutCap` | `422` | `EXCEEDS_PAYOUT_CAP` | Bounty exceeds contract limit |
| `12: ExceedsWindowBudget` | `429` | `GLOBAL_CAP_EXCEEDED` | Grid budget exhausted; retry tomorrow |
| `13: ExceedsRecipientWindowLimit` | `429` | `WALLET_DAILY_LIMIT_EXCEEDED`| Max claims reached for 24h window |
| `14: ContractPaused` | `503` | `CONTRACT_PAUSED` | Payouts temporarily suspended by admin |

---

## 7. Storage, Scaling & Hosting Choices

### Persistence
- **Storage:** SQLite 3 in WAL (Write-Ahead Logging) mode, stored on a persistent volume mount (`/data/echogrid.db`).
- **Store Trait:** `Store` trait abstracting challenge issuance, claim reservations, limit checks, and reconciliation.
- **Scale-Out Boundary:** Single SQLite file accommodates up to ~1,000 requests/sec with serialized writes. The on-chain Soroban submission queue requires a single-writer sequence model anyway. Scaling horizontally would require migrating from SQLite to Postgres and managing distributed transaction sequencing.

### Hosting Platform: Fly.io
- **Reasoning:** First-class persistent volume support (`fly volumes`), global low-latency edge proxy, managed secrets, and simple container deployment.
- **Fallback:** Railway.app (if chosen by operator at Gate 1).

### Simulator Mode Policy
When `SIMULATOR_MODE=true` or client flags `simulated: true`:
- The challenge and proof validations run identically.
- No transaction envelope is constructed or submitted to Soroban RPC.
- Mock receipts with `simulated: true` and fake tx hash `SIM_TX_<hash>` are returned.
- Simulated payouts do not consume real on-chain budget or live wallet daily caps.

---

## 8. Threat Model & Maximum Loss Under Total Oracle Compromise

| Threat Vector | Mitigation | Residual Risk |
|---|---|---|
| **Edge audio spoofing / synthetic humming** | Short-lived challenge nonces (120 s), wallet binding, strict frequency bounds. | Attacker playing hum near phone microphone during challenge window. |
| **Proof replay attack** | On-chain persistent storage (`DataKey::ClaimUsed`) + Oracle single-use challenge consumption. | Zero. Replayed hash is rejected unconditionally. |
| **Sybil wallet spam** | Per-wallet cooldown (10 min) + per-wallet daily claims (2) + IP rate limiting. | Attacker with many wallets bounded by global daily cap. |
| **Concurrency sequence collisions** | In-process serialized submission queue with mutex-locked sequence increments. | Zero sequence collision errors. |
| **Total Oracle Secret Key Leak** | Strictly mathematically bounded on-chain by the smart contract. | **Max loss = 20 XLM per 24 hours**. Admin can pause contract or rotate oracle key. |

### Mathematical Bound on Worst-Case Compromise
Even if an adversary acquires the oracle's private Ed25519 signing key and submits transactions directly to Stellar Testnet bypassing the Axum service completely:
$$\text{Max Daily Drain} = \text{window\_budget} = 20\text{ XLM}$$
$$\text{Max Single Payout} = \text{per\_payout\_cap} = 2\text{ XLM}$$
$$\text{Max Recipient Drain} = 3 \times 1\text{ XLM} = 3\text{ XLM}$$
The municipality cold key (`GAQPI...`) retains unilateral power to invoke `pause()` or `set_oracle(new_key)` at any time.
