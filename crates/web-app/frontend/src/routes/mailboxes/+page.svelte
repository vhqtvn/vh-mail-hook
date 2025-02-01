<script lang="ts">
  import { onMount } from 'svelte';
  import { get, post, del } from '$lib/api';
  import { page } from '$app/stores';

  interface Mailbox {
    id: string;
    email: string;
    description: string;
    created_at: string;
  }

  let mailboxes: Mailbox[] = [];
  let loading = true;
  let error = '';
  let showCreateModal = false;
  let newMailboxDescription = '';
  let expiresInDays = 7; // Default expiration of 7 days

  onMount(async () => {
    try {
      const response = await get('/api/mailboxes');
      if (!response.ok) {
        throw new Error('Failed to fetch mailboxes');
      }
      mailboxes = await response.json();
    } catch (e: any) {
      error = e.message;
    } finally {
      loading = false;
    }
  });

  async function createMailbox() {
    try {
      const response = await post('/api/mailboxes', {
        description: newMailboxDescription,
        expires_in_days: expiresInDays,
        owner_id: $page.data.user.id,
        public_key: "age1creym8a9ncefdvplrqrfy7wf8k3fw2l7w5z7nwp03jgfyhc56gcqgq27cg" // TODO: Generate or get from user
      });

      if (!response.ok) {
        throw new Error('Failed to create mailbox');
      }

      const newMailbox = await response.json();
      mailboxes = [...mailboxes, newMailbox];
      showCreateModal = false;
      newMailboxDescription = '';
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
    <button class="btn btn-primary" on:click={() => showCreateModal = true}>
      Create New Mailbox
    </button>
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
    <div class="text-center py-8">
      <h3 class="text-xl mb-2">No mailboxes yet</h3>
      <p class="text-base-content/60">Create your first mailbox to get started</p>
    </div>
  {:else}
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      {#each mailboxes as mailbox (mailbox.id)}
        <div class="card bg-base-200">
          <div class="card-body">
            <h2 class="card-title font-mono text-sm break-all">{mailbox.email}</h2>
            <p class="text-base-content/70">{mailbox.description}</p>
            <p class="text-sm text-base-content/50">
              Created {new Date(mailbox.created_at).toLocaleDateString()}
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

<!-- Create Mailbox Modal -->
{#if showCreateModal}
  <div class="modal modal-open">
    <div class="modal-box">
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
          <label class="label" for="expires">
            <span class="label-text">Expires in (days)</span>
          </label>
          <input
            type="number"
            id="expires"
            bind:value={expiresInDays}
            class="input input-bordered w-full"
            min="1"
            max="365"
            required
          />
        </div>
        <div class="modal-action">
          <button type="button" class="btn" on:click={() => showCreateModal = false}>
            Cancel
          </button>
          <button type="submit" class="btn btn-primary">Create</button>
        </div>
      </form>
    </div>
    <div class="modal-backdrop" on:click={() => showCreateModal = false}></div>
  </div>
{/if} 