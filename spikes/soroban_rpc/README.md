# Soroban RPC Spike

Research spike evaluating Stellar Testnet Soroban JSON-RPC connectivity directly from Rust.

## Stellar Testnet Parameters
- **Public RPC Endpoint**: `https://soroban-testnet.stellar.org`
- **Network Passphrase**: `Test SDF Network ; September 2015`
- **Horizon Fallback URL**: `https://horizon-testnet.stellar.org`

## Environment Variables
Create an `.env` file (or pass as flags) to customize endpoints:
```bash
SOROBAN_RPC_URL="https://soroban-testnet.stellar.org"
STELLAR_NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
```

## Running Health Check
Query live Soroban RPC health and the latest finalized ledger sequence:
```bash
cargo run -p soroban-rpc-spike -- health
```

Expected output:
```text
Connecting to Soroban RPC: https://soroban-testnet.stellar.org
Network Passphrase: Test SDF Network ; September 2015
[OK] Health status: healthy
[OK] Latest Ledger Sequence: 1045920 (Protocol v22, Hash: ...)
```
