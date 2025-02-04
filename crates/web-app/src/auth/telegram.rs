use axum::{extract::State, Json};
use common::{AppError, AuthType, User, db::Database};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::{Sha256, Digest};
use std::sync::Arc;
use crate::AppState;
use tracing::{info, error, debug};

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
    info!("Received Telegram auth request: {:?}", auth_data);
    
    // Verify the authentication data
    if !verify_telegram_auth(&auth_data)? {
        error!("Telegram auth verification failed");
        return Err(AppError::Auth("Invalid Telegram authentication".to_string()));
    }
    
    debug!("Telegram auth verification successful");
    
    // Check if the auth_date is not too old (e.g., within last hour)
    let now = chrono::Utc::now().timestamp();
    if now - auth_data.auth_date > 3600 {
        error!("Telegram auth expired: auth_date={}, now={}", auth_data.auth_date, now);
        return Err(AppError::Auth("Telegram authentication expired".to_string()));
    }
    
    // Check if user exists by Telegram ID first
    debug!("Looking up user with Telegram ID: {}", auth_data.id);
    let existing_user = sqlx::query_as::<_, User>(
        "SELECT u.* FROM users u
         JOIN user_credentials c ON u.id = c.user_id
         WHERE c.telegram_id = ?",
    )
    .bind(auth_data.id.to_string())
    .fetch_optional(state.db.pool())
    .await
    .map_err(|e| {
        error!("Database error while looking up user: {}", e);
        AppError::Internal("An error occurred during authentication. Please try again.".to_string())
    })?;
    
    let user = if let Some(user) = existing_user {
        debug!("Found existing user: {}", user.id);
        user
    } else {
        // Generate a unique username
        let base_username = auth_data
            .username
            .unwrap_or_else(|| format!("telegram_{}", auth_data.id));
        
        let mut username = base_username.clone();
        let mut counter = 1;
        
        // Try to create user with incremental usernames until we succeed
        loop {
            match state.db.create_user(&username, AuthType::Telegram).await {
                Ok(user) => {
                    debug!("Created new user with username: {}", username);
                    
                    // Store credentials
                    if let Err(e) = store_credentials(
                        &state.db,
                        &user.id,
                        None,
                        None,
                        None,
                        Some(&auth_data.id.to_string()),
                    ).await {
                        error!("Failed to store credentials: {}", e);
                        return Err(AppError::Internal(
                            "Failed to complete account setup. Please try again.".to_string()
                        ));
                    }
                    
                    break user;
                }
                Err(e) => {
                    if e.to_string().contains("UNIQUE constraint failed") {
                        // Try next username
                        username = format!("{}_{}", base_username, counter);
                        counter += 1;
                        if counter > 100 {
                            error!("Failed to generate unique username after 100 attempts");
                            return Err(AppError::Internal(
                                "Unable to create account. Please try again later.".to_string()
                            ));
                        }
                    } else {
                        error!("Database error while creating user: {}", e);
                        return Err(AppError::Internal(
                            "Unable to create account. Please try again later.".to_string()
                        ));
                    }
                }
            }
        }
    };
    
    // Generate JWT token
    let token = create_token(&user.id)?;
    info!("Successfully authenticated Telegram user: {}", user.id);
    
    Ok(Json(AuthResponse { token, user }))
}

fn verify_telegram_auth(auth_data: &TelegramAuth) -> Result<bool, AppError> {
    debug!("Verifying Telegram auth data");
    
    let bot_token = std::env::var("TELEGRAM_BOT_TOKEN")
        .map_err(|_| {
            error!("TELEGRAM_BOT_TOKEN not set");
            AppError::Internal("TELEGRAM_BOT_TOKEN not set".to_string())
        })?;
    
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
    debug!("Data check string: {}", data_check_string);
    
    // Generate secret key - use SHA256 of bot token
    let secret = Sha256::digest(bot_token.as_bytes());
    
    // Calculate HMAC-SHA256
    let mut mac = Hmac::<Sha256>::new_from_slice(&secret)
        .map_err(|e| {
            error!("Failed to create HMAC: {}", e);
            AppError::Internal(format!("Failed to create HMAC: {}", e))
        })?;
    
    mac.update(data_check_string.as_bytes());
    let result = mac.finalize();
    let calculated_hash = hex::encode(result.into_bytes());
    
    debug!("Hash comparison: calculated={}, received={}", calculated_hash, auth_data.hash);
    Ok(calculated_hash == auth_data.hash)
} 