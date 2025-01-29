use std::sync::Arc;
use tracing::info;
use mail_service::{Config, run};
use clap::Parser;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Parse command line arguments
    let config = Config::parse();

    info!("Mail service starting...");
    
    if let Err(e) = run(config).await {
        tracing::error!("Mail service error: {}", e);
        std::process::exit(1);
    }
} 