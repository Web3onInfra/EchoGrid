# Phase 2: Platform Configuration & Deployment Runbook

## 1. Hosting Architecture
EchoGrid Oracle runs as a hardened single-instance service mounted with a persistent NVMe volume for SQLite storage in WAL mode.

### Platform Target: Fly.io (Primary)
- **App Configuration:** `fly.toml`
- **Volume Mount:** `echogrid_data` mounted at `/data`
- **Port:** HTTP 8080 (terminated to public HTTPS on 443 with TLS 1.3)
- **Healthcheck:** `GET /api/v1/health` polled every 30s

### Platform Target: Railway (Alternative)
- **App Configuration:** `railway.json`
- **Volume Mount:** Persistent volume attached at `/data`

---

## 2. Secrets Management & Platform Bindings
The oracle's Ed25519 signing secret (`ORACLE_SECRET_KEY`) **must never** be committed to Git, logged, or bundled in Docker images.

### Setting Secrets on Fly.io
```bash
# Set oracle signing secret key
fly secrets set ORACLE_SECRET_KEY="SXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"

# Verify secret presence (masked)
fly secrets list
```

### Setting Secrets on Railway
In Railway Project Settings -> **Variables**:
- `ORACLE_SECRET_KEY`: `S...`

---

## 3. Persistent Volume Provisioning

### Fly.io
```bash
# Provision 1 GB persistent volume in target region (e.g. iad)
fly volumes create echogrid_data --region iad --size 1
```

---

## 4. Reverse Proxy & Rate Limiting (Client IP Extraction)
- In `fly.toml`, `TRUST_PROXY="true"` is enabled.
- The Axum service reads `fly-client-ip` injected by the Fly.io edge proxy.
- Direct client spoofing of `X-Forwarded-For` or `Fly-Client-IP` is rejected when operating behind the trusted proxy layer.

---

## 5. Deployment Commands
```bash
# Deploy latest image
just deploy-fly

# Monitor deployment logs
just logs-fly

# Check service health and machine state
just status-fly
```
