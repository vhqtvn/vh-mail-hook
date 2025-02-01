use anyhow::Result;
use common::{AppError, Email, db::Database};
use std::{sync::Arc, net::IpAddr, time::Duration};
use tracing::{error, info, warn};
use dashmap::DashMap;
use governor::{RateLimiter, Quota, state::{InMemoryState, NotKeyed}};
use ipnetwork::IpNetwork;
use mail_parser::Message;
use trust_dns_resolver::TokioAsyncResolver;
use crate::security::encryption::encrypt_email;

#[derive(Clone)]
pub struct ServiceConfig {
    pub domain: String,
    pub blocked_networks: Vec<IpNetwork>,
    pub max_email_size: usize,
    pub rate_limit_per_hour: u32,
    pub enable_greylisting: bool,
    pub greylist_delay: Duration,
    pub enable_spf: bool,
    pub enable_dkim: bool,
}

pub struct MailService {
    db: Arc<dyn Database>,
    domain: String,
    blocked_networks: Vec<IpNetwork>,
    max_email_size: usize,
    rate_limiter: Arc<RateLimiter<NotKeyed, InMemoryState, governor::clock::DefaultClock>>,
    greylist: Arc<DashMap<(IpAddr, String, String), i64>>, // (IP, from, to) -> first_seen
    enable_greylisting: bool,
    greylist_delay: Duration,
    enable_spf: bool,
    enable_dkim: bool,
    #[allow(unused)]
    dns_resolver: TokioAsyncResolver,
}

impl MailService {
    pub async fn new(
        db: Arc<dyn Database>, 
        config: ServiceConfig,
    ) -> Result<Self> {
        let rate_limiter = Arc::new(RateLimiter::direct(Quota::per_hour(
            std::num::NonZeroU32::new(config.rate_limit_per_hour).unwrap()
        )));

        let dns_resolver = TokioAsyncResolver::tokio_from_system_conf()?;

        Ok(Self { 
            db,
            domain: config.domain,
            blocked_networks: config.blocked_networks,
            max_email_size: config.max_email_size,
            rate_limiter,
            greylist: Arc::new(DashMap::new()),
            enable_greylisting: config.enable_greylisting,
            greylist_delay: config.greylist_delay,
            enable_spf: config.enable_spf,
            enable_dkim: config.enable_dkim,
            dns_resolver,
        })
    }

    pub fn domain(&self) -> &str {
        &self.domain
    }

    pub fn max_email_size(&self) -> usize {
        self.max_email_size
    }

    pub async fn process_incoming_email(
        &self,
        raw_email: &[u8],
        recipient: &str,
        sender: &str,
        client_ip: IpAddr,
    ) -> Result<(), AppError> {
        info!("Processing incoming email for recipient: {} from {}", recipient, sender);
        
        // Check greylisting if enabled
        if self.enable_greylisting {
            let key = (client_ip, sender.to_string(), recipient.to_string());
            let now = chrono::Utc::now().timestamp();
            
            if let Some(first_seen) = self.greylist.get(&key) {
                if now - *first_seen < self.greylist_delay.as_secs() as i64 {
                    return Err(AppError::Mail("Greylisted, try again later".to_string()));
                }
            } else {
                self.greylist.insert(key, now);
                return Err(AppError::Mail("Greylisted, try again later".to_string()));
            }
        }

        // Parse email for validation and extraction
        let _parsed_email = Message::parse(raw_email)
            .ok_or_else(|| AppError::Mail("Failed to parse email".to_string()))?;

        // Validate SPF if enabled
        if self.enable_spf {
            let spf_result = self.check_spf(sender, client_ip).await?;
            if !spf_result {
                return Err(AppError::Mail("SPF validation failed".to_string()));
            }
        }

        // Validate DKIM if enabled
        if self.enable_dkim {
            let dkim_result = self.verify_dkim(raw_email).await?;
            if !dkim_result {
                return Err(AppError::Mail("DKIM validation failed".to_string()));
            }
        }
        
        let mailbox = self.db.get_mailbox_by_address(recipient)
            .await?
            .ok_or_else(|| AppError::Mail(format!("Mailbox not found: {}", recipient)))?;

        // Encrypt email content using age encryption
        let encrypted_content = encrypt_email(raw_email, &mailbox.public_key)?;

        let email = Email {
            id: uuid::Uuid::new_v4().to_string(),
            mailbox_id: mailbox.id.clone(),
            encrypted_content,
            received_at: chrono::Utc::now().timestamp(),
            expires_at: mailbox.expires_at,
        };

        self.db.save_email(&email).await?;
        Ok(())
    }

    async fn check_spf(&self, _sender: &str, _client_ip: IpAddr) -> Result<bool, AppError> {
        // TODO: Implement SPF checking
        warn!("SPF checking is temporarily disabled");
        Ok(true) // Temporarily allow all SPF checks to pass
    }

    async fn verify_dkim(&self, _raw_email: &[u8]) -> Result<bool, AppError> {
        // TODO: Implement DKIM verification
        warn!("DKIM verification is temporarily disabled");
        Ok(true) // Temporarily allow all DKIM checks to pass
    }

    pub fn is_ip_blocked(&self, ip: IpAddr) -> bool {
        self.blocked_networks.iter().any(|net| net.contains(ip))
    }

    pub fn check_rate_limit(&self, _ip: IpAddr) -> bool {
        self.rate_limiter.check().is_ok()
    }

    pub async fn cleanup_expired(&self) -> Result<(), AppError> {
        info!("Running cleanup for expired mailboxes and emails");
        
        self.db.cleanup_expired_emails().await?;
        self.db.cleanup_expired_mailboxes().await?;

        Ok(())
    }

    pub async fn get_mailbox_emails(&self, mailbox_id: &str) -> Result<Vec<Email>, AppError> {
        self.db.get_mailbox_emails(mailbox_id).await
    }

    pub async fn start_cleanup_task(self: Arc<Self>, interval: Duration) {
        let service = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval);
            loop {
                interval.tick().await;
                if let Err(e) = service.cleanup_expired().await {
                    error!("Cleanup task error: {}", e);
                }
                
                // Cleanup old greylist entries
                let now = chrono::Utc::now().timestamp();
                service.greylist.retain(|_, first_seen| {
                    now - *first_seen < (service.greylist_delay.as_secs() * 2) as i64
                });
            }
        });
    }
} 