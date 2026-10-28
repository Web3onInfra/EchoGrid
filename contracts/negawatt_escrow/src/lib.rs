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
    contract, contractimpl, contracttype, token::Client as TokenClient, Address, BytesN, Env,
    Symbol,
};

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
    ) {
        // Guard: prevent re-initialisation which could overwrite the oracle address.
        if env
            .storage()
            .instance()
            .get::<_, bool>(&DataKey::Initialized)
            .unwrap_or(false)
        {
            panic!("EchoGrid: contract already initialized");
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
    pub fn deposit(env: Env, amount: i128) {
        // ── AgentGuard: ROLE_MUNICIPALITY ───────────────────────────────────
        let municipality = Self::require_municipality_role(&env);

        // Validate deposit amount — negative deposits would drain the contract.
        if amount <= 0 {
            panic!("EchoGrid: deposit amount must be positive");
        }

        let token_id: Address = env
            .storage()
            .instance()
            .get(&DataKey::TokenId)
            .expect("EchoGrid: contract not initialized");

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
    }

    /// **Set Bounty** — Update the per-claim NegaWatt payout rate.
    ///
    /// Allows the municipality to react to grid conditions — higher bounties
    /// during peak demand, lower during off-peak.
    pub fn set_bounty(env: Env, new_bounty: i128) {
        // ── AgentGuard: ROLE_MUNICIPALITY ───────────────────────────────────
        let municipality = Self::require_municipality_role(&env);

        if new_bounty <= 0 {
            panic!("EchoGrid: bounty must be positive");
        }

        env.storage()
            .instance()
            .set(&DataKey::BountyPerClaim, &new_bounty);

        env.events().publish(
            (Symbol::new(&env, EVENT_BOUNTY_SET), municipality),
            new_bounty,
        );
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
    pub fn payout(env: Env, recipient: Address, proof_hash: BytesN<32>) {
        // ── AgentGuard: ROLE_ORACLE ─────────────────────────────────────────
        let oracle = Self::require_oracle_role(&env);

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
            panic!("EchoGrid: proof already claimed — replay attack blocked");
        }

        let token_id: Address = env
            .storage()
            .instance()
            .get(&DataKey::TokenId)
            .expect("EchoGrid: contract not initialized");

        // NegaWatt payout triggered here ─────────────────────────────────────
        let bounty: i128 = env
            .storage()
            .instance()
            .get(&DataKey::BountyPerClaim)
            .expect("EchoGrid: bounty not set");

        // Sanity-check: make sure the escrow still has enough funds.
        let balance = TokenClient::new(&env, &token_id).balance(&env.current_contract_address());
        if balance < bounty {
            panic!("EchoGrid: insufficient escrow balance — municipality must top up");
        }

        // Transfer the NegaWatt bounty from escrow → user wallet.
        TokenClient::new(&env, &token_id).transfer(
            &env.current_contract_address(), // from: escrow
            &recipient,                      // to: verified user
            &bounty,
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
    }

    // -----------------------------------------------------------------------
    // ── Read-Only Views ──────────────────────────────────────────────────────
    // -----------------------------------------------------------------------

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
    fn require_municipality_role(env: &Env) -> Address {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::MunicipalityAdmin)
            .expect("EchoGrid: not initialized");
        // require_auth() aborts the transaction if the signature is invalid.
        admin.require_auth();
        admin
    }

    /// Retrieves the `OracleAddress` and calls `require_auth()`.
    /// This ensures only the Rust AgentPay backend (holding the oracle keypair)
    /// can trigger payouts — no user can call `payout` directly.
    fn require_oracle_role(env: &Env) -> Address {
        let oracle: Address = env
            .storage()
            .instance()
            .get(&DataKey::OracleAddress)
            .expect("EchoGrid: not initialized");
        oracle.require_auth();
        oracle
    }
}

// ---------------------------------------------------------------------------
// ── Tests ────────────────────────────────────────────────────────────────────
// ---------------------------------------------------------------------------

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, token::StellarAssetClient, Address, BytesN, Env};

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
    #[should_panic(expected = "proof already claimed")]
    fn test_replay_attack_blocked() {
        let (env, _, client, _, _, user) = setup();

        client.deposit(&1_000_i128);
        let proof: BytesN<32> = BytesN::from_array(&env, &[2u8; 32]);

        // First claim — should succeed.
        client.payout(&user, &proof);

        // Second claim with the same proof — must panic.
        client.payout(&user, &proof);
    }

    #[test]
    #[should_panic(expected = "contract already initialized")]
    fn test_reinit_blocked() {
        let (_, _, client, municipality, oracle, _) = setup();
        let fake_token = municipality.clone();
        // Attempting to re-initialize must be rejected.
        client.initialize(&municipality, &oracle, &fake_token, &999_i128);
    }

    #[test]
    fn test_set_bounty() {
        let (_, _, client, _, _, _) = setup();
        client.set_bounty(&250_i128);
        assert_eq!(client.bounty(), 250_i128);
    }
}
