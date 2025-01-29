pub mod config;
pub mod service;
pub mod smtp;
pub mod security;

use anyhow::Result;
pub use config::Config;  // Re-export Config
pub use service::MailService;  // Re-export MailService
use smtp::server::run_smtp_server;
use std::sync::Arc;
use std::time::Duration;

pub async fn run(mut config: Config) -> Result<()> {
    // Parse blocked networks
    let blocked_networks = config.blocked_networks.take()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|cidr| cidr.parse().ok())
        .collect();

    let db = common::db::SqliteDatabase::new(&format!("sqlite:{}", config.database_path)).await?;
    let service = Arc::new(MailService::new(
        Arc::new(db),
        config.email_domain.clone(),
        blocked_networks,
        config.max_email_size,
        config.rate_limit_per_hour,
        config.enable_greylisting,
        Duration::from_secs(config.greylist_delay * 60),
        config.enable_spf,
        config.enable_dkim,
    ).await?);

    // Start cleanup task
    let cleanup_service = service.clone();
    tokio::spawn(async move {
        cleanup_service.start_cleanup_task(Duration::from_secs(config.cleanup_interval * 60)).await;
    });

    // Run SMTP server
    run_smtp_server(&config, service).await?;

    Ok(())
}
