//! # Soroban Payout Submission & Confirmation Pipeline
//!
//! Orchestrates the full lifecycle:
//! Sequence lookup -> Simulation -> Assembly/Signing -> Submission -> Polling confirmation.
//! Features bounded retries and exponential backoff for transient failures.

use ed25519_dalek::SigningKey;
use std::time::{Duration, Instant};
use tracing::{debug, warn};

use crate::chain::builder::PayoutTxBuilder;
use crate::chain::rpc::{RpcClientError, SorobanRpcClient};

#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("RPC client error: {0}")]
    Rpc(#[from] RpcClientError),
    #[error("Builder error: {0}")]
    Builder(String),
    #[error("Simulation failed: {0}")]
    SimulationFailed(String),
    #[error("Submission rejected: {0}")]
    SubmissionRejected(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubmitResult {
    Confirmed {
        tx_hash: String,
        ledger: u32,
    },
    Failed {
        tx_hash: Option<String>,
        error: String,
    },
    Timeout {
        tx_hash: String,
    },
}

pub struct SubmitPipeline {
    rpc_client: SorobanRpcClient,
    signing_key: SigningKey,
    contract_id: String,
    network_passphrase: String,
    max_retries: usize,
    poll_timeout: Duration,
    poll_interval: Duration,
}

impl SubmitPipeline {
    pub fn new(
        rpc_client: SorobanRpcClient,
        signing_key: SigningKey,
        contract_id: String,
        network_passphrase: String,
        poll_timeout: Duration,
    ) -> Self {
        Self {
            rpc_client,
            signing_key,
            contract_id,
            network_passphrase,
            max_retries: 3,
            poll_timeout,
            poll_interval: Duration::from_millis(500),
        }
    }

    pub fn set_poll_interval(&mut self, interval: Duration) {
        self.poll_interval = interval;
    }

    pub async fn submit_payout(
        &self,
        recipient_addr: &str,
        proof_hash: &[u8; 32],
        sequence: i64,
    ) -> Result<SubmitResult, PipelineError> {
        let mut attempts = 0;
        let mut backoff = Duration::from_millis(100);

        loop {
            attempts += 1;

            // 1. Build draft envelope for simulation
            let (_draft_envelope, draft_xdr) = PayoutTxBuilder::build_and_sign_envelope(
                &self.signing_key,
                &self.network_passphrase,
                &self.contract_id,
                recipient_addr,
                proof_hash,
                sequence,
                100,
            )
            .map_err(|e| PipelineError::Builder(e.to_string()))?;

            // 2. Simulate transaction
            let sim_res = self.rpc_client.simulate_transaction(&draft_xdr).await?;
            if let Some(err) = sim_res.error {
                return Ok(SubmitResult::Failed {
                    tx_hash: None,
                    error: format!("Simulation error: {err}"),
                });
            }

            let resource_fee: u32 = sim_res
                .min_resource_fee
                .and_then(|f| f.parse().ok())
                .unwrap_or(100);

            // 3. Assemble finalized envelope with resource fee
            let total_fee = 100 + resource_fee;
            let (_final_envelope, final_xdr) = PayoutTxBuilder::build_and_sign_envelope(
                &self.signing_key,
                &self.network_passphrase,
                &self.contract_id,
                recipient_addr,
                proof_hash,
                sequence,
                total_fee,
            )
            .map_err(|e| PipelineError::Builder(e.to_string()))?;

            // 4. Send transaction
            let send_res = self.rpc_client.send_transaction(&final_xdr).await?;
            let tx_hash = send_res.hash;

            match send_res.status.as_str() {
                "PENDING" => {
                    debug!(hash = %tx_hash, "Transaction accepted, polling for ledger confirmation");
                    return self.poll_confirmation(&tx_hash).await;
                }
                "DUPLICATE" => {
                    return Ok(SubmitResult::Confirmed {
                        tx_hash,
                        ledger: send_res.latest_ledger.unwrap_or(0),
                    });
                }
                "TRY_AGAIN_LATER" => {
                    if attempts < self.max_retries {
                        warn!(attempt = attempts, "Transient RPC rejection, backing off");
                        tokio::time::sleep(backoff).await;
                        backoff *= 2;
                        continue;
                    }
                    return Ok(SubmitResult::Failed {
                        tx_hash: Some(tx_hash),
                        error: "Exhausted retries on TRY_AGAIN_LATER".into(),
                    });
                }
                "ERROR" => {
                    let err_msg = send_res
                        .error_result_xdr
                        .unwrap_or_else(|| "Submission rejected with status ERROR".into());
                    return Ok(SubmitResult::Failed {
                        tx_hash: Some(tx_hash),
                        error: err_msg,
                    });
                }
                other => {
                    return Ok(SubmitResult::Failed {
                        tx_hash: Some(tx_hash),
                        error: format!("Unrecognized sendTransaction status '{other}'"),
                    });
                }
            }
        }
    }

    async fn poll_confirmation(&self, tx_hash: &str) -> Result<SubmitResult, PipelineError> {
        let start = Instant::now();

        while start.elapsed() < self.poll_timeout {
            let res = self.rpc_client.get_transaction(tx_hash).await?;
            match res.status.as_str() {
                "SUCCESS" => {
                    return Ok(SubmitResult::Confirmed {
                        tx_hash: tx_hash.to_string(),
                        ledger: res.ledger.unwrap_or(0),
                    });
                }
                "FAILED" => {
                    return Ok(SubmitResult::Failed {
                        tx_hash: Some(tx_hash.to_string()),
                        error: "Transaction failed during ledger execution".into(),
                    });
                }
                _ => {
                    tokio::time::sleep(self.poll_interval).await;
                }
            }
        }

        Ok(SubmitResult::Timeout {
            tx_hash: tx_hash.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_signing_key() -> SigningKey {
        SigningKey::from_bytes(&[7u8; 32])
    }

    #[tokio::test]
    async fn test_pipeline_happy_path_confirmation() {
        let server = MockServer::start().await;

        // Mock simulateTransaction -> success
        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": { "minResourceFee": "200" }
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        // Mock sendTransaction -> PENDING
        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": { "status": "PENDING", "hash": "tx_mock_hash_123" }
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        // Mock getTransaction -> SUCCESS
        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": { "status": "SUCCESS", "ledger": 1050 }
            })))
            .mount(&server)
            .await;

        let rpc_client = SorobanRpcClient::new(&server.uri(), Duration::from_secs(2));
        let mut pipeline = SubmitPipeline::new(
            rpc_client,
            test_signing_key(),
            "CDFOK4NF4CY27N4G5PYA6SXTJA5MEKPVU7DQRMJUS74LIXHOKYNRHOPF".into(),
            "Test SDF Network ; September 2015".into(),
            Duration::from_secs(2),
        );
        pipeline.set_poll_interval(Duration::from_millis(5));

        let res = pipeline
            .submit_payout(
                "GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT",
                &[1u8; 32],
                10,
            )
            .await
            .unwrap();

        assert_eq!(
            res,
            SubmitResult::Confirmed {
                tx_hash: "tx_mock_hash_123".into(),
                ledger: 1050
            }
        );
    }

    #[tokio::test]
    async fn test_pipeline_simulation_failure() {
        let server = MockServer::start().await;

        // Mock simulateTransaction -> error
        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": { "error": "HostError: Error(Contract, #10)" }
            })))
            .mount(&server)
            .await;

        let rpc_client = SorobanRpcClient::new(&server.uri(), Duration::from_secs(2));
        let pipeline = SubmitPipeline::new(
            rpc_client,
            test_signing_key(),
            "CDFOK4NF4CY27N4G5PYA6SXTJA5MEKPVU7DQRMJUS74LIXHOKYNRHOPF".into(),
            "Test SDF Network ; September 2015".into(),
            Duration::from_secs(2),
        );

        let res = pipeline
            .submit_payout(
                "GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT",
                &[2u8; 32],
                10,
            )
            .await
            .unwrap();

        match res {
            SubmitResult::Failed { error, .. } => {
                assert!(error.contains("Contract, #10"));
            }
            _ => panic!("Expected simulation failure"),
        }
    }

    #[tokio::test]
    async fn test_pipeline_poll_timeout() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": { "minResourceFee": "100" }
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": { "status": "PENDING", "hash": "tx_slow_123" }
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": { "status": "NOT_FOUND" }
            })))
            .mount(&server)
            .await;

        let rpc_client = SorobanRpcClient::new(&server.uri(), Duration::from_secs(2));
        let mut pipeline = SubmitPipeline::new(
            rpc_client,
            test_signing_key(),
            "CDFOK4NF4CY27N4G5PYA6SXTJA5MEKPVU7DQRMJUS74LIXHOKYNRHOPF".into(),
            "Test SDF Network ; September 2015".into(),
            Duration::from_millis(50),
        );
        pipeline.set_poll_interval(Duration::from_millis(10));

        let res = pipeline
            .submit_payout(
                "GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT",
                &[3u8; 32],
                10,
            )
            .await
            .unwrap();

        assert_eq!(
            res,
            SubmitResult::Timeout {
                tx_hash: "tx_slow_123".into()
            }
        );
    }
}
