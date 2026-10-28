# ⚡ EchoGrid — Decentralized NegaWatt Marketplace

> *"To balance the grid in Ghana, we don't need millions of dollars in imported smart meters. We just need to listen."*

EchoGrid replaces expensive IoT smart meters with the microphone on ubiquitous smartphones. Local Edge AI detects the distinct low-frequency acoustic signature of heavy appliances (ACs, water pumps). When the grid is strained, a municipal authority funds a Soroban smart contract bounty. Users are challenged to power down. If the AI detects the sudden absence of the appliance hum, it generates a cryptographic **Proof of Power-Down**, which a Rust oracle verifies before triggering an autonomous Stellar micro-payment.

---

## Monorepo Structure

`
EchoGrid/
├── contracts/
│   └── negawatt_escrow/       # Phase 1 — Soroban Smart Contract (Rust/no_std)
│       ├── Cargo.toml
│       └── src/lib.rs
├── oracle/                    # Phase 2 — Rust Oracle / AgentPay API (Axum)
│   ├── Cargo.toml
│   ├── .env.example
│   └── src/
│       ├── main.rs
│       ├── routes.rs
│       ├── stellar.rs
│       └── types.rs
├── acoustic_cli/              # Phase 3 — Edge AI Mock CLI (Rust)
│   ├── Cargo.toml
│   └── src/main.rs
├── Cargo.toml                 # Workspace root (oracle + acoustic_cli)
└── README.md
`

---

## Architecture

`
  [Smartphone Mic]
       |
       v
  [Edge AI Model]  --- detects AC hum ON / OFF --->  [Acoustic CLI]
                                                           |
                                                 generates Proof-of-Power-Down
                                                           |
                                                           v
                                                     [Rust Oracle]
                                                  POST /api/v1/proof
                                                           |
                                              verifies cryptographic proof
                                                           |
                                                           v
                                             [Soroban Smart Contract]
                                                  (NegaWatt Escrow)
                                                           |
                                               releases USDC bounty
                                                           |
                                                           v
                                                [User Stellar Wallet]
`

---

## Quick Start

### Prerequisites
- Rust stable + Cargo
- `soroban-cli`: `cargo install --locked soroban-cli`

### Phase 1 — Build & Test the Contract
`ash
cd contracts/negawatt_escrow

# Build to WASM
cargo build --target wasm32-unknown-unknown --release

# Optimise binary size
soroban contract optimize --wasm target/wasm32-unknown-unknown/release/negawatt_escrow.wasm

# Run the in-process unit tests
cargo test
`

### Phase 2 — Run the Oracle
`ash
cd oracle
cp .env.example .env      # fill in keys for live-network mode (optional for MVP)
cargo run
# Server starts on http://localhost:8080
`

### Phase 3 — Run the Acoustic CLI Demo
`ash
cd acoustic_cli
# Replace G... with any 56-char Stellar address for the demo
cargo run -- --wallet GABC1234567890ABCDEF1234567890ABCDEF1234567890ABCDEF1234 --challenge-id ch_001
`

Expected output:
`
[EchoGrid] Initialising acoustic classifier...
[EchoGrid] Listening to ambient noise...
[EchoGrid] AC Hum DETECTED (72 Hz +/-4, confidence 94%)
[EchoGrid] Challenge received [ch_001] - please turn off your appliance!
[EchoGrid] Monitoring for power-down event...
[EchoGrid] AC Hum ABSENT - power-down confirmed!
[EchoGrid] Generating Proof-of-Power-Down...
[EchoGrid] Sending proof to EchoGrid Oracle...
[EchoGrid] NegaWatt bounty received! TX: MOCK_TX_...
`

---

## Security Model (AgentGuard RBAC)

| Role | Address stored in | Capabilities |
|------|------------------|--------------|
| ROLE_MUNICIPALITY | DataKey::MunicipalityAdmin | initialize, deposit, set_bounty |
| ROLE_ORACLE | DataKey::OracleAddress | payout only |

Every privileged function calls ddress.require_auth() — the Stellar network verifies Ed25519 signatures at the protocol level. No trust placed in callers' self-reported identity.

**Replay protection:** every Proof-of-Power-Down hash is stored in persistent storage after first use. A second submission with the same hash panics immediately.

---

## Live Network Integration (Stellar Testnet)

1. Deploy the contract: `soroban contract deploy --wasm negawatt_escrow.wasm --network testnet`
2. Copy the returned contract ID to oracle/.env as `CONTRACT_ID`
3. Add your oracle keypair as `ORACLE_SECRET_KEY`
4. Run the oracle with `cargo run --features live-network`

The stellar.rs file contains a step-by-step guide for wiring up the XDR-building + Horizon submission code.

### Production Deployment (Fly.io)

```bash
# 1. Provision persistent volume
fly volumes create echogrid_data --size 1

# 2. Configure secret key in platform vault
fly secrets set ORACLE_SECRET_KEY="S..."

# 3. Deploy via justfile
just deploy-fly
```
