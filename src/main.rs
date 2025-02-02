use clap::Parser;
use tokio::try_join;
use tracing::{error, info};

#[derive(Parser)]
pub struct Config {
    /// Web app URL (e.g. 'https://example.com')
    #[arg(long, env = "WEB_APP_URL", default_value = "https://example.com")]
    pub web_app_url: String,

    /// SQLite database path (e.g. 'data.db' or ':memory:' for in-memory database)
    #[arg(long, env = "DATABASE_PATH", default_value = "data.db")]
    pub database_path: String,

    /// HTTP server bind address
    #[arg(long, env = "WEB_BIND_ADDR", default_value = "127.0.0.1:3000")]
    pub web_bind_addr: String,

    /// SMTP server bind address
    #[arg(long, env = "SMTP_BIND_ADDR", default_value = "127.0.0.1:2525")]
    pub smtp_bind_addr: String,

    /// Email domain for generated mailbox addresses (e.g. 'example.com')
    #[arg(long, env = "EMAIL_DOMAIN", default_value = "example.com")]
    pub email_domain: String,

    /// TLS certificate path (optional)
    #[arg(long, env = "TLS_CERT_PATH")]
    pub tls_cert_path: Option<std::path::PathBuf>,

    /// TLS private key path (optional)
    #[arg(long, env = "TLS_KEY_PATH")]
    pub tls_key_path: Option<std::path::PathBuf>,

    /// TLS certificate chain path (optional)
    #[arg(long, env = "TLS_CHAIN_PATH")]
    pub tls_chain_path: Option<std::path::PathBuf>,

    /// TLS file polling interval in seconds (for watching TLS certificate changes)
    #[arg(long, env = "TLS_POLL_INTERVAL", default_value = "300")]
    pub tls_poll_interval: u64,

    /// Maximum email size in bytes (default: 10MB)
    #[arg(long, env = "MAX_EMAIL_SIZE", default_value = "10485760")]
    pub max_email_size: usize,

    /// Rate limit per hour
    #[arg(long, env = "RATE_LIMIT_PER_HOUR", default_value = "100")]
    pub rate_limit_per_hour: u32,

    /// Enable greylisting
    #[arg(long, env = "ENABLE_GREYLISTING", default_value = "true")]
    pub enable_greylisting: bool,

    /// Greylist delay in minutes
    #[arg(long, env = "GREYLIST_DELAY", default_value = "5")]
    pub greylist_delay: u64,

    /// Enable SPF checking
    #[arg(long, env = "ENABLE_SPF", default_value = "true")]
    pub enable_spf: bool,

    /// Enable DKIM verification
    #[arg(long, env = "ENABLE_DKIM", default_value = "true")]
    pub enable_dkim: bool,

    /// Cleanup interval in minutes
    #[arg(long, env = "CLEANUP_INTERVAL", default_value = "60")]
    pub cleanup_interval: u64,

    /// Blocked IP networks in CIDR format (e.g. "10.0.0.0/8,192.168.0.0/16")
    #[arg(long, env = "BLOCKED_NETWORKS", value_delimiter = ',')]
    pub blocked_networks: Option<Vec<String>>,
}

#[tokio::main]
async fn main() {
    // Load .env file if it exists
    dotenv::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Parse command line arguments
    let config = Config::parse();

    info!("Starting mail hook application...");

    // Create web app config
    let web_config = web_app::Config {
        database_path: config.database_path.clone(),
        bind_addr: config.web_bind_addr.clone(),
        email_domain: config.email_domain.clone(),
        web_app_url: config.web_app_url.clone(),
    };

    // Create mail service config
    let mail_config = mail_service::Config {
        database_path: config.database_path.clone(),
        smtp_bind_addr: config.smtp_bind_addr.clone(),
        email_domain: config.email_domain,
        tls_cert_path: config.tls_cert_path,
        tls_key_path: config.tls_key_path,
        tls_chain_path: config.tls_chain_path,
        tls_poll_interval: config.tls_poll_interval,
        blocked_networks: config.blocked_networks,
        max_email_size: config.max_email_size,
        rate_limit_per_hour: config.rate_limit_per_hour,
        enable_greylisting: config.enable_greylisting,
        greylist_delay: config.greylist_delay,
        enable_spf: config.enable_spf,
        enable_dkim: config.enable_dkim,
        cleanup_interval: config.cleanup_interval,
    };

    // Run both services concurrently
    info!(
        "Starting web server on {} and SMTP server on {}",
        config.web_bind_addr, config.smtp_bind_addr
    );

    if let Err(e) = try_join!(web_app::run(web_config), mail_service::run(mail_config)) {
        error!("Application error: {}", e);
        std::process::exit(1);
    }
}
