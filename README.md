> **Note**: This project, including all code, documentation, and configuration, is 100% AI-generated.

# VHMailHook

VHMailHook is a secure email handling system that provides temporary mailboxes and robust encryption for protecting your emails.

## System Overview

VHMailHook consists of two main components:

- **Mail Service**: Handles reception, processing, and encryption of emails. It manages incoming emails, encrypts them with mailbox-specific keys, enforces rate limits, and automatically cleans up expired emails. Features flexible mailbox matching and enhanced email processing resilience.
- **Web Application**: Provides a RESTful API for mailbox management, supports multiple authentication methods (GitHub, Telegram, password-based), and offers an admin interface for system configuration. Includes a modern two-column layout for efficient email management.

## Features

- **Secure Email Handling**: End-to-end encryption using AGE for all stored emails
- **Flexible Mailbox Matching**: Smart email address matching with local part normalization
- **Modern Web Interface**: Intuitive two-column layout for efficient email management
- **Client Library**: Official NPM package for easy integration
- **Multiple Authentication Methods**: Support for GitHub, Google, Telegram, and password-based auth
- **Robust Email Processing**: Enhanced SMTP handling with improved resilience
- **Automated Cleanup**: Configurable retention policies for emails

## Requirements

- **Rust**: Version 1.70 or later
- **Node.js**: Version 16.x or later (for frontend development)
- **SQLite**: Ensure SQLite3 and libsqlite3-dev are installed
- **System**: Linux/Unix-based (for SMTP service)
- **Memory**: Minimum 1GB RAM (2GB recommended)
- **Storage**: 20GB space recommended for email archives

## Setup

### Installation

1. Install Rust (latest stable version).
2. Clone the repository.
3. Install system dependencies. For Ubuntu/Debian:
   ```bash
   sudo apt-get install sqlite3 libsqlite3-dev
   ```
4. Build the project:
   ```bash
   cargo build
   ```

### Configuration

#### Mail Service

Create a `.env` file in the mail-service directory with the following:

```bash
# SMTP Configuration
SMTP_PORT=25
SMTP_HOST=0.0.0.0
SMTP_DOMAIN=your-domain.com

# Storage Path
STORAGE_PATH=/path/to/email/storage

# Email Settings
MAX_EMAIL_SIZE=10485760  # 10MB maximum size
EMAIL_RETENTION_DAYS=30
CLEANUP_INTERVAL_HOURS=24

# Rate Limiting
SMTP_RATE_LIMIT=100  # per minute
API_RATE_LIMIT=1000  # per hour
```

#### Web Application

Create a `.env` file in the web-app directory with:

```bash
# Server Configuration
PORT=3000
HOST=0.0.0.0

# Database Connection
DATABASE_URL=sqlite://vhmailhook.db

# API Rate Limiting
API_RATE_LIMIT_WINDOW=3600  # in seconds (1 hour)
API_RATE_LIMIT_MAX_REQUESTS=1000
```

## Running the Services

### Mail Service

Run the mail service using:

```bash
cargo run -p mail-service
```

### Web Application

Run the web application using:

```bash
cargo run -p web-app
```

The web app will be accessible at [http://localhost:3000](http://localhost:3000).

## Project Structure

The workspace is organized into several crates:
- `common`: Shared utilities and types.
- `mail-service`: Responsible for email processing.
- `web-app`: Provides the web interface and API.

## API Endpoints

### Authentication
- POST /api/auth/register — Register an account.
- POST /api/auth/login — Login using username/password.
- GET /api/auth/github/login — Start GitHub OAuth.
- GET /api/auth/github/callback — GitHub OAuth callback.
- GET /api/auth/google/login — Start Google OAuth.
- GET /api/auth/google/callback — Google OAuth callback.
- POST /api/auth/telegram/verify — Verify Telegram login.
- GET /api/auth/me — Get current user info.
- GET /api/auth/connected-accounts — List linked auth methods.
- POST /api/auth/delete-account — Remove an account.
- POST /api/auth/set-password — Set or update the password.
- POST /api/auth/telegram/disconnect — Disconnect Telegram integration.
- POST /api/auth/google/disconnect — Disconnect Google integration.
- POST /api/auth/github/disconnect — Disconnect GitHub integration.

### Mailboxes
- GET /api/mailboxes — List user mailboxes.
- POST /api/mailboxes — Create a new mailbox.
- GET /api/mailboxes/:id — Get mailbox details.
- DELETE /api/mailboxes/:id — Delete a mailbox.
- PATCH /api/mailboxes/:id — Update mailbox settings.
- GET /api/mailboxes/:id/emails — List emails in a mailbox.
- GET /api/mailboxes/:id/emails/:email_id — Get details of an email.
- DELETE /api/mailboxes/:id/emails/:email_id — Delete an email.

### System
- GET /api/supported-domains — List supported email domains.

## Authentication Setup

VHMailHook supports multiple authentication methods:

1. **Password-based Authentication**
2. **OAuth Providers**: GitHub and Google.
3. **Telegram Login Widget**

### Environment Variables for Authentication

```bash
# JWT Token
JWT_SECRET=your-256-bit-secret

# GitHub OAuth
GITHUB_CLIENT_ID=your-github-client-id
GITHUB_CLIENT_SECRET=your-github-client-secret

# Google OAuth
GOOGLE_CLIENT_ID=your-google-client-id
GOOGLE_CLIENT_SECRET=your-google-client-secret

# Telegram
TELEGRAM_BOT_TOKEN=your-telegram-bot-token
TELEGRAM_BOT_NAME=your-telegram-bot-name

# Application URL
APP_URL=http://localhost:3000
```

### OAuth Provider Configuration

1. **GitHub OAuth**
   - Create an OAuth App in GitHub Developer Settings.
   - Set the callback URL to `{APP_URL}/auth/github/callback`.
   - Use the provided Client ID and Client Secret.

2. **Google OAuth**
   - Set up a project and enable OAuth in Google Cloud Console.
   - Create OAuth credentials with redirect URI `{APP_URL}/auth/google/callback`.

3. **Telegram Login**
   - Create a Telegram bot with @BotFather.
   - Configure the login widget using:
     ```html
     <script async src="https://telegram.org/js/telegram-widget.js?22" 
             data-telegram-login="YOUR_BOT_NAME" 
             data-size="large" 
             data-auth-url="{APP_URL}/auth/telegram/verify"
             data-request-access="write">
     </script>
     ```

## Security & Encryption

VHMailHook uses AGE encryption to secure emails:
- Each email is encrypted using a unique key tied to its mailbox.
- Private keys are never stored on the server.
- Supports both X25519 and SSH keys.
- Email lifecycle includes reception, parsing, encryption, and secure retention.

## Docker

This repository automatically builds the Docker image: vanhoavn/vh-mail-hook.

You can simplify running the application using Docker. For example:

    docker run -d --name vh-mail-hook vanhoavn/vh-mail-hook

## License

This project is licensed under the MIT License.

## Client Library

VHMailHook provides an official NPM package for easy integration with your JavaScript/TypeScript projects:

```bash
npm install @vhmailhook/client
```

Example usage:

```typescript
import { VHMailHook } from '@vhmailhook/client';

const client = new VHMailHook({
  baseUrl: 'https://your-instance.com'
});

// Get emails from a mailbox
const emails = await client.getEmails('your-mailbox-id');

// Get emails with automatic decryption
const decryptedEmails = await client.getEmails('your-mailbox-id', 'your-age-private-key');

// Delete an email
await client.deleteEmail('your-mailbox-id', 'email-id');
```

For detailed client library documentation, visit the [NPM package documentation](packages/npm/README.md).