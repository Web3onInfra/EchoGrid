# Phase 2: Production Container & Image Budget

## 1. Overview
The EchoGrid Oracle service is packaged as a hardened, multi-stage Linux container designed for persistent single-instance deployment (Fly.io volume mount or local Docker compose).

## 2. Image Size Budget & Composition

| Layer / Component | Base / Source | Size (Uncompressed) | Size (Compressed) | Rationale |
|:---|:---|:---|:---|:---|
| **Base OS** | `debian:bookworm-slim` | ~74 MB | ~30 MB | Hardened glibc base, matching Rust build toolchain |
| **System Utilities** | `ca-certificates`, `curl`, `libssl3` | ~16 MB | ~6 MB | TLS CA trust for Stellar RPC, healthcheck probe |
| **Oracle Binary** | Stripped release binary (`/usr/local/bin/oracle`) | ~18 MB | ~6 MB | Stripped symbols with LTO/opt-level 3 |
| **Data Directory** | `/data` | 0 MB (empty mount) | 0 MB | Mount point for SQLite database in WAL mode |
| **Total Image** | **Target Budget** | **~108 MB** | **~42 MB** | **Strictly under the 100 MB compressed budget** |

## 3. Security Hardening
- **Non-Root Execution:** Runs as unprivileged user `echogrid:echogrid` (UID 10001, GID 10001) with a disabled shell (`/usr/sbin/nologin`).
- **No Secrets in Image:** All secrets (`ORACLE_SECRET_KEY`) are injected exclusively via environment variables at container runtime.
- **Minimal Attack Surface:** Compiler tools, package managers (`apt`), and temporary build dependencies are discarded in the builder stage.
- **Built-in Healthcheck:** Periodically queries `GET /api/v1/health` with a 30s interval and 5s timeout.

## 4. Local Execution in Mock Mode
Run the oracle locally using Docker Compose:

```bash
docker compose up -d
docker compose ps
curl http://localhost:8080/api/v1/health
docker compose down
```
