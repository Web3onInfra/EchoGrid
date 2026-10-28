//! # EchoGrid Oracle — Entry Point
//!
//! Boots the Axum HTTP server, configures middleware (CORS, tracing), and
//! registers all routes.

pub mod chain;
pub mod config;
pub mod error;
mod routes;
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

use crate::config::AppConfig;

/// Shared application state threaded through all Axum handlers.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            config: Arc::new(
                AppConfig::from_env_source(|_| None)
                    .expect("Default fallback configuration must always be valid"),
            ),
        }
    }
}

#[tokio::main]
async fn main() {
    // ── Initialise structured logging ─────────────────────────────────────────
    // Respects RUST_LOG env var; defaults to `info` level.
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // ── Load .env file if present (ignored in production) ─────────────────────
    let _ = dotenvy::dotenv(); // Not fatal if missing

    let config = AppConfig::from_env().expect("Failed to load and validate configuration");
    let addr = format!("{}:{}", config.host, config.port);
    let mode = config.mode;

    // ── Build the router ───────────────────────────────────────────────────────
    let app = Router::new()
        // Health probe — the first thing a judge will hit.
        .route("/api/v1/health", get(routes::health))
        // The AgentPay endpoint — receives Proofs-of-Power-Down.
        .route("/api/v1/proof", post(routes::submit_proof))
        .with_state(AppState {
            config: Arc::new(config),
        })
        // Tracing middleware — every request logged with latency.
        .layer(TraceLayer::new_for_http())
        // CORS — allow the web demo frontend to call us directly.
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    info!(mode = %mode, "EchoGrid Oracle listening on http://{addr}");
    info!("   POST /api/v1/proof   — submit Proof-of-Power-Down");
    info!("   GET  /api/v1/health  — liveness check");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("Failed to bind TCP listener to {addr}: {e}"));

    axum::serve(listener, app)
        .await
        .expect("Axum server crashed unexpectedly");
}
