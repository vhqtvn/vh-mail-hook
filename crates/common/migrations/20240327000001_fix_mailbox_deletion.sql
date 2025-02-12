-- Fix foreign key constraint for mailbox deletion
-- First, drop the existing foreign key constraint
DROP INDEX IF EXISTS idx_emails_mailbox;

-- Create a temporary table without the constraint
CREATE TABLE emails_temp (
    id TEXT PRIMARY KEY,
    mailbox_id TEXT NOT NULL,
    encrypted_content TEXT NOT NULL,
    received_at INTEGER NOT NULL,
    expires_at INTEGER
);

-- Copy data from the original table
INSERT INTO emails_temp SELECT * FROM emails;

-- Drop the original table
DROP TABLE emails;

-- Create the new table with the CASCADE constraint
CREATE TABLE emails (
    id TEXT PRIMARY KEY,
    mailbox_id TEXT NOT NULL,
    encrypted_content TEXT NOT NULL,
    received_at INTEGER NOT NULL,
    expires_at INTEGER,
    FOREIGN KEY(mailbox_id) REFERENCES mailboxes(id) ON DELETE CASCADE
);

-- Copy data back
INSERT INTO emails SELECT * FROM emails_temp;

-- Drop the temporary table
DROP TABLE emails_temp;

-- Recreate the index
CREATE INDEX idx_emails_mailbox ON emails(mailbox_id); 