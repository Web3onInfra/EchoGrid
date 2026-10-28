# EchoGrid Oracle — Phase 2 Security Findings & Verification Report

## Overview
This report records the findings, threat model verifications, and stress results conducted during the implementation of the Phase 2 security and abuse suite (`oracle/src/security_test.rs`).

## Attack Vectors Tested & Invariants Verified

### 1. Same-Wallet Hammering (Cooldown & Daily Claim Limits)
- **Threat:** Malicious client attempts rapid sequential claims to drain oracle reserves for a single wallet.
- **Verification:**
  - Claim 1 succeeds (`200 OK`).
  - Immediate subsequent claim within the 600-second cooldown window is rejected with `422 Unprocessable Entity` (`COOLDOWN_ACTIVE`).
  - Advancing time past cooldown allows Claim 2.
  - Third claim within the 24-hour window is rejected with `422 Unprocessable Entity` (`WALLET_DAILY_LIMIT_EXCEEDED`).
- **Result:** Pass. Oracle bounds limits strictly tighter than the smart contract (Oracle: 2 claims/day; Contract: 3 claims/day).

### 2. Challenge Nonce Reuse
- **Threat:** Client attempts replay attacks by reusing an already-consumed challenge ID.
- **Verification:**
  - First submission marks challenge `used = true` and succeeds (`200 OK`).
  - Re-submission of the identical challenge returns `409 Conflict` (`CHALLENGE_ALREADY_USED`).
- **Result:** Pass. Single-use challenge state transition is strictly atomic.

### 3. Cross-Challenge Proof Replay Protection
- **Threat:** Client obtains a new challenge and submits an identical proof hash from a previous power-down event.
- **Verification:**
  - Duplicate proof hash lookup fails with `409 Conflict` (`REPLAY_PROOF`).
- **Result:** Pass. Every proof hash is uniquely recorded and rejected upon re-submission.

### 4. Parallel Race Conditions & Budget Exhaustion
- **Threat:** 15 concurrent clients simultaneously submit claims when remaining daily budget only allows 10 claims.
- **Verification:**
  - Spawning 15 parallel tokio tasks resulted in exactly 10 claims confirmed and exactly 5 claims rejected (`GLOBAL_CAP_EXCEEDED`).
  - Total stroops disbursed equaled exactly 100,000,000 stroops (10 XLM), never exceeding the cap.
- **Result:** Pass. Store transaction atomicity prevents race-condition overspend.

### 5. Malformed Proofs & Parser Fuzzing
- **Threat:** Corrupted JSON, negative numbers, buffer overflows, or invalid cryptographic hashes crashing the daemon.
- **Verification:**
  - 14 fuzz payloads (invalid JSON, wrong field types, invalid hex, out-of-bounds timestamps, zero/oversized durations) were submitted.
  - Zero panics or hangs observed; all malformed inputs cleanly returned client errors (`400 Bad Request` or `422 Unprocessable Entity`).
- **Result:** Pass. Strong typing and validation bounds safeguard daemon stability.

## Logged for PR 30 (Exit Report)
- No critical vulnerabilities or state leakage found.
- In-memory and SQLite storage primitives both maintain strict atomicity under concurrency.

