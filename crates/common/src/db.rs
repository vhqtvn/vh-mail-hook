use crate::{ApiKey, AppError, AuthType, Email, Mailbox, User, UserSettings};
use async_trait::async_trait;
use sqlx::{migrate::MigrateDatabase, sqlite::SqlitePool, Row, Sqlite};
use std::{future::Future, sync::Arc};
use tracing::info;

#[async_trait]
pub trait Database: Send + Sync {
    fn pool(&self) -> &SqlitePool;

    async fn init(&self) -> Result<(), AppError>;

    // User operations
    async fn create_user(&self, username: &str, auth_type: AuthType) -> Result<User, AppError>;
    async fn get_user(&self, user_id: &str) -> Result<Option<User>, AppError>;

    // User settings operations
    async fn get_user_settings(&self, user_id: &str) -> Result<Option<UserSettings>, AppError>;
    async fn update_user_settings(&self, settings: &UserSettings) -> Result<(), AppError>;

    // Mailbox operations
    async fn create_mailbox(&self, mailbox: &Mailbox) -> Result<(), AppError>;
    async fn get_mailbox(&self, mailbox_id: &str) -> Result<Option<Mailbox>, AppError>;
    async fn get_mailbox_by_address(&self, local_part: &str) -> Result<Option<Mailbox>, AppError>;
    async fn get_mailboxes_by_owner(&self, owner_id: &str) -> Result<Vec<Mailbox>, AppError>;
    async fn delete_mailbox(&self, mailbox_id: &str) -> Result<(), AppError>;
    async fn cleanup_expired_mailboxes(&self) -> Result<(), AppError>;
    async fn update_mailbox(&self, mailbox: &Mailbox) -> Result<(), AppError>;

    // Email operations
    async fn save_email(&self, email: &Email) -> Result<(), AppError>;
    async fn get_email(&self, email_id: &str) -> Result<Option<Email>, AppError>;
    async fn get_mailbox_emails(&self, mailbox_id: &str) -> Result<Vec<Email>, AppError>;
    async fn delete_email(&self, email_id: &str) -> Result<(), AppError>;
    async fn cleanup_expired_emails(&self) -> Result<(), AppError>;

    // API Key operations
    async fn create_api_key(&self, user_id: &str) -> Result<ApiKey, AppError>;
    async fn get_api_key(&self, key: &str) -> Result<Option<ApiKey>, AppError>;
    async fn delete_api_key(&self, key_id: &str) -> Result<(), AppError>;
}

pub struct SqliteDatabase {
    pool: SqlitePool,
}

impl SqliteDatabase {
    pub fn new_in_memory() -> impl Future<Output = Result<SqliteDatabase, AppError>> {
        Self::new("sqlite::memory:")
    }

    pub async fn new(database_url: &str) -> Result<Self, AppError> {
        let trimmed_db_url = database_url.trim();
        let filename = trimmed_db_url.trim_start_matches("sqlite:").to_string();
        let in_memory = filename == ":memory:";

        if filename != ":memory:" && !Sqlite::database_exists(database_url).await.unwrap_or(false) {
            info!("Creating database {}", filename);
            Sqlite::create_database(&filename)
                .await
                .map_err(|e| AppError::Database(format!("Failed to create database: {}", e)))?;
        } else if filename == ":memory:" {
            info!("Using in-memory database");
        }

        // Configure connection options for concurrent access
        let connect_options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(filename)
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal) // Use WAL mode for better concurrency
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal) // Balance between safety and performance
            .busy_timeout(std::time::Duration::from_secs(30)); // Wait up to 30 seconds if database is locked

        // In-memory database should have a single connection, otherwise we'll have multiple independent databases for each connection
        let max_connections = if in_memory { 1 } else { 10 };

        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(max_connections)
            .connect_with(connect_options)
            .await
            .map_err(|e| AppError::Database(format!("Failed to connect to database: {}", e)))?;

        let db = Self { pool };
        db.init().await?;
        Ok(db)
    }
}

#[async_trait]
impl Database for SqliteDatabase {
    fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    async fn init(&self) -> Result<(), AppError> {
        // Enable foreign key constraints
        sqlx::query("PRAGMA foreign_keys = ON;")
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::Database(format!("Failed to enable foreign key constraints: {}", e))
            })?;

        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to run migrations: {}", e)))?;

        Ok(())
    }

    async fn create_user(&self, username: &str, auth_type: AuthType) -> Result<User, AppError> {
        let user = User {
            id: uuid::Uuid::new_v4().to_string(),
            username: username.to_string(),
            auth_type,
            created_at: chrono::Utc::now().timestamp(),
        };

        sqlx::query("INSERT INTO users (id, username, auth_type, created_at) VALUES (?, ?, ?, ?)")
            .bind(&user.id)
            .bind(&user.username)
            .bind(&user.auth_type)
            .bind(user.created_at)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(user)
    }

    async fn get_user(&self, user_id: &str) -> Result<Option<User>, AppError> {
        let user = sqlx::query("SELECT * FROM users WHERE id = ?")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        match user {
            Some(row) => {
                let auth_type_str: String = row.get("auth_type");
                let auth_type: AuthType = match auth_type_str.as_str() {
                    "password" => AuthType::Password,
                    "github" => AuthType::GitHub,
                    "telegram" => AuthType::Telegram,
                    _ => return Err(AppError::Database("Invalid auth_type".to_string())),
                };

                Ok(Some(User {
                    id: row.get("id"),
                    username: row.get("username"),
                    auth_type,
                    created_at: row.get("created_at"),
                }))
            }
            None => Ok(None),
        }
    }

    async fn get_user_settings(&self, user_id: &str) -> Result<Option<UserSettings>, AppError> {
        let settings = sqlx::query("SELECT * FROM user_settings WHERE user_id = ?")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        match settings {
            Some(row) => Ok(Some(UserSettings {
                user_id: row.get("user_id"),
                email_notifications: row.get("email_notifications"),
                auto_delete_expired: row.get("auto_delete_expired"),
                default_mailbox_expiry: row.get("default_mailbox_expiry"),
            })),
            None => Ok(None),
        }
    }

    async fn update_user_settings(&self, settings: &UserSettings) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO user_settings (user_id, email_notifications, auto_delete_expired, default_mailbox_expiry)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(user_id) DO UPDATE SET
                email_notifications = excluded.email_notifications,
                auto_delete_expired = excluded.auto_delete_expired,
                default_mailbox_expiry = excluded.default_mailbox_expiry
            "#,
        )
        .bind(&settings.user_id)
        .bind(settings.email_notifications)
        .bind(settings.auto_delete_expired)
        .bind(settings.default_mailbox_expiry)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn create_mailbox(&self, mailbox: &Mailbox) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO mailboxes (id, alias, public_key, owner_id, created_at, expires_at) 
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&mailbox.id)
        .bind(&mailbox.alias)
        .bind(&mailbox.public_key)
        .bind(&mailbox.owner_id)
        .bind(mailbox.created_at)
        .bind(mailbox.expires_at)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn get_mailbox(&self, mailbox_id: &str) -> Result<Option<Mailbox>, AppError> {
        let mailbox = sqlx::query("SELECT * FROM mailboxes WHERE id = ?")
            .bind(mailbox_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        match mailbox {
            Some(row) => Ok(Some(Mailbox {
                id: row.get("id"),
                alias: row.get("alias"),
                public_key: row.get("public_key"),
                owner_id: row.get("owner_id"),
                created_at: row.get("created_at"),
                expires_at: row.get("expires_at"),
            })),
            None => Ok(None),
        }
    }

    async fn get_mailbox_by_address(&self, local_part: &str) -> Result<Option<Mailbox>, AppError> {
        let mailbox = sqlx::query("SELECT * FROM mailboxes WHERE alias = ?")
            .bind(local_part)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        match mailbox {
            Some(row) => Ok(Some(Mailbox {
                id: row.get("id"),
                alias: row.get("alias"),
                public_key: row.get("public_key"),
                owner_id: row.get("owner_id"),
                created_at: row.get("created_at"),
                expires_at: row.get("expires_at"),
            })),
            None => Ok(None),
        }
    }

    async fn get_mailboxes_by_owner(&self, owner_id: &str) -> Result<Vec<Mailbox>, AppError> {
        let mailboxes = sqlx::query("SELECT * FROM mailboxes WHERE owner_id = ?")
            .bind(owner_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(mailboxes
            .into_iter()
            .map(|row| Mailbox {
                id: row.get("id"),
                alias: row.get("alias"),
                public_key: row.get("public_key"),
                owner_id: row.get("owner_id"),
                created_at: row.get("created_at"),
                expires_at: row.get("expires_at"),
            })
            .collect())
    }

    async fn delete_mailbox(&self, mailbox_id: &str) -> Result<(), AppError> {
        sqlx::query("DELETE FROM mailboxes WHERE id = ?")
            .bind(mailbox_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn cleanup_expired_mailboxes(&self) -> Result<(), AppError> {
        let now = chrono::Utc::now().timestamp();
        sqlx::query("DELETE FROM mailboxes WHERE expires_at IS NOT NULL AND expires_at < ?")
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn update_mailbox(&self, mailbox: &Mailbox) -> Result<(), AppError> {
        sqlx::query("UPDATE mailboxes SET expires_at = ? WHERE id = ?")
            .bind(mailbox.expires_at)
            .bind(&mailbox.id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to update mailbox: {}", e)))?;

        Ok(())
    }

    async fn save_email(&self, email: &Email) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO emails (id, mailbox_id, encrypted_content, received_at, expires_at) 
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&email.id)
        .bind(&email.mailbox_id)
        .bind(&email.encrypted_content)
        .bind(email.received_at)
        .bind(email.expires_at)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn get_email(&self, email_id: &str) -> Result<Option<Email>, AppError> {
        let row = sqlx::query(
            "SELECT id, mailbox_id, encrypted_content, received_at, expires_at FROM emails WHERE id = ?"
        )
        .bind(email_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to get email: {}", e)))?;

        match row {
            Some(row) => Ok(Some(Email {
                id: row.get("id"),
                mailbox_id: row.get("mailbox_id"),
                encrypted_content: row.get("encrypted_content"),
                received_at: row.get("received_at"),
                expires_at: row.get("expires_at"),
            })),
            None => Ok(None),
        }
    }

    async fn get_mailbox_emails(&self, mailbox_id: &str) -> Result<Vec<Email>, AppError> {
        let emails = sqlx::query("SELECT * FROM emails WHERE mailbox_id = ?")
            .bind(mailbox_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(emails
            .into_iter()
            .map(|row| Email {
                id: row.get("id"),
                mailbox_id: row.get("mailbox_id"),
                encrypted_content: row.get("encrypted_content"),
                received_at: row.get("received_at"),
                expires_at: row.get("expires_at"),
            })
            .collect())
    }

    async fn delete_email(&self, email_id: &str) -> Result<(), AppError> {
        sqlx::query("DELETE FROM emails WHERE id = ?")
            .bind(email_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn cleanup_expired_emails(&self) -> Result<(), AppError> {
        let now = chrono::Utc::now().timestamp();
        sqlx::query("DELETE FROM emails WHERE expires_at IS NOT NULL AND expires_at < ?")
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn create_api_key(&self, user_id: &str) -> Result<ApiKey, AppError> {
        let api_key = ApiKey {
            id: uuid::Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            key: uuid::Uuid::new_v4().to_string(),
            created_at: chrono::Utc::now().timestamp(),
            expires_at: None,
        };

        sqlx::query(
            "INSERT INTO api_keys (id, user_id, key, created_at, expires_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&api_key.id)
        .bind(&api_key.user_id)
        .bind(&api_key.key)
        .bind(api_key.created_at)
        .bind(api_key.expires_at)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(api_key)
    }

    async fn get_api_key(&self, key: &str) -> Result<Option<ApiKey>, AppError> {
        let api_key = sqlx::query("SELECT * FROM api_keys WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        match api_key {
            Some(row) => Ok(Some(ApiKey {
                id: row.get("id"),
                user_id: row.get("user_id"),
                key: row.get("key"),
                created_at: row.get("created_at"),
                expires_at: row.get("expires_at"),
            })),
            None => Ok(None),
        }
    }

    async fn delete_api_key(&self, key_id: &str) -> Result<(), AppError> {
        sqlx::query("DELETE FROM api_keys WHERE id = ?")
            .bind(key_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }
}

#[async_trait]
impl<D: Database + ?Sized> Database for Arc<D> {
    fn pool(&self) -> &SqlitePool {
        (**self).pool()
    }

    async fn init(&self) -> Result<(), AppError> {
        (**self).init().await
    }

    async fn create_user(&self, username: &str, auth_type: AuthType) -> Result<User, AppError> {
        (**self).create_user(username, auth_type).await
    }

    async fn get_user(&self, user_id: &str) -> Result<Option<User>, AppError> {
        (**self).get_user(user_id).await
    }

    async fn get_user_settings(&self, user_id: &str) -> Result<Option<UserSettings>, AppError> {
        (**self).get_user_settings(user_id).await
    }

    async fn update_user_settings(&self, settings: &UserSettings) -> Result<(), AppError> {
        (**self).update_user_settings(settings).await
    }

    async fn create_mailbox(&self, mailbox: &Mailbox) -> Result<(), AppError> {
        (**self).create_mailbox(mailbox).await
    }

    async fn get_mailbox(&self, mailbox_id: &str) -> Result<Option<Mailbox>, AppError> {
        (**self).get_mailbox(mailbox_id).await
    }

    async fn get_mailbox_by_address(&self, local_part: &str) -> Result<Option<Mailbox>, AppError> {
        (**self).get_mailbox_by_address(local_part).await
    }

    async fn get_mailboxes_by_owner(&self, owner_id: &str) -> Result<Vec<Mailbox>, AppError> {
        (**self).get_mailboxes_by_owner(owner_id).await
    }

    async fn delete_mailbox(&self, mailbox_id: &str) -> Result<(), AppError> {
        (**self).delete_mailbox(mailbox_id).await
    }

    async fn cleanup_expired_mailboxes(&self) -> Result<(), AppError> {
        (**self).cleanup_expired_mailboxes().await
    }

    async fn update_mailbox(&self, mailbox: &Mailbox) -> Result<(), AppError> {
        (**self).update_mailbox(mailbox).await
    }

    async fn save_email(&self, email: &Email) -> Result<(), AppError> {
        (**self).save_email(email).await
    }

    async fn get_email(&self, email_id: &str) -> Result<Option<Email>, AppError> {
        (**self).get_email(email_id).await
    }

    async fn get_mailbox_emails(&self, mailbox_id: &str) -> Result<Vec<Email>, AppError> {
        (**self).get_mailbox_emails(mailbox_id).await
    }

    async fn delete_email(&self, email_id: &str) -> Result<(), AppError> {
        (**self).delete_email(email_id).await
    }

    async fn cleanup_expired_emails(&self) -> Result<(), AppError> {
        (**self).cleanup_expired_emails().await
    }

    async fn create_api_key(&self, user_id: &str) -> Result<ApiKey, AppError> {
        (**self).create_api_key(user_id).await
    }

    async fn get_api_key(&self, key: &str) -> Result<Option<ApiKey>, AppError> {
        (**self).get_api_key(key).await
    }

    async fn delete_api_key(&self, key_id: &str) -> Result<(), AppError> {
        (**self).delete_api_key(key_id).await
    }
}
