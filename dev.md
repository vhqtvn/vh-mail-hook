# VHMailHook - Developer Documentation

This document serves as a guide for developers working on the VHMailHook project. It covers system components, API endpoints, frontend development, and key features.

## Components

### Mail Service
- Responsible for receiving, processing, and encrypting emails.
- Key tasks:
  - Parsing incoming emails.
  - Encrypting emails using each mailbox's public key.
  - Automatically deleting emails based on retention policies.

### Web Application
Provides the interface for both users and administrators.
- **User Authentication:** Supports login via GitHub, Telegram, and password (with optional 2FA).
- **Mailbox Management:**
  - Create and delete secure mailboxes.
  - Each mailbox is uniquely identified with a random string and secured with its own encryption key.
- **Settings Management:**
  - Update passwords.
  - Generate or revoke API keys.
- **Administration Interface:**
  - Manage users, domains, and mailboxes.
  - Configure integrations such as Google OAuth and Telegram authentication.

## API Endpoints

### Authentication
- POST /api/auth/register — Register a new account.
- POST /api/auth/login — Login with username and password.
- GET /api/auth/github/login — Initiate GitHub OAuth flow.
- GET /api/auth/github/callback — Handle GitHub OAuth callback.
- GET /api/auth/google/login — Initiate Google OAuth flow.
- GET /api/auth/google/callback — Handle Google OAuth callback.
- POST /api/auth/telegram/verify — Verify Telegram login data.
- GET /api/auth/me — Retrieve current user information.
- GET /api/auth/connected-accounts — List connected authentication methods.
- POST /api/auth/delete-account — Delete a user account.
- POST /api/auth/set-password — Update account password.
- POST /api/auth/telegram/disconnect — Disconnect Telegram integration.
- POST /api/auth/google/disconnect — Disconnect Google integration.
- POST /api/auth/github/disconnect — Disconnect GitHub integration.

### Mailbox Management
- GET /api/mailboxes — List all user mailboxes.
- POST /api/mailboxes — Create a new mailbox.
- GET /api/mailboxes/:id — Retrieve mailbox details.
- DELETE /api/mailboxes/:id — Delete a mailbox.
- PATCH /api/mailboxes/:id — Update mailbox settings.
- GET /api/mailboxes/:id/emails — List emails in a mailbox.
- GET /api/mailboxes/:id/emails/:email_id — Retrieve details of an email.
- DELETE /api/mailboxes/:id/emails/:email_id — Delete an email.

### System
- GET /api/supported-domains — List supported email domains.

## Frontend Development

For frontend development, ensure Node.js and pnpm are installed.

1. Navigate to the frontend directory:
   ```bash
   cd crates/web-app/frontend
   pnpm dev
   ```
2. The development server typically runs on port 5173.
3. The Rust backend injects runtime configuration values (e.g., OAuth settings, Telegram Bot Name) into the frontend, so ensure the backend is running when developing.

## Key Features

- **Security by Design:**
  - All emails are encrypted upon receipt using robust encryption methods.
  - Each mailbox utilizes its own public key for encryption.

- **Resource Management:**
  - Automated email expiration and cleanup based on retention policies.

- **Flexibility & Integration:**
  - Supports multiple authentication methods, including 2FA.
  - Provides a comprehensive API for seamless integration and automation.

## Additional Notes

- Database: The project uses SQLite as its primary database. Ensure SQLite3 and libsqlite3-dev are installed and configured as described in the README.

This documentation is intended for developers contributing to VHMailHook. For further setup and configuration details, refer to the main README file.
