//! # NegaWatt Escrow — Soroban Smart Contract
//!
//! This contract is the on-chain treasury for EchoGrid's NegaWatt marketplace.
//!
//! ## Flow
//! 1. A **Municipality** admin calls `initialize` to set the oracle address and
//!    the per-claim bounty amount (in stroops of the mock USDC token).
//! 2. The **Municipality** calls `deposit` to lock USDC into the contract.
//! 3. When a user's phone proves power-down off-chain, the **Oracle** calls
//!    `payout` — the only way to move funds out of escrow.
//!
//! ## Security Model (AgentGuard RBAC)
//! Two privileged roles are stored in contract storage:
//!   - `ROLE_MUNICIPALITY` — can `initialize`, `deposit`, and `set_bounty`.
//!   - `ROLE_ORACLE`       — can call `payout` (the AgentPay trigger).
//!
//! Both roles are enforced by `require_role`, which calls
//! `env.require_auth()` so the Stellar network itself verifies the signature —
//! no trust in the caller's claim.
#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, token::Client as TokenClient, Address,
    BytesN, Env, Symbol,
};

// ---------------------------------------------------------------------------
// ── Error Enum ─────────────────────────────────────────────────────────────
// ---------------------------------------------------------------------------

/// Typed errors with stable numeric codes for auditability and client handling.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
    InvalidAmount = 4,
    InvalidBounty = 5,
    InvalidCap = 6,
    InvalidBudget = 7,
    InvariantViolated = 8,
    InsufficientBalance = 9,
    ReplayClaim = 10,
    ExceedsPayoutCap = 11,
    ExceedsWindowBudget = 12,
    ExceedsRecipientWindowLimit = 13,
    ContractPaused = 14,
}

// ---------------------------------------------------------------------------
// ── Configuration Struct ───────────────────────────────────────────────────
// ---------------------------------------------------------------------------

/// Central configuration for contract roles, token, and safety limits.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
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

// ---------------------------------------------------------------------------
// ── Storage Key Enum ────────────────────────────────────────────────────────
// ---------------------------------------------------------------------------

/// All persistent state lives under typed keys.  Using an enum prevents
/// collisions and makes storage layout explicit for auditors.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// The Stellar address of the municipality admin.
    MunicipalityAdmin,
    /// The Stellar address of the trusted off-chain oracle (Rust backend).
    OracleAddress,
    /// Contract address of the USDC-compatible token we hold in escrow.
    TokenId,
    /// How many stroops are paid out per verified NegaWatt event.
    BountyPerClaim,
    /// Whether the contract has been initialised (prevents re-init attacks).
    Initialized,
    /// Tracks whether a given Proof-of-Power-Down hash has already been paid
    /// — prevents replay attacks.
    ClaimUsed(BytesN<32>),
    /// Global contract configuration struct.
    Config,
    /// Emergency circuit breaker pause status.
    Paused,
    /// Current window spend: (window_id, spent_amount).
    CurrentWindowSpend,
    /// Per-recipient claim count in a given window: (window_id, recipient).
    RecipientClaims(u64, Address),
}

// ---------------------------------------------------------------------------
// ── Events ───────────────────────────────────────────────────────────────────
// ---------------------------------------------------------------------------

/// Emitted when the escrow contract is initialized.
const EVENT_INITIALIZED: &str = "negawatt_initialized";
/// Emitted when the municipality deposits USDC into escrow.
const EVENT_DEPOSITED: &str = "negawatt_deposited";
/// Emitted for every successful NegaWatt payout to a user.
const EVENT_PAYOUT: &str = "negawatt_payout";
/// Emitted when the bounty rate is updated.
const EVENT_BOUNTY_SET: &str = "negawatt_bounty_set";
/// Emitted when configuration parameters are updated.
const EVENT_CONFIG_UPDATED: &str = "negawatt_config_updated";
/// Emitted when emergency pause is activated.
const EVENT_PAUSED: &str = "negawatt_paused";
/// Emitted when emergency pause is lifted.
const EVENT_UNPAUSED: &str = "negawatt_unpaused";
/// Emitted when the oracle key is rotated.
const EVENT_ORACLE_ROTATED: &str = "negawatt_oracle_rotated";
/// Emitted when funds are withdrawn by the municipality.
const EVENT_WITHDRAWN: &str = "negawatt_withdrawn";

// ---------------------------------------------------------------------------
// ── TTL Rent Parameters ───────────────────────────────────────────────────
// ---------------------------------------------------------------------------

/// Threshold in ledgers before extending instance TTL (~7 days @ 5s/ledger).
pub const INSTANCE_TTL_THRESHOLD: u32 = 120_960;
/// Target in ledgers to extend instance TTL to (~31 days @ 5s/ledger).
pub const INSTANCE_EXTEND_TO: u32 = 535_680;

/// Threshold in ledgers before extending persistent claim TTL (~30 days).
pub const PERSISTENT_CLAIM_THRESHOLD: u32 = 518_400;
/// Maximum extension for persistent claim TTL (~347 days / ~6M ledgers).
pub const PERSISTENT_CLAIM_EXTEND_TO: u32 = 6_000_000;

// ---------------------------------------------------------------------------
// ── Contract Entry Point ────────────────────────────────────────────────────
// ---------------------------------------------------------------------------

#[contract]
pub struct NegaWattEscrow;

#[contractimpl]
impl NegaWattEscrow {
    // -----------------------------------------------------------------------
    // ── Initialization ──────────────────────────────────────────────────────
    // -----------------------------------------------------------------------

    /// **One-time setup** — must be called by the municipality before anything else.
    ///
    /// # Arguments
    /// * `municipality` — Address that will have admin rights (depositing funds,
    ///   adjusting bounty). The caller must authorise this transaction.
    /// * `oracle`       — Address of the Rust AgentPay oracle. Only this address
    ///   may call `payout`.
    /// * `token`        — Contract ID of the USDC-equivalent token.
    /// * `bounty`       — Initial per-claim payout in stroops (1 XLM = 10^7 stroops).
    pub fn initialize(
        env: Env,
        municipality: Address,
        oracle: Address,
        token: Address,
        bounty: i128,
    ) -> Result<(), Error> {
        // Guard: prevent re-initialisation which could overwrite the oracle address.
        if env
            .storage()
            .instance()
            .get::<_, bool>(&DataKey::Initialized)
            .unwrap_or(false)
        {
            return Err(Error::AlreadyInitialized);
        }

        if bounty <= 0 {
            return Err(Error::InvalidBounty);
        }

        // The municipality must sign this transaction, anchoring ownership.
        municipality.require_auth();

        // Persist RBAC roles and configuration.
        env.storage()
            .instance()
            .set(&DataKey::MunicipalityAdmin, &municipality);
        env.storage()
            .instance()
            .set(&DataKey::OracleAddress, &oracle);
        env.storage().instance().set(&DataKey::TokenId, &token);
        env.storage()
            .instance()
            .set(&DataKey::BountyPerClaim, &bounty);
        env.storage().instance().set(&DataKey::Initialized, &true);

        let cap = if bounty > 20_000_000 {
            bounty
        } else {
            20_000_000
        };
        let budget = if cap > 200_000_000 { cap } else { 200_000_000 };
        let config = Config {
            municipality: municipality.clone(),
            oracle: oracle.clone(),
            token: token.clone(),
            bounty,
            per_payout_cap: cap,
            window_len: 86_400,
            window_budget: budget,
            max_recipient_window_claims: 3,
        };
        env.storage().instance().set(&DataKey::Config, &config);
        env.storage().instance().set(&DataKey::Paused, &false);

        env.events().publish(
            (Symbol::new(&env, EVENT_INITIALIZED), municipality.clone()),
            (oracle.clone(), token.clone(), bounty),
        );

        Self::extend_instance_ttl(&env);

        Ok(())
    }

    // -----------------------------------------------------------------------
    // ── Municipality Functions (AgentGuard: ROLE_MUNICIPALITY) ──────────────
    // -----------------------------------------------------------------------

    /// **Deposit** — The municipality locks USDC into this escrow contract.
    ///
    /// Only the `MunicipalityAdmin` role may call this.  The token transfer
    /// is authorised by the municipality's Stellar keypair at the network level.
    ///
    /// # Arguments
    /// * `amount` — How many stroops of the token to deposit.
    pub fn deposit(env: Env, amount: i128) -> Result<(), Error> {
        // ── AgentGuard: ROLE_MUNICIPALITY ───────────────────────────────────
        let municipality = Self::require_municipality_role(&env)?;

        // Validate deposit amount — negative deposits would drain the contract.
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        let token_id: Address = env
            .storage()
            .instance()
            .get(&DataKey::TokenId)
            .ok_or(Error::NotInitialized)?;

        // Transfer USDC from the municipality wallet into *this* contract.
        // The SDK handles the SAC (Stellar Asset Contract) cross-call here.
        TokenClient::new(&env, &token_id).transfer(
            &municipality,                   // from: the municipality's account
            &env.current_contract_address(), // to: this escrow contract
            &amount,
        );

        // Emit an indexed event so dashboards can track deposits in real time.
        env.events()
            .publish((Symbol::new(&env, EVENT_DEPOSITED), municipality), amount);

        Self::extend_instance_ttl(&env);

        Ok(())
    }

    /// **Withdraw** — Withdraw funds from the escrow pool back to a destination address.
    ///
    /// Municipality-only. Bounded by pool balance. Allowed while paused.
    pub fn withdraw(env: Env, to: Address, amount: i128) -> Result<(), Error> {
        let municipality = Self::require_municipality_role(&env)?;

        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        let token_id: Address = env
            .storage()
            .instance()
            .get(&DataKey::TokenId)
            .ok_or(Error::NotInitialized)?;

        let balance = TokenClient::new(&env, &token_id).balance(&env.current_contract_address());
        if balance < amount {
            return Err(Error::InsufficientBalance);
        }

        TokenClient::new(&env, &token_id).transfer(&env.current_contract_address(), &to, &amount);

        env.events().publish(
            (Symbol::new(&env, EVENT_WITHDRAWN), municipality, to),
            amount,
        );

        Self::extend_instance_ttl(&env);

        Ok(())
    }

    /// **Set Bounty** — Update the per-claim NegaWatt payout rate.
    ///
    /// Allows the municipality to react to grid conditions — higher bounties
    /// during peak demand, lower during off-peak.
    pub fn set_bounty(env: Env, new_bounty: i128) -> Result<(), Error> {
        // ── AgentGuard: ROLE_MUNICIPALITY ───────────────────────────────────
        let municipality = Self::require_municipality_role(&env)?;

        if new_bounty <= 0 {
            return Err(Error::InvalidBounty);
        }

        let mut config: Config = env
            .storage()
            .instance()
            .get(&DataKey::Config)
            .ok_or(Error::NotInitialized)?;

        // Enforce invariant: bounty <= per_payout_cap
        if new_bounty > config.per_payout_cap {
            return Err(Error::InvariantViolated);
        }

        config.bounty = new_bounty;
        env.storage().instance().set(&DataKey::Config, &config);
        env.storage()
            .instance()
            .set(&DataKey::BountyPerClaim, &new_bounty);

        env.events().publish(
            (Symbol::new(&env, EVENT_BOUNTY_SET), municipality.clone()),
            new_bounty,
        );
        env.events().publish(
            (Symbol::new(&env, EVENT_CONFIG_UPDATED), municipality),
            config,
        );

        Self::extend_instance_ttl(&env);

        Ok(())
    }

    /// **Set Payout Cap** — Update the per-payout maximum ceiling.
    ///
    /// Municipality-only. Enforces invariant: bounty <= new_cap <= window_budget.
    pub fn set_payout_cap(env: Env, new_cap: i128) -> Result<(), Error> {
        let municipality = Self::require_municipality_role(&env)?;

        if new_cap <= 0 {
            return Err(Error::InvalidCap);
        }

        let mut config: Config = env
            .storage()
            .instance()
            .get(&DataKey::Config)
            .ok_or(Error::NotInitialized)?;

        // Enforce invariant: bounty <= new_cap <= window_budget
        if config.bounty > new_cap || new_cap > config.window_budget {
            return Err(Error::InvariantViolated);
        }

        config.per_payout_cap = new_cap;
        env.storage().instance().set(&DataKey::Config, &config);

        env.events().publish(
            (Symbol::new(&env, EVENT_CONFIG_UPDATED), municipality),
            config,
        );

        Self::extend_instance_ttl(&env);

        Ok(())
    }

    /// **Set Window Budget** — Update the pool-level payout budget per window.
    ///
    /// Municipality-only. Enforces invariant: per_payout_cap <= new_budget.
    pub fn set_window_budget(env: Env, new_budget: i128) -> Result<(), Error> {
        let municipality = Self::require_municipality_role(&env)?;

        if new_budget <= 0 {
            return Err(Error::InvalidBudget);
        }

        let mut config: Config = env
            .storage()
            .instance()
            .get(&DataKey::Config)
            .ok_or(Error::NotInitialized)?;

        // Enforce invariant: per_payout_cap <= new_budget
        if config.per_payout_cap > new_budget {
            return Err(Error::InvariantViolated);
        }

        config.window_budget = new_budget;
        env.storage().instance().set(&DataKey::Config, &config);

        env.events().publish(
            (Symbol::new(&env, EVENT_CONFIG_UPDATED), municipality),
            config,
        );

        Self::extend_instance_ttl(&env);

        Ok(())
    }

    /// **Set Recipient Claim Limit** — Set max claims per recipient per window.
    ///
    /// Municipality-only.
    pub fn set_recipient_claim_limit(env: Env, new_limit: u32) -> Result<(), Error> {
        let municipality = Self::require_municipality_role(&env)?;

        if new_limit == 0 {
            return Err(Error::InvalidAmount);
        }

        let mut config: Config = env
            .storage()
            .instance()
            .get(&DataKey::Config)
            .ok_or(Error::NotInitialized)?;

        config.max_recipient_window_claims = new_limit;
        env.storage().instance().set(&DataKey::Config, &config);

        env.events().publish(
            (Symbol::new(&env, EVENT_CONFIG_UPDATED), municipality),
            config,
        );

        Self::extend_instance_ttl(&env);

        Ok(())
    }

    /// **Pause** — Emergency circuit breaker to halt payouts.
    ///
    /// Municipality-only.
    pub fn pause(env: Env) -> Result<(), Error> {
        let municipality = Self::require_municipality_role(&env)?;
        if !env.storage().instance().has(&DataKey::Config) {
            return Err(Error::NotInitialized);
        }
        env.storage().instance().set(&DataKey::Paused, &true);

        env.events()
            .publish((Symbol::new(&env, EVENT_PAUSED), municipality), ());

        Self::extend_instance_ttl(&env);

        Ok(())
    }

    /// **Unpause** — Resume payout operations after a pause.
    ///
    /// Municipality-only.
    pub fn unpause(env: Env) -> Result<(), Error> {
        let municipality = Self::require_municipality_role(&env)?;
        if !env.storage().instance().has(&DataKey::Config) {
            return Err(Error::NotInitialized);
        }
        env.storage().instance().set(&DataKey::Paused, &false);

        env.events()
            .publish((Symbol::new(&env, EVENT_UNPAUSED), municipality), ());

        Self::extend_instance_ttl(&env);

        Ok(())
    }

    /// **Set Oracle** — Rotates the authorized oracle key.
    ///
    /// Municipality-only. The old oracle key immediately loses authorization
    /// to trigger payouts.
    pub fn set_oracle(env: Env, new_oracle: Address) -> Result<(), Error> {
        let municipality = Self::require_municipality_role(&env)?;

        let mut config: Config = env
            .storage()
            .instance()
            .get(&DataKey::Config)
            .ok_or(Error::NotInitialized)?;

        let old_oracle = config.oracle.clone();
        config.oracle = new_oracle.clone();
        env.storage().instance().set(&DataKey::Config, &config);
        env.storage()
            .instance()
            .set(&DataKey::OracleAddress, &new_oracle);

        env.events().publish(
            (Symbol::new(&env, EVENT_ORACLE_ROTATED), municipality),
            (old_oracle, new_oracle),
        );

        Self::extend_instance_ttl(&env);

        Ok(())
    }

    // -----------------------------------------------------------------------
    // ── Oracle Function (AgentGuard: ROLE_ORACLE / AgentPay) ────────────────
    // -----------------------------------------------------------------------

    /// **Payout** — The AgentPay trigger.  Called exclusively by the Rust oracle
    /// after it has cryptographically verified the Proof-of-Power-Down.
    ///
    /// This is the NegaWatt payout: real USDC moves from escrow to the user
    /// who silenced their appliance, in exchange for grid relief.
    ///
    /// # Arguments
    /// * `recipient`  — Stellar address of the user who earned the bounty.
    /// * `proof_hash` — The 32-byte SHA-256 hash that constitutes the
    ///   Proof-of-Power-Down.  Stored after use to prevent replay attacks.
    pub fn payout(env: Env, recipient: Address, proof_hash: BytesN<32>) -> Result<(), Error> {
        // ── Emergency Circuit Breaker ───────────────────────────────────────
        if Self::is_paused(env.clone()) {
            return Err(Error::ContractPaused);
        }

        // ── AgentGuard: ROLE_ORACLE ─────────────────────────────────────────
        let oracle = Self::require_oracle_role(&env)?;

        let config: Config = env
            .storage()
            .instance()
            .get(&DataKey::Config)
            .ok_or(Error::NotInitialized)?;

        // ── Replay-attack prevention ────────────────────────────────────────
        // Each proof_hash must be unique (it embeds timestamp + wallet + nonce).
        // If we've paid this hash before, reject immediately.
        let claim_key = DataKey::ClaimUsed(proof_hash.clone());
        if env
            .storage()
            .persistent()
            .get::<_, bool>(&claim_key)
            .unwrap_or(false)
        {
            return Err(Error::ReplayClaim);
        }

        let token_id: Address = env
            .storage()
            .instance()
            .get(&DataKey::TokenId)
            .ok_or(Error::NotInitialized)?;

        // NegaWatt payout triggered here ─────────────────────────────────────
        let bounty: i128 = env
            .storage()
            .instance()
            .get(&DataKey::BountyPerClaim)
            .ok_or(Error::NotInitialized)?;

        // Enforce per-payout cap
        if bounty > config.per_payout_cap {
            return Err(Error::ExceedsPayoutCap);
        }

        // Fixed-window pool budget accounting
        let window_len = config.window_len.max(1);
        let current_window_id = env.ledger().timestamp() / window_len;
        let (stored_window_id, stored_spend): (u64, i128) = env
            .storage()
            .instance()
            .get(&DataKey::CurrentWindowSpend)
            .unwrap_or((0, 0));

        let current_window_spent = if stored_window_id == current_window_id {
            stored_spend
        } else {
            0
        };

        if current_window_spent + bounty > config.window_budget {
            return Err(Error::ExceedsWindowBudget);
        }

        // Per-recipient claim limit per window
        let recipient_claims_key = DataKey::RecipientClaims(current_window_id, recipient.clone());
        let current_recipient_claims: u32 = env
            .storage()
            .temporary()
            .get(&recipient_claims_key)
            .unwrap_or(0);

        if current_recipient_claims >= config.max_recipient_window_claims {
            return Err(Error::ExceedsRecipientWindowLimit);
        }

        // Sanity-check: make sure the escrow still has enough funds.
        let balance = TokenClient::new(&env, &token_id).balance(&env.current_contract_address());
        if balance < bounty {
            return Err(Error::InsufficientBalance);
        }

        // Transfer the NegaWatt bounty from escrow → user wallet.
        TokenClient::new(&env, &token_id).transfer(
            &env.current_contract_address(), // from: escrow
            &recipient,                      // to: verified user
            &bounty,
        );

        // Update pool window spend
        env.storage().instance().set(
            &DataKey::CurrentWindowSpend,
            &(current_window_id, current_window_spent + bounty),
        );

        // Update per-recipient claims count in temporary storage
        env.storage()
            .temporary()
            .set(&recipient_claims_key, &(current_recipient_claims + 1));

        // Mark this proof as spent so it can never be replayed.
        // We use `persistent` storage so it survives ledger expiry TTLs.
        env.storage().persistent().set(&claim_key, &true);

        // Extend the TTL of the claim record for ~1 year (≈ 6M ledgers @ 5s/ledger).
        // Without this, the entry would expire and a replay could succeed.
        env.storage().persistent().extend_ttl(
            &claim_key,
            PERSISTENT_CLAIM_THRESHOLD,
            PERSISTENT_CLAIM_EXTEND_TO,
        );

        // Emit payout event — indexers will use this to build the user dashboard.
        env.events().publish(
            (Symbol::new(&env, EVENT_PAYOUT), oracle, recipient.clone()),
            (bounty, proof_hash.clone()),
        );

        Self::extend_instance_ttl(&env);

        Ok(())
    }

    // -----------------------------------------------------------------------
    // ── Read-Only Views ────────────────────────────────----------------------
    // -----------------------------------------------------------------------

    /// Returns the global Config struct.
    pub fn get_config(env: Env) -> Result<Config, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Config)
            .ok_or(Error::NotInitialized)
    }

    /// Returns the current escrow pool balance of the held token (successor to escrow_balance).
    pub fn pool_balance(env: Env) -> i128 {
        Self::escrow_balance(env)
    }

    /// Returns whether emergency pause is active.
    pub fn is_paused(env: Env) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false)
    }

    /// Returns the amount paid out in the current window.
    pub fn window_spent(env: Env) -> i128 {
        let (window_id, spent): (u64, i128) = env
            .storage()
            .instance()
            .get(&DataKey::CurrentWindowSpend)
            .unwrap_or((0, 0));
        let config = env.storage().instance().get::<_, Config>(&DataKey::Config);
        if let Some(cfg) = config {
            let current_window = env.ledger().timestamp() / cfg.window_len.max(1);
            if current_window == window_id {
                return spent;
            }
        }
        0
    }

    /// Returns the number of claims by a recipient in the current window.
    pub fn recipient_window_claims(env: Env, recipient: Address) -> u32 {
        let config = env.storage().instance().get::<_, Config>(&DataKey::Config);
        if let Some(cfg) = config {
            let current_window = env.ledger().timestamp() / cfg.window_len.max(1);
            let key = DataKey::RecipientClaims(current_window, recipient);
            return env.storage().temporary().get(&key).unwrap_or(0);
        }
        0
    }

    /// Returns the current escrow balance of the held token.
    pub fn escrow_balance(env: Env) -> i128 {
        let token_id: Address = env
            .storage()
            .instance()
            .get(&DataKey::TokenId)
            .expect("EchoGrid: not initialized");
        TokenClient::new(&env, &token_id).balance(&env.current_contract_address())
    }

    /// Returns the current per-claim NegaWatt bounty (in stroops).
    pub fn bounty(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::BountyPerClaim)
            .expect("EchoGrid: not initialized")
    }

    /// Returns whether a specific proof_hash has already been spent.
    pub fn is_claimed(env: Env, proof_hash: BytesN<32>) -> bool {
        env.storage()
            .persistent()
            .get::<_, bool>(&DataKey::ClaimUsed(proof_hash))
            .unwrap_or(false)
    }

    // -----------------------------------------------------------------------
    // ── Private: AgentGuard Role Enforcement ────────────────────────────────
    // -----------------------------------------------------------------------

    /// Retrieves the `MunicipalityAdmin` address and calls `require_auth()`.
    /// The Stellar network verifies the Ed25519 signature — we never trust
    /// the caller's assertion of identity.
    fn require_municipality_role(env: &Env) -> Result<Address, Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::MunicipalityAdmin)
            .ok_or(Error::NotInitialized)?;
        // require_auth() aborts the transaction if the signature is invalid.
        admin.require_auth();
        Ok(admin)
    }

    /// Retrieves the `OracleAddress` and calls `require_auth()`.
    /// This ensures only the Rust AgentPay backend (holding the oracle keypair)
    /// can trigger payouts — no user can call `payout` directly.
    fn require_oracle_role(env: &Env) -> Result<Address, Error> {
        let oracle: Address = env
            .storage()
            .instance()
            .get(&DataKey::OracleAddress)
            .ok_or(Error::NotInitialized)?;
        oracle.require_auth();
        Ok(oracle)
    }

    /// Extends instance storage TTL so contract state remains live without rent decay.
    fn extend_instance_ttl(env: &Env) {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_EXTEND_TO);
    }
}

// ---------------------------------------------------------------------------
// ── Tests ────────────────────────────────────────────────────────────────────
// ---------------------------------------------------------------------------

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Events as _, Ledger as _},
        token::StellarAssetClient,
        Address, BytesN, Env,
    };

    /// Helper: deploy a fresh contract and a mock USDC token, returning
    /// handles for all three parties (municipality, oracle, user).
    fn setup() -> (
        Env,
        Address,
        NegaWattEscrowClient<'static>,
        Address,
        Address,
        Address,
    ) {
        let env = Env::default();
        // Allow all auth in tests — we test auth separately.
        env.mock_all_auths();

        let municipality = Address::generate(&env);
        let oracle = Address::generate(&env);
        let user = Address::generate(&env);

        // Deploy a Stellar Asset Contract (mock USDC) for testing.
        let token_admin = Address::generate(&env);
        let token_id = env
            .register_stellar_asset_contract_v2(token_admin.clone())
            .address();

        // Mint 10_000 USDC (in stroops) to the municipality so it can deposit.
        StellarAssetClient::new(&env, &token_id).mint(&municipality, &10_000_i128);

        // Deploy the NegaWatt escrow contract.
        let contract_id = env.register_contract(None, NegaWattEscrow);
        let client = NegaWattEscrowClient::new(&env, &contract_id);

        // Initialize with a 100-stroop bounty per NegaWatt event.
        client.initialize(&municipality, &oracle, &token_id, &100_i128);

        (env, token_id, client, municipality, oracle, user)
    }

    #[test]
    fn test_deposit_and_balance() {
        let (_, _, client, _, _, _) = setup();

        client.deposit(&5_000_i128);
        assert_eq!(client.escrow_balance(), 5_000_i128);
    }

    #[test]
    fn test_payout_reduces_balance() {
        let (env, _, client, _, _, user) = setup();

        // Municipality deposits 500 stroops.
        client.deposit(&500_i128);

        // Oracle submits a valid proof — simulate a NegaWatt event.
        let proof: BytesN<32> = BytesN::from_array(&env, &[1u8; 32]);
        client.payout(&user, &proof);

        // Balance should be 500 - 100 (bounty) = 400.
        assert_eq!(client.escrow_balance(), 400_i128);
    }

    #[test]
    fn test_replay_attack_blocked() {
        let (env, _, client, _, _, user) = setup();

        client.deposit(&1_000_i128);
        let proof: BytesN<32> = BytesN::from_array(&env, &[2u8; 32]);

        // First claim — should succeed.
        client.payout(&user, &proof);

        // Second claim with the same proof — must return Error::ReplayClaim.
        assert_eq!(
            client.try_payout(&user, &proof).unwrap_err(),
            Ok(Error::ReplayClaim)
        );
    }

    #[test]
    fn test_reinit_blocked() {
        let (_, _, client, municipality, oracle, _) = setup();
        let fake_token = municipality.clone();
        // Attempting to re-initialize must return Error::AlreadyInitialized.
        assert_eq!(
            client
                .try_initialize(&municipality, &oracle, &fake_token, &999_i128)
                .unwrap_err(),
            Ok(Error::AlreadyInitialized)
        );
    }

    #[test]
    fn test_set_bounty() {
        let (_, _, client, _, _, _) = setup();
        client.set_bounty(&250_i128);
        assert_eq!(client.bounty(), 250_i128);
    }

    #[test]
    fn test_deposit_zero_or_negative_fails() {
        let (_, _, client, _, _, _) = setup();
        assert_eq!(
            client.try_deposit(&0_i128).unwrap_err(),
            Ok(Error::InvalidAmount)
        );
        assert_eq!(
            client.try_deposit(&-50_i128).unwrap_err(),
            Ok(Error::InvalidAmount)
        );
    }

    #[test]
    fn test_set_bounty_zero_or_negative_fails() {
        let (_, _, client, _, _, _) = setup();
        assert_eq!(
            client.try_set_bounty(&0_i128).unwrap_err(),
            Ok(Error::InvalidBounty)
        );
        assert_eq!(
            client.try_set_bounty(&-10_i128).unwrap_err(),
            Ok(Error::InvalidBounty)
        );
    }

    #[test]
    fn test_initialize_invalid_bounty_fails() {
        let env = Env::default();
        env.mock_all_auths();
        let municipality = Address::generate(&env);
        let oracle = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let token_id = env
            .register_stellar_asset_contract_v2(token_admin)
            .address();
        let contract_id = env.register_contract(None, NegaWattEscrow);
        let client = NegaWattEscrowClient::new(&env, &contract_id);

        assert_eq!(
            client
                .try_initialize(&municipality, &oracle, &token_id, &0_i128)
                .unwrap_err(),
            Ok(Error::InvalidBounty)
        );
    }

    #[test]
    fn test_payout_insufficient_balance() {
        let (env, _, client, _, _, user) = setup();
        // Escrow has 0 balance, bounty is 100
        let proof: BytesN<32> = BytesN::from_array(&env, &[9u8; 32]);
        assert_eq!(
            client.try_payout(&user, &proof).unwrap_err(),
            Ok(Error::InsufficientBalance)
        );
    }

    #[test]
    fn test_is_claimed_view() {
        let (env, _, client, _, _, user) = setup();
        client.deposit(&500_i128);
        let proof: BytesN<32> = BytesN::from_array(&env, &[5u8; 32]);
        assert!(!client.is_claimed(&proof));
        client.payout(&user, &proof);
        assert!(client.is_claimed(&proof));
    }

    #[test]
    #[should_panic]
    fn test_initialize_auth_failure_without_municipality_auth() {
        let env = Env::default();
        // NOTE: env.mock_all_auths() is intentionally omitted!
        let municipality = Address::generate(&env);
        let oracle = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let token_id = env
            .register_stellar_asset_contract_v2(token_admin)
            .address();
        let contract_id = env.register_contract(None, NegaWattEscrow);
        let client = NegaWattEscrowClient::new(&env, &contract_id);

        // Calling without municipality authorization must fail
        client.initialize(&municipality, &oracle, &token_id, &100_i128);
    }

    #[test]
    #[should_panic]
    fn test_deposit_auth_failure_without_municipality_auth() {
        let (env, _, client, _, _, _) = setup();
        // Clear mock_all_auths by providing an empty list of mock auths
        env.mock_auths(&[]);
        client.deposit(&100_i128);
    }

    #[test]
    #[should_panic]
    fn test_set_bounty_auth_failure_without_municipality_auth() {
        let (env, _, client, _, _, _) = setup();
        env.mock_auths(&[]);
        client.set_bounty(&300_i128);
    }

    #[test]
    #[should_panic]
    fn test_payout_auth_failure_without_oracle_auth() {
        let (env, _, client, _, _, user) = setup();
        client.deposit(&500_i128);
        let proof = BytesN::from_array(&env, &[8u8; 32]);
        // Restrict auth so oracle signature is absent
        env.mock_auths(&[]);
        client.payout(&user, &proof);
    }

    #[test]
    fn test_get_config_view() {
        let (_, token_id, client, municipality, oracle, _) = setup();
        let config = client.get_config();
        assert_eq!(config.municipality, municipality);
        assert_eq!(config.oracle, oracle);
        assert_eq!(config.token, token_id);
        assert_eq!(config.bounty, 100_i128);
        assert_eq!(config.per_payout_cap, 20_000_000_i128);
        assert_eq!(config.window_len, 86_400);
        assert_eq!(config.window_budget, 200_000_000_i128);
        assert_eq!(config.max_recipient_window_claims, 3);
    }

    #[test]
    fn test_pool_balance_view() {
        let (_, _, client, _, _, _) = setup();
        assert_eq!(client.pool_balance(), client.escrow_balance());
        client.deposit(&2_500_i128);
        assert_eq!(client.pool_balance(), 2_500_i128);
        assert_eq!(client.pool_balance(), client.escrow_balance());
    }

    #[test]
    fn test_is_paused_view() {
        let (_, _, client, _, _, _) = setup();
        assert!(!client.is_paused());
    }

    #[test]
    fn test_window_spent_view() {
        let (_, _, client, _, _, _) = setup();
        assert_eq!(client.window_spent(), 0_i128);
    }

    #[test]
    fn test_get_config_uninitialized_fails() {
        let env = Env::default();
        let contract_id = env.register_contract(None, NegaWattEscrow);
        let client = NegaWattEscrowClient::new(&env, &contract_id);
        assert_eq!(
            client.try_get_config().unwrap_err(),
            Ok(Error::NotInitialized)
        );
    }

    #[test]
    fn test_set_payout_cap_success() {
        let (_, _, client, _, _, _) = setup();
        // Default bounty is 100, budget is 200_000_000
        client.set_payout_cap(&50_000_000_i128);
        assert_eq!(client.get_config().per_payout_cap, 50_000_000_i128);
    }

    #[test]
    #[should_panic]
    fn test_set_payout_cap_auth_failure() {
        let (env, _, client, _, _, _) = setup();
        env.mock_auths(&[]);
        client.set_payout_cap(&50_000_000_i128);
    }

    #[test]
    fn test_set_payout_cap_invalid_or_invariant_fails() {
        let (_, _, client, _, _, _) = setup();
        // Negative / 0 cap
        assert_eq!(
            client.try_set_payout_cap(&0_i128).unwrap_err(),
            Ok(Error::InvalidCap)
        );
        assert_eq!(
            client.try_set_payout_cap(&-1_i128).unwrap_err(),
            Ok(Error::InvalidCap)
        );

        // Invariant: bounty <= new_cap (bounty is 100)
        assert_eq!(
            client.try_set_payout_cap(&99_i128).unwrap_err(),
            Ok(Error::InvariantViolated)
        );

        // Invariant: new_cap <= window_budget (budget is 200_000_000)
        assert_eq!(
            client.try_set_payout_cap(&200_000_001_i128).unwrap_err(),
            Ok(Error::InvariantViolated)
        );
    }

    #[test]
    fn test_bounty_cap_boundary_enforcement() {
        let (_, _, client, _, _, _) = setup();
        // Set cap to exactly 1_000
        client.set_payout_cap(&1_000_i128);

        // Boundary: bounty == cap succeeds
        client.set_bounty(&1_000_i128);
        assert_eq!(client.bounty(), 1_000_i128);

        // Boundary: bounty == cap + 1 fails
        assert_eq!(
            client.try_set_bounty(&1_001_i128).unwrap_err(),
            Ok(Error::InvariantViolated)
        );
    }

    #[test]
    fn test_set_window_budget_success_and_invariants() {
        let (_, _, client, _, _, _) = setup();
        // Cap is 20_000_000
        client.set_window_budget(&500_000_000_i128);
        assert_eq!(client.get_config().window_budget, 500_000_000_i128);

        // Budget <= 0 fails
        assert_eq!(
            client.try_set_window_budget(&0_i128).unwrap_err(),
            Ok(Error::InvalidBudget)
        );

        // Invariant: budget < cap fails
        assert_eq!(
            client.try_set_window_budget(&19_999_999_i128).unwrap_err(),
            Ok(Error::InvariantViolated)
        );
    }

    #[test]
    #[should_panic]
    fn test_set_window_budget_auth_failure() {
        let (env, _, client, _, _, _) = setup();
        env.mock_auths(&[]);
        client.set_window_budget(&500_000_000_i128);
    }

    #[test]
    fn test_window_budget_exhaustion_boundary_and_rollover() {
        let (env, _, client, _, _, user) = setup();
        // Deposit plenty of funds
        client.deposit(&10_000_i128);

        // Lower cap and budget for testing: cap = 100, budget = 200 (exact 2 claims)
        client.set_payout_cap(&100_i128);
        client.set_window_budget(&200_i128);

        let proof1 = BytesN::from_array(&env, &[11u8; 32]);
        let proof2 = BytesN::from_array(&env, &[12u8; 32]);
        let proof3 = BytesN::from_array(&env, &[13u8; 32]);

        // First payout: 100 stroops
        client.payout(&user, &proof1);
        assert_eq!(client.window_spent(), 100_i128);

        // Second payout: 100 stroops -> exact boundary (200 / 200)
        client.payout(&user, &proof2);
        assert_eq!(client.window_spent(), 200_i128);

        // Third payout in same window: exceeds budget
        assert_eq!(
            client.try_payout(&user, &proof3).unwrap_err(),
            Ok(Error::ExceedsWindowBudget)
        );

        // Advance ledger timestamp to next window (+86_400 seconds)
        let current_ts = env.ledger().timestamp();
        env.ledger().set_timestamp(current_ts + 86_400);

        // Window rollover: window_spent resets to 0, unused budget does not roll over
        assert_eq!(client.window_spent(), 0_i128);

        // Third payout now succeeds in the new window!
        client.payout(&user, &proof3);
        assert_eq!(client.window_spent(), 100_i128);
    }

    #[test]
    fn test_set_recipient_claim_limit_success_and_invariants() {
        let (_, _, client, _, _, _) = setup();
        assert_eq!(client.get_config().max_recipient_window_claims, 3);

        client.set_recipient_claim_limit(&5_u32);
        assert_eq!(client.get_config().max_recipient_window_claims, 5);

        // Limit == 0 fails
        assert_eq!(
            client.try_set_recipient_claim_limit(&0_u32).unwrap_err(),
            Ok(Error::InvalidAmount)
        );
    }

    #[test]
    #[should_panic]
    fn test_set_recipient_claim_limit_auth_failure() {
        let (env, _, client, _, _, _) = setup();
        env.mock_auths(&[]);
        client.set_recipient_claim_limit(&5_u32);
    }

    #[test]
    fn test_recipient_claim_limit_hit_and_different_recipients_unaffected() {
        let (env, _, client, _, _, user1) = setup();
        let user2 = Address::generate(&env);
        client.deposit(&5_000_i128);

        let p1 = BytesN::from_array(&env, &[21u8; 32]);
        let p2 = BytesN::from_array(&env, &[22u8; 32]);
        let p3 = BytesN::from_array(&env, &[23u8; 32]);
        let p4 = BytesN::from_array(&env, &[24u8; 32]);
        let p5 = BytesN::from_array(&env, &[25u8; 32]);

        // Default limit is 3 claims per window
        client.payout(&user1, &p1);
        assert_eq!(client.recipient_window_claims(&user1), 1);
        client.payout(&user1, &p2);
        assert_eq!(client.recipient_window_claims(&user1), 2);
        client.payout(&user1, &p3);
        assert_eq!(client.recipient_window_claims(&user1), 3);

        // 4th claim for user1 fails
        assert_eq!(
            client.try_payout(&user1, &p4).unwrap_err(),
            Ok(Error::ExceedsRecipientWindowLimit)
        );

        // User2 is unaffected by user1's limit!
        client.payout(&user2, &p5);
        assert_eq!(client.recipient_window_claims(&user2), 1);
        assert_eq!(client.recipient_window_claims(&user1), 3);
    }

    #[test]
    fn test_recipient_claim_limit_reset_on_rollover() {
        let (env, _, client, _, _, user) = setup();
        client.deposit(&5_000_i128);

        let p1 = BytesN::from_array(&env, &[31u8; 32]);
        let p2 = BytesN::from_array(&env, &[32u8; 32]);
        let p3 = BytesN::from_array(&env, &[33u8; 32]);
        let p4 = BytesN::from_array(&env, &[34u8; 32]);

        client.payout(&user, &p1);
        client.payout(&user, &p2);
        client.payout(&user, &p3);
        assert_eq!(client.recipient_window_claims(&user), 3);

        // 4th claim fails in same window
        assert_eq!(
            client.try_payout(&user, &p4).unwrap_err(),
            Ok(Error::ExceedsRecipientWindowLimit)
        );

        // Rollover to next window (+86_400 seconds)
        let ts = env.ledger().timestamp();
        env.ledger().set_timestamp(ts + 86_400);

        // Claims counter in new window is 0
        assert_eq!(client.recipient_window_claims(&user), 0);

        // Payout succeeds in new window
        client.payout(&user, &p4);
        assert_eq!(client.recipient_window_claims(&user), 1);
    }

    #[test]
    fn test_pause_and_unpause_success() {
        let (_, _, client, _, _, _) = setup();
        assert!(!client.is_paused());

        client.pause();
        assert!(client.is_paused());

        client.unpause();
        assert!(!client.is_paused());
    }

    #[test]
    #[should_panic]
    fn test_pause_auth_failure() {
        let (env, _, client, _, _, _) = setup();
        env.mock_auths(&[]);
        client.pause();
    }

    #[test]
    #[should_panic]
    fn test_unpause_auth_failure() {
        let (env, _, client, _, _, _) = setup();
        client.pause();
        env.mock_auths(&[]);
        client.unpause();
    }

    #[test]
    fn test_pause_blocks_payout_but_allows_deposit() {
        let (env, _, client, _, _, user) = setup();
        client.deposit(&2_000_i128);

        let proof1 = BytesN::from_array(&env, &[41u8; 32]);

        // Pause contract
        client.pause();
        assert!(client.is_paused());

        // Payout fails while paused
        assert_eq!(
            client.try_payout(&user, &proof1).unwrap_err(),
            Ok(Error::ContractPaused)
        );

        // Deposit is STILL allowed while paused
        client.deposit(&1_000_i128);
        assert_eq!(client.pool_balance(), 3_000_i128);

        // Unpause
        client.unpause();
        assert!(!client.is_paused());

        // Payout succeeds after unpause
        client.payout(&user, &proof1);
        assert_eq!(client.pool_balance(), 2_900_i128);
    }

    #[test]
    fn test_pause_during_exhausted_budget() {
        let (env, _, client, _, _, user) = setup();
        client.deposit(&5_000_i128);

        client.set_payout_cap(&100_i128);
        client.set_window_budget(&100_i128);

        let proof1 = BytesN::from_array(&env, &[51u8; 32]);
        let proof2 = BytesN::from_array(&env, &[52u8; 32]);

        // Payout consumes whole budget (100 stroops)
        client.payout(&user, &proof1);
        assert_eq!(client.window_spent(), 100_i128);

        // Budget is now exhausted
        assert_eq!(
            client.try_payout(&user, &proof2).unwrap_err(),
            Ok(Error::ExceedsWindowBudget)
        );

        // Municipality pauses contract while budget is exhausted
        client.pause();
        assert!(client.is_paused());

        // Attempting payout now returns ContractPaused
        assert_eq!(
            client.try_payout(&user, &proof2).unwrap_err(),
            Ok(Error::ContractPaused)
        );

        // Advance ledger timestamp to next window (+86_400 seconds)
        let ts = env.ledger().timestamp();
        env.ledger().set_timestamp(ts + 86_400);

        // Budget rolled over, but contract is STILL paused
        assert_eq!(client.window_spent(), 0_i128);
        assert_eq!(
            client.try_payout(&user, &proof2).unwrap_err(),
            Ok(Error::ContractPaused)
        );

        // Unpause: payout immediately succeeds
        client.unpause();
        client.payout(&user, &proof2);
        assert_eq!(client.window_spent(), 100_i128);
    }

    #[test]
    fn test_set_oracle_success_and_config() {
        let (env, _, client, _, old_oracle, user) = setup();
        client.deposit(&5_000_i128);

        assert_eq!(client.get_config().oracle, old_oracle);

        let new_oracle = Address::generate(&env);
        client.set_oracle(&new_oracle);

        assert_eq!(client.get_config().oracle, new_oracle);

        let proof = BytesN::from_array(&env, &[61u8; 32]);
        client.payout(&user, &proof);
        assert_eq!(client.pool_balance(), 4_900_i128);
    }

    #[test]
    #[should_panic]
    fn test_set_oracle_auth_failure_non_municipality() {
        let (env, _, client, _, _, _) = setup();
        let new_oracle = Address::generate(&env);
        env.mock_auths(&[]);
        client.set_oracle(&new_oracle);
    }

    #[test]
    #[should_panic]
    fn test_set_oracle_old_key_rejected() {
        let (env, _, client, _, old_oracle, user) = setup();
        client.deposit(&5_000_i128);

        let new_oracle = Address::generate(&env);
        client.set_oracle(&new_oracle);

        let proof = BytesN::from_array(&env, &[62u8; 32]);

        use soroban_sdk::testutils::{MockAuth, MockAuthInvoke};
        use soroban_sdk::IntoVal;

        env.mock_auths(&[MockAuth {
            address: &old_oracle,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "payout",
                args: (user.clone(), proof.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }]);

        client.payout(&user, &proof);
    }

    #[test]
    fn test_set_oracle_new_key_accepted() {
        let (env, _, client, _, _, user) = setup();
        client.deposit(&5_000_i128);

        let new_oracle = Address::generate(&env);
        client.set_oracle(&new_oracle);

        let proof = BytesN::from_array(&env, &[63u8; 32]);

        use soroban_sdk::testutils::{MockAuth, MockAuthInvoke};
        use soroban_sdk::IntoVal;

        env.mock_auths(&[MockAuth {
            address: &new_oracle,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "payout",
                args: (user.clone(), proof.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }]);

        client.payout(&user, &proof);
        assert_eq!(client.pool_balance(), 4_900_i128);
    }

    #[test]
    fn test_withdraw_success() {
        let (env, token_id, client, _, _, recipient) = setup();
        client.deposit(&5_000_i128);
        assert_eq!(client.pool_balance(), 5_000_i128);

        client.withdraw(&recipient, &2_000_i128);
        assert_eq!(client.pool_balance(), 3_000_i128);

        let token = TokenClient::new(&env, &token_id);
        assert_eq!(token.balance(&recipient), 2_000_i128);
    }

    #[test]
    fn test_withdraw_over_withdraw_fails() {
        let (_, _, client, _, _, recipient) = setup();
        client.deposit(&1_000_i128);

        assert_eq!(
            client.try_withdraw(&recipient, &1_001_i128).unwrap_err(),
            Ok(Error::InsufficientBalance)
        );
    }

    #[test]
    fn test_withdraw_invalid_amount_fails() {
        let (_, _, client, _, _, recipient) = setup();
        client.deposit(&1_000_i128);

        assert_eq!(
            client.try_withdraw(&recipient, &0_i128).unwrap_err(),
            Ok(Error::InvalidAmount)
        );
        assert_eq!(
            client.try_withdraw(&recipient, &-50_i128).unwrap_err(),
            Ok(Error::InvalidAmount)
        );
    }

    #[test]
    #[should_panic]
    fn test_withdraw_wrong_caller_fails() {
        let (env, _, client, _, _, recipient) = setup();
        client.deposit(&2_000_i128);
        env.mock_auths(&[]);
        client.withdraw(&recipient, &1_000_i128);
    }

    #[test]
    fn test_withdraw_while_paused() {
        let (env, token_id, client, _, _, recipient) = setup();
        client.deposit(&4_000_i128);

        // Pause contract
        client.pause();
        assert!(client.is_paused());

        // Withdraw must succeed even while paused
        client.withdraw(&recipient, &1_500_i128);
        assert_eq!(client.pool_balance(), 2_500_i128);

        let token = TokenClient::new(&env, &token_id);
        assert_eq!(token.balance(&recipient), 1_500_i128);
    }

    #[test]
    fn test_events_emitted_for_all_operations() {
        let env = Env::default();
        env.mock_all_auths();

        let municipality = Address::generate(&env);
        let oracle = Address::generate(&env);
        let user = Address::generate(&env);

        let token_admin = Address::generate(&env);
        let token_id = env
            .register_stellar_asset_contract_v2(token_admin)
            .address();
        StellarAssetClient::new(&env, &token_id).mint(&municipality, &10_000_i128);

        let contract_id = env.register_contract(None, NegaWattEscrow);
        let client = NegaWattEscrowClient::new(&env, &contract_id);

        use soroban_sdk::TryFromVal;

        // 1. Initialize event
        client.initialize(&municipality, &oracle, &token_id, &100_i128);
        let init_event = env.events().all().last().unwrap();
        assert_eq!(init_event.0, contract_id);
        let sym = Symbol::try_from_val(&env, &init_event.1.get(0).unwrap()).unwrap();
        assert_eq!(sym, Symbol::new(&env, EVENT_INITIALIZED));
        let init_data: (Address, Address, i128) =
            <(Address, Address, i128)>::try_from_val(&env, &init_event.2).unwrap();
        assert_eq!(init_data, (oracle.clone(), token_id.clone(), 100_i128));

        // 2. Deposit event
        client.deposit(&5_000_i128);
        let dep_event = env.events().all().last().unwrap();
        assert_eq!(dep_event.0, contract_id);
        let sym = Symbol::try_from_val(&env, &dep_event.1.get(0).unwrap()).unwrap();
        assert_eq!(sym, Symbol::new(&env, EVENT_DEPOSITED));
        let dep_amount = i128::try_from_val(&env, &dep_event.2).unwrap();
        assert_eq!(dep_amount, 5_000_i128);

        // 3. Set bounty & config updated event
        client.set_bounty(&150_i128);
        let cfg_event = env.events().all().last().unwrap();
        assert_eq!(cfg_event.0, contract_id);
        let sym = Symbol::try_from_val(&env, &cfg_event.1.get(0).unwrap()).unwrap();
        assert_eq!(sym, Symbol::new(&env, EVENT_CONFIG_UPDATED));
        let cfg_data = Config::try_from_val(&env, &cfg_event.2).unwrap();
        assert_eq!(cfg_data.bounty, 150_i128);

        // 4. Set payout cap event
        client.set_payout_cap(&500_i128);
        let cap_event = env.events().all().last().unwrap();
        let sym = Symbol::try_from_val(&env, &cap_event.1.get(0).unwrap()).unwrap();
        assert_eq!(sym, Symbol::new(&env, EVENT_CONFIG_UPDATED));
        let cap_data = Config::try_from_val(&env, &cap_event.2).unwrap();
        assert_eq!(cap_data.per_payout_cap, 500_i128);

        // 5. Set window budget event
        client.set_window_budget(&1_000_i128);
        let bud_event = env.events().all().last().unwrap();
        let sym = Symbol::try_from_val(&env, &bud_event.1.get(0).unwrap()).unwrap();
        assert_eq!(sym, Symbol::new(&env, EVENT_CONFIG_UPDATED));
        let bud_data = Config::try_from_val(&env, &bud_event.2).unwrap();
        assert_eq!(bud_data.window_budget, 1_000_i128);

        // 6. Set recipient claim limit event
        client.set_recipient_claim_limit(&5_u32);
        let rcl_event = env.events().all().last().unwrap();
        let sym = Symbol::try_from_val(&env, &rcl_event.1.get(0).unwrap()).unwrap();
        assert_eq!(sym, Symbol::new(&env, EVENT_CONFIG_UPDATED));
        let rcl_data = Config::try_from_val(&env, &rcl_event.2).unwrap();
        assert_eq!(rcl_data.max_recipient_window_claims, 5);

        // 7. Pause event
        client.pause();
        let pause_event = env.events().all().last().unwrap();
        let sym = Symbol::try_from_val(&env, &pause_event.1.get(0).unwrap()).unwrap();
        assert_eq!(sym, Symbol::new(&env, EVENT_PAUSED));

        // 8. Unpause event
        client.unpause();
        let unpause_event = env.events().all().last().unwrap();
        let sym = Symbol::try_from_val(&env, &unpause_event.1.get(0).unwrap()).unwrap();
        assert_eq!(sym, Symbol::new(&env, EVENT_UNPAUSED));

        // 9. Oracle rotation event
        let new_oracle = Address::generate(&env);
        client.set_oracle(&new_oracle);
        let rot_event = env.events().all().last().unwrap();
        let sym = Symbol::try_from_val(&env, &rot_event.1.get(0).unwrap()).unwrap();
        assert_eq!(sym, Symbol::new(&env, EVENT_ORACLE_ROTATED));
        let rot_data = <(Address, Address)>::try_from_val(&env, &rot_event.2).unwrap();
        assert_eq!(rot_data, (oracle.clone(), new_oracle.clone()));

        // 10. Payout event (with recipient, amount, proof hash)
        let proof = BytesN::from_array(&env, &[77u8; 32]);
        client.payout(&user, &proof);
        let payout_event = env.events().all().last().unwrap();
        let sym = Symbol::try_from_val(&env, &payout_event.1.get(0).unwrap()).unwrap();
        assert_eq!(sym, Symbol::new(&env, EVENT_PAYOUT));
        let (payout_amount, payout_proof): (i128, BytesN<32>) =
            <(i128, BytesN<32>)>::try_from_val(&env, &payout_event.2).unwrap();
        assert_eq!(payout_amount, 150_i128);
        assert_eq!(payout_proof, proof);

        // 11. Withdraw event
        let dest = Address::generate(&env);
        client.withdraw(&dest, &1_000_i128);
        let withdraw_event = env.events().all().last().unwrap();
        let sym = Symbol::try_from_val(&env, &withdraw_event.1.get(0).unwrap()).unwrap();
        assert_eq!(sym, Symbol::new(&env, EVENT_WITHDRAWN));
        let withdraw_amount = i128::try_from_val(&env, &withdraw_event.2).unwrap();
        assert_eq!(withdraw_amount, 1_000_i128);
    }

    #[test]
    fn test_persistent_replay_hash_ttl_survives_ledger_advancement() {
        let (env, _, client, _, _, user) = setup();
        client.deposit(&5_000_i128);

        let proof = BytesN::from_array(&env, &[88u8; 32]);
        client.payout(&user, &proof);
        assert!(client.is_claimed(&proof));

        // Advance ledger sequence by 100,000 ledgers (far beyond default 4,096 threshold)
        let current_seq = env.ledger().sequence();
        env.ledger().set_sequence_number(current_seq + 100_000);

        // The claim entry must STILL exist and survive due to extend_ttl
        assert!(client.is_claimed(&proof));

        // Replay attempt after 100,000 ledgers is still rejected
        assert_eq!(
            client.try_payout(&user, &proof).unwrap_err(),
            Ok(Error::ReplayClaim)
        );
    }
}
