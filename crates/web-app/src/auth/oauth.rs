use axum::{
    extract::{Query, State},
    response::Redirect,
};
use common::{AppError, AuthType, User, db::Database};
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use std::sync::Arc;
use serde::Deserialize;
use crate::AppState;

use crate::auth::{create_token, store_credentials};

// OAuth callback parameters
#[derive(Debug, Deserialize)]
pub struct OAuthCallback {
    code: String,
    #[allow(dead_code)]
    state: String,
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

// GitHub OAuth handlers
pub async fn github_login_handler() -> Result<Redirect, AppError> {
    let client = github_oauth_client()?;
    let (auth_url, _csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("read:user".to_string()))
        .url();
    
    Ok(Redirect::to(auth_url.as_str()))
}

pub async fn github_callback_handler<D: Database>(
    State(state): State<Arc<AppState<D>>>,
    Query(params): Query<OAuthCallback>,
) -> Result<Redirect, AppError> {
    let client = github_oauth_client()?;
    
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
    
    // Redirect to frontend with token
    Ok(Redirect::to(&format!("/auth/callback?token={}", token)))
}

// Google OAuth handlers
pub async fn google_login_handler() -> Result<Redirect, AppError> {
    let client = google_oauth_client()?;
    let (auth_url, _csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("https://www.googleapis.com/auth/userinfo.profile".to_string()))
        .add_scope(Scope::new("https://www.googleapis.com/auth/userinfo.email".to_string()))
        .url();
    
    Ok(Redirect::to(auth_url.as_str()))
}

pub async fn google_callback_handler<D: Database>(
    State(state): State<Arc<AppState<D>>>,
    Query(params): Query<OAuthCallback>,
) -> Result<Redirect, AppError> {
    let client = google_oauth_client()?;
    
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
    
    // Create or get user
    let user = create_or_get_oauth_user(&state.db, &google_user.email, "google", &google_user.id).await?;
    
    // Generate JWT token
    let token = create_token(&user.id)?;
    
    // Redirect to frontend with token
    Ok(Redirect::to(&format!("/auth/callback?token={}", token)))
}

// Helper functions

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
        "{}/auth/github/callback",
        std::env::var("APP_URL").unwrap_or_else(|_| "http://localhost:3000".to_string())
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
    let redirect_url = RedirectUrl::new(format!(
        "{}/auth/google/callback",
        std::env::var("APP_URL").unwrap_or_else(|_| "http://localhost:3000".to_string())
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