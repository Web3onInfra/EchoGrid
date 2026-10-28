# EchoGrid Phase 1: NegaWatt Escrow Smart Contract Interface

This document specifies the public interface of the hardened `NegaWattEscrow` Soroban contract (`contracts/negawatt_escrow/src/lib.rs`), deployed on Stellar Testnet as documented in `deployments/testnet.json`.

---

## 1. Public Functions

| Function | Access / Auth | Parameters | Returns | Description |
|---|---|---|---|---|
| `initialize` | Municipality Admin (`require_auth`) | `municipality: Address`, `oracle: Address`, `token: Address`, `bounty: i128` | `Result<(), Error>` | One-time contract setup. Configures admin, oracle, token address, default limits, and initial bounty. |
| `deposit` | Municipality Admin (`require_auth`) | `amount: i128` | `Result<(), Error>` | Locks USDC-compatible tokens from the municipality account into escrow. |
| `withdraw` | Municipality Admin (`require_auth`) | `to: Address`, `amount: i128` | `Result<(), Error>` | Withdraws tokens from escrow back to a destination address. Bounded by pool balance. |
| `set_bounty` | Municipality Admin (`require_auth`) | `new_bounty: i128` | `Result<(), Error>` | Updates per-claim bounty in stroops. Enforces invariant: `bounty <= per_payout_cap`. |
| `set_payout_cap` | Municipality Admin (`require_auth`) | `new_cap: i128` | `Result<(), Error>` | Updates per-payout maximum ceiling. Enforces invariant: `bounty <= per_payout_cap <= window_budget`. |
| `set_window_budget` | Municipality Admin (`require_auth`) | `new_budget: i128` | `Result<(), Error>` | Updates pool spend budget per window. Enforces invariant: `per_payout_cap <= window_budget`. |
| `set_recipient_claim_limit` | Municipality Admin (`require_auth`) | `new_limit: u32` | `Result<(), Error>` | Sets maximum allowed claims per recipient per window. |
| `pause` | Municipality Admin (`require_auth`) | *(none)* | `Result<(), Error>` | Emergency circuit breaker. Halts all `payout` operations. |
| `unpause` | Municipality Admin (`require_auth`) | *(none)* | `Result<(), Error>` | Resumes payout operations after an emergency pause. |
| `set_oracle` | Municipality Admin (`require_auth`) | `new_oracle: Address` | `Result<(), Error>` | Rotates the authorized oracle key. The old key immediately loses payout authority. |
| `payout` | Oracle Hot Key (`require_auth`) | `recipient: Address`, `proof_hash: BytesN<32>` | `Result<(), Error>` | AgentPay trigger. Verifies replay status and limits, and transfers bounty from escrow to recipient. |
| `get_config` | Public (Read-Only) | *(none)* | `Result<Config, Error>` | Returns the active configuration struct. |
| `pool_balance` | Public (Read-Only) | *(none)* | `i128` | Returns current escrow token balance. |
| `escrow_balance` | Public (Read-Only) | *(none)* | `i128` | Legacy alias for `pool_balance`. |
| `bounty` | Public (Read-Only) | *(none)* | `i128` | Returns current per-claim bounty in stroops. |
| `is_claimed` | Public (Read-Only) | `proof_hash: BytesN<32>` | `bool` | Checks if a given proof hash has already been spent. |
| `is_paused` | Public (Read-Only) | *(none)* | `bool` | Checks whether the contract is currently paused. |
| `window_spent` | Public (Read-Only) | *(none)* | `i128` | Returns total stroops paid out in the current window. |
| `recipient_window_claims` | Public (Read-Only) | `recipient: Address` | `u32` | Returns claims made by a recipient in the current window. |

---

## 2. Error Codes (`Error`)

The contract defines typed numeric errors using `#[contracterror]`:

| Code | Error Variant | HTTP Status (Oracle Mapping) | Description / Cause |
|---|---|---|---|
| `1` | `AlreadyInitialized` | `409 Conflict` / `500` | Attempted to initialize an already-initialized contract. |
| `2` | `NotInitialized` | `500 Internal Server Error` | Attempted to access an uninitialized contract instance. |
| `3` | `Unauthorized` | `403 Forbidden` | Caller failed signature authentication or lacked required role. |
| `4` | `InvalidAmount` | `400 Bad Request` | Deposit, withdrawal, or limit amount is non-positive or zero. |
| `5` | `InvalidBounty` | `400 Bad Request` | Specified bounty amount is non-positive. |
| `6` | `InvalidCap` | `400 Bad Request` | Specified payout cap is non-positive. |
| `7` | `InvalidBudget` | `400 Bad Request` | Specified window budget is non-positive. |
| `8` | `InvariantViolated` | `422 Unprocessable Entity` | Parameter violates `bounty <= per_payout_cap <= window_budget`. |
| `9` | `InsufficientBalance` | `503 Service Unavailable` | Contract token balance is lower than the requested bounty/withdrawal. |
| `10` | `ReplayClaim` | `409 Conflict` | Proof hash has already been claimed and recorded in persistent storage. |
| `11` | `ExceedsPayoutCap` | `422 Unprocessable Entity` | Bounty exceeds the configured per-payout maximum cap. |
| `12` | `ExceedsWindowBudget` | `429 Too Many Requests` | Window spend would exceed the pool's configured window budget. |
| `13` | `ExceedsRecipientWindowLimit` | `429 Too Many Requests` | Recipient has reached maximum claims allowed in the active window. |
| `14` | `ContractPaused` | `503 Service Unavailable` | Payout attempt while emergency circuit breaker is active. |

---

## 3. Events (`env.events().publish`)

| Event Name Symbol | Topics | Data Payload | Emitted When |
|---|---|---|---|
| `"negawatt_initialized"` | `(Symbol, municipality: Address)` | `(oracle: Address, token: Address, bounty: i128)` | Contract initialization completes successfully. |
| `"negawatt_deposited"` | `(Symbol, municipality: Address)` | `amount: i128` | Municipality deposits tokens into escrow. |
| `"negawatt_withdrawn"` | `(Symbol, municipality: Address, to: Address)` | `amount: i128` | Municipality withdraws tokens from escrow. |
| `"negawatt_bounty_set"` | `(Symbol, municipality: Address)` | `new_bounty: i128` | Municipality updates the bounty rate. |
| `"negawatt_config_updated"` | `(Symbol, municipality: Address)` | `config: Config` | Any configuration parameter is updated. |
| `"negawatt_paused"` | `(Symbol, municipality: Address)` | `()` | Municipality activates emergency pause. |
| `"negawatt_unpaused"` | `(Symbol, municipality: Address)` | `()` | Municipality deactivates emergency pause. |
| `"negawatt_oracle_rotated"` | `(Symbol, municipality: Address)` | `(old_oracle: Address, new_oracle: Address)` | Municipality rotates oracle authorization. |
| `"negawatt_payout"` | `(Symbol, oracle: Address, recipient: Address)` | `(amount: i128, proof_hash: BytesN<32>)` | Oracle successfully executes a bounty payout to a user. |

---

## 4. Configuration Schema (`Config`)

```rust
pub struct Config {
    pub municipality: Address,
    pub oracle: Address,
    pub token: Address,
    pub bounty: i128,
    pub per_payout_cap: i128,
    pub window_len: u64,
    pub window_budget: i128,
    pub max_recipient_window_claims: u32,
}
```

- Storage tier: Instance storage under `DataKey::Config`
- Invariant enforced on every configuration write: `bounty <= per_payout_cap <= window_budget`
