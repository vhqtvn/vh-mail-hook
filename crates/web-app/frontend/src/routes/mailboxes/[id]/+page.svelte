<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { get, del } from '$lib/api';
  import ErrorAlert from '$lib/components/ErrorAlert.svelte';
  import Toast from '$lib/components/Toast.svelte';
  import * as age from 'age-encryption';
  import { getPrivateKey } from '$lib/storage';

  interface Email {
    id: string;
    mailbox_id: string;
    encrypted_content: string;
    received_at: number;
    expires_at: number | null;
  }

  interface DecryptedEmail extends Omit<Email, 'encrypted_content'> {
    content: string;
    parsed: {
      subject: string;
      from: string;
      to: string;
      date: string;
      body: string;
    };
  }

  interface DecryptionResult {
    email: DecryptedEmail | null;
    error?: string;
  }

  interface Mailbox {
    id: string;
    alias: string;
    name: string;
    public_key: string;
  }

  let loading = true;
  let error: unknown | null = null;
  let emails: Email[] = [];
  let decryptionResults: Map<string, DecryptionResult> = new Map();
  let toastMessage = '';
  let showToast = false;
  let mailbox: Mailbox | null = null;
  let selectedEmailId: string | null = null;

  function showNotification(message: string) {
    toastMessage = message;
    showToast = true;
    setTimeout(() => {
      showToast = false;
    }, 3000);
  }

  async function decryptEmail(email: Email, privateKey: string): Promise<DecryptionResult> {
    try {
      const d = new age.Decrypter();
      d.addIdentity(privateKey);
      
      // Convert base64 to Uint8Array using browser APIs
      const binaryString = atob(email.encrypted_content);
      const bytes = new Uint8Array(binaryString.length);
      for (let i = 0; i < binaryString.length; i++) {
        bytes[i] = binaryString.charCodeAt(i);
      }
      
      const decryptedUint8Array = await d.decrypt(bytes);
      const decoder = new TextDecoder();
      const content = decoder.decode(decryptedUint8Array);

      // Parse email content
      const parsed = {
        subject: '',
        from: '',
        to: '',
        date: '',
        body: ''
      };

      const lines = content.split('\r\n');
      let isBody = false;
      for (const line of lines) {
        if (isBody) {
          parsed.body += line + '\n';
        } else if (line === '') {
          isBody = true;
        } else {
          const [key, ...valueParts] = line.split(':');
          const value = valueParts.join(':').trim();
          switch (key.toLowerCase()) {
            case 'subject':
              parsed.subject = value;
              break;
            case 'from':
              parsed.from = value;
              break;
            case 'to':
              parsed.to = value;
              break;
            case 'date':
              parsed.date = value;
              break;
          }
        }
      }

      return {
        email: {
          ...email,
          content,
          parsed
        }
      };
    } catch (error) {
      console.error('Error decrypting email:', error);
      const errorMessage = error instanceof Error ? error.message : 'Failed to decrypt email';
      
      // Check if the error indicates no matching key
      if (errorMessage.includes('no identity matched any of the file\'s recipients')) {
        return {
          email: null,
          error: 'This email was encrypted with a different key. The private key stored in your browser cannot decrypt it.'
        };
      }
      
      return {
        email: null,
        error: errorMessage
      };
    }
  }

  async function loadEmails() {
    loading = true;
    error = null;
    try {
      const [mailboxResponse, emailsResponse] = await Promise.all([
        get<Mailbox>('/api/mailboxes/' + $page.params.id),
        get<Email[]>('/api/mailboxes/' + $page.params.id + '/emails')
      ]);

      mailbox = mailboxResponse.data!;
      emails = emailsResponse.data || [];

      // Get the private key from localStorage
      const privateKey = getPrivateKey(mailbox.public_key);

      if (privateKey) {
        // Decrypt each email independently
        const results = await Promise.all(
          emails.map(async email => {
            const result = await decryptEmail(email, privateKey);
            return [email.id, result] as [string, DecryptionResult];
          })
        );
        decryptionResults = new Map(results);
      } else {
        // Set decryption error for all emails when no private key is available
        decryptionResults = new Map(
          emails.map(email => [
            email.id,
            {
              email: null,
              error: 'This email was encrypted with a different key. The private key stored in your browser cannot decrypt it.'
            }
          ])
        );
      }
    } catch (e) {
      error = e;
    } finally {
      loading = false;
    }
  }

  async function deleteEmail(emailId: string) {
    if (!confirm('Are you sure you want to delete this email?')) {
      return;
    }

    try {
      await del('/api/mailboxes/' + $page.params.id + '/emails/' + emailId);
      emails = emails.filter(e => e.id !== emailId);
      decryptionResults.delete(emailId);
      showNotification('Email deleted successfully');
      // Reset selected email if it was deleted
      if (selectedEmailId === emailId) {
        selectedEmailId = null;
      }
    } catch (e) {
      error = e;
    }
  }

  // Add time formatting utilities
  function formatRelativeTime(timestamp: number): string {
    const now = Math.floor(Date.now() / 1000);
    const diff = Math.abs(timestamp - now);
    const future = timestamp > now;
    
    const minute = 60;
    const hour = minute * 60;
    const day = hour * 24;
    const week = day * 7;
    const month = day * 30;
    const year = day * 365;

    let value: number;
    let unit: string;

    if (diff < minute) {
      value = diff;
      unit = 'second';
    } else if (diff < hour) {
      value = Math.floor(diff / minute);
      unit = 'minute';
    } else if (diff < day) {
      value = Math.floor(diff / hour);
      unit = 'hour';
    } else if (diff < week) {
      value = Math.floor(diff / day);
      unit = 'day';
    } else if (diff < month) {
      value = Math.floor(diff / week);
      unit = 'week';
    } else if (diff < year) {
      value = Math.floor(diff / month);
      unit = 'month';
    } else {
      value = Math.floor(diff / year);
      unit = 'year';
    }

    if (value !== 1) {
      unit += 's';
    }

    return future ? `in ${value} ${unit}` : `${value} ${unit} ago`;
  }

  function formatDateTime(timestamp: number): string {
    const date = new Date(timestamp * 1000);
    return `${date.toLocaleDateString()} ${date.toLocaleTimeString()}`;
  }

  onMount(() => {
    loadEmails();
  });
</script>

<div class="container mx-auto px-4 py-8">
  <div class="flex justify-between items-center mb-8">
    <div>
      <h1 class="text-2xl font-bold">Mailbox Emails</h1>
      {#if mailbox}
        <div class="text-base-content/70 mt-1 font-mono">{mailbox.alias}</div>
      {/if}
    </div>
    <button class="btn" on:click={loadEmails}>
      <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 mr-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
      </svg>
      Refresh
    </button>
  </div>

  {#if error}
    <ErrorAlert {error} />
  {/if}

  {#if loading}
    <div class="flex justify-center items-center h-64">
      <div class="loading loading-spinner loading-lg"></div>
    </div>
  {:else}
    <div class="flex gap-4 h-[calc(100vh-12rem)]">
      <!-- Email List - Left Side -->
      <div class="w-1/3 overflow-y-auto border rounded-lg">
        {#if emails.length === 0}
          <div class="p-4 text-center text-base-content/70">
            No emails found
          </div>
        {:else}
          {#each emails as email (email.id)}
            {@const decryptionResult = decryptionResults.get(email.id)}
            {@const decryptedEmail = decryptionResult?.email}
            <div 
              class="p-4 border-b cursor-pointer hover:bg-base-200 transition-colors"
              class:bg-base-200={decryptedEmail && selectedEmailId === email.id}
              on:click={() => selectedEmailId = email.id}
            >
              {#if decryptedEmail}
                <div class="font-medium mb-1">{decryptedEmail.parsed.subject || '(No subject)'}</div>
                <div class="text-sm text-base-content/70 mb-1">{decryptedEmail.parsed.from}</div>
                <div class="text-xs text-base-content/50 flex justify-between">
                  <span title={formatDateTime(email.received_at)}>{formatRelativeTime(email.received_at)}</span>
                  {#if email.expires_at}
                    <span title="Expires {formatDateTime(email.expires_at)}">
                      Expires {formatRelativeTime(email.expires_at)}
                    </span>
                  {/if}
                </div>
              {:else if decryptionResult?.error}
                <div class="text-error text-sm mb-1">{decryptionResult.error}</div>
                <div class="text-xs text-base-content/50 flex justify-between">
                  <span title={formatDateTime(email.received_at)}>{formatRelativeTime(email.received_at)}</span>
                  {#if email.expires_at}
                    <span title="Expires {formatDateTime(email.expires_at)}">
                      Expires {formatRelativeTime(email.expires_at)}
                    </span>
                  {/if}
                </div>
              {:else}
                <div class="text-base-content/50">Loading...</div>
              {/if}
            </div>
          {/each}
        {/if}
      </div>

      <!-- Email Content - Right Side -->
      <div class="w-2/3 overflow-y-auto border rounded-lg">
        {#if !getPrivateKey(mailbox?.public_key || '')}
          <div class="p-6">
            <div class="alert alert-warning">
              <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
              </svg>
              <span>Private key not found in browser storage. Cannot decrypt emails. If you recently updated the key, old emails cannot be decrypted with the new key.</span>
            </div>
          </div>
        {/if}
        {#if selectedEmailId}
          {@const decryptionResult = decryptionResults.get(selectedEmailId)}
          {@const decryptedEmail = decryptionResult?.email}
          {@const selectedEmail = emails.find(e => e.id === selectedEmailId)}
          {#if decryptedEmail}
            <div class="p-6">
              <div class="flex justify-between items-start mb-6">
                <div>
                  <h2 class="text-xl font-bold mb-4">{decryptedEmail.parsed.subject || '(No subject)'}</h2>
                  <div class="space-y-2">
                    <div><span class="font-medium">From:</span> {decryptedEmail.parsed.from}</div>
                    <div><span class="font-medium">To:</span> {decryptedEmail.parsed.to}</div>
                    <div><span class="font-medium">Date:</span> {decryptedEmail.parsed.date}</div>
                  </div>
                </div>
                <button 
                  class="btn btn-error btn-sm"
                  on:click={() => selectedEmailId && deleteEmail(selectedEmailId)}
                >
                  Delete
                </button>
              </div>
              <div class="divider"></div>
              <div class="whitespace-pre-wrap font-mono mb-6">{decryptedEmail.parsed.body}</div>

              <!-- Raw Content Sections -->
              <div class="space-y-4">
                <div class="collapse bg-base-200">
                  <input type="checkbox" id="raw-decrypted-{selectedEmailId}" class="peer" /> 
                  <label for="raw-decrypted-{selectedEmailId}" class="collapse-title font-medium flex items-center cursor-pointer">
                    <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 mr-2 transition-transform peer-checked:rotate-90" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
                    </svg>
                    Raw Decrypted Content
                  </label>
                  <div class="collapse-content">
                    <div class="font-mono text-xs break-all whitespace-pre-wrap">
                      {decryptedEmail.content}
                    </div>
                    <div class="mt-2">
                      <button 
                        class="btn btn-sm btn-ghost"
                        on:click={() => {
                          navigator.clipboard.writeText(decryptedEmail.content);
                          showNotification('Raw decrypted content copied to clipboard');
                        }}
                      >
                        <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 mr-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-12a2 2 0 00-2-2h-2M8 5a2 2 0 002 2h4a2 2 0 002-2M8 5a2 2 0 012-2h4a2 2 0 012 2" />
                        </svg>
                        Copy Raw
                      </button>
                    </div>
                  </div>
                </div>

                <div class="collapse bg-base-200">
                  <input type="checkbox" id="raw-encrypted-{selectedEmailId}" class="peer" /> 
                  <label for="raw-encrypted-{selectedEmailId}" class="collapse-title font-medium flex items-center cursor-pointer">
                    <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 mr-2 transition-transform peer-checked:rotate-90" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
                    </svg>
                    Raw Encrypted Content
                  </label>
                  <div class="collapse-content">
                    <div class="font-mono text-xs break-all whitespace-pre-wrap">
                      {selectedEmail?.encrypted_content}
                    </div>
                    <div class="mt-2">
                      <button 
                        class="btn btn-sm btn-ghost"
                        on:click={() => {
                          if (selectedEmail) {
                            navigator.clipboard.writeText(selectedEmail.encrypted_content);
                            showNotification('Raw encrypted content copied to clipboard');
                          }
                        }}
                      >
                        <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 mr-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-12a2 2 0 00-2-2h-2M8 5a2 2 0 002 2h4a2 2 0 002-2M8 5a2 2 0 012-2h4a2 2 0 012 2" />
                        </svg>
                        Copy Raw
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          {:else if decryptionResult?.error}
            <div class="p-6 space-y-4">
              <div class="alert alert-error">
                <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                </svg>
                <div>
                  <div class="font-semibold">Unable to decrypt email</div>
                  <div class="text-sm">{decryptionResult.error}</div>
                </div>
              </div>

              <button 
                class="btn btn-error w-full"
                on:click={() => selectedEmailId && deleteEmail(selectedEmailId)}
              >
                Delete Email
              </button>

              <!-- Show Raw Encrypted Content for Failed Decryption -->
              <div class="collapse bg-base-200">
                <input type="checkbox" id="raw-encrypted-error-{selectedEmailId}" class="peer" /> 
                <label for="raw-encrypted-error-{selectedEmailId}" class="collapse-title font-medium flex items-center cursor-pointer">
                  <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 mr-2 transition-transform peer-checked:rotate-90" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
                  </svg>
                  Raw Encrypted Content
                </label>
                <div class="collapse-content">
                  <div class="font-mono text-xs break-all whitespace-pre-wrap">
                    {selectedEmail?.encrypted_content}
                  </div>
                  <div class="mt-2">
                    <button 
                      class="btn btn-sm btn-ghost"
                      on:click={() => {
                        if (selectedEmail) {
                          navigator.clipboard.writeText(selectedEmail.encrypted_content);
                          showNotification('Raw encrypted content copied to clipboard');
                        }
                      }}
                    >
                      <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 mr-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-12a2 2 0 00-2-2h-2M8 5a2 2 0 002 2h4a2 2 0 002-2M8 5a2 2 0 012-2h4a2 2 0 012 2" />
                      </svg>
                      Copy Raw
                    </button>
                  </div>
                </div>
              </div>
            </div>
          {:else}
            <div class="flex justify-center items-center h-full">
              <div class="loading loading-spinner loading-lg"></div>
            </div>
          {/if}
        {:else}
          <div class="flex justify-center items-center h-full text-base-content/70">
            Select an email to view its content
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<Toast message={toastMessage} visible={showToast} />

<style>
  /* Add any custom styles here */
  .divider {
    @apply my-4 border-t border-base-300;
  }
</style> 