-- Add migration script here
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    auth_type TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS user_credentials (
    user_id TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    password_hash TEXT,
    oauth_provider TEXT,
    oauth_id TEXT,
    telegram_id TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS user_settings (
    user_id TEXT PRIMARY KEY,
    email_notifications BOOLEAN NOT NULL DEFAULT true,
    auto_delete_expired BOOLEAN NOT NULL DEFAULT true,
    default_mailbox_expiry INTEGER,
    FOREIGN KEY(user_id) REFERENCES users(id)
);

CREATE TABLE IF NOT EXISTS mailboxes (
    id TEXT PRIMARY KEY,
    alias TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL DEFAULT '',
    public_key TEXT NOT NULL,
    owner_id TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    mail_expires_in INTEGER,
    FOREIGN KEY(owner_id) REFERENCES users(id)
);

CREATE TABLE IF NOT EXISTS emails (
    id TEXT PRIMARY KEY,
    mailbox_id TEXT NOT NULL,
    encrypted_content TEXT NOT NULL,
    received_at INTEGER NOT NULL,
    expires_at INTEGER,
    FOREIGN KEY(mailbox_id) REFERENCES mailboxes(id)
);

CREATE TABLE IF NOT EXISTS api_keys (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    key TEXT NOT NULL UNIQUE,
    created_at INTEGER NOT NULL,
    expires_at INTEGER,
    FOREIGN KEY(user_id) REFERENCES users(id)
);

CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL
);

-- Create indexes for better performance
CREATE INDEX IF NOT EXISTS idx_mailboxes_owner ON mailboxes(owner_id);
CREATE INDEX IF NOT EXISTS idx_emails_mailbox ON emails(mailbox_id);
CREATE INDEX IF NOT EXISTS idx_api_keys_user ON api_keys(user_id);

CREATE INDEX IF NOT EXISTS idx_user_credentials_oauth 
ON user_credentials(oauth_provider, oauth_id)
WHERE oauth_provider IS NOT NULL AND oauth_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_user_credentials_telegram
ON user_credentials(telegram_id)
WHERE telegram_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_sessions_expires_at
ON sessions(expires_at);

CREATE INDEX IF NOT EXISTS idx_sessions_user_id
ON sessions(user_id); 