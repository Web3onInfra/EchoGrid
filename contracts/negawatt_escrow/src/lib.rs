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
}

// ---------------------------------------------------------------------------
// ── Events ───────────────────────────────────────────────────────────────────
// ---------------------------------------------------------------------------

/// Emitted when the municipality deposits USDC into escrow.
const EVENT_DEPOSITED: &str = "negawatt_deposited";
/// Emitted for every successful NegaWatt payout to a user.
const EVENT_PAYOUT: &str = "negawatt_payout";
/// Emitted when the bounty rate is updated.
const EVENT_BOUNTY_SET: &str = "negawatt_bounty_set";

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
            (Symbol::new(&env, EVENT_BOUNTY_SET), municipality),
            new_bounty,
        );

        Ok(())
    }

    /// **Set Payout Cap** — Update the per-payout maximum ceiling.
    ///
    /// Municipality-only. Enforces invariant: bounty <= new_cap <= window_budget.
    pub fn set_payout_cap(env: Env, new_cap: i128) -> Result<(), Error> {
        let _municipality = Self::require_municipality_role(&env)?;

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

        Ok(())
    }

    /// **Set Window Budget** — Update the pool-level payout budget per window.
    ///
    /// Municipality-only. Enforces invariant: per_payout_cap <= new_budget.
    pub fn set_window_budget(env: Env, new_budget: i128) -> Result<(), Error> {
        let _municipality = Self::require_municipality_role(&env)?;

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

        // Mark this proof as spent so it can never be replayed.
        // We use `persistent` storage so it survives ledger expiry TTLs.
        env.storage().persistent().set(&claim_key, &true);

        // Extend the TTL of the claim record for ~1 year (≈ 6M ledgers @ 5s/ledger).
        // Without this, the entry would expire and a replay could succeed.
        env.storage()
            .persistent()
            .extend_ttl(&claim_key, 6_000_000, 6_000_000);

        // Emit payout event — indexers will use this to build the user dashboard.
        env.events().publish(
            (Symbol::new(&env, EVENT_PAYOUT), oracle, recipient.clone()),
            bounty,
        );

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
}

// ---------------------------------------------------------------------------
// ── Tests ────────────────────────────────────────────────────────────────────
// ---------------------------------------------------------------------------

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Ledger as _},
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
}
