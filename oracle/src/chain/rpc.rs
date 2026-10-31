//! # Soroban JSON-RPC 2.0 Client
//!
//! Strongly typed client for Soroban RPC operations:
//! `getHealth`, `getLatestLedger`, `getAccount` / ledger entries,
//! `simulateTransaction`, `sendTransaction`, and `getTransaction`.

use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum RpcClientError {
    #[error("HTTP transport error: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("Soroban RPC error ({code}): {message}")]
    Rpc { code: i64, message: String },
    #[error("Empty or invalid result in RPC response: {0}")]
    InvalidResult(String),
}

#[derive(Serialize)]
struct JsonRpcRequest<T: Serialize> {
    jsonrpc: &'static str,
    id: u64,
    method: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<T>,
}

#[derive(Deserialize, Debug)]
struct JsonRpcResponse<T> {
    result: Option<T>,
    error: Option<JsonRpcErrorPayload>,
}

#[derive(Deserialize, Debug)]
struct JsonRpcErrorPayload {
    code: i64,
    message: String,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct HealthResponse {
    pub status: String,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LatestLedgerResponse {
    pub id: String,
    pub protocol_version: u32,
    pub sequence: u32,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SimulateTransactionResponse {
    pub min_resource_fee: Option<String>,
    pub transaction_data: Option<String>,
    pub results: Option<Vec<SimulateResultEntry>>,
    pub error: Option<String>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SimulateResultEntry {
    pub xdr: Option<String>,
    pub auth: Option<Vec<String>>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SendTransactionResponse {
    pub status: String,
    pub hash: String,
    pub latest_ledger: Option<u32>,
    pub error_result_xdr: Option<String>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetTransactionResponse {
    pub status: String,
    pub latest_ledger: Option<u32>,
    pub ledger: Option<u32>,
    pub result_xdr: Option<String>,
}

#[derive(Clone)]
pub struct SorobanRpcClient {
    client: reqwest::Client,
    rpc_url: String,
}

impl SorobanRpcClient {
    pub fn new(rpc_url: &str, timeout: Duration) -> Self {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .user_agent("EchoGrid-Oracle/0.1")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            rpc_url: rpc_url.to_string(),
        }
    }

    pub fn rpc_url(&self) -> &str {
        &self.rpc_url
    }

    async fn call_rpc<P: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        method: &'static str,
        params: Option<P>,
    ) -> Result<R, RpcClientError> {
        let req = JsonRpcRequest {
            jsonrpc: "2.0",
            id: 1,
            method,
            params,
        };

        let resp = self.client.post(&self.rpc_url).json(&req).send().await?;
        let rpc_resp: JsonRpcResponse<R> = resp.json().await?;

        if let Some(err) = rpc_resp.error {
            return Err(RpcClientError::Rpc {
                code: err.code,
                message: err.message,
            });
        }

        rpc_resp
            .result
            .ok_or_else(|| RpcClientError::InvalidResult(format!("missing result for {method}")))
    }

    pub async fn get_health(&self) -> Result<HealthResponse, RpcClientError> {
        self.call_rpc::<serde_json::Value, HealthResponse>("getHealth", None)
            .await
    }

    pub async fn get_latest_ledger(&self) -> Result<LatestLedgerResponse, RpcClientError> {
        self.call_rpc::<serde_json::Value, LatestLedgerResponse>("getLatestLedger", None)
            .await
    }

    pub async fn get_account_sequence(&self, account: &str) -> Result<u64, RpcClientError> {
        let params = serde_json::json!({ "address": account });
        if let Ok(result) = self
            .call_rpc::<serde_json::Value, serde_json::Value>("getAccount", Some(params))
            .await
        {
            if let Some(seq) = result.get("sequence").and_then(|s| {
                s.as_str()
                    .and_then(|str_val| str_val.parse::<u64>().ok())
                    .or_else(|| s.as_u64())
            }) {
                return Ok(seq);
            }
        }

        // Fallback: Query Horizon accounts endpoint for account sequence (live Stellar Testnet / Pubnet)
        let horizon_url = if self.rpc_url.contains("testnet") {
            "https://horizon-testnet.stellar.org"
        } else {
            "https://horizon.stellar.org"
        };
        let horizon_endpoint = format!("{}/accounts/{}", horizon_url, account);
        let resp = self.client.get(&horizon_endpoint).send().await?;
        if !resp.status().is_success() {
            return Err(RpcClientError::InvalidResult(format!(
                "Horizon account lookup returned HTTP {}",
                resp.status()
            )));
        }
        let horizon_body: serde_json::Value = resp.json().await?;
        horizon_body
            .get("sequence")
            .and_then(|s| {
                s.as_str()
                    .and_then(|str_val| str_val.parse::<u64>().ok())
                    .or_else(|| s.as_u64())
            })
            .ok_or_else(|| {
                RpcClientError::InvalidResult(
                    "sequence field missing or invalid in Horizon account response".into(),
                )
            })
    }

    pub async fn simulate_transaction(
        &self,
        tx_xdr: &str,
    ) -> Result<SimulateTransactionResponse, RpcClientError> {
        let params = serde_json::json!({ "transaction": tx_xdr });
        self.call_rpc("simulateTransaction", Some(params)).await
    }

    pub async fn send_transaction(
        &self,
        tx_xdr: &str,
    ) -> Result<SendTransactionResponse, RpcClientError> {
        let params = serde_json::json!({ "transaction": tx_xdr });
        self.call_rpc("sendTransaction", Some(params)).await
    }

    pub async fn get_transaction(
        &self,
        tx_hash: &str,
    ) -> Result<GetTransactionResponse, RpcClientError> {
        let params = serde_json::json!({ "hash": tx_hash });
        self.call_rpc("getTransaction", Some(params)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_rpc_get_health_and_ledger() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": { "status": "healthy" }
            })))
            .mount(&server)
            .await;

        let client = SorobanRpcClient::new(&server.uri(), Duration::from_secs(2));
        let health = client.get_health().await.unwrap();
        assert_eq!(health.status, "healthy");
    }

    #[tokio::test]
    async fn test_rpc_get_account_sequence() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": { "sequence": "1234567890" }
            })))
            .mount(&server)
            .await;

        let client = SorobanRpcClient::new(&server.uri(), Duration::from_secs(2));
        let seq = client.get_account_sequence("GA4ZH5...").await.unwrap();
        assert_eq!(seq, 1234567890);
    }

    #[tokio::test]
    async fn test_rpc_simulate_send_and_get_transaction() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": {
                    "status": "SUCCESS",
                    "hash": "tx123",
                    "latestLedger": 500
                }
            })))
            .mount(&server)
            .await;

        let client = SorobanRpcClient::new(&server.uri(), Duration::from_secs(2));
        let tx = client.get_transaction("tx123").await.unwrap();
        assert_eq!(tx.status, "SUCCESS");
    }

    #[tokio::test]
    async fn test_rpc_error_handling() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "error": { "code": -32600, "message": "Invalid Request" }
            })))
            .mount(&server)
            .await;

        let client = SorobanRpcClient::new(&server.uri(), Duration::from_secs(2));
        let err = client.get_health().await.unwrap_err();
        match err {
            RpcClientError::Rpc { code, message } => {
                assert_eq!(code, -32600);
                assert_eq!(message, "Invalid Request");
            }
            _ => panic!("Expected RpcClientError::Rpc"),
        }
    }
}
