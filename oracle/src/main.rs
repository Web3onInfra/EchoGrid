//! # EchoGrid Oracle — Entry Point
//!
//! Boots the Axum HTTP server, configures middleware (CORS, tracing), and
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
    routing::{get, post},
    Router,
};
use tower_http::{
    cors::{Any, CorsLayer},
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

    // ── Build the router ───────────────────────────────────────────────────────
    let app = Router::new()
        // Health probe
        .route("/api/v1/health", get(routes::health))
        // Challenge issuance
        .route("/api/v1/challenge", get(challenge::get_challenge))
        // The AgentPay endpoint
        .route("/api/v1/proof", post(routes::submit_proof))
        .with_state(AppState {
            config: Arc::new(config),
            store,
            clock,
            queue,
        })
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    info!(mode = %mode, "EchoGrid Oracle listening on http://{addr}");
    info!("   GET  /api/v1/challenge — issue challenge nonce");
    info!("   POST /api/v1/proof     — submit Proof-of-Power-Down");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("Failed to bind to {addr}: {e}"));
    axum::serve(listener, app).await.unwrap();
}
