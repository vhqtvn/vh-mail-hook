# VHMailHook TypeScript Client

A TypeScript client library for interacting with the VHMailHook service. This library provides a simple interface for retrieving and decrypting emails from VHMailHook mailboxes.

## Installation

```bash
npm install vhmailhook
# or
yarn add vhmailhook
```

## Usage

```typescript
import { VHMailHookClient } from 'vhmailhook';

// Initialize the client
const client = new VHMailHookClient({
  apiKey: 'your-api-key',
  // Optional: override the default domain
  // domain: 'https://your-custom-domain.com'
});

// Get emails without decryption
const emails = await client.getEmails('your-mailbox-id');
console.log(emails);
// [
//   {
//     id: 'email-id',
//     received_at: 1234567890,
//     expires_at: 1234567890,
//     encrypted_content: 'base64-encoded-encrypted-content'
//   }
// ]

// Get emails with automatic decryption
const decryptedEmails = await client.getEmails('your-mailbox-id', 'your-age-private-key');
console.log(decryptedEmails);
// [
//   {
//     id: 'email-id',
//     received_at: 1234567890,
//     expires_at: 1234567890,
//     content: 'decrypted-email-content'
//   }
// ]

// Delete an email
await client.deleteEmail('your-mailbox-id', 'email-id');
```

## API Reference

### `VHMailHookClient`

The main client class for interacting with the VHMailHook service.

#### Constructor

```typescript
new VHMailHookClient(config: VHMailHookConfig)
```

- `config.apiKey` (required): Your VHMailHook API key
- `config.domain` (optional): Override the default VHMailHook domain

#### Methods

##### `getEmails(mailboxId: string, privateKey?: string): Promise<Email[] | DecryptedEmail[]>`

Retrieves emails from a mailbox. If a private key is provided, the emails will be automatically decrypted.

- `mailboxId`: The ID of the mailbox to retrieve emails from
- `privateKey` (optional): AGE private key for decrypting emails

##### `deleteEmail(mailboxId: string, emailId: string): Promise<void>`

Deletes an email from a mailbox.

- `mailboxId`: The ID of the mailbox containing the email
- `emailId`: The ID of the email to delete

## Types

The library exports the following TypeScript types:

- `VHMailHookConfig`
- `Email`
- `DecryptedEmail`
- `APIResponse<T>`
- `EmailListResponse`
- `EmailResponse`

## Error Handling

The client will throw errors in the following cases:

- Invalid API key or authentication failure
- Network errors
- API errors (non-200 responses)
- Decryption errors (when using automatic decryption)

## License

MIT 