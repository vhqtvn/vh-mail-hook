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
    - Password-based login (with options for 2FA).

- **Authenticated User Features**
    - **Mailbox Management:**
        - Create/delete secure mailboxes.
        - Mailbox identification: A unique, random string appended to `@domain.com`.
        - Security: Each mailbox has:
            - A secure public key for email encryption.
            - Configurable expiration for emails (mailboxes do not expire).
    - **Settings Management:**
        - Update password.
        - Generate or revoke API keys for automated mailbox interaction.

- **Admin Interface**
    - Manage core system resources:
        - Users.
        - Domains.
        - Mailboxes.
    - Configure integrations:
        - Google OAuth.
        - Telegram Auth.

- **API**
    - Provides programmatic access to:
        - Create/delete mailboxes.
        - Read mailbox content.
        - Long-polling for new emails in a mailbox.
        - Delete individual emails from a mailbox.

## Frontend Development
To run the frontend in development mode, ensure that Node.js and pnpm are installed.
Navigate to the frontend directory:

  cd crates/web-app/frontend
  pnpm dev

The development server typically runs on port 5173.

## Key Features
- **Security by Design**:
  - Emails are encrypted upon receipt.
  - Each mailbox has isolated and unique credentials.
- **Resource Management**:
  - Expiration policies for emails to ensure cleanup and efficient resource usage.
- **Flexibility**:
  - Multiple authentication methods.
  - API for automation and advanced usage.
