# EchoGrid MVP Demo Video Script (75 Seconds)

## Production Guidelines & Recording Instructions

- **Target Total Duration:** 75 seconds (Acceptable range: 60–90 seconds).
- **Target Resolution:** 1080p Full HD (1920×1080 landscape or 1080×1920 portrait mobile screen recording).
- **Audio Capture:** Clear voiceover narration; mic gain calibrated to avoid clipping.
- **Privacy & Security Checklist:**
  - Enable **Do Not Disturb** on the recording device (zero notifications, badges, or personal message popups).
  - **Zero Secrets On Screen:** Never open the "Export Secret Key" modal in the Wallet tab; never show private seed phrases or secret keys.
  - **Simulation Disclosures:** Ensure any simulated grid event displays the "Simulated" banner.
  - Microphone claim must be performed with live acoustic capture and submit to live Stellar Testnet.

---

## Shot-by-Shot Storyboard & Narration

### Shot 1: Grid Strain Alert (0:00 – 0:10 | 10s)
- **Visual:** Open PWA at [https://echogrid-pwa.vercel.app](https://echogrid-pwa.vercel.app). Landing on the **Grid** tab. Point out the active demand-response event banner with the prominent `[Simulated]` label.
- **Narration:**
  > *"When peak electrical demand stresses the municipal grid, EchoGrid incentivizes residents to shed heavy inductive appliance load without expensive smart meters."*

---

### Shot 2: Instant Non-Custodial Wallet (0:10 – 0:22 | 12s)
- **Visual:** Tap the **Wallet** tab. Generate or show the funded Demo Browser Keypair. Highlight the Stellar Testnet balance and Friendbot button. (Do NOT click export private key).
- **Narration:**
  > *"Users connect a non-custodial Stellar wallet or generate a temporary in-browser keypair funded instantly via Friendbot on Stellar Testnet."*

---

### Shot 3: Real-Time Acoustic Spectrogram (0:22 – 0:38 | 16s)
- **Visual:** Tap the **Monitor** tab. Tap "Start Listening". The live Canvas spectrogram and 50/60 Hz band meters render in real-time as an appliance (or speaker playing 100/120 Hz mains hum) runs. Point out calibration baseline locks.
- **Narration:**
  > *"Using pure in-browser DSP via Web Audio API and Web Workers, EchoGrid's Edge detector listens for the characteristic acoustic hum of heavy motors and air conditioners. Audio never leaves the device."*

---

### Shot 4: Appliance Power-Down & Detection (0:38 – 0:50 | 12s)
- **Visual:** Turn off the appliance (or stop the hum track). The spectrogram frequency band drops sharply; the band meter falls from green to black. The detector automatically transitions from *Monitoring* to *Power-Down Detected (30 dB drop)*.
- **Narration:**
  > *"When the appliance powers off, our streaming detector recognizes the sudden drop in mains frequency harmonics and immediately constructs a cryptographic proof of power-down."*

---

### Shot 5: Proof Submission & On-Chain Settlement (0:50 – 1:05 | 15s)
- **Visual:** The client submits the proof to the live Rust Oracle at `https://echogrid-oracle.fly.dev`. A spinner runs briefly; the transaction confirms on Stellar Testnet. Tap the transaction link to view the live settlement transaction on StellarExpert.
- **Narration:**
  > *"The proof is verified by the live Rust oracle against a time-bound challenge nonce. The Soroban escrow contract executes an automated micropayment bounty on Stellar Testnet within seconds."*

---

### Shot 6: Honest Framing & Transparency Close (1:05 – 1:18 | 13s)
- **Visual:** Open the in-app **About / How it works** sheet from the header or first-run notice banner. Highlight the trust disclosure.
- **Narration (Canonical Statement Verbatim):**
  > *"The EchoGrid MVP trusts the client's claim of an appliance power-down. While cryptographic challenge nonces, recipient cooldowns, and on-chain spending caps strictly limit potential abuse, they do not prove that an appliance physically turned off. This live demonstration showcases the end-to-end micropayment loop and real-time acoustic detection user experience on Stellar Testnet; physical and cryptographic verification remains post-MVP work."*

---

## Technical Timing Summary

| Shot # | Timestamp Range | Duration | Focus Area |
| :---: | :---: | :---: | :--- |
| **1** | `00:00 - 00:10` | 10 s | Simulated Grid Strain Event |
| **2** | `00:10 - 00:22` | 12 s | Non-custodial Wallet & Friendbot |
| **3** | `00:22 - 00:38` | 16 s | Live Web Audio DSP & Spectrogram |
| **4** | `00:38 - 00:50` | 12 s | Appliance Power-Off & Detection |
| **5** | `00:50 - 01:05` | 15 s | Oracle Verification & Stellar Settlement |
| **6** | `01:05 - 01:18` | 13 s | Canonical Trust Statement & Close |
| **Total** | `00:00 - 01:18` | **78 s** | **Within 60–90 second target** |
