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

  interface Mailbox {
    id: string;
    alias: string;
    name: string;
    public_key: string;
  }

  let loading = true;
  let error: unknown | null = null;
  let emails: Email[] = [];
  let decryptedEmails: DecryptedEmail[] = [];
  let toastMessage = '';
  let showToast = false;
  let mailbox: Mailbox | null = null;

  function showNotification(message: string) {
    toastMessage = message;
    showToast = true;
    setTimeout(() => {
      showToast = false;
    }, 3000);
  }

  async function decryptEmail(email: Email, privateKey: string): Promise<DecryptedEmail> {
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
        ...email,
        content,
        parsed
      };
    } catch (error) {
      console.error('Error decrypting email:', error);
      throw new Error('Failed to decrypt email');
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
        // Decrypt all emails
        decryptedEmails = await Promise.all(
          emails.map(email => decryptEmail(email, privateKey))
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
      decryptedEmails = decryptedEmails.filter(e => e.id !== emailId);
      showNotification('Email deleted successfully');
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

<div class="container mx-auto px-4 py-8 max-w-4xl">
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

  <ErrorAlert {error} className="mb-4" />

  {#if loading}
    <div class="flex justify-center py-12">
      <span class="loading loading-spinner loading-lg"></span>
    </div>
  {:else if emails.length === 0}
    <div class="text-center py-12">
      <p class="text-base-content/70">No emails in this mailbox yet.</p>
    </div>
  {:else}
    <div class="space-y-4">
      {#if emails.length > 0 && decryptedEmails.length === 0}
        <div class="alert alert-warning">
          <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
          </svg>
          <span>Private key not found in browser storage. Cannot decrypt emails.</span>
        </div>
      {/if}

      {#each emails as email (email.id)}
        <div class="card bg-base-200">
          <div class="card-body">
            <div class="flex justify-between items-start mb-4">
              <div>
                <div class="text-sm font-semibold mb-2">Email ID</div>
                <div class="font-mono text-sm">{email.id}</div>
              </div>
              <button 
                class="btn btn-square btn-sm btn-ghost text-error" 
                on:click={() => deleteEmail(email.id)}
                aria-label="Delete email"
              >
                <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                </svg>
              </button>
            </div>

            <div class="grid grid-cols-2 gap-4 text-sm mb-4">
              <div>
                <span class="font-semibold">Received:</span>
                <div class="tooltip" data-tip={formatDateTime(email.received_at)}>
                  {formatRelativeTime(email.received_at)}
                </div>
              </div>
              {#if email.expires_at}
                <div>
                  <span class="font-semibold">Expires:</span>
                  <div class="tooltip" data-tip={formatDateTime(email.expires_at)}>
                    {formatRelativeTime(email.expires_at)}
                  </div>
                </div>
              {/if}
            </div>

            <div class="collapse bg-base-300">
              <input type="checkbox" id="raw-content-{email.id}" class="peer" /> 
              <label for="raw-content-{email.id}" class="collapse-title font-medium flex items-center cursor-pointer">
                <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 mr-2 transition-transform peer-checked:rotate-90" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
                </svg>
                Raw Encrypted Content
              </label>
              <div class="collapse-content">
                <div class="font-mono text-xs break-all whitespace-pre-wrap">
                  {email.encrypted_content}
                </div>
                <div class="mt-2">
                  <button 
                    class="btn btn-sm btn-ghost"
                    on:click={() => {
                      navigator.clipboard.writeText(email.encrypted_content);
                      showNotification('Encrypted content copied to clipboard');
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

            {#if decryptedEmails.length > 0}
              {@const decryptedEmail = decryptedEmails.find(d => d.id === email.id)}
              {#if decryptedEmail}
                <div class="mt-4">
                  <div class="divider">Decrypted Content</div>
                  <div class="space-y-4">
                    <h2 class="text-xl font-bold">{decryptedEmail.parsed.subject || '(No Subject)'}</h2>
                    
                    <div class="grid grid-cols-2 gap-4 text-sm">
                      <div>
                        <span class="font-semibold">From:</span> {decryptedEmail.parsed.from}
                      </div>
                      <div>
                        <span class="font-semibold">To:</span> {decryptedEmail.parsed.to}
                      </div>
                    </div>

                    <div class="whitespace-pre-wrap font-mono text-sm bg-base-300 p-4 rounded">
                      {decryptedEmail.parsed.body}
                    </div>

                    <div class="collapse bg-base-300">
                      <input type="checkbox" id="raw-decrypted-{email.id}" class="peer" /> 
                      <label for="raw-decrypted-{email.id}" class="collapse-title font-medium flex items-center cursor-pointer">
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
                  </div>
                </div>
              {/if}
            {/if}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<Toast message={toastMessage} visible={showToast} /> 