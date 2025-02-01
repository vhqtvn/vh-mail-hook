use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use thiserror::Error;
use uuid::Uuid;
use axum::response::{IntoResponse, Response};
use axum::http::StatusCode;

pub mod db;
pub mod security;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Authentication error: {0}")]
    Auth(String),
    #[error("Database error: {0}")]
    Database(String),
    #[error("Mail processing error: {0}")]
    Mail(String),
    #[error("Internal error: {0}")]
    Internal(String),
    #[error("Not found: {0}")]
    NotFound(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::Auth(msg) => (StatusCode::UNAUTHORIZED, msg),
            AppError::Database(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::Mail(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
        };
        
        (status, message).into_response()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Mailbox {
    pub id: String,
    pub address: String,
    pub public_key: String,
    pub owner_id: String,
    pub expires_at: Option<i64>,
    pub created_at: i64,
}

impl Mailbox {
    pub fn new(owner_id: &str, domain: &str, expires_at: Option<i64>) -> Self {
        let id = Uuid::new_v4().to_string();
        let address = format!("{}@{}", Uuid::new_v4(), domain);
        let now = chrono::Utc::now();

        Self {
            id,
            address,
            public_key: "dummy_key".to_string(), // TODO: Implement proper key generation
            owner_id: owner_id.to_string(),
            expires_at,
            created_at: now.timestamp(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Email {
    pub id: String,
    pub mailbox_id: String,
    pub encrypted_content: String,
    pub received_at: i64,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct User {
    pub id: String,
    pub username: String,
    pub auth_type: AuthType,
    pub created_at: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[derive(sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub enum AuthType {
    Password,
    GitHub,
    Telegram,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApiKey {
    pub id: String,
    pub user_id: String,
    pub key: String,
    pub created_at: i64,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserSettings {
    pub user_id: String,
    pub email_notifications: bool,
    pub auto_delete_expired: bool,
    pub default_mailbox_expiry: Option<i64>,
} 