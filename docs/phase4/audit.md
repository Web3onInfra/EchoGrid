# Phase 4: Gap Audit & Verification Plan

## 1. Executive Summary

This audit assesses the state of **EchoGrid** across all previous milestones (Phase 0 through Phase 3) prior to executing **Phase 4: Ship**. Every acceptance criterion has been evaluated by directly executing the relevant commands and verifying live network endpoints.

---

## 2. Phases 0–3 Definitions of Done vs. Reality

| Phase | Defined Deliverable | Audit Command Executed | Actual Result | Status |
| :--- | :--- | :--- | :--- | :---: |
| **Phase 0** | Python DSP hum detection, STFT feature extraction & golden vectors | `research/hum_detection/.venv/Scripts/python.exe -m pytest research/hum_detection/tests` | **22 passed** in 18.97s. Golden vectors and spectral analysis pass. | `VERIFIED` |
| **Phase 0** | Soroban RPC spike & live testnet payout characterization | `cargo test -p soroban_rpc` | **5 passed**, 0 failed. Deserialization and tx parsing verified. | `VERIFIED` |
| **Phase 1** | Soroban `negawatt_escrow` contract with caps, budgets, pause, and replay guards | `cargo test --manifest-path contracts/negawatt_escrow/Cargo.toml` | **47 passed**, 0 failed. Invariant test suites and error branches pass. | `VERIFIED` |
| **Phase 1** | Testnet deployment & contract size budget ($\le 30$ KB) | Inspected `deployments/testnet.json` and on-chain contract state | Contract ID [`CDFOK4NF4CY...`](https://stellar.expert/explorer/testnet/contract/CDFOK4NF4CY27N4G5PYA6SXTJA5MEKPVU7DQRMJUS74LIXHOKYNRHOPF) confirmed on ledger; WASM size **25,455 bytes**. | `VERIFIED` |
| **Phase 2** | Live Rust Oracle daemon with Axum, SQLite WAL, cooldowns, and rate limiting | `cargo test --workspace` | **74 passed**, 0 failed (66 oracle tests, 3 acoustic_cli tests, 5 spike tests). | `VERIFIED` |
| **Phase 2** | Oracle production deployment & operational endpoints | `curl -s https://echogrid-oracle.fly.dev/api/v1/health` and `/api/v1/ready` | HTTP 200 OK: `status: "ok"`, `version: "0.1.0"`, `rpc_healthy: true`, `contract_paused: false`. | `VERIFIED` |
| **Phase 3** | Mobile-first React/TypeScript PWA with Web Audio DSP and wallet integrations | `npm --prefix app test` | **31 test suites passed, 158 tests passed** (including a11y axe-core audits). | `VERIFIED` |
| **Phase 3** | Client JS bundle size budget ($\le 200$ KB gzipped) | `npm --prefix app run build` | Main bundle **184.87 KB** gzipped (`dist/assets/index-*.js`). | `VERIFIED` |
| **Phase 3** | Live PWA deployment on Vercel | `curl -I https://echogrid-pwa.vercel.app` | HTTP 200 OK. Content-Security-Policy & Permissions-Policy verified. | `VERIFIED` |

---

## 3. Phase 4 Conditional-Item Matrix

| Item | Requirement | Current Repository Status | Gap Action |
| :--- | :--- | :--- | :---: |
| **CI Gate** | Unified required CI gate with concurrency, caching, Dependabot, audit, link-check | `.github/workflows/ci.yml` has separate path-filtered jobs without aggregate `ci-ok` check; no lychee link checker; no Dependabot configuration. | **FILL** (PR 3) |
| **Oracle Operations** | Verified `GET /api/v1/health`, `/ready`, documented log field schema, secret redaction test, uptime monitor | Endpoints exist and are live; log fields not formally schema-documented; access-log redaction lacks explicit test; no automated uptime ping workflow. | **FILL** (PR 4) |
| **API Documentation** | `/challenge` and `/proof` request/response shapes documented with curl examples | `docs/api/openapi.yaml` exists, but root README lacks reproducible curl quickstarts and error response shapes. | **FILL** (PR 6) |
| **Research Reproducibility** | Python STFT hum-detection prototype reproducible by outsiders | Prototype code and tests exist, but setup steps, venv requirements, and commands need explicit documentation. | **FILL** (PR 6) |
| **Honest Framing** | Canonical text: MVP trusts client claim, limits damage via nonce/cooldown, verification is post-MVP | Absent across README, app, demo scripts, and release notes. | **FILL** (PR 2, 5, 6, 7, 9) |

---

## 4. Discrepancies Discovered & Remediations

1. **Discrepancy 1 — Missing Phase 1 Exit Report:**
   - *Finding:* `docs/phase1/EXIT.md` was missing from the repository (Phase 1 only contained `contract-interface.md` and `design.md`).
   - *Remediation:* Created `docs/phase1/EXIT.md` capturing on-chain addresses, transaction hashes, WASM size metrics, and configuration parameters.

2. **Discrepancy 2 — Phase 2 Exit Report Filename Inconsistency:**
   - *Finding:* Phase 2 exit report was named `docs/phase2/exit-report.md` instead of `docs/phase2/EXIT.md`.
   - *Remediation:* Created `docs/phase2/EXIT.md` mirror to satisfy canonical path resolution.

3. **Discrepancy 3 — CI Missing Aggregate Required Status Check:**
   - *Finding:* Path filtering in `.github/workflows/ci.yml` skips jobs for unaffected components on pull requests. Because GitHub branch protection requires consistent status check names, PRs touching only documentation or app code would hang or fail branch protection.
   - *Remediation:* PR 3 will introduce an aggregate `ci-ok` gate that depends on all component jobs and always runs.

4. **Discrepancy 4 — Smoke Test Runner Shebang Vitest VM Collision:**
   - *Finding:* `app/scripts/smoke-test.js` contained `#!/usr/bin/env node`, causing `SyntaxError: Invalid or unexpected token` when imported by `app/src/pwa/smokeTest.test.ts` in Vitest.
   - *Remediation:* Handled shebang in `smoke-test.js`, restoring 100% test pass rate for all 31 client test suites.

5. **Discrepancy 5 — System Python Environment Isolation:**
   - *Finding:* System Python (v3.14) lacks `scipy` and `soundfile`. Tests only pass when run inside `research/hum_detection/.venv`.
   - *Remediation:* Standardize instructions and CI on `uv` or explicit `.venv` activation.

6. **Discrepancy 6 — In-App Trust Disclosure Absent:**
   - *Finding:* The deployed PWA does not present the honest framing or "what's mocked" disclosure to the end user.
   - *Remediation:* PR 5 will add an in-app "How it works" sheet and first-run banner containing the canonical trust statement.
