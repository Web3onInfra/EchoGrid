//! End-to-end scenario tests for Acoustic CLI against mock and live oracles.

use acoustic_cli::{run_acoustic_workflow, WorkflowConfig};
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TEST_WALLET: &str = "GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT";

#[tokio::test]
async fn test_offline_acoustic_workflow_mock_happy_path() {
    let mock_server = MockServer::start().await;

    // 1. Mock challenge endpoint
    Mock::given(method("GET"))
        .and(path("/api/v1/challenge"))
        .and(query_param("wallet", TEST_WALLET))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "challenge_id": "ch_mock_test_001",
            "wallet_address": TEST_WALLET,
            "issued_at": "2026-10-28T00:00:00Z",
            "expires_at": "2026-10-28T00:02:00Z"
        })))
        .mount(&mock_server)
        .await;

    // 2. Mock proof submission endpoint
    Mock::given(method("POST"))
        .and(path("/api/v1/proof"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": "confirmed",
            "claim_id": "clm_mock_001",
            "tx_hash": "tx_mock_hash_12345",
            "explorer_url": "https://stellar.expert/explorer/testnet/tx/tx_mock_hash_12345",
            "amount_stroops": 10000000,
            "simulated": false
        })))
        .mount(&mock_server)
        .await;

    let config = WorkflowConfig {
        wallet: TEST_WALLET.to_string(),
        oracle_url: mock_server.uri(),
        challenge_id: None,
        simulated: false,
        fast: true,
    };

    let receipt = tokio::task::spawn_blocking(move || run_acoustic_workflow(&config))
        .await
        .unwrap()
        .expect("Workflow should succeed on mock server");
    assert_eq!(receipt.claim_id, "clm_mock_001");
    assert_eq!(receipt.tx_hash, "tx_mock_hash_12345");
    assert_eq!(receipt.amount_stroops, 10000000);
    assert!(!receipt.simulated);
}

#[tokio::test]
async fn test_offline_acoustic_workflow_pending_polling_fallback() {
    let mock_server = MockServer::start().await;

    // 1. Mock challenge endpoint
    Mock::given(method("GET"))
        .and(path("/api/v1/challenge"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "challenge_id": "ch_mock_poll_001",
            "wallet_address": TEST_WALLET,
            "issued_at": "2026-10-28T00:00:00Z",
            "expires_at": "2026-10-28T00:02:00Z"
        })))
        .mount(&mock_server)
        .await;

    // 2. Mock proof submission returning 202 Accepted (pending)
    Mock::given(method("POST"))
        .and(path("/api/v1/proof"))
        .respond_with(ResponseTemplate::new(202).set_body_json(serde_json::json!({
            "status": "pending",
            "claim_id": "clm_pending_001",
            "poll_url": "/api/v1/claims/clm_pending_001"
        })))
        .mount(&mock_server)
        .await;

    // 3. Mock claim polling endpoint
    Mock::given(method("GET"))
        .and(path("/api/v1/claims/clm_pending_001"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "claim_id": "clm_pending_001",
            "wallet_address": TEST_WALLET,
            "status": "confirmed",
            "amount_stroops": 10000000,
            "tx_hash": "tx_polled_hash_99999",
            "explorer_url": "https://stellar.expert/explorer/testnet/tx/tx_polled_hash_99999",
            "simulated": false,
            "created_at": "2026-10-28T00:00:00Z",
            "updated_at": "2026-10-28T00:00:02Z"
        })))
        .mount(&mock_server)
        .await;

    let config = WorkflowConfig {
        wallet: TEST_WALLET.to_string(),
        oracle_url: mock_server.uri(),
        challenge_id: None,
        simulated: false,
        fast: true,
    };

    let receipt = tokio::task::spawn_blocking(move || run_acoustic_workflow(&config))
        .await
        .unwrap()
        .expect("Workflow should poll and succeed with receipt");
    assert_eq!(receipt.claim_id, "clm_pending_001");
    assert_eq!(receipt.tx_hash, "tx_polled_hash_99999");
}

#[tokio::test]
async fn test_offline_acoustic_workflow_error_rejection() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/challenge"))
        .respond_with(ResponseTemplate::new(429).set_body_string("Rate limit exceeded"))
        .mount(&mock_server)
        .await;

    let config = WorkflowConfig {
        wallet: TEST_WALLET.to_string(),
        oracle_url: mock_server.uri(),
        challenge_id: None,
        simulated: false,
        fast: true,
    };

    let result = tokio::task::spawn_blocking(move || run_acoustic_workflow(&config))
        .await
        .unwrap();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("Challenge issuance rejected"));
}

#[tokio::test]
#[ignore = "Requires live testnet connection and running oracle service (set LIVE_TESTNET=1)"]
async fn test_live_testnet_e2e_scenario() {
    if std::env::var("LIVE_TESTNET").is_err() {
        return;
    }

    let oracle_url = std::env::var("ECHOGRID_ORACLE_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());
    let wallet = std::env::var("ECHOGRID_WALLET").unwrap_or_else(|_| TEST_WALLET.to_string());

    let config = WorkflowConfig {
        wallet,
        oracle_url,
        challenge_id: None,
        simulated: true,
        fast: true,
    };

    let receipt = tokio::task::spawn_blocking(move || run_acoustic_workflow(&config))
        .await
        .unwrap()
        .expect("Live testnet end-to-end acoustic claim should succeed");
    assert!(!receipt.claim_id.is_empty());
}
