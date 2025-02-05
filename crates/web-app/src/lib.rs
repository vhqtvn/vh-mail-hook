use axum::{
    extract::{Json, Path, State}, http::{HeaderValue, StatusCode, header}, middleware, routing::{delete, get, patch, post}, Router,
    response::{IntoResponse, Response},
};
use common::{db::Database, handle_json_response, AppError, Email, Mailbox};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, net::SocketAddr};
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tracing::{info, error};
use clap::Parser;
use tokio::net::TcpListener;
use rust_embed::RustEmbed;
use std::sync::OnceLock;
use sqlx::Row;

mod auth;
mod api_spec;
use auth::Claims;

mod api_auth {
    use axum::{
        async_trait,
        extract::FromRequestParts,
        http::{request::Parts, StatusCode},
        response::{IntoResponse, Response},
    };
    use serde::Serialize;
    use crate::{AppState, Database};
    use std::sync::Arc;

    #[derive(Debug, Serialize)]
    pub struct ApiClaims {
        pub user_id: String,
    }

    #[async_trait]
    impl<D> FromRequestParts<Arc<AppState<D>>> for ApiClaims
    where
        D: Database + Send + Sync + 'static,
    {
        type Rejection = Response;

        async fn from_request_parts(
            parts: &mut Parts,
            state: &Arc<AppState<D>>,
        ) -> Result<Self, Self::Rejection> {
            // Get the Authorization header
            let auth_header = parts
                .headers
                .get("Authorization")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.strip_prefix("Bearer "))
                .ok_or_else(|| {
                    (StatusCode::UNAUTHORIZED, "Missing or invalid Authorization header").into_response()
                })?;

            // Query the database to find the user associated with this API key
            let user_id: Option<String> = sqlx::query_scalar(
                "SELECT user_id FROM api_keys WHERE key = ? AND (expires_at IS NULL OR expires_at > unixepoch())"
            )
            .bind(auth_header)
            .fetch_optional(state.db.pool())
            .await
            .map_err(|e| {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)).into_response()
            })?;

            match user_id {
                Some(user_id) => Ok(ApiClaims { user_id }),
                None => Err((StatusCode::UNAUTHORIZED, "Invalid API key").into_response()),
            }
        }
    }
}

#[derive(RustEmbed)]
#[folder = "static"]
struct StaticAssets;

#[derive(Parser, Debug, Clone)]
pub struct Config {
    /// SQLite database path (e.g. 'data.db' or ':memory:' for in-memory database)
    #[arg(long, env = "DATABASE_PATH", default_value = "data.db")]
    pub database_path: String,
    
    /// HTTP server bind address
    #[arg(long, env = "BIND_ADDR", default_value = "127.0.0.1:3000")]
    pub bind_addr: String,

    /// Web app URL (e.g. 'https://example.com')
    #[arg(long, env = "WEB_APP_URL", default_value = "https://example.com")]
    pub web_app_url: String,

    /// Supported email domains (comma-separated)
    #[arg(long, env = "SUPPORTED_DOMAINS", value_delimiter = ',', default_value = "mail-hook.example.com")]
    pub supported_domains: Vec<String>,
}

static CONFIG: OnceLock<Config> = OnceLock::new();

#[cfg(not(test))]
pub fn init_config(config: Config) {
    CONFIG.set(config).expect("Config already initialized");
}

#[cfg(test)]
pub fn init_config(config: Config) {
    // In test mode, we want to allow reinitialization
    // First try to get the existing config
    if let Some(existing) = CONFIG.get() {
        // If the configs are the same, just return
        if existing.database_path == config.database_path 
            && existing.bind_addr == config.bind_addr 
            && existing.web_app_url == config.web_app_url
            && existing.supported_domains == config.supported_domains {
            return;
        }
    }
    // If we get here, either there was no config or it was different
    // We can't unset a OnceLock, so we'll just ignore the result
    let _ = CONFIG.set(config);
}

pub fn get_web_app_url() -> String {
    CONFIG.get()
        .expect("Config not initialized")
        .web_app_url
        .clone()
}

pub struct AppState<D: Database> {
    db: Arc<D>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SupportedDomainsResponse {
    domains: Vec<String>,
}

impl<T> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateMailboxRequest {
    name: String,
    expires_in_seconds: Option<i64>,
    public_key: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMailboxRequest {
    name: Option<String>,
    expires_in_seconds: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ApiKey {
    pub id: String,
    pub key: String,
    pub created_at: i64,
    pub expires_at: Option<i64>,
}

pub async fn run(config: Config) -> anyhow::Result<()> {
    init_config(config.clone());

    let db = common::db::SqliteDatabase::new(&format!("sqlite:{}", config.database_path)).await?;
    let db = Arc::new(db);
    
    let app = create_app(db);

    let addr: SocketAddr = config.bind_addr.parse()?;
    info!("Starting web server on {}", addr);
    
    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}

pub fn create_app<D: Database + 'static>(
    db: Arc<D>,
) -> Router {
    let state = Arc::new(AppState {
        db,
    });

    let web_app_url: Url = get_web_app_url().parse().unwrap();

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::exact(HeaderValue::from_str(&web_app_url.origin().ascii_serialization()).unwrap()))
        .allow_methods(Any)
        .allow_headers(Any);

    // Create a router for protected mailbox routes
    let frontend_routes = Router::new()
        .route("/api/mailboxes", get(list_mailboxes::<D>))
        .route("/api/mailboxes", post(create_mailbox::<D>))
        .route("/api/mailboxes/:id", get(get_mailbox::<D>))
        .route("/api/mailboxes/:id", delete(delete_mailbox::<D>))
        .route("/api/mailboxes/:id", patch(update_mailbox::<D>))
        .route("/api/mailboxes/:id/emails", get(get_mailbox_emails::<D>))
        .route("/api/mailboxes/:id/emails/:email_id", get(get_email::<D>))
        .route("/api/mailboxes/:id/emails/:email_id", delete(delete_email::<D>))
        .route("/api/supported-domains", get(get_supported_domains::<D>))
        .route("/api/api-keys", get(list_api_keys::<D>))
        .route("/api/api-keys", post(create_api_key::<D>))
        .route("/api/api-keys/:id", delete(delete_api_key::<D>))
        .layer(middleware::from_fn(handle_json_response));

    let api_routes = Router::new()
        .route("/v1/mailboxes/:id/emails", get(api_get_mailbox_emails::<D>))
        .route("/v1/mailboxes/:id/emails/:email_id", get(api_get_email::<D>))
        .route("/v1/mailboxes/:id/emails/:email_id", delete(api_delete_email::<D>))
        .route("/v1/swagger-spec.json", get(serve_swagger_spec))
        .layer(middleware::from_fn(handle_json_response));

    Router::new()
        .merge(auth::create_routes::<D>())
        .nest("/", frontend_routes.layer(middleware::from_fn(auth::auth)))
        .nest("/api", api_routes)   
        .fallback(static_handler)
        .layer(cors)
        .with_state(state)
}

async fn static_handler(uri: axum::http::Uri, method: axum::http::Method) -> impl IntoResponse {
    // Only serve static files for GET requests
    if method != axum::http::Method::GET {
        return Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(axum::body::Body::empty())
            .unwrap();
    }

    let path = uri.path().trim_start_matches('/');
    
    // Special case for API documentation
    if path == "api/docs" {
        return match StaticAssets::get("swagger.html") {
            Some(content) => Response::builder()
                .header(header::CONTENT_TYPE, "text/html")
                .body(axum::body::Body::from(content.data))
                .unwrap(),
            None => Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(axum::body::Body::from("404 Not Found"))
                .unwrap(),
        };
    }
    
    // Don't try to serve static files for API routes
    if path.starts_with("api/") {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(axum::body::Body::empty())
            .unwrap();
    }

    // First try to serve the exact static file
    if let Some(content) = StaticAssets::get(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        return Response::builder()
            .header(header::CONTENT_TYPE, mime.as_ref())
            .body(axum::body::Body::from(content.data))
            .unwrap();
    }

    // If no static file is found, serve index.html for client-side routing
    match StaticAssets::get("index.html") {
        Some(content) => {
            let html = String::from_utf8_lossy(&content.data);
            
            // Get configuration values
            let telegram_bot_name = std::env::var("TELEGRAM_BOT_NAME").unwrap_or_default();
            
            // Create the runtime config script
            let config_script = format!(
                r#"<script>
                    window.RUNTIME_CONFIG = {{
                        TELEGRAM_BOT_NAME: "{}",
                    }};
                </script>"#,
                telegram_bot_name
            );
            
            // Replace the placeholder with the actual config
            let html = html.replace("<!--RUNTIME_CONFIG_PLACEHOLDER-->", &config_script);
            
            Response::builder()
                .header(header::CONTENT_TYPE, "text/html")
                .body(axum::body::Body::from(html))
                .unwrap()
        }
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(axum::body::Body::from("404 Not Found"))
            .unwrap(),
    }
}

async fn create_mailbox<D: Database>(
    State(state): State<Arc<AppState<D>>>,
    claims: axum::extract::Extension<Claims>,
    Json(req): Json<CreateMailboxRequest>,
) -> Result<Json<ApiResponse<Mailbox>>, StatusCode> {
    // Validate expiration time
    if let Some(seconds) = req.expires_in_seconds {
        if seconds <= 0 {
            return Ok(Json(ApiResponse::error("Expiration time must be positive")));
        }
        if seconds > 30 * 24 * 60 * 60 {
            return Ok(Json(ApiResponse::error("Maximum expiration time is 30 days")));
        }
    }

    let mailbox = Mailbox {
        id: common::generate_random_id(12),
        alias: common::generate_random_id(12),
        name: req.name,
        public_key: req.public_key,
        owner_id: claims.sub.clone(),
        created_at: chrono::Utc::now().timestamp(),
        mail_expires_in: req.expires_in_seconds,
    };
    
    match state.db.create_mailbox(&mailbox).await {
        Ok(_) => Ok(Json(ApiResponse::success(mailbox))),
        Err(e) => {
            error!("Failed to create mailbox: {}", e);
            // Check if it's a unique constraint violation
            if e.to_string().contains("UNIQUE constraint failed") {
                Ok(Json(ApiResponse::error("A mailbox with this alias already exists")))
            } else {
                Ok(Json(ApiResponse::error("Unable to create mailbox. Please try again later")))
            }
        }
    }
}

async fn get_mailbox<D: Database>(
    State(state): State<Arc<AppState<D>>>,
    claims: axum::extract::Extension<Claims>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Mailbox>>, StatusCode> {
    match state.db.get_mailbox(&id).await {
        Ok(Some(mailbox)) => {
            // Ensure the mailbox belongs to the authenticated user
            if mailbox.owner_id != claims.sub {
                return Ok(Json(ApiResponse::error("You do not have permission to access this mailbox")));
            }
            Ok(Json(ApiResponse::success(mailbox)))
        }
        Ok(None) => Ok(Json(ApiResponse::error("Mailbox not found"))),
        Err(e) => {
            error!("Database error while getting mailbox: {}", e);
            Ok(Json(ApiResponse::error("Unable to retrieve mailbox. Please try again later")))
        }
    }
}

async fn delete_mailbox<D: Database>(
    State(state): State<Arc<AppState<D>>>,
    claims: axum::extract::Extension<Claims>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    // First check if the mailbox belongs to the authenticated user
    match state.db.get_mailbox(&id).await {
        Ok(Some(mailbox)) => {
            if mailbox.owner_id != claims.sub {
                return Ok(Json(ApiResponse::error("You do not have permission to delete this mailbox")));
            }
            match state.db.delete_mailbox(&id).await {
                Ok(_) => Ok(Json(ApiResponse::success(()))),
                Err(e) => {
                    error!("Database error while deleting mailbox: {}", e);
                    Ok(Json(ApiResponse::error("Unable to delete mailbox. Please try again later")))
                }
            }
        }
        Ok(None) => Ok(Json(ApiResponse::error("Mailbox not found"))),
        Err(e) => {
            error!("Database error while checking mailbox: {}", e);
            Ok(Json(ApiResponse::error("Unable to process request. Please try again later")))
        }
    }
}

async fn update_mailbox<D: Database>(
    State(state): State<Arc<AppState<D>>>,
    claims: axum::extract::Extension<Claims>,
    Path(id): Path<String>,
    Json(req): Json<UpdateMailboxRequest>,
) -> Result<Json<ApiResponse<Mailbox>>, StatusCode> {
    let result: Result<Mailbox, AppError> = async {
        let mut mailbox = state.db.get_mailbox(&id).await?
            .ok_or_else(|| AppError::NotFound("Mailbox not found".into()))?;

        // Ensure the mailbox belongs to the authenticated user
        if mailbox.owner_id != claims.sub {
            return Err(AppError::Auth("Unauthorized".into()));
        }

        if let Some(name) = req.name {
            mailbox.name = name;
        }

        if let Some(seconds) = req.expires_in_seconds {
            if seconds <= 0 {
                return Err(AppError::Mail("Expiration time must be positive".into()));
            }
            if seconds > 30 * 24 * 60 * 60 {
                return Err(AppError::Mail("Maximum expiration time is 30 days".into()));
            }
            mailbox.mail_expires_in = Some(seconds);
        }

        state.db.update_mailbox(&mailbox).await?;
        Ok(mailbox)
    }.await;

    match result {
        Ok(mailbox) => Ok(Json(ApiResponse::success(mailbox))),
        Err(e) => {
            error!("Failed to update mailbox: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

async fn get_mailbox_emails_for_user<D: Database>(
    state: &Arc<AppState<D>>,
    user_id: &str,
    mailbox_id: &str,
) -> Result<Vec<Email>, AppError> {
    // First check if the mailbox belongs to the user
    let mailbox = state.db.get_mailbox(mailbox_id).await?
        .ok_or_else(|| AppError::NotFound("Mailbox not found".into()))?;

    if mailbox.owner_id != user_id {
        return Err(AppError::Auth("You do not have permission to access emails from this mailbox".into()));
    }

    state.db.get_mailbox_emails(mailbox_id).await
}

async fn get_mailbox_emails<D: Database>(
    State(state): State<Arc<AppState<D>>>,
    claims: axum::extract::Extension<Claims>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Vec<Email>>>, StatusCode> {
    match get_mailbox_emails_for_user(&state, &claims.sub, &id).await {
        Ok(emails) => Ok(Json(ApiResponse::success(emails))),
        Err(e) => {
            error!("Error while retrieving emails: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

async fn get_email_for_user<D: Database>(
    state: &Arc<AppState<D>>,
    user_id: &str,
    mailbox_id: &str,
    email_id: &str,
) -> Result<Email, AppError> {
    // First check if the mailbox belongs to the user
    let mailbox = state.db.get_mailbox(mailbox_id).await?
        .ok_or_else(|| AppError::NotFound("Mailbox not found".into()))?;

    if mailbox.owner_id != user_id {
        return Err(AppError::Auth("You do not have permission to access this email".into()));
    }

    let email = state.db.get_email(email_id).await?
        .ok_or_else(|| AppError::NotFound("Email not found".into()))?;

    if email.mailbox_id != mailbox_id {
        return Err(AppError::NotFound("Email not found in this mailbox".into()));
    }

    Ok(email)
}

async fn get_email<D: Database>(
    State(state): State<Arc<AppState<D>>>,
    claims: axum::extract::Extension<Claims>,
    Path((mailbox_id, email_id)): Path<(String, String)>,
) -> Result<Json<ApiResponse<Email>>, StatusCode> {
    match get_email_for_user(&state, &claims.sub, &mailbox_id, &email_id).await {
        Ok(email) => Ok(Json(ApiResponse::success(email))),
        Err(e) => {
            error!("Error while retrieving email: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

async fn delete_email_for_user<D: Database>(
    state: &Arc<AppState<D>>,
    user_id: &str,
    mailbox_id: &str,
    email_id: &str,
) -> Result<(), AppError> {
    // First check if the mailbox belongs to the user
    let mailbox = state.db.get_mailbox(mailbox_id).await?
        .ok_or_else(|| AppError::NotFound("Mailbox not found".into()))?;

    if mailbox.owner_id != user_id {
        return Err(AppError::Auth("You do not have permission to delete this email".into()));
    }

    let email = state.db.get_email(email_id).await?
        .ok_or_else(|| AppError::NotFound("Email not found".into()))?;

    if email.mailbox_id != mailbox_id {
        return Err(AppError::NotFound("Email not found in this mailbox".into()));
    }

    state.db.delete_email(email_id).await
}

async fn delete_email<D: Database>(
    State(state): State<Arc<AppState<D>>>,
    claims: axum::extract::Extension<Claims>,
    Path((mailbox_id, email_id)): Path<(String, String)>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    match delete_email_for_user(&state, &claims.sub, &mailbox_id, &email_id).await {
        Ok(_) => Ok(Json(ApiResponse::success(()))),
        Err(e) => {
            error!("Error while deleting email: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

async fn list_mailboxes<D: Database>(
    State(state): State<Arc<AppState<D>>>,
    claims: axum::extract::Extension<Claims>,
) -> Result<Json<ApiResponse<Vec<Mailbox>>>, StatusCode> {
    match state.db.get_mailboxes_by_owner(&claims.sub).await {
        Ok(mailboxes) => Ok(Json(ApiResponse::success(mailboxes))),
        Err(e) => {
            error!("Database error while listing mailboxes: {}", e);
            Ok(Json(ApiResponse::error("Unable to retrieve mailboxes. Please try again later")))
        }
    }
}

async fn get_supported_domains<D: Database>(
    State(_state): State<Arc<AppState<D>>>,
) -> Result<Json<ApiResponse<SupportedDomainsResponse>>, StatusCode> {
    let domains = CONFIG.get()
        .expect("Config not initialized")
        .supported_domains
        .clone();
    
    Ok(Json(ApiResponse::success(SupportedDomainsResponse { domains })))
}

async fn list_api_keys<D: Database>(
    State(state): State<Arc<AppState<D>>>,
    claims: axum::extract::Extension<Claims>,
) -> Result<Json<ApiResponse<Vec<ApiKey>>>, StatusCode> {
    let rows = sqlx::query(
        "SELECT id, key, created_at, expires_at FROM api_keys WHERE user_id = ?"
    )
    .bind(&claims.sub)
    .fetch_all(state.db.pool())
    .await
    .map_err(|e| {
        error!("Database error while listing API keys: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let api_keys = rows.iter().map(|row| ApiKey {
        id: row.get("id"),
        key: row.get("key"),
        created_at: row.get("created_at"),
        expires_at: row.get("expires_at"),
    }).collect();

    Ok(Json(ApiResponse::success(api_keys)))
}

async fn create_api_key<D: Database>(
    State(state): State<Arc<AppState<D>>>,
    claims: axum::extract::Extension<Claims>,
) -> Result<Json<ApiResponse<ApiKey>>, StatusCode> {
    let api_key = state.db.create_api_key(&claims.sub)
        .await
        .map_err(|e| {
            error!("Database error while creating API key: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(ApiResponse::success(ApiKey {
        id: api_key.id,
        key: api_key.key,
        created_at: api_key.created_at,
        expires_at: api_key.expires_at,
    })))
}

async fn delete_api_key<D: Database>(
    State(state): State<Arc<AppState<D>>>,
    claims: axum::extract::Extension<Claims>,
    Path(key_id): Path<String>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    // First verify the API key belongs to the user
    let user_id: Option<String> = sqlx::query_scalar(
        "SELECT user_id FROM api_keys WHERE id = ?"
    )
    .bind(&key_id)
    .fetch_optional(state.db.pool())
    .await
    .map_err(|e| {
        error!("Database error while verifying API key ownership: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    match user_id {
        Some(id) if id == claims.sub => {
            state.db.delete_api_key(&key_id)
                .await
                .map_err(|e| {
                    error!("Database error while deleting API key: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
            Ok(Json(ApiResponse::success(())))
        }
        Some(_) => Ok(Json(ApiResponse::error("You don't have permission to delete this API key"))),
        None => Ok(Json(ApiResponse::error("API key not found"))),
    }
}

// @APIDOC-START
/// Get emails from a mailbox
/// 
/// Lists all emails in the specified mailbox. Requires API authentication.
/// 
/// Authorization:
/// - Requires a valid API key in the Authorization header
/// - Format: `Authorization: Bearer <api-key>`
/// 
/// Parameters:
/// - `id`: The ID of the mailbox to retrieve emails from
/// 
/// Returns:
/// - 200: List of emails in the mailbox
/// - 401: Missing or invalid API key
/// - 403: API key owner doesn't have access to the mailbox
/// - 404: Mailbox not found
/// 
/// Example response:
/// ```json
/// {
///   "success": true,
///   "data": [
///     {
///       "id": "string",
///       "mailbox_id": "string",
///       "subject": "string",
///       "from": "string",
///       "to": "string",
///       "content": "string",
///       "received_at": 1234567890
///     }
///   ]
/// }
/// ```
async fn api_get_mailbox_emails<D>(
    State(state): State<Arc<AppState<D>>>,
    api_claims: api_auth::ApiClaims,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Vec<Email>>>, StatusCode>
where
    D: Database + Send + Sync + 'static,
{
    match get_mailbox_emails_for_user(&state, &api_claims.user_id, &id).await {
        Ok(emails) => Ok(Json(ApiResponse::success(emails))),
        Err(e) => {
            error!("API error while retrieving emails: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

// @APIDOC-START
/// Get a specific email from a mailbox
/// 
/// Retrieves a single email by its ID from the specified mailbox. Requires API authentication.
/// 
/// Authorization:
/// - Requires a valid API key in the Authorization header
/// - Format: `Authorization: Bearer <api-key>`
/// 
/// Parameters:
/// - `mailbox_id`: The ID of the mailbox containing the email
/// - `email_id`: The ID of the email to retrieve
/// 
/// Returns:
/// - 200: The requested email
/// - 401: Missing or invalid API key
/// - 403: API key owner doesn't have access to the mailbox
/// - 404: Mailbox or email not found
/// 
/// Example response:
/// ```json
/// {
///   "success": true,
///   "data": {
///     "id": "string",
///     "mailbox_id": "string",
///     "subject": "string",
///     "from": "string",
///     "to": "string",
///     "content": "string",
///     "received_at": 1234567890
///   }
/// }
/// ```
async fn api_get_email<D>(
    State(state): State<Arc<AppState<D>>>,
    api_claims: api_auth::ApiClaims,
    Path((mailbox_id, email_id)): Path<(String, String)>,
) -> Result<Json<ApiResponse<Email>>, StatusCode>
where
    D: Database + Send + Sync + 'static,
{
    match get_email_for_user(&state, &api_claims.user_id, &mailbox_id, &email_id).await {
        Ok(email) => Ok(Json(ApiResponse::success(email))),
        Err(e) => {
            error!("API error while retrieving email: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

// @APIDOC-START
/// Delete an email from a mailbox
/// 
/// Permanently deletes a single email from the specified mailbox. 
/// This operation cannot be undone. Requires API authentication.
/// 
/// Authorization:
/// - Requires a valid API key in the Authorization header
/// - Format: `Authorization: Bearer <api-key>`
/// 
/// Parameters:
/// - `mailbox_id`: The ID of the mailbox containing the email
/// - `email_id`: The ID of the email to delete
/// 
/// Returns:
/// - 200: Email successfully deleted
/// - 401: Missing or invalid API key
/// - 403: API key owner doesn't have access to the mailbox
/// - 404: Mailbox or email not found
/// 
/// Example response:
/// ```json
/// {
///   "success": true,
///   "data": null
/// }
/// ```
async fn api_delete_email<D>(
    State(state): State<Arc<AppState<D>>>,
    api_claims: api_auth::ApiClaims,
    Path((mailbox_id, email_id)): Path<(String, String)>,
) -> Result<Json<ApiResponse<()>>, StatusCode>
where
    D: Database + Send + Sync + 'static,
{
    match delete_email_for_user(&state, &api_claims.user_id, &mailbox_id, &email_id).await {
        Ok(_) => Ok(Json(ApiResponse::success(()))),
        Err(e) => {
            error!("API error while deleting email: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

// Re-export auth types for public use
pub use auth::{AuthResponse, LoginRequest, RegisterRequest};

async fn serve_swagger_spec() -> impl IntoResponse {
    Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .body(api_spec::SWAGGER_SPEC.to_string())
        .unwrap()
} 