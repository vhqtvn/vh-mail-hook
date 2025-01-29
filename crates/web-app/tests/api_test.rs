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
    create_app(db, "test.example.com".to_string())
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

async fn create_test_user<S>(app: &mut S) -> String 
where
    S: Service<Request<Body>, Response = Response> + Send,
    S::Future: Send,
    S::Error: std::fmt::Debug,
{
    let request_body = json!({
        "username": "test-user",
        "auth_type": "Password"
    });

    let response = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/users")
                .header("Content-Type", "application/json")
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK, "Expected OK status code");

    let response: ApiResponse<User> = read_body(response).await;
    assert!(response.success, "Expected successful response");
    assert!(response.data.is_some(), "Expected response data to be present");
    
    response.data.unwrap().id
}

#[tokio::test]
async fn test_create_mailbox() {
    setup();
    let mut app = setup_test_app().await.into_service();

    // Create a test user first
    let owner_id = create_test_user(&mut app).await;

    let request_body = json!({
        "owner_id": owner_id,
        "expires_in_days": 7
    });

    info!("Sending request with body: {}", request_body);

    let response = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/mailboxes")
                .header("Content-Type", "application/json")
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
    let mut app = setup_test_app().await.into_service();

    // Create a test user first
    let owner_id = create_test_user(&mut app).await;

    // First create a mailbox
    let create_response = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/mailboxes")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "owner_id": owner_id,
                        "expires_in_days": 7
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
    let get_response = app
        .call(
            Request::builder()
                .method("GET")
                .uri(format!("/api/mailboxes/{}", mailbox.id))
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
    let mut app = setup_test_app().await.into_service();

    // Create a test user first
    let owner_id = create_test_user(&mut app).await;

    // First create a mailbox
    let create_response = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/mailboxes")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "owner_id": owner_id,
                        "expires_in_days": 7
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
    let update_response = app
        .call(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/mailboxes/{}", mailbox.id))
                .header("Content-Type", "application/json")
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

    let update_result: ApiResponse<Mailbox> = read_body(update_response).await;
    assert!(update_result.success);
    assert!(update_result.data.is_some());
    let updated_mailbox = update_result.data.unwrap();
    assert_eq!(updated_mailbox.id, mailbox.id);
    assert!(updated_mailbox.expires_at.unwrap() > mailbox.expires_at.unwrap());
}

#[tokio::test]
async fn test_delete_mailbox() {
    setup();
    let mut app = setup_test_app().await.into_service();

    // Create a test user first
    let owner_id = create_test_user(&mut app).await;

    // First create a mailbox
    let create_response = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/mailboxes")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "owner_id": owner_id,
                        "expires_in_days": 7
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
    let delete_response = app
        .call(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/mailboxes/{}", mailbox.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(delete_response.status(), StatusCode::OK);

    let delete_result: ApiResponse<()> = read_body(delete_response).await;
    assert!(delete_result.success);

    // Verify mailbox is deleted
    let get_response = app
        .call(
            Request::builder()
                .method("GET")
                .uri(format!("/api/mailboxes/{}", mailbox.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let get_result: ApiResponse<Mailbox> = read_body(get_response).await;
    assert!(!get_result.success);
    assert!(get_result.data.is_none());
}

#[tokio::test]
async fn test_get_mailbox_emails() {
    setup();
    let mut app = setup_test_app().await.into_service();

    // Create a test user first
    let owner_id = create_test_user(&mut app).await;

    // First create a mailbox
    let create_response = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/mailboxes")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "owner_id": owner_id,
                        "expires_in_days": 7
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let create_result: ApiResponse<Mailbox> = read_body(create_response).await;
    let mailbox = create_result.data.unwrap();

    // Get emails (should be empty initially)
    let get_response = app
        .call(
            Request::builder()
                .method("GET")
                .uri(format!("/api/mailboxes/{}/emails", mailbox.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_response.status(), StatusCode::OK);

    let get_result: ApiResponse<Vec<Email>> = read_body(get_response).await;
    assert!(get_result.success);
    assert!(get_result.data.is_some());
    let emails = get_result.data.unwrap();
    assert!(emails.is_empty());
} 