//! # EchoGrid Oracle — Abuse & Concurrency Security Test Suite
//!
//! Validates resilience against malicious edge attacks, Sybil spam, race conditions,
//! replay attempts, budget exhaustion, and malformed inputs.
//!
//! All caps and safety invariants are strictly asserted:
//! 1. Same wallet hammering (cooldown & daily limit enforcement).
//! 2. Single-use challenge reuse protection.
//! 3. Cross-challenge proof replay protection.
//! 4. Parallel race-condition budget exhaustion (global daily cap never breached).
//! 5. Fuzzing and boundary parser resilience (never panics, returns 400).

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use chrono::{Duration, Utc};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tower::ServiceExt;

    use crate::build_app;
    use crate::config::AppConfig;
    use crate::proof::{derive_proof_hash_hex, ProofV1};
    use crate::state::types::Challenge;
    use crate::state::{Clock, MemoryStore, MockClock, StateStore};
    use crate::AppState;

    fn valid_test_wallet(index: u8) -> String {
        let mut raw = [0u8; 32];
        raw[0] = index;
        raw[31] = 0xAA;
        stellar_strkey::ed25519::PublicKey(raw)
            .to_string()
            .as_str()
            .to_string()
    }

    fn setup_test_app(
        now: chrono::DateTime<Utc>,
    ) -> (axum::Router, Arc<MemoryStore>, Arc<MockClock>) {
        let store = Arc::new(MemoryStore::new());
        let clock = Arc::new(MockClock::new(now));
        let mut env = HashMap::new();
        env.insert("ALLOWED_ORIGINS".to_string(), "*".to_string());
        env.insert("COOLDOWN_SECONDS".to_string(), "600".to_string());
        env.insert("WALLET_DAILY_CLAIM_LIMIT".to_string(), "2".to_string());
        env.insert(
            "ORACLE_DAILY_CAP_STROOPS".to_string(),
            "100000000".to_string(),
        ); // 100M stroops = 10 claims
        env.insert("BOUNTY_STROOPS".to_string(), "10000000".to_string()); // 10M stroops = 1 claim
        env.insert("RATE_LIMIT_PER_MIN".to_string(), "1000".to_string()); // High IP limit to isolate business logic
        let config = AppConfig::from_env_source(|k| env.get(k).cloned()).unwrap();
        let rate_limiter = Arc::new(crate::rate_limit::RateLimiter::new(clock.clone()));

        let state = AppState {
            config: Arc::new(config),
            store: store.clone(),
            clock: clock.clone(),
            queue: None,
            rate_limiter,
            readiness_report: Arc::new(tokio::sync::RwLock::new(None)),
            metrics: Arc::new(crate::metrics::Metrics::new()),
        };

        (build_app(state), store, clock)
    }

    async fn issue_test_challenge(
        store: &Arc<MemoryStore>,
        wallet: &str,
        id: &str,
        issued_at: chrono::DateTime<Utc>,
    ) {
        let ch = Challenge {
            id: id.to_string(),
            wallet_address: wallet.to_string(),
            issued_at,
            expires_at: issued_at + Duration::seconds(120),
            used: false,
        };
        store.save_challenge(ch).await.unwrap();
    }

    fn make_valid_proof(
        wallet: &str,
        ch_id: &str,
        ts: chrono::DateTime<Utc>,
        nonce: &str,
    ) -> ProofV1 {
        let ts_str = ts.to_rfc3339();
        let hash = derive_proof_hash_hex(wallet, ch_id, &ts_str, nonce);
        ProofV1 {
            wallet_address: wallet.to_string(),
            challenge_id: ch_id.to_string(),
            timestamp: ts_str,
            nonce: nonce.to_string(),
            proof_hash: hash,
            detector_version: "echonet-1.2".to_string(),
            duration_seconds: 15,
            simulated: false,
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 1. Same Wallet Hammering (Cooldown & Daily Limit)
    // ─────────────────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn test_same_wallet_hammering() {
        let now = Utc::now();
        let (app, store, clock) = setup_test_app(now);
        let wallet = valid_test_wallet(1);

        // Claim 1: OK
        issue_test_challenge(&store, &wallet, "ch_hammer_1", now).await;
        let proof1 = make_valid_proof(&wallet, "ch_hammer_1", now, "1000000000000001");
        let req1 = Request::builder()
            .method("POST")
            .uri("/api/v1/proof")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&proof1).unwrap()))
            .unwrap();
        let resp1 = app.clone().oneshot(req1).await.unwrap();
        assert_eq!(resp1.status(), StatusCode::OK);

        // Immediate Claim 2 without cooldown -> 429 Too Many Requests (WALLET_COOLDOWN_ACTIVE)
        issue_test_challenge(&store, &wallet, "ch_hammer_2", now).await;
        let proof2 = make_valid_proof(&wallet, "ch_hammer_2", now, "2000000000000002");
        let req2 = Request::builder()
            .method("POST")
            .uri("/api/v1/proof")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&proof2).unwrap()))
            .unwrap();
        let resp2 = app.clone().oneshot(req2).await.unwrap();
        assert_eq!(resp2.status(), StatusCode::TOO_MANY_REQUESTS);

        // Advance past 600s cooldown: Claim 2 -> OK
        clock.advance(Duration::seconds(601));
        let now2 = clock.now();
        issue_test_challenge(&store, &wallet, "ch_hammer_2b", now2).await;
        let proof2b = make_valid_proof(&wallet, "ch_hammer_2b", now2, "3000000000000003");
        let req2b = Request::builder()
            .method("POST")
            .uri("/api/v1/proof")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&proof2b).unwrap()))
            .unwrap();
        let resp2b = app.clone().oneshot(req2b).await.unwrap();
        assert_eq!(resp2b.status(), StatusCode::OK);

        // Advance past cooldown again: Claim 3 -> 429 Too Many Requests (WALLET_DAILY_LIMIT_EXCEEDED)
        clock.advance(Duration::seconds(601));
        let now3 = clock.now();
        issue_test_challenge(&store, &wallet, "ch_hammer_3", now3).await;
        let proof3 = make_valid_proof(&wallet, "ch_hammer_3", now3, "4000000000000004");
        let req3 = Request::builder()
            .method("POST")
            .uri("/api/v1/proof")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&proof3).unwrap()))
            .unwrap();
        let resp3 = app.clone().oneshot(req3).await.unwrap();
        assert_eq!(resp3.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 2. Challenge Reuse Protection
    // ─────────────────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn test_same_challenge_reuse_rejected() {
        let now = Utc::now();
        let (app, store, _) = setup_test_app(now);
        let wallet = valid_test_wallet(2);

        issue_test_challenge(&store, &wallet, "ch_single_use", now).await;
        let proof1 = make_valid_proof(&wallet, "ch_single_use", now, "1000000000000011");

        // First use: OK
        let req1 = Request::builder()
            .method("POST")
            .uri("/api/v1/proof")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&proof1).unwrap()))
            .unwrap();
        let resp1 = app.clone().oneshot(req1).await.unwrap();
        assert_eq!(resp1.status(), StatusCode::OK);

        // Second use with different nonce: 409 Conflict
        let proof2 = make_valid_proof(&wallet, "ch_single_use", now, "2000000000000012");
        let req2 = Request::builder()
            .method("POST")
            .uri("/api/v1/proof")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&proof2).unwrap()))
            .unwrap();
        let resp2 = app.clone().oneshot(req2).await.unwrap();
        assert_eq!(resp2.status(), StatusCode::CONFLICT);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 3. Proof Replay Protection Across Challenges
    // ─────────────────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn test_proof_replay_across_challenges_rejected() {
        let now = Utc::now();
        let (app, store, _) = setup_test_app(now);
        let wallet_a = valid_test_wallet(3);
        let wallet_b = valid_test_wallet(4);

        issue_test_challenge(&store, &wallet_a, "ch_rep_1", now).await;
        issue_test_challenge(&store, &wallet_b, "ch_rep_2", now).await;

        let proof1 = make_valid_proof(&wallet_a, "ch_rep_1", now, "1000000000000021");

        // Submit first proof
        let req1 = Request::builder()
            .method("POST")
            .uri("/api/v1/proof")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&proof1).unwrap()))
            .unwrap();
        let resp1 = app.clone().oneshot(req1).await.unwrap();
        assert_eq!(resp1.status(), StatusCode::OK);

        // Attempt replay: exact same proof hash on challenge 2
        let mut replayed_proof = make_valid_proof(&wallet_b, "ch_rep_2", now, "2000000000000022");
        replayed_proof.proof_hash = proof1.proof_hash.clone(); // replaying hash

        let req2 = Request::builder()
            .method("POST")
            .uri("/api/v1/proof")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&replayed_proof).unwrap()))
            .unwrap();
        let resp2 = app.clone().oneshot(req2).await.unwrap();
        assert!(resp2.status().is_client_error());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 4. Concurrent Budget Exhaustion Under High Load
    // ─────────────────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn test_concurrent_budget_exhaustion_cannot_breach_cap() {
        let now = Utc::now();
        let (app, store, _) = setup_test_app(now);

        // Pre-create 15 distinct wallets and valid challenges
        // Daily cap = 100M stroops, Bounty = 10M stroops => Exactly 10 claims max!
        let total_claimants = 15;
        let mut proofs = Vec::new();

        for i in 0..total_claimants {
            let wallet = valid_test_wallet(10 + i as u8);
            let ch_id = format!("ch_race_{i}");
            issue_test_challenge(&store, &wallet, &ch_id, now).await;
            let nonce = format!("{:08x}{:08x}", i + 1, i + 100);
            let proof = make_valid_proof(&wallet, &ch_id, now, &nonce);
            proofs.push(proof);
        }

        // Fire all 15 claims concurrently
        let mut handles = Vec::new();
        for proof in proofs {
            let app_clone = app.clone();
            handles.push(tokio::spawn(async move {
                let req = Request::builder()
                    .method("POST")
                    .uri("/api/v1/proof")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&proof).unwrap()))
                    .unwrap();
                let resp = app_clone.oneshot(req).await.unwrap();
                resp.status()
            }));
        }

        let mut success_count = 0;
        let mut rejected_count = 0;

        for h in handles {
            let status = h.await.unwrap();
            if status == StatusCode::OK {
                success_count += 1;
            } else if status == StatusCode::TOO_MANY_REQUESTS {
                rejected_count += 1;
            }
        }

        // Assert strictly capped: exactly 10 successes, exactly 5 rejections
        assert_eq!(
            success_count, 10,
            "Exactly 10 claims should be funded by the cap"
        );
        assert_eq!(
            rejected_count, 5,
            "Excess 5 claims must be rejected with 429"
        );

        // Verify state store total spent matches exactly 100M stroops
        let since = now - Duration::days(1);
        let total_spent = store.get_daily_spent_and_reserved(since).await.unwrap();
        assert_eq!(total_spent, 100_000_000);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 5. Malformed Proofs Fuzzing & Boundary Tests
    // ─────────────────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn test_malformed_proofs_fuzzing_resilience() {
        let now = Utc::now();
        let (app, _, _) = setup_test_app(now);

        let malformed_payloads: Vec<&[u8]> = vec![
            b"",
            b"{",
            b"[]",
            b"\"string\"",
            b"{\"wallet_address\": 12345}",
            b"{\"wallet_address\": \"\", \"challenge_id\": \"\"}",
            b"{\"wallet_address\": \"GINVALID\", \"duration_seconds\": -5}",
            b"{\"wallet_address\": \"GA4Z\", \"proof_hash\": \"short\"}",
            b"{\"wallet_address\": \"GA4Z\", \"proof_hash\": \"0xNOTHEX\"}",
            b"{\"duration_seconds\": 0}",
            b"{\"duration_seconds\": 4000}",
            b"{\"timestamp\": \"not-a-date\"}",
            b"{\"timestamp\": \"1970-01-01T00:00:00Z\"}",
            b"{\"timestamp\": \"2999-01-01T00:00:00Z\"}",
        ];

        for (idx, payload) in malformed_payloads.into_iter().enumerate() {
            let req = Request::builder()
                .method("POST")
                .uri("/api/v1/proof")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .unwrap();

            let resp = app.clone().oneshot(req).await.unwrap();
            assert!(
                resp.status().is_client_error(),
                "Payload {idx} must be rejected with 4xx, got: {}",
                resp.status()
            );
        }
    }
}
