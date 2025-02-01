use axum::{
    routing::{get, post, delete, patch},
    Router,
    extract::{State, Json, Path},
    http::StatusCode,
};
use common::{Mailbox, Email, db::Database, AppError};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, net::SocketAddr};
use tower_http::cors::CorsLayer;
use tracing::{info, error};
use clap::Parser;
use tokio::net::TcpListener;

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
}

pub struct AppState {
    db: Arc<dyn Database>,
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

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    username: String,
    auth_type: common::AuthType,
}

pub async fn run(config: Config) -> anyhow::Result<()> {
    let db = common::db::SqliteDatabase::new(&format!("sqlite:{}", config.database_path)).await?;
    let db = Arc::new(db);
    
    let app = create_app(
        db,
        config.email_domain,
    );

    let addr: SocketAddr = config.bind_addr.parse()?;
    info!("Starting web server on {}", addr);
    
    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}

pub fn create_app(
    db: Arc<dyn Database>,
    email_domain: String,
) -> Router {
    let state = Arc::new(AppState {
        db,
        email_domain,
    });

    let cors = CorsLayer::permissive();

    Router::new()
        .route("/api/users", post(create_user))
        .route("/api/mailboxes", post(create_mailbox))
        .route("/api/mailboxes/:id", get(get_mailbox))
        .route("/api/mailboxes/:id", delete(delete_mailbox))
        .route("/api/mailboxes/:id", patch(update_mailbox))
        .route("/api/mailboxes/:id/emails", get(get_mailbox_emails))
        .route("/api/mailboxes/:id/emails/:email_id", get(get_email))
        .route("/api/mailboxes/:id/emails/:email_id", delete(delete_email))
        .layer(cors)
        .with_state(state)
}

async fn create_user(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<ApiResponse<common::User>>, StatusCode> {
    match state.db.create_user(&req.username, req.auth_type).await {
        Ok(user) => Ok(Json(ApiResponse::success(user))),
        Err(e) => {
            error!("Failed to create user: {}", e);
            Ok(Json(ApiResponse::error("Failed to create user")))
        }
    }
}

async fn create_mailbox(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateMailboxRequest>,
) -> Result<Json<ApiResponse<Mailbox>>, StatusCode> {
    let expires_at = req.expires_in_days.map(|days| {
        chrono::Utc::now() + chrono::Duration::days(days)
    }).map(|dt| dt.timestamp());

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

async fn get_mailbox(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Mailbox>>, StatusCode> {
    match state.db.get_mailbox(&id).await {
        Ok(Some(mailbox)) => Ok(Json(ApiResponse::success(mailbox))),
        Ok(None) => Ok(Json(ApiResponse::error("Mailbox not found"))),
        Err(e) => {
            error!("Failed to get mailbox: {}", e);
            Ok(Json(ApiResponse::error("Failed to get mailbox")))
        }
    }
}

async fn delete_mailbox(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    match state.db.delete_mailbox(&id).await {
        Ok(_) => Ok(Json(ApiResponse::success(()))),
        Err(e) => {
            error!("Failed to delete mailbox: {}", e);
            Ok(Json(ApiResponse::error("Failed to delete mailbox")))
        }
    }
}

async fn update_mailbox(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateMailboxRequest>,
) -> Result<Json<ApiResponse<Mailbox>>, StatusCode> {
    let result: Result<Mailbox, AppError> = async {
        let mut mailbox = state.db.get_mailbox(&id).await?
            .ok_or_else(|| AppError::NotFound("Mailbox not found".into()))?;

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

async fn get_mailbox_emails(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Vec<Email>>>, StatusCode> {
    match state.db.get_mailbox(&id).await {
        Ok(Some(_)) => {
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

async fn get_email(
    State(state): State<Arc<AppState>>,
    Path((mailbox_id, email_id)): Path<(String, String)>,
) -> Result<Json<ApiResponse<Email>>, StatusCode> {
    let result = async {
        let _ = state.db.get_mailbox(&mailbox_id).await?
            .ok_or_else(|| AppError::NotFound("Mailbox not found".into()))?;

        let email = state.db.get_email(&email_id).await?
            .ok_or_else(|| AppError::NotFound("Email not found".into()))?;

        if email.mailbox_id != mailbox_id {
            return Err(AppError::NotFound("Email not found in this mailbox".into()));
        }

        Ok(email)
    }.await;

    match result {
        Ok(email) => Ok(Json(ApiResponse::success(email))),
        Err(e) => {
            error!("Failed to get email: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

async fn delete_email(
    State(state): State<Arc<AppState>>,
    Path((mailbox_id, email_id)): Path<(String, String)>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    let result = async {
        let _ = state.db.get_mailbox(&mailbox_id).await?
            .ok_or_else(|| AppError::NotFound("Mailbox not found".into()))?;

        let email = state.db.get_email(&email_id).await?
            .ok_or_else(|| AppError::NotFound("Email not found".into()))?;

        if email.mailbox_id != mailbox_id {
            return Err(AppError::NotFound("Email not found in this mailbox".into()));
        }

        state.db.delete_email(&email_id).await?;
        Ok(())
    }.await;

    match result {
        Ok(_) => Ok(Json(ApiResponse::success(()))),
        Err(e) => {
            error!("Failed to delete email: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
} 