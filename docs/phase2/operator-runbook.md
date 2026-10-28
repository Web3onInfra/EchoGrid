# EchoGrid Oracle — Operator Runbook & Deployment Guide

> **Core Axiom:** The oracle **cannot** prove a physical power-down occurred. Its sole architectural responsibility is to bound abuse, enforce economic caps and cooldowns, verify cryptographic proofs, and settle payouts safely to the Stellar Testnet smart contract.

---

## 1. First-Time Setup & Deployment

### 1.1 Prerequisites

- Fly.io CLI (`flyctl`) authenticated with operator credentials (`fly auth login`).
- Deployed Soroban `negawatt_escrow` contract on Stellar Testnet.
- Oracle hot wallet funded with testnet XLM for transaction fees.

### 1.2 Step-by-Step Provisioning

```bash
# 1. Create Fly.io application
fly apps create echogrid-oracle

# 2. Allocate persistent volume for SQLite in WAL mode
fly volumes create echogrid_data --region iad --size 1

# 3. Store oracle private signing key in secret store (NEVER commit to git)
fly secrets set ORACLE_SECRET_KEY="SXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"

# 4. Deploy application container
fly deploy

# 5. Verify service health and readiness
curl -f https://echogrid-oracle.fly.dev/api/v1/health
curl -f https://echogrid-oracle.fly.dev/api/v1/ready
```

---

## 2. Key Rotation Procedure

To rotate the oracle hot wallet signing key:

1. Generate a new Stellar Ed25519 keypair (`G_NEW...`, `S_NEW...`).
2. Fund `G_NEW...` with at least 5 testnet XLM for transaction sequence fees.
3. Update contract configuration on-chain: municipality admin invokes `set_oracle(G_NEW...)`.
4. Update the platform secret store:
   ```bash
   fly secrets set ORACLE_SECRET_KEY="S_NEW..."
   ```
5. The container restarts automatically. The startup self-check (`GET /api/v1/ready`) verifies the loaded public key matches `get_config().oracle_address`. If mismatched, it will return HTTP 503 and log a descriptive error.

---

## 3. Contract Upgrade & Pause Response

### Responding to `CONTRACT_PAUSED` (HTTP 503)

When municipality administrators pause the escrow contract on-chain:

- The oracle detects the pause status via periodic views or transaction simulation (`Error(Contract, #14)`).
- Incoming proof requests fail fast with HTTP 503 (`CONTRACT_PAUSED`).
- Reserved claim limits are preserved or safely refunded; no stuck sequence numbers occur.
- When unpaused by admin, the oracle resumes automatic settlement without requiring service restarts.

### Updating Contract Addresses

If a new contract or token is deployed:

1. Update `CONTRACT_ID` and `TOKEN_CONTRACT_ID` in `fly.toml` or via `fly env set`:
   ```bash
   fly env set CONTRACT_ID="CNEW..." TOKEN_CONTRACT_ID="CNEWTOKEN..."
   ```
2. Verify startup self-check logs: `fly logs`.

---

## 4. Handling Stuck Transactions & Sequence Gaps

The oracle employs serialized single-writer queue submission and crash reconciliation:

- **Crash Recovery:** On process restart, `reconcile_inflight_claims()` queries Soroban RPC `getTransaction` for all `Submitted` claims.
  - If transaction confirmed on-chain: transitions claim to `Confirmed`.
  - If transaction failed or not found past timeout: marks claim `Failed` and releases reserved daily cap.
- **Manual Intervention:** If an on-chain ledger delay or RPC sequence desync occurs:
  ```bash
  # Restart the oracle process to trigger startup reconciliation
  fly apps restart echogrid-oracle
  ```

---

## 5. Metrics & Observability

Metrics are bound to loopback only (`127.0.0.1:8080/internal/metrics`) and cannot be scraped publicly.

### Inspecting Live Metrics via Fly SSH

```bash
fly ssh console -C "curl -s http://127.0.0.1:8080/internal/metrics"
```

### Key Metrics Tracked

- `claims_total`: Total number of proof claims processed.
- `payouts_confirmed_total`: Successfully settled transactions on Stellar Testnet.
- `payouts_simulated_total`: Simulated claims processed without chain interaction.
- `rejections_by_reason`: Count of rejections labeled by code (`COOLDOWN_ACTIVE`, `GLOBAL_CAP_EXCEEDED`, etc.).
- `low_balance_warning`: Set to `1` when oracle fee balance falls below 5 XLM.

---

## 6. Volume Recovery & Disaster Recovery

### Creating Volume Snapshots

```bash
fly volumes list
fly volumes snapshots create <volume-id>
```

### Restoring SQLite Database

```bash
# Create replacement volume from existing snapshot
fly volumes create echogrid_data --snapshot-id <snapshot-id> --size 1

# Validate database file integrity within container
fly ssh console -C "sqlite3 /data/echogrid.db 'PRAGMA integrity_check;'"
```
