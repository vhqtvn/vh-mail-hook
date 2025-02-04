use crate::auth::{create_token, store_credentials};
use crate::{get_web_app_url, AppState};
use axum::{
    extract::{Query, State},
    response::Redirect,
    Json,
};
use common::{db::Database, AppError, AuthType, User};
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl,
    Scope, TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

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
pub async fn github_login_handler(
    Query(params): Query<HashMap<String, String>>,
) -> Result<Redirect, AppError> {
    let client = github_oauth_client()?;
    let app_url = get_app_url();
    let redirect_url = format!("{}/auth/github/callback", app_url);

    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("read:user".to_string()))
        .set_redirect_uri(Cow::Owned(RedirectUrl::new(redirect_url).unwrap()))
        .url();

    // Store redirect_to, user_id, and action in state if provided
    let state = if let Some(redirect_to) = params.get("redirect_to") {
        if let Some(user_id) = params.get("state") {
            if let Some(action) = params.get("action") {
                format!(
                    "{}:{}:{}:{}",
                    csrf_token.secret(),
                    redirect_to,
                    user_id,
                    action
                )
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

    Ok(Redirect::to(
        &auth_url.to_string().replace(csrf_token.secret(), &state),
    ))
}

pub async fn github_callback_handler<D: Database>(
    State(state): State<Arc<AppState<D>>>,
    Query(params): Query<OAuthCallback>,
) -> Result<Json<AuthResponse>, AppError> {
    // Extract redirect_to, user_id, and action from state if present
    let state_str = params.state.clone();
    let parts: Vec<&str> = state_str.split(':').collect();
    let (_csrf_state, redirect_to, user_id, action) = match parts.len() {
        4 => (
            parts[0].to_string(),
            Some(parts[1].to_string()),
            Some(parts[2].to_string()),
            Some(parts[3].to_string()),
        ),
        3 => (
            parts[0].to_string(),
            Some(parts[1].to_string()),
            Some(parts[2].to_string()),
            None,
        ),
        2 => (parts[0].to_string(), Some(parts[1].to_string()), None, None),
        _ => (state_str, None, None, None),
    };

    // Exchange the code for an access token with custom headers
    let token_response = reqwest::Client::new()
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .header("User-Agent", "vh-mail-hook")
        .form(&[
            (
                "client_id",
                std::env::var("GITHUB_CLIENT_ID")
                    .map_err(|_| AppError::Internal("GITHUB_CLIENT_ID not set".to_string()))?,
            ),
            (
                "client_secret",
                std::env::var("GITHUB_CLIENT_SECRET")
                    .map_err(|_| AppError::Internal("GITHUB_CLIENT_SECRET not set".to_string()))?,
            ),
            ("code", params.code.clone()),
            (
                "redirect_uri",
                format!("{}/auth/github/callback", get_app_url()),
            ),
        ])
        .send()
        .await
        .map_err(|e| AppError::Auth(format!("Failed to exchange GitHub code: {}", e)))?;

    let body = token_response
        .text()
        .await
        .map_err(|e| AppError::Auth(format!("Failed to get token response text: {}", e)))?;

    // Parse the token response
    #[derive(Deserialize)]
    struct GitHubTokenResponse {
        access_token: String,
        #[allow(dead_code)]
        token_type: String,
    }

    let token_data: GitHubTokenResponse = serde_json::from_str(&body)
        .map_err(|e| AppError::Auth(format!("Failed to parse GitHub token response: {}", e)))?;

    // Get GitHub user info
    let response = reqwest::Client::new()
        .get("https://api.github.com/user")
        .header(
            "Authorization",
            format!("Bearer {}", token_data.access_token),
        )
        .header("User-Agent", "vh-mail-hook")
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| AppError::Auth(format!("Failed to get GitHub user info: {}", e)))?;

    let text = response
        .text()
        .await
        .map_err(|e| AppError::Auth(format!("Failed to get response text: {}", e)))?;

    // Parse the response text back into JSON
    let github_user: GitHubUser = serde_json::from_str(&text)
        .map_err(|e| AppError::Auth(format!("Failed to parse GitHub user info: {}", e)))?;

    // Check if user exists with this GitHub ID
    let existing_user = sqlx::query_as::<_, User>(
        "SELECT u.* FROM users u
         JOIN user_credentials c ON u.id = c.user_id
         WHERE c.github_id = ?",
    )
    .bind(github_user.id.to_string())
    .fetch_optional(state.db.pool())
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Handle different actions
    match action.as_deref().or(params.action.as_deref()) {
        // Connect action - link GitHub account to existing user
        Some("connect") => {
            let user_id = user_id
                .ok_or_else(|| AppError::Auth("Invalid state for connect action".to_string()))?;

            // Check if this GitHub account is already connected to another user
            if let Some(existing) = &existing_user {
                if existing.id != user_id {
                    return Err(AppError::Auth(
                        "This GitHub account is already connected to another user".to_string(),
                    ));
                }
                return Err(AppError::Auth(
                    "This GitHub account is already connected to your account".to_string(),
                ));
            }

            // Update the user's credentials while preserving other OAuth connections
            sqlx::query(
                "UPDATE user_credentials 
                 SET github_id = ?,
                     updated_at = ?
                 WHERE user_id = ?",
            )
            .bind(github_user.id.to_string())
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
            Ok(Json(AuthResponse {
                token,
                user,
                redirect_to,
            }))
        }

        // Login action - check if account exists
        Some("login") => match existing_user {
            Some(user) => {
                let token = create_token(&user.id)?;
                let redirect_to = redirect_to.unwrap_or_else(|| "/mailboxes".to_string());
                Ok(Json(AuthResponse {
                    token,
                    user,
                    redirect_to,
                }))
            }
            None => Err(AppError::Auth(
                "No account found with this GitHub account. Please register first.".to_string(),
            )),
        },

        // Register action - create new account
        Some("register") => {
            if existing_user.is_some() {
                Err(AppError::Auth(
                    "This GitHub account is already registered. Please login instead.".to_string(),
                ))
            } else {
                // Generate unique username from GitHub login
                let username = crate::auth::generate_unique_username(
                    &state.db,
                    &github_user.login,
                    AuthType::GitHub,
                )
                .await?;

                // Create new user
                let user = state.db.create_user(&username, AuthType::GitHub).await?;

                // Store GitHub credentials
                store_credentials(
                    &state.db,
                    &user.id,
                    None,
                    Some("github"),
                    Some(&github_user.id.to_string()),
                    None,
                )
                .await?;

                let token = create_token(&user.id)?;
                let redirect_to = redirect_to.unwrap_or_else(|| "/mailboxes".to_string());
                Ok(Json(AuthResponse {
                    token,
                    user,
                    redirect_to,
                }))
            }
        }

        // Invalid action
        _ => Err(AppError::Auth("Invalid authentication action".to_string())),
    }
}

// Google OAuth handlers
pub async fn google_login_handler(
    Query(params): Query<HashMap<String, String>>,
) -> Result<Redirect, AppError> {
    let client = google_oauth_client()?;
    let app_url = get_app_url();
    let redirect_url = format!("{}/auth/google/callback", app_url);

    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new(
            "https://www.googleapis.com/auth/userinfo.profile".to_string(),
        ))
        .add_scope(Scope::new(
            "https://www.googleapis.com/auth/userinfo.email".to_string(),
        ))
        .set_redirect_uri(Cow::Owned(RedirectUrl::new(redirect_url).unwrap()))
        .url();

    // Store redirect_to, user_id, and action in state if provided
    let state = if let Some(redirect_to) = params.get("redirect_to") {
        if let Some(user_id) = params.get("state") {
            if let Some(action) = params.get("action") {
                format!(
                    "{}:{}:{}:{}",
                    csrf_token.secret(),
                    redirect_to,
                    user_id,
                    action
                )
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

    Ok(Redirect::to(
        &auth_url.to_string().replace(csrf_token.secret(), &state),
    ))
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
        4 => (
            parts[0].to_string(),
            Some(parts[1].to_string()),
            Some(parts[2].to_string()),
            Some(parts[3].to_string()),
        ),
        3 => (
            parts[0].to_string(),
            Some(parts[1].to_string()),
            Some(parts[2].to_string()),
            None,
        ),
        2 => (parts[0].to_string(), Some(parts[1].to_string()), None, None),
        _ => (state_str, None, None, None),
    };

    // Exchange the code for an access token
    let token = client
        .exchange_code(AuthorizationCode::new(params.code))
        .add_extra_param("Accept", "application/json")
        .request_async(oauth2::reqwest::async_http_client)
        .await
        .map_err(|e| AppError::Auth(format!("Failed to exchange GitHub code: {}", e)))?;

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
         WHERE c.google_id = ?",
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
                    return Err(AppError::Auth(
                        "This Google account is already connected to another user".to_string(),
                    ));
                }
                return Err(AppError::Auth(
                    "This Google account is already connected to your account".to_string(),
                ));
            }

            // Update the user's credentials while preserving other OAuth connections
            sqlx::query(
                "UPDATE user_credentials 
                 SET google_id = ?,
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
            Ok(Json(AuthResponse {
                token,
                user,
                redirect_to,
            }))
        }

        // Login action - check if account exists
        Some("login") => match existing_user {
            Some(user) => {
                let token = create_token(&user.id)?;
                let redirect_to = redirect_to.unwrap_or_else(|| "/mailboxes".to_string());
                Ok(Json(AuthResponse {
                    token,
                    user,
                    redirect_to,
                }))
            }
            None => Err(AppError::Auth(
                "No account found with this Google account. Please register first.".to_string(),
            )),
        },

        // Register action - create new account
        Some("register") => {
            if existing_user.is_some() {
                Err(AppError::Auth(
                    "This Google account is already registered. Please login instead.".to_string(),
                ))
            } else {
                // Generate unique username from email
                let base_username = google_user
                    .email
                    .split('@')
                    .next()
                    .unwrap_or(&google_user.email);
                let username = crate::auth::generate_unique_username(
                    &state.db,
                    base_username,
                    AuthType::Google,
                )
                .await?;

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
                Ok(Json(AuthResponse {
                    token,
                    user,
                    redirect_to,
                }))
            }
        }

        // Invalid action
        _ => Err(AppError::Auth("Invalid authentication action".to_string())),
    }
}

// Helper functions

fn get_app_url() -> String {
    get_web_app_url()
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
    let app_url = get_app_url();
    let redirect_url = RedirectUrl::new(format!("{}/auth/github/callback", app_url))
        .map_err(|e| AppError::Internal(format!("Invalid redirect URL: {}", e)))?;

    Ok(
        BasicClient::new(client_id, Some(client_secret), auth_url, Some(token_url))
            .set_redirect_uri(redirect_url),
    )
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
    let redirect_url = RedirectUrl::new(format!("{}/auth/google/callback", frontend_url))
        .map_err(|e| AppError::Internal(format!("Invalid redirect URL: {}", e)))?;

    Ok(
        BasicClient::new(client_id, Some(client_secret), auth_url, Some(token_url))
            .set_redirect_uri(redirect_url),
    )
}
