//! # Challenge Issuance & Verification
//!
//! Handles `GET /api/v1/challenge` with strict Stellar StrKey checksum validation,
//! single-use nonce generation, wallet binding, and TTL enforcement.

use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use stellar_strkey::Strkey;
use tracing::info;

use crate::error::ApiError;
use crate::state::types::Challenge;
use crate::AppState;

/// Query parameters for `GET /api/v1/challenge`.
#[derive(Debug, Deserialize)]
pub struct ChallengeQuery {
    pub wallet: String,
}

/// JSON payload returned on successful challenge issuance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChallengeResponse {
    pub challenge_id: String,
    pub wallet_address: String,
    pub issued_at: String,
    pub expires_at: String,
}

/// Validates that a string is a valid Stellar Ed25519 public key (starts with 'G', passing CRC16).
pub fn validate_stellar_address(address: &str) -> Result<(), ApiError> {
    match Strkey::from_string(address) {
        Ok(Strkey::PublicKeyEd25519(_)) => Ok(()),
        _ => Err(ApiError::bad_request(
            "INVALID_WALLET_ADDRESS",
            format!("Invalid Stellar G... address format or checksum: '{address}'"),
        )),
    }
}

/// Generates a cryptographic random 8-byte hexadecimal challenge identifier.
pub fn generate_challenge_id() -> String {
    let mut bytes = [0u8; 8];
    getrandom::getrandom(&mut bytes).expect("OS entropy failure");
    format!("ch_{}", hex::encode(bytes))
}

/// `GET /api/v1/challenge?wallet={G...}`
///
/// Issues a short-lived, single-use, wallet-bound challenge nonce.
pub async fn get_challenge(
    State(state): State<AppState>,
    Query(query): Query<ChallengeQuery>,
) -> Result<Json<ChallengeResponse>, ApiError> {
    validate_stellar_address(&query.wallet)?;

    state
        .rate_limiter
        .check_wallet_challenge(&query.wallet, state.config.challenge_rate_per_min)?;

    let now = state.clock.now();
    let ttl_seconds = state.config.challenge_ttl_seconds;
    let expires_at = now + chrono::Duration::seconds(ttl_seconds as i64);
    let id = generate_challenge_id();

    let challenge = Challenge {
        id: id.clone(),
        wallet_address: query.wallet.clone(),
        issued_at: now,
        expires_at,
        used: false,
    };

    state
        .store
        .save_challenge(challenge)
        .await
        .map_err(|e| ApiError::internal("STORE_ERROR", e.to_string()))?;

    info!(
        wallet = %query.wallet,
        challenge = %id,
        ttl_seconds,
        "Issued wallet-bound challenge"
    );

    Ok(Json(ChallengeResponse {
        challenge_id: id,
        wallet_address: query.wallet,
        issued_at: now.to_rfc3339(),
        expires_at: expires_at.to_rfc3339(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use std::sync::Arc;

    use crate::state::{Clock, MemoryStore, MockClock, StateStore};

    const VALID_WALLET: &str = "GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT";
    const OTHER_WALLET: &str = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";

    #[tokio::test]
    async fn test_challenge_issuance_and_strkey_validation() {
        assert!(validate_stellar_address(VALID_WALLET).is_ok());
        assert!(validate_stellar_address("NOT_A_STELLAR_KEY").is_err());
        assert!(validate_stellar_address(
            "GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHX0"
        )
        .is_err()); // bad CRC
    }

    #[tokio::test]
    async fn test_challenge_expiry_and_reuse_with_mock_clock() {
        let base_time = Utc::now();
        let clock = Arc::new(MockClock::new(base_time));
        let store: Arc<dyn StateStore> = Arc::new(MemoryStore::new());

        let app_state = AppState {
            clock: clock.clone(),
            store: store.clone(),
            ..Default::default()
        };

        // 1. Issue challenge
        let res = get_challenge(
            State(app_state.clone()),
            Query(ChallengeQuery {
                wallet: VALID_WALLET.into(),
            }),
        )
        .await
        .expect("Challenge issuance failed");

        let ch = store
            .get_challenge(&res.challenge_id)
            .await
            .unwrap()
            .expect("Challenge not in store");

        // 2. Validate at current time -> valid
        assert!(ch.is_valid(clock.now(), VALID_WALLET));

        // 3. Wrong wallet -> invalid
        assert!(!ch.is_valid(clock.now(), OTHER_WALLET));

        // 4. Advance clock by 121 seconds -> expired
        clock.advance(Duration::seconds(121));
        assert!(!ch.is_valid(clock.now(), VALID_WALLET));

        // 5. Test reuse restriction
        clock.set(base_time);
        assert!(store.mark_challenge_used(&res.challenge_id).await.is_ok());
        assert!(store.mark_challenge_used(&res.challenge_id).await.is_err());
    }
}
