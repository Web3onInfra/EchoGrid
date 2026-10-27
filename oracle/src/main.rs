//! # EchoGrid Oracle — Entry Point
//!
//! Boots the Axum HTTP server, configures middleware (CORS, tracing), and
//! registers all routes.
//!
//! ## Environment variables
//! | Variable              | Required (live-network) | Description                        |
//! |-----------------------|-------------------------|------------------------------------|
//! | `ORACLE_SECRET_KEY`   | ✅                       | Ed25519 secret key (S...)          |
//! | `CONTRACT_ID`         | ✅                       | Deployed NegaWattEscrow address    |
//! | `HORIZON_URL`         | No (defaults testnet)   | Horizon REST API base URL          |
//! | `TOKEN_CONTRACT_ID`   | ✅                       | USDC-T SAC contract ID             |
//! | `PORT`                | No (defaults 8080)      | HTTP listen port                   |

mod routes;
mod stellar;
mod types;

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

/// Shared application state threaded through all Axum handlers.
/// Currently empty for the MVP, but ready to hold a DB pool, config, etc.
#[derive(Clone)]
pub struct AppState;

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

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let addr = format!("0.0.0.0:{port}");

    // ── Build the router ───────────────────────────────────────────────────────
    let app = Router::new()
        // Health probe — the first thing a judge will hit.
        .route("/api/v1/health", get(routes::health))
        // The AgentPay endpoint — receives Proofs-of-Power-Down.
        .route("/api/v1/proof", post(routes::submit_proof))
        .with_state(AppState)
        // Tracing middleware — every request logged with latency.
        .layer(TraceLayer::new_for_http())
        // CORS — allow the web demo frontend to call us directly.
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    info!("🚀 EchoGrid Oracle listening on http://{addr}");
    info!("   POST /api/v1/proof   — submit Proof-of-Power-Down");
    info!("   GET  /api/v1/health  — liveness check");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind TCP listener — is port {port} already in use?");

    axum::serve(listener, app)
        .await
        .expect("Axum server crashed unexpectedly");
}
