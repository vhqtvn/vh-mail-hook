-- Add migration script here
ALTER TABLE mailboxes ADD COLUMN name TEXT NOT NULL DEFAULT ''; 