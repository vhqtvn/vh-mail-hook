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

mod auth;
use auth::Claims;

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
            && existing.web_app_url == config.web_app_url {
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
    let mailbox_routes = Router::new()
        .route("/api/mailboxes", get(list_mailboxes::<D>))
        .route("/api/mailboxes", post(create_mailbox::<D>))
        .route("/api/mailboxes/:id", get(get_mailbox::<D>))
        .route("/api/mailboxes/:id", delete(delete_mailbox::<D>))
        .route("/api/mailboxes/:id", patch(update_mailbox::<D>))
        .route("/api/mailboxes/:id/emails", get(get_mailbox_emails::<D>))
        .route("/api/mailboxes/:id/emails/:email_id", get(get_email::<D>))
        .route("/api/mailboxes/:id/emails/:email_id", delete(delete_email::<D>))
        .route("/api/supported-domains", get(get_supported_domains::<D>))
        .layer(middleware::from_fn(handle_json_response));

    Router::new()
        .merge(auth::create_routes::<D>())
        .nest("/", mailbox_routes.layer(middleware::from_fn(auth::auth)).layer(middleware::from_fn(handle_json_response)))
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

async fn get_mailbox_emails<D: Database>(
    State(state): State<Arc<AppState<D>>>,
    claims: axum::extract::Extension<Claims>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Vec<Email>>>, StatusCode> {
    // First check if the mailbox belongs to the authenticated user
    match state.db.get_mailbox(&id).await {
        Ok(Some(mailbox)) => {
            if mailbox.owner_id != claims.sub {
                return Ok(Json(ApiResponse::error("You do not have permission to access emails from this mailbox")));
            }
            match state.db.get_mailbox_emails(&id).await {
                Ok(emails) => Ok(Json(ApiResponse::success(emails))),
                Err(e) => {
                    error!("Database error while retrieving emails: {}", e);
                    Ok(Json(ApiResponse::error("Unable to retrieve emails. Please try again later")))
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

async fn get_email<D: Database>(
    State(state): State<Arc<AppState<D>>>,
    claims: axum::extract::Extension<Claims>,
    Path((mailbox_id, email_id)): Path<(String, String)>,
) -> Result<Json<ApiResponse<Email>>, StatusCode> {
    // First check if the mailbox belongs to the authenticated user
    match state.db.get_mailbox(&mailbox_id).await {
        Ok(Some(mailbox)) => {
            if mailbox.owner_id != claims.sub {
                return Ok(Json(ApiResponse::error("You do not have permission to access this email")));
            }
            match state.db.get_email(&email_id).await {
                Ok(Some(email)) => {
                    if email.mailbox_id != mailbox_id {
                        return Ok(Json(ApiResponse::error("Email not found in this mailbox")));
                    }
                    Ok(Json(ApiResponse::success(email)))
                }
                Ok(None) => Ok(Json(ApiResponse::error("Email not found"))),
                Err(e) => {
                    error!("Database error while retrieving email: {}", e);
                    Ok(Json(ApiResponse::error("Unable to retrieve email. Please try again later")))
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

async fn delete_email<D: Database>(
    State(state): State<Arc<AppState<D>>>,
    claims: axum::extract::Extension<Claims>,
    Path((mailbox_id, email_id)): Path<(String, String)>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    // First check if the mailbox belongs to the authenticated user
    match state.db.get_mailbox(&mailbox_id).await {
        Ok(Some(mailbox)) => {
            if mailbox.owner_id != claims.sub {
                return Ok(Json(ApiResponse::error("You do not have permission to delete this email")));
            }
            match state.db.get_email(&email_id).await {
                Ok(Some(email)) => {
                    if email.mailbox_id != mailbox_id {
                        return Ok(Json(ApiResponse::error("Email not found in this mailbox")));
                    }
                    match state.db.delete_email(&email_id).await {
                        Ok(_) => Ok(Json(ApiResponse::success(()))),
                        Err(e) => {
                            error!("Database error while deleting email: {}", e);
                            Ok(Json(ApiResponse::error("Unable to delete email. Please try again later")))
                        }
                    }
                }
                Ok(None) => Ok(Json(ApiResponse::error("Email not found"))),
                Err(e) => {
                    error!("Database error while checking email: {}", e);
                    Ok(Json(ApiResponse::error("Unable to process request. Please try again later")))
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
    // TODO: For now, return a hardcoded list. In the future, this could be configurable or fetched from a database
    let domains = vec![
        "mail-hook.example.com".to_string(),
        // Add more supported domains here
    ];
    
    Ok(Json(ApiResponse::success(SupportedDomainsResponse { domains })))
}

// Re-export auth types for public use
pub use auth::{AuthResponse, LoginRequest, RegisterRequest}; 