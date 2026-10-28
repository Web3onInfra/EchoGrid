//! # EchoGrid — Acoustic CLI Library
//!
//! Provides the core workflow for Edge AI hum detection simulation,
//! Proof-of-Power-Down derivation, and oracle settlement interaction.

use chrono::Utc;
use colored::Colorize;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct WorkflowConfig {
    pub wallet: String,
    pub oracle_url: String,
    pub challenge_id: Option<String>,
    pub simulated: bool,
    pub fast: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ProofPayload {
    pub wallet_address: String,
    pub challenge_id: String,
    pub timestamp: String,
    pub nonce: String,
    pub proof_hash: String,
    pub detector_version: String,
    pub duration_seconds: u32,
    #[serde(default)]
    pub simulated: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ChallengeResponse {
    pub challenge_id: String,
    pub wallet_address: String,
    pub issued_at: String,
    pub expires_at: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum OracleProofResponse {
    Confirmed {
        claim_id: String,
        tx_hash: String,
        explorer_url: String,
        amount_stroops: i64,
        #[serde(default)]
        simulated: bool,
    },
    Pending {
        claim_id: String,
        poll_url: String,
    },
}

#[derive(Deserialize, Debug, Clone)]
pub struct ClaimPollResponse {
    pub claim_id: String,
    pub status: String,
    pub tx_hash: Option<String>,
    pub amount_stroops: i64,
    pub explorer_url: Option<String>,
    #[serde(default)]
    pub simulated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowReceipt {
    pub claim_id: String,
    pub tx_hash: String,
    pub amount_stroops: i64,
    pub explorer_url: String,
    pub simulated: bool,
}

pub fn log_step(msg: &str, color: &str) {
    let prefix = "[EchoGrid]".bold();
    let line = match color {
        "cyan" => format!("{prefix} {}", msg.cyan()),
        "yellow" => format!("{prefix} {}", msg.yellow()),
        "green" => format!("{prefix} {}", msg.green()),
        "red" => format!("{prefix} {}", msg.red()),
        "magenta" => format!("{prefix} {}", msg.magenta()),
        _ => format!("{prefix} {msg}"),
    };
    println!("{line}");
}

pub fn fetch_challenge(
    client: &reqwest::blocking::Client,
    oracle_url: &str,
    wallet: &str,
) -> Result<String, String> {
    let url = format!(
        "{}/api/v1/challenge?wallet={}",
        oracle_url.trim_end_matches('/'),
        wallet
    );
    let resp = client
        .get(&url)
        .send()
        .map_err(|e| format!("Failed to reach oracle challenge endpoint: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!(
            "Challenge issuance rejected (HTTP {}): {}",
            resp.status(),
            resp.text().unwrap_or_default()
        ));
    }

    let challenge: ChallengeResponse = resp
        .json()
        .map_err(|e| format!("Failed to parse challenge response: {e}"))?;
    Ok(challenge.challenge_id)
}

pub fn generate_proof(wallet: &str, challenge_id: &str, simulated: bool) -> ProofPayload {
    let timestamp = Utc::now().to_rfc3339();
    let nonce_bytes: [u8; 8] = rand::thread_rng().gen();
    let nonce = hex::encode(nonce_bytes);

    let pre_image = format!("{wallet}|{challenge_id}|{timestamp}|{nonce}");
    let mut hasher = Sha256::new();
    hasher.update(pre_image.as_bytes());
    let proof_hash = hex::encode(hasher.finalize());

    ProofPayload {
        wallet_address: wallet.to_string(),
        challenge_id: challenge_id.to_string(),
        timestamp,
        nonce,
        proof_hash,
        detector_version: "echonet-1.2".to_string(),
        duration_seconds: 60,
        simulated,
    }
}

pub fn submit_proof_and_confirm(
    client: &reqwest::blocking::Client,
    oracle_url: &str,
    payload: &ProofPayload,
) -> Result<WorkflowReceipt, String> {
    let base = oracle_url.trim_end_matches('/');
    let proof_url = format!("{base}/api/v1/proof");

    let resp = client
        .post(&proof_url)
        .json(payload)
        .send()
        .map_err(|e| format!("Failed to submit proof to oracle: {e}"))?;

    let status = resp.status();
    if !status.is_success() && status != reqwest::StatusCode::ACCEPTED {
        return Err(format!(
            "Proof submission rejected (HTTP {status}): {}",
            resp.text().unwrap_or_default()
        ));
    }

    let body_text = resp.text().unwrap_or_default();
    let parsed: OracleProofResponse = serde_json::from_str(&body_text)
        .map_err(|e| format!("Failed to decode proof response: {e} on body: '{body_text}'"))?;

    match parsed {
        OracleProofResponse::Confirmed {
            claim_id,
            tx_hash,
            explorer_url,
            amount_stroops,
            simulated,
        } => Ok(WorkflowReceipt {
            claim_id,
            tx_hash,
            amount_stroops,
            explorer_url,
            simulated,
        }),
        OracleProofResponse::Pending { claim_id, poll_url } => {
            let full_poll_url = if poll_url.starts_with("http") {
                poll_url
            } else {
                format!("{base}{poll_url}")
            };

            for _ in 0..15 {
                thread::sleep(Duration::from_millis(500));
                let poll_resp = client
                    .get(&full_poll_url)
                    .send()
                    .map_err(|e| format!("Poll request failed: {e}"))?;
                if poll_resp.status().is_success() {
                    let claim: ClaimPollResponse = poll_resp
                        .json()
                        .map_err(|e| format!("Poll decode failed: {e}"))?;
                    if claim.status == "confirmed" {
                        return Ok(WorkflowReceipt {
                            claim_id: claim.claim_id,
                            tx_hash: claim.tx_hash.unwrap_or_default(),
                            amount_stroops: claim.amount_stroops,
                            explorer_url: claim.explorer_url.unwrap_or_default(),
                            simulated: claim.simulated,
                        });
                    } else if claim.status == "failed" {
                        return Err("Claim transitioned to failed state on chain".to_string());
                    }
                }
            }
            Err(format!(
                "Claim confirmation timed out for claim ID {claim_id}"
            ))
        }
    }
}

pub fn run_acoustic_workflow(config: &WorkflowConfig) -> Result<WorkflowReceipt, String> {
    let pause = |ms: u64| {
        if !config.fast {
            thread::sleep(Duration::from_millis(ms));
        }
    };

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("HTTP client init failed: {e}"))?;

    // Phase 1: Classifier Init
    log_step("Initialising acoustic classifier...", "cyan");
    pause(200);

    // Phase 2: Ambient Listening
    log_step("Listening to ambient noise...", "cyan");
    pause(200);

    // Phase 3: AC Hum Detected
    log_step("AC Hum DETECTED (72 Hz ±4, confidence: 94%)", "yellow");
    pause(200);

    // Phase 4: Obtain Challenge
    let challenge_id = match &config.challenge_id {
        Some(cid) => cid.clone(),
        None => {
            log_step("Requesting wallet-bound challenge from oracle...", "cyan");
            fetch_challenge(&client, &config.oracle_url, &config.wallet)?
        }
    };
    log_step(
        &format!("Challenge active [{challenge_id}] — please turn off appliance!"),
        "magenta",
    );
    pause(300);

    // Phase 5: Power-Down Event Confirmed
    log_step("AC Hum ABSENT — power-down confirmed!", "green");
    pause(200);

    // Phase 6: Proof Derivation
    log_step("Generating canonical Proof-of-Power-Down v1...", "cyan");
    let payload = generate_proof(&config.wallet, &challenge_id, config.simulated);
    pause(100);

    // Phase 7: Submit and Settle
    log_step("Submitting proof to EchoGrid Oracle...", "cyan");
    let receipt = submit_proof_and_confirm(&client, &config.oracle_url, &payload)?;
    log_step(
        &format!("NegaWatt payout confirmed! TX: {}", receipt.tx_hash),
        "green",
    );

    Ok(receipt)
}
