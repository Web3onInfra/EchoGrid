#!/usr/bin/env bash
# EchoGrid: Idempotent Testnet Deploy Script
# Usage:
#   bash scripts/deploy_testnet.sh [--dry-run]
set -euo pipefail

if [ -d "/mnt/c/Users/isaac/.cargo/bin" ]; then
    export PATH="/mnt/c/Users/isaac/.cargo/bin:$PATH"
fi
export PATH="$HOME/.cargo/bin:$PATH"

if ! command -v cargo &> /dev/null && command -v cargo.exe &> /dev/null; then
    cargo() {
        cargo.exe "$@"
    }
fi

if ! command -v stellar &> /dev/null && command -v stellar.exe &> /dev/null; then
    stellar() {
        stellar.exe "$@"
    }
fi

DRY_RUN=false
for arg in "$@"; do
    case "$arg" in
        --dry-run)
            DRY_RUN=true
            ;;
        *)
            ;;
    esac
done

NETWORK="testnet"
RPC_URL="https://soroban-testnet.stellar.org"
NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
MUNI_IDENTITY="${MUNICIPALITY_IDENTITY:-echo_municipality}"
ORACLE_IDENTITY="${ORACLE_IDENTITY:-echo_oracle}"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "============================================================"
echo "EchoGrid: Idempotent Testnet Deployment"
echo "Network:             ${NETWORK}"
echo "Dry Run:             ${DRY_RUN}"
echo "Municipality ID:     ${MUNI_IDENTITY}"
echo "Oracle ID:           ${ORACLE_IDENTITY}"
echo "============================================================"

ensure_identity() {
    local id="$1"
    if stellar keys address "$id" &> /dev/null; then
        echo "Identity '${id}' exists."
    else
        echo "Identity '${id}' not found. Generating and funding via friendbot..."
        stellar keys generate --network "$NETWORK" --fund "$id"
    fi
}

echo "==> Verifying keystore identities..."
ensure_identity "$MUNI_IDENTITY"
ensure_identity "$ORACLE_IDENTITY"

MUNI_PUBKEY="$(stellar keys address "$MUNI_IDENTITY")"
ORACLE_PUBKEY="$(stellar keys address "$ORACLE_IDENTITY")"
echo "Municipality Address: ${MUNI_PUBKEY}"
echo "Oracle Address:       ${ORACLE_PUBKEY}"

echo "==> Resolving Native XLM SAC ID..."
TOKEN_ID="$(stellar contract id asset --asset native --network "$NETWORK")"
echo "Native XLM Token ID:  ${TOKEN_ID}"

echo "==> Building and optimizing contract..."
bash scripts/build_contract.sh

WASM_OPT="contracts/negawatt_escrow/target/wasm32-unknown-unknown/release/negawatt_escrow.optimized.wasm"
WASM_HASH="$(awk '{print $1}' deployments/wasm-hash.txt)"
WASM_SIZE="$(awk '{print $3}' deployments/wasm-hash.txt)"

BOUNTY_STROOPS="10000000"
CAP_STROOPS="20000000"
WINDOW_BUDGET_STROOPS="200000000"
WINDOW_LEN="86400"
MAX_CLAIMS="3"

if [ "$DRY_RUN" = true ]; then
    echo "============================================================"
    echo "DRY RUN COMPLETE — No transactions submitted"
    echo "Planned Contract Deploy:"
    echo "  WASM File:        ${WASM_OPT}"
    echo "  WASM Hash:        ${WASM_HASH}"
    echo "  WASM Size:        ${WASM_SIZE} bytes"
    echo "  Deployer:         ${MUNI_IDENTITY} (${MUNI_PUBKEY})"
    echo "Planned Initialization Parameters:"
    echo "  Municipality:     ${MUNI_PUBKEY}"
    echo "  Oracle:           ${ORACLE_PUBKEY}"
    echo "  Token ID:         ${TOKEN_ID}"
    echo "  Bounty:           ${BOUNTY_STROOPS} stroops (1 XLM)"
    echo "  Payout Cap:       ${CAP_STROOPS} stroops (2 XLM)"
    echo "  Window Budget:    ${WINDOW_BUDGET_STROOPS} stroops (20 XLM)"
    echo "  Window Length:    ${WINDOW_LEN} seconds (24h)"
    echo "  Max Claims/Win:   ${MAX_CLAIMS}"
    echo "============================================================"
    exit 0
fi

echo "==> Deploying contract to ${NETWORK}..."
CONTRACT_ID="$(stellar contract deploy \
    --wasm "$WASM_OPT" \
    --source "$MUNI_IDENTITY" \
    --network "$NETWORK")"
echo "Contract deployed successfully. ID: ${CONTRACT_ID}"

echo "==> Initializing contract with confirmed defaults..."
stellar contract invoke \
    --id "$CONTRACT_ID" \
    --source "$MUNI_IDENTITY" \
    --network "$NETWORK" \
    -- initialize \
    --municipality "$MUNI_PUBKEY" \
    --oracle "$ORACLE_PUBKEY" \
    --token "$TOKEN_ID" \
    --bounty "$BOUNTY_STROOPS"

echo "==> Verifying initialized configuration on-chain..."
stellar contract invoke \
    --id "$CONTRACT_ID" \
    --source "$MUNI_IDENTITY" \
    --network "$NETWORK" \
    -- get_config

TIMESTAMP="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
mkdir -p deployments
cat > deployments/testnet.json <<EOF
{
  "network": "${NETWORK}",
  "rpc_url": "${RPC_URL}",
  "network_passphrase": "${NETWORK_PASSPHRASE}",
  "contract_id": "${CONTRACT_ID}",
  "token_id": "${TOKEN_ID}",
  "wasm_hash": "${WASM_HASH}",
  "wasm_size_bytes": ${WASM_SIZE},
  "public_keys": {
    "municipality": "${MUNI_PUBKEY}",
    "oracle": "${ORACLE_PUBKEY}"
  },
  "config": {
    "bounty_stroops": ${BOUNTY_STROOPS},
    "per_payout_cap_stroops": ${CAP_STROOPS},
    "window_budget_stroops": ${WINDOW_BUDGET_STROOPS},
    "window_len_seconds": ${WINDOW_LEN},
    "max_recipient_window_claims": ${MAX_CLAIMS}
  },
  "deployed_at": "${TIMESTAMP}",
  "explorer_url": "https://stellar.expert/explorer/testnet/contract/${CONTRACT_ID}"
}
EOF

echo "============================================================"
echo "Deployment SUCCESSFUL!"
echo "Contract ID:  ${CONTRACT_ID}"
echo "Config saved: deployments/testnet.json"
echo "Explorer:     https://stellar.expert/explorer/testnet/contract/${CONTRACT_ID}"
echo "============================================================"
