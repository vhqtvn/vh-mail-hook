<script lang="ts">
  import { onMount } from 'svelte';
  import { get, post, del } from '$lib/api';
  import { page } from '$app/stores';
  import { generateAgeKeyPair, validateAgePublicKey, type AgeKeyPair } from '$lib/age';
  import ErrorAlert from '$lib/components/ErrorAlert.svelte';
  import Toast from '$lib/components/Toast.svelte';

  interface Mailbox {
    id: string;
    alias: string;
    name: string;
    address: string;
    public_key: string;
    owner_id: string;
    expires_at: number | null;
    created_at: number;
    mail_expires_in: number;
  }

  let mailboxes: Mailbox[] = [];
  let loading = true;
  let error: unknown | null = null;
  let showCreateModal = false;
  let mailboxName = '';
  
  // Expiration time controls
  let expirationDays = 7;
  let expirationHours = 0;
  let expirationMinutes = 0;
  let expirationSeconds = 0;

  // Public key management
  let publicKey = '';
  let privateKey = '';
  let publicKeyError = '';
  let generatingKey = false;
  let showPrivateKeyModal = false;
  let supportedDomains: string[] = [];
  let selectedDomain = '';

  // Add after other let declarations
  let toastMessage = '';
  let showToast = false;

  function validatePublicKey(key: string): boolean {
    return validateAgePublicKey(key);
  }

  function updatePublicKey(key: string) {
    publicKey = key;
    if (!key) {
      publicKeyError = '';
    } else if (!validatePublicKey(key)) {
      publicKeyError = 'Invalid age public key format. Should start with "age1" and be exactly 63 characters long.';
    } else {
      publicKeyError = '';
    }
  }

  async function generatePublicKey() {
    generatingKey = true;
    error = null;
    try {
      const keyPair = await generateAgeKeyPair();
      updatePublicKey(keyPair.publicKey);
      privateKey = keyPair.privateKey;
      showPrivateKeyModal = true;
    } catch (e) {
      error = e;
    } finally {
      generatingKey = false;
    }
  }

  function closePrivateKeyModal() {
    if (!confirm('Have you saved your private key? You won\'t be able to see it again!')) {
      return;
    }
    showPrivateKeyModal = false;
  }

  function calculateTotalSeconds(): number {
    return (
      expirationDays * 24 * 60 * 60 +
      expirationHours * 60 * 60 +
      expirationMinutes * 60 +
      expirationSeconds
    );
  }

  function formatExpirationTime(seconds: number): string {
    if (seconds <= 0) return '0 seconds';
    
    const days = Math.floor(seconds / (24 * 60 * 60));
    seconds %= (24 * 60 * 60);
    const hours = Math.floor(seconds / (60 * 60));
    seconds %= (60 * 60);
    const minutes = Math.floor(seconds / 60);
    seconds %= 60;

    const parts: string[] = [];
    if (days > 0) parts.push(`${days} ${days === 1 ? 'day' : 'days'}`);
    if (hours > 0) parts.push(`${hours} ${hours === 1 ? 'hour' : 'hours'}`);
    if (minutes > 0) parts.push(`${minutes} ${minutes === 1 ? 'minute' : 'minutes'}`);
    if (seconds > 0) parts.push(`${seconds} ${seconds === 1 ? 'second' : 'seconds'}`);

    return parts.join(', ');
  }

  function showNotification(message: string) {
    toastMessage = message;
    showToast = true;
    setTimeout(() => {
      showToast = false;
    }, 3000);
  }

  onMount(async () => {
    try {
      const [mailboxesResponse, domainsResponse] = await Promise.all([
        get<Mailbox[]>('/api/mailboxes'),
        get<{ domains: string[] }>('/api/supported-domains')
      ]);
      
      mailboxes = mailboxesResponse.data || [];
      supportedDomains = domainsResponse.data?.domains || [];
      if (supportedDomains.length > 0) {
        selectedDomain = supportedDomains[0];
      }
    } catch (e) {
      error = e;
    } finally {
      loading = false;
    }
  });

  async function createMailbox() {
    if (!validatePublicKey(publicKey)) {
      error = new Error('Please provide a valid public key');
      return;
    }

    const totalSeconds = calculateTotalSeconds();
    if (totalSeconds <= 0) {
      error = new Error('Please set a valid expiration time');
      return;
    }

    error = null;
    try {
      const response = await post<Mailbox>('/api/mailboxes', {
        name: mailboxName,
        expires_in_seconds: totalSeconds,
        public_key: publicKey
      });

      mailboxes = [...mailboxes, response.data!];
      showCreateModal = false;
      mailboxName = '';
      publicKey = '';
      expirationDays = 7;
      expirationHours = 0;
      expirationMinutes = 0;
      expirationSeconds = 0;
    } catch (e) {
      error = e;
    }
  }

  async function deleteMailbox(id: string) {
    if (!confirm('Are you sure you want to delete this mailbox?')) {
      return;
    }

    error = null;
    try {
      await del(`/api/mailboxes/${id}`);
      mailboxes = mailboxes.filter(m => m.id !== id);
    } catch (e) {
      error = e;
    }
  }
</script>

<div class="space-y-6">
  <div class="flex justify-between items-center">
    <h1 class="text-2xl font-bold">Your Mailboxes</h1>
    {#if mailboxes.length > 0}
      <button class="btn btn-primary" on:click={() => showCreateModal = true}>
        <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 mr-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
        </svg>
        New Mailbox
      </button>
    {/if}
  </div>

  <ErrorAlert {error} className="mb-4" />

  {#if loading}
    <div class="flex justify-center py-12">
      <span class="loading loading-spinner loading-lg"></span>
    </div>
  {:else if mailboxes.length === 0}
    <div class="flex flex-col items-center py-16 max-w-lg mx-auto text-center space-y-6">
      <!-- Empty state illustration -->
      <svg xmlns="http://www.w3.org/2000/svg" class="h-24 w-24 text-base-content/30" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4" />
      </svg>
      
      <!-- Main heading with improved typography -->
      <h3 class="font-bold text-2xl mb-2">You're all set to create your first secure mailbox!</h3>
      
      <!-- Informative description -->
      <p class="text-base-content/70 text-lg">
        Mail Hook provides encrypted email storage to keep your messages safe and private.
      </p>

      <!-- Enhanced CTA button -->
      <div class="space-y-3">
        <button class="btn btn-primary btn-lg" on:click={() => showCreateModal = true}>
          <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 mr-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
          </svg>
          Create Mailbox
        </button>
        <p class="text-base-content/60 text-sm">Secure your emails with just one click</p>
      </div>
    </div>
  {:else}
    <div class="grid gap-4 grid-cols-1">
      {#each mailboxes as mailbox (mailbox.id)}
        <div class="card bg-base-200">
          <div class="card-body">
            <div class="flex justify-between items-start">
              <div>
                <h2 class="card-title text-xl">{mailbox.name}</h2>
              </div>
              <button 
                class="btn btn-square btn-sm btn-ghost text-error" 
                on:click={() => deleteMailbox(mailbox.id)}
                aria-label="Delete mailbox"
              >
                <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                </svg>
              </button>
            </div>

            <div class="space-y-3">
              <!-- Email Address -->
              <div>
                <div class="text-sm font-semibold mb-2">Email Address</div>
                <div class="flex gap-2 items-center">
                  <div class="text-base text-base-content/70 flex-1 font-mono bg-base-300 p-3 rounded min-w-0">
                    <div class="flex items-center">
                      <span class="text-primary font-semibold whitespace-nowrap">{mailbox.alias}@</span>
                      {#if supportedDomains.length > 1}
                        <div class="dropdown dropdown-hover inline-block">
                          <button 
                            class="text-base-content/50 cursor-pointer"
                            aria-label="Select domain"
                          >{selectedDomain}</button>
                          <ul class="dropdown-content z-[1] menu p-2 shadow bg-base-200 rounded-box">
                            {#each supportedDomains as domain}
                              <li>
                                <button 
                                  class="whitespace-nowrap" 
                                  on:click={() => selectedDomain = domain}
                                >
                                  {domain}
                                </button>
                              </li>
                            {/each}
                          </ul>
                        </div>
                      {:else}
                        <span class="text-base-content/50">{supportedDomains[0] || 'any-supported-domain'}</span>
                      {/if}
                    </div>
                  </div>
                  <button 
                    class="btn btn-square btn-sm btn-ghost shrink-0"
                    on:click={() => {
                      const domain = selectedDomain || supportedDomains[0] || 'any-supported-domain';
                      const email = `${mailbox.alias}@${domain}`;
                      navigator.clipboard.writeText(email);
                      showNotification('Email address copied to clipboard');
                    }}
                    aria-label="Copy mailbox address"
                  >
                    <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-12a2 2 0 00-2-2h-2M8 5a2 2 0 002 2h4a2 2 0 002-2M8 5a2 2 0 012-2h4a2 2 0 012 2" />
                    </svg>
                  </button>
                </div>
              </div>

              <!-- Mailbox ID for API -->
              <div>
                <div class="text-sm font-semibold mb-2">Mailbox ID (for API)</div>
                <div class="flex gap-2 items-center">
                  <div class="text-base text-base-content/70 flex-1 font-mono bg-base-300 p-3 rounded min-w-0">
                    <div class="truncate">
                      {mailbox.id}
                    </div>
                  </div>
                  <button 
                    class="btn btn-square btn-sm btn-ghost shrink-0"
                    on:click={() => {
                      navigator.clipboard.writeText(mailbox.id);
                      showNotification('Mailbox ID copied to clipboard');
                    }}
                    aria-label="Copy ID"
                  >
                    <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-1M8 5a2 2 0 002 2h2a2 2 0 002-2M8 5a2 2 0 012-2h2a2 2 0 012 2m0 0h2a2 2 0 012 2v3m2 4H10m0 0l3-3m-3 3l3 3" />
                    </svg>
                  </button>
                </div>
              </div>

              <!-- Public Key -->
              <div>
                <div class="text-sm font-semibold mb-2">Public Key</div>
                <div class="flex gap-2 items-center">
                  <div class="text-base text-base-content/70 flex-1 font-mono bg-base-300 p-3 rounded min-w-0">
                    <div class="truncate">
                      {mailbox.public_key}
                    </div>
                  </div>
                  <button 
                    class="btn btn-square btn-sm btn-ghost shrink-0"
                    on:click={() => {
                      navigator.clipboard.writeText(mailbox.public_key);
                      showNotification('Public key copied to clipboard');
                    }}
                    aria-label="Copy public key"
                  >
                    <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-12a2 2 0 00-2-2h-2M8 5a2 2 0 002 2h4a2 2 0 002-2M8 5a2 2 0 012-2h4a2 2 0 012 2" />
                    </svg>
                  </button>
                </div>
              </div>

              <div class="flex justify-between items-center text-sm">
                <div class="text-base-content/70">
                  Created {new Date(mailbox.created_at * 1000).toLocaleDateString()}
                </div>
                {#if mailbox.mail_expires_in}
                  <div class="text-warning">
                    Emails expire after arriving in your mailbox {formatExpirationTime(mailbox.mail_expires_in)}
                  </div>
                {/if}
              </div>
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<!-- Create Mailbox Modal -->
{#if showCreateModal}
  <div class="modal modal-open">
    <div class="modal-box">
      <h3 class="font-bold text-lg mb-4">Create New Mailbox</h3>
      
      <ErrorAlert {error} className="mb-4" />

      <form on:submit|preventDefault={createMailbox}>
        <div class="form-control">
          <label class="label" for="mailbox-name">
            <span class="label-text">Mailbox Name</span>
          </label>
          <input
            type="text"
            id="mailbox-name"
            class="input input-bordered w-full"
            bind:value={mailboxName}
            placeholder="Enter mailbox name"
            required
          />
        </div>

        <div class="form-control mt-4">
          <label class="label" for="expiration-time">
            <span class="label-text">Expiration Time (max 30 days)</span>
          </label>
          <div class="grid grid-cols-4 gap-2">
            <div class="form-control">
              <input
                type="number"
                id="expiration-days"
                class="input input-bordered w-full"
                bind:value={expirationDays}
                min="0"
                max="30"
                placeholder="Days"
              />
              <label class="label" for="expiration-days">
                <span class="label-text-alt">Days</span>
              </label>
            </div>
            <div class="form-control">
              <input
                type="number"
                id="expiration-hours"
                class="input input-bordered w-full"
                bind:value={expirationHours}
                min="0"
                max="23"
                placeholder="Hours"
              />
              <label class="label" for="expiration-hours">
                <span class="label-text-alt">Hours</span>
              </label>
            </div>
            <div class="form-control">
              <input
                type="number"
                id="expiration-minutes"
                class="input input-bordered w-full"
                bind:value={expirationMinutes}
                min="0"
                max="59"
                placeholder="Mins"
              />
              <label class="label" for="expiration-minutes">
                <span class="label-text-alt">Minutes</span>
              </label>
            </div>
            <div class="form-control">
              <input
                type="number"
                id="expiration-seconds"
                class="input input-bordered w-full"
                bind:value={expirationSeconds}
                min="0"
                max="59"
                placeholder="Secs"
              />
              <label class="label" for="expiration-seconds">
                <span class="label-text-alt">Seconds</span>
              </label>
            </div>
          </div>
        </div>

        <div class="form-control mt-4">
          <label class="label" for="public-key">
            <span class="label-text">Public Key (age format)</span>
          </label>
          <div class="join w-full">
            <input
              type="text"
              id="public-key"
              class="input input-bordered join-item w-full"
              bind:value={publicKey}
              on:input={(e) => updatePublicKey((e.target as HTMLInputElement).value)}
              placeholder="age1..."
              required
            />
            <button 
              type="button" 
              class="btn"
              on:click|preventDefault={generatePublicKey}
              disabled={generatingKey}
            >
              {#if generatingKey}
                <span class="loading loading-spinner loading-sm"></span>
              {:else}
                <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 mr-1" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
                </svg>
                Generate
              {/if}
            </button>
          </div>
          {#if publicKeyError}
            <div class="text-error text-sm mt-1">{publicKeyError}</div>
          {/if}
          <div class="text-info text-sm mt-1 flex items-center gap-1">
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            <span>The public key will be used to encrypt emails sent to this mailbox</span>
          </div>
        </div>

        <div class="modal-action">
          <button type="button" class="btn" on:click={() => showCreateModal = false}>Cancel</button>
          <button type="submit" class="btn btn-primary">Create</button>
        </div>
      </form>
    </div>
  </div>
{/if}

<!-- Private Key Modal -->
{#if showPrivateKeyModal}
  <div class="modal modal-open">
    <div class="modal-box">
      <h3 class="font-bold text-lg mb-4">Save Your Private Key</h3>
      <p class="text-warning mb-4">
        Important: Save this private key now. You won't be able to see it again!
      </p>
      <div class="bg-base-300 p-4 rounded-lg font-mono text-sm mb-4 break-all">
        {privateKey}
      </div>
      <div class="flex gap-2 mb-4">
        <button 
          class="btn btn-outline flex-1"
          on:click={() => {
            navigator.clipboard.writeText(privateKey);
          }}
        >
          <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 mr-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-12a2 2 0 00-2-2h-2M8 5a2 2 0 002 2h4a2 2 0 002-2M8 5a2 2 0 012-2h4a2 2 0 012 2" />
          </svg>
          Copy to Clipboard
        </button>
        <button 
          class="btn btn-outline flex-1"
          on:click={() => {
            const blob = new Blob([privateKey], { type: 'text/plain' });
            const url = URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = 'private_key.txt';
            document.body.appendChild(a);
            a.click();
            document.body.removeChild(a);
            URL.revokeObjectURL(url);
          }}
        >
          <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 mr-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
          </svg>
          Download
        </button>
      </div>
      <div class="modal-action">
        <button class="btn btn-primary" on:click={closePrivateKeyModal}>I've Saved It</button>
      </div>
    </div>
  </div>
{/if}

<Toast message={toastMessage} visible={showToast} /> 