use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use thiserror::Error;

pub mod db;
pub mod security;

// 24 characters chosen to be visually distinct
const ID_CHARSET: &[u8] = b"3479acdefhjkmnpqrstuvwxy";

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

pub fn generate_random_id(len: usize) -> String {
    const BASE: u128 = 24;
    const CHUNK_SIZE: usize = 13;
    const MAX_CHUNK_VALUE: u128 = BASE.pow(CHUNK_SIZE as u32);
    let mut result = String::with_capacity(len);

    while result.len() < len {
        // Pull 64 bits; if >= 24^13, discard and retry
        let val = loop {
            let r = OsRng.next_u64() as u128;
            if r < MAX_CHUNK_VALUE {
                break r;
            }
        };

        // Convert this chunk into 13 base-24 digits
        let mut tmp = val;
        let mut chunk = [0u8; CHUNK_SIZE];
        for i in 0..CHUNK_SIZE {
            chunk[i] = ID_CHARSET[(tmp % BASE) as usize];
            tmp /= BASE;
        }

        // Append them in most-significant digit first (reverse order)
        for &c in chunk.iter().rev() {
            if result.len() == len {
                break;
            }
            result.push(c as char);
        }
    }

    result
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Mailbox {
    pub id: String,
    pub alias: String,
    pub public_key: String,
    pub owner_id: String,
    pub expires_at: Option<i64>,
    pub created_at: i64,
}

impl Mailbox {
    pub fn new(owner_id: &str, _domain: &str, expires_at: Option<i64>) -> Self {
        let id = generate_random_id(12); // Use 12 characters for the ID
        let alias = generate_random_id(12);
        let now = chrono::Utc::now();

        Self {
            id,
            alias,
            public_key: "dummy_key".to_string(), // TODO: Implement proper key generation
            owner_id: owner_id.to_string(),
            expires_at,
            created_at: now.timestamp(),
        }
    }

    pub fn get_address(&self, domain: &str) -> String {
        format!("{}@{}", self.alias, domain)
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

#[derive(Debug, Serialize, Deserialize, Clone, sqlx::Type)]
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
