use axum::{extract::State, Json};
use base64::{engine::general_purpose::URL_SAFE, Engine};
use common::{AppError, AuthType, User, db::Database};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::{Sha256, Digest};
use std::sync::Arc;
use crate::AppState;

use crate::auth::{create_token, store_credentials, AuthResponse};

// Telegram login widget data
#[derive(Debug, Deserialize)]
pub struct TelegramAuth {
    id: i64,
    first_name: String,
    username: Option<String>,
    photo_url: Option<String>,
    auth_date: i64,
    hash: String,
}

pub async fn telegram_verify_handler<D: Database>(
    State(state): State<Arc<AppState<D>>>,
    Json(auth_data): Json<TelegramAuth>,
) -> Result<Json<AuthResponse>, AppError> {
    // Verify the authentication data
    if !verify_telegram_auth(&auth_data)? {
        return Err(AppError::Auth("Invalid Telegram authentication".to_string()));
    }
    
    // Check if the auth_date is not too old (e.g., within last hour)
    let now = chrono::Utc::now().timestamp();
    if now - auth_data.auth_date > 3600 {
        return Err(AppError::Auth("Telegram authentication expired".to_string()));
    }
    
    // Get or create user
    let username = auth_data
        .username
        .unwrap_or_else(|| format!("telegram_{}", auth_data.id));
    
    // Check if user exists
    let existing_user = sqlx::query_as::<_, User>(
        "SELECT u.* FROM users u
         JOIN user_credentials c ON u.id = c.user_id
         WHERE c.telegram_id = ?",
    )
    .bind(auth_data.id.to_string())
    .fetch_optional(state.db.pool())
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;
    
    let user = if let Some(user) = existing_user {
        user
    } else {
        // Create new user
        let user = state.db.create_user(&username, AuthType::Telegram).await?;
        
        // Store credentials
        store_credentials(
            &state.db,
            &user.id,
            None,
            None,
            None,
            Some(&auth_data.id.to_string()),
        )
        .await?;
        
        user
    };
    
    // Generate JWT token
    let token = create_token(&user.id)?;
    
    Ok(Json(AuthResponse { token, user }))
}

fn verify_telegram_auth(auth_data: &TelegramAuth) -> Result<bool, AppError> {
    let bot_token = std::env::var("TELEGRAM_BOT_TOKEN")
        .map_err(|_| AppError::Internal("TELEGRAM_BOT_TOKEN not set".to_string()))?;
    
    // Create a sorted string of all fields except hash
    let mut fields = vec![
        format!("auth_date={}", auth_data.auth_date),
        format!("first_name={}", auth_data.first_name),
        format!("id={}", auth_data.id),
    ];
    
    if let Some(ref username) = auth_data.username {
        fields.push(format!("username={}", username));
    }
    if let Some(ref photo_url) = auth_data.photo_url {
        fields.push(format!("photo_url={}", photo_url));
    }
    
    fields.sort();
    let data_check_string = fields.join("\n");
    
    // Generate secret key
    let secret_key = Hmac::<Sha256>::new_from_slice(
        &Sha256::digest(bot_token.as_bytes()),
    )
    .map_err(|e| AppError::Internal(format!("Failed to create HMAC: {}", e)))?;
    
    // Calculate hash
    let mut mac = secret_key;
    mac.update(data_check_string.as_bytes());
    let result = mac.finalize();
    let calculated_hash = URL_SAFE.encode(result.into_bytes());
    
    Ok(calculated_hash == auth_data.hash)
} 