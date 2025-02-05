<script lang="ts">
  import { onMount } from 'svelte';
  import { type ApiKey, listApiKeys, createApiKey, deleteApiKey } from '$lib/api';
  import ErrorAlert from '$lib/components/ErrorAlert.svelte';

  let apiKeys: ApiKey[] = [];
  let loading = false;
  let error: unknown | null = null;
  let success = '';

  async function fetchApiKeys() {
    try {
      const response = await listApiKeys();
      apiKeys = response.data || [];
    } catch (e) {
      error = e;
    } finally {
      loading = false;
    }
  }

  async function handleCreateApiKey() {
    try {
      const response = await createApiKey();
      if (response.data) {
        apiKeys = [...apiKeys, response.data];
        success = 'API key created successfully';
      }
    } catch (e) {
      error = e;
    }
  }

  async function handleDeleteApiKey(keyId: string) {
    if (!confirm('Are you sure you want to delete this API key? Any applications using it will stop working.')) {
      return;
    }

    try {
      await deleteApiKey(keyId);
      apiKeys = apiKeys.filter(key => key.id !== keyId);
      success = 'API key deleted successfully';
    } catch (e) {
      error = e;
    }
  }

  onMount(() => {
    loading = true;
    fetchApiKeys();
  });
</script>

<div class="container mx-auto px-4 py-8 max-w-4xl">
  <div class="flex justify-between items-center mb-8">
    <h1 class="text-3xl font-bold">API Keys</h1>
    <button
      class="btn btn-primary"
      on:click={handleCreateApiKey}
    >
      Create API Key
    </button>
  </div>

  {#if error}
    <div class="mb-4">
      <ErrorAlert {error} />
    </div>
  {/if}

  {#if success}
    <div class="alert alert-success mb-4">
      <span>{success}</span>
    </div>
  {/if}

  <div class="card bg-base-200">
    <div class="card-body">
      <p class="text-base mb-6">
        API keys allow you to authenticate with the API programmatically. Keep them secure and never share them.
        Each key has full access to your account, so treat them like passwords.
        View the <a href="/api/docs" class="link link-primary" target="_blank" rel="noopener noreferrer">API documentation</a> to learn how to use these keys.
        For code examples and implementation guides, check out our <a href="https://github.com/vhqtvn/vh-mail-hook/tree/main/examples" class="link link-primary" target="_blank" rel="noopener noreferrer">examples repository</a>.
      </p>

      {#if loading}
        <div class="flex justify-center py-8">
          <span class="loading loading-spinner loading-lg"></span>
        </div>
      {:else}
        {#if apiKeys.length > 0}
          <div class="overflow-x-auto">
            <table class="table">
              <thead>
                <tr>
                  <th>Key</th>
                  <th>Created</th>
                  <th class="text-right">Actions</th>
                </tr>
              </thead>
              <tbody>
                {#each apiKeys as key}
                  <tr>
                    <td class="font-mono text-sm">{key.key}</td>
                    <td>{new Date(key.created_at * 1000).toLocaleString()}</td>
                    <td class="text-right">
                      <button
                        class="btn btn-error btn-sm"
                        on:click={() => handleDeleteApiKey(key.id)}
                      >
                        Delete
                      </button>
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {:else}
          <div class="text-center py-8 text-base-content/60">
            <p>You haven't created any API keys yet.</p>
          </div>
        {/if}
      {/if}
    </div>
  </div>
</div> 