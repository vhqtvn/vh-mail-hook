-- Add migration script here
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    auth_type TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS mailboxes (
    id TEXT PRIMARY KEY,
    address TEXT NOT NULL UNIQUE,
    public_key TEXT NOT NULL,
    owner_id TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    expires_at INTEGER,
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