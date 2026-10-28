use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

pub const DEFAULT_TESTNET_RPC_URL: &str = "https://soroban-testnet.stellar.org";
pub const DEFAULT_TESTNET_PASSPHRASE: &str = "Test SDF Network ; September 2015";
pub const DEFAULT_CONTRACT_ID: &str = "CANCKQXV4H6PCJ6DTAOIVMGUFYN2J4GTHJWFEUYZEHP3RFZBZHTBF6SX";
pub const DEFAULT_SOURCE_ACCOUNT: &str = "echogrid_deployer";
pub const DEFAULT_RECIPIENT: &str = "GCJCQGHJMGCUU5YNB67WP3ORARHH6THZ2V6G2CXMJGB5JHP7LEDMWWHW";
pub const DEFAULT_PAYOUT_AMOUNT: i128 = 10_000_000; // 1 XLM = 10,000,000 stroops

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

    /// Execute or verify a payout invocation on Stellar Testnet
    Payout {
        #[arg(
            long,
            env = "ESCROW_CONTRACT_ID",
            default_value = DEFAULT_CONTRACT_ID,
            help = "Contract ID of deployed negawatt_escrow"
        )]
        contract_id: String,

        #[arg(
            long,
            env = "STELLAR_SOURCE_ACCOUNT",
            default_value = DEFAULT_SOURCE_ACCOUNT,
            help = "Source account / keystore identity for signing"
        )]
        source_account: String,

        #[arg(
            long,
            env = "PAYOUT_RECIPIENT",
            default_value = DEFAULT_RECIPIENT,
            help = "Recipient Stellar address"
        )]
        recipient: String,

        #[arg(
            long,
            default_value_t = DEFAULT_PAYOUT_AMOUNT,
            help = "Payout amount in stroops (1 XLM = 10,000,000 stroops)"
        )]
        amount: i128,

        #[arg(
            long,
            help = "32-byte hex proof hash (generates random hash if omitted)"
        )]
        proof_hash: Option<String>,

        #[arg(
            long,
            help = "Existing transaction hash to poll and verify via Soroban RPC"
        )]
        tx_hash: Option<String>,
    },
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

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetTransactionResult {
    pub status: String,
    pub latest_ledger: Option<u32>,
    pub ledger: Option<u32>,
    pub created_at: Option<String>,
}

pub struct SorobanRpcClient {
    client: reqwest::Client,
    rpc_url: String,
}

impl SorobanRpcClient {
    pub fn new(rpc_url: &str) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("EchoGrid-Spike/0.1")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
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

    pub async fn get_transaction(&self, hash: &str) -> Result<GetTransactionResult> {
        let params = serde_json::json!({ "hash": hash });
        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            id: 3,
            method: "getTransaction",
            params: Some(params),
        };

        let res = self
            .client
            .post(&self.rpc_url)
            .json(&request)
            .send()
            .await
            .context("Failed to send getTransaction request")?;

        let rpc_res: JsonRpcResponse<GetTransactionResult> = res
            .json()
            .await
            .context("Failed to parse getTransaction JSON response")?;

        if let Some(err) = rpc_res.error {
            anyhow::bail!("getTransaction RPC error ({}): {}", err.code, err.message);
        }

        rpc_res
            .result
            .context("Empty result in getTransaction response")
    }

    pub async fn poll_transaction(
        &self,
        hash: &str,
        timeout_secs: u64,
    ) -> Result<GetTransactionResult> {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_secs);

        loop {
            let res = self.get_transaction(hash).await?;
            if res.status == "SUCCESS" {
                return Ok(res);
            } else if res.status == "FAILED" {
                anyhow::bail!("Transaction failed on ledger: {}", hash);
            }

            if start.elapsed() > timeout {
                anyhow::bail!("Timeout waiting for transaction confirmation: {}", hash);
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }
}

pub fn generate_random_proof_hash() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{:032x}{:032x}", nanos, !nanos)
}

pub fn extract_tx_hash(output: &str) -> Option<String> {
    for line in output.lines() {
        if line.contains("Signing transaction:") {
            if let Some(hash) = line.split("Signing transaction:").nth(1) {
                let clean = hash.trim();
                if clean.len() == 64 && clean.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Some(clean.to_string());
                }
            }
        }
        if line.contains("/tx/") {
            if let Some(hash) = line.split("/tx/").nth(1) {
                let clean = hash.trim_matches(|c: char| !c.is_ascii_hexdigit());
                if clean.len() == 64 {
                    return Some(clean.to_string());
                }
            }
        }
    }
    None
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();
    let client = SorobanRpcClient::new(&cli.rpc_url);

    match cli.command {
        Commands::Health => {
            println!("Connecting to Soroban RPC: {}", cli.rpc_url);
            println!("Network Passphrase: {}", cli.network_passphrase);

            let health = client.get_health().await?;
            println!("[OK] Health status: {}", health.status);

            let ledger = client.get_latest_ledger().await?;
            println!(
                "[OK] Latest Ledger Sequence: {} (Protocol v{}, Hash: {})",
                ledger.sequence, ledger.protocol_version, ledger.id
            );
        }
        Commands::Payout {
            contract_id,
            source_account,
            recipient,
            amount: _,
            proof_hash,
            tx_hash,
        } => {
            let final_tx_hash = if let Some(hash) = tx_hash {
                println!("Verifying existing transaction via Soroban RPC: {}", hash);
                hash
            } else {
                let proof = proof_hash.unwrap_or_else(generate_random_proof_hash);
                println!("Submitting payout invocation to Stellar Testnet...");
                println!("  Contract:       {}", contract_id);
                println!("  Source Account: {}", source_account);
                println!("  Recipient:      {}", recipient);
                println!("  Proof Hash:     {}", proof);

                let output = std::process::Command::new("stellar")
                    .args([
                        "contract",
                        "invoke",
                        "--id",
                        &contract_id,
                        "--source-account",
                        &source_account,
                        "--network",
                        "testnet",
                        "--",
                        "payout",
                        "--proof_hash",
                        &proof,
                        "--recipient",
                        &recipient,
                    ])
                    .output()
                    .context("Failed to execute stellar CLI invocation")?;

                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let full_output = format!("{}\n{}", stdout, stderr);

                if !output.status.success() {
                    anyhow::bail!("Stellar CLI invocation failed:\n{}", full_output);
                }

                extract_tx_hash(&full_output)
                    .context("Failed to parse transaction hash from CLI output")?
            };

            println!("\n[OK] Transaction hash: {}", final_tx_hash);
            println!("Polling Soroban RPC getTransaction for inclusion...");
            let tx_result = client.poll_transaction(&final_tx_hash, 30).await?;

            println!("Status:        {}", tx_result.status);
            if let Some(ledger) = tx_result.ledger {
                println!("Ledger:        {}", ledger);
            }
            if let Some(created_at) = tx_result.created_at {
                println!("Created At:    {}", created_at);
            }
            println!(
                "StellarExpert: https://stellar.expert/explorer/testnet/tx/{}",
                final_tx_hash
            );
            println!(
                "Contract:      https://stellar.expert/explorer/testnet/contract/{}",
                contract_id
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

    #[test]
    fn test_deserialize_get_transaction_response() {
        let json_data = r#"{
            "jsonrpc": "2.0",
            "id": 3,
            "result": {
                "status": "SUCCESS",
                "latestLedger": 5426021,
                "ledger": 5426019,
                "createdAt": "1793153735"
            }
        }"#;

        let parsed: JsonRpcResponse<GetTransactionResult> =
            serde_json::from_str(json_data).expect("Must parse");
        let result = parsed.result.expect("Expected result");
        assert_eq!(result.status, "SUCCESS");
        assert_eq!(result.ledger, Some(5426019));
        assert_eq!(result.latest_ledger, Some(5426021));
        assert_eq!(result.created_at, Some("1793153735".to_string()));
    }

    #[test]
    fn test_extract_tx_hash_from_cli_output() {
        let sample = "ℹ️ Signing transaction: 4e8e6154ba3cae2c8e815b7067750cdb4178ab9ba70ad57aa834846185936506\n✅ Transaction submitted successfully!";
        let extracted = extract_tx_hash(sample);
        assert_eq!(
            extracted,
            Some("4e8e6154ba3cae2c8e815b7067750cdb4178ab9ba70ad57aa834846185936506".to_string())
        );

        let url_sample = "🔗 https://stellar.expert/explorer/testnet/tx/4e8e6154ba3cae2c8e815b7067750cdb4178ab9ba70ad57aa834846185936506";
        let extracted_url = extract_tx_hash(url_sample);
        assert_eq!(
            extracted_url,
            Some("4e8e6154ba3cae2c8e815b7067750cdb4178ab9ba70ad57aa834846185936506".to_string())
        );
    }

    #[test]
    fn test_generate_random_proof_hash_length() {
        let hash = generate_random_proof_hash();
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
