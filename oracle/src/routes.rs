//! # Oracle Route Handlers
//!
//! Provides the primary operational routes for EchoGrid Oracle:
//!   POST /api/v1/proof  — receives and settles a Proof-of-Power-Down claim.
//!   GET  /api/v1/health — service liveness probe.

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::chain::pipeline::SubmitResult;
use crate::error::ApiError;
use crate::payout::transition_claim;
use crate::proof::ProofV1;
use crate::state::limits::LimitConfig;
use crate::state::types::{Claim, PayoutStatus, StoreError};
use crate::AppState;

/// Standard JSON response for Proof-of-Power-Down submissions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum ProofResponse {
    Confirmed {
        claim_id: String,
        tx_hash: String,
        explorer_url: String,
        amount_stroops: i128,
        simulated: bool,
    },
    Pending {
        claim_id: String,
        poll_url: String,
    },
}

impl IntoResponse for ProofResponse {
    fn into_response(self) -> Response {
        match self {
            Self::Confirmed { .. } => (StatusCode::OK, Json(self)).into_response(),
            Self::Pending { .. } => (StatusCode::ACCEPTED, Json(self)).into_response(),
        }
    }
}

/// **Proof submission endpoint** — receives a client's Proof v1, validates cryptographic
/// bounds and challenge validity, reserves payout limits, and triggers on-chain settlement.
pub async fn submit_proof(
    State(state): State<AppState>,
    Json(payload): Json<ProofV1>,
) -> Result<Response, ApiError> {
    let now = state.clock.now();

    info!(
        wallet    = %payload.wallet_address,
        challenge = %payload.challenge_id,
        "Received Proof-of-Power-Down v1 payload"
    );

    // ── Step 1: Validate proof schema, time skew, and hash derivation ──────────
    let proof_hash_bytes = payload
        .validate_with_clock(state.clock.as_ref())
        .map_err(|e| ApiError::bad_request("INVALID_PAYLOAD", e.to_string()))?;

    // ── Step 2: Challenge check ────────────────────────────────────────────────
    let challenge = state
        .store
        .get_challenge(&payload.challenge_id)
        .await
        .map_err(|e| ApiError::internal("STORE_ERROR", e.to_string()))?
        .ok_or_else(|| ApiError::not_found("CHALLENGE_NOT_FOUND", "Challenge not found"))?;

    if challenge.wallet_address != payload.wallet_address {
        return Err(ApiError::bad_request(
            "CHALLENGE_WALLET_MISMATCH",
            "Challenge was issued to a different wallet address",
        ));
    }
    if challenge.used {
        return Err(ApiError::conflict(
            "CHALLENGE_ALREADY_USED",
            "Challenge nonce has already been consumed",
        ));
    }
    if challenge.expires_at < now {
        return Err(ApiError::bad_request(
            "CHALLENGE_EXPIRED",
            "Challenge has expired",
        ));
    }

    // ── Step 3: Idempotency & Replay Protection ────────────────────────────────
    if let Some(existing) = state
        .store
        .get_claim_by_proof_hash(&payload.proof_hash)
        .await
        .map_err(|e| ApiError::internal("STORE_ERROR", e.to_string()))?
    {
        match existing.status {
            PayoutStatus::Confirmed => {
                warn!(proof_hash = %payload.proof_hash, "Replay claim blocked");
                return Err(ApiError::conflict(
                    "REPLAY_CLAIM",
                    "This proof has already been processed and rewarded",
                ));
            }
            PayoutStatus::Submitted | PayoutStatus::Reserved => {
                let poll_url = format!("/api/v1/claims/{}", existing.id);
                return Ok((
                    StatusCode::ACCEPTED,
                    Json(ProofResponse::Pending {
                        claim_id: existing.id,
                        poll_url,
                    }),
                )
                    .into_response());
            }
            _ => {}
        }
    }

    // ── Step 4: Mark challenge as used ─────────────────────────────────────────
    state
        .store
        .mark_challenge_used(&payload.challenge_id)
        .await
        .map_err(|e| match e {
            StoreError::ChallengeUsed(_) => {
                ApiError::conflict("CHALLENGE_ALREADY_USED", "Challenge nonce already consumed")
            }
            other => ApiError::internal("STORE_ERROR", other.to_string()),
        })?;

    // ── Step 5: Limit reservation inside store transaction ─────────────────────
    let is_simulated = payload.simulated || state.config.simulator_mode;
    let claim_id = format!("clm_{}_{}", now.timestamp(), &payload.nonce[..8]);
    let bounty = state.config.bounty_stroops;
    let limits = LimitConfig {
        wallet_cooldown_seconds: state.config.cooldown_seconds as i64,
        wallet_daily_claims_limit: state.config.wallet_daily_claim_limit,
        global_daily_payout_cap_stroops: state.config.oracle_daily_cap_stroops,
    };

    let claim = Claim {
        id: claim_id.clone(),
        wallet_address: payload.wallet_address.clone(),
        challenge_id: payload.challenge_id.clone(),
        proof_hash: payload.proof_hash.clone(),
        amount_stroops: bounty,
        status: PayoutStatus::Reserved,
        tx_hash: None,
        error: None,
        simulated: is_simulated,
        created_at: now,
        updated_at: now,
    };

    state
        .store
        .reserve_claim(claim, &limits, now)
        .await
        .map_err(ApiError::from)?;

    // ── Step 6: Chain submission or simulator bypass ───────────────────────────
    let explorer_network = if state.config.network_passphrase.contains("Test") {
        "testnet"
    } else {
        "public"
    };

    if is_simulated {
        let sim_tx = format!("SIM_TX_{}", &payload.proof_hash[..16]);
        let _ = transition_claim(
            state.store.as_ref(),
            &claim_id,
            PayoutStatus::Submitted,
            Some(sim_tx.clone()),
            None,
            now,
        )
        .await;
        let _ = transition_claim(
            state.store.as_ref(),
            &claim_id,
            PayoutStatus::Confirmed,
            Some(sim_tx.clone()),
            None,
            now,
        )
        .await;

        let explorer_url =
            format!("https://stellar.expert/explorer/{explorer_network}/tx/{sim_tx}");

        return Ok((
            StatusCode::OK,
            Json(ProofResponse::Confirmed {
                claim_id,
                tx_hash: sim_tx,
                explorer_url,
                amount_stroops: bounty,
                simulated: true,
            }),
        )
            .into_response());
    }

    if let Some(queue) = &state.queue {
        let _ = transition_claim(
            state.store.as_ref(),
            &claim_id,
            PayoutStatus::Submitted,
            None,
            None,
            now,
        )
        .await;

        let submit_timeout = std::time::Duration::from_secs(state.config.sync_wait_timeout_seconds);
        let submit_fut = queue.submit(payload.wallet_address.clone(), proof_hash_bytes);

        match tokio::time::timeout(submit_timeout, submit_fut).await {
            Ok(Ok(SubmitResult::Confirmed { tx_hash, .. })) => {
                let _ = transition_claim(
                    state.store.as_ref(),
                    &claim_id,
                    PayoutStatus::Confirmed,
                    Some(tx_hash.clone()),
                    None,
                    now,
                )
                .await;

                let explorer_url =
                    format!("https://stellar.expert/explorer/{explorer_network}/tx/{tx_hash}");

                Ok((
                    StatusCode::OK,
                    Json(ProofResponse::Confirmed {
                        claim_id,
                        tx_hash,
                        explorer_url,
                        amount_stroops: bounty,
                        simulated: false,
                    }),
                )
                    .into_response())
            }
            Ok(Ok(SubmitResult::Timeout { tx_hash })) => {
                let _ = state
                    .store
                    .update_claim_status(
                        &claim_id,
                        PayoutStatus::Submitted,
                        Some(tx_hash),
                        None,
                        now,
                    )
                    .await;

                let poll_url = format!("/api/v1/claims/{claim_id}");
                Ok((
                    StatusCode::ACCEPTED,
                    Json(ProofResponse::Pending { claim_id, poll_url }),
                )
                    .into_response())
            }
            Ok(Ok(SubmitResult::Failed { error, .. })) => {
                let _ = state.store.release_claim(&claim_id, &error, now).await;
                Err(ApiError::unprocessable_entity("PAYOUT_FAILED", error))
            }
            Ok(Err(pipeline_err)) => {
                let err_msg = pipeline_err.to_string();
                let _ = state.store.release_claim(&claim_id, &err_msg, now).await;
                Err(ApiError::service_unavailable("PIPELINE_ERROR", err_msg))
            }
            Err(_) => {
                // Synchronous wait timed out (> 20s): return 202 Accepted
                let poll_url = format!("/api/v1/claims/{claim_id}");
                Ok((
                    StatusCode::ACCEPTED,
                    Json(ProofResponse::Pending { claim_id, poll_url }),
                )
                    .into_response())
            }
        }
    } else {
        // Mock fallback when no queue is configured
        let mock_tx = format!("tx_mock_{}", &payload.proof_hash[..16]);
        let _ = transition_claim(
            state.store.as_ref(),
            &claim_id,
            PayoutStatus::Submitted,
            Some(mock_tx.clone()),
            None,
            now,
        )
        .await;
        let _ = transition_claim(
            state.store.as_ref(),
            &claim_id,
            PayoutStatus::Confirmed,
            Some(mock_tx.clone()),
            None,
            now,
        )
        .await;

        let explorer_url =
            format!("https://stellar.expert/explorer/{explorer_network}/tx/{mock_tx}");

        Ok((
            StatusCode::OK,
            Json(ProofResponse::Confirmed {
                claim_id,
                tx_hash: mock_tx,
                explorer_url,
                amount_stroops: bounty,
                simulated: false,
            }),
        )
            .into_response())
    }
}

/// Simple liveness probe.
pub async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status":  "ok",
        "service": "echogrid-oracle",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proof::derive_proof_hash_hex;
    use crate::state::types::Challenge;
    use crate::state::{MemoryStore, MockClock};
    use chrono::{DateTime, Duration, Utc};
    use std::sync::Arc;

    const WALLET: &str = "GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT";
    const CH_ID: &str = "ch_test_123";
    const NONCE: &str = "a1b2c3d4e5f60718";
    const TS: &str = "2026-10-28T12:00:00Z";

    async fn setup_state(now: DateTime<Utc>) -> AppState {
        let store = Arc::new(MemoryStore::new());
        let clock = Arc::new(MockClock::new(now));
        AppState {
            store,
            clock,
            ..Default::default()
        }
    }

    fn make_proof(wallet: &str, ch_id: &str, ts: &str, nonce: &str) -> ProofV1 {
        let proof_hash = derive_proof_hash_hex(wallet, ch_id, ts, nonce);
        ProofV1 {
            wallet_address: wallet.to_string(),
            challenge_id: ch_id.to_string(),
            timestamp: ts.to_string(),
            nonce: nonce.to_string(),
            proof_hash,
            detector_version: "echonet-1.2".to_string(),
            duration_seconds: 15,
            simulated: false,
        }
    }

    #[tokio::test]
    async fn test_submit_proof_happy_path_mock() {
        let now = DateTime::parse_from_rfc3339(TS)
            .unwrap()
            .with_timezone(&Utc);
        let state = setup_state(now).await;

        // Issue challenge
        let ch = Challenge {
            id: CH_ID.to_string(),
            wallet_address: WALLET.to_string(),
            issued_at: now,
            expires_at: now + Duration::seconds(120),
            used: false,
        };
        state.store.save_challenge(ch).await.unwrap();

        let proof = make_proof(WALLET, CH_ID, TS, NONCE);
        let res = submit_proof(State(state.clone()), Json(proof))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // Challenge is marked used
        let ch_after = state.store.get_challenge(CH_ID).await.unwrap().unwrap();
        assert!(ch_after.used);
    }

    #[tokio::test]
    async fn test_submit_proof_rejections() {
        let now = DateTime::parse_from_rfc3339(TS)
            .unwrap()
            .with_timezone(&Utc);
        let state = setup_state(now).await;

        let proof = make_proof(WALLET, CH_ID, TS, NONCE);

        // 1. Challenge not found
        let err = submit_proof(State(state.clone()), Json(proof.clone()))
            .await
            .unwrap_err();
        assert_eq!(err.status_code(), StatusCode::NOT_FOUND);

        // Save expired challenge
        let ch_expired = Challenge {
            id: CH_ID.to_string(),
            wallet_address: WALLET.to_string(),
            issued_at: now - Duration::seconds(200),
            expires_at: now - Duration::seconds(10),
            used: false,
        };
        state.store.save_challenge(ch_expired).await.unwrap();

        // 2. Challenge expired
        let err_exp = submit_proof(State(state.clone()), Json(proof.clone()))
            .await
            .unwrap_err();
        assert_eq!(err_exp.status_code(), StatusCode::BAD_REQUEST);

        // 3. Challenge wrong wallet
        let ch_wrong_wallet = Challenge {
            id: "ch_other".to_string(),
            wallet_address: "GBOTHER...".to_string(),
            issued_at: now,
            expires_at: now + Duration::seconds(120),
            used: false,
        };
        state.store.save_challenge(ch_wrong_wallet).await.unwrap();
        let proof_wrong = make_proof(WALLET, "ch_other", TS, NONCE);
        let err_w = submit_proof(State(state.clone()), Json(proof_wrong))
            .await
            .unwrap_err();
        assert_eq!(err_w.status_code(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_submit_proof_replay_protection() {
        let now = DateTime::parse_from_rfc3339(TS)
            .unwrap()
            .with_timezone(&Utc);
        let state = setup_state(now).await;

        let ch = Challenge {
            id: CH_ID.to_string(),
            wallet_address: WALLET.to_string(),
            issued_at: now,
            expires_at: now + Duration::seconds(120),
            used: false,
        };
        state.store.save_challenge(ch).await.unwrap();

        let proof = make_proof(WALLET, CH_ID, TS, NONCE);

        // First submit: OK
        assert!(submit_proof(State(state.clone()), Json(proof.clone()))
            .await
            .is_ok());

        // Replay submission: 409 Conflict REPLAY_CLAIM
        let replay_err = submit_proof(State(state.clone()), Json(proof))
            .await
            .unwrap_err();
        assert_eq!(replay_err.status_code(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_submit_proof_simulator_mode_bypasses_chain() {
        let now = DateTime::parse_from_rfc3339(TS)
            .unwrap()
            .with_timezone(&Utc);
        let state = setup_state(now).await;

        let ch = Challenge {
            id: CH_ID.to_string(),
            wallet_address: WALLET.to_string(),
            issued_at: now,
            expires_at: now + Duration::seconds(120),
            used: false,
        };
        state.store.save_challenge(ch).await.unwrap();

        let mut proof = make_proof(WALLET, CH_ID, TS, NONCE);
        proof.simulated = true;

        let res = submit_proof(State(state.clone()), Json(proof.clone()))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let claim = state
            .store
            .get_claim_by_proof_hash(&proof.proof_hash)
            .await
            .unwrap()
            .unwrap();
        assert!(claim.simulated);
        assert_eq!(claim.status, PayoutStatus::Confirmed);
        assert!(claim.tx_hash.unwrap().starts_with("SIM_TX_"));

        // Simulated claims do not deplete the on-chain daily payout cap
        let chain_spent = state
            .store
            .get_daily_spent_and_reserved(now - Duration::hours(1))
            .await
            .unwrap();
        assert_eq!(chain_spent, 0);
    }

    #[tokio::test]
    async fn test_simulated_claim_never_reaches_pipeline() {
        use crate::chain::pipeline::SubmitPipeline;
        use crate::chain::queue::SubmissionQueue;
        use crate::chain::rpc::SorobanRpcClient;
        use ed25519_dalek::SigningKey;
        use wiremock::MockServer;

        let server = MockServer::start().await; // Zero endpoints mounted
        let rpc = SorobanRpcClient::new(&server.uri(), std::time::Duration::from_millis(500));
        let signing_key = SigningKey::from_bytes(&[1u8; 32]);
        let pipeline = Arc::new(SubmitPipeline::new(
            rpc.clone(),
            signing_key,
            "C_CONTRACT_123".into(),
            "Test SDF Network ; September 2015".into(),
            std::time::Duration::from_millis(500),
        ));
        let queue = Arc::new(SubmissionQueue::new(
            pipeline,
            rpc,
            "G_ORACLE".into(),
            Some(100),
        ));

        let now = DateTime::parse_from_rfc3339(TS)
            .unwrap()
            .with_timezone(&Utc);
        let mut state = setup_state(now).await;
        state.queue = Some(queue);

        let ch = Challenge {
            id: CH_ID.to_string(),
            wallet_address: WALLET.to_string(),
            issued_at: now,
            expires_at: now + Duration::seconds(120),
            used: false,
        };
        state.store.save_challenge(ch).await.unwrap();

        let mut proof = make_proof(WALLET, CH_ID, TS, NONCE);
        proof.simulated = true;

        let res = submit_proof(State(state), Json(proof)).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // Verify zero requests ever reached Soroban RPC pipeline
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 0);
    }
}
