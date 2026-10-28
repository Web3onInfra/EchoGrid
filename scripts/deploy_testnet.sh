#!/usr/bin/env bash
# Deploy EchoGrid NegaWatt Escrow contract to Stellar Testnet
set -euo pipefail

SOURCE_IDENTITY="${1:-echogrid_deployer}"
NETWORK="testnet"
RPC_URL="https://soroban-testnet.stellar.org"
NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
CONTRACT_DIR="${ROOT_DIR}/contracts/negawatt_escrow"
DEPLOYMENTS_DIR="${ROOT_DIR}/deployments"

echo "=== EchoGrid Testnet Deployment ==="
echo "Identity: ${SOURCE_IDENTITY}"
echo "Network: ${NETWORK}"

# 1. Resolve Deployer Public Key
DEPLOYER_PUBKEY="$(stellar keys address "${SOURCE_IDENTITY}")"
echo "Deployer Public Key: ${DEPLOYER_PUBKEY}"

# 2. Build and optimize contract WASM
echo "Building contract WASM..."
cd "${CONTRACT_DIR}"
cargo build --target wasm32-unknown-unknown --release

WASM_FILE="${CONTRACT_DIR}/target/wasm32-unknown-unknown/release/negawatt_escrow.wasm"
OPTIMIZED_WASM="${CONTRACT_DIR}/target/wasm32-unknown-unknown/release/negawatt_escrow.optimized.wasm"

echo "Optimizing WASM..."
stellar contract optimize --wasm "${WASM_FILE}" --wasm-out "${OPTIMIZED_WASM}"

# 3. Retrieve Native XLM SAC Token ID
echo "Resolving Native XLM Token ID..."
TOKEN_ID="$(stellar contract id asset --asset native --network "${NETWORK}")"
echo "Native Token ID: ${TOKEN_ID}"

# 4. Deploy contract to Testnet
echo "Deploying contract to Stellar Testnet..."
CONTRACT_ID="$(stellar contract deploy \
  --wasm "${OPTIMIZED_WASM}" \
  --source "${SOURCE_IDENTITY}" \
  --network "${NETWORK}")"
echo "Contract Deployed: ${CONTRACT_ID}"

# 5. Initialize contract (1 XLM bounty per claim)
BOUNTY_STROOPS="10000000"
echo "Initializing contract..."
stellar contract invoke \
  --id "${CONTRACT_ID}" \
  --source "${SOURCE_IDENTITY}" \
  --network "${NETWORK}" \
  -- initialize \
  --municipality "${DEPLOYER_PUBKEY}" \
  --oracle "${DEPLOYER_PUBKEY}" \
  --token "${TOKEN_ID}" \
  --bounty "${BOUNTY_STROOPS}"

# 6. Fund escrow (Deposit 5 XLM)
DEPOSIT_STROOPS="50000000"
echo "Depositing ${DEPOSIT_STROOPS} stroops into escrow..."
stellar contract invoke \
  --id "${CONTRACT_ID}" \
  --source "${SOURCE_IDENTITY}" \
  --network "${NETWORK}" \
  -- deposit \
  --amount "${DEPOSIT_STROOPS}"

# 7. Write deployments/testnet.json
mkdir -p "${DEPLOYMENTS_DIR}"
cat > "${DEPLOYMENTS_DIR}/testnet.json" <<EOF
{
  "network": "${NETWORK}",
  "rpc_url": "${RPC_URL}",
  "network_passphrase": "${NETWORK_PASSPHRASE}",
  "contract_id": "${CONTRACT_ID}",
  "token_id": "${TOKEN_ID}",
  "oracle_public_key": "${DEPLOYER_PUBKEY}",
  "municipality_public_key": "${DEPLOYER_PUBKEY}",
  "bounty_stroops": ${BOUNTY_STROOPS},
  "initial_deposit_stroops": ${DEPOSIT_STROOPS},
  "explorer_url": "https://stellar.expert/explorer/testnet/contract/${CONTRACT_ID}"
}
EOF

echo "=== Deployment Complete ==="
echo "Saved to: ${DEPLOYMENTS_DIR}/testnet.json"
