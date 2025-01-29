# VHMailHook

A secure email handling system with temporary mailboxes and encryption.

## Components

### Mail Service
- Core service for receiving and routing emails
- Handles email encryption and expiration
- Automatic cleanup of expired mailboxes and emails

### Web Application
- RESTful API for mailbox management
- User authentication (GitHub, Telegram, Password)
- Admin interface for system management

## Setup

1. Install Rust (latest stable version)
2. Clone the repository
3. Build the project:
```bash
cargo build
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

## License

MIT License 