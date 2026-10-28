use serde::{Deserialize, Serialize};

/// Error taxonomy mapping on-chain Soroban failures and RPC errors into
/// HTTP-friendly status codes for the oracle service.
#[allow(dead_code)]
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

#[allow(dead_code)]
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
            || lower.contains("account not found")
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
