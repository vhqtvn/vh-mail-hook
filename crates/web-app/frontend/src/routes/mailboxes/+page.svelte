<script lang="ts">
  import { onMount } from 'svelte';
  import { get, post, del } from '$lib/api';
  import { page } from '$app/stores';
  import { generateAgeKeyPair, validateAgePublicKey, type AgeKeyPair } from '$lib/age';
  import ErrorAlert from '$lib/components/ErrorAlert.svelte';

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

  onMount(async () => {
    try {
      const response = await get<Mailbox[]>('/api/mailboxes');
      mailboxes = response.data || [];
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
    <div class="grid gap-4 grid-cols-1 md:grid-cols-2 lg:grid-cols-3">
      {#each mailboxes as mailbox (mailbox.id)}
        <div class="card bg-base-200">
          <div class="card-body">
            <div class="flex justify-between items-start">
              <div>
                <h2 class="card-title">{mailbox.name}</h2>
                <div class="text-sm text-base-content/70 break-all">{mailbox.address}</div>
              </div>
              <button 
                class="btn btn-square btn-sm btn-ghost text-error" 
                on:click={() => deleteMailbox(mailbox.id)}
              >
                <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                </svg>
              </button>
            </div>
            <div class="text-sm text-base-content/70">
              Created {new Date(mailbox.created_at * 1000).toLocaleDateString()}
            </div>
            {#if mailbox.mail_expires_in}
              <div class="text-sm text-warning">
                Emails expire after {Math.floor(mailbox.mail_expires_in / 86400)} days
              </div>
            {/if}
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
          <label class="label">
            <span class="label-text">Mailbox Name</span>
          </label>
          <input
            type="text"
            bind:value={mailboxName}
            class="input input-bordered w-full"
            placeholder="My Mailbox"
            required
          />
        </div>

        <div class="form-control mt-4">
          <label class="label">
            <span class="label-text">Expiration Time (max 30 days)</span>
          </label>
          <div class="grid grid-cols-4 gap-2">
            <div>
              <input
                type="number"
                bind:value={expirationDays}
                class="input input-bordered w-full"
                min="0"
                max="30"
                placeholder="Days"
              />
              <label class="label">
                <span class="label-text-alt">Days</span>
              </label>
            </div>
            <div>
              <input
                type="number"
                bind:value={expirationHours}
                class="input input-bordered w-full"
                min="0"
                max="23"
                placeholder="Hours"
              />
              <label class="label">
                <span class="label-text-alt">Hours</span>
              </label>
            </div>
            <div>
              <input
                type="number"
                bind:value={expirationMinutes}
                class="input input-bordered w-full"
                min="0"
                max="59"
                placeholder="Mins"
              />
              <label class="label">
                <span class="label-text-alt">Minutes</span>
              </label>
            </div>
            <div>
              <input
                type="number"
                bind:value={expirationSeconds}
                class="input input-bordered w-full"
                min="0"
                max="59"
                placeholder="Secs"
              />
              <label class="label">
                <span class="label-text-alt">Seconds</span>
              </label>
            </div>
          </div>
        </div>

        <div class="form-control mt-4">
          <label class="label">
            <span class="label-text">Public Key (age format)</span>
          </label>
          <div class="flex gap-2">
            <input
              type="text"
              bind:value={publicKey}
              on:input={(e) => updatePublicKey((e.target as HTMLInputElement).value)}
              class="input input-bordered flex-1 font-mono text-sm"
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
                Generate
              {/if}
            </button>
          </div>
          {#if publicKeyError}
            <label class="label">
              <span class="label-text-alt text-error">{publicKeyError}</span>
            </label>
          {/if}
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
      <div class="modal-action">
        <button class="btn btn-primary" on:click={closePrivateKeyModal}>I've Saved It</button>
      </div>
    </div>
  </div>
{/if} 