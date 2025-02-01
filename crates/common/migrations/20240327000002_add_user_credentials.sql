-- Add user_credentials table
CREATE TABLE IF NOT EXISTS user_credentials (
    user_id TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    password_hash TEXT,
    oauth_provider TEXT,
    oauth_id TEXT,
    telegram_id TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- Add index for oauth lookups
CREATE INDEX IF NOT EXISTS idx_user_credentials_oauth 
ON user_credentials(oauth_provider, oauth_id)
WHERE oauth_provider IS NOT NULL AND oauth_id IS NOT NULL;

-- Add index for telegram lookups
CREATE INDEX IF NOT EXISTS idx_user_credentials_telegram
ON user_credentials(telegram_id)
WHERE telegram_id IS NOT NULL;

-- Add sessions table for managing user sessions
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL
);

-- Add index for session cleanup
CREATE INDEX IF NOT EXISTS idx_sessions_expires_at
ON sessions(expires_at);

-- Add index for user sessions lookup
CREATE INDEX IF NOT EXISTS idx_sessions_user_id
ON sessions(user_id); 