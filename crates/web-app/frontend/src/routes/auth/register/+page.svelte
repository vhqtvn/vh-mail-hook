<script lang="ts">
  import { post, setAuthToken } from '$lib/api';
  import { auth } from '$lib/stores/auth';
  import ErrorAlert from '$lib/components/ErrorAlert.svelte';
  import TelegramLoginWidget from '$lib/components/TelegramLoginWidget.svelte';
  import GoogleLoginButton from '$lib/components/GoogleLoginButton.svelte';
  import GitHubLoginButton from '$lib/components/GitHubLoginButton.svelte';

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
    <TelegramLoginWidget 
      action="register"
      onError={(e) => error = e} 
    />
    <GoogleLoginButton action="register" onError={(e) => error = e} />
    <GitHubLoginButton action="register" onError={(e) => error = e} />
  </div>

  <p class="text-center mt-4">
    Already have an account?
    <a href="/auth/login" class="link link-primary">Sign in</a>
  </p>
</div> 