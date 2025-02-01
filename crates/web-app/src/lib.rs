use axum::{
    extract::{Json, Path, State}, http::{HeaderValue, StatusCode, header}, middleware, routing::{delete, get, patch, post}, Router,
    response::{IntoResponse, Response},
};
use common::{Mailbox, Email, db::Database, AppError};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, net::SocketAddr};
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tracing::{info, error};
use clap::Parser;
use tokio::net::TcpListener;
use rust_embed::RustEmbed;

mod auth;
use auth::Claims;

#[derive(RustEmbed)]
#[folder = "static"]
struct StaticAssets;

#[derive(Parser)]
pub struct Config {
    /// SQLite database path (e.g. 'data.db' or ':memory:' for in-memory database)
    #[arg(long, env = "DATABASE_PATH", default_value = "data.db")]
    pub database_path: String,
    
    /// HTTP server bind address
    #[arg(long, env = "BIND_ADDR", default_value = "127.0.0.1:3000")]
    pub bind_addr: String,

    /// Email domain for generated mailbox addresses (e.g. 'example.com')
    #[arg(long, env = "EMAIL_DOMAIN", default_value = "example.com")]
    pub email_domain: String,

    /// Web app URL (e.g. 'https://example.com')
    #[arg(long, env = "WEB_APP_URL", default_value = "https://example.com")]
    pub web_app_url: String,
}

pub struct AppState<D: Database> {
    db: Arc<D>,
    email_domain: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
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
    expires_in_days: Option<i64>,
    owner_id: String,
    public_key: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMailboxRequest {
    expires_in_days: Option<i64>,
}

pub async fn run(config: Config) -> anyhow::Result<()> {
    let db = common::db::SqliteDatabase::new(&format!("sqlite:{}", config.database_path)).await?;
    let db = Arc::new(db);
    
    let app = create_app(
        db,
        config.email_domain,
        config.web_app_url,
    );

    let addr: SocketAddr = config.bind_addr.parse()?;
    info!("Starting web server on {}", addr);
    
    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}

pub fn create_app<D: Database + 'static>(
    db: Arc<D>,
    email_domain: String,
    web_app_url: String,
) -> Router {
    let state = Arc::new(AppState {
        db,
        email_domain,
    });

    let web_app_url: Url = web_app_url.parse().unwrap();

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::exact(HeaderValue::from_str(&web_app_url.origin().ascii_serialization()).unwrap()))
        .allow_methods(Any)
        .allow_headers(Any);

    // Create a router for protected mailbox routes
    let mailbox_routes = Router::new()
        .route("/api/mailboxes", post(create_mailbox::<D>))
        .route("/api/mailboxes/:id", get(get_mailbox::<D>))
        .route("/api/mailboxes/:id", delete(delete_mailbox::<D>))
        .route("/api/mailboxes/:id", patch(update_mailbox::<D>))
        .route("/api/mailboxes/:id/emails", get(get_mailbox_emails::<D>))
        .route("/api/mailboxes/:id/emails/:email_id", get(get_email::<D>))
        .route("/api/mailboxes/:id/emails/:email_id", delete(delete_email::<D>));

    Router::new()
        .merge(auth::create_routes::<D>())
        .nest("/", mailbox_routes.layer(middleware::from_fn(auth::auth)))
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
        Some(content) => Response::builder()
            .header(header::CONTENT_TYPE, "text/html")
            .body(axum::body::Body::from(content.data))
            .unwrap(),
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
    // Ensure the owner_id matches the authenticated user
    if req.owner_id != claims.sub {
        return Ok(Json(ApiResponse::error("Unauthorized")));
    }

    let expires_at = req.expires_in_days.map(|days| {
        (chrono::Utc::now() + chrono::Duration::days(days)).timestamp()
    });

    let mailbox = Mailbox {
        id: uuid::Uuid::new_v4().to_string(),
        address: format!("{}@{}", uuid::Uuid::new_v4(), state.email_domain),
        public_key: req.public_key,
        owner_id: req.owner_id,
        created_at: chrono::Utc::now().timestamp(),
        expires_at,
    };
    
    match state.db.create_mailbox(&mailbox).await {
        Ok(_) => Ok(Json(ApiResponse::success(mailbox))),
        Err(e) => {
            error!("Failed to create mailbox: {}", e);
            Ok(Json(ApiResponse::error("Failed to create mailbox")))
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
                return Ok(Json(ApiResponse::error("Unauthorized")));
            }
            Ok(Json(ApiResponse::success(mailbox)))
        }
        Ok(None) => Ok(Json(ApiResponse::error("Mailbox not found"))),
        Err(e) => {
            error!("Failed to get mailbox: {}", e);
            Ok(Json(ApiResponse::error("Failed to get mailbox")))
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
                return Ok(Json(ApiResponse::error("Unauthorized")));
            }
            match state.db.delete_mailbox(&id).await {
                Ok(_) => Ok(Json(ApiResponse::success(()))),
                Err(e) => {
                    error!("Failed to delete mailbox: {}", e);
                    Ok(Json(ApiResponse::error("Failed to delete mailbox")))
                }
            }
        }
        Ok(None) => Ok(Json(ApiResponse::error("Mailbox not found"))),
        Err(e) => {
            error!("Failed to get mailbox: {}", e);
            Ok(Json(ApiResponse::error("Failed to get mailbox")))
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

        if let Some(days) = req.expires_in_days {
            mailbox.expires_at = Some((chrono::Utc::now() + chrono::Duration::days(days)).timestamp());
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
                return Ok(Json(ApiResponse::error("Unauthorized")));
            }
            match state.db.get_mailbox_emails(&id).await {
                Ok(emails) => Ok(Json(ApiResponse::success(emails))),
                Err(e) => {
                    error!("Failed to get emails: {}", e);
                    Ok(Json(ApiResponse::error("Failed to get emails")))
                }
            }
        }
        Ok(None) => Ok(Json(ApiResponse::error("Mailbox not found"))),
        Err(e) => {
            error!("Failed to get mailbox: {}", e);
            Ok(Json(ApiResponse::error("Failed to get mailbox")))
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
                return Ok(Json(ApiResponse::error("Unauthorized")));
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
                    error!("Failed to get email: {}", e);
                    Ok(Json(ApiResponse::error("Failed to get email")))
                }
            }
        }
        Ok(None) => Ok(Json(ApiResponse::error("Mailbox not found"))),
        Err(e) => {
            error!("Failed to get mailbox: {}", e);
            Ok(Json(ApiResponse::error("Failed to get mailbox")))
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
                return Ok(Json(ApiResponse::error("Unauthorized")));
            }
            match state.db.get_email(&email_id).await {
                Ok(Some(email)) => {
                    if email.mailbox_id != mailbox_id {
                        return Ok(Json(ApiResponse::error("Email not found in this mailbox")));
                    }
                    match state.db.delete_email(&email_id).await {
                        Ok(_) => Ok(Json(ApiResponse::success(()))),
                        Err(e) => {
                            error!("Failed to delete email: {}", e);
                            Ok(Json(ApiResponse::error("Failed to delete email")))
                        }
                    }
                }
                Ok(None) => Ok(Json(ApiResponse::error("Email not found"))),
                Err(e) => {
                    error!("Failed to get email: {}", e);
                    Ok(Json(ApiResponse::error("Failed to get email")))
                }
            }
        }
        Ok(None) => Ok(Json(ApiResponse::error("Mailbox not found"))),
        Err(e) => {
            error!("Failed to get mailbox: {}", e);
            Ok(Json(ApiResponse::error("Failed to get mailbox")))
        }
    }
}

// Re-export auth types for public use
pub use auth::{AuthResponse, LoginRequest, RegisterRequest}; 