# EchoGrid Phase 1: Smart Contract Escrow — Completion & Exit Report

## 1. Executive Summary

Phase 1 ("Smart Contract Escrow") delivers the on-chain Soroban escrow contract (`negawatt_escrow`) on the Stellar Testnet. The contract acts as an automated municipal bounty clearinghouse: it escrows USDC tokens, validates authorized oracle payout triggers with cryptographically unique proof hashes, enforces daily spend budgets and per-recipient claim caps, supports administrative parameter updates and emergency pause functionality, and maintains state TTLs.

---

## 2. On-Chain Deployment Details (Stellar Testnet)

| Parameter | Value |
| :--- | :--- |
| **Contract ID** | [`CDFOK4NF4CY27N4G5PYA6SXTJA5MEKPVU7DQRMJUS74LIXHOKYNRHOPF`](https://stellar.expert/explorer/testnet/contract/CDFOK4NF4CY27N4G5PYA6SXTJA5MEKPVU7DQRMJUS74LIXHOKYNRHOPF) |
| **WASM Hash** | `68d41a5f281e3161e109acf08891b02ba292608f617bae7cb1e7926165c2868d` |
| **WASM Size** | **25,455 bytes** (budget: $\le$ 30,000 bytes) |
| **Deployed At** | `2026-10-28T05:58:40Z` |
| **Deploy Tx** | [`6edcbd9ad86343562aa13bc5e75ab57c649dd8abeb481761738f0609c2a63e81`](https://stellar.expert/explorer/testnet/tx/6edcbd9ad86343562aa13bc5e75ab57c649dd8abeb481761738f0609c2a63e81) |
| **Init Tx** | [`ae8c0438a6ecc258b99b79e87cfcbab6325ac85e1248063f2787d8f74e8e8951`](https://stellar.expert/explorer/testnet/tx/ae8c0438a6ecc258b99b79e87cfcbab6325ac85e1248063f2787d8f74e8e8951) |
| **USDC Token ID** | [`CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC`](https://stellar.expert/explorer/testnet/contract/CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC) |
| **Municipality Key** | `GAQPI4JFBAZRSAYCBQF6IK6L5D4DCRW6LCARSHWDJ6RZR3MB7QYFIH2K` |
| **Oracle Hot Key** | `GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT` |

---

## 3. Configured Escrow Parameters

| Setting | Value | Description |
| :--- | :--- | :--- |
| `bounty_stroops` | 10,000,000 (1.0000000 USDC) | Base bounty payout per valid power-down claim |
| `per_payout_cap_stroops` | 20,000,000 (2.0000000 USDC) | Upper ceiling bounding any individual payout |
| `window_len_seconds` | 86,400 s (24 hours) | Rolling window for spending budget and rate caps |
| `window_budget_stroops` | 200,000,000 (20.0000000 USDC) | Cumulative payout ceiling allowed in a 24-hour window |
| `max_recipient_window_claims` | 3 claims | Maximum payouts permitted per wallet per 24 hours |

---

## 4. Verification & Testing

- **Contract Tests**: 47 unit and randomized invariant tests passing (`cargo test --manifest-path contracts/negawatt_escrow/Cargo.toml`).
- **Security & Replay Protection**: Nonce and proof-hash uniqueness verified with persistent TTL storage.
- **Contract Interface**: Complete ABI documented in `docs/phase1/contract-interface.md`.
