//! # EchoGrid — Acoustic CLI (Edge AI Mock)
//!
//! Simulates the on-device acoustic classification pipeline that would run
//! on a user's smartphone.  For the hackathon MVP this tool replaces a real
//! TFLite / ONNX audio model with a scripted state machine and coloured
//! terminal output — perfect for a live demo.
//!
//! ## Demo flow
//! ```
//! acoustic_cli --wallet G... --challenge-id ch_001
//!
//! [EchoGrid] 🎙  Initialising acoustic classifier...
//! [EchoGrid] 📡  Listening to ambient noise...
//! [EchoGrid] ⚡  AC Hum DETECTED  (72 Hz ±4, confidence 94%)
//! [EchoGrid] 📣  Challenge received — please turn off your appliance!
//! [EchoGrid] 👂  Monitoring for power-down event...
//! [EchoGrid] 🔇  AC Hum ABSENT  — power-down confirmed!
//! [EchoGrid] 🔐  Generating Proof-of-Power-Down...
//! [EchoGrid] 📤  Sending proof to EchoGrid Oracle...
//! [EchoGrid] ✅  NegaWatt bounty received!  TX: MOCK_TX_...
//! ```

use clap::Parser;
use colored::Colorize;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::thread;
use std::time::Duration;

// ---------------------------------------------------------------------------
// ── CLI Arguments ────────────────────────────────────────────────────────────
// ---------------------------------------------------------------------------

/// EchoGrid Acoustic CLI — simulates the smartphone Edge AI pipeline.
#[derive(Parser, Debug)]
#[command(
    name = "acoustic_cli",
    about = "EchoGrid Edge AI mock — generates Proof-of-Power-Down and claims NegaWatt bounty",
    version,
    author
)]
struct Args {
    /// Your Stellar wallet address (G...).
    #[arg(long, short = 'w', env = "ECHOGRID_WALLET")]
    wallet: String,

    /// The challenge ID issued by the municipality for this grid event.
    #[arg(long, short = 'c', default_value = "ch_grid_001")]
    challenge_id: String,

    /// Oracle API base URL.
    #[arg(long, default_value = "http://localhost:8080")]
    oracle_url: String,

    /// Skip the interactive pauses (useful for automated test runs).
    #[arg(long, default_value_t = false)]
    fast: bool,
}

// ---------------------------------------------------------------------------
// ── Proof Payload (must match oracle/src/types.rs) ───────────────────────────
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct ProofPayload {
    wallet_address: String,
    challenge_id: String,
    timestamp: String,
    proof_hash: String,
    nonce: String,
}

/// Oracle's response envelope.
#[derive(Deserialize, Debug)]
struct OracleResponse {
    success: bool,
    message: String,
    tx_hash: Option<String>,
}

// ---------------------------------------------------------------------------
// ── Main ─────────────────────────────────────────────────────────────────────
// ---------------------------------------------------------------------------

fn main() {
    let args = Args::parse();

    print_banner();

    // Validate wallet address format (basic sanity check — not full Ed25519).
    if !args.wallet.starts_with('G') || args.wallet.len() != 56 {
        eprintln!(
            "{} Wallet address must start with 'G' and be 56 characters long.",
            "[EchoGrid] ❌".red().bold()
        );
        std::process::exit(1);
    }

    let pause = |ms: u64| {
        if !args.fast {
            thread::sleep(Duration::from_millis(ms));
        }
    };

    // ── Phase 1: Classifier Boot ─────────────────────────────────────────────
    log_step("🎙 ", "Initialising acoustic classifier...", "cyan");
    pause(800);
    println!(
        "    {}  Model: EchoNet-Lite v1.2 | Classes: [AC_HUM, FAN, PUMP, AMBIENT]",
        "│".dimmed()
    );
    pause(600);

    // ── Phase 2: Ambient Listening ────────────────────────────────────────────
    log_step("📡 ", "Listening to ambient noise...", "cyan");
    pause(1200);

    // Simulate background frequency readings.
    let freqs = [68.1, 70.3, 71.9, 72.4, 72.1];
    for f in &freqs {
        println!("    {}  Frequency sample: {:.1} Hz", "│".dimmed(), f);
        pause(300);
    }

    // ── Phase 3: AC Hum Detected ──────────────────────────────────────────────
    log_step(
        "⚡ ",
        "AC Hum DETECTED  (72 Hz ±4, confidence: 94%)",
        "yellow",
    );
    pause(600);
    println!(
        "    {}  Appliance class: AIR_CONDITIONER | State: ON",
        "│".dimmed()
    );

    // ── Phase 4: Challenge Received ───────────────────────────────────────────
    pause(800);
    log_step(
        "📣 ",
        &format!(
            "Challenge received [{}] — please turn off your appliance!",
            args.challenge_id.bold()
        ),
        "magenta",
    );
    pause(1000);

    // ── Phase 5: Monitor for Power-Down ──────────────────────────────────────
    log_step("👂 ", "Monitoring for power-down event...", "cyan");
    let countdown_freqs = [71.8, 69.2, 55.1, 38.4, 12.2, 3.1, 0.4];
    for f in &countdown_freqs {
        pause(400);
        println!(
            "    {}  Frequency: {:.1} Hz  {}",
            "│".dimmed(),
            f,
            if *f < 10.0 {
                "↓↓↓".red().to_string()
            } else {
                "↓".yellow().to_string()
            }
        );
    }

    // ── Phase 6: Power-Down Confirmed ────────────────────────────────────────
    pause(500);
    log_step(
        "🔇 ",
        "AC Hum ABSENT — power-down event CONFIRMED!",
        "green",
    );
    pause(700);

    // ── Phase 7: Generate Proof-of-Power-Down ────────────────────────────────
    log_step("🔐 ", "Generating Proof-of-Power-Down...", "cyan");
    pause(400);

    // Timestamp: use current UTC seconds as a simple ISO string mock.
    // In production this would come from a trusted time source.
    let timestamp = chrono_now_mock();

    // Random 8-byte nonce prevents grinding attacks on known wallet addresses.
    let nonce_bytes: [u8; 8] = rand::thread_rng().gen();
    let nonce = hex::encode(nonce_bytes);

    // Pre-image: same format the oracle's verify_proof() expects.
    let pre_image = format!(
        "{}|{}|{}|{}",
        args.wallet, args.challenge_id, timestamp, nonce
    );

    let mut hasher = Sha256::new();
    hasher.update(pre_image.as_bytes());
    let proof_hash = hex::encode(hasher.finalize());

    println!(
        "    {}  Proof hash:  {}",
        "│".dimmed(),
        proof_hash.bright_white()
    );
    println!("    {}  Timestamp:   {}", "│".dimmed(), timestamp);
    println!("    {}  Nonce:       {}", "│".dimmed(), nonce);
    pause(600);

    // ── Phase 8: Submit to Oracle ─────────────────────────────────────────────
    let oracle_endpoint = format!("{}/api/v1/proof", args.oracle_url.trim_end_matches('/'));
    log_step(
        "📤 ",
        &format!("Sending proof to EchoGrid Oracle at {}...", oracle_endpoint),
        "cyan",
    );
    pause(400);

    let payload = ProofPayload {
        wallet_address: args.wallet.clone(),
        challenge_id: args.challenge_id.clone(),
        timestamp,
        proof_hash,
        nonce,
    };

    // Use reqwest's blocking client — fine for a CLI tool.
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to build HTTP client");

    match client.post(&oracle_endpoint).json(&payload).send() {
        Ok(resp) => {
            let status = resp.status();
            match resp.json::<OracleResponse>() {
                Ok(oracle_resp) if oracle_resp.success => {
                    pause(300);
                    log_step(
                        "✅ ",
                        &format!("NegaWatt bounty received!  {}", oracle_resp.message),
                        "green",
                    );
                    if let Some(tx) = oracle_resp.tx_hash {
                        println!("    {}  Stellar TX:  {}", "│".dimmed(), tx.bright_cyan());
                    }
                    println!(
                        "\n    {} You just helped balance Ghana's power grid! 🌍\n",
                        "🎉".green()
                    );
                }
                Ok(oracle_resp) => {
                    log_step(
                        "❌ ",
                        &format!("Oracle rejected proof: {}", oracle_resp.message),
                        "red",
                    );
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!(
                        "{} Failed to parse oracle response (HTTP {}): {e}",
                        "[EchoGrid] ❌".red().bold(),
                        status
                    );
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            // Network failure is expected if the oracle isn't running yet.
            // Provide a helpful message so judges know what to do.
            eprintln!(
                "{} Could not reach oracle at {oracle_endpoint}: {e}",
                "[EchoGrid] ❌".red().bold()
            );
            eprintln!(
                "    Tip: start the oracle first with  {}",
                "cd oracle && cargo run".bright_white()
            );
            // Still print the proof so the judge can manually verify.
            println!("\n    Generated payload (for manual verification):");
            println!(
                "    {}",
                serde_json::to_string_pretty(&payload).unwrap_or_default()
            );
            std::process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// ── Helpers ──────────────────────────────────────────────────────────────────
// ---------------------------------------------------------------------------

/// Print a coloured step log line.
fn log_step(icon: &str, msg: &str, color: &str) {
    let prefix = format!("[EchoGrid] {icon}").bold();
    let line = match color {
        "cyan" => format!("{prefix} {}", msg.cyan()),
        "yellow" => format!("{prefix} {}", msg.yellow()),
        "green" => format!("{prefix} {}", msg.green()),
        "red" => format!("{prefix} {}", msg.red()),
        "magenta" => format!("{prefix} {}", msg.magenta()),
        _ => format!("{prefix} {msg}"),
    };
    println!("{line}");
}

/// Produce a mock ISO-8601 UTC timestamp using elapsed seconds since epoch.
/// In a real app, use `chrono::Utc::now().to_rfc3339()`.
fn chrono_now_mock() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Format as a basic ISO string — not RFC 3339 but readable and unique.
    format!(
        "2026-10-27T{:02}:{:02}:{:02}Z",
        (secs / 3600) % 24,
        (secs / 60) % 60,
        secs % 60,
    )
}

/// Print an ASCII art banner for the demo presentation.
fn print_banner() {
    println!(
        "{}",
        r#"
  _____ _     _       ____      _     _ 
 | ____| |__ | | ___ / ___|_ __(_) __| |
 |  _| | '_ \| |/ _ \ |  _| '__| |/ _` |
 | |___| | | | |  __/ |_| | |  | | (_| |
 |_____|_| |_|_|\___|\____|_|  |_|\__,_|
    NegaWatt Marketplace — Acoustic Proof-of-Power
"#
        .bright_cyan()
        .bold()
    );
}
