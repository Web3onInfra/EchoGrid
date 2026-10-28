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
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
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

// ---------------------------------------------------------------------------
// ── Escrow & Soroban Error Taxonomy ─────────────────────────────────────────
// ---------------------------------------------------------------------------

/// Error taxonomy mapping on-chain Soroban failures and RPC errors into
/// HTTP-friendly status codes for the oracle service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EscrowError {
    /// The proof hash was already claimed and recorded in contract persistent storage.
    ReplayDetected,

    /// The transaction caller or signer lacks the authorized oracle role.
    UnauthorizedSigner,

    /// Escrow contract balance is lower than the required bounty amount.
    InsufficientPoolBalance,

    /// The contract was not properly initialized or missing token configuration.
    ContractNotInitialized,

    /// Proof hash argument is malformed (not 32-byte hex).
    InvalidProofFormat,

    /// Soroban RPC node timed out waiting for ledger inclusion.
    RpcTimeout,

    /// Soroban RPC endpoint is unreachable or returned HTTP network failure.
    RpcUnavailable,

    /// Transaction simulation failed before submission.
    SimulationFailed(String),

    /// Unclassified or internal error.
    Unknown(String),
}

impl EscrowError {
    /// Maps each failure mode to standard HTTP status codes.
    pub fn http_status(&self) -> u16 {
        match self {
            Self::ReplayDetected => 409,          // 409 Conflict
            Self::UnauthorizedSigner => 403,      // 403 Forbidden
            Self::InsufficientPoolBalance => 503, // 503 Service Unavailable
            Self::InvalidProofFormat => 400,      // 400 Bad Request
            Self::RpcTimeout => 504,              // 504 Gateway Timeout
            Self::RpcUnavailable => 502,          // 502 Bad Gateway
            Self::SimulationFailed(_) => 422,     // 422 Unprocessable Entity
            Self::ContractNotInitialized => 500,  // 500 Internal Server Error
            Self::Unknown(_) => 500,              // 500 Internal Server Error
        }
    }

    /// Parses error logs, Soroban simulation outputs, or CLI errors into the taxonomy.
    pub fn from_error_str(err: &str) -> Self {
        let lower = err.to_lowercase();
        if lower.contains("proof already claimed")
            || lower.contains("already claimed")
            || lower.contains("unreachablecodereached")
        {
            Self::ReplayDetected
        } else if lower.contains("require_auth")
            || lower.contains("unauthorized")
            || lower.contains("bad auth")
            || lower.contains("no private key")
            || lower.contains("missing auth")
        {
            Self::UnauthorizedSigner
        } else if lower.contains("insufficient escrow balance")
            || lower.contains("insufficient balance")
            || lower.contains("insufficient funds")
        {
            Self::InsufficientPoolBalance
        } else if lower.contains("not initialized") {
            Self::ContractNotInitialized
        } else if lower.contains("timeout") || lower.contains("deadline") {
            Self::RpcTimeout
        } else if lower.contains("connection refused")
            || lower.contains("forbidden")
            || lower.contains("unavailable")
        {
            Self::RpcUnavailable
        } else if lower.contains("simulation failed") {
            Self::SimulationFailed(err.to_string())
        } else {
            Self::Unknown(err.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_taxonomy_status_codes() {
        assert_eq!(EscrowError::ReplayDetected.http_status(), 409);
        assert_eq!(EscrowError::UnauthorizedSigner.http_status(), 403);
        assert_eq!(EscrowError::InsufficientPoolBalance.http_status(), 503);
        assert_eq!(EscrowError::InvalidProofFormat.http_status(), 400);
        assert_eq!(EscrowError::RpcTimeout.http_status(), 504);
        assert_eq!(EscrowError::RpcUnavailable.http_status(), 502);
        assert_eq!(
            EscrowError::SimulationFailed("fail".into()).http_status(),
            422
        );
        assert_eq!(EscrowError::ContractNotInitialized.http_status(), 500);
        assert_eq!(EscrowError::Unknown("other".into()).http_status(), 500);
    }

    #[test]
    fn test_from_error_str_patterns() {
        assert_eq!(
            EscrowError::from_error_str(
                "Error(WasmVm, InvalidAction): VM call trapped: UnreachableCodeReached"
            ),
            EscrowError::ReplayDetected
        );
        assert_eq!(
            EscrowError::from_error_str(
                "panic: EchoGrid: proof already claimed — replay attack blocked"
            ),
            EscrowError::ReplayDetected
        );
        assert_eq!(
            EscrowError::from_error_str("HostError: require_auth failed for address"),
            EscrowError::UnauthorizedSigner
        );
        assert_eq!(
            EscrowError::from_error_str(
                "HostError: EchoGrid: insufficient escrow balance — municipality must top up"
            ),
            EscrowError::InsufficientPoolBalance
        );
        assert_eq!(
            EscrowError::from_error_str("Timeout waiting for transaction confirmation"),
            EscrowError::RpcTimeout
        );
    }
}
