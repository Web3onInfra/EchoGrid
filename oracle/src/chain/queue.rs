//! # Serialized Soroban Submission Queue
//!
//! Provides a single-writer submission queue ensuring concurrent payout requests
//! cannot collide on the Stellar account sequence number.

use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, info};

use crate::chain::pipeline::{PipelineError, SubmitPipeline, SubmitResult};
use crate::chain::rpc::SorobanRpcClient;

/// Payout task submitted to the background sequencer.
struct PayoutJob {
    recipient: String,
    proof_hash: [u8; 32],
    responder: oneshot::Sender<Result<SubmitResult, PipelineError>>,
}

/// Handle to the single-writer submission queue.
#[derive(Clone)]
pub struct SubmissionQueue {
    sender: mpsc::Sender<PayoutJob>,
}

impl SubmissionQueue {
    /// Spawns the single-writer worker task managing monotonic sequence numbers.
    pub fn new(
        pipeline: Arc<SubmitPipeline>,
        rpc_client: SorobanRpcClient,
        oracle_account: String,
        initial_sequence: Option<i64>,
    ) -> Self {
        let (sender, mut receiver) = mpsc::channel::<PayoutJob>(100);

        tokio::spawn(async move {
            let mut current_seq = initial_sequence;

            while let Some(job) = receiver.recv().await {
                // Initialize sequence if unknown
                if current_seq.is_none() {
                    match rpc_client.get_account_sequence(&oracle_account).await {
                        Ok(seq) => {
                            debug!(oracle = %oracle_account, seq, "Initialized oracle account sequence");
                            current_seq = Some(seq as i64);
                        }
                        Err(e) => {
                            error!(err = %e, "Failed to look up oracle sequence number");
                            let _ = job.responder.send(Err(PipelineError::Rpc(e)));
                            continue;
                        }
                    }
                }

                let next_seq = current_seq.unwrap() + 1;
                debug!(seq = next_seq, recipient = %job.recipient, "Processing serialized payout submission");

                let result = pipeline
                    .submit_payout(&job.recipient, &job.proof_hash, next_seq)
                    .await;

                match &result {
                    Ok(SubmitResult::Confirmed { .. }) | Ok(SubmitResult::Timeout { .. }) => {
                        // Monotonically advance on successful network inclusion
                        current_seq = Some(next_seq);
                    }
                    Ok(SubmitResult::Failed { error, .. }) => {
                        // If failure was sequence-related, invalidate cached sequence
                        if error.contains("bad sequence") || error.contains("txBAD_SEQ") {
                            debug!("Sequence desync detected, invalidating sequence cache");
                            current_seq = None;
                        } else {
                            current_seq = Some(next_seq);
                        }
                    }
                    Err(PipelineError::Rpc(_)) => {
                        // Transient RPC error, invalidate sequence to be safe
                        current_seq = None;
                    }
                    _ => {}
                }

                let _ = job.responder.send(result);
            }

            info!("Submission queue worker terminated");
        });

        Self { sender }
    }

    /// Submits a payout claim to the serialized queue and awaits the result.
    pub async fn submit(
        &self,
        recipient: String,
        proof_hash: [u8; 32],
    ) -> Result<SubmitResult, PipelineError> {
        let (responder, rx) = oneshot::channel();
        let job = PayoutJob {
            recipient,
            proof_hash,
            responder,
        };

        self.sender
            .send(job)
            .await
            .map_err(|_| PipelineError::SubmissionRejected("Submission queue closed".into()))?;

        rx.await.map_err(|_| {
            PipelineError::SubmissionRejected("Sequencer dropped response channel".into())
        })?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use std::time::Duration;
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_signing_key() -> SigningKey {
        SigningKey::from_bytes(&[9u8; 32])
    }

    #[tokio::test]
    async fn test_n_concurrent_submits_are_serialized_and_succeed() {
        let server = MockServer::start().await;

        // Mock getAccount sequence lookup
        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_string_contains("getAccount"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": { "sequence": "1000" }
            })))
            .mount(&server)
            .await;

        // Mock simulateTransaction -> returns fee
        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_string_contains("simulateTransaction"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": { "minResourceFee": "100" }
            })))
            .mount(&server)
            .await;

        // Mock sendTransaction -> returns PENDING with tx_hash
        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_string_contains("sendTransaction"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": { "status": "PENDING", "hash": "tx_concurrent_mock" }
            })))
            .mount(&server)
            .await;

        // Mock getTransaction -> returns SUCCESS
        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_string_contains("getTransaction"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": { "status": "SUCCESS", "ledger": 5000 }
            })))
            .mount(&server)
            .await;

        let rpc_client = SorobanRpcClient::new(&server.uri(), Duration::from_secs(2));
        let mut pipeline = SubmitPipeline::new(
            rpc_client.clone(),
            test_signing_key(),
            "CDFOK4NF4CY27N4G5PYA6SXTJA5MEKPVU7DQRMJUS74LIXHOKYNRHOPF".into(),
            "Test SDF Network ; September 2015".into(),
            Duration::from_secs(2),
        );
        pipeline.set_poll_interval(Duration::from_millis(5));

        let queue = SubmissionQueue::new(
            Arc::new(pipeline),
            rpc_client,
            "GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT".into(),
            Some(1000), // Pre-set sequence at 1000
        );

        // Spawn N concurrent tasks
        const CONCURRENT_COUNT: usize = 10;
        let mut handles = Vec::new();

        for i in 0..CONCURRENT_COUNT {
            let q = queue.clone();
            let handle = tokio::spawn(async move {
                let proof_hash = [i as u8; 32];
                q.submit(
                    "GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT".into(),
                    proof_hash,
                )
                .await
            });
            handles.push(handle);
        }

        // Await all concurrent tasks
        for handle in handles {
            let res = handle.await.unwrap().expect("Submit failed");
            assert!(matches!(res, SubmitResult::Confirmed { .. }));
        }
    }
}
