> **Note**: This entire project, including all code, documentation, and configuration, is 100% AI-generated.

# VHMailHook

A secure email handling system with temporary mailboxes and encryption.

## Components

### Mail Service
- Core service for receiving and routing emails
- Handles email encryption and expiration
- Automatic cleanup of expired emails (mailboxes don't expire)
- Built-in rate limiting for API endpoints and SMTP connections

### Web Application
- RESTful API for mailbox management
- User authentication (GitHub, Telegram, Password)
- Admin interface for system management
- Rate-limited API access

## System Requirements

- Rust (1.70 or later)
- Node.js (16.x or later) for frontend development
- PostgreSQL (14.x or later)
- Linux/Unix-based system for SMTP service
- 1GB RAM minimum (2GB recommended)
- 20GB storage space recommended for email storage

## Setup

1. Install Rust (latest stable version)
2. Clone the repository
3. Install system dependencies:
   ```bash
   # Ubuntu/Debian
   sudo apt-get install postgresql libpq-dev
   ```
4. Build the project:
   ```bash
   cargo build
   ```

## Configuration

### Mail Service Configuration
Create a `.env` file in the mail-service directory:
```bash
# SMTP Configuration
SMTP_PORT=25
SMTP_HOST=0.0.0.0
SMTP_DOMAIN=your-domain.com

# Storage Configuration
STORAGE_PATH=/path/to/email/storage

# Email Settings
MAX_EMAIL_SIZE=10485760  # 10MB
EMAIL_RETENTION_DAYS=30
CLEANUP_INTERVAL_HOURS=24

# Rate Limiting
SMTP_RATE_LIMIT=100  # Connections per minute
API_RATE_LIMIT=1000  # Requests per hour
```

### Web Application Configuration
Create a `.env` file in the web-app directory:
```bash
# Server Configuration
PORT=3000
HOST=0.0.0.0

# Database
DATABASE_URL=postgresql://user:password@localhost/vhmailhook

# Rate Limiting
API_RATE_LIMIT_WINDOW=3600  # 1 hour in seconds
API_RATE_LIMIT_MAX_REQUESTS=1000
```

## Running the Services

### Mail Service
```bash
cargo run -p mail-service
```

### Web Application
```bash
cargo run -p web-app
```

The web application will be available at `http://localhost:3000`.

## Development

The project uses a workspace structure with the following crates:
- `common`: Shared types and utilities
- `mail-service`: Email processing service
- `web-app`: Web application and API

## API Endpoints

### Authentication
- `POST /api/auth/register` - Register with username/password
- `POST /api/auth/login` - Login with username/password
- `GET /api/auth/github/login` - Start GitHub OAuth flow
- `GET /api/auth/github/callback` - GitHub OAuth callback
- `GET /api/auth/google/login` - Start Google OAuth flow
- `GET /api/auth/google/callback` - Google OAuth callback
- `POST /api/auth/telegram/verify` - Verify Telegram login widget data
- `GET /api/auth/me` - Get current user info
- `GET /api/auth/connected-accounts` - List connected authentication methods
- `POST /api/auth/delete-account` - Delete user account
- `POST /api/auth/set-password` - Set password for account
- `POST /api/auth/telegram/disconnect` - Disconnect Telegram account
- `POST /api/auth/google/disconnect` - Disconnect Google account
- `POST /api/auth/github/disconnect` - Disconnect GitHub account

### Mailboxes
- `GET /api/mailboxes` - List all mailboxes for current user
- `POST /api/mailboxes` - Create a new mailbox
- `GET /api/mailboxes/:id` - Get mailbox details
- `DELETE /api/mailboxes/:id` - Delete a mailbox
- `PATCH /api/mailboxes/:id` - Update mailbox settings
- `GET /api/mailboxes/:id/emails` - List emails in mailbox
- `GET /api/mailboxes/:id/emails/:email_id` - Get email details
- `DELETE /api/mailboxes/:id/emails/:email_id` - Delete an email

### System
- `GET /api/supported-domains` - List supported email domains

## License

MIT License

## Authentication Setup

The application supports multiple authentication methods:

1. Password-based authentication
2. OAuth providers (GitHub and Google)
3. Telegram Login Widget

### Environment Variables

Set the following environment variables for authentication:

```bash
# JWT Configuration
JWT_SECRET=your-256-bit-secret

# GitHub OAuth
GITHUB_CLIENT_ID=your-github-client-id
GITHUB_CLIENT_SECRET=your-github-client-secret

# Google OAuth
GOOGLE_CLIENT_ID=your-google-client-id
GOOGLE_CLIENT_SECRET=your-google-client-secret

# Telegram Login Widget
TELEGRAM_BOT_TOKEN=your-telegram-bot-token
TELEGRAM_BOT_NAME=your-telegram-bot-name

# Application URL (for OAuth callbacks)
APP_URL=http://localhost:3000
```

### Setting Up OAuth Providers

1. GitHub OAuth:
   - Go to GitHub Developer Settings
   - Create a new OAuth App
   - Set the callback URL to `{APP_URL}/auth/github/callback`
   - Copy the Client ID and Client Secret

2. Google OAuth:
   - Go to Google Cloud Console
   - Create a new project
   - Enable the OAuth consent screen
   - Create OAuth 2.0 credentials
   - Set the authorized redirect URI to `{APP_URL}/auth/google/callback`
   - Copy the Client ID and Client Secret

3. Telegram Login Widget:
   - Create a bot using @BotFather
   - Get the bot token
   - Configure the login widget domain in @BotFather
   - Add the Telegram Login Widget to your frontend:
   ```html
   <script async src="https://telegram.org/js/telegram-widget.js?22" 
           data-telegram-login="YOUR_BOT_NAME" 
           data-size="large" 
           data-auth-url="{APP_URL}/auth/telegram/verify"
           data-request-access="write">
   </script>
   ```

### Setting Up Telegram Authentication

1. Create a new bot using [@BotFather](https://t.me/botfather) on Telegram:
   - Send `/newbot` to @BotFather
   - Choose a name for your bot (e.g. "VH Mail Hook")
   - Choose a username for your bot (e.g. "vh_mail_hook_bot")
   - Save the bot token provided by @BotFather

2. Configure the bot for website authentication:
   - Send `/setdomain` to @BotFather
   - Select your bot
   - Enter your website domain (e.g. `example.com`)

3. Set the required environment variables:
   ```bash
   # Telegram Bot Configuration
   TELEGRAM_BOT_TOKEN=your_bot_token_from_botfather
   TELEGRAM_BOT_NAME=your_bot_username  # This will be injected into the frontend at runtime
   ```

### Frontend Development

The frontend uses runtime configuration injected by the Rust backend. No frontend environment files are needed.
To run the frontend in development mode, ensure you have Node.js and pnpm installed. Navigate to the frontend directory:

```bash
cd crates/web-app/frontend
pnpm dev
```

The development server typically runs on port 5173. 

Note: When running in development mode, make sure the backend is also running as it provides the necessary runtime configuration. 

## Security and Encryption

VHMailHook uses the AGE (Actually Good Encryption) system for email encryption:

- All emails are encrypted at rest using AGE
- Each mailbox has its own encryption key pair
- Private keys are never stored on the server
- Supports both X25519 and SSH keys
- Encrypted emails can only be decrypted with the mailbox's private key

### Email Lifecycle

1. **Reception**: Emails are received via SMTP
2. **Processing**: 
   - Headers and content are parsed
   - Attachments are processed
   - Content is encrypted using the mailbox's public key
3. **Storage**: 
   - Encrypted content is stored
   - Original email is securely deleted
4. **Expiration**:
   - Emails automatically expire after the configured retention period
   - Expired emails are permanently deleted during cleanup
   - Mailboxes remain until explicitly deleted

### Rate Limiting

The system implements rate limiting at multiple levels:

1. **SMTP Connections**:
   - Limits incoming SMTP connections per IP
   - Configurable window and maximum connections
   - Prevents spam and DoS attacks

2. **API Endpoints**:
   - Per-user and per-IP rate limiting
   - Separate limits for authentication endpoints
   - Configurable limits for different endpoint categories

3. **Storage Quotas**:
   - Maximum email size limits
   - Per-mailbox storage quotas
   - System-wide storage monitoring

## Additional Notes

- The system is designed to handle a large number of users and emails efficiently.
- The rate limiting features help prevent abuse and ensure fair usage.
- The encryption system ensures that emails are secure and confidential.
- The system is scalable and can be deployed in various environments.

## Conclusion

VHMailHook provides a secure, efficient, and user-friendly email handling system. It supports multiple authentication methods, rate limiting, and encryption.

## License

MIT License

## Authentication Setup

The application supports multiple authentication methods:

1. Password-based authentication
2. OAuth providers (GitHub and Google)
3. Telegram Login Widget

### Environment Variables

Set the following environment variables for authentication:

```bash
# JWT Configuration
JWT_SECRET=your-256-bit-secret

# GitHub OAuth
GITHUB_CLIENT_ID=your-github-client-id
GITHUB_CLIENT_SECRET=your-github-client-secret

# Google OAuth
GOOGLE_CLIENT_ID=your-google-client-id
GOOGLE_CLIENT_SECRET=your-google-client-secret

# Telegram Login Widget
TELEGRAM_BOT_TOKEN=your-telegram-bot-token
TELEGRAM_BOT_NAME=your-telegram-bot-name

# Application URL (for OAuth callbacks)
APP_URL=http://localhost:3000
```

### Setting Up OAuth Providers

1. GitHub OAuth:
   - Go to GitHub Developer Settings
   - Create a new OAuth App
   - Set the callback URL to `{APP_URL}/auth/github/callback`
   - Copy the Client ID and Client Secret

2. Google OAuth:
   - Go to Google Cloud Console
   - Create a new project
   - Enable the OAuth consent screen
   - Create OAuth 2.0 credentials
   - Set the authorized redirect URI to `{APP_URL}/auth/google/callback`
   - Copy the Client ID and Client Secret

3. Telegram Login Widget:
   - Create a bot using @BotFather
   - Get the bot token
   - Configure the login widget domain in @BotFather
   - Add the Telegram Login Widget to your frontend:
   ```html
   <script async src="https://telegram.org/js/telegram-widget.js?22" 
           data-telegram-login="YOUR_BOT_NAME" 
           data-size="large" 
           data-auth-url="{APP_URL}/auth/telegram/verify"
           data-request-access="write">
   </script>
   ```

### Setting Up Telegram Authentication

1. Create a new bot using [@BotFather](https://t.me/botfather) on Telegram:
   - Send `/newbot` to @BotFather
   - Choose a name for your bot (e.g. "VH Mail Hook")
   - Choose a username for your bot (e.g. "vh_mail_hook_bot")
   - Save the bot token provided by @BotFather

2. Configure the bot for website authentication:
   - Send `/setdomain` to @BotFather
   - Select your bot
   - Enter your website domain (e.g. `example.com`)

3. Set the required environment variables:
   ```bash
   # Telegram Bot Configuration
   TELEGRAM_BOT_TOKEN=your_bot_token_from_botfather
   TELEGRAM_BOT_NAME=your_bot_username  # This will be injected into the frontend at runtime
   ```

### Frontend Development

The frontend uses runtime configuration injected by the Rust backend. No frontend environment files are needed.
To run the frontend in development mode, ensure you have Node.js and pnpm installed. Navigate to the frontend directory:

```bash
cd crates/web-app/frontend
pnpm dev
```

The development server typically runs on port 5173. 

Note: When running in development mode, make sure the backend is also running as it provides the necessary runtime configuration. 