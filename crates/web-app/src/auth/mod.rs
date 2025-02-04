use crate::{ApiResponse, AppState};
use axum::{
    body::Body,
    extract::{Json, State},
    http::{header, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use common::{db::Database, AppError, AuthType, User};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::error;

mod oauth;
mod password;
mod telegram;

pub use oauth::*;
pub use telegram::*;

// JWT Claims structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String, // user_id
    pub exp: usize,  // expiration time
    pub iat: usize,  // issued at
}

// Registration request
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

// Login request
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

// Auth response
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: User,
}

#[derive(Debug, Serialize)]
pub struct ConnectedAccount {
    provider: String,
    connected_at: i64,
    provider_id: Option<String>,
}

// Delete account request
#[derive(Debug, Deserialize)]
pub struct DeleteAccountRequest {
    pub password: Option<String>,
}

// Set password request
#[derive(Debug, Deserialize)]
pub struct SetPasswordRequest {
    pub new_password: String,
}

// Create auth routes
pub fn create_routes<D: Database + 'static>() -> Router<Arc<AppState<D>>> {
    Router::new()
        .route("/api/auth/register", post(register_handler::<D>))
        .route("/api/auth/login", post(login_handler::<D>))
        .route("/api/auth/github/login", get(github_login_handler))
        .route(
            "/api/auth/github/callback",
            get(github_callback_handler::<D>),
        )
        .route("/api/auth/google/login", get(google_login_handler))
        .route(
            "/api/auth/google/callback",
            get(google_callback_handler::<D>),
        )
        .nest(
            "/api/auth",
            Router::new()
                .route("/telegram/verify", post(telegram_verify_handler::<D>))
                .layer(middleware::from_fn(auth_optional)),
        )
        .nest(
            "/api/auth",
            Router::new()
                .route("/me", get(me_handler::<D>))
                .route("/connected-accounts", get(connected_accounts_handler::<D>))
                .route("/delete-account", post(delete_account_handler::<D>))
                .route("/set-password", post(set_password_handler::<D>))
                .route("/telegram/disconnect", post(telegram_disconnect_handler::<D>))
                .route("/google/disconnect", post(google_disconnect_handler::<D>))
                .route("/github/disconnect", post(github_disconnect_handler::<D>))
                .layer(middleware::from_fn(auth)),
        )
}

// Register handler
async fn register_handler<D: Database>(
    State(state): State<Arc<AppState<D>>>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<ApiResponse<AuthResponse>>, AppError> {
    // Create user with password auth type
    let user = state
        .db
        .create_user(&req.username, AuthType::Password)
        .await
        .map_err(|e| {
            tracing::error!("Database error during user creation: {}", e);
            if e.to_string().contains("Duplicate entry") || e.to_string().contains("UNIQUE constraint failed") {
                AppError::Auth("Username is already taken. Please choose a different username.".to_string())
            } else {
                AppError::Auth("Unable to create account. Please try again later or contact support if the problem persists.".to_string())
            }
        })?;

    // Hash password and store credentials
    let password_hash = password::hash_password(&req.password)?;
    store_credentials(&state.db, &user.id, Some(&password_hash), None, None, None).await
        .map_err(|e| {
            tracing::error!("Database error during credential storage: {}", e);
            AppError::Auth("Account created but unable to set up credentials. Please try logging in, or contact support if you cannot access your account.".to_string())
        })?;

    // Generate JWT token
    let token = create_token(&user.id)?;

    Ok(Json(ApiResponse::success(AuthResponse { token, user })))
}

// Login handler
async fn login_handler<D: Database>(
    State(state): State<Arc<AppState<D>>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<ApiResponse<AuthResponse>>, AppError> {
    // Get user by username
    let user = get_user_by_username(&state.db, &req.username).await
        .map_err(|e| {
            if matches!(e, AppError::NotFound(_)) {
                AppError::Auth("The username or password you entered is incorrect. Please check your credentials and try again.".to_string())
            } else {
                tracing::error!("Database error during login: {}", e);
                AppError::Auth("Unable to process login request. Please try again later or contact support if the problem persists.".to_string())
            }
        })?;

    // Verify password
    let credentials = get_credentials(&state.db, &user.id).await
        .map_err(|e| {
            tracing::error!("Database error while fetching credentials: {}", e);
            AppError::Auth("Unable to verify credentials. Please try again later or contact support if the problem persists.".to_string())
        })?;
    
    let password_hash = credentials.password_hash.as_deref().unwrap_or_default();
    if password_hash.is_empty() {
        return Err(AppError::Auth("No password has been set for this account. Please use another login method or reset your password.".to_string()));
    }

    if !password::verify_password(&req.password, password_hash)? {
        return Err(AppError::Auth("The username or password you entered is incorrect. Please check your credentials and try again.".to_string()));
    }

    // Generate JWT token
    let token = create_token(&user.id)?;

    Ok(Json(ApiResponse::success(AuthResponse { token, user })))
}

// Me handler to check authentication status
async fn me_handler<D: Database>(
    State(state): State<Arc<AppState<D>>>,
    claims: axum::extract::Extension<Claims>,
) -> Result<Json<ApiResponse<User>>, AppError> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(&claims.sub)
        .fetch_optional(state.db.pool())
        .await
        .map_err(|e| {
            tracing::error!("Database error while fetching user: {}", e);
            AppError::Internal("Unable to fetch user profile. Please try refreshing the page or logging in again.".to_string())
        })?
        .ok_or_else(|| AppError::Auth("Your session has expired. Please log in again to continue.".to_string()))?;

    Ok(Json(ApiResponse::success(user)))
}

fn extract_claims(req: &Request<Body>) -> Result<Option<Claims>, AppError> {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());
    
    match auth_header {
        Some(header) => {
            if let Some(token) = header.strip_prefix("Bearer ") {
                let claims = decode::<Claims>(
                    token,
                    &DecodingKey::from_secret(get_jwt_secret().as_bytes()),
                    &Validation::default(),
                ).map_err(|_| AppError::Auth("Invalid token".to_string()))?;
                Ok(Some(claims.claims))
            } else {
                Err(AppError::Auth("Invalid authorization header format".to_string()))
            }
        }
        None => Ok(None),
    }
}

pub async fn auth_optional(req: Request<Body>, next: Next) -> Response {
    match extract_claims(&req) {
        Ok(Some(claims)) => {
            let mut req = req;
            req.extensions_mut().insert(claims);
            next.run(req).await
        }
        Ok(None) => next.run(req).await,
        Err(e) => {
            error!("Failed to extract claims: {}", e);
            (StatusCode::UNAUTHORIZED, "Unauthorized").into_response()
        }
    }
}

// Auth middleware
pub async fn auth(req: Request<Body>, next: Next) -> Response {
    match extract_claims(&req) {
        Ok(Some(claims)) => {
            let mut req = req;
            req.extensions_mut().insert(claims);
            next.run(req).await
        }
        _ => {
            (StatusCode::UNAUTHORIZED, "Unauthorized").into_response()
        }
    }
}

// Helper functions

pub(crate) async fn store_credentials<D: Database>(
    db: &D,
    user_id: &str,
    password_hash: Option<&str>,
    provider: Option<&str>,
    provider_id: Option<&str>,
    telegram_id: Option<&str>,
) -> Result<(), AppError> {
    let now = chrono::Utc::now().timestamp();

    let mut query = String::from(
        "INSERT INTO user_credentials (user_id, password_hash, created_at, updated_at"
    );
    let mut values = String::from("VALUES (?, ?, ?, ?");
    let mut params: Vec<String> = vec![
        user_id.to_string(),
        password_hash.unwrap_or_default().to_string(),
        now.to_string(),
        now.to_string(),
    ];

    if let (Some(provider), Some(id)) = (provider, provider_id) {
        match provider {
            "google" => {
                query.push_str(", google_id");
                values.push_str(", ?");
                params.push(id.to_string());
            }
            "github" => {
                query.push_str(", github_id");
                values.push_str(", ?");
                params.push(id.to_string());
            }
            _ => {}
        }
    }

    if let Some(id) = telegram_id {
        query.push_str(", telegram_id");
        values.push_str(", ?");
        params.push(id.to_string());
    }

    query.push_str(") ");
    values.push(')');
    query.push_str(&values);

    let mut db_query = sqlx::query(&query);
    for param in params {
        db_query = db_query.bind(param);
    }

    db_query
        .execute(db.pool())
        .await
        .map_err(|e| {
            tracing::error!("Database error while storing credentials: {}", e);
            AppError::Internal("Unable to set up user credentials. Please try logging in again, or contact support if the issue persists.".to_string())
        })?;

    Ok(())
}

pub(crate) async fn get_credentials<D: Database>(
    db: &D,
    user_id: &str,
) -> Result<UserCredentials, AppError> {
    sqlx::query_as::<_, UserCredentials>("SELECT * FROM user_credentials WHERE user_id = ?")
        .bind(user_id)
        .fetch_optional(db.pool())
        .await
        .map_err(|e| {
            tracing::error!("Database error while fetching credentials: {}", e);
            AppError::Internal("Unable to verify user credentials. Please try logging in again.".to_string())
        })?
        .ok_or_else(|| AppError::Auth("Account credentials not found. Please try logging in with a different method or contact support.".to_string()))
}

pub(crate) async fn get_user_by_username<D: Database>(
    db: &D,
    username: &str,
) -> Result<User, AppError> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ?")
        .bind(username)
        .fetch_optional(db.pool())
        .await
        .map_err(|e| {
            tracing::error!("Database error while fetching user by username: {}", e);
            AppError::Internal("Unable to verify user account. Please try again later.".to_string())
        })?
        .ok_or_else(|| AppError::NotFound("Account not found. Please check your username or register a new account.".to_string()))
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
pub(crate) struct UserCredentials {
    pub user_id: String,
    pub password_hash: Option<String>,
    pub google_id: Option<String>,
    pub github_id: Option<String>,
    pub telegram_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

fn create_token(user_id: &str) -> Result<String, AppError> {
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        exp: now + 24 * 3600, // 24 hours from now
        iat: now,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(get_jwt_secret().as_bytes()),
    )
    .map_err(|e| AppError::Internal(format!("Failed to create token: {}", e)))
}

fn get_jwt_secret() -> String {
    std::env::var("JWT_SECRET").unwrap_or_else(|_| "your-256-bit-secret".to_string())
}

// Connected accounts handler
async fn connected_accounts_handler<D: Database>(
    State(state): State<Arc<AppState<D>>>,
    claims: axum::extract::Extension<Claims>,
) -> Result<Json<ApiResponse<Vec<ConnectedAccount>>>, AppError> {
    let credentials = sqlx::query_as::<_, UserCredentials>(
        "SELECT * FROM user_credentials WHERE user_id = ?"
    )
    .bind(&claims.sub)
    .fetch_one(state.db.pool())
    .await
    .map_err(|e| {
        error!("Database error while fetching credentials: {}", e);
        AppError::Internal("Unable to fetch account information".to_string())
    })?;

    let mut accounts = Vec::new();

    // Add password if set
    if let Some(hash) = &credentials.password_hash {
        if !hash.is_empty() {
            accounts.push(ConnectedAccount {
                provider: "password".to_string(),
                connected_at: credentials.created_at,
                provider_id: None,
            });
        }
    }

    // Add Google if present
    if let Some(google_id) = credentials.google_id {
        accounts.push(ConnectedAccount {
            provider: "google".to_string(),
            connected_at: credentials.created_at,
            provider_id: Some(google_id),
        });
    }

    // Add GitHub if present
    if let Some(github_id) = credentials.github_id {
        accounts.push(ConnectedAccount {
            provider: "github".to_string(),
            connected_at: credentials.created_at,
            provider_id: Some(github_id),
        });
    }

    // Add Telegram if present
    if let Some(telegram_id) = credentials.telegram_id {
        accounts.push(ConnectedAccount {
            provider: "telegram".to_string(),
            connected_at: credentials.created_at,
            provider_id: Some(telegram_id),
        });
    }

    Ok(Json(ApiResponse::success(accounts)))
}

// Set password handler
async fn set_password_handler<D: Database>(
    State(state): State<Arc<AppState<D>>>,
    claims: axum::extract::Extension<Claims>,
    Json(req): Json<SetPasswordRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let credentials = get_credentials(&state.db, &claims.sub).await?;
    
    if credentials.password_hash.is_some() {
        return Err(AppError::Auth("Password is already set. Use change password instead.".to_string()));
    }

    let password_hash = password::hash_password(&req.new_password)?;
    
    sqlx::query(
        "UPDATE user_credentials SET password_hash = ?, updated_at = ? WHERE user_id = ?",
    )
    .bind(&password_hash)
    .bind(chrono::Utc::now().timestamp())
    .bind(&claims.sub)
    .execute(state.db.pool())
    .await
    .map_err(|e| {
        tracing::error!("Database error while setting password: {}", e);
        AppError::Internal("Failed to set password. Please try again later.".to_string())
    })?;

    Ok(Json(ApiResponse::success(())))
}

// Delete account handler
async fn delete_account_handler<D: Database>(
    State(state): State<Arc<AppState<D>>>,
    claims: axum::extract::Extension<Claims>,
    Json(req): Json<DeleteAccountRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    // Get user credentials to verify password if provided
    let credentials = get_credentials(&state.db, &claims.sub).await
        .map_err(|e| {
            tracing::error!("Database error while fetching credentials: {}", e);
            AppError::Auth("Unable to verify credentials. Please try again later.".to_string())
        })?;

    // If user has password auth and password was provided, verify it
    if let (Some(ref password_hash), Some(ref password)) = (&credentials.password_hash, &req.password) {
        if !password::verify_password(password, password_hash)? {
            return Err(AppError::Auth("Incorrect password. Please try again.".to_string()));
        }
    } else if credentials.password_hash.is_some() && req.password.is_none() {
        // If user has password but didn't provide one
        return Err(AppError::Auth("Password is required to delete account.".to_string()));
    }

    // Delete the user - this will cascade to all related tables
    sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(&claims.sub)
        .execute(state.db.pool())
        .await
        .map_err(|e| {
            tracing::error!("Database error while deleting user: {}", e);
            AppError::Internal("Failed to delete account. Please try again later.".to_string())
        })?;

    Ok(Json(ApiResponse::success(())))
}

// Helper function to generate a unique username
pub(crate) async fn generate_unique_username<D: Database>(
    db: &D,
    base_username: &str,
    auth_type: AuthType,
) -> Result<String, AppError> {
    let mut counter = 0;
    loop {
        let username = if counter == 0 {
            base_username.to_string()
        } else {
            format!("{}_{}", base_username, counter)
        };

        match db.create_user(&username, auth_type.clone()).await {
            Ok(user) => {
                // Delete the temporary user since we only wanted to check username availability
                sqlx::query("DELETE FROM users WHERE id = ?")
                    .bind(&user.id)
                    .execute(db.pool())
                    .await
                    .map_err(|e| {
                        tracing::error!("Database error while cleaning up temporary user: {}", e);
                        AppError::Internal("Error during username generation".to_string())
                    })?;
                return Ok(username);
            }
            Err(e) => {
                if e.to_string().contains("UNIQUE constraint failed") || e.to_string().contains("Duplicate entry") {
                    counter += 1;
                    if counter > 100 {
                        tracing::error!("Failed to generate unique username after 100 attempts");
                        return Err(AppError::Internal("Unable to generate unique username. Please try again later.".to_string()));
                    }
                    continue;
                }
                tracing::error!("Database error during username check: {}", e);
                return Err(AppError::Internal("Unable to generate unique username. Please try again later.".to_string()));
            }
        }
    }
}

pub async fn github_disconnect_handler<D: Database>(
    State(state): State<Arc<AppState<D>>>,
    claims: axum::extract::Extension<Claims>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    // Check if user has other authentication methods before disconnecting
    let credentials = sqlx::query_as::<_, UserCredentials>(
        "SELECT * FROM user_credentials WHERE user_id = ?"
    )
    .bind(&claims.sub)
    .fetch_one(state.db.pool())
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Check if user is authenticated with GitHub
    if credentials.github_id.is_none() {
        return Err(AppError::Auth("No GitHub account connected".to_string()));
    }

    // Ensure user has at least one other authentication method
    let has_password = credentials.password_hash.is_some();
    let has_google = credentials.google_id.is_some();
    let has_telegram = credentials.telegram_id.is_some();
    if !has_password && !has_google && !has_telegram {
        return Err(AppError::Auth(
            "Cannot disconnect GitHub account: it is your only authentication method".to_string(),
        ));
    }

    // Remove GitHub credentials
    sqlx::query(
        r#"UPDATE user_credentials 
           SET github_id = NULL, 
               updated_at = ? 
           WHERE user_id = ?"#
    )
    .bind(chrono::Utc::now().timestamp())
    .bind(&claims.sub)
    .execute(state.db.pool())
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(ApiResponse::success(())))
}

pub async fn google_disconnect_handler<D: Database>(
    State(state): State<Arc<AppState<D>>>,
    claims: axum::extract::Extension<Claims>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    // Check if user has other authentication methods before disconnecting
    let credentials = sqlx::query_as::<_, UserCredentials>(
        "SELECT * FROM user_credentials WHERE user_id = ?"
    )
    .bind(&claims.sub)
    .fetch_one(state.db.pool())
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Check if user is authenticated with Google
    if credentials.google_id.is_none() {
        return Err(AppError::Auth("No Google account connected".to_string()));
    }

    // Ensure user has at least one other authentication method
    let has_password = credentials.password_hash.is_some();
    let has_github = credentials.github_id.is_some();
    let has_telegram = credentials.telegram_id.is_some();
    if !has_password && !has_github && !has_telegram {
        return Err(AppError::Auth(
            "Cannot disconnect Google account: it is your only authentication method".to_string(),
        ));
    }

    // Remove Google credentials
    sqlx::query(
        r#"UPDATE user_credentials 
           SET google_id = NULL, 
               updated_at = ? 
           WHERE user_id = ?"#
    )
    .bind(chrono::Utc::now().timestamp())
    .bind(&claims.sub)
    .execute(state.db.pool())
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(ApiResponse::success(())))
}
