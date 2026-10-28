//! # Contract Read Views
//!
//! Client for querying read-only contract state (`get_config`, `pool_balance`,
//! `is_paused`, `window_spent`) via Soroban RPC transaction simulation.

use serde::{Deserialize, Serialize};
use stellar_strkey::ed25519::PublicKey as StrkeyPublicKey;
use stellar_strkey::Contract as StrkeyContract;
use stellar_strkey::Strkey;
use stellar_xdr::curr::{
    AccountId, Hash, HostFunction, InvokeContractArgs, InvokeHostFunctionOp, Limits, Memo,
    MuxedAccount, Operation, OperationBody, Preconditions, PublicKey, ReadXdr, ScAddress, ScSymbol,
    ScVal, SequenceNumber, Transaction, TransactionEnvelope, TransactionV1Envelope, Uint256,
    WriteXdr,
};

use crate::chain::rpc::{RpcClientError, SorobanRpcClient};

#[derive(Debug, thiserror::Error)]
pub enum ViewError {
    #[error("RPC error: {0}")]
    Rpc(#[from] RpcClientError),
    #[error("Invalid strkey: {0}")]
    InvalidStrkey(String),
    #[error("XDR conversion error: {0}")]
    Xdr(String),
    #[error("Simulation failed or returned no result: {0}")]
    SimulationFailed(String),
    #[error("Failed to decode contract value: {0}")]
    DecodeError(String),
}

/// Strongly typed configuration returned by `get_config`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractConfig {
    pub municipality: String,
    pub oracle: String,
    pub token: String,
    pub bounty: i128,
    pub per_payout_cap: i128,
    pub window_len: u64,
    pub window_budget: i128,
    pub max_recipient_window_claims: u32,
}

/// Client for read-only Soroban contract view functions.
#[derive(Clone)]
pub struct ContractViewClient {
    rpc_client: SorobanRpcClient,
    contract_id: String,
}

impl ContractViewClient {
    pub fn new(rpc_client: SorobanRpcClient, contract_id: String) -> Self {
        Self {
            rpc_client,
            contract_id,
        }
    }

    /// Helper to simulate a zero-argument contract function and return the decoded `ScVal`.
    async fn call_read_fn(&self, fn_name: &str) -> Result<ScVal, ViewError> {
        let contract_bytes = match Strkey::from_string(&self.contract_id) {
            Ok(Strkey::Contract(c)) => c.0,
            _ => return Err(ViewError::InvalidStrkey(self.contract_id.clone())),
        };

        let symbol: ScSymbol = fn_name
            .try_into()
            .map_err(|e| ViewError::Xdr(format!("{e:?}")))?;
        let invoke_args = InvokeContractArgs {
            contract_address: ScAddress::Contract(Hash(contract_bytes)),
            function_name: symbol,
            args: vec![]
                .try_into()
                .map_err(|e| ViewError::Xdr(format!("{e:?}")))?,
        };

        let op = Operation {
            source_account: None,
            body: OperationBody::InvokeHostFunction(InvokeHostFunctionOp {
                host_function: HostFunction::InvokeContract(invoke_args),
                auth: vec![]
                    .try_into()
                    .map_err(|e| ViewError::Xdr(format!("{e:?}")))?,
            }),
        };

        let tx = Transaction {
            source_account: MuxedAccount::Ed25519(Uint256([0u8; 32])),
            fee: 100,
            seq_num: SequenceNumber(0),
            cond: Preconditions::None,
            memo: Memo::None,
            operations: vec![op]
                .try_into()
                .map_err(|e| ViewError::Xdr(format!("{e:?}")))?,
            ext: stellar_xdr::curr::TransactionExt::V0,
        };

        let envelope_b64 = TransactionEnvelope::Tx(TransactionV1Envelope {
            tx,
            signatures: vec![]
                .try_into()
                .map_err(|e| ViewError::Xdr(format!("{e:?}")))?,
        })
        .to_xdr_base64(Limits::none())
        .map_err(|e| ViewError::Xdr(format!("{e:?}")))?;

        let sim_res = self.rpc_client.simulate_transaction(&envelope_b64).await?;
        if let Some(err) = sim_res.error {
            return Err(ViewError::SimulationFailed(err));
        }

        let first_res = sim_res
            .results
            .and_then(|r| r.into_iter().next())
            .and_then(|e| e.xdr)
            .ok_or_else(|| {
                ViewError::SimulationFailed("No return value in simulation result".into())
            })?;

        ScVal::from_xdr_base64(&first_res, Limits::none())
            .map_err(|e| ViewError::Xdr(format!("{e:?}")))
    }

    pub async fn is_paused(&self) -> Result<bool, ViewError> {
        match self.call_read_fn("is_paused").await? {
            ScVal::Bool(b) => Ok(b),
            other => Err(ViewError::DecodeError(format!(
                "Expected Bool, got {other:?}"
            ))),
        }
    }

    pub async fn pool_balance(&self) -> Result<i128, ViewError> {
        decode_i128(&self.call_read_fn("pool_balance").await?)
    }

    pub async fn window_spent(&self) -> Result<i128, ViewError> {
        decode_i128(&self.call_read_fn("window_spent").await?)
    }

    pub async fn get_config(&self) -> Result<ContractConfig, ViewError> {
        decode_config(&self.call_read_fn("get_config").await?)
    }
}

pub fn decode_i128(val: &ScVal) -> Result<i128, ViewError> {
    match val {
        ScVal::I128(p) => Ok(((p.hi as i128) << 64) | (p.lo as i128)),
        ScVal::I64(v) => Ok(*v as i128),
        ScVal::U64(v) => Ok(*v as i128),
        ScVal::U32(v) => Ok(*v as i128),
        other => Err(ViewError::DecodeError(format!(
            "Expected i128, got {other:?}"
        ))),
    }
}

pub fn decode_address(val: &ScVal) -> Result<String, ViewError> {
    match val {
        ScVal::Address(ScAddress::Account(AccountId(PublicKey::PublicKeyTypeEd25519(
            Uint256(b),
        )))) => Ok(StrkeyPublicKey(*b).to_string().as_str().to_string()),
        ScVal::Address(ScAddress::Contract(Hash(b))) => {
            Ok(StrkeyContract(*b).to_string().as_str().to_string())
        }
        other => Err(ViewError::DecodeError(format!(
            "Expected Address, got {other:?}"
        ))),
    }
}

pub fn decode_config(val: &ScVal) -> Result<ContractConfig, ViewError> {
    let map = match val {
        ScVal::Map(Some(m)) => m,
        other => {
            return Err(ViewError::DecodeError(format!(
                "Expected ScVal::Map, got {other:?}"
            )))
        }
    };

    let (mut muni, mut ora, mut tok) = (None, None, None);
    let (mut bnty, mut cap, mut w_len, mut w_bdgt, mut r_claims) = (None, None, None, None, None);

    for entry in map.iter() {
        if let ScVal::Symbol(sym) = &entry.key {
            match sym.to_string().as_str() {
                "municipality" => muni = Some(decode_address(&entry.val)?),
                "oracle" => ora = Some(decode_address(&entry.val)?),
                "token" => tok = Some(decode_address(&entry.val)?),
                "bounty" => bnty = Some(decode_i128(&entry.val)?),
                "per_payout_cap" => cap = Some(decode_i128(&entry.val)?),
                "window_len" => {
                    w_len = match entry.val {
                        ScVal::U64(v) => Some(v),
                        ScVal::U32(v) => Some(v as u64),
                        _ => None,
                    }
                }
                "window_budget" => w_bdgt = Some(decode_i128(&entry.val)?),
                "max_recipient_window_claims" => {
                    r_claims = match entry.val {
                        ScVal::U32(v) => Some(v),
                        _ => None,
                    }
                }
                _ => {}
            }
        }
    }

    Ok(ContractConfig {
        municipality: muni.ok_or_else(|| ViewError::DecodeError("Missing municipality".into()))?,
        oracle: ora.ok_or_else(|| ViewError::DecodeError("Missing oracle".into()))?,
        token: tok.ok_or_else(|| ViewError::DecodeError("Missing token".into()))?,
        bounty: bnty.ok_or_else(|| ViewError::DecodeError("Missing bounty".into()))?,
        per_payout_cap: cap
            .ok_or_else(|| ViewError::DecodeError("Missing per_payout_cap".into()))?,
        window_len: w_len.ok_or_else(|| ViewError::DecodeError("Missing window_len".into()))?,
        window_budget: w_bdgt
            .ok_or_else(|| ViewError::DecodeError("Missing window_budget".into()))?,
        max_recipient_window_claims: r_claims
            .ok_or_else(|| ViewError::DecodeError("Missing max_recipient_window_claims".into()))?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn mock_sim_res(xdr_b64: &str) -> ResponseTemplate {
        ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": { "results": [{ "xdr": xdr_b64 }] }
        }))
    }

    #[tokio::test]
    async fn test_views_queries_and_decoding() {
        let fixtures_raw = include_str!("../../tests/fixtures/contract_views.json");
        let fixtures: serde_json::Value = serde_json::from_str(fixtures_raw).unwrap();

        let server = MockServer::start().await;

        let paused_b64 = fixtures["paused_xdr"].as_str().unwrap();
        let bal_b64 = fixtures["balance_xdr"].as_str().unwrap();
        let spnt_b64 = fixtures["spent_xdr"].as_str().unwrap();
        let cfg_b64 = fixtures["config_xdr"].as_str().unwrap();

        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_string_contains("simulateTransaction"))
            .respond_with(mock_sim_res(paused_b64))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_string_contains("simulateTransaction"))
            .respond_with(mock_sim_res(bal_b64))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_string_contains("simulateTransaction"))
            .respond_with(mock_sim_res(spnt_b64))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_string_contains("simulateTransaction"))
            .respond_with(mock_sim_res(cfg_b64))
            .mount(&server)
            .await;

        let rpc = SorobanRpcClient::new(&server.uri(), Duration::from_secs(2));
        let client = ContractViewClient::new(
            rpc,
            "CDFOK4NF4CY27N4G5PYA6SXTJA5MEKPVU7DQRMJUS74LIXHOKYNRHOPF".into(),
        );

        assert!(!client.is_paused().await.unwrap());
        assert_eq!(client.pool_balance().await.unwrap(), 50_000);
        assert_eq!(client.window_spent().await.unwrap(), 10_000);

        let cfg = client.get_config().await.unwrap();
        assert_eq!(cfg.bounty, 1_000);
        assert_eq!(cfg.per_payout_cap, 5_000);
        assert_eq!(cfg.window_len, 86400);
        assert_eq!(cfg.window_budget, 20_000);
        assert_eq!(cfg.max_recipient_window_claims, 2);
    }
}
