use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

pub const DEFAULT_TESTNET_RPC_URL: &str = "https://soroban-testnet.stellar.org";
pub const DEFAULT_TESTNET_PASSPHRASE: &str = "Test SDF Network ; September 2015";

#[derive(Parser, Debug)]
#[command(name = "soroban_rpc", about = "EchoGrid Phase 0 Soroban RPC Spike CLI")]
pub struct Cli {
    #[arg(
        long,
        env = "SOROBAN_RPC_URL",
        default_value = DEFAULT_TESTNET_RPC_URL,
        help = "Stellar/Soroban JSON-RPC endpoint"
    )]
    pub rpc_url: String,

    #[arg(
        long,
        env = "STELLAR_NETWORK_PASSPHRASE",
        default_value = DEFAULT_TESTNET_PASSPHRASE,
        help = "Stellar Network Passphrase"
    )]
    pub network_passphrase: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Query RPC health and latest ledger sequence
    Health,
}

#[derive(Serialize)]
pub struct JsonRpcRequest<T: Serialize> {
    pub jsonrpc: &'static str,
    pub id: u64,
    pub method: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<T>,
}

#[derive(Deserialize, Debug)]
pub struct JsonRpcResponse<T> {
    pub jsonrpc: String,
    pub id: u64,
    pub result: Option<T>,
    pub error: Option<JsonRpcError>,
}

#[derive(Deserialize, Debug)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
}

#[derive(Deserialize, Debug, PartialEq, Eq)]
pub struct HealthResult {
    pub status: String,
}

#[derive(Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LatestLedgerResult {
    pub id: String,
    pub protocol_version: u32,
    pub sequence: u32,
}

pub struct SorobanRpcClient {
    client: reqwest::Client,
    rpc_url: String,
}

impl SorobanRpcClient {
    pub fn new(rpc_url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            rpc_url: rpc_url.to_string(),
        }
    }

    pub async fn get_health(&self) -> Result<HealthResult> {
        let request = JsonRpcRequest::<serde_json::Value> {
            jsonrpc: "2.0",
            id: 1,
            method: "getHealth",
            params: None,
        };

        let res = self
            .client
            .post(&self.rpc_url)
            .json(&request)
            .send()
            .await
            .context("Failed to send getHealth request")?;

        let rpc_res: JsonRpcResponse<HealthResult> = res
            .json()
            .await
            .context("Failed to parse getHealth JSON response")?;

        if let Some(err) = rpc_res.error {
            anyhow::bail!("getHealth RPC error ({}): {}", err.code, err.message);
        }

        rpc_res.result.context("Empty result in getHealth response")
    }

    pub async fn get_latest_ledger(&self) -> Result<LatestLedgerResult> {
        let request = JsonRpcRequest::<serde_json::Value> {
            jsonrpc: "2.0",
            id: 2,
            method: "getLatestLedger",
            params: None,
        };

        let res = self
            .client
            .post(&self.rpc_url)
            .json(&request)
            .send()
            .await
            .context("Failed to send getLatestLedger request")?;

        let rpc_res: JsonRpcResponse<LatestLedgerResult> = res
            .json()
            .await
            .context("Failed to parse getLatestLedger JSON response")?;

        if let Some(err) = rpc_res.error {
            anyhow::bail!("getLatestLedger RPC error ({}): {}", err.code, err.message);
        }

        rpc_res
            .result
            .context("Empty result in getLatestLedger response")
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();

    match cli.command {
        Commands::Health => {
            println!("Connecting to Soroban RPC: {}", cli.rpc_url);
            println!("Network Passphrase: {}", cli.network_passphrase);

            let client = SorobanRpcClient::new(&cli.rpc_url);

            let health = client.get_health().await?;
            println!("[OK] Health status: {}", health.status);

            let ledger = client.get_latest_ledger().await?;
            println!(
                "[OK] Latest Ledger Sequence: {} (Protocol v{}, Hash: {})",
                ledger.sequence, ledger.protocol_version, ledger.id
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_health_response() {
        let json_data = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "status": "healthy"
            }
        }"#;

        let parsed: JsonRpcResponse<HealthResult> =
            serde_json::from_str(json_data).expect("Must parse");
        assert_eq!(
            parsed.result,
            Some(HealthResult {
                status: "healthy".to_string()
            })
        );
    }

    #[test]
    fn test_deserialize_latest_ledger_response() {
        let json_data = r#"{
            "jsonrpc": "2.0",
            "id": 2,
            "result": {
                "id": "e0bfa9c3d4062a74c2fa67243de0cbb577e923e200bc50630fcbb9930f9a26be",
                "protocolVersion": 22,
                "sequence": 1045920
            }
        }"#;

        let parsed: JsonRpcResponse<LatestLedgerResult> =
            serde_json::from_str(json_data).expect("Must parse");
        let result = parsed.result.expect("Expected result");
        assert_eq!(result.sequence, 1045920);
        assert_eq!(result.protocol_version, 22);
    }
}
