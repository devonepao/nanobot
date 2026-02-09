mod agent;
mod bus;
mod channels;
mod cli;
mod config;
mod cron;
mod heartbeat;
mod providers;
mod session;
mod utils;

use clap::Parser;
use std::process;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Parse CLI arguments and run
    let result = cli::Cli::parse().run().await;

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
