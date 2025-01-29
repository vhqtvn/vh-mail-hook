use std::{sync::Arc, net::IpAddr, time::Duration};
use anyhow::Result;
use common::{db::Database, db::SqliteDatabase, Mailbox, Email, User, AuthType, AppError, security::{encrypt_email, decrypt_email}};
use mail_service::{Config, MailService};
use ipnetwork::IpNetwork;
use uuid::Uuid;

const TEST_PUBLIC_KEY: &str = "age1creym8a9ncefdvplrqrfy7wf8k3fw2l7w5z7nwp03jgfyhc56gcqgq27cg";
const TEST_SECRET_KEY: &str = "AGE-SECRET-KEY-10Q6FGH2JQD9VS0ZM50KV7XVC8SAC50MM5DDH9DKWQR3RCSJKYM6QAX66U8";

// Test utilities
async fn setup_test_db() -> Result<Arc<dyn Database>> {
    let db = SqliteDatabase::new("sqlite::memory:").await?;
    db.init().await?;  // Initialize the database schema
    Ok(Arc::new(db))
}

async fn create_test_user(db: &Arc<dyn Database>) -> Result<User> {
    let username = format!("test_user_{}", Uuid::new_v4());
    db.create_user(&username, AuthType::Password).await.map_err(|e| anyhow::anyhow!(e))
}

async fn setup_test_service(enable_greylisting: bool) -> Result<(Arc<MailService>, Arc<dyn Database>)> {
    let db = setup_test_db().await?;
    let blocked_networks = vec![
        "10.0.0.0/8".parse().unwrap(),
    ];
    
    let service = MailService::new(
        db.clone(),
        "test.com".to_string(),
        blocked_networks,
        1024 * 1024, // 1MB max email size
        100, // rate limit
        enable_greylisting, // configurable greylisting
        Duration::from_secs(1), // 1 second greylist delay for testing
        false, // disable SPF for testing
        false, // disable DKIM for testing
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
        address: "test@test.com".to_string(),
        public_key: TEST_PUBLIC_KEY.to_string(),
        owner_id: test_user.id,
        created_at: chrono::Utc::now().timestamp(),
        expires_at: Some(chrono::Utc::now().timestamp() + 3600),
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
        &test_mailbox.address,  // Use the actual mailbox address
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
        address: "test@test.com".to_string(),
        public_key: TEST_PUBLIC_KEY.to_string(),
        owner_id: test_user.id,
        created_at: chrono::Utc::now().timestamp(),
        expires_at: Some(chrono::Utc::now().timestamp() + 3600),
    };
    db.create_mailbox(&test_mailbox).await?;
    
    let test_ip: IpAddr = "192.168.1.1".parse()?;
    let email_content = b"test email content";
    
    // First attempt should be greylisted
    let result = service.process_incoming_email(
        email_content,
        &test_mailbox.address,
        "sender@example.com",
        test_ip
    ).await;
    
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Greylisted"));
    
    // Wait for greylist delay
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Second attempt should succeed
    let result = service.process_incoming_email(
        email_content,
        &test_mailbox.address,
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
    
    // Create test mailbox that expires soon
    let test_mailbox = Mailbox {
        id: Uuid::new_v4().to_string(),
        address: "test@test.com".to_string(),
        public_key: TEST_PUBLIC_KEY.to_string(),
        owner_id: test_user.id,
        created_at: chrono::Utc::now().timestamp(),
        expires_at: Some(chrono::Utc::now().timestamp() + 1),
    };
    
    // Create mailbox using database
    let mailbox_id = test_mailbox.id.clone();
    db.create_mailbox(&test_mailbox).await?;
    
    // Add test email through service
    let email_content = "Test email content";
    service.process_incoming_email(
        email_content.as_bytes(),
        &test_mailbox.address,
        "sender@example.com",
        "192.168.1.1".parse()?,
    ).await?;
    
    // Wait for expiration
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Run cleanup
    service.cleanup_expired().await?;
    
    // Verify cleanup
    let emails = service.get_mailbox_emails(&mailbox_id).await?;
    assert!(emails.is_empty());
    
    Ok(())
} 