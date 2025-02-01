<script lang="ts">
  import { onMount } from 'svelte';
  import { get, post } from '$lib/api';

  let currentPassword = '';
  let newPassword = '';
  let confirmNewPassword = '';
  let loading = false;
  let error = '';
  let success = '';

  interface ConnectedAccount {
    provider: string;
    connected_at: string;
  }

  let connectedAccounts: ConnectedAccount[] = [];
  let loadingAccounts = true;

  onMount(async () => {
    try {
      const response = await get('/api/auth/connected-accounts');
      if (!response.ok) {
        throw new Error('Failed to fetch connected accounts');
      }
      connectedAccounts = await response.json();
    } catch (e: any) {
      error = e.message;
    } finally {
      loadingAccounts = false;
    }
  });

  async function changePassword() {
    if (newPassword !== confirmNewPassword) {
      error = 'New passwords do not match';
      return;
    }

    loading = true;
    error = '';
    success = '';

    try {
      const response = await post('/api/auth/change-password', {
        current_password: currentPassword,
        new_password: newPassword,
      });

      if (!response.ok) {
        const data = await response.json();
        throw new Error(data.message || 'Failed to change password');
      }

      success = 'Password changed successfully';
      currentPassword = '';
      newPassword = '';
      confirmNewPassword = '';
    } catch (e: any) {
      error = e.message;
    } finally {
      loading = false;
    }
  }

  async function disconnectAccount(provider: string) {
    if (!confirm(`Are you sure you want to disconnect your ${provider} account?`)) {
      return;
    }

    try {
      const response = await post(`/api/auth/${provider}/disconnect`, {});

      if (!response.ok) {
        throw new Error(`Failed to disconnect ${provider} account`);
      }

      connectedAccounts = connectedAccounts.filter(acc => acc.provider !== provider);
      success = `${provider} account disconnected successfully`;
    } catch (e: any) {
      error = e.message;
    }
  }
</script>

<div class="container mx-auto px-4 py-8 max-w-2xl">
  <h1 class="text-3xl font-bold mb-8">Account Settings</h1>

  {#if error}
    <div class="alert alert-error mb-4">
      <span>{error}</span>
    </div>
  {/if}

  {#if success}
    <div class="alert alert-success mb-4">
      <span>{success}</span>
    </div>
  {/if}

  <div class="card bg-base-200 mb-8">
    <div class="card-body">
      <h2 class="card-title">Change Password</h2>
      <form on:submit|preventDefault={changePassword} class="space-y-4">
        <div class="form-control">
          <label class="label" for="current-password">
            <span class="label-text">Current Password</span>
          </label>
          <input
            type="password"
            id="current-password"
            bind:value={currentPassword}
            class="input input-bordered w-full"
            required
          />
        </div>

        <div class="form-control">
          <label class="label" for="new-password">
            <span class="label-text">New Password</span>
          </label>
          <input
            type="password"
            id="new-password"
            bind:value={newPassword}
            class="input input-bordered w-full"
            required
            minlength="8"
          />
        </div>

        <div class="form-control">
          <label class="label" for="confirm-new-password">
            <span class="label-text">Confirm New Password</span>
          </label>
          <input
            type="password"
            id="confirm-new-password"
            bind:value={confirmNewPassword}
            class="input input-bordered w-full"
            required
            minlength="8"
          />
        </div>

        <button type="submit" class="btn btn-primary w-full" disabled={loading}>
          {#if loading}
            <span class="loading loading-spinner"></span>
          {/if}
          Change Password
        </button>
      </form>
    </div>
  </div>

  <div class="card bg-base-200">
    <div class="card-body">
      <h2 class="card-title mb-4">Connected Accounts</h2>

      {#if loadingAccounts}
        <div class="flex justify-center">
          <span class="loading loading-spinner loading-lg"></span>
        </div>
      {:else}
        <div class="space-y-4">
          <!-- Telegram -->
          {#if connectedAccounts.find(acc => acc.provider === 'telegram')}
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-2">
                <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 496 512">
                  <path fill="currentColor" d="M248 8C111 8 0 119 0 256s111 248 248 248 248-111 248-248S385 8 248 8zm121.8 169.9l-40.7 191.8c-3 13.6-11.1 16.9-22.4 10.5l-62-45.7-29.9 28.8c-3.3 3.3-6.1 6.1-12.5 6.1l4.4-63.1 114.9-103.8c5-4.4-1.1-6.9-7.7-2.5l-142 89.4-61.2-19.1c-13.3-4.2-13.6-13.3 2.8-19.7l239.1-92.2c11.1-4 20.8 2.7 17.2 19.5z"/>
                </svg>
                <span>Telegram</span>
              </div>
              <button
                class="btn btn-sm btn-error"
                on:click={() => disconnectAccount('telegram')}
              >
                Disconnect
              </button>
            </div>
          {:else}
            <a href="/api/auth/telegram" class="btn btn-outline w-full">
              <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 496 512">
                <path fill="currentColor" d="M248 8C111 8 0 119 0 256s111 248 248 248 248-111 248-248S385 8 248 8zm121.8 169.9l-40.7 191.8c-3 13.6-11.1 16.9-22.4 10.5l-62-45.7-29.9 28.8c-3.3 3.3-6.1 6.1-12.5 6.1l4.4-63.1 114.9-103.8c5-4.4-1.1-6.9-7.7-2.5l-142 89.4-61.2-19.1c-13.3-4.2-13.6-13.3 2.8-19.7l239.1-92.2c11.1-4 20.8 2.7 17.2 19.5z"/>
              </svg>
              Connect Telegram
            </a>
          {/if}

          <!-- Google -->
          {#if connectedAccounts.find(acc => acc.provider === 'google')}
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-2">
                <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 488 512">
                  <path fill="currentColor" d="M488 261.8C488 403.3 391.1 504 248 504 110.8 504 0 393.2 0 256S110.8 8 248 8c66.8 0 123 24.5 166.3 64.9l-67.5 64.9C258.5 52.6 94.3 116.6 94.3 256c0 86.5 69.1 156.6 153.7 156.6 98.2 0 135-70.4 140.8-106.9H248v-85.3h236.1c2.3 12.7 3.9 24.9 3.9 41.4z"/>
                </svg>
                <span>Google</span>
              </div>
              <button
                class="btn btn-sm btn-error"
                on:click={() => disconnectAccount('google')}
              >
                Disconnect
              </button>
            </div>
          {:else}
            <a href="/api/auth/oauth/google" class="btn btn-outline w-full">
              <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 488 512">
                <path fill="currentColor" d="M488 261.8C488 403.3 391.1 504 248 504 110.8 504 0 393.2 0 256S110.8 8 248 8c66.8 0 123 24.5 166.3 64.9l-67.5 64.9C258.5 52.6 94.3 116.6 94.3 256c0 86.5 69.1 156.6 153.7 156.6 98.2 0 135-70.4 140.8-106.9H248v-85.3h236.1c2.3 12.7 3.9 24.9 3.9 41.4z"/>
              </svg>
              Connect Google
            </a>
          {/if}
        </div>
      {/if}
    </div>
  </div>
</div> 