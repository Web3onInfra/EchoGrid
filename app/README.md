# 📱 EchoGrid PWA — Acoustic Grid-Strain Settlement Client

The EchoGrid Progressive Web Application (PWA) enables ubiquitous smartphones to act as decentralized acoustic grid-strain sensors. By monitoring the acoustic power-down signature of heavy appliances (e.g., air conditioners, water heaters) during municipal grid strain events, participants can prove negawatt curtailment and receive instant Stellar Testnet micro-payments.

---

## 🚀 Key Features & Architecture

The mobile-first PWA is organized into four core screens:

1. **Grid**: Live grid status displaying simulated grid strain events, active substation telemetry, and oracle bounty parameters fetched from `/api/v1/config`.
2. **Monitor**: High-fidelity acoustic analysis running client-side in the browser. Features a real-time 0–200 Hz scrolling spectrogram, mains hum band meters (50 Hz / 60 Hz + harmonics), and a state-machine-driven calibration and arming workflow.
3. **Wallet**: Connect via Freighter extension on supported desktop/mobile browsers, create a browser-isolated Demo Testnet Wallet, fund with Friendbot, or paste a standard Stellar public address (`G...`).
4. **Receipt**: Transparent settlement history detailing verified claims, cryptographic pre-image hashes, and direct links to StellarExpert Testnet explorer.

---

## 🔒 Privacy & Security Guarantee

> **Audio never leaves the device.**
> The EchoGrid client processes all audio locally via high-resolution Short-Time Fourier Transform (STFT) and acoustic feature extractors in browser Web Workers. Only derived summary statistics (energy ratios, drop latency, timestamp, challenge nonce) and SHA-256 pre-images are submitted to the Oracle. Zero raw audio buffers or recordings are ever transmitted or stored.

- **Client is Untrusted**: All cryptographic proofs are verified on-chain against the Soroban Negawatt Escrow contract.
- **Public Bundle**: `VITE_*` environment variables are strictly public configuration flags. No private keys or secrets are bundled.
- **Testnet Only**: Hard-coded safety guards disable live payouts and transactions if connected to any network other than Stellar Testnet.

---

## 📲 Mobile Installation Guide (PWA)

EchoGrid is designed as a standalone Progressive Web App. For optimal performance, install to your mobile home screen:

### Android (Chrome / Chromium)
1. Open the deployed application URL in Chrome.
2. Tap the **Install App** banner at the top of the screen (or open the browser menu `⋮` and select **Install app** or **Add to Home screen**).
3. Confirm installation. The app will launch in standalone display mode without URL bars.

### iOS (Safari)
1. Open the deployed URL in Safari.
2. Tap the **Share** button (box with an upward arrow) in the bottom navigation bar.
3. Scroll down and tap **Add to Home Screen**.
4. Tap **Add** in the top-right corner. Launch EchoGrid directly from your home screen.

---

## 🎙️ Microphone Permissions & Troubleshooting

### Critical: Foreground-Only Monitoring
Web browsers deliberately suspend audio capture and `AudioContext` processing when a tab is backgrounded or when the mobile device screen locks.
- **Keep Screen On**: EchoGrid automatically requests a **Screen Wake Lock** while the monitor is calibrating or armed.
- **Foreground Operation**: Keep the EchoGrid PWA visible in the foreground while awaiting an appliance power-down event.

### Audio Permission Issues
- **Microphone Blocked**: If permission was denied, tap the padlock / permissions icon in your mobile browser address bar and toggle Microphone to **Allow**, then reload the app.
- **Hardware Mute**: On iOS, ensure the physical mute switch / Action Button is not disabling audio capture hardware.
- **DSP / Audio Effects**: EchoGrid automatically disables browser Echo Cancellation, Noise Suppression, and Auto Gain Control (AGC) so the low-frequency mains hum is not filtered out as noise.

---

## 🧪 Simulator Walkthrough (Testing Without Real Appliance)

EchoGrid includes a built-in hum synthesis engine for testing the complete end-to-end flow without waiting for a physical AC unit:

1. Navigate to the **Monitor** screen.
2. Toggle the **Simulator Mode** switch.
3. Tap **Start Monitoring**. The simulator begins synthesizing an authentic 60 Hz mains hum with realistic harmonic undertones and noise.
4. Allow the calibration phase to complete (~5 seconds) while the baseline hum is established.
5. Once the monitor transitions to the **Armed** state, tap **Simulate Appliance Power-Down**.
6. The detector will register the steep spectral drop within 300 ms, lock the challenge nonce, and display the confirmed power-down receipt!

---

## 🛠️ Development & Local Testing

### Prerequisites
- Node.js 20+
- Live Oracle daemon (`http://localhost:8080` or remote testnet oracle)

### Running Locally
```bash
cd app
npm install
npm run dev
```

### Production Build & Verification
```bash
npm run typecheck
npm run lint
npm test
npm run build
npm run size
```

### Live Smoke Testing
Run the automated PWA and Oracle endpoint health check:
```bash
node scripts/smoke-test.js http://localhost:5173 --skip-oracle
```

---

## 🌐 Deployment (Vercel)

EchoGrid PWA is configured for zero-configuration continuous deployment on Vercel:
- Configuration: `app/vercel.json` provides strict CSP headers, SPA rewrites, and Permissions-Policy.
- Pipeline: `.github/workflows/deploy-vercel.yml` automatically builds, tests, deploys, and smoke-tests every push to `main`.
