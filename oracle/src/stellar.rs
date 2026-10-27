//! # Stellar Interaction Layer — AgentPay
//!
//! This module handles everything Stellar-related:
//!   1. Building the Soroban contract invocation XDR.
//!   2. Signing with the oracle keypair.
//!   3. Submitting to the Horizon REST API and awaiting confirmation.
//!
//! For the hackathon MVP the real network call is feature-gated behind
//! `--features live-network`.  In mock mode the function returns a fake
//! transaction hash so judges can see the full flow without a Testnet key.

use anyhow::Result;
use tracing::{info, warn};

use crate::types::ProofPayload;

/// Environment variables expected when `live-network` is active.
pub const ENV_ORACLE_SECRET: &str = "ORACLE_SECRET_KEY";    // S...
pub const ENV_CONTRACT_ID:   &str = "CONTRACT_ID";           // C...
pub const ENV_HORIZON_URL:   &str = "HORIZON_URL";           // https://horizon-testnet.stellar.org
pub const ENV_TOKEN_ID:      &str = "TOKEN_CONTRACT_ID";     // C... (USDC-T contract)

// ---------------------------------------------------------------------------
// ── Public entry point ───────────────────────────────────────────────────────
// ---------------------------------------------------------------------------

/// **AgentPay** — triggers the NegaWatt payout on the Soroban contract.
///
/// Accepts the verified `ProofPayload` and the raw `proof_hash` bytes.
/// Returns the Stellar transaction hash on success.
///
/// # Mock mode (default for hackathon MVP)
/// Returns a deterministic fake TX hash constructed from the proof hash so
/// judges can verify the full pipeline runs end-to-end.
pub async fn trigger_payout(payload: &ProofPayload, proof_hash_bytes: [u8; 32]) -> Result<String> {
    #[cfg(feature = "live-network")]
    {
        live::invoke_soroban_payout(payload, proof_hash_bytes).await
    }

    #[cfg(not(feature = "live-network"))]
    {
        mock::simulate_payout(payload, proof_hash_bytes).await
    }
}

// ---------------------------------------------------------------------------
// ── Mock implementation ──────────────────────────────────────────────────────
// ---------------------------------------------------------------------------

#[cfg(not(feature = "live-network"))]
mod mock {
    use super::*;

    pub async fn simulate_payout(
        payload: &ProofPayload,
        proof_hash_bytes: [u8; 32],
    ) -> Result<String> {
        // Simulate network latency so the demo feels realistic.
        tokio::time::sleep(std::time::Duration::from_millis(350)).await;

        // Construct a fake but deterministic TX hash from the proof material.
        let fake_tx = format!(
            "MOCK_TX_{}_{}",
            hex::encode(&proof_hash_bytes[..8]),
            &payload.wallet_address[..6]
        );

        info!(
            wallet    = %payload.wallet_address,
            challenge = %payload.challenge_id,
            tx        = %fake_tx,
            "🟢 [MOCK] NegaWatt payout simulated — Soroban invocation would fire here"
        );

        Ok(fake_tx)
    }
}

// ---------------------------------------------------------------------------
// ── Live network implementation ──────────────────────────────────────────────
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Live-network Soroban invocation stub
// ---------------------------------------------------------------------------
// When you're ready to connect to Stellar Testnet:
//
//  1. Add to oracle/Cargo.toml under [dependencies]:
//       stellar-xdr = "0.0.16"
//       reqwest     = { version = "0.11", features = ["json"] }
//
//  2. Enable the feature in oracle/Cargo.toml [features]:
//       live-network = ["stellar-xdr", "reqwest"]
//
//  3. Fill in the XDR-building logic using the pattern below.
//
// High-level steps for a real Soroban invocation:
//
//   a. Load oracle account sequence from Horizon:
//        GET https://horizon-testnet.stellar.org/accounts/{oracle_public_key}
//
//   b. Build an InvokeHostFunctionOp XDR for the `payout` function:
//        - ScVal::Address(recipient)
//        - ScVal::Bytes(proof_hash_32_bytes)
//
//   c. Simulate via Soroban-RPC simulateTransaction to get resource limits.
//
//   d. Set the auth + resource fees from simulation, sign with oracle keypair,
//      and submit via Horizon POST /transactions.
//
//   e. Poll for PENDING → SUCCESS / FAILED.
//
// The mock implementation above covers all other layers of the pipeline.
#[cfg(feature = "live-network")]
mod live {
    use super::*;

    pub async fn invoke_soroban_payout(
        payload: &ProofPayload,
        _proof_hash_bytes: [u8; 32],
    ) -> Result<String> {
        // TODO: implement using stellar-xdr + reqwest once deps are added.
        // See the comment block above for the step-by-step approach.
        anyhow::bail!(
            "live-network Soroban invocation not yet wired up. \
             See stellar.rs for implementation guide."
        )
    }
}
