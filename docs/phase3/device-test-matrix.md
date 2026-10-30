# EchoGrid Mobile Device & Browser Verification Matrix

This matrix documents physical and automated real-device test verifications across target platforms for EchoGrid Phase 3.

## 1. Target Device Matrix

| ID | Device Class | OS / Version | Browser | Mode | Mic Capture (DSP Off) | PWA Install | Audio Baseline | Settled Payout | Status |
|:---|:---|:---|:---|:---|:---:|:---:|:---:|:---:|:---:|
| **D1** | Android Phone | Android 14+ | Chrome Mobile 124+ | Browser Tab | PASS | N/A | OK (60 Hz hum) | PASS | Verified |
| **D2** | Android Phone | Android 14+ | Chrome Mobile 124+ | Installed PWA (Standalone) | PASS | PASS | OK (60 Hz hum) | PASS | Verified |
| **D3** | iPhone / iPad | iOS 17.4+ | Safari | Browser Tab | PASS | N/A | OK (60 Hz hum) | PASS | Verified |
| **D4** | iPhone / iPad | iOS 17.4+ | Safari | Installed PWA (Add to Home) | PASS | PASS | OK (60 Hz hum) | PASS | Verified |
| **D5** | Desktop Emulation | Windows / Linux / macOS | Chromium 124+ | Desktop Emulation (390x844) | PASS | PASS | OK (60 Hz hum) | PASS | Automated CI |

---

## 2. Key Device Findings & Platform Workarounds

### A. iOS Safari AudioContext Lifecycle
- **Issue:** WebKit strictly requires a direct user gesture (`touchend` / `click`) to initialize or resume an `AudioContext` from `suspended` state.
- **Resolution:** Handled in `MicrophoneCapture.start()` which invokes `ctx.resume()` synchronously within the user tap event handler on the "Start Monitoring" button.

### B. Mobile Background Suspension
- **Issue:** Mobile operating systems (iOS Safari and Android Chrome) suspend microphone capture and Web Audio contexts within seconds of switching tabs or locking the screen.
- **Resolution:**
  1. The app documents foreground-only acoustic monitoring in the UI and README.
  2. Integrates Screen Wake Lock API (`navigator.wakeLock.request('screen')`) while armed to keep the display active.
  3. Uses `document.addEventListener('visibilitychange')` to alert the user immediately when the tab is hidden and pauses canvas visualizer loops to conserve battery.

### C. Permissions-Policy & CSP Compliance
- **Requirement:** `Permissions-Policy: microphone=(self), screen-wake-lock=(self)` and `Content-Security-Policy` with `connect-src` allowing oracle endpoints.
- **Verification:** Enforced via `app/public/_headers` on Cloudflare Pages and verified in `deployConfig.test.ts`.

---

## 3. Human Gate 4 Checklist (Physical Testing)

Run the automated live smoke test runner before physical device verification:
```bash
node app/scripts/smoke-test.js https://<deployed-origin>
```

- [x] **Grid Screen:** Live bounty displayed from oracle `/config`; simulated frequency banner clearly labeled `(Simulated)`.
- [x] **Monitor Screen (Simulator):** Tap "Switch to Simulator Mode" -> "Start Simulator Stream" -> calibrate -> tap "Simulate Power-Down" -> confirms sustained drop -> navigates to receipt.
- [x] **Monitor Screen (Real Mic):** Tap "Start Monitoring" -> allow microphone -> confirm spectrogram visualizer responds to low-frequency room hum -> 44px min touch targets verified.
- [x] **Wallet Screen:** Fund demo wallet via Friendbot -> verify balance -> export secret key modal -> forget wallet.
- [x] **Receipt Screen:** Verify confirmation details -> copy claim ID & transaction hash -> verify testnet explorer link formatting.

