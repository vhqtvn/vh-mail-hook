<script lang="ts">
  import { onMount } from 'svelte';
  import { get, post } from '$lib/api';
  import ErrorAlert from '$lib/components/ErrorAlert.svelte';
  import TelegramLoginWidget from '$lib/components/TelegramLoginWidget.svelte';
  import GoogleLoginButton from '$lib/components/GoogleLoginButton.svelte';
  import GitHubLoginButton from '$lib/components/GitHubLoginButton.svelte';
  import { fade } from 'svelte/transition';
  import { elasticOut } from 'svelte/easing';

  let currentPassword = '';
  let newPassword = '';
  let confirmNewPassword = '';
  let deleteAccountPassword = '';
  let loading = false;
  let error: unknown | null = null;
  let success = '';
  let hasPassword = false;
  let errorElement: HTMLElement;
  let shakeKey = 0;

  interface ConnectedAccount {
    provider: string;
    connected_at: string;
  }

  let connectedAccounts: ConnectedAccount[] = [];
  let loadingAccounts = true;

  async function fetchConnectedAccounts() {
    try {
      const response = await get<ConnectedAccount[]>('/api/auth/connected-accounts');
      connectedAccounts = response.data || [];
      hasPassword = connectedAccounts.some(acc => acc.provider === 'password');
    } catch (e) {
      error = e;
    } finally {
      loadingAccounts = false;
    }
  }

  onMount(fetchConnectedAccounts);

  async function handleTelegramConnect() {
    await fetchConnectedAccounts();
    success = 'Telegram account connected successfully';
  }

  async function handleGoogleConnect() {
    await fetchConnectedAccounts();
    success = 'Google account connected successfully';
  }

  async function handleGitHubConnect() {
    await fetchConnectedAccounts();
    success = 'GitHub account connected successfully';
  }

  async function setPassword() {
    if (newPassword !== confirmNewPassword) {
      error = new Error('Passwords do not match');
      return;
    }

    loading = true;
    error = null;
    success = '';

    try {
      await post('/api/auth/set-password', {
        new_password: newPassword,
      });

      success = 'Password set successfully';
      newPassword = '';
      confirmNewPassword = '';
      hasPassword = true;
    } catch (e) {
      error = e;
    } finally {
      loading = false;
    }
  }

  async function changePassword() {
    if (newPassword !== confirmNewPassword) {
      error = new Error('New passwords do not match');
      return;
    }

    loading = true;
    error = null;
    success = '';

    try {
      await post('/api/auth/change-password', {
        current_password: currentPassword,
        new_password: newPassword,
      });

      success = 'Password changed successfully';
      currentPassword = '';
      newPassword = '';
      confirmNewPassword = '';
    } catch (e) {
      error = e;
    } finally {
      loading = false;
    }
  }

  async function disconnectAccount(provider: string) {
    if (!confirm(`Are you sure you want to disconnect your ${provider} account?`)) {
      return;
    }

    error = null;
    success = '';
    try {
      const endpoint = provider === 'telegram' ? '/api/auth/telegram/disconnect' : `/api/auth/${provider}/disconnect`;
      await post(endpoint, {}, { requireAuth: true });
      await fetchConnectedAccounts();
      success = `${provider} account disconnected successfully`;
    } catch (e) {
      error = e;
    }
  }

  async function handleDeleteAccount() {
    if (hasPassword && !deleteAccountPassword) {
      error = new Error('Please enter your password to confirm account deletion');
      return;
    }
    if (!window.confirm('Are you absolutely sure you want to delete your account? This action cannot be undone.')) {
      return;
    }
    
    loading = true;
    error = null;
    try {
      await post('/api/auth/delete-account', hasPassword ? {
        password: deleteAccountPassword
      } : {});
      // Redirect to home and clear session
      window.localStorage.removeItem('token');
      window.location.href = '/';
    } catch (e) {
      error = e;
      loading = false;
    }
  }

  async function disconnectGoogle() {
    try {
      await post('/api/auth/google/disconnect', {});
      await fetchConnectedAccounts();
      success = 'Google account disconnected successfully';
    } catch (e) {
      error = e;
    }
  }

  async function disconnectGitHub() {
    try {
      await post('/api/auth/github/disconnect', {});
      await fetchConnectedAccounts();
      success = 'GitHub account disconnected successfully';
    } catch (e) {
      error = e;
    }
  }

  function scrollIntoView(node: HTMLElement) {
    errorElement = node;
    setTimeout(() => {
      node.scrollIntoView({ behavior: 'smooth', block: 'center' });
      // Force animation restart
      node.classList.remove('shake-animation');
      void node.offsetWidth; // Trigger reflow
      node.classList.add('shake-animation');
    }, 100);
    return {
      destroy() {}
    };
  }

  $: if (error) {
    shakeKey++; // Force animation restart when error changes
  }
</script>

<style>
  @keyframes shake {
    0%, 100% { transform: translateX(0); }
    10%, 30%, 50%, 70%, 90% { transform: translateX(-4px); }
    20%, 40%, 60%, 80% { transform: translateX(4px); }
  }

  .error-container {
    margin-bottom: 1rem;
  }

  .shake-animation {
    animation: shake 0.5s cubic-bezier(.36,.07,.19,.97) both;
  }
</style>

<div class="container mx-auto px-4 py-8 max-w-2xl">
  <h1 class="text-3xl font-bold mb-8">Account Settings</h1>

  {#if error}
    <div 
      class="error-container shake-animation"
      use:scrollIntoView
    >
      <ErrorAlert {error} />
    </div>
  {/if}
  {#if success}
    <div class="alert alert-success mb-4">
      <span>{success}</span>
    </div>
  {/if}

  <div class="card bg-base-200 mb-8">
    <div class="card-body">
      <h2 class="card-title">{hasPassword ? 'Change Password' : 'Set Password'}</h2>
      {#if !hasPassword}
        <p class="text-sm mb-4">
          Setting a password allows you to log in with your username and password, and is required for some account operations.
        </p>
      {/if}
      <form on:submit|preventDefault={hasPassword ? changePassword : setPassword} class="space-y-4">
        {#if hasPassword}
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
              autocomplete="current-password"
            />
          </div>
        {/if}

        <div class="form-control">
          <label class="label" for="new-password">
            <span class="label-text">{hasPassword ? 'New Password' : 'Password'}</span>
          </label>
          <input
            type="password"
            id="new-password"
            bind:value={newPassword}
            class="input input-bordered w-full"
            required
            minlength="8"
            autocomplete="new-password"
          />
        </div>

        <div class="form-control">
          <label class="label" for="confirm-new-password">
            <span class="label-text">Confirm {hasPassword ? 'New ' : ''}Password</span>
          </label>
          <input
            type="password"
            id="confirm-new-password"
            bind:value={confirmNewPassword}
            class="input input-bordered w-full"
            required
            minlength="8"
            autocomplete="new-password"
          />
        </div>

        <button type="submit" class="btn btn-primary w-full" disabled={loading}>
          {#if loading}
            <span class="loading loading-spinner"></span>
          {/if}
          {hasPassword ? 'Change Password' : 'Set Password'}
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
            <div class="flex items-center justify-between p-4 bg-base-300 rounded-lg">
              <div class="flex items-center gap-2">
                <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 496 512">
                  <path fill="currentColor" d="M248 8C111 8 0 119 0 256s111 248 248 248 248-111 248-248S385 8 248 8zm121.8 169.9l-40.7 191.8c-3 13.6-11.1 16.9-22.4 10.5l-62-45.7-29.9 28.8c-3.3 3.3-6.1 6.1-12.5 6.1l4.4-63.1 114.9-103.8c5-4.4-1.1-6.9-7.7-2.5l-142 89.4-61.2-19.1c-13.3-4.2-13.6-13.3 2.8-19.7l239.1-92.2c11.1-4 20.8 2.7 17.2 19.5z"/>
                </svg>
                <div>
                  <span class="font-medium">Telegram</span>
                  <div class="text-sm opacity-70">Connected</div>
                </div>
              </div>
              <button
                class="btn btn-sm btn-error"
                on:click={() => disconnectAccount('telegram')}
              >
                Disconnect
              </button>
            </div>
          {:else}
            <div class="flex flex-col items-center gap-4 p-4 bg-base-300 rounded-lg">
              <div class="text-center">
                <h3 class="font-medium mb-2">Connect Telegram</h3>
                <p class="text-sm opacity-70 mb-4">Link your Telegram account to enable notifications and quick access.</p>
              </div>
              {#if import.meta.env.VITE_TELEGRAM_BOT_NAME}
                <TelegramLoginWidget 
                  botName={import.meta.env.VITE_TELEGRAM_BOT_NAME} 
                  action="connect"
                  onSuccess={handleTelegramConnect}
                />
              {:else}
                <div class="text-error text-sm text-center">
                  Telegram login is not configured (VITE_TELEGRAM_BOT_NAME not set)
                </div>
              {/if}
            </div>
          {/if}

          <!-- Google -->
          {#if connectedAccounts.find(acc => acc.provider === 'google')}
            <div class="flex items-center justify-between p-4 bg-base-300 rounded-lg">
              <div class="flex items-center gap-2">
                <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 488 512">
                  <path fill="currentColor" d="M488 261.8C488 403.3 391.1 504 248 504 110.8 504 0 393.2 0 256S110.8 8 248 8c66.8 0 123 24.5 166.3 64.9l-67.5 64.9C258.5 52.6 94.3 116.6 94.3 256c0 86.5 69.1 156.6 153.7 156.6 98.2 0 135-70.4 140.8-106.9H248v-85.3h236.1c2.3 12.7 3.9 24.9 3.9 41.4z"/>
                </svg>
                <div>
                  <span class="font-medium">Google</span>
                  <div class="text-sm opacity-70">Connected</div>
                </div>
              </div>
              <button
                class="btn btn-sm btn-error"
                on:click={disconnectGoogle}
              >
                Disconnect
              </button>
            </div>
          {:else}
            <div class="flex flex-col items-center gap-4 p-4 bg-base-300 rounded-lg">
              <GoogleLoginButton action="connect" onSuccess={handleGoogleConnect} onError={(e) => error = e} />
            </div>
          {/if}

          <!-- GitHub -->
          {#if connectedAccounts.find(acc => acc.provider === 'github')}
            <div class="flex items-center justify-between p-4 bg-base-300 rounded-lg">
              <div class="flex items-center gap-2">
                <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 488 512">
                  <path fill="currentColor" d="M488 261.8C488 403.3 391.1 504 248 504 110.8 504 0 393.2 0 256S110.8 8 248 8c66.8 0 123 24.5 166.3 64.9l-67.5 64.9C258.5 52.6 94.3 116.6 94.3 256c0 86.5 69.1 156.6 153.7 156.6 98.2 0 135-70.4 140.8-106.9H248v-85.3h236.1c2.3 12.7 3.9 24.9 3.9 41.4z"/>
                </svg>
                <div>
                  <span class="font-medium">GitHub</span>
                  <div class="text-sm opacity-70">Connected</div>
                </div>
              </div>
              <button
                class="btn btn-sm btn-error"
                on:click={() => disconnectAccount('github')}
              >
                Disconnect
              </button>
            </div>
          {:else}
            <div class="flex flex-col items-center gap-4 p-4 bg-base-300 rounded-lg">
              <GitHubLoginButton action="connect" onSuccess={handleGitHubConnect} onError={(e) => error = e} />
            </div>
          {/if}
        </div>
      {/if}
    </div>
  </div>

  <div class="card bg-base-200 mt-8">
    <div class="card-body">
      <h2 class="card-title text-error">Delete Account</h2>
      <p class="text-sm mb-4">
        This action cannot be undone. All your data will be permanently deleted.
      </p>
      {#if hasPassword}
        <div class="form-control mb-4">
          <label class="label" for="delete-account-password">
            <span class="label-text">Enter your password to confirm</span>
          </label>
          <input
            type="password"
            id="delete-account-password"
            bind:value={deleteAccountPassword}
            class="input input-bordered w-full"
            required
            autocomplete="current-password"
          />
        </div>
      {/if}
      <button
        class="btn btn-error w-full"
        on:click={handleDeleteAccount}
        disabled={loading}
      >
        {#if loading}
          <span class="loading loading-spinner"></span>
        {/if}
        Delete Account
      </button>
    </div>
  </div>
</div> 