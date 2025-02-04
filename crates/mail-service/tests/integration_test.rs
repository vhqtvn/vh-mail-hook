use std::{sync::Arc, net::IpAddr, time::Duration};
use anyhow::Result;
use common::{db::{Database, SqliteDatabase}, Mailbox, User, AuthType, security::decrypt_email};
use mail_service::{MailService, ServiceConfig};
use mail_service::dns::MockDnsResolver;
use uuid::Uuid;

// Test constants
const TEST_PUBLIC_KEY: &str = "age1f7s2nyhnfvvc4jkpt4hmk8zxunkkn98tzh586ajndwpsx86xs5vsqkjqvf";
const TEST_SECRET_KEY: &str = "AGE-SECRET-KEY-1Q05RKVD23NKTSKEFMDN4ATCWMVG4WY8DR97YWC7CS2JMK2FDAVPSF5YJ38";

// Test utilities
async fn setup_test_db() -> Result<Arc<dyn Database>> {
    let db = SqliteDatabase::new("sqlite::memory:").await?;
    db.init().await?;  // Initialize the database schema
    Ok(Arc::new(db))
}

async fn create_test_user(db: &Arc<dyn Database>) -> Result<User> {
    let username = "test_user".to_string();
    let user = db.create_user(&username, AuthType::Password).await?;
    Ok(user)
}

async fn setup_test_service(enable_greylisting: bool) -> Result<(Arc<MailService>, Arc<dyn Database>)> {
    let db = setup_test_db().await?;
    let blocked_networks = vec![
        "10.0.0.0/8".parse().unwrap(),
    ];
    
    let config = ServiceConfig {
        blocked_networks,
        max_email_size: 1024 * 1024, // 1MB max email size
        rate_limit_per_hour: 100, // rate limit
        enable_greylisting,
        greylist_delay: Duration::from_secs(5), // increased to 5 seconds for more reliable testing
        enable_spf: false, // disable SPF for testing
        enable_dkim: false, // disable DKIM for testing
    };

    // Create a mock resolver with test MX records
    let dns_resolver = Arc::new(MockDnsResolver::new(vec!["test-mx.test.com".to_string()]));
    let service = MailService::new_with_resolver(
        db.clone(),
        config,
        dns_resolver,
    ).await?;
    
    Ok((Arc::new(service), db))
}

#[tokio::test]
async fn test_smtp_basic_flow() -> Result<()> {
    let (service, db) = setup_test_service(false).await?; // Disable greylisting for this test
    
    // Create a test user first
    let test_user = create_test_user(&db).await?;
    
    // Create a test mailbox
    let test_mailbox = Mailbox {
        id: Uuid::new_v4().to_string(),
        alias: "test".to_string(),
        name: "Test Mailbox".to_string(),
        public_key: TEST_PUBLIC_KEY.to_string(),
        owner_id: test_user.id,
        created_at: chrono::Utc::now().timestamp(),
        mail_expires_in: Some(3600), // 1 hour expiration
    };
    
    // Create mailbox using database
    let mailbox_id = test_mailbox.id.clone();
    db.create_mailbox(&test_mailbox).await?;
    
    // Test email sending
    let email_content = "From: sender@example.com\r\n\
                        To: test@test.com\r\n\
                        Subject: Test Email\r\n\
                        \r\n\
                        This is a test email.";
    
    service.process_incoming_email(
        email_content.as_bytes(),
        &test_mailbox.get_address("test.com"),  // Use the helper method to get full address
        "sender@example.com",
        "192.168.1.1".parse()?,
    ).await?;
    
    // Verify email was stored
    let emails = service.get_mailbox_emails(&mailbox_id).await?;
    assert_eq!(emails.len(), 1);
    
    // Verify decrypted content matches original
    let decrypted = decrypt_email(&emails[0].encrypted_content, TEST_SECRET_KEY)?;
    assert_eq!(decrypted, email_content.as_bytes());
    
    Ok(())
}

#[tokio::test]
async fn test_ip_blocking() -> Result<()> {
    let (service, _) = setup_test_service(false).await?;
    
    // Test blocked IP
    let blocked_ip: IpAddr = "10.0.0.1".parse()?;
    assert!(service.is_ip_blocked(blocked_ip));
    
    // Test allowed IP
    let allowed_ip: IpAddr = "192.168.1.1".parse()?;
    assert!(!service.is_ip_blocked(allowed_ip));
    
    Ok(())
}

#[tokio::test]
async fn test_rate_limiting() -> Result<()> {
    let (service, _) = setup_test_service(false).await?;
    let test_ip: IpAddr = "192.168.1.1".parse()?;
    
    // Should allow initial requests
    for _ in 0..10 {
        assert!(service.check_rate_limit(test_ip));
    }
    
    // Send many requests to trigger rate limit
    for _ in 0..200 {
        let _ = service.check_rate_limit(test_ip);
    }
    
    // Should be rate limited now
    assert!(!service.check_rate_limit(test_ip));
    
    Ok(())
}

#[tokio::test]
async fn test_greylisting() -> Result<()> {
    let (service, db) = setup_test_service(true).await?;
    
    // Create a test user and mailbox first
    let test_user = create_test_user(&db).await?;
    let test_mailbox = Mailbox {
        id: Uuid::new_v4().to_string(),
        alias: "test".to_string(),
        name: "Test Mailbox".to_string(),
        public_key: TEST_PUBLIC_KEY.to_string(),
        owner_id: test_user.id,
        created_at: chrono::Utc::now().timestamp(),
        mail_expires_in: Some(3600), // 1 hour expiration
    };
    db.create_mailbox(&test_mailbox).await?;
    
    let test_ip: IpAddr = "192.168.1.1".parse()?;
    let email_content = b"test email content";
    
    // First attempt should be greylisted
    let result = service.process_incoming_email(
        email_content,
        &test_mailbox.get_address("test.com"),
        "sender@example.com",
        test_ip
    ).await;
    
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Greylisted"));
    
    // Wait for greylist delay (wait 7 seconds to be safe, as delay is 5 seconds)
    tokio::time::sleep(Duration::from_secs(7)).await;
    
    // Second attempt should succeed
    let result = service.process_incoming_email(
        email_content,
        &test_mailbox.get_address("test.com"),
        "sender@example.com",
        test_ip
    ).await;
    
    assert!(result.is_ok());
    
    Ok(())
}

#[tokio::test]
async fn test_cleanup() -> Result<()> {
    let (service, db) = setup_test_service(false).await?; // Disable greylisting for this test
    
    // Create test user first
    let test_user = create_test_user(&db).await?;
    
    // Create test mailbox that expires immediately
    let test_mailbox = Mailbox {
        id: Uuid::new_v4().to_string(),
        alias: "test".to_string(),
        name: "Test Mailbox".to_string(),
        public_key: TEST_PUBLIC_KEY.to_string(),
        owner_id: test_user.id,
        created_at: chrono::Utc::now().timestamp(),
        mail_expires_in: Some(1), // 1 second expiration
    };
    
    // Create mailbox using database
    let mailbox_id = test_mailbox.id.clone();
    db.create_mailbox(&test_mailbox).await?;
    
    // Add test email through service
    let email_content = "Test email content";
    service.process_incoming_email(
        email_content.as_bytes(),
        &test_mailbox.get_address("test.com"),
        "sender@example.com",
        "192.168.1.1".parse()?,
    ).await?;
    
    // Wait 3 seconds to ensure expiration (1 second expiry + 2 second buffer)
    tokio::time::sleep(Duration::from_secs(3)).await;
    
    // Run cleanup
    service.cleanup_expired().await?;
    
    // Verify cleanup
    let emails = service.get_mailbox_emails(&mailbox_id).await?;
    assert!(emails.is_empty());
    
    Ok(())
}

#[tokio::test]
async fn test_nonexistent_mailbox() -> Result<()> {
    let (service, _) = setup_test_service(false).await?;
    
    // Try to send email to a non-existent mailbox
    let email_content = "From: sender@example.com\r\n\
                        To: nonexistent@test.com\r\n\
                        Subject: Test Email\r\n\
                        \r\n\
                        This is a test email.";
    
    let result = service.process_incoming_email(
        email_content.as_bytes(),
        "nonexistent@test.com",
        "sender@example.com",
        "192.168.1.1".parse()?,
    ).await;
    
    // Should return an error indicating mailbox not found
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Mailbox not found"));
    
    Ok(())
} 