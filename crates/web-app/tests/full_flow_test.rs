use axum::{
    response::Response,
    http::{Request, StatusCode},
    body::Body,
};
use common::{
    db::Database, 
    db::SqliteDatabase, 
    Mailbox, 
    User, 
    Email,
    security::decrypt_email,
    AuthType,
};
use mail_service::{
    MailService, 
    ServiceConfig,
};
use serde_json::json;
use std::{sync::Arc, net::IpAddr, time::Duration, path::PathBuf, env};
use tower::ServiceExt;
use web_app::{create_app, ApiResponse};
use http_body_util::BodyExt;
use tracing::{info, error};

const TEST_PUBLIC_KEY: &str = "age1creym8a9ncefdvplrqrfy7wf8k3fw2l7w5z7nwp03jgfyhc56gcqgq27cg";
const TEST_SECRET_KEY: &str = "AGE-SECRET-KEY-10Q6FGH2JQD9VS0ZM50KV7XVC8SAC50MM5DDH9DKWQR3RCSJKYM6QAX66U8";
const TEST_USERNAME: &str = "test-user";
const TEST_PASSWORD: &str = "test-password";

async fn read_body<T>(response: Response) -> T 
where
    T: serde::de::DeserializeOwned,
{
    let status = response.status();
    let body = response.into_body();
    let bytes = BodyExt::collect(body)
        .await
        .unwrap()
        .to_bytes();
    
    let body_str = String::from_utf8_lossy(&bytes);
    info!("Response status: {}, body: {}", status, body_str);
    
    if !status.is_success() {
        panic!("Request failed with status {}: {}", status, body_str);
    }
    
    match serde_json::from_slice::<T>(&bytes) {
        Ok(value) => value,
        Err(e) => {
            error!("Failed to parse response body: {}. Body was: {}", e, body_str);
            panic!("Failed to parse response body: {}. Body was: {}", e, body_str);
        }
    }
}

fn setup() {
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .with_env_filter("debug")
        .try_init();
}

// Auth response type for tests
#[derive(serde::Deserialize)]
struct AuthResponse {
    token: String,
    user: User,
}

#[tokio::test]
async fn test_complete_flow() -> anyhow::Result<()> {
    setup();
    
    info!("Setting up test environment...");
    
    // Set up shared database
    let db = SqliteDatabase::new_in_memory().await?;
    db.init().await?;
    let db = Arc::new(db);
    
    info!("Database initialized");
    
    // Set JWT secret for auth
    std::env::set_var("JWT_SECRET", "test-secret-key");
    
    // Set up migrations path
    let workspace_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let migrations_path = workspace_dir.join("common").join("migrations");
    env::set_var("SQLX_MIGRATIONS_DIR", migrations_path);
    
    info!("Creating web app...");
    
    // Set up web app
    let app = create_app(
        db.clone(),
        "http://localhost:3000".to_string()
    );
    
    info!("Registering test user...");
    
    // Register user with password
    let register_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(json!({
                    "username": TEST_USERNAME,
                    "password": TEST_PASSWORD,
                    "auth_type": AuthType::Password
                }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let auth_response: ApiResponse<AuthResponse> = read_body(register_response).await;
    let auth_data = auth_response.data.unwrap();
    let user = auth_data.user;
    let token = auth_data.token;
    
    info!("User registered successfully with id: {}", user.id);

    // Create a mailbox with test public key
    let create_mailbox_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/mailboxes")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::from(
                    json!({
                        "name": "Test Mailbox",
                        "expires_in_seconds": 7 * 24 * 60 * 60,
                        "public_key": TEST_PUBLIC_KEY
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let mailbox_response: ApiResponse<Mailbox> = read_body(create_mailbox_response).await;
    let mailbox = mailbox_response.data.unwrap();
    assert_eq!(mailbox.public_key, TEST_PUBLIC_KEY);
    
    // Set up mail service with the same database
    let config = ServiceConfig {
        blocked_networks: vec![],
        max_email_size: 1024 * 1024,
        rate_limit_per_hour: 100,
        enable_greylisting: false,
        greylist_delay: Duration::from_secs(1),
        enable_spf: false,
        enable_dkim: false,
    };

    let service = MailService::with_mock_resolver(
        db.clone(),
        config,
        vec!["test-mx.test.example.com".to_string()],
    ).await?;
    
    // Send a test email
    let email_content = "From: sender@example.com\r\n\
                        To: test@test.com\r\n\
                        Subject: Test Email\r\n\
                        \r\n\
                        This is a test email.";
    
    service.process_incoming_email(
        email_content.as_bytes(),
        &mailbox.get_address("test.example.com"),
        "sender@example.com",
        "192.168.1.1".parse::<IpAddr>()?,
    ).await?;
    
    // Get emails through API
    let get_emails_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/mailboxes/{}/emails", mailbox.id))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let emails_response: ApiResponse<Vec<Email>> = read_body(get_emails_response).await;
    let emails = emails_response.data.unwrap();
    assert_eq!(emails.len(), 1);
    
    // Decrypt the email
    let decrypted = decrypt_email(&emails[0].encrypted_content, TEST_SECRET_KEY)?;
    assert_eq!(decrypted, email_content.as_bytes());
    println!("Decrypted email: {:?}", decrypted);
    
    // Delete the email
    let delete_email_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/mailboxes/{}/emails/{}", mailbox.id, emails[0].id))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(delete_email_response.status(), StatusCode::OK);
    
    // Verify email is deleted
    let get_emails_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/mailboxes/{}/emails", mailbox.id))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let emails_response: ApiResponse<Vec<Email>> = read_body(get_emails_response).await;
    let emails = emails_response.data.unwrap();
    assert!(emails.is_empty());
    
    Ok(())
} 