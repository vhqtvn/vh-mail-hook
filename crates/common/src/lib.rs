use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use thiserror::Error;
use axum::middleware::Next;
use axum::http::Request;
use axum::body::Body;

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

pub async fn handle_json_response(
    req: Request<Body>,
    next: Next,
) -> Response {
    // Get the Accept header before processing
    let wants_json = req
        .headers()
        .get("Accept")
        .and_then(|h| h.to_str().ok())
        .map(|h| h.contains("application/json"))
        .unwrap_or(false);

    // Process the request
    let res = next.run(req).await;

    // If not an error or doesn't want JSON, return as is
    if res.status().is_success() || !wants_json {
        return res;
    }

    // Convert error response to JSON
    let status = res.status();
    
    // Create JSON error response
    let error_response = serde_json::json!({
        "success": false,
        "error": status.to_string(),
        "data": null
    });
    
    (
        status,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        axum::Json(error_response)
    ).into_response()
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

        // Create JSON error response
        let error_response = serde_json::json!({
            "success": false,
            "error": message,
            "data": null
        });
        
        (
            status,
            [(axum::http::header::CONTENT_TYPE, "application/json")],
            axum::Json(error_response)
        ).into_response()
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
        chunk.iter_mut().take(CHUNK_SIZE).for_each(|digit| {
            *digit = ID_CHARSET[(tmp % BASE) as usize];
            tmp /= BASE;
        });

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
    pub name: String,
    pub public_key: String,
    pub owner_id: String,
    pub mail_expires_in: Option<i64>,
    pub created_at: i64,
}

impl Mailbox {
    pub fn new(owner_id: &str, _domain: &str, mail_expires_in: Option<i64>) -> Self {
        let id = generate_random_id(12); // Use 12 characters for the ID
        let alias = generate_random_id(12); // Use 12 characters for the alias
        Self {
            id,
            alias,
            name: String::new(),
            public_key: String::new(),
            owner_id: owner_id.to_string(),
            mail_expires_in,
            created_at: chrono::Utc::now().timestamp(),
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
    Google,
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
