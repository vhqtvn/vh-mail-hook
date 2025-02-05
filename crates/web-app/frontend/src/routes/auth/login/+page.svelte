<script lang="ts">
  import { post, setAuthToken } from '$lib/api';
  import ErrorAlert from '$lib/components/ErrorAlert.svelte';
  import { auth } from '$lib/stores/auth';
  import { goto, invalidateAll } from '$app/navigation';
  import TelegramLoginWidget from '$lib/components/TelegramLoginWidget.svelte';
  import GoogleLoginButton from '$lib/components/GoogleLoginButton.svelte';
  import GitHubLoginButton from '$lib/components/GitHubLoginButton.svelte';
  import { getTelegramBotName } from '$lib/config';

  const botName = getTelegramBotName();
  let username = '';
  let password = '';
  let loading = false;
  let error: unknown | null = null;

  async function handleSubmit() {
    loading = true;
    error = null;
    
    try {
      const response = await post('/api/auth/login', 
        { username, password },
        { requireAuth: false }
      );

      if (!response.success || !response.data) {
        throw new Error('Login failed');
      }

      await auth.login(response.data.token, response.data.user);
    } catch (e) {
      error = e;
    } finally {
      loading = false;
    }
  }
</script>

<div class="max-w-md mx-auto">
  <h1 class="text-3xl font-bold text-center mb-8">Sign In</h1>

  <form on:submit|preventDefault={handleSubmit} class="space-y-4">
    <ErrorAlert {error} />

    <div class="form-control">
      <label class="label" for="username">
        <span class="label-text">Username</span>
      </label>
      <input
        type="text"
        id="username"
        bind:value={username}
        class="input input-bordered w-full"
        required
        autocomplete="username"
      />
    </div>

    <div class="form-control">
      <label class="label" for="password">
        <span class="label-text">Password</span>
      </label>
      <input
        type="password"
        id="password"
        bind:value={password}
        class="input input-bordered w-full"
        required
        autocomplete="current-password"
      />
    </div>

    <button type="submit" class="btn btn-primary w-full" disabled={loading}>
      {#if loading}
        <span class="loading loading-spinner"></span>
      {/if}
      Sign In
    </button>
  </form>

  <div class="divider">OR</div>

  <div class="space-y-4">
    {#if botName}
      <TelegramLoginWidget 
        botName={botName} 
        action="login"
        onError={(e) => error = e}
      />
    {:else}
      <div class="text-error text-sm text-center">
        Telegram login is not configured (TELEGRAM_BOT_NAME not set)
      </div>
    {/if}

    <GoogleLoginButton action="login" onError={(e) => error = e} />
    <GitHubLoginButton action="login" onError={(e) => error = e} />
  </div>

  <p class="text-center mt-4">
    Don't have an account?
    <a href="/auth/register" class="link link-primary">Sign up</a>
  </p>
</div> 