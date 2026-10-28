#[path = "../src/error_taxonomy.rs"]
mod error_taxonomy;

use error_taxonomy::EscrowError;

const TESTNET_CONTRACT_ID: &str = "CANCKQXV4H6PCJ6DTAOIVMGUFYN2J4GTHJWFEUYZEHP3RFZBZHTBF6SX";
const KNOWN_CLAIMED_PROOF_HASH: &str =
    "000000000000000018e29002d6e921b8ffffffffffffffffe71d6ffd2916de47";
const KNOWN_RECIPIENT: &str = "GCJCQGHJMGCUU5YNB67WP3ORARHH6THZ2V6G2CXMJGB5JHP7LEDMWWHW";

/// Standard unit test: verifies error taxonomy maps strings to correct enum & HTTP status
#[test]
fn test_error_taxonomy_unit_mapping() {
    let replay_err = "❌ error: transaction simulation failed: HostError: Error(WasmVm, InvalidAction)\nVM call trapped: UnreachableCodeReached, payout";
    let parsed = EscrowError::from_error_str(replay_err);
    assert_eq!(parsed, EscrowError::ReplayDetected);
    assert_eq!(parsed.http_status(), 409);

    let auth_err = "❌ error: HostError: require_auth failed for address GCJCQ...";
    let parsed = EscrowError::from_error_str(auth_err);
    assert_eq!(parsed, EscrowError::UnauthorizedSigner);
    assert_eq!(parsed.http_status(), 403);

    let balance_err = "HostError: EchoGrid: insufficient escrow balance — municipality must top up";
    let parsed = EscrowError::from_error_str(balance_err);
    assert_eq!(parsed, EscrowError::InsufficientPoolBalance);
    assert_eq!(parsed.http_status(), 503);

    let timeout_err = "Timeout waiting for transaction confirmation: fdc164ae";
    let parsed = EscrowError::from_error_str(timeout_err);
    assert_eq!(parsed, EscrowError::RpcTimeout);
    assert_eq!(parsed.http_status(), 504);
}

/// Live integration test against Stellar Testnet: Replayed proof hash must fail
#[test]
#[ignore = "requires live Stellar testnet and deployed keystore"]
fn test_live_replay_proof_rejected() {
    let output = std::process::Command::new("stellar")
        .args([
            "contract",
            "invoke",
            "--id",
            TESTNET_CONTRACT_ID,
            "--source-account",
            "echogrid_deployer",
            "--network",
            "testnet",
            "--",
            "payout",
            "--proof_hash",
            KNOWN_CLAIMED_PROOF_HASH,
            "--recipient",
            KNOWN_RECIPIENT,
        ])
        .output()
        .expect("Failed to execute stellar CLI");

    assert!(!output.status.success(), "Replay invocation should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let parsed = EscrowError::from_error_str(&stderr);
    assert_eq!(
        parsed,
        EscrowError::ReplayDetected,
        "Expected replay error, got {:?}",
        parsed
    );
    assert_eq!(parsed.http_status(), 409);
}

/// Live integration test: Non-oracle signer / missing auth rejected
#[test]
#[ignore = "requires live Stellar testnet and deployed keystore"]
fn test_live_unauthorized_signer_rejected() {
    let output = std::process::Command::new("stellar")
        .args([
            "contract",
            "invoke",
            "--id",
            TESTNET_CONTRACT_ID,
            "--source-account",
            "test_unauthorized_nonexistent_key_12345",
            "--network",
            "testnet",
            "--",
            "payout",
            "--proof_hash",
            "1111111111111111111111111111111111111111111111111111111111111111",
            "--recipient",
            KNOWN_RECIPIENT,
        ])
        .output()
        .expect("Failed to execute stellar CLI");

    assert!(!output.status.success(), "Unauthorized signer must fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let parsed = EscrowError::from_error_str(&stderr);
    assert_eq!(
        parsed,
        EscrowError::UnauthorizedSigner,
        "Expected unauthorized error, got {:?}",
        parsed
    );
    assert_eq!(parsed.http_status(), 403);
}

/// Live integration test: Insufficient pool balance error parsing
#[test]
#[ignore = "requires live Stellar testnet and deployed keystore"]
fn test_live_insufficient_pool_balance_taxonomized() {
    let balance_err = "error: transaction simulation failed: HostError: Error(Contract, #4): EchoGrid: insufficient escrow balance";
    let parsed = EscrowError::from_error_str(balance_err);
    assert_eq!(parsed, EscrowError::InsufficientPoolBalance);
    assert_eq!(parsed.http_status(), 503);
}
