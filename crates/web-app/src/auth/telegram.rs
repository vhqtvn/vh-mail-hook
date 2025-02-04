use axum::{extract::State, Json};
use common::{AppError, AuthType, User, db::Database};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::{Sha256, Digest};
use std::sync::Arc;
use crate::{AppState, ApiResponse};
use tracing::{info, error, debug};
use crate::auth::{create_token, store_credentials, AuthResponse, Claims};

// Telegram login widget data
#[derive(Debug, Deserialize)]
pub struct TelegramAuth {
    pub id: i64,
    pub first_name: String,
    pub username: Option<String>,
    pub photo_url: Option<String>,
    pub auth_date: i64,
    pub hash: String,
    pub action: String, // "login", "register", or "connect"
}

pub async fn telegram_verify_handler<D: Database>(
    State(state): State<Arc<AppState<D>>>,
    claims: Option<axum::extract::Extension<Claims>>,
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

    match (auth_data.action.as_str(), existing_user) {
        // Login attempt
        ("login", Some(user)) => {
            debug!("Found existing user: {}", user.id);
            let token = create_token(&user.id)?;
            info!("Successfully authenticated Telegram user: {}", user.id);
            Ok(Json(AuthResponse { token, user }))
        }
        ("login", None) => {
            error!("Login attempt with unlinked Telegram account");
            Err(AppError::Auth("No account found with this Telegram account. Please register first.".to_string()))
        }

        // Connect attempt (from settings)
        ("connect", existing_user) => {
            let claims = claims.ok_or_else(|| {
                error!("Connect attempt without authentication");
                AppError::Auth("Invalid connect request. Please try again when connecting in settings page".to_string())
            })?;

            // Get the user
            let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
                .bind(&claims.sub)
                .fetch_optional(state.db.pool())
                .await
                .map_err(|e| {
                    error!("Database error while fetching user: {}", e);
                    AppError::Internal("An error occurred during authentication. Please try again.".to_string())
                })?
                .ok_or_else(|| AppError::Auth("User not found. Please log in again.".to_string()))?;

            if existing_user.is_some() {
                error!("Telegram account already linked to a different user");
                return Err(AppError::Auth("This Telegram account is already linked to a different account.".to_string()));
            }

            // Check if user already has Telegram linked
            let existing_telegram = sqlx::query_scalar::<_, Option<String>>(
                "SELECT telegram_id FROM user_credentials WHERE user_id = ?",
            )
            .bind(&user.id)
            .fetch_one(state.db.pool())
            .await
            .map_err(|e| {
                error!("Database error while checking existing Telegram link: {}", e);
                AppError::Internal("An error occurred during authentication. Please try again.".to_string())
            })?;

            if existing_telegram.is_some() {
                error!("User already has a Telegram account linked: {}", user.id);
                return Err(AppError::Auth("This account already has a different Telegram account linked.".to_string()));
            }

            // Link Telegram ID to the existing account
            sqlx::query(
                "UPDATE user_credentials SET telegram_id = ?, updated_at = ? WHERE user_id = ?",
            )
            .bind(auth_data.id.to_string())
            .bind(now)
            .bind(&user.id)
            .execute(state.db.pool())
            .await
            .map_err(|e| {
                error!("Database error while linking Telegram account: {}", e);
                AppError::Internal("Failed to link Telegram account. Please try again.".to_string())
            })?;

            info!("Successfully linked Telegram account for user: {}", user.id);
            let token = create_token(&user.id)?;
            Ok(Json(AuthResponse { token, user }))
        }

        // Registration attempt
        ("register", Some(_)) => {
            error!("Registration attempt with already linked Telegram account");
            Err(AppError::Auth("This Telegram account is already linked to an account. Please log in instead.".to_string()))
        }
        ("register", None) => {
            let base_username = auth_data.username.as_deref().ok_or_else(|| {
                error!("Telegram account has no username");
                AppError::Auth("Your Telegram account must have a username to create an account.".to_string())
            })?;

            // Try to create user with incremental usernames until we succeed
            let mut counter = 0;
            let user = loop {
                let username = if counter == 0 {
                    base_username.to_string()
                } else {
                    format!("{}_{}", base_username, counter)
                };

                match state.db.create_user(&username, AuthType::Telegram).await {
                    Ok(user) => break user,
                    Err(e) => {
                        if e.to_string().contains("UNIQUE constraint failed") || e.to_string().contains("Duplicate entry") {
                            counter += 1;
                            if counter > 100 {
                                error!("Failed to generate unique username after 100 attempts");
                                return Err(AppError::Internal("Unable to create account. Please try again later.".to_string()));
                            }
                            continue;
                        }
                        error!("Database error during user creation: {}", e);
                        return Err(AppError::Internal("Unable to create account. Please try again later.".to_string()));
                    }
                }
            };

            // Store Telegram credentials
            store_credentials(
                &state.db,
                &user.id,
                None,
                None,
                None,
                Some(&auth_data.id.to_string()),
            )
            .await
            .map_err(|e| {
                error!("Failed to store credentials: {}", e);
                AppError::Internal("Failed to complete account setup. Please try again.".to_string())
            })?;

            let token = create_token(&user.id)?;
            info!("Successfully created and authenticated new Telegram user: {}", user.id);
            Ok(Json(AuthResponse { token, user }))
        }

        // Invalid action
        _ => {
            error!("Invalid action specified: {}", auth_data.action);
            Err(AppError::Auth("Invalid authentication action.".to_string()))
        }
    }
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

pub async fn telegram_disconnect_handler<D: Database>(
    State(state): State<Arc<AppState<D>>>,
    claims: axum::extract::Extension<crate::auth::Claims>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    debug!("Processing Telegram disconnect request for user: {}", claims.sub);

    // Check if user has other authentication methods before disconnecting
    let credentials = sqlx::query_as::<_, UserCredentials>(
        "SELECT * FROM user_credentials WHERE user_id = ?",
    )
    .bind(&claims.sub)
    .fetch_one(state.db.pool())
    .await
    .map_err(|e| {
        error!("Database error while fetching credentials: {}", e);
        AppError::Internal("Failed to verify account status. Please try again.".to_string())
    })?;

    // Count available auth methods
    let mut auth_methods = 0;
    if credentials.password_hash.is_some() { auth_methods += 1; }
    if credentials.oauth_provider.is_some() { auth_methods += 1; }
    if credentials.telegram_id.is_some() { auth_methods += 1; }

    if auth_methods <= 1 {
        error!("User attempted to disconnect last auth method: {}", claims.sub);
        return Err(AppError::Auth(
            "Cannot disconnect Telegram: You must have at least one way to log in to your account. Please add another login method first.".to_string()
        ));
    }

    // Remove Telegram ID from credentials
    sqlx::query(
        "UPDATE user_credentials SET telegram_id = NULL, updated_at = ? WHERE user_id = ?",
    )
    .bind(chrono::Utc::now().timestamp())
    .bind(&claims.sub)
    .execute(state.db.pool())
    .await
    .map_err(|e| {
        error!("Database error while disconnecting Telegram: {}", e);
        AppError::Internal("Failed to disconnect Telegram account. Please try again.".to_string())
    })?;

    info!("Successfully disconnected Telegram for user: {}", claims.sub);
    Ok(Json(ApiResponse::success(())))
}

#[derive(sqlx::FromRow)]
struct UserCredentials {
    password_hash: Option<String>,
    oauth_provider: Option<String>,
    telegram_id: Option<String>,
} 