# VHMailHook Examples

This directory contains example code demonstrating how to use VHMailHook's features.

## Node.js Examples

The `node` directory contains examples using Node.js to interact with VHMailHook.

### Prerequisites

- Node.js 16.x or later
- npm or pnpm package manager
- A VHMailHook account with API access
- AGE encryption keys

### Setup

1. Navigate to the node example directory:
```bash
cd node
```

2. Install dependencies:
```bash
npm install
# or
pnpm install
```

3. Set up environment variables:
```bash
cp .env.example .env
```

Edit the `.env` file with your credentials:
```bash
DOMAIN=https://your-vhmailhook-instance.com
API_KEY=your_api_key
MAILBOX_ID=your_mailbox_id
PRIVATE_KEY=your_age_private_key
```

### Examples

#### 1. Read Encrypted Emails (read-email.js)

This example demonstrates how to:
- Connect to the VHMailHook API
- List emails in a mailbox
- Decrypt emails using AGE encryption
- Handle the decrypted content

Usage:
```bash
node read-email.js
```

Key concepts demonstrated:
- API authentication
- Email retrieval
- AGE decryption
- Error handling

#### 2. Create Mailbox (create-mailbox.js) [Coming Soon]

This example will show how to:
- Create a new mailbox
- Generate AGE key pairs
- Configure mailbox settings

#### 3. Monitor Mailbox (monitor-mailbox.js) [Coming Soon]

This example will demonstrate:
- Real-time email monitoring
- Webhook integration
- Event handling

## Contributing

Feel free to contribute additional examples! Please follow these guidelines:
1. Create a new directory for different languages/frameworks
2. Include clear documentation and setup instructions
3. Add error handling and best practices
4. Keep examples focused and concise

## Security Notes

- Never commit real API keys or private keys
- Use environment variables for sensitive data
- Follow security best practices in your implementations 