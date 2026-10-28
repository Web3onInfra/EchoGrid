//! # EchoGrid — Acoustic CLI Binary (Edge AI Mock)
//!
//! CLI entrypoint for running acoustic detection simulation and claim settlement.

use acoustic_cli::{run_acoustic_workflow, WorkflowConfig};
use clap::Parser;
use colored::Colorize;

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

    /// Optional challenge ID. If omitted, fetched automatically via GET /api/v1/challenge.
    #[arg(long, short = 'c', env = "ECHOGRID_CHALLENGE_ID")]
    challenge_id: Option<String>,

    /// Oracle API base URL.
    #[arg(
        long,
        default_value = "http://localhost:8080",
        env = "ECHOGRID_ORACLE_URL"
    )]
    oracle_url: String,

    /// Mark claim as simulated (bypasses on-chain submission).
    #[arg(long, default_value_t = false, env = "ECHOGRID_SIMULATED")]
    simulated: bool,

    /// Skip interactive demo pauses.
    #[arg(long, default_value_t = false)]
    fast: bool,
}

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

fn main() {
    let args = Args::parse();
    print_banner();

    if !args.wallet.starts_with('G') || args.wallet.len() != 56 {
        eprintln!(
            "{} Wallet address must start with 'G' and be 56 characters long.",
            "[EchoGrid] [ERROR]".red().bold()
        );
        std::process::exit(1);
    }

    let config = WorkflowConfig {
        wallet: args.wallet,
        oracle_url: args.oracle_url,
        challenge_id: args.challenge_id,
        simulated: args.simulated,
        fast: args.fast,
    };

    match run_acoustic_workflow(&config) {
        Ok(receipt) => {
            println!(
                "\n{} Payout successfully settled!",
                "[EchoGrid] [SUCCESS]".green().bold()
            );
            println!("  Claim ID:     {}", receipt.claim_id.bright_white());
            println!("  Stellar TX:   {}", receipt.tx_hash.bright_cyan());
            println!("  Amount:       {} stroops", receipt.amount_stroops);
            println!("  Explorer:     {}", receipt.explorer_url.underline());
            println!("  Simulated:    {}", receipt.simulated);
            println!("\n  You just helped balance Ghana's power grid!\n");
        }
        Err(e) => {
            eprintln!(
                "\n{} Workflow failed: {e}\n",
                "[EchoGrid] [ERROR]".red().bold()
            );
            std::process::exit(1);
        }
    }
}
