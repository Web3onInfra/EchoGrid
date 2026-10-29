# EchoGrid Phase 3: Web App (PWA) Design Document

- **Status:** Proposed (Under Review — Gate 1)
- **Target Network:** Stellar Testnet
- **App Directory:** `app/`
- **Stack:** Vite, React 19, TypeScript, Tailwind CSS, `@stellar/stellar-base`, `@stellar/freighter-api`, `vite-plugin-pwa`
- **Hosting Target:** Cloudflare Pages (or Vercel pending Gate 1 confirmation)

---

## 1. Routes & Mobile Navigation

EchoGrid is designed mobile-first with a fixed bottom tab bar (44 px minimum touch targets, safe-area-inset padding) and an omnipresent Testnet status header:

1. **Grid (`/`):** Simulated municipal grid strain view. Displays live power frequency (e.g. 49.8 Hz in 50 Hz grid or 59.7 Hz in 60 Hz grid), alert severity badge, and real bounty parameters dynamically fetched from the live Oracle's `GET /api/v1/config`.
2. **Monitor (`/monitor`):** Real-time acoustic capture, live spectrogram canvas (50/60/120 Hz focus), band-energy level meters, appliance state tracking, and challenge-proof trigger. Lazy-loaded to preserve initial bundle budget.
3. **Wallet (`/wallet`):** Stellar testnet wallet connector. Supports Freighter browser extension/wallet, paste-address read-only mode, and an in-app ephemeral Demo Keypair funded via Stellar Friendbot.
4. **Receipt (`/receipt/:id`):** Visual confirmation of settlement. Displays paid bounty, transaction hash with StellarExpert testnet explorer link, copy/share actions, or explicit simulated-mode notice.

---

## 2. Monitor & Claim State Machine

The acoustic monitoring and settlement lifecycle is governed by an explicit finite state machine:

```
[Idle]
  │ (User taps "Start Monitoring")
  ▼
[Permission] ── (Denied / No mic) ──► [Error: MicDenied]
  │ (getUserMedia granted with DSP off)
  ▼
[Calibrating] (Estimates ambient baseline for 3-5 s)
  │ (Appliance hum detected > 6 dB SNR)
  ▼
[Armed] ◄─── (Background Challenge Refresher every 60s)
  │ (Appliance hum drops >= 6 dB for >= 3.0 s)
  ▼
[Drop Confirmed] (Locks freshest unexpired challenge nonce)
  │ (Auto-triggers or user confirms "Claim Bounty")
  ▼
[Claiming] (Derives Proof v1 SHA-256 pre-image; POST /api/v1/proof)
  │
  ├── (HTTP 200 Confirmed) ────────► [Success: Confirmed]
  ├── (HTTP 202 Accepted) ─────────► [Pending: Polling]
  │                                       │ (Poll /claims/:id)
  │                                       ├── (Finalized) ──► [Success: Confirmed]
  │                                       └── (Timeout/Fail)► [Error: ClaimFailed]
  └── (HTTP 4xx/5xx / Network Fail) ──► [Error: Rejected]
```

### Challenge TTL Background Rotation
- Oracle challenges expire in `120s`. Monitoring sessions can run arbitrarily long while waiting for a power-down event.
- **Strategy:** Once armed, a background timer requests a fresh challenge via `GET /api/v1/challenge?wallet={address}` every `60s` (well within the 6 req/min rate limit), retaining the two freshest challenges.
- When power-down is confirmed, the app binds the derived acoustic proof to the freshest unexpired challenge (`expires_at - now > 15s`).
- *Phase 2 Schema Compatibility:* Confirmed compatible. Unused challenges simply expire in Oracle memory without penalty or rate penalty.

---

## 3. Wallet Strategy

All operations strictly target Stellar Testnet (`Test SDF Network ; September 2015`). The app halts and disables signing if the oracle reports any other network.

1. **Provider Abstraction:** Unified interface `IWalletProvider`:
   - `getPublicKey(): Promise<string>`
   - `signProof?(payload: Uint8Array): Promise<Uint8Array>` (optional attestation)
   - `isAvailable(): boolean`
2. **Freighter Integration:** Queries `@stellar/freighter-api`. Checks extension presence and testnet network configuration. If unavailable (e.g. mobile Safari/Chrome), shows clear fallback messaging.
3. **Demo Wallet & Friendbot:**
   - Generates local Ed25519 keypair via `@stellar/stellar-base`.
   - Prominently labeled with a high-visibility warning: *"Testnet Demo Key — Never store real funds"*.
   - One-tap Friendbot funding via `https://friendbot.stellar.org?addr={publicKey}`.
   - Keys stored in browser `sessionStorage` (cleared on tab close unless exported).
4. **Read-Only Paste Mode:** Allows entering any valid 56-character `G...` Stellar address for testing observation.

---

## 4. Grid Strain Simulation Engine

To test grid responsiveness without real utility dispatch:
- **Scenarios:** `Normal` (frequency 50.00 Hz / bounty idle), `Strain` (49.82 Hz / bounty active), and `Emergency` (49.50 Hz / peak alert).
- **Simulated Banner:** Every grid card and banner is stamped with a permanent `[SIMULATED SCENARIO]` badge.
- **Dynamic Oracle Parameters:** Payout bounty (e.g. 1 XLM) and daily limits are loaded live from `GET /api/v1/config`.

---

## 5. Simulator Mode Policy

For environments without physical appliance hum:
- A built-in Web Audio oscillator generates the synthetic 50/60/120 Hz acoustic tone.
- When the user toggles "Cut Power", the simulator mutes the tone, triggering the real TypeScript detector state machine.
- Proofs submitted from simulator mode set `simulated: true` in the Proof v1 payload.
- As defined in Phase 2, the Oracle returns mock settlement receipts with fake transaction hashes (`SIM_TX_...`) without consuming live testnet lumens or daily limits. The UI displays a distinct purple "Simulated Payout (Zero On-Chain XLM)" banner.

---

## 6. Audio Architecture & Privacy

### Privacy Guarantee: Zero Audio Exfiltration
- **Fundamental Principle:** Raw audio never leaves the device.
- The `AudioContext` and `AudioWorklet` run locally. No audio buffers or raw waveform chunks are ever transmitted over the network or saved to persistent storage.
- The payload sent to `POST /api/v1/proof` strictly contains derived summaries: `wallet_address`, `challenge_id`, `timestamp`, `nonce`, `proof_hash`, `detector_version`, and `duration_seconds`.
- Automated test assertion verifies that outgoing HTTP payloads contain no binary or base64 audio chunks.

### Audio Pipeline
- `getUserMedia` requested with mandatory raw constraints:
  ```ts
  { audio: { channelCount: 1, sampleRate: 16000, echoCancellation: false, noiseSuppression: false, autoGainControl: false } }
  ```
- Audio samples stream to an `AudioWorkletNode` feeding a ring buffer.
- Processing runs in a dedicated Web Worker executing STFT and band-power calculations matching `detector_params.json` (FFT size 1024, hop 512, bands: 45-55 Hz, 55-65 Hz, 115-125 Hz).
- Feature summaries (band energies) post back to the main thread at ~15 fps for canvas visualization.

---

## 7. PWA Architecture, Caching & Security

- **Service Worker (`vite-plugin-pwa`):**
  - **App Shell & Assets:** Precached via Workbox (`index.html`, bundle JS, CSS, icons).
  - **Static Assets:** `StaleWhileRevalidate` for web fonts and SVG icons.
  - **API Requests:** `NetworkOnly` for all Oracle endpoints (`/api/v1/*`), Friendbot, and Soroban RPC. API responses are **never cached**.
  - **Offline Fallback:** Dedicated offline screen notifying user that claim settlement requires an active network connection.
- **Content Security Policy (CSP):**
  ```text
  default-src 'self';
  script-src 'self';
  style-src 'self' 'unsafe-inline';
  connect-src 'self' https://echogrid-oracle.fly.dev http://localhost:8080 https://friendbot.stellar.org https://soroban-testnet.stellar.org https://stellar.expert;
  img-src 'self' data:;
  Permissions-Policy: microphone=(self)
  ```
- **Foreground Audio Limitation:**
  - Modern mobile browsers suspend `AudioContext` and microphone streams when the page is placed in the background or device sleeps.
  - Mitigated via the Screen Wake Lock API while `Armed`.
  - Prominent UI warning: *"Keep screen open during monitoring. Background capture is paused by iOS/Android."*

---

## 8. Accessibility & Quality Budgets

- **Accessibility (WCAG 2.1 AA):**
  - All interactive elements have $\ge 44 \times 44\text{ px}$ hit targets.
  - Detector state changes announced via `aria-live="polite"`.
  - Color contrast ratio $\ge 4.5:1$ on text; colorblind-friendly meter palettes.
  - Respects `prefers-reduced-motion` by disabling spectrogram waterfall scrolling.
  - Optional haptic feedback via `navigator.vibrate` on power-down detection.
- **Performance Budgets:**
  - Initial JS bundle $\le 200\text{ KB}$ gzipped (`Monitor` screen lazy-loaded).
  - Main-thread budget: audio worker keeps UI thread execution $< 16\text{ ms}$ per frame (60 fps).
  - Lighthouse mobile: Performance $\ge 90$, Accessibility $\ge 95$.

---

## 9. Risk Matrix & Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| **iOS Safari AudioContext Suspended** | Audio will not capture on boot. | AudioContext created/resumed strictly inside user-gesture tap ("Start Monitoring"). |
| **iOS Standalone PWA Mic Permission** | In older iOS versions, WebKit PWA drops mic permissions. | Detect standalone mode; show clear tutorial if permission dialog fails. |
| **Mobile Hardware High-Pass Filter** | Fundamental 50/60 Hz severely attenuated by phone mics. | Port Phase 0 multi-band detector emphasizing 2nd harmonic (`120 Hz`). |
| **Challenge Expiry During Long Monitoring** | Claim rejected with `CHALLENGE_EXPIRED` (401). | Background challenge refresher rotates challenges every 60s while armed. |
| **Network Interruption During Claim** | Uncertainty whether claim confirmed. | Submit returns `202 Pending` with `poll_url`; persisted in `localStorage` for retry. |
