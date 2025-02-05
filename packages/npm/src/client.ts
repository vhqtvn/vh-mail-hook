import fetch from 'cross-fetch';
import * as age from 'age-encryption';
import { VHMailHookConfig, Email, EmailListResponse, EmailResponse, DecryptedEmail } from './types';

export class VHMailHookClient {
  private domain: string;
  private apiKey: string;

  constructor(config: VHMailHookConfig) {
    this.domain = config.domain?.replace(/\/$/, '') || 'https://mail-hook.libvh.dev';
    this.apiKey = config.apiKey;
  }

  private async request<T>(path: string, method: 'GET' | 'DELETE' = 'GET'): Promise<T> {
    const response = await fetch(`${this.domain}${path}`, {
      method,
      headers: {
        'Authorization': `Bearer ${this.apiKey}`,
        'Accept': 'application/json'
      }
    });

    if (!response.ok) {
      throw new Error(`API request failed: ${response.status} ${response.statusText}`);
    }

    const data = await response.json();
    if (!data.success) {
      throw new Error(data.error || 'Unknown API error');
    }

    return data;
  }

  private async decryptContent(encryptedContent: string, privateKey: string): Promise<string> {
    try {
      const d = new age.Decrypter();
      d.addIdentity(privateKey);
      const encryptedUint8Array = Buffer.from(encryptedContent, 'base64');
      const decryptedUint8Array = await d.decrypt(encryptedUint8Array);
      return Buffer.from(decryptedUint8Array).toString('utf-8');
    } catch (error) {
      throw new Error(`Failed to decrypt email content: ${(error as Error).message}`);
    }
  }

  /**
   * Get all emails from a mailbox
   * @param mailboxId The ID of the mailbox
   * @param privateKey Optional AGE private key for automatic decryption
   * @returns Array of emails (decrypted if privateKey is provided)
   */
  async getEmails(mailboxId: string, privateKey?: string): Promise<Email[] | DecryptedEmail[]> {
    const response = await this.request<EmailListResponse>(`/api/v1/mailboxes/${mailboxId}/emails`);
    
    if (!privateKey) {
      return response.data;
    }

    return Promise.all(
      response.data.map(async (email) => {
        const content = await this.decryptContent(email.encrypted_content, privateKey);
        const { encrypted_content, ...rest } = email;
        return {
          ...rest,
          content
        };
      })
    );
  }

  /**
   * Delete an email from a mailbox
   * @param mailboxId The ID of the mailbox
   * @param emailId The ID of the email to delete
   */
  async deleteEmail(mailboxId: string, emailId: string): Promise<void> {
    await this.request(`/api/v1/mailboxes/${mailboxId}/emails/${emailId}`, 'DELETE');
  }
} 