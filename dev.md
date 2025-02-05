# VHMailHook

## Components

### MailService

- Core service responsible for receiving and routing emails to the appropriate mailboxes.
- Features:
    - Parses and processes incoming emails.
    - Encrypts emails using the mailbox's public key.
    - Deletes expired emails based on their expiration settings (mailboxes don't expire).

### Web Application
Interface for users and admins to interact with the system.

- **User Authentication**
    - Supports login via:
        - GitHub
        - Telegram
        - Password-based login (with options for 2FA)

- **Authenticated User Features**
    - **Mailbox Management:**
        - Create/delete secure mailboxes
        - Mailbox identification: A unique, random string appended to `@domain.com`
        - Security: Each mailbox has:
            - A secure public key for email encryption
            - Configurable expiration for emails (mailboxes do not expire)
    - **Settings Management:**
        - Update password
        - Generate or revoke API keys for automated mailbox interaction

- **Admin Interface**
    - Manage core system resources:
        - Users
        - Domains
        - Mailboxes
    - Configure integrations:
        - Google OAuth
        - Telegram Auth

- **API Endpoints**
    - **Authentication:**
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
    
    - **Mailbox Management:**
        - `GET /api/mailboxes` - List all mailboxes for current user
        - `POST /api/mailboxes` - Create a new mailbox
        - `GET /api/mailboxes/:id` - Get mailbox details
        - `DELETE /api/mailboxes/:id` - Delete a mailbox
        - `PATCH /api/mailboxes/:id` - Update mailbox settings
        - `GET /api/mailboxes/:id/emails` - List emails in mailbox
        - `GET /api/mailboxes/:id/emails/:email_id` - Get email details
        - `DELETE /api/mailboxes/:id/emails/:email_id` - Delete an email
    
    - **System:**
        - `GET /api/supported-domains` - List supported email domains

## Frontend Development
To run the frontend in development mode, ensure that Node.js and pnpm are installed.
Navigate to the frontend directory:

```bash
cd crates/web-app/frontend
pnpm dev
```

The development server typically runs on port 5173.

### Runtime Configuration
The frontend application uses runtime configuration injected by the Rust backend. This means:
1. Configuration values (like Telegram Bot Name, OAuth settings) are managed by the backend
2. No frontend environment files are needed
3. Configuration can be changed without rebuilding the frontend
4. The backend must be running to provide the configuration

When developing locally:
1. Set environment variables in your backend environment (`.env` file or system environment)
2. The backend will inject these values into the frontend at runtime
3. Changes to environment variables take effect after restarting the backend server

## Key Features
- **Security by Design:**
    - Emails are encrypted upon receipt
    - Each mailbox has isolated and unique credentials
- **Resource Management:**
    - Expiration policies for emails to ensure cleanup and efficient resource usage
- **Flexibility:**
    - Multiple authentication methods
    - API for automation and advanced usage
