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
        .route(
            "/api/auth/telegram/verify",
            post(telegram_verify_handler::<D>),
        )
        .nest(
            "/api/auth/me",
            Router::new()
                .route("/", get(me_handler::<D>))
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
        .await?;

    // Hash password and store credentials
    let password_hash = password::hash_password(&req.password)?;
    store_credentials(&state.db, &user.id, Some(&password_hash), None, None, None).await?;

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
    let user = get_user_by_username(&state.db, &req.username).await?;

    // Verify password
    let credentials = get_credentials(&state.db, &user.id).await?;
    if !password::verify_password(
        &req.password,
        credentials.password_hash.as_deref().unwrap_or_default(),
    )? {
        return Ok(Json(ApiResponse::error("Invalid password".to_string())));
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
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    Ok(Json(ApiResponse::success(user)))
}

// Auth middleware
pub async fn auth(req: Request<Body>, next: Next) -> Response {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());

    let auth_header = match auth_header {
        Some(header) => header,
        None => {
            return (StatusCode::UNAUTHORIZED, "Missing authorization header").into_response();
        }
    };

    if !auth_header.starts_with("Bearer ") {
        return (
            StatusCode::UNAUTHORIZED,
            "Invalid authorization header format",
        )
            .into_response();
    }

    let token = auth_header.trim_start_matches("Bearer ");
    let jwt_secret = match std::env::var("JWT_SECRET") {
        Ok(secret) => secret,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "JWT_SECRET not set").into_response();
        }
    };

    let claims = match decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    ) {
        Ok(token_data) => token_data.claims,
        Err(e) => {
            return (StatusCode::UNAUTHORIZED, format!("Invalid token: {}", e)).into_response();
        }
    };

    // Add user_id to request extensions
    let mut req = req;
    req.extensions_mut().insert(claims);

    next.run(req).await
}

// Helper functions

pub(crate) async fn store_credentials<D: Database>(
    db: &D,
    user_id: &str,
    password_hash: Option<&str>,
    oauth_provider: Option<&str>,
    oauth_id: Option<&str>,
    telegram_id: Option<&str>,
) -> Result<(), AppError> {
    let now = chrono::Utc::now().timestamp();

    sqlx::query(
        "INSERT INTO user_credentials (user_id, password_hash, oauth_provider, oauth_id, telegram_id, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(user_id)
    .bind(password_hash)
    .bind(oauth_provider)
    .bind(oauth_id)
    .bind(telegram_id)
    .bind(now)
    .bind(now)
    .execute(db.pool())
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

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
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("User credentials not found".to_string()))
}

pub(crate) async fn get_user_by_username<D: Database>(
    db: &D,
    username: &str,
) -> Result<User, AppError> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ?")
        .bind(username)
        .fetch_optional(db.pool())
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
pub(crate) struct UserCredentials {
    pub user_id: String,
    pub password_hash: Option<String>,
    pub oauth_provider: Option<String>,
    pub oauth_id: Option<String>,
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
