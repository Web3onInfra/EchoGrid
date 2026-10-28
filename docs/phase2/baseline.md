# Phase 2 Baseline: Existing Test Suite & Tooling Status

- **Date:** 2026-10-28
- **Phase:** Phase 2 (Live Oracle) - PR 1 Baseline

---

## 1. Test Suite Execution Baseline

Run command: `cargo test --workspace`

### Results Summary
| Package | Binary / Target | Tests Passed | Tests Failed | Tests Ignored | Total |
|---|---|---|---|---|---|
| `acoustic-cli` | `src/main.rs` | 0 | 0 | 0 | 0 |
| `echogrid-oracle` | `src/main.rs` | 2 | 0 | 0 | 2 |
| `soroban-rpc-spike` | `src/main.rs` | 5 | 0 | 0 | 5 |
| `soroban-rpc-spike` | `tests/negative_paths.rs` | 1 | 0 | 3 | 4 |
| **Total** | | **8** | **0** | **3** | **11** |

### Individual Test Inventory

#### `echogrid-oracle`
- `types::tests::test_error_taxonomy_status_codes`: **PASS** (verifies status code mappings for taxonomy)
- `types::tests::test_from_error_str_patterns`: **PASS** (verifies string pattern matching to error variants)

#### `soroban-rpc-spike`
- `tests::test_deserialize_latest_ledger_response`: **PASS**
- `tests::test_deserialize_health_response`: **PASS**
- `tests::test_deserialize_get_transaction_response`: **PASS**
- `tests::test_extract_tx_hash_from_cli_output`: **PASS**
- `tests::test_generate_random_proof_hash_length`: **PASS**
- `tests/negative_paths.rs::test_error_taxonomy_unit_mapping`: **PASS**
- `tests/negative_paths.rs::test_live_insufficient_pool_balance_taxonomized`: **IGNORED** (live testnet integration)
- `tests/negative_paths.rs::test_live_replay_proof_rejected`: **IGNORED** (live testnet integration)
- `tests/negative_paths.rs::test_live_unauthorized_signer_rejected`: **IGNORED** (live testnet integration)

---

## 2. CI & Security Tooling Baseline

- `cargo fmt --check`: **PASS**
- `cargo clippy --workspace --all-targets -- -D warnings`: **PASS**
- `cargo deny check`: **PASS** (`advisories ok`, `bans ok`, `licenses ok`, `sources ok`)
- Workspace licenses: Standardized on `MIT OR Apache-2.0` with `deny.toml` configuring allowed OSI/FSF licenses.
