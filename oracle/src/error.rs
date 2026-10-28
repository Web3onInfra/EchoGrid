//! # EchoGrid Oracle — API Error Model
//!
//! Provides a unified, machine-readable JSON error format for all oracle endpoints,
//! with complete mappings for smart contract errors (1..14) from Phase 1.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

/// Standardized JSON response payload for all HTTP errors across the Oracle API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiErrorBody {
    /// Stable machine-readable uppercase error identifier (e.g. "REPLAY_CLAIM").
    pub code: String,
    /// Human-readable description of the error.
    pub message: String,
    /// HTTP status code mirror.
    pub status: u16,
    /// Optional additional contextual details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

/// Unified domain error type for the oracle service.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum ApiError {
    #[error("{message}")]
    BadRequest { code: String, message: String },

    #[error("{message}")]
    Unauthorized { code: String, message: String },

    #[error("{message}")]
    NotFound { code: String, message: String },

    #[error("{message}")]
    Conflict { code: String, message: String },

    #[error("{message}")]
    UnprocessableEntity { code: String, message: String },

    #[error("{message}")]
    TooManyRequests { code: String, message: String },

    #[error("{message}")]
    ServiceUnavailable { code: String, message: String },

    #[error("{message}")]
    Internal { code: String, message: String },

    #[error("Contract error ({contract_code}): {message}")]
    ContractError {
        contract_code: u32,
        code: String,
        message: String,
        status: StatusCode,
    },
}

impl ApiError {
    pub fn bad_request(code: impl Into<String>, msg: impl Into<String>) -> Self {
        Self::BadRequest {
            code: code.into(),
            message: msg.into(),
        }
    }

    pub fn unauthorized(code: impl Into<String>, msg: impl Into<String>) -> Self {
        Self::Unauthorized {
            code: code.into(),
            message: msg.into(),
        }
    }

    pub fn not_found(code: impl Into<String>, msg: impl Into<String>) -> Self {
        Self::NotFound {
            code: code.into(),
            message: msg.into(),
        }
    }

    pub fn conflict(code: impl Into<String>, msg: impl Into<String>) -> Self {
        Self::Conflict {
            code: code.into(),
            message: msg.into(),
        }
    }

    pub fn unprocessable_entity(code: impl Into<String>, msg: impl Into<String>) -> Self {
        Self::UnprocessableEntity {
            code: code.into(),
            message: msg.into(),
        }
    }

    pub fn too_many_requests(code: impl Into<String>, msg: impl Into<String>) -> Self {
        Self::TooManyRequests {
            code: code.into(),
            message: msg.into(),
        }
    }

    pub fn service_unavailable(code: impl Into<String>, msg: impl Into<String>) -> Self {
        Self::ServiceUnavailable {
            code: code.into(),
            message: msg.into(),
        }
    }

    pub fn internal(code: impl Into<String>, msg: impl Into<String>) -> Self {
        Self::Internal {
            code: code.into(),
            message: msg.into(),
        }
    }

    /// Returns the corresponding HTTP status code.
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest { .. } => StatusCode::BAD_REQUEST,
            Self::Unauthorized { .. } => StatusCode::FORBIDDEN,
            Self::NotFound { .. } => StatusCode::NOT_FOUND,
            Self::Conflict { .. } => StatusCode::CONFLICT,
            Self::UnprocessableEntity { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            Self::TooManyRequests { .. } => StatusCode::TOO_MANY_REQUESTS,
            Self::ServiceUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
            Self::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Self::ContractError { status, .. } => *status,
        }
    }

    /// Returns the stable machine-readable error string.
    pub fn error_code(&self) -> &str {
        match self {
            Self::BadRequest { code, .. }
            | Self::Unauthorized { code, .. }
            | Self::NotFound { code, .. }
            | Self::Conflict { code, .. }
            | Self::UnprocessableEntity { code, .. }
            | Self::TooManyRequests { code, .. }
            | Self::ServiceUnavailable { code, .. }
            | Self::Internal { code, .. }
            | Self::ContractError { code, .. } => code.as_str(),
        }
    }

    /// Maps Soroban NegaWattEscrow error codes (1..14) to the Oracle API error model.
    pub fn from_contract_error_code(code: u32) -> Self {
        match code {
            1 => Self::internal("INTERNAL_CONTRACT_ERROR", "Contract already initialized"),
            2 => Self::internal("INTERNAL_CONTRACT_ERROR", "Contract not initialized"),
            3 => Self::ContractError {
                contract_code: 3,
                code: "ORACLE_UNAUTHORIZED".into(),
                message: "Caller is not the authorized oracle signer".into(),
                status: StatusCode::FORBIDDEN,
            },
            4 => Self::ContractError {
                contract_code: 4,
                code: "INVALID_PAYLOAD".into(),
                message: "Invalid transaction arguments or amount".into(),
                status: StatusCode::BAD_REQUEST,
            },
            5 => Self::internal("INVALID_CONFIG", "Contract bounty parameter is invalid"),
            6 => Self::internal("INVALID_CONFIG", "Contract payout cap is invalid"),
            7 => Self::internal("INVALID_CONFIG", "Contract window budget is invalid"),
            8 => Self::internal(
                "CONTRACT_INVARIANT_VIOLATED",
                "Contract safety invariant violated",
            ),
            9 => Self::ContractError {
                contract_code: 9,
                code: "POOL_DEPLETED".into(),
                message: "Escrow pool balance insufficient for payout".into(),
                status: StatusCode::SERVICE_UNAVAILABLE,
            },
            10 => Self::ContractError {
                contract_code: 10,
                code: "REPLAY_CLAIM".into(),
                message: "Proof hash has already been claimed on-chain".into(),
                status: StatusCode::CONFLICT,
            },
            11 => Self::ContractError {
                contract_code: 11,
                code: "EXCEEDS_PAYOUT_CAP".into(),
                message: "Requested payout exceeds maximum per-payout cap".into(),
                status: StatusCode::UNPROCESSABLE_ENTITY,
            },
            12 => Self::ContractError {
                contract_code: 12,
                code: "GLOBAL_CAP_EXCEEDED".into(),
                message: "Contract window payout budget exhausted".into(),
                status: StatusCode::TOO_MANY_REQUESTS,
            },
            13 => Self::ContractError {
                contract_code: 13,
                code: "WALLET_DAILY_LIMIT_EXCEEDED".into(),
                message: "Recipient maximum claim limit reached for current window".into(),
                status: StatusCode::TOO_MANY_REQUESTS,
            },
            14 => Self::ContractError {
                contract_code: 14,
                code: "CONTRACT_PAUSED".into(),
                message: "Payouts temporarily suspended by municipality pause".into(),
                status: StatusCode::SERVICE_UNAVAILABLE,
            },
            other => Self::internal(
                "UNKNOWN_CONTRACT_ERROR",
                format!("Unclassified Soroban contract error code {other}"),
            ),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = ApiErrorBody {
            code: self.error_code().to_string(),
            message: self.to_string(),
            status: status.as_u16(),
            details: None,
        };
        (status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contract_error_mappings() {
        let test_cases = [
            (
                1,
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_CONTRACT_ERROR",
            ),
            (
                2,
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_CONTRACT_ERROR",
            ),
            (3, StatusCode::FORBIDDEN, "ORACLE_UNAUTHORIZED"),
            (4, StatusCode::BAD_REQUEST, "INVALID_PAYLOAD"),
            (5, StatusCode::INTERNAL_SERVER_ERROR, "INVALID_CONFIG"),
            (6, StatusCode::INTERNAL_SERVER_ERROR, "INVALID_CONFIG"),
            (7, StatusCode::INTERNAL_SERVER_ERROR, "INVALID_CONFIG"),
            (
                8,
                StatusCode::INTERNAL_SERVER_ERROR,
                "CONTRACT_INVARIANT_VIOLATED",
            ),
            (9, StatusCode::SERVICE_UNAVAILABLE, "POOL_DEPLETED"),
            (10, StatusCode::CONFLICT, "REPLAY_CLAIM"),
            (11, StatusCode::UNPROCESSABLE_ENTITY, "EXCEEDS_PAYOUT_CAP"),
            (12, StatusCode::TOO_MANY_REQUESTS, "GLOBAL_CAP_EXCEEDED"),
            (
                13,
                StatusCode::TOO_MANY_REQUESTS,
                "WALLET_DAILY_LIMIT_EXCEEDED",
            ),
            (14, StatusCode::SERVICE_UNAVAILABLE, "CONTRACT_PAUSED"),
            (
                99,
                StatusCode::INTERNAL_SERVER_ERROR,
                "UNKNOWN_CONTRACT_ERROR",
            ),
        ];

        for (code, expected_status, expected_id) in test_cases {
            let err = ApiError::from_contract_error_code(code);
            assert_eq!(err.status_code(), expected_status, "Failed on code {code}");
            assert_eq!(err.error_code(), expected_id, "Failed on code {code}");
        }
    }

    #[test]
    fn test_into_response_json_structure() {
        let err = ApiError::bad_request("CHALLENGE_EXPIRED", "The challenge has expired");
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}
