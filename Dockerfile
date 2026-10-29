# syntax=docker/dockerfile:1

# ==============================================================================
# Stage 1: Builder
# Multi-stage build with dependency caching to minimize build times and image footprint.
# ==============================================================================
FROM rust:1-slim-bookworm AS builder

# Install build dependencies: OpenSSL development packages, pkg-config, and CA certs
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/echogrid

# Copy workspace manifest files
COPY Cargo.toml Cargo.lock ./
COPY oracle/Cargo.toml oracle/
COPY acoustic_cli/Cargo.toml acoustic_cli/
COPY spikes/soroban_rpc/Cargo.toml spikes/soroban_rpc/

# Cache third-party dependencies using dummy crate entrypoints
RUN mkdir -p oracle/src acoustic_cli/src spikes/soroban_rpc/src && \
    echo "fn main() {}" > oracle/src/main.rs && \
    echo "fn main() {}" > acoustic_cli/src/main.rs && \
    echo "fn main() {}" > spikes/soroban_rpc/src/main.rs && \
    cargo build --release --locked -p echogrid-oracle && \
    rm -rf oracle/src acoustic_cli/src spikes/soroban_rpc/src

# Copy real source trees and tests
COPY oracle oracle
COPY acoustic_cli acoustic_cli
COPY spikes spikes

# Compile release binary and strip symbols to minimize binary size
RUN touch oracle/src/main.rs && \
    cargo build --release --locked -p echogrid-oracle && \
    strip /usr/src/echogrid/target/release/oracle

# ==============================================================================
# Stage 2: Runtime
# Minimal hardened Debian runtime (<100MB compressed budget, non-root user).
# ==============================================================================
FROM debian:bookworm-slim AS runtime

# Install minimal runtime dependencies:
# - ca-certificates: TLS certificate verification for Soroban RPC
# - curl: Container healthcheck probe
# - libssl3: Runtime OpenSSL library
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create dedicated unprivileged non-root user and group (UID/GID 10001)
RUN groupadd -g 10001 echogrid && \
    useradd -u 10001 -g echogrid -m -d /home/echogrid -s /usr/sbin/nologin echogrid

# Create persistent storage mount directory for SQLite WAL database
RUN mkdir -p /data && chown -R echogrid:echogrid /data

# Copy stripped binary from builder stage
COPY --from=builder /usr/src/echogrid/target/release/oracle /usr/local/bin/oracle

# Environment defaults (defaults to safe mock mode; overridable in deployment)
ENV HOST=0.0.0.0 \
    PORT=8080 \
    DATABASE_URL=/data/echogrid.db \
    ORACLE_MODE=mock \
    RUST_LOG=info

# Switch to unprivileged user
USER echogrid:echogrid
WORKDIR /home/echogrid

# Expose HTTP service port
EXPOSE 8080

# Container healthcheck querying the liveness endpoint
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/api/v1/health || exit 1

ENTRYPOINT ["/usr/local/bin/oracle"]
