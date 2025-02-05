/**
 * VHMailHook Email Reader Example
 * 
 * This example demonstrates how to:
 * 1. Connect to the VHMailHook API
 * 2. Retrieve emails from a mailbox
 * 3. Decrypt emails using AGE encryption
 * 
 * Prerequisites:
 * - Node.js 16.x or later
 * - age-encryption npm package
 * - Valid VHMailHook API credentials
 * - AGE private key for decryption
 */

import * as age from 'age-encryption';
import dotenv from 'dotenv';

// Load environment variables
dotenv.config();

const DOMAIN = process.env.DOMAIN || 'https://mail-hook.libvh.dev/';
const API_KEY = process.env.API_KEY;
const MAILBOX_ID = process.env.MAILBOX_ID;
const PRIVATE_KEY = process.env.PRIVATE_KEY;

// Validate required environment variables
if (!API_KEY) {
    throw new Error('API_KEY is not set in environment variables');
}

if (!MAILBOX_ID) {
    throw new Error('MAILBOX_ID is not set in environment variables');
}

if (!PRIVATE_KEY) {
    throw new Error('PRIVATE_KEY is not set in environment variables');
}

/**
 * Decrypts content using AGE encryption
 * @param {string} encrypted_content - Base64 encoded encrypted content
 * @returns {Promise<string>} Decrypted content as UTF-8 string
 */
async function age_decrypt(encrypted_content) {
    try {
        const d = new age.Decrypter();
        d.addIdentity(PRIVATE_KEY);
        const encrypted_uint8_array = Buffer.from(encrypted_content, 'base64');
        const decrypted_uint8_array = await d.decrypt(encrypted_uint8_array);
        return Buffer.from(decrypted_uint8_array).toString('utf-8');
    } catch (error) {
        console.error('Error decrypting content:', error.message);
        throw new Error('Failed to decrypt email content');
    }
}

/**
 * Main function to retrieve and decrypt emails
 */
async function main() {
    try {
        // Fetch emails from the mailbox
        const response = await fetch(`${DOMAIN.replace(/\/$/, '')}/api/v1/mailboxes/${MAILBOX_ID}/emails`, {
            headers: {
                'Authorization': `Bearer ${API_KEY}`,
                'Accept': 'application/json'
            }
        });

        if (!response.ok) {
            throw new Error(`API request failed: ${response.status} ${response.statusText}`);
        }

        const data = await response.json();

        // Validate API response
        if (!data.success) {
            throw new Error(data.error || 'Unknown API error');
        }

        // Process each email
        for (const email of data.data) {
            console.log('\nEmail Details:');
            console.log('ID:', email.id);
            console.log('Received:', new Date(email.received_at * 1000).toLocaleString());
            console.log('Expires:', new Date(email.expires_at * 1000).toLocaleString());
            console.log('\nDecrypted Content:');
            console.log('='.repeat(50));
            
            try {
                const decrypted_content = await age_decrypt(email.encrypted_content);
                console.log(decrypted_content);
            } catch (error) {
                console.error(`Failed to decrypt email ${email.id}:`, error.message);
            }
            
            console.log('='.repeat(50));
        }
    } catch (error) {
        console.error('Error:', error.message);
        process.exit(1);
    }
}

// Run the example
main();
