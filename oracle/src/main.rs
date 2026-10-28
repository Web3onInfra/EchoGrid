//! # EchoGrid Oracle — Entry Point
//!
//! Boots the Axum HTTP server, configures middleware (CORS, tracing, security headers), and
//! registers all routes.

pub mod chain;
pub mod challenge;
pub mod config;
pub mod error;
pub mod payout;
pub mod proof;
pub mod routes;
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
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            config: Arc::new(
                AppConfig::from_env_source(|_| None)
                    .expect("Default fallback configuration must always be valid"),
            ),
            store: Arc::new(MemoryStore::new()),
            clock: Arc::new(SystemClock),
            queue: None,
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
        // Health probe
        .route("/api/v1/health", get(routes::health))
        // Challenge issuance
        .route("/api/v1/challenge", get(challenge::get_challenge))
        // The AgentPay endpoint
        .route("/api/v1/proof", post(routes::submit_proof))
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

    let state = AppState {
        config: Arc::new(config),
        store,
        clock,
        queue,
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
        AppState {
            config: Arc::new(config),
            store: Arc::new(MemoryStore::new()),
            clock: Arc::new(SystemClock),
            queue: None,
        }
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
}
