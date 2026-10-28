# Phase 2: Continuous Deployment Pipeline

## 1. Overview
The EchoGrid Oracle deployment pipeline (`.github/workflows/deploy.yml`) automates building, verifying, and deploying the oracle service to Fly.io.

## 2. Trigger Strategy
- **Automated Deploy on Push:** Every commit merged into `main` affecting `oracle/**`, manifests, or Docker configuration triggers the CD workflow.
- **Manual Dispatch (`workflow_dispatch`):** Operators can manually trigger deployments from GitHub Actions UI targeting `production` or `staging`.
- **Concurrency Control:** Serialized deployments per branch (`cancel-in-progress: false`) ensure no race conditions on machines or SQLite storage.

## 3. Pipeline Stages
1. **Pre-deployment Verification:**
   - Strict `cargo fmt --check`
   - Zero-warning `cargo clippy --workspace --all-targets -- -D warnings`
   - Complete unit and integration test suite (`cargo test --workspace`)
2. **Fly.io Setup & Deployment:**
   - Installs `flyctl` via `superfly/flyctl-actions/setup-flyctl`.
   - Executes remote builder deployment (`flyctl deploy --remote-only --config fly.toml`).
   - Graceful fallback: If `FLY_API_TOKEN` is not yet configured, logs a diagnostic notice and exits cleanly without failing CI.

## 4. Repository Secrets Required
- `FLY_API_TOKEN`: Fly.io deploy token generated via `fly tokens create deploy -a echogrid-oracle`.
