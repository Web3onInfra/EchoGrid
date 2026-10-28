//! # Startup Self-Checks & Readiness Evaluation
//!
//! Validates chain connectivity, configuration alignment, and account funding
//! on boot in live mode and exposes results for readiness reporting.

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::chain::rpc::SorobanRpcClient;
use crate::chain::views::ContractViewClient;

#[derive(Debug, thiserror::Error)]
pub enum SelfCheckError {
    #[error("Soroban RPC health check failed: {0}")]
    RpcUnreachable(String),
    #[error(
        "Oracle public key mismatch: contract expects '{expected}', local key is '{configured}'"
    )]
    OracleUnauthorized {
        expected: String,
        configured: String,
    },
    #[error("Failed to query contract view: {0}")]
    ContractQueryFailed(String),
    #[error("Oracle account '{0}' is not funded or missing on network")]
    OracleUnfunded(String),
}

/// Detailed results of the startup self-check sequence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelfCheckReport {
    pub rpc_healthy: bool,
    pub oracle_authorized: bool,
    pub contract_paused: bool,
    pub oracle_funded: bool,
    pub contract_id: String,
    pub oracle_address: String,
}

impl SelfCheckReport {
    /// Returns true if all critical checks pass and the contract is not paused.
    pub fn is_ready(&self) -> bool {
        self.rpc_healthy && self.oracle_authorized && !self.contract_paused && self.oracle_funded
    }
}

/// Runs the full startup self-check sequence against the chain and contract.
pub async fn run_startup_self_checks(
    rpc_client: &SorobanRpcClient,
    view_client: &ContractViewClient,
    expected_oracle_addr: &str,
    contract_id: &str,
) -> Result<SelfCheckReport, SelfCheckError> {
    info!(
        contract = contract_id,
        oracle = expected_oracle_addr,
        "Executing startup self-checks"
    );

    // 1. Verify RPC health
    let health = rpc_client
        .get_health()
        .await
        .map_err(|e| SelfCheckError::RpcUnreachable(e.to_string()))?;
    if health.status != "healthy" {
        return Err(SelfCheckError::RpcUnreachable(format!(
            "RPC reported status '{}'",
            health.status
        )));
    }

    // 2. Verify oracle public key matches contract configuration
    let config = view_client
        .get_config()
        .await
        .map_err(|e| SelfCheckError::ContractQueryFailed(e.to_string()))?;
    if config.oracle != expected_oracle_addr {
        return Err(SelfCheckError::OracleUnauthorized {
            expected: config.oracle,
            configured: expected_oracle_addr.to_string(),
        });
    }

    // 3. Verify pause status (warn and degrade, do not hard-fail boot)
    let contract_paused = view_client
        .is_paused()
        .await
        .map_err(|e| SelfCheckError::ContractQueryFailed(e.to_string()))?;
    if contract_paused {
        warn!(
            contract = contract_id,
            "Contract is currently PAUSED — payouts degraded"
        );
    }

    // 4. Verify oracle account is funded and has sequence number on ledger
    let oracle_funded = match rpc_client.get_account_sequence(expected_oracle_addr).await {
        Ok(_) => true,
        Err(e) => return Err(SelfCheckError::OracleUnfunded(e.to_string())),
    };

    info!(
        rpc_healthy = true,
        oracle_authorized = true,
        contract_paused,
        oracle_funded,
        "Startup self-checks completed successfully"
    );

    Ok(SelfCheckReport {
        rpc_healthy: true,
        oracle_authorized: true,
        contract_paused,
        oracle_funded,
        contract_id: contract_id.to_string(),
        oracle_address: expected_oracle_addr.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const CONTRACT_ID: &str = "CDFOK4NF4CY27N4G5PYA6SXTJA5MEKPVU7DQRMJUS74LIXHOKYNRHOPF";

    #[tokio::test]
    async fn test_startup_self_checks_success() {
        let server = MockServer::start().await;
        let fixtures_raw = include_str!("../../tests/fixtures/contract_views.json");
        let fixtures: serde_json::Value = serde_json::from_str(fixtures_raw).unwrap();

        // 1. getHealth
        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_string_contains("getHealth"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": { "status": "healthy" }
            })))
            .mount(&server)
            .await;

        // 2. get_config
        let cfg_b64 = fixtures["config_xdr"].as_str().unwrap();
        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_string_contains("simulateTransaction"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": { "results": [{ "xdr": cfg_b64 }] }
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        // 3. is_paused
        let paused_b64 = fixtures["paused_xdr"].as_str().unwrap();
        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_string_contains("simulateTransaction"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": { "results": [{ "xdr": paused_b64 }] }
            })))
            .mount(&server)
            .await;

        // 4. getAccount sequence
        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_string_contains("getAccount"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": { "sequence": "42" }
            })))
            .mount(&server)
            .await;

        let rpc = SorobanRpcClient::new(&server.uri(), Duration::from_secs(2));
        let views = ContractViewClient::new(rpc.clone(), CONTRACT_ID.into());

        // Note: fixture oracle public key decodes from bytes [2u8; 32]
        let expected_oracle = stellar_strkey::ed25519::PublicKey([2u8; 32])
            .to_string()
            .as_str()
            .to_string();

        let report = run_startup_self_checks(&rpc, &views, &expected_oracle, CONTRACT_ID)
            .await
            .expect("self-checks should succeed");

        assert!(report.is_ready());
        assert!(!report.contract_paused);
        assert!(report.oracle_authorized);
    }

    #[tokio::test]
    async fn test_startup_self_checks_oracle_mismatch() {
        let server = MockServer::start().await;
        let fixtures_raw = include_str!("../../tests/fixtures/contract_views.json");
        let fixtures: serde_json::Value = serde_json::from_str(fixtures_raw).unwrap();

        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_string_contains("getHealth"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": { "status": "healthy" }
            })))
            .mount(&server)
            .await;

        let cfg_b64 = fixtures["config_xdr"].as_str().unwrap();
        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_string_contains("simulateTransaction"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": { "results": [{ "xdr": cfg_b64 }] }
            })))
            .mount(&server)
            .await;

        let rpc = SorobanRpcClient::new(&server.uri(), Duration::from_secs(2));
        let views = ContractViewClient::new(rpc.clone(), CONTRACT_ID.into());

        let res = run_startup_self_checks(&rpc, &views, "G_WRONG_ORACLE_KEY", CONTRACT_ID).await;
        assert!(matches!(
            res,
            Err(SelfCheckError::OracleUnauthorized { .. })
        ));
    }
}
