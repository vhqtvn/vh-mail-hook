use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
pub struct Config {
    /// SQLite database path (e.g. 'data.db' or ':memory:' for in-memory database)
    #[arg(long, env = "DATABASE_PATH", default_value = "data.db")]
    pub database_path: String,

    /// SMTP server bind address
    #[arg(long, env = "SMTP_BIND_ADDR", default_value = "127.0.0.1:2525")]
    pub smtp_bind_addr: String,

    /// SMTP TLS server bind address
    #[arg(long, env = "SMTP_TLS_BIND_ADDR", default_value = "127.0.0.1:465")]
    pub smtp_tls_bind_addr: String,

    /// TLS certificate file path (optional, enables TLS if provided)
    #[arg(long, env = "TLS_CERT_PATH")]
    pub tls_cert_path: Option<PathBuf>,

    /// TLS private key file path (required if TLS cert is provided)
    #[arg(long, env = "TLS_KEY_PATH")]
    pub tls_key_path: Option<PathBuf>,

    /// TLS chain file path (required if TLS cert is provided)
    #[arg(long, env = "TLS_CHAIN_PATH")]
    pub tls_chain_path: Option<PathBuf>,

    /// Blocked IP networks in CIDR format (e.g. "10.0.0.0/8,192.168.0.0/16")
    #[arg(long, env = "BLOCKED_NETWORKS", value_delimiter = ',')]
    pub blocked_networks: Option<Vec<String>>,

    /// Maximum email size in bytes
    #[arg(long, env = "MAX_EMAIL_SIZE", default_value = "10485760")] // 10MB
    pub max_email_size: usize,

    /// Rate limit for emails per hour per IP
    #[arg(long, env = "RATE_LIMIT_PER_HOUR", default_value = "100")]
    pub rate_limit_per_hour: u32,

    /// Enable greylisting
    #[arg(long, env = "ENABLE_GREYLISTING")]
    pub enable_greylisting: bool,

    /// Greylist delay in minutes
    #[arg(long, env = "GREYLIST_DELAY", default_value = "5")]
    pub greylist_delay: u64,

    /// Enable SPF validation
    #[arg(long, env = "ENABLE_SPF")]
    pub enable_spf: bool,

    /// Enable DKIM validation
    #[arg(long, env = "ENABLE_DKIM")]
    pub enable_dkim: bool,

    /// Cleanup interval in minutes
    #[arg(long, env = "CLEANUP_INTERVAL", default_value = "60")]
    pub cleanup_interval: u64,

    /// TLS file polling interval in seconds (for watching TLS certificate changes)
    #[arg(long, env = "TLS_POLL_INTERVAL", default_value = "300")]
    pub tls_poll_interval: u64,
} 