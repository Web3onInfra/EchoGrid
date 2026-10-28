# EchoGrid Phase 1: NegaWatt Escrow Hardening & Threat Model

## 1. Overview & Threat Model
The NegaWatt Escrow contract (`contracts/negawatt_escrow/`) manages treasury payouts on Stellar for verified power-down events.
- **Roles**:
  - `MunicipalityAdmin` (`ROLE_MUNICIPALITY`): Cold/multisig key governing parameters, deposits, pauses, rotations, and withdrawals.
  - `OracleAddress` (`ROLE_ORACLE`): Hot server key triggering automated `payout` calls upon verifying acoustic proof-of-power-down off-chain.
- **Threat Vector**:
  - The oracle key resides on network-connected infrastructure and may leak, be compromised, or suffer logic bypasses.
- **Damage Bounding Guarantee**:
  - The contract strictly bounds maximum possible loss mathematically on-chain without trusting the oracle.
  - In the event of a total oracle key compromise, maximum loss is strictly capped at **20 XLM per 24-hour window** (`window_budget`), and no single recipient address can claim more than **3 payouts per window** (maximum 3 XLM total per recipient).
  - The municipality can pause the contract or rotate the oracle key at any time.

## 2. Default Safety Limits & Invariant
All amounts are stroops (`1 XLM = 10^7 stroops` in `i128`, no floating point):
- **`bounty`**: `1 XLM` (`10_000_000` stroops) per proof payout.
- **`per_payout_cap`**: `2 XLM` (`20_000_000` stroops). Single payouts exceeding this are rejected.
- **`window_len`**: `86,400` seconds (24-hour fixed windows).
- **`window_budget`**: `20 XLM` (`200_000_000` stroops) pool-wide maximum spend per window.
- **`max_recipient_window_claims`**: `3` claims per recipient address per window.
- **Core Invariant**:
  $$\text{bounty} \le \text{per\_payout\_cap} \le \text{window\_budget}$$
  Enforced on every state-changing configuration write.

## 3. Storage Layout & Tier Justification
Soroban provides three storage tiers. Each is chosen based on lifecycle and rent cost:
1. **Instance Storage (`storage().instance()`)**:
   - `DataKey::Config`: Structured `Config` containing roles, token, and limit settings.
   - `DataKey::Paused`: Boolean contract pause flag.
   - `DataKey::CurrentWindowSpend`: `(u64, i128)` tracking `(window_id, spent_amount)`.
   - *Rationale*: Compact, loaded on every invocation, automatically extended with instance TTL.
2. **Persistent Storage (`storage().persistent()`)**:
   - `DataKey::ClaimUsed(BytesN<32>)`: Records spent Proof-of-Power-Down hashes.
   - *Rationale*: Replay protection must endure for long horizons. Extended to ~1 year (~6M ledgers @ 5s/ledger) per claim.
3. **Temporary Storage (`storage().temporary()`)**:
   - `DataKey::RecipientClaims(u64, Address)`: Tracks claim counts per recipient for `window_id = timestamp / 86400`.
   - *Rationale*: Window limits are epoch-scoped. Once the window passes, claim counts are irrelevant. Temporary storage automatically expires, avoiding ledger bloat and rent costs.

## 4. TTL & Rent Management
- **Instance TTL**: State mutations (`deposit`, `payout`, `set_config`, `pause`, `unpause`, `withdraw`, `set_oracle`) extend instance TTL by `535_680` ledgers (~31 days).
- **Persistent Replay TTL**: Each `payout` extends the persistent `ClaimUsed` entry TTL to `6_000_000` ledgers (~1 year).
- **Phase 2 Nonce Bridge**: Beyond 1 year, Phase 2's short-lived challenge nonces (valid for < 15 minutes) prevent replay of stale proofs even if a persistent entry were ever expired.

## 5. Contract Errors (`#[contracterror]`)
Panics are replaced with a typed enum with stable numeric codes:
- `1: AlreadyInitialized`
- `2: NotInitialized`
- `3: Unauthorized`
- `4: InvalidAmount`
- `5: InvalidBounty`
- `6: InvalidCap`
- `7: InvalidBudget`
- `8: InvariantViolated`
- `9: InsufficientBalance`
- `10: ReplayClaim`
- `11: ExceedsPayoutCap`
- `12: ExceedsWindowBudget`
- `13: ExceedsRecipientWindowLimit`
- `14: ContractPaused`

## 6. Events Emitted
Events are published using typed symbols and indexed topics:
- `negawatt_initialized`: `(Symbol, municipality)`, `(oracle, token_id, bounty)`
- `negawatt_deposited`: `(Symbol, municipality)`, `amount`
- `negawatt_payout`: `(Symbol, oracle, recipient)`, `(amount, proof_hash)`
- `negawatt_bounty_set`: `(Symbol, municipality)`, `new_bounty`
- `negawatt_config_updated`: `(Symbol, municipality)`, `config`
- `negawatt_paused`: `(Symbol, municipality)`, `()`
- `negawatt_unpaused`: `(Symbol, municipality)`, `()`
- `negawatt_oracle_rotated`: `(Symbol, municipality)`, `(old_oracle, new_oracle)`
- `negawatt_withdrawn`: `(Symbol, municipality, to)`, `amount`

## 7. Public Function Signatures & Oracle Compatibility
Public signatures are 100% backward-compatible with Phase 0 callers (`oracle/src/stellar.rs`):
```rust
// Core State-Changing Functions
pub fn initialize(env: Env, municipality: Address, oracle: Address, token: Address, bounty: i128);
pub fn deposit(env: Env, amount: i128);
pub fn set_bounty(env: Env, new_bounty: i128);
pub fn payout(env: Env, recipient: Address, proof_hash: BytesN<32>);

// Hardening & Administration (Municipality Only)
pub fn set_config(env: Env, config: Config);
pub fn set_oracle(env: Env, new_oracle: Address);
pub fn pause(env: Env);
pub fn unpause(env: Env);
pub fn withdraw(env: Env, to: Address, amount: i128);

// Read-Only Views
pub fn get_config(env: Env) -> Config;
pub fn pool_balance(env: Env) -> i128;
pub fn escrow_balance(env: Env) -> i128; // legacy alias
pub fn bounty(env: Env) -> i128;
pub fn is_claimed(env: Env, proof_hash: BytesN<32>) -> bool;
pub fn is_paused(env: Env) -> bool;
pub fn window_spent(env: Env) -> i128;
pub fn recipient_window_claims(env: Env, recipient: Address) -> u32;
```
