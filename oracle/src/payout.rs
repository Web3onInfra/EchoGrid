//! # Payout State Machine & Startup Reconciliation
//!
//! Manages claim lifecycle transitions (Reserved -> Submitted -> Confirmed / Failed / Expired)
//! and reconciles in-flight claims against Soroban RPC upon node boot.

use chrono::{DateTime, Utc};

use crate::chain::rpc::SorobanRpcClient;
use crate::state::types::{PayoutStatus, StoreError};
use crate::state::StateStore;

/// Stale threshold in seconds after which unconfirmed in-flight claims expire (5 minutes).
pub const RECONCILIATION_STALE_SECONDS: i64 = 300;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PayoutStateMachineError {
    #[error("Invalid claim state transition from {from:?} to {to:?}")]
    InvalidTransition {
        from: PayoutStatus,
        to: PayoutStatus,
    },
    #[error("Claim not found: {0}")]
    NotFound(String),
    #[error("State store error: {0}")]
    Store(String),
    #[error("Soroban RPC error during reconciliation: {0}")]
    Rpc(String),
}

impl From<StoreError> for PayoutStateMachineError {
    fn from(err: StoreError) -> Self {
        match err {
            StoreError::NotFound(id) => Self::NotFound(id),
            other => Self::Store(other.to_string()),
        }
    }
}

/// Checks whether a claim state transition is allowed by the lifecycle state machine.
pub fn is_valid_transition(from: PayoutStatus, to: PayoutStatus) -> bool {
    matches!(
        (from, to),
        (PayoutStatus::Reserved, PayoutStatus::Submitted)
            | (PayoutStatus::Reserved, PayoutStatus::Failed)
            | (PayoutStatus::Reserved, PayoutStatus::Expired)
            | (PayoutStatus::Submitted, PayoutStatus::Confirmed)
            | (PayoutStatus::Submitted, PayoutStatus::Failed)
            | (PayoutStatus::Submitted, PayoutStatus::Expired)
    )
}

/// Transitions a claim to a new lifecycle state, validating allowed transitions.
pub async fn transition_claim(
    store: &dyn StateStore,
    claim_id: &str,
    target: PayoutStatus,
    tx_hash: Option<String>,
    error: Option<String>,
    now: DateTime<Utc>,
) -> Result<(), PayoutStateMachineError> {
    let claim = store
        .get_claim(claim_id)
        .await?
        .ok_or_else(|| PayoutStateMachineError::NotFound(claim_id.to_string()))?;

    if !is_valid_transition(claim.status, target) {
        return Err(PayoutStateMachineError::InvalidTransition {
            from: claim.status,
            to: target,
        });
    }

    let final_tx_hash = tx_hash.or(claim.tx_hash);
    store
        .update_claim_status(claim_id, target, final_tx_hash, error, now)
        .await?;
    Ok(())
}

/// Results summary of startup reconciliation of in-flight claims.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReconciliationReport {
    pub examined: usize,
    pub confirmed: usize,
    pub failed: usize,
    pub expired: usize,
    pub pending: usize,
}

/// Reconciles all in-flight claims on service startup.
pub async fn reconcile_in_flight_claims(
    store: &dyn StateStore,
    rpc: &SorobanRpcClient,
    now: DateTime<Utc>,
) -> Result<ReconciliationReport, PayoutStateMachineError> {
    let in_flight = store.get_in_flight_claims().await?;
    let mut report = ReconciliationReport {
        examined: in_flight.len(),
        ..Default::default()
    };

    for claim in in_flight {
        let age_seconds = (now - claim.created_at).num_seconds();

        match claim.status {
            PayoutStatus::Reserved => {
                if age_seconds >= RECONCILIATION_STALE_SECONDS {
                    transition_claim(
                        store,
                        &claim.id,
                        PayoutStatus::Expired,
                        None,
                        Some("Expired during startup reconciliation".into()),
                        now,
                    )
                    .await?;
                    report.expired += 1;
                } else {
                    report.pending += 1;
                }
            }
            PayoutStatus::Submitted => {
                if let Some(ref tx_hash) = claim.tx_hash {
                    match rpc.get_transaction(tx_hash).await {
                        Ok(tx_res) => match tx_res.status.to_uppercase().as_str() {
                            "SUCCESS" => {
                                transition_claim(
                                    store,
                                    &claim.id,
                                    PayoutStatus::Confirmed,
                                    Some(tx_hash.clone()),
                                    None,
                                    now,
                                )
                                .await?;
                                report.confirmed += 1;
                            }
                            "FAILED" => {
                                transition_claim(
                                    store,
                                    &claim.id,
                                    PayoutStatus::Failed,
                                    Some(tx_hash.clone()),
                                    Some("On-chain transaction execution failed".into()),
                                    now,
                                )
                                .await?;
                                report.failed += 1;
                            }
                            _ => {
                                if age_seconds >= RECONCILIATION_STALE_SECONDS {
                                    transition_claim(
                                        store,
                                        &claim.id,
                                        PayoutStatus::Failed,
                                        Some(tx_hash.clone()),
                                        Some("Transaction not confirmed within timeout".into()),
                                        now,
                                    )
                                    .await?;
                                    report.failed += 1;
                                } else {
                                    report.pending += 1;
                                }
                            }
                        },
                        Err(_) => {
                            report.pending += 1;
                        }
                    }
                } else if age_seconds >= RECONCILIATION_STALE_SECONDS {
                    transition_claim(
                        store,
                        &claim.id,
                        PayoutStatus::Failed,
                        None,
                        Some("Submitted claim missing tx_hash expired".into()),
                        now,
                    )
                    .await?;
                    report.failed += 1;
                } else {
                    report.pending += 1;
                }
            }
            _ => {}
        }
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::types::Claim;
    use crate::state::MemoryStore;
    use chrono::Duration;
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn make_test_claim(id: &str, status: PayoutStatus, created_at: DateTime<Utc>) -> Claim {
        Claim {
            id: id.to_string(),
            wallet_address: "GA4Z...".to_string(),
            challenge_id: "ch_1".to_string(),
            proof_hash: format!("hash_{id}"),
            amount_stroops: 10_000_000,
            status,
            tx_hash: if status == PayoutStatus::Submitted {
                Some("tx_abc123".to_string())
            } else {
                None
            },
            error: None,
            simulated: false,
            created_at,
            updated_at: created_at,
        }
    }

    #[tokio::test]
    async fn test_state_machine_valid_and_invalid_transitions() {
        let store = MemoryStore::new();
        let now = Utc::now();
        let claim = make_test_claim("c1", PayoutStatus::Reserved, now);
        store.save_claim(claim).await.unwrap();

        // Invalid: Reserved -> Confirmed directly
        let err = transition_claim(
            &store,
            "c1",
            PayoutStatus::Confirmed,
            Some("tx".into()),
            None,
            now,
        )
        .await;
        assert_eq!(
            err,
            Err(PayoutStateMachineError::InvalidTransition {
                from: PayoutStatus::Reserved,
                to: PayoutStatus::Confirmed
            })
        );

        // Valid: Reserved -> Submitted
        assert!(transition_claim(
            &store,
            "c1",
            PayoutStatus::Submitted,
            Some("tx_1".into()),
            None,
            now
        )
        .await
        .is_ok());

        // Valid: Submitted -> Confirmed
        assert!(
            transition_claim(&store, "c1", PayoutStatus::Confirmed, None, None, now)
                .await
                .is_ok()
        );

        // Invalid: Confirmed -> Reserved
        assert!(
            transition_claim(&store, "c1", PayoutStatus::Reserved, None, None, now)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_crash_reconciliation_reserved_and_submitted_scenarios() {
        let server = MockServer::start().await;
        let rpc = SorobanRpcClient::new(&server.uri(), std::time::Duration::from_secs(5));
        let store = MemoryStore::new();
        let now = Utc::now();

        // 1. Crash while Reserved > 5m ago
        store
            .save_claim(make_test_claim(
                "c_reserved_stale",
                PayoutStatus::Reserved,
                now - Duration::minutes(6),
            ))
            .await
            .unwrap();

        // 2. Crash while Submitted > 5m ago, on-chain SUCCESS
        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_string_contains("getTransaction"))
            .and(body_string_contains("tx_success"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": { "status": "SUCCESS", "latestLedger": 100 }
            })))
            .mount(&server)
            .await;

        let mut c_success = make_test_claim(
            "c_success",
            PayoutStatus::Submitted,
            now - Duration::minutes(2),
        );
        c_success.tx_hash = Some("tx_success".into());
        store.save_claim(c_success).await.unwrap();

        // 3. Crash while Submitted > 5m ago, NOT_FOUND (dropped)
        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_string_contains("getTransaction"))
            .and(body_string_contains("tx_dropped"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": 2,
                "result": { "status": "NOT_FOUND", "latestLedger": 100 }
            })))
            .mount(&server)
            .await;

        let mut c_dropped = make_test_claim(
            "c_dropped",
            PayoutStatus::Submitted,
            now - Duration::minutes(6),
        );
        c_dropped.tx_hash = Some("tx_dropped".into());
        store.save_claim(c_dropped).await.unwrap();

        // Run boot reconciliation
        let report = reconcile_in_flight_claims(&store, &rpc, now).await.unwrap();
        assert_eq!(report.examined, 3);
        assert_eq!(report.expired, 1);
        assert_eq!(report.confirmed, 1);
        assert_eq!(report.failed, 1);

        assert_eq!(
            store
                .get_claim("c_reserved_stale")
                .await
                .unwrap()
                .unwrap()
                .status,
            PayoutStatus::Expired
        );
        assert_eq!(
            store.get_claim("c_success").await.unwrap().unwrap().status,
            PayoutStatus::Confirmed
        );
        assert_eq!(
            store.get_claim("c_dropped").await.unwrap().unwrap().status,
            PayoutStatus::Failed
        );
    }
}
