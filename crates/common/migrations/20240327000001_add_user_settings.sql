-- Add migration script here
CREATE TABLE IF NOT EXISTS user_settings (
    user_id TEXT PRIMARY KEY,
    email_notifications BOOLEAN NOT NULL DEFAULT true,
    auto_delete_expired BOOLEAN NOT NULL DEFAULT true,
    default_mailbox_expiry INTEGER,
    FOREIGN KEY(user_id) REFERENCES users(id)
);

-- Add indexes for better performance
CREATE INDEX IF NOT EXISTS idx_mailboxes_owner ON mailboxes(owner_id);
CREATE INDEX IF NOT EXISTS idx_emails_mailbox ON emails(mailbox_id);
CREATE INDEX IF NOT EXISTS idx_api_keys_user ON api_keys(user_id); 