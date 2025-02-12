/**
 * VHMailHook Test Email Sender
 * 
 * This script sends randomly generated test emails to a specified mailbox.
 * It uses the Faker library to generate realistic-looking email content.
 * 
 * Usage:
 * 1. Install dependencies: npm install
 * 2. Run: node tests/scripts/send-test-email.js <mailbox-alias>
 * 
 * Example:
 * node tests/scripts/send-test-email.js test123
 */

import { faker } from '@faker-js/faker';
import nodemailer from 'nodemailer';
import dotenv from 'dotenv';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import fs from 'fs';

// Get the directory of the current module
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Load environment variables from the project root
const envPath = join(__dirname, '../../.env');
if (fs.existsSync(envPath)) {
  dotenv.config({ path: envPath });
} else {
  console.warn('Warning: .env file not found in project root');
}

const DOMAIN = process.env.DOMAIN || 'mail-hook.localhost';
const MAILBOX_ALIAS = process.argv[2];

if (!MAILBOX_ALIAS) {
  console.error('Error: Mailbox alias is required as first argument');
  console.error('Usage: node send-test-email.js <mailbox-alias>');
  console.error('Example: node send-test-email.js test123');
  process.exit(1);
}

// Create a test SMTP transport
const transport = nodemailer.createTransport({
  host: 'localhost',
  port: 2525,
  secure: false,
  tls: {
    rejectUnauthorized: false
  }
});

/**
 * Generate a random email with realistic content
 */
function generateTestEmail() {
  const fromName = faker.person.fullName();
  const fromEmail = faker.internet.email();
  const subject = faker.helpers.arrayElement([
    faker.company.catchPhrase(),
    faker.hacker.phrase(),
    `[${faker.company.name()}] ${faker.commerce.productName()}`,
    `Re: ${faker.hacker.phrase()}`,
    `Fwd: ${faker.company.catchPhrase()}`
  ]);

  const paragraphs = [];
  const numParagraphs = faker.number.int({ min: 2, max: 5 });
  
  for (let i = 0; i < numParagraphs; i++) {
    paragraphs.push(faker.lorem.paragraph());
  }

  // Sometimes add a signature
  const hasSignature = faker.datatype.boolean();
  if (hasSignature) {
    paragraphs.push('\\n--');
    paragraphs.push(fromName);
    paragraphs.push(faker.company.name());
    paragraphs.push(faker.phone.number());
  }

  return {
    from: `"${fromName}" <${fromEmail}>`,
    subject,
    text: paragraphs.join('\\n\\n')
  };
}

/**
 * Send a test email
 */
async function sendTestEmail() {
  try {
    const email = generateTestEmail();
    const recipient = `${MAILBOX_ALIAS}@${DOMAIN}`;
    
    console.log('Sending test email to:', recipient);
    const info = await transport.sendMail({
      from: email.from,
      to: recipient,
      subject: email.subject,
      text: email.text
    });

    console.log('Test email sent successfully!');
    console.log('From:', email.from);
    console.log('To:', recipient);
    console.log('Subject:', email.subject);
    console.log('Preview:', email.text.slice(0, 100) + '...');
    console.log('Response:', info.response);
  } catch (error) {
    console.error('Failed to send test email:', error.message);
    process.exit(1);
  }
}

// Send the test email
sendTestEmail(); 