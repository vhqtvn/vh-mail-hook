use crate::security::encryption::encrypt_email;
use crate::dns::{DnsResolver, TrustDnsResolver};
#[cfg(any(test, feature = "test"))]
use crate::dns::MockDnsResolver;
use anyhow::Result;
use common::{db::Database, AppError, Email};
use dashmap::DashMap;
use governor::{
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use ipnetwork::IpNetwork;
use mail_parser::Message;
use std::{net::IpAddr, sync::Arc, time::Duration};
use tracing::{error, info, warn};

#[derive(Clone)]
pub struct ServiceConfig {
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
    blocked_networks: Vec<IpNetwork>,
    max_email_size: usize,
    rate_limiter: Arc<RateLimiter<NotKeyed, InMemoryState, governor::clock::DefaultClock>>,
    greylist: Arc<DashMap<(IpAddr, String, String), i64>>, // (IP, from, to) -> first_seen
    enable_greylisting: bool,
    greylist_delay: Duration,
    enable_spf: bool,
    enable_dkim: bool,
    dns_resolver: Arc<dyn DnsResolver>,
}

impl MailService {
    pub async fn new(db: Arc<dyn Database>, config: ServiceConfig) -> Result<Self> {
        let rate_limiter = Arc::new(RateLimiter::direct(Quota::per_hour(
            std::num::NonZeroU32::new(config.rate_limit_per_hour).unwrap(),
        )));

        let dns_resolver = Arc::new(TrustDnsResolver::new().await?);

        Ok(Self {
            db,
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

    pub async fn new_with_resolver(
        db: Arc<dyn Database>,
        config: ServiceConfig,
        dns_resolver: Arc<dyn DnsResolver>,
    ) -> Result<Self> {
        let rate_limiter = Arc::new(RateLimiter::direct(Quota::per_hour(
            std::num::NonZeroU32::new(config.rate_limit_per_hour).unwrap(),
        )));

        Ok(Self {
            db,
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

    #[cfg(any(test, feature = "test"))]
    pub async fn with_mock_resolver(db: Arc<dyn Database>, config: ServiceConfig, mx_records: Vec<String>) -> Result<Self> {
        let rate_limiter = Arc::new(RateLimiter::direct(Quota::per_hour(
            std::num::NonZeroU32::new(config.rate_limit_per_hour).unwrap(),
        )));

        let dns_resolver = Arc::new(MockDnsResolver::new(mx_records));

        Ok(Self {
            db,
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
        info!(
            "Processing incoming email for recipient: {} from {}",
            recipient, sender
        );

        // Extract local_part and domain from recipient
        let (local_part, domain) = recipient.split_once('@')
            .ok_or_else(|| AppError::Mail("Invalid recipient address format".to_string()))?;

        // Check if domain has valid MX records
        let mx_records = self.dns_resolver.mx_lookup(domain).await?;
        if mx_records.is_empty() {
            return Err(AppError::Mail("No MX records found for domain".to_string()));
        }

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
            // the removal is done here to avoid deadlock with if let
            // Remove from greylist after successful delay period
            self.greylist.remove(&key);
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

        let mailbox = self
            .db
            .get_mailbox_by_address(local_part)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_resolver() {
        let mock_records = vec!["test-mx.example.com".to_string()];
        let resolver = MockDnsResolver::new(mock_records.clone());
        let result = resolver.mx_lookup("example.com").await.unwrap();
        assert_eq!(result, mock_records);
    }
}
