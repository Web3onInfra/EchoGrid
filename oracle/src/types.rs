//! # EchoGrid Oracle — Shared Types
//!
//! All JSON-serialisable structs shared between the HTTP layer and the
//! Stellar interaction layer live here.  Keeping them in one place makes
//! it trivial to generate OpenAPI schemas or a TypeScript client later.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// ── Inbound: Proof-of-Power-Down Payload ────────────────────────────────────
// ---------------------------------------------------------------------------

/// The JSON body that the Acoustic CLI (or a real phone app) sends to the
/// oracle when it believes a power-down event has occurred.
///
/// Fields are deliberately minimal for the MVP — in production you'd also
/// include device attestation tokens, GPS region codes, etc.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProofPayload {
    /// Stellar address of the user claiming the NegaWatt bounty.
    pub wallet_address: String,

    /// Unique challenge ID issued by the municipality for this grid event.
    /// Prevents proofs from one event being submitted against another.
    pub challenge_id: String,

    /// ISO-8601 UTC timestamp of the detected power-down, e.g. "2024-10-27T18:00:00Z".
    /// Part of the pre-image for the proof hash so each event is unique.
    pub timestamp: String,

    /// Hex-encoded SHA-256 hash — the Proof-of-Power-Down.
    /// Pre-image: SHA-256(wallet_address || challenge_id || timestamp || nonce)
    pub proof_hash: String,

    /// 8-byte random nonce (hex-encoded) mixed into the hash to prevent
    /// grinding attacks where a bad actor pre-computes hashes for known wallets.
    pub nonce: String,
}

// ---------------------------------------------------------------------------
// ── Outbound: Oracle API Response ───────────────────────────────────────────
// ---------------------------------------------------------------------------

/// Standard envelope for every oracle API response.
#[derive(Debug, Serialize)]
pub struct OracleResponse {
    pub success: bool,
    pub message: String,
    /// Optional: Stellar transaction hash of the payout, if one was triggered.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_hash: Option<String>,
}

impl OracleResponse {
    pub fn ok(msg: impl Into<String>, tx_hash: Option<String>) -> Self {
        Self {
            success: true,
            message: msg.into(),
            tx_hash,
        }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            message: msg.into(),
            tx_hash: None,
        }
    }
}

// ---------------------------------------------------------------------------
// ── Internal: Verification Result ───────────────────────────────────────────
// ---------------------------------------------------------------------------

/// Returned by the proof verifier after cryptographic validation.
pub struct VerificationResult {
    /// True if the hash is mathematically valid.
    pub valid: bool,
    /// Human-readable reason for rejection (empty on success).
    pub reason: String,
}
