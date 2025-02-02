use axum::{
    routing::Router,
    response::Response,
    http::{Request, StatusCode},
    body::Body,
};
use common::{db::Database, db::SqliteDatabase, Mailbox, User, Email};
use serde_json::json;
use std::{sync::Arc, env, path::PathBuf};
use tower::Service;
use web_app::{create_app, ApiResponse};
use http_body_util::BodyExt;
use tracing::{info, error};

const TEST_PUBLIC_KEY: &str = "age1creym8a9ncefdvplrqrfy7wf8k3fw2l7w5z7nwp03jgfyhc56gcqgq27cg";
#[allow(dead_code)]
const TEST_SECRET_KEY: &str = "AGE-SECRET-KEY-10Q6FGH2JQD9VS0ZM50KV7XVC8SAC50MM5DDH9DKWQR3RCSJKYM6QAX66U8";
const TEST_USERNAME: &str = "test-user";
const TEST_PASSWORD: &str = "test-password";

async fn setup_test_app() -> Router {
    info!("Setting up test database");
    
    // Set up the path to migrations
    let workspace_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let migrations_path = workspace_dir.join("common").join("migrations");
    
    info!("Using migrations from: {}", migrations_path.display());
    env::set_var("SQLX_MIGRATIONS_DIR", migrations_path);
    env::set_var("JWT_SECRET", "test-secret-key");
    
    let db = match SqliteDatabase::new_in_memory().await {
        Ok(db) => Arc::new(db),
        Err(e) => {
            error!("Failed to create database: {}", e);
            panic!("Failed to create database: {}", e);
        }
    };

    // Initialize the database schema
    if let Err(e) = db.init().await {
        error!("Failed to initialize database schema: {}", e);
        panic!("Failed to initialize database schema: {}", e);
    }

    info!("Database setup complete");
    create_app(
        db,
        "test.example.com".to_string(),
        "http://localhost:3000".to_string()
    )
}

// Helper function to read response body
async fn read_body<T>(response: axum::response::Response) -> T 
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

// Helper function to create a test user and get auth token
async fn create_test_user_with_auth<S>(app: &mut S) -> (String, String) 
where
    S: Service<Request<Body>, Response = Response> + Send,
    S::Future: Send,
    S::Error: std::fmt::Debug,
{
    // Register user with password
    let register_response = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(json!({
                    "username": TEST_USERNAME,
                    "password": TEST_PASSWORD
                }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(register_response.status(), StatusCode::OK);
    let auth_response: ApiResponse<AuthResponse> = read_body(register_response).await;
    assert!(auth_response.success);
    let auth_data = auth_response.data.unwrap();
    
    (auth_data.user.id, auth_data.token)
}

#[tokio::test]
async fn test_create_mailbox() {
    setup();
    let app = setup_test_app().await;
    let mut app_service = app.into_service();

    // Create a test user with auth
    let (owner_id, token) = create_test_user_with_auth(&mut app_service).await;

    let request_body = json!({
        "expires_in_days": 7,
        "public_key": TEST_PUBLIC_KEY
    });

    info!("Sending request with body: {}", request_body);

    let response = app_service
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/mailboxes")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK, "Expected OK status code");

    let response: ApiResponse<Mailbox> = read_body(response).await;
    assert!(response.success, "Expected successful response");
    assert!(response.data.is_some(), "Expected response data to be present");
    let mailbox = response.data.unwrap();
    assert_eq!(mailbox.owner_id, owner_id, "Owner ID mismatch");
    assert!(mailbox.address.ends_with("@test.example.com"), "Invalid email address format");
}

#[tokio::test]
async fn test_get_mailbox() {
    setup();
    let app = setup_test_app().await;
    let mut app_service = app.into_service();

    // Create a test user with auth
    let (owner_id, token) = create_test_user_with_auth(&mut app_service).await;

    // First create a mailbox
    let create_response = app_service
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/mailboxes")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::from(
                    json!({
                        "expires_in_days": 7,
                        "public_key": TEST_PUBLIC_KEY
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let create_result: ApiResponse<Mailbox> = read_body(create_response).await;
    let mailbox = create_result.data.unwrap();

    // Then get the mailbox
    let get_response = app_service
        .call(
            Request::builder()
                .method("GET")
                .uri(format!("/api/mailboxes/{}", mailbox.id))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_response.status(), StatusCode::OK);

    let get_result: ApiResponse<Mailbox> = read_body(get_response).await;
    assert!(get_result.success);
    assert!(get_result.data.is_some());
    let retrieved_mailbox = get_result.data.unwrap();
    assert_eq!(retrieved_mailbox.id, mailbox.id);
    assert_eq!(retrieved_mailbox.owner_id, owner_id);
}

#[tokio::test]
async fn test_update_mailbox() {
    setup();
    let app = setup_test_app().await;
    let mut app_service = app.into_service();

    // Create a test user with auth
    let (owner_id, token) = create_test_user_with_auth(&mut app_service).await;

    // First create a mailbox
    let create_response = app_service
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/mailboxes")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::from(
                    json!({
                        "expires_in_days": 7,
                        "public_key": TEST_PUBLIC_KEY
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let create_result: ApiResponse<Mailbox> = read_body(create_response).await;
    let mailbox = create_result.data.unwrap();

    // Update the mailbox
    let update_response = app_service
        .call(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/mailboxes/{}", mailbox.id))
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::from(
                    json!({
                        "expires_in_days": 14
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(update_response.status(), StatusCode::OK);

    // Verify the update
    let get_response = app_service
        .call(
            Request::builder()
                .method("GET")
                .uri(format!("/api/mailboxes/{}", mailbox.id))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let get_result: ApiResponse<Mailbox> = read_body(get_response).await;
    assert!(get_result.success);
    let updated_mailbox = get_result.data.unwrap();
    assert_eq!(updated_mailbox.id, mailbox.id);
    assert_eq!(updated_mailbox.owner_id, owner_id);
}

#[tokio::test]
async fn test_delete_mailbox() {
    setup();
    let app = setup_test_app().await;
    let mut app_service = app.into_service();

    // Create a test user with auth
    let (_owner_id, token) = create_test_user_with_auth(&mut app_service).await;

    // First create a mailbox
    let create_response = app_service
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/mailboxes")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::from(
                    json!({
                        "expires_in_days": 7,
                        "public_key": TEST_PUBLIC_KEY
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let create_result: ApiResponse<Mailbox> = read_body(create_response).await;
    let mailbox = create_result.data.unwrap();

    // Delete the mailbox
    let delete_response = app_service
        .call(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/mailboxes/{}", mailbox.id))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(delete_response.status(), StatusCode::OK);

    // Verify mailbox is deleted
    let get_response = app_service
        .call(
            Request::builder()
                .method("GET")
                .uri(format!("/api/mailboxes/{}", mailbox.id))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let get_result: ApiResponse<Mailbox> = read_body(get_response).await;
    assert!(!get_result.success);
    assert!(get_result.error.is_some());
}

#[tokio::test]
async fn test_get_mailbox_emails() {
    setup();
    let app = setup_test_app().await;
    let mut app_service = app.into_service();

    // Create a test user with auth
    let (_owner_id, token) = create_test_user_with_auth(&mut app_service).await;

    // First create a mailbox
    let create_response = app_service
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/mailboxes")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::from(
                    json!({
                        "expires_in_days": 7,
                        "public_key": TEST_PUBLIC_KEY
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let create_result: ApiResponse<Mailbox> = read_body(create_response).await;
    let mailbox = create_result.data.unwrap();

    // Get emails (should be empty)
    let get_emails_response = app_service
        .call(
            Request::builder()
                .method("GET")
                .uri(format!("/api/mailboxes/{}/emails", mailbox.id))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_emails_response.status(), StatusCode::OK);

    let emails_response: ApiResponse<Vec<Email>> = read_body(get_emails_response).await;
    assert!(emails_response.success);
    let emails = emails_response.data.unwrap();
    assert!(emails.is_empty());
}

#[tokio::test]
async fn test_login() {
    setup();
    let app = setup_test_app().await;
    let mut app_service = app.into_service();

    // First register a user
    let register_response = app_service
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(json!({
                    "username": TEST_USERNAME,
                    "password": TEST_PASSWORD
                }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(register_response.status(), StatusCode::OK);

    // Then try to login
    let login_response = app_service
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(json!({
                    "username": TEST_USERNAME,
                    "password": TEST_PASSWORD
                }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(login_response.status(), StatusCode::OK);
    let auth_response: ApiResponse<AuthResponse> = read_body(login_response).await;
    assert!(auth_response.success);
    let auth_data = auth_response.data.unwrap();
    assert_eq!(auth_data.user.username, TEST_USERNAME);
}

#[tokio::test]
async fn test_auth_check() {
    setup();
    let app = setup_test_app().await;
    let mut app_service = app.into_service();

    // Create a test user with auth
    let (_, token) = create_test_user_with_auth(&mut app_service).await;

    // Check authentication with the token
    let auth_check_response = app_service
        .call(
            Request::builder()
                .method("GET")
                .uri("/api/auth/me")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(auth_check_response.status(), StatusCode::OK);
    let auth_check: ApiResponse<User> = read_body(auth_check_response).await;
    assert!(auth_check.success);
    let user = auth_check.data.unwrap();
    assert_eq!(user.username, TEST_USERNAME);

    // Check authentication without token should fail
    let auth_check_no_token = app_service
        .call(
            Request::builder()
                .method("GET")
                .uri("/api/auth/me")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(auth_check_no_token.status(), StatusCode::UNAUTHORIZED);
} 