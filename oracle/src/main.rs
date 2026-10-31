//! # EchoGrid Oracle — Entry Point
//!
//! Boots the Axum HTTP server, configures middleware (CORS, tracing, security headers), and
//! registers all routes.

pub mod chain;
pub mod challenge;
pub mod config;
pub mod error;
pub mod metrics;
pub mod payout;
pub mod proof;
pub mod rate_limit;
pub mod routes;
#[cfg(test)]
mod security_test;
pub mod state;
mod stellar;
mod types;

use std::sync::Arc;

use axum::{
    extract::{DefaultBodyLimit, Request},
    http::{header, HeaderName, HeaderValue, Method},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Router,
};
use tower_http::{
    cors::{AllowOrigin, Any, CorsLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::chain::queue::SubmissionQueue;
use crate::config::AppConfig;
use crate::state::{Clock, MemoryStore, SqliteStore, StateStore, SystemClock};

/// Shared application state threaded through all Axum handlers.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub store: Arc<dyn StateStore>,
    pub clock: Arc<dyn Clock>,
    pub queue: Option<Arc<SubmissionQueue>>,
    pub rate_limiter: Arc<rate_limit::RateLimiter>,
    pub readiness_report:
        Arc<tokio::sync::RwLock<Option<crate::chain::self_check::SelfCheckReport>>>,
    pub metrics: Arc<metrics::Metrics>,
}

impl Default for AppState {
    fn default() -> Self {
        let clock = Arc::new(SystemClock);
        Self {
            config: Arc::new(
                AppConfig::from_env_source(|_| None)
                    .expect("Default fallback configuration must always be valid"),
            ),
            store: Arc::new(MemoryStore::new()),
            clock: clock.clone(),
            queue: None,
            rate_limiter: Arc::new(rate_limit::RateLimiter::new(clock)),
            readiness_report: Arc::new(tokio::sync::RwLock::new(None)),
            metrics: Arc::new(metrics::Metrics::new()),
        }
    }
}

/// Builds the CORS middleware layer enforcing the configured origin allowlist.
pub fn build_cors_layer(allowed_origins: &[String]) -> CorsLayer {
    let mut layer = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::ACCEPT,
            HeaderName::from_static("x-request-id"),
        ]);

    if allowed_origins.iter().any(|o| o == "*") {
        layer = layer.allow_origin(Any);
    } else {
        let origins: Vec<HeaderValue> = allowed_origins
            .iter()
            .filter_map(|s| s.parse().ok())
            .collect();
        layer = layer.allow_origin(AllowOrigin::list(origins));
    }

    layer
}

/// Middleware injecting and propagating request IDs with structured tracing instrumentation.
pub async fn request_id_and_tracing_middleware(mut req: Request, next: Next) -> Response {
    let req_id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            let mut bytes = [0u8; 8];
            getrandom::getrandom(&mut bytes).expect("OS entropy failure");
            format!("req_{}", hex::encode(bytes))
        });

    let header_val = header::HeaderValue::from_str(&req_id)
        .unwrap_or_else(|_| header::HeaderValue::from_static("req_unknown"));

    req.headers_mut()
        .insert(HeaderName::from_static("x-request-id"), header_val.clone());

    let start = std::time::Instant::now();
    let method = req.method().clone();
    let uri = req.uri().path().to_string();

    let span = tracing::info_span!(
        "http_request",
        request_id = %req_id,
        method = %method,
        uri = %uri,
    );

    use tracing::Instrument;
    let mut resp = next.run(req).instrument(span).await;
    let latency_ms = start.elapsed().as_millis() as u64;
    let status = resp.status().as_u16();

    tracing::info!(
        request_id = %req_id,
        method = %method,
        path = %uri,
        status = status,
        latency_ms = latency_ms,
        "access_log"
    );

    resp.headers_mut()
        .insert(HeaderName::from_static("x-request-id"), header_val);
    resp
}

/// Internal metrics endpoint restricted to loopback callers.
pub async fn internal_metrics(
    axum::extract::State(state): axum::extract::State<AppState>,
    req: Request,
) -> Result<axum::Json<metrics::MetricsSnapshot>, crate::error::ApiError> {
    let peer_addr = req
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|ci| ci.0);
    let client_ip =
        crate::rate_limit::extract_client_ip(req.headers(), peer_addr, state.config.trust_proxy);

    if !client_ip.is_loopback() {
        return Err(crate::error::ApiError::unauthorized(
            "METRICS_FORBIDDEN",
            "Metrics endpoint is restricted to internal loopback callers",
        ));
    }

    Ok(axum::Json(state.metrics.snapshot()))
}

/// Middleware enforcing per-IP rate limits across all incoming requests.
pub async fn ip_rate_limit_middleware(
    axum::extract::State(state): axum::extract::State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, crate::error::ApiError> {
    let peer_addr = req
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|ci| ci.0);
    let client_ip =
        crate::rate_limit::extract_client_ip(req.headers(), peer_addr, state.config.trust_proxy);
    state
        .rate_limiter
        .check_ip(client_ip, state.config.rate_limit_per_min)?;
    Ok(next.run(req).await)
}

/// Middleware injecting standard HTTP security headers on all responses.
pub async fn security_headers_middleware(req: Request, next: Next) -> Response {
    let mut resp = next.run(req).await;
    let headers = resp.headers_mut();
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        HeaderName::from_static("x-xss-protection"),
        HeaderValue::from_static("1; mode=block"),
    );
    headers.insert(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );
    headers.insert(
        HeaderName::from_static("content-security-policy"),
        HeaderValue::from_static("default-src 'none'"),
    );
    resp
}

/// Builds the fully configured Axum application with all routes and hardening layers.
pub fn build_app(state: AppState) -> Router {
    let timeout_duration = std::time::Duration::from_secs(state.config.request_timeout_seconds);
    let cors_layer = build_cors_layer(&state.config.allowed_origins);

    Router::new()
        // Operational & health probes
        .route("/api/v1/health", get(routes::health))
        .route("/api/v1/ready", get(routes::ready))
        .route("/api/v1/version", get(routes::version))
        .route("/api/v1/config", get(routes::public_config))
        .route("/api/v1/claims/:id", get(routes::get_claim))
        // Challenge issuance
        .route("/api/v1/challenge", get(challenge::get_challenge))
        // The AgentPay endpoint
        .route("/api/v1/proof", post(routes::submit_proof))
        // Private internal telemetry endpoint
        .route("/internal/metrics", get(internal_metrics))
        .layer(middleware::from_fn(request_id_and_tracing_middleware))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            ip_rate_limit_middleware,
        ))
        .layer(middleware::from_fn(security_headers_middleware))
        .layer(DefaultBodyLimit::max(64 * 1024))
        .layer(TimeoutLayer::new(timeout_duration))
        .layer(TraceLayer::new_for_http())
        .layer(cors_layer)
        .with_state(state)
}

#[tokio::main]
async fn main() {
    // ── Initialise structured logging ─────────────────────────────────────────
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // ── Load .env file if present (ignored in production) ─────────────────────
    let _ = dotenvy::dotenv();

    let config = AppConfig::from_env().expect("Failed to load and validate configuration");
    let addr = format!("{}:{}", config.host, config.port);
    let mode = config.mode;

    // ── Initialize persistent store ───────────────────────────────────────────
    let store: Arc<dyn StateStore> = match SqliteStore::open(&config.database_url) {
        Ok(s) => Arc::new(s),
        Err(e) => {
            tracing::warn!(err = %e, "Falling back to in-memory store");
            Arc::new(MemoryStore::new())
        }
    };
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);

    // ── Initialize submission queue if configured ──────────────────────────────
    let queue: Option<Arc<SubmissionQueue>> = if let (Some(secret), Some(contract_id)) =
        (&config.oracle_secret_key, &config.contract_id)
    {
        use secrecy::ExposeSecret;
        if let Ok(pk) = stellar_strkey::ed25519::PrivateKey::from_string(secret.expose_secret()) {
            let signing_key = ed25519_dalek::SigningKey::from_bytes(&pk.0);
            let public_strkey =
                stellar_strkey::ed25519::PublicKey(signing_key.verifying_key().to_bytes());
            let oracle_account = public_strkey.to_string().as_str().to_string();
            let rpc = crate::chain::rpc::SorobanRpcClient::new(
                &config.rpc_url,
                std::time::Duration::from_secs(config.sync_wait_timeout_seconds),
            );
            let pipeline = Arc::new(crate::chain::pipeline::SubmitPipeline::new(
                rpc.clone(),
                signing_key,
                contract_id.clone(),
                config.network_passphrase.clone(),
                std::time::Duration::from_secs(config.sync_wait_timeout_seconds),
            ));
            Some(Arc::new(SubmissionQueue::new(
                pipeline,
                rpc,
                oracle_account,
                None,
            )))
        } else {
            None
        }
    } else {
        None
    };

    let rate_limiter = Arc::new(rate_limit::RateLimiter::new(clock.clone()));
    let readiness_report = Arc::new(tokio::sync::RwLock::new(None));
    let metrics = Arc::new(metrics::Metrics::new());
    let state = AppState {
        config: Arc::new(config),
        store,
        clock,
        queue,
        rate_limiter,
        readiness_report,
        metrics,
    };

    let app = build_app(state);

    info!(mode = %mode, "EchoGrid Oracle listening on http://{addr}");
    info!("   GET  /api/v1/challenge — issue challenge nonce");
    info!("   POST /api/v1/proof     — submit Proof-of-Power-Down");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("Failed to bind to {addr}: {e}"));
    axum::serve(listener, app).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use std::collections::HashMap;
    use tower::ServiceExt;

    fn test_app_state(origins: Vec<String>) -> AppState {
        let mut env = HashMap::new();
        env.insert("ALLOWED_ORIGINS".to_string(), origins.join(","));
        let config = AppConfig::from_env_source(|k| env.get(k).cloned()).unwrap();
        let clock = Arc::new(SystemClock);
        let rate_limiter = Arc::new(rate_limit::RateLimiter::new(clock.clone()));
        AppState {
            config: Arc::new(config),
            store: Arc::new(MemoryStore::new()),
            clock,
            queue: None,
            rate_limiter,
            readiness_report: Arc::new(tokio::sync::RwLock::new(None)),
            metrics: Arc::new(metrics::Metrics::new()),
        }
    }

    #[tokio::test]
    async fn test_ip_rate_limiting_middleware() {
        let mut env = HashMap::new();
        env.insert("ALLOWED_ORIGINS".to_string(), "*".to_string());
        env.insert("RATE_LIMIT_PER_MIN".to_string(), "2".to_string());
        let config = AppConfig::from_env_source(|k| env.get(k).cloned()).unwrap();
        let clock = Arc::new(SystemClock);
        let rate_limiter = Arc::new(rate_limit::RateLimiter::new(clock.clone()));
        let state = AppState {
            config: Arc::new(config),
            store: Arc::new(MemoryStore::new()),
            clock,
            queue: None,
            rate_limiter,
            readiness_report: Arc::new(tokio::sync::RwLock::new(None)),
            metrics: Arc::new(metrics::Metrics::new()),
        };
        let app = build_app(state);

        // Request 1: OK
        let req1 = Request::builder()
            .uri("/api/v1/health")
            .body(Body::empty())
            .unwrap();
        let resp1 = app.clone().oneshot(req1).await.unwrap();
        assert_eq!(resp1.status(), StatusCode::OK);

        // Request 2: OK
        let req2 = Request::builder()
            .uri("/api/v1/health")
            .body(Body::empty())
            .unwrap();
        let resp2 = app.clone().oneshot(req2).await.unwrap();
        assert_eq!(resp2.status(), StatusCode::OK);

        // Request 3: 429 Too Many Requests
        let req3 = Request::builder()
            .uri("/api/v1/health")
            .body(Body::empty())
            .unwrap();
        let resp3 = app.clone().oneshot(req3).await.unwrap();
        assert_eq!(resp3.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn test_wallet_challenge_rate_limiting_endpoint() {
        let mut env = HashMap::new();
        env.insert("ALLOWED_ORIGINS".to_string(), "*".to_string());
        env.insert("CHALLENGE_RATE_PER_MIN".to_string(), "2".to_string());
        let config = AppConfig::from_env_source(|k| env.get(k).cloned()).unwrap();
        let clock = Arc::new(SystemClock);
        let rate_limiter = Arc::new(rate_limit::RateLimiter::new(clock.clone()));
        let state = AppState {
            config: Arc::new(config),
            store: Arc::new(MemoryStore::new()),
            clock,
            queue: None,
            rate_limiter,
            readiness_report: Arc::new(tokio::sync::RwLock::new(None)),
            metrics: Arc::new(metrics::Metrics::new()),
        };
        let app = build_app(state);

        let wallet = "GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT";

        // Challenge 1: OK
        let req1 = Request::builder()
            .uri(format!("/api/v1/challenge?wallet={wallet}"))
            .body(Body::empty())
            .unwrap();
        let resp1 = app.clone().oneshot(req1).await.unwrap();
        assert_eq!(resp1.status(), StatusCode::OK);

        // Challenge 2: OK
        let req2 = Request::builder()
            .uri(format!("/api/v1/challenge?wallet={wallet}"))
            .body(Body::empty())
            .unwrap();
        let resp2 = app.clone().oneshot(req2).await.unwrap();
        assert_eq!(resp2.status(), StatusCode::OK);

        // Challenge 3: 429 Too Many Requests
        let req3 = Request::builder()
            .uri(format!("/api/v1/challenge?wallet={wallet}"))
            .body(Body::empty())
            .unwrap();
        let resp3 = app.clone().oneshot(req3).await.unwrap();
        assert_eq!(resp3.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn test_cors_preflight_allowed_origin() {
        let state = test_app_state(vec!["http://localhost:3000".to_string()]);
        let app = build_app(state);

        let req = Request::builder()
            .method("OPTIONS")
            .uri("/api/v1/proof")
            .header("Origin", "http://localhost:3000")
            .header("Access-Control-Request-Method", "POST")
            .header("Access-Control-Request-Headers", "content-type")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers()
                .get("access-control-allow-origin")
                .and_then(|v| v.to_str().ok()),
            Some("http://localhost:3000")
        );
    }

    #[tokio::test]
    async fn test_cors_preflight_disallowed_origin() {
        let state = test_app_state(vec!["http://localhost:3000".to_string()]);
        let app = build_app(state);

        let req = Request::builder()
            .method("OPTIONS")
            .uri("/api/v1/proof")
            .header("Origin", "https://untrusted-site.com")
            .header("Access-Control-Request-Method", "POST")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_ne!(
            resp.headers()
                .get("access-control-allow-origin")
                .and_then(|v| v.to_str().ok()),
            Some("https://untrusted-site.com")
        );
    }

    #[tokio::test]
    async fn test_security_headers_present() {
        let state = test_app_state(vec!["*".to_string()]);
        let app = build_app(state);

        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/health")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let headers = resp.headers();
        assert_eq!(
            headers
                .get("x-content-type-options")
                .and_then(|v| v.to_str().ok()),
            Some("nosniff")
        );
        assert_eq!(
            headers.get("x-frame-options").and_then(|v| v.to_str().ok()),
            Some("DENY")
        );
        assert_eq!(
            headers
                .get("x-xss-protection")
                .and_then(|v| v.to_str().ok()),
            Some("1; mode=block")
        );
        assert_eq!(
            headers
                .get("strict-transport-security")
                .and_then(|v| v.to_str().ok()),
            Some("max-age=31536000; includeSubDomains")
        );
        assert_eq!(
            headers
                .get("content-security-policy")
                .and_then(|v| v.to_str().ok()),
            Some("default-src 'none'")
        );
    }

    #[tokio::test]
    async fn test_body_size_limit_exceeded() {
        let state = test_app_state(vec!["*".to_string()]);
        let app = build_app(state);

        // 70 KB payload exceeds 64 KB limit
        let large_payload = vec![b'a'; 70 * 1024];

        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/proof")
            .header("content-type", "application/json")
            .body(Body::from(large_payload))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn test_request_id_generation_and_propagation() {
        let state = test_app_state(vec!["*".to_string()]);
        let app = build_app(state);

        // Auto-generated request id
        let req1 = Request::builder()
            .uri("/api/v1/health")
            .body(Body::empty())
            .unwrap();
        let resp1 = app.clone().oneshot(req1).await.unwrap();
        let req_id1 = resp1
            .headers()
            .get("x-request-id")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(req_id1.starts_with("req_"));

        // Propagated custom request id
        let req2 = Request::builder()
            .uri("/api/v1/health")
            .header("x-request-id", "custom-client-id-999")
            .body(Body::empty())
            .unwrap();
        let resp2 = app.clone().oneshot(req2).await.unwrap();
        let req_id2 = resp2
            .headers()
            .get("x-request-id")
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(req_id2, "custom-client-id-999");
    }

    #[tokio::test]
    async fn test_internal_metrics_access_control() {
        let state = test_app_state(vec!["*".to_string()]);
        let app = build_app(state);

        // Internal loopback request -> 200 OK
        let req_loopback = Request::builder()
            .uri("/internal/metrics")
            .body(Body::empty())
            .unwrap();
        let resp_ok = app.clone().oneshot(req_loopback).await.unwrap();
        assert_eq!(resp_ok.status(), StatusCode::OK);

        // External non-loopback request behind trusted proxy -> 401 METRICS_FORBIDDEN
        let mut env = HashMap::new();
        env.insert("ALLOWED_ORIGINS".to_string(), "*".to_string());
        env.insert("TRUST_PROXY".to_string(), "true".to_string());
        let config = AppConfig::from_env_source(|k| env.get(k).cloned()).unwrap();
        let mut state_proxy = test_app_state(vec!["*".to_string()]);
        state_proxy.config = Arc::new(config);
        let app_proxy = build_app(state_proxy);

        let req_external = Request::builder()
            .uri("/internal/metrics")
            .header("Fly-Client-IP", "198.51.100.25")
            .body(Body::empty())
            .unwrap();
        let resp_forbidden = app_proxy.oneshot(req_external).await.unwrap();
        assert_eq!(resp_forbidden.status(), StatusCode::FORBIDDEN);
    }

    static TEST_LOG_BUFFER: std::sync::Mutex<Vec<u8>> = std::sync::Mutex::new(Vec::new());
    static INIT_TEST_TRACING: std::sync::Once = std::sync::Once::new();

    #[tokio::test]
    async fn test_access_log_emission_and_secret_redaction() {
        use std::io::Write;

        #[derive(Clone)]
        struct StaticBufferWriter;
        impl Write for StaticBufferWriter {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                if let Ok(mut lock) = TEST_LOG_BUFFER.lock() {
                    lock.extend_from_slice(buf);
                }
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for StaticBufferWriter {
            type Writer = StaticBufferWriter;
            fn make_writer(&'a self) -> Self::Writer {
                StaticBufferWriter
            }
        }

        INIT_TEST_TRACING.call_once(|| {
            let subscriber = tracing_subscriber::fmt()
                .with_writer(StaticBufferWriter)
                .with_ansi(false)
                .with_max_level(tracing::Level::INFO)
                .finish();
            let _ = tracing::subscriber::set_global_default(subscriber);
        });

        TEST_LOG_BUFFER.lock().unwrap().clear();

        let state = test_app_state(vec!["*".to_string()]);
        let app = build_app(state);

        let sensitive_token = "s3cr3t_auth_bearer_token_xyz999";
        let sensitive_payload = "confidential_proof_raw_data_abc";

        let req = Request::builder()
            .uri("/api/v1/health")
            .header("authorization", format!("Bearer {}", sensitive_token))
            .header("x-secret-key", sensitive_token)
            .body(Body::from(sensitive_payload))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let logs = String::from_utf8(TEST_LOG_BUFFER.lock().unwrap().clone()).unwrap();
        assert!(logs.contains("access_log"), "Logs must contain access_log event: got '{}'", logs);
        assert!(logs.contains("method=GET") || logs.contains("method=\"GET\""), "Logs must contain method");
        assert!(logs.contains("path=/api/v1/health") || logs.contains("path=\"/api/v1/health\""), "Logs must contain path");
        assert!(logs.contains("status=200"), "Logs must contain HTTP status 200");
        assert!(logs.contains("request_id="), "Logs must contain request_id");
        assert!(logs.contains("latency_ms="), "Logs must contain latency_ms");

        // Verify secrets and request bodies are NEVER logged
        assert!(!logs.contains(sensitive_token), "Sensitive auth tokens must NOT appear in access logs");
        assert!(!logs.contains(sensitive_payload), "Payload bodies must NOT appear in access logs");
    }
}