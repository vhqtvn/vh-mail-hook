use tracing::info;
use web_app::{Config, run};
use clap::Parser;
use dotenv;

#[tokio::main]
async fn main() {
    // Load .env file if it exists
    dotenv::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Parse command line arguments
    let config = Config::parse();

    info!("Starting web application...");
    
    if let Err(e) = run(config).await {
        tracing::error!("Application error: {}", e);
        std::process::exit(1);
    }
} 