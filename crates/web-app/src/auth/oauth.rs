use axum::{
    extract::{Query, State},
    response::Redirect,
    Json,
};
use common::{AppError, AuthType, User, db::Database};
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use crate::{AppState, ApiResponse, get_web_app_url};
use std::collections::HashMap;
use crate::auth::{create_token, store_credentials, Claims};
use std::borrow::Cow;

// OAuth callback parameters
#[derive(Debug, Deserialize)]
pub struct OAuthCallback {
    code: String,
    #[allow(dead_code)]
    state: String,
    action: Option<String>,
}

// GitHub user info
#[derive(Debug, Deserialize)]
struct GitHubUser {
    id: i64,
    login: String,
}

// Google user info
#[derive(Debug, Deserialize)]
struct GoogleUser {
    id: String,
    email: String,
    verified_email: bool,
}

// Auth response
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: User,
    pub redirect_to: String,
}

// GitHub OAuth handlers
pub async fn github_login_handler(Query(params): Query<HashMap<String, String>>) -> Result<Redirect, AppError> {
    let client = github_oauth_client()?;
    let app_url = get_app_url();
    let redirect_url = format!("{}/auth/github/callback", app_url);

    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("read:user".to_string()))
        .set_redirect_uri(Cow::Owned(RedirectUrl::new(redirect_url).unwrap()))
        .url();
    
    // Store redirect_to in state if provided
    let state = if let Some(redirect_to) = params.get("redirect_to") {
        format!("{}:{}", csrf_token.secret(), redirect_to)
    } else {
        csrf_token.secret().to_string()
    };

    Ok(Redirect::to(&auth_url.to_string().replace(csrf_token.secret(), &state)))
}

pub async fn github_callback_handler<D: Database>(
    State(state): State<Arc<AppState<D>>>,
    Query(params): Query<OAuthCallback>,
) -> Result<Json<AuthResponse>, AppError> {
    let client = github_oauth_client()?;
    
    // Extract redirect_to from state if present
    let state_str = params.state.clone();
    let (_csrf_state, redirect_to) = if let Some(idx) = state_str.find(':') {
        let (csrf, redirect) = state_str.split_at(idx);
        (csrf.to_string(), Some(redirect[1..].to_string()))
    } else {
        (state_str, None)
    };
    
    // Exchange the code for an access token
    let token = client
        .exchange_code(AuthorizationCode::new(params.code))
        .request_async(oauth2::reqwest::async_http_client)
        .await
        .map_err(|e| AppError::Auth(format!("Failed to exchange GitHub code: {}", e)))?;
    
    // Get GitHub user info
    let github_user: GitHubUser = reqwest::Client::new()
        .get("https://api.github.com/user")
        .header(
            "Authorization",
            format!("Bearer {}", token.access_token().secret()),
        )
        .header("User-Agent", "vh-mail-hook")
        .send()
        .await
        .map_err(|e| AppError::Auth(format!("Failed to get GitHub user info: {}", e)))?
        .json()
        .await
        .map_err(|e| AppError::Auth(format!("Failed to parse GitHub user info: {}", e)))?;
    
    // Create or get user
    let user = create_or_get_oauth_user(&state.db, &github_user.login, "github", &github_user.id.to_string()).await?;
    
    // Generate JWT token
    let token = create_token(&user.id)?;
    
    // Determine redirect URL based on action
    let redirect_to = redirect_to.unwrap_or_else(|| "/mailboxes".to_string());
    
    Ok(Json(AuthResponse { token, user, redirect_to }))
}

// Google OAuth handlers
pub async fn google_login_handler(Query(params): Query<HashMap<String, String>>) -> Result<Redirect, AppError> {
    let client = google_oauth_client()?;
    let app_url = get_app_url();
    let redirect_url = format!("{}/auth/google/callback", app_url);

    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("https://www.googleapis.com/auth/userinfo.profile".to_string()))
        .add_scope(Scope::new("https://www.googleapis.com/auth/userinfo.email".to_string()))
        .set_redirect_uri(Cow::Owned(RedirectUrl::new(redirect_url).unwrap()))
        .url();
    
    // Store redirect_to, user_id, and action in state if provided
    let state = if let Some(redirect_to) = params.get("redirect_to") {
        if let Some(user_id) = params.get("state") {
            if let Some(action) = params.get("action") {
                format!("{}:{}:{}:{}", csrf_token.secret(), redirect_to, user_id, action)
            } else {
                format!("{}:{}:{}", csrf_token.secret(), redirect_to, user_id)
            }
        } else if let Some(action) = params.get("action") {
            format!("{}:{}::{}", csrf_token.secret(), redirect_to, action)
        } else {
            format!("{}:{}", csrf_token.secret(), redirect_to)
        }
    } else if let Some(user_id) = params.get("state") {
        if let Some(action) = params.get("action") {
            format!("{}::{}:{}", csrf_token.secret(), user_id, action)
        } else {
            format!("{}::{}", csrf_token.secret(), user_id)
        }
    } else if let Some(action) = params.get("action") {
        format!("{}:::{}", csrf_token.secret(), action)
    } else {
        csrf_token.secret().to_string()
    };

    Ok(Redirect::to(&auth_url.to_string().replace(csrf_token.secret(), &state)))
}

pub async fn google_callback_handler<D: Database>(
    State(state): State<Arc<AppState<D>>>,
    Query(params): Query<OAuthCallback>,
) -> Result<Json<AuthResponse>, AppError> {
    let client = google_oauth_client()?;
    
    // Extract redirect_to, user_id, and action from state if present
    let state_str = params.state.clone();
    let parts: Vec<&str> = state_str.split(':').collect();
    let (_csrf_state, redirect_to, user_id, action) = match parts.len() {
        4 => (parts[0].to_string(), Some(parts[1].to_string()), Some(parts[2].to_string()), Some(parts[3].to_string())),
        3 => (parts[0].to_string(), Some(parts[1].to_string()), Some(parts[2].to_string()), None),
        2 => (parts[0].to_string(), Some(parts[1].to_string()), None, None),
        _ => (state_str, None, None, None),
    };
    
    // Exchange the code for an access token
    let token = client
        .exchange_code(AuthorizationCode::new(params.code))
        .request_async(oauth2::reqwest::async_http_client)
        .await
        .map_err(|e| AppError::Auth(format!("Failed to exchange Google code: {}", e)))?;
    
    // Get Google user info
    let google_user: GoogleUser = reqwest::Client::new()
        .get("https://www.googleapis.com/oauth2/v2/userinfo")
        .header(
            "Authorization",
            format!("Bearer {}", token.access_token().secret()),
        )
        .send()
        .await
        .map_err(|e| AppError::Auth(format!("Failed to get Google user info: {}", e)))?
        .json()
        .await
        .map_err(|e| AppError::Auth(format!("Failed to parse Google user info: {}", e)))?;
    
    if !google_user.verified_email {
        return Err(AppError::Auth("Google email not verified".to_string()));
    }

    // Check if user exists with this Google ID
    let existing_user = sqlx::query_as::<_, User>(
        "SELECT u.* FROM users u
         JOIN user_credentials c ON u.id = c.user_id
         WHERE c.oauth_provider = 'google' AND c.oauth_id = ?",
    )
    .bind(&google_user.id)
    .fetch_optional(state.db.pool())
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Handle different actions
    match action.as_deref().or(params.action.as_deref()) {
        // Connect action - link Google account to existing user
        Some("connect") => {
            let user_id = user_id
                .ok_or_else(|| AppError::Auth("Invalid state for connect action".to_string()))?;

            // Check if this Google account is already connected to another user
            if let Some(existing) = &existing_user {
                if existing.id != user_id {
                    return Err(AppError::Auth("This Google account is already connected to another user".to_string()));
                }
                return Err(AppError::Auth("This Google account is already connected to your account".to_string()));
            }

            // Update the user's credentials
            sqlx::query(
                "UPDATE user_credentials 
                 SET oauth_provider = 'google',
                     oauth_id = ?,
                     updated_at = ?
                 WHERE user_id = ?",
            )
            .bind(&google_user.id)
            .bind(chrono::Utc::now().timestamp())
            .bind(&user_id)
            .execute(state.db.pool())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

            // Return success response
            let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
                .bind(&user_id)
                .fetch_one(state.db.pool())
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;

            let token = create_token(&user.id)?;
            let redirect_to = redirect_to.unwrap_or_else(|| "/settings?success=true".to_string());
            Ok(Json(AuthResponse { token, user, redirect_to }))
        }

        // Login action - check if account exists
        Some("login") => {
            match existing_user {
                Some(user) => {
                    let token = create_token(&user.id)?;
                    let redirect_to = redirect_to.unwrap_or_else(|| "/mailboxes".to_string());
                    Ok(Json(AuthResponse { token, user, redirect_to }))
                }
                None => {
                    Err(AppError::Auth("No account found with this Google account. Please register first.".to_string()))
                }
            }
        }

        // Register action - create new account
        Some("register") => {
            if existing_user.is_some() {
                Err(AppError::Auth("This Google account is already registered. Please login instead.".to_string()))
            } else {
                // Generate unique username from email
                let base_username = google_user.email.split('@').next().unwrap_or(&google_user.email);
                let username = crate::auth::generate_unique_username(&state.db, base_username, AuthType::Google).await?;

                // Create new user
                let user = state.db.create_user(&username, AuthType::Google).await?;

                // Store Google credentials
                store_credentials(
                    &state.db,
                    &user.id,
                    None,
                    Some("google"),
                    Some(&google_user.id),
                    None,
                )
                .await?;

                let token = create_token(&user.id)?;
                let redirect_to = redirect_to.unwrap_or_else(|| "/mailboxes".to_string());
                Ok(Json(AuthResponse { token, user, redirect_to }))
            }
        }

        // Invalid action
        _ => {
            Err(AppError::Auth("Invalid authentication action".to_string()))
        }
    }
}

// Helper function to extract user ID from state
#[allow(dead_code)]
fn extract_user_id_from_state(state: &str) -> Option<String> {
    state.split(':').nth(2).map(|s| s.to_string())
}

// Google disconnect handler
pub async fn google_disconnect_handler<D: Database>(
    State(state): State<Arc<AppState<D>>>,
    claims: axum::extract::Extension<Claims>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    // Check if user has other authentication methods before disconnecting
    let credentials = sqlx::query_as::<_, (Option<String>, Option<String>, Option<String>, Option<String>)>(
        "SELECT password_hash, telegram_id, oauth_provider, oauth_id FROM user_credentials WHERE user_id = ?"
    )
    .bind(&claims.sub)
    .fetch_one(state.db.pool())
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Check if user is authenticated with Google
    let (oauth_provider, _oauth_id) = (credentials.2, credentials.3);
    if oauth_provider.as_deref() != Some("google") {
        return Err(AppError::Auth("No Google account connected".to_string()));
    }

    // Ensure user has at least one other authentication method
    let has_password = credentials.0.is_some();
    let has_telegram = credentials.1.is_some();
    if !has_password && !has_telegram {
        return Err(AppError::Auth(
            "Cannot disconnect Google account: it is your only authentication method".to_string(),
        ));
    }

    // Remove Google credentials
    sqlx::query(
        r#"UPDATE user_credentials 
           SET oauth_provider = NULL, 
               oauth_id = NULL, 
               updated_at = ? 
           WHERE user_id = ? AND oauth_provider = 'google'"#
    )
    .bind(chrono::Utc::now().timestamp())
    .bind(&claims.sub)
    .execute(state.db.pool())
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(ApiResponse::success(())))
}

// Helper functions

fn get_app_url() -> String {
    get_web_app_url()
}

async fn create_or_get_oauth_user<D: Database>(
    db: &D,
    username: &str,
    provider: &str,
    provider_id: &str,
) -> Result<User, AppError> {
    // Check if user exists with this OAuth provider and ID
    let existing_user = sqlx::query_as::<_, User>(
        "SELECT u.* FROM users u
         JOIN user_credentials c ON u.id = c.user_id
         WHERE c.oauth_provider = ? AND c.oauth_id = ?",
    )
    .bind(provider)
    .bind(provider_id)
    .fetch_optional(db.pool())
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;
    
    if let Some(user) = existing_user {
        return Ok(user);
    }
    
    // Create new user
    let user = db.create_user(username, AuthType::GitHub).await?;
    
    // Store OAuth credentials
    store_credentials(db, &user.id, None, Some(provider), Some(provider_id), None).await?;
    
    Ok(user)
}

fn github_oauth_client() -> Result<BasicClient, AppError> {
    let client_id = ClientId::new(
        std::env::var("GITHUB_CLIENT_ID")
            .map_err(|_| AppError::Internal("GITHUB_CLIENT_ID not set".to_string()))?,
    );
    let client_secret = ClientSecret::new(
        std::env::var("GITHUB_CLIENT_SECRET")
            .map_err(|_| AppError::Internal("GITHUB_CLIENT_SECRET not set".to_string()))?,
    );
    let auth_url = AuthUrl::new("https://github.com/login/oauth/authorize".to_string())
        .map_err(|e| AppError::Internal(format!("Invalid GitHub auth URL: {}", e)))?;
    let token_url = TokenUrl::new("https://github.com/login/oauth/access_token".to_string())
        .map_err(|e| AppError::Internal(format!("Invalid GitHub token URL: {}", e)))?;
    let redirect_url = RedirectUrl::new(format!(
        "{}/api/auth/github/callback",
        get_app_url()
    ))
    .map_err(|e| AppError::Internal(format!("Invalid redirect URL: {}", e)))?;
    
    Ok(BasicClient::new(
        client_id,
        Some(client_secret),
        auth_url,
        Some(token_url),
    )
    .set_redirect_uri(redirect_url))
}

fn google_oauth_client() -> Result<BasicClient, AppError> {
    let client_id = ClientId::new(
        std::env::var("GOOGLE_CLIENT_ID")
            .map_err(|_| AppError::Internal("GOOGLE_CLIENT_ID not set".to_string()))?,
    );
    let client_secret = ClientSecret::new(
        std::env::var("GOOGLE_CLIENT_SECRET")
            .map_err(|_| AppError::Internal("GOOGLE_CLIENT_SECRET not set".to_string()))?,
    );
    let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
        .map_err(|e| AppError::Internal(format!("Invalid Google auth URL: {}", e)))?;
    let token_url = TokenUrl::new("https://oauth2.googleapis.com/token".to_string())
        .map_err(|e| AppError::Internal(format!("Invalid Google token URL: {}", e)))?;
    let frontend_url = get_app_url();
    let redirect_url = RedirectUrl::new(format!(
        "{}/auth/google/callback",
        frontend_url
    ))
    .map_err(|e| AppError::Internal(format!("Invalid redirect URL: {}", e)))?;
    
    Ok(BasicClient::new(
        client_id,
        Some(client_secret),
        auth_url,
        Some(token_url),
    )
    .set_redirect_uri(redirect_url))
} 