<script lang="ts">
  import { onMount } from 'svelte';
  import { get, post, del, patch } from '$lib/api';
  import { page } from '$app/stores';
  import { generateAgeKeyPair, validateAgePublicKey, type AgeKeyPair } from '$lib/age';
  import ErrorAlert from '$lib/components/ErrorAlert.svelte';
  import Toast from '$lib/components/Toast.svelte';
  import { savePrivateKey, getPrivateKey, hasPrivateKey } from '$lib/storage';

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

  // Add to the script section after other let declarations
  let showUpdateKeyModal = false;
  let updatingMailbox: Mailbox | null = null;
  let newPublicKey = '';
  let newPrivateKey = '';
  let updatingKey = false;

  async function validatePublicKey(key: string): Promise<boolean> {
    return validateAgePublicKey(key);
  }

  async function updatePublicKey(key: string) {
    publicKey = key;
    if (!key) {
      publicKeyError = '';
    } else if (!(await validatePublicKey(key))) {
      publicKeyError = 'Invalid age public key format.';
    } else {
      publicKeyError = '';
    }
  }

  async function generatePublicKey() {
    generatingKey = true;
    error = null;
    try {
      const keyPair = await generateAgeKeyPair();
      await updatePublicKey(keyPair.publicKey);
      privateKey = keyPair.privateKey;
      // Save private key to localStorage
      savePrivateKey(keyPair.publicKey, keyPair.privateKey);
      showPrivateKeyModal = true;
    } catch (e) {
      error = e;
    } finally {
      generatingKey = false;
    }
  }

  function closePrivateKeyModal() {
    if (!confirm('The private key has been automatically saved to your browser\'s localStorage. We recommend also keeping a backup of this key. Do you want to close this window?')) {
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

  async function copyToClipboard(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      toastMessage = 'Copied to clipboard';
      showToast = true;
      setTimeout(() => {
        showToast = false;
      }, 3000);
    } catch (err) {
      console.error('Failed to copy:', err);
    }
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

  // Add new function for updating the public key
  async function updateMailboxKey(mailbox: Mailbox) {
    if (!validatePublicKey(newPublicKey)) {
      error = new Error('Please provide a valid public key');
      return;
    }

    error = null;
    try {
      const response = await patch<Mailbox>(`/api/mailboxes/${mailbox.id}`, {
        public_key: newPublicKey
      });

      // Update the mailbox in the list
      mailboxes = mailboxes.map(m => m.id === mailbox.id ? response.data! : m);
      
      // Save the new private key if it exists
      if (newPrivateKey) {
        savePrivateKey(newPublicKey, newPrivateKey);
      }

      showUpdateKeyModal = false;
      updatingMailbox = null;
      newPublicKey = '';
      newPrivateKey = '';
      showNotification('Mailbox key updated successfully');
    } catch (e) {
      error = e;
    }
  }

  // Add to the script section after other functions
  async function openUpdateKeyModal(mailbox: Mailbox) {
    updatingMailbox = mailbox;
    newPublicKey = '';
    newPrivateKey = '';
    showUpdateKeyModal = true;
  }
</script>

<style>
  /* Responsive grid layout for mailboxes */
  .mailbox-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(100%, 400px), 1fr));
    gap: 1rem;
    padding: 1rem;
  }

  /* Responsive card styles */
  .mailbox-card {
    @apply bg-base-100 rounded-lg border border-base-200 shadow-sm p-4;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    height: 100%;
  }

  /* Card content wrapper to push button to bottom */
  .mailbox-card-content {
    @apply flex-1 flex flex-col gap-3;
  }

  /* Responsive button groups */
  .button-group {
    @apply flex flex-wrap gap-2 mt-auto pt-2;
  }

  /* Modal responsiveness */
  .modal-content {
    @apply w-full max-w-md mx-auto p-4 sm:p-6;
  }

  @media (max-width: 640px) {
    .input-group {
      @apply flex-col gap-2;
    }
    .input-group > * {
      @apply w-full;
    }
  }
</style>

<div class="container mx-auto px-4 sm:px-6 py-6">
  <div class="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4 mb-6">
    <h1 class="text-2xl font-bold text-base-content">Your Mailboxes</h1>
    <button
      on:click={() => showCreateModal = true}
      class="btn btn-primary"
    >
      <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 20 20" fill="currentColor">
        <path fill-rule="evenodd" d="M10 3a1 1 0 011 1v5h5a1 1 0 110 2h-5v5a1 1 0 11-2 0v-5H4a1 1 0 110-2h5V4a1 1 0 011-1z" clip-rule="evenodd" />
      </svg>
      <span class="ml-2">Create Mailbox</span>
    </button>
  </div>

  {#if error}
    <ErrorAlert error={error} className="mb-4" />
  {/if}

  {#if loading}
    <div class="flex justify-center items-center min-h-[200px]">
      <span class="loading loading-spinner loading-lg text-primary"></span>
    </div>
  {:else if mailboxes.length === 0}
    <div class="text-center py-12">
      <h3 class="text-lg font-medium text-base-content/70">No mailboxes yet</h3>
      <p class="mt-2 text-base-content/50">Create your first mailbox to get started</p>
    </div>
  {:else}
    <div class="mailbox-grid">
      {#each mailboxes as mailbox}
        <div class="mailbox-card">
          <div class="mailbox-card-content">
            <div class="flex justify-between items-start gap-4">
              <div class="flex-1 min-w-0">
                <h3 class="text-lg font-medium truncate" title={mailbox.name}>{mailbox.name}</h3>
              </div>
              <div class="dropdown dropdown-end">
                <button class="btn btn-ghost btn-sm btn-square">
                  <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 20 20" fill="currentColor">
                    <path d="M10 6a2 2 0 110-4 2 2 0 010 4zM10 12a2 2 0 110-4 2 2 0 010 4zM10 18a2 2 0 110-4 2 2 0 010 4z" />
                  </svg>
                </button>
                <ul class="dropdown-content menu p-2 shadow-lg bg-base-100 rounded-box w-52">
                  <li><button on:click={() => openUpdateKeyModal(mailbox)}>Update Key</button></li>
                  <li><button class="text-error" on:click={() => deleteMailbox(mailbox.id)}>Delete</button></li>
                </ul>
              </div>
            </div>

            <!-- Email Address -->
            <div class="space-y-1">
              <div class="text-sm font-medium text-base-content/70">Email Address</div>
              <div class="flex gap-2 items-center bg-base-200/50 p-2 rounded-lg">
                <div class="flex-1 min-w-0 font-mono text-sm">
                  <div class="flex items-center">
                    <span class="text-primary font-medium truncate">{mailbox.alias}</span>
                    {#if supportedDomains.length > 1}
                      <div class="dropdown dropdown-hover">
                        <button class="text-base-content/50">@{selectedDomain}</button>
                        <ul class="dropdown-content z-[1] menu p-2 shadow bg-base-100 rounded-box w-52">
                          {#each supportedDomains as domain}
                            <li><button on:click={() => selectedDomain = domain}>{domain}</button></li>
                          {/each}
                        </ul>
                      </div>
                    {:else}
                      <span class="text-base-content/50">@{supportedDomains[0] || ''}</span>
                    {/if}
                  </div>
                </div>
                <button 
                  class="btn btn-ghost btn-sm btn-square"
                  on:click={() => copyToClipboard(`${mailbox.alias}@${selectedDomain || supportedDomains[0] || ''}`)}
                >
                  <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-12a2 2 0 00-2-2h-2M8 5a2 2 0 002 2h4a2 2 0 002-2M8 5a2 2 0 012-2h4a2 2 0 012 2" />
                  </svg>
                </button>
              </div>
            </div>

            <!-- Mailbox ID -->
            <div class="space-y-1">
              <div class="text-sm font-medium text-base-content/70">Mailbox ID (for API)</div>
              <div class="flex gap-2 items-center bg-base-200/50 p-2 rounded-lg">
                <code class="flex-1 min-w-0 text-sm truncate">{mailbox.id}</code>
                <button 
                  class="btn btn-ghost btn-sm btn-square"
                  on:click={() => copyToClipboard(mailbox.id)}
                >
                  <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-12a2 2 0 00-2-2h-2M8 5a2 2 0 002 2h4a2 2 0 002-2M8 5a2 2 0 012-2h4a2 2 0 012 2" />
                  </svg>
                </button>
              </div>
            </div>

            <!-- Public Key -->
            <div class="space-y-1">
              <div class="text-sm font-medium text-base-content/70">Public Key</div>
              <div class="flex gap-2 items-center bg-base-200/50 p-2 rounded-lg">
                <code class="flex-1 min-w-0 text-sm truncate">{mailbox.public_key}</code>
                <button 
                  class="btn btn-ghost btn-sm btn-square"
                  on:click={() => copyToClipboard(mailbox.public_key)}
                >
                  <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-12a2 2 0 00-2-2h-2M8 5a2 2 0 002 2h4a2 2 0 002-2M8 5a2 2 0 012-2h4a2 2 0 012 2" />
                  </svg>
                </button>
              </div>
            </div>

            <!-- Private Key Status -->
            <div class="space-y-1">
              <div class="text-sm font-medium text-base-content/70">Private Key</div>
              <div class="flex gap-2 items-center bg-base-200/50 p-2 rounded-lg">
                {#if hasPrivateKey(mailbox.public_key)}
                  {@const privateKey = getPrivateKey(mailbox.public_key) || ''}
                  {@const keyContent = privateKey.replace('AGE-SECRET-KEY-', '')}
                  <code class="flex-1 min-w-0 text-sm truncate">AGE-SECRET-KEY-{keyContent.slice(0, 8)}•••••••••••{keyContent.slice(-8)}</code>
                  <button 
                    class="btn btn-ghost btn-sm btn-square"
                    on:click={() => copyToClipboard(privateKey)}
                  >
                    <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-12a2 2 0 00-2-2h-2M8 5a2 2 0 002 2h4a2 2 0 002-2M8 5a2 2 0 012-2h4a2 2 0 012 2" />
                    </svg>
                  </button>
                {:else}
                  <span class="text-base-content/50 text-sm">No private key saved in browser</span>
                {/if}
              </div>
            </div>

            <div class="flex justify-between items-center text-sm">
              <div class="text-base-content/70">
                Created {new Date(mailbox.created_at * 1000).toLocaleDateString()}
              </div>
              {#if mailbox.mail_expires_in}
                <div class="text-warning text-xs sm:text-sm">
                  Emails expire after {formatExpirationTime(mailbox.mail_expires_in)}
                </div>
              {/if}
            </div>
          </div>

          <div class="button-group">
            <a href="/mailboxes/{mailbox.id}" class="btn btn-sm btn-primary flex-1">
              <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 mr-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 8l7.89 5.26a2 2 0 002.22 0L21 8M5 19h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
              </svg>
              View Emails
            </a>
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
        Important: Store this private key securely. It will be saved in your browser but you should keep a backup. Without it, you won't be able to decrypt your emails!
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

<!-- Update Key Modal -->
{#if showUpdateKeyModal && updatingMailbox}
  <div class="modal modal-open">
    <div class="modal-box">
      <h3 class="font-bold text-lg mb-4">Update Mailbox Key</h3>
      
      <ErrorAlert {error} className="mb-4" />

      <div class="alert alert-warning mb-4">
        <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
        </svg>
        <span>Warning: Updating the key will prevent decryption of previously received emails with the old key.</span>
      </div>

      <form on:submit|preventDefault={() => {
        if (updatingMailbox) {
          updateMailboxKey(updatingMailbox);
        }
      }}>
        <div class="form-control mt-4">
          <label class="label" for="public-key">
            <span class="label-text">New Public Key (age format)</span>
          </label>
          <div class="join w-full">
            <input
              type="text"
              id="new-public-key"
              class="input input-bordered join-item w-full"
              bind:value={newPublicKey}
              on:input={(e) => updatePublicKey((e.target as HTMLInputElement).value)}
              placeholder="age1..."
              required
            />
            <button 
              type="button" 
              class="btn"
              on:click|preventDefault={async () => {
                updatingKey = true;
                try {
                  const keyPair = await generateAgeKeyPair();
                  await updatePublicKey(keyPair.publicKey);
                  newPublicKey = keyPair.publicKey;
                  newPrivateKey = keyPair.privateKey;
                } catch (e) {
                  error = e;
                } finally {
                  updatingKey = false;
                }
              }}
              disabled={updatingKey}
            >
              {#if updatingKey}
                <span class="loading loading-spinner loading-sm"></span>
              {:else}
                <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 mr-1" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
                </svg>
                Generate New
              {/if}
            </button>
          </div>
          {#if publicKeyError}
            <div class="text-error text-sm mt-1">{publicKeyError}</div>
          {/if}
        </div>

        {#if newPrivateKey}
          <div class="form-control mt-4">
            <label class="label">
              <span class="label-text">New Private Key</span>
            </label>
            <div class="bg-base-300 p-4 rounded-lg font-mono text-sm break-all">
              {newPrivateKey}
            </div>
            <div class="text-warning text-sm mt-2">
              Important: Store this private key securely. It will be saved in your browser but you should keep a backup. Without it, you won't be able to decrypt your emails!
            </div>
          </div>
        {/if}

        <div class="modal-action">
          <button type="button" class="btn" on:click={() => {
            showUpdateKeyModal = false;
            updatingMailbox = null;
            newPublicKey = '';
            newPrivateKey = '';
          }}>Cancel</button>
          <button type="submit" class="btn btn-primary">Update Key</button>
        </div>
      </form>
    </div>
  </div>
{/if}

<Toast message={toastMessage} visible={showToast} /> 