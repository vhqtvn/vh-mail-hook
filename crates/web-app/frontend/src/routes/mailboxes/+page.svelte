<script lang="ts">
  import { onMount } from 'svelte';
  import { get, post, del } from '$lib/api';
  import { page } from '$app/stores';
  import { generateAgeKeyPair, validateAgePublicKey, type AgeKeyPair } from '$lib/age';

  interface Mailbox {
    id: string;
    address: string;
    public_key: string;
    owner_id: string;
    expires_at: number | null;
    created_at: number;
  }

  let mailboxes: Mailbox[] = [];
  let loading = true;
  let error = '';
  let showCreateModal = false;
  let newMailboxDescription = '';
  
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
    error = ''; // Clear any previous errors
    try {
      const keyPair = await generateAgeKeyPair();
      console.log('Generated key pair:', keyPair); // Debug log
      updatePublicKey(keyPair.publicKey);
      privateKey = keyPair.privateKey;
      showPrivateKeyModal = true;
    } catch (e: any) {
      console.error('Error generating key:', e); // Debug log
      error = `Failed to generate key: ${e.message}`;
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
      const response = await get('/api/mailboxes');
      if (!response.ok) {
        throw new Error('Failed to fetch mailboxes');
      }
      const apiResponse = await response.json();
      if (!apiResponse.success) {
        throw new Error(apiResponse.error || 'Failed to fetch mailboxes');
      }
      mailboxes = apiResponse.data || [];
    } catch (e: any) {
      error = e.message;
    } finally {
      loading = false;
    }
  });

  async function createMailbox() {
    if (!validatePublicKey(publicKey)) {
      error = 'Please provide a valid public key';
      return;
    }

    try {
      const response = await post('/api/mailboxes', {
        expires_in_days: expirationDays,
        public_key: publicKey
      });

      if (!response.ok) {
        throw new Error('Failed to create mailbox');
      }

      const apiResponse = await response.json();
      if (!apiResponse.success) {
        throw new Error(apiResponse.error || 'Failed to create mailbox');
      }

      mailboxes = [...mailboxes, apiResponse.data];
      showCreateModal = false;
      newMailboxDescription = '';
      publicKey = '';
      expirationDays = 7;
      expirationHours = 0;
      expirationMinutes = 0;
      expirationSeconds = 0;
    } catch (e: any) {
      error = e.message;
    }
  }

  async function deleteMailbox(id: string) {
    if (!confirm('Are you sure you want to delete this mailbox?')) {
      return;
    }

    try {
      const response = await del(`/api/mailboxes/${id}`);

      if (!response.ok) {
        throw new Error('Failed to delete mailbox');
      }

      mailboxes = mailboxes.filter(m => m.id !== id);
    } catch (e: any) {
      error = e.message;
    }
  }
</script>

<div class="container mx-auto px-4">
  <div class="flex justify-between items-center mb-8">
    <h1 class="text-3xl font-bold">Your Mailboxes</h1>
    {#if mailboxes.length > 0}
      <button class="btn btn-primary" on:click={() => showCreateModal = true}>
        Create New Mailbox
      </button>
    {/if}
  </div>

  {#if error}
    <div class="alert alert-error mb-4">
      <span>{error}</span>
    </div>
  {/if}
  {#if loading}
    <div class="flex justify-center">
      <span class="loading loading-spinner loading-lg"></span>
    </div>
  {:else if mailboxes.length === 0}
    <div class="flex flex-col items-center justify-center py-16 px-4">
      <div class="w-24 h-24 mb-6 text-base-content/30">
        <!-- Heroicon: inbox -->
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" d="M2.25 13.5h3.86a2.25 2.25 0 012.012 1.244l.256.512a2.25 2.25 0 002.013 1.244h3.218a2.25 2.25 0 002.013-1.244l.256-.512a2.25 2.25 0 012.013-1.244h3.859m-19.5.338V18a2.25 2.25 0 002.25 2.25h15A2.25 2.25 0 0021.75 18v-4.162c0-.224-.034-.447-.1-.661L19.24 5.338a2.25 2.25 0 00-2.15-1.588H6.911a2.25 2.25 0 00-2.15 1.588L2.35 13.177a2.25 2.25 0 00-.1.661z" />
        </svg>
      </div>
      <h3 class="text-2xl font-semibold mb-2">No mailboxes yet</h3>
      <p class="text-base-content/60 text-center max-w-md mb-8">Create your first disposable mailbox to start receiving emails securely and privately.</p>
      <button class="btn btn-primary" on:click={() => showCreateModal = true}>
        Create Your First Mailbox
      </button>
    </div>
  {:else}
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      {#each mailboxes as mailbox (mailbox.id)}
        <div class="card bg-base-200">
          <div class="card-body">
            <h2 class="card-title font-mono text-sm break-all">{mailbox.address}</h2>
            <p class="text-base-content/70">{mailbox.public_key}</p>
            <p class="text-sm text-base-content/50">
              Created {new Date(mailbox.created_at * 1000).toLocaleDateString()}
            </p>
            <div class="card-actions justify-end mt-4">
              <button
                class="btn btn-sm btn-error"
                on:click={() => deleteMailbox(mailbox.id)}
              >
                Delete
              </button>
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<!-- Private Key Modal -->
{#if showPrivateKeyModal}
  <div class="modal modal-open" style="z-index: 200">
    <div class="modal-box relative" style="z-index: 202">
      <h3 class="font-bold text-lg mb-4 text-warning">Save Your Private Key</h3>
      <p class="mb-4 text-warning">
        This is your only chance to save the private key. You will need it to decrypt emails sent to this mailbox.
        Store it securely and don't share it with anyone!
      </p>
      <div class="form-control">
        <label class="label">
          <span class="label-text">Private Key</span>
        </label>
        <textarea
          class="textarea textarea-bordered font-mono text-sm h-32"
          readonly
          value={privateKey}
        />
      </div>
      <div class="flex gap-2 mt-4">
        <button
          class="btn btn-sm"
          on:click={() => navigator.clipboard.writeText(privateKey)}
        >
          Copy to Clipboard
        </button>
        <a
          class="btn btn-sm"
          href={`data:text/plain;charset=utf-8,${encodeURIComponent(privateKey)}`}
          download="mailbox-private-key.txt"
        >
          Download as File
        </a>
      </div>
      <div class="modal-action">
        <button class="btn btn-warning" on:click={closePrivateKeyModal}>
          I Have Saved My Private Key
        </button>
      </div>
    </div>
    <div class="modal-backdrop" style="z-index: 201"></div>
  </div>
{/if}

<!-- Create Mailbox Modal -->
{#if showCreateModal}
  <div class="modal modal-open" style="z-index: 100">
    <div class="modal-box max-w-2xl" style="z-index: 102">
      <h3 class="font-bold text-lg mb-4">Create New Mailbox</h3>
      <form on:submit|preventDefault={createMailbox}>
        <div class="form-control">
          <label class="label" for="description">
            <span class="label-text">Description</span>
          </label>
          <input
            type="text"
            id="description"
            bind:value={newMailboxDescription}
            class="input input-bordered w-full"
            placeholder="e.g., Shopping, Social Media, etc."
            required
          />
        </div>

        <div class="form-control mt-4">
          <label class="label">
            <span class="label-text">Expiration Time</span>
          </label>
          <div class="grid grid-cols-4 gap-2">
            <div>
              <label class="label">
                <span class="label-text-alt">Days</span>
              </label>
              <input
                type="number"
                bind:value={expirationDays}
                class="input input-bordered w-full"
                min="0"
                max="365"
                required
              />
            </div>
            <div>
              <label class="label">
                <span class="label-text-alt">Hours</span>
              </label>
              <input
                type="number"
                bind:value={expirationHours}
                class="input input-bordered w-full"
                min="0"
                max="23"
                required
              />
            </div>
            <div>
              <label class="label">
                <span class="label-text-alt">Minutes</span>
              </label>
              <input
                type="number"
                bind:value={expirationMinutes}
                class="input input-bordered w-full"
                min="0"
                max="59"
                required
              />
            </div>
            <div>
              <label class="label">
                <span class="label-text-alt">Seconds</span>
              </label>
              <input
                type="number"
                bind:value={expirationSeconds}
                class="input input-bordered w-full"
                min="0"
                max="59"
                required
              />
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
          <button type="button" class="btn" on:click={() => showCreateModal = false}>
            Cancel
          </button>
          <button 
            type="submit" 
            class="btn btn-primary"
            disabled={!!publicKeyError || generatingKey}
          >
            Create
          </button>
        </div>
      </form>
    </div>
    <div class="modal-backdrop" style="z-index: 101"></div>
  </div>
{/if} 