<script lang="ts">
  import { post, setAuthToken } from '$lib/api';
  import { auth } from '$lib/stores/auth';
  import ErrorAlert from '$lib/components/ErrorAlert.svelte';
  import TelegramLoginWidget from '$lib/components/TelegramLoginWidget.svelte';

  let username = '';
  let password = '';
  let confirmPassword = '';
  let loading = false;
  let error: unknown | null = null;

  async function handleSubmit() {
    loading = true;
    error = null;

    if (password !== confirmPassword) {
      error = new Error('Passwords do not match');
      loading = false;
      return;
    }
    
    try {
      const response = await post('/api/auth/register', 
        { username, password },
        { requireAuth: false }
      );

      if (!response.success || !response.data) {
        throw new Error('Registration failed');
      }

      await auth.register(response.data.token, response.data.user);
    } catch (e) {
      error = e;
    } finally {
      loading = false;
    }
  }
</script>

<div class="max-w-md mx-auto">
  <h1 class="text-3xl font-bold text-center mb-8">Create Account</h1>

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
        minlength="8"
        autocomplete="new-password"
      />
    </div>

    <div class="form-control">
      <label class="label" for="confirm-password">
        <span class="label-text">Confirm Password</span>
      </label>
      <input
        type="password"
        id="confirm-password"
        bind:value={confirmPassword}
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
      Create Account
    </button>
  </form>

  <div class="divider">OR</div>

  <div class="space-y-4">
    {#if import.meta.env.VITE_TELEGRAM_BOT_NAME}
      <TelegramLoginWidget 
        botName={import.meta.env.VITE_TELEGRAM_BOT_NAME} 
        action="register"
      />
    {:else}
      <div class="text-error text-sm text-center">
        Telegram login is not configured (VITE_TELEGRAM_BOT_NAME not set)
      </div>
    {/if}

    <a href="/api/auth/oauth/google" class="btn w-full">
      <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 488 512">
        <path fill="currentColor" d="M488 261.8C488 403.3 391.1 504 248 504 110.8 504 0 393.2 0 256S110.8 8 248 8c66.8 0 123 24.5 166.3 64.9l-67.5 64.9C258.5 52.6 94.3 116.6 94.3 256c0 86.5 69.1 156.6 153.7 156.6 98.2 0 135-70.4 140.8-106.9H248v-85.3h236.1c2.3 12.7 3.9 24.9 3.9 41.4z"/>
      </svg>
      Continue with Google
    </a>
  </div>

  <p class="text-center mt-4">
    Already have an account?
    <a href="/auth/login" class="link link-primary">Sign in</a>
  </p>
</div> 