export interface VHMailHookConfig {
  domain?: string;
  apiKey: string;
}

export interface Email {
  id: string;
  received_at: number;
  expires_at: number;
  encrypted_content: string;
}

export interface APIResponse<T> {
  success: boolean;
  error?: string;
  data?: T;
}

export interface EmailListResponse extends APIResponse<Email[]> {
  data: Email[];
}

export interface EmailResponse extends APIResponse<Email> {
  data: Email;
}

export interface DecryptedEmail extends Omit<Email, 'encrypted_content'> {
  content: string;
} 