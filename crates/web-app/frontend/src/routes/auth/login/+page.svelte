<script lang="ts">
  import { post } from '$lib/api';

  let username = '';
  let password = '';
  let loading = false;
  let error = '';

  async function handleSubmit() {
    loading = true;
    error = '';
    
    try {
      const response = await post('/api/auth/login', 
        { username, password },
        { requireAuth: false }
      );

      if (!response.ok) {
        const data = await response.json();
        throw new Error(data.message || 'Login failed');
      }

      window.location.href = '/mailboxes';
    } catch (e: any) {
      error = e.message;
    } finally {
      loading = false;
    }
  }
</script>

<div class="max-w-md mx-auto">
  <h1 class="text-3xl font-bold text-center mb-8">Sign In</h1>

  <form on:submit|preventDefault={handleSubmit} class="space-y-4">
    {#if error}
      <div class="alert alert-error">
        <span>{error}</span>
      </div>
    {/if}

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
    <a href="/api/auth/telegram" class="btn w-full">
      <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 496 512">
        <path fill="currentColor" d="M248 8C111 8 0 119 0 256s111 248 248 248 248-111 248-248S385 8 248 8zm121.8 169.9l-40.7 191.8c-3 13.6-11.1 16.9-22.4 10.5l-62-45.7-29.9 28.8c-3.3 3.3-6.1 6.1-12.5 6.1l4.4-63.1 114.9-103.8c5-4.4-1.1-6.9-7.7-2.5l-142 89.4-61.2-19.1c-13.3-4.2-13.6-13.3 2.8-19.7l239.1-92.2c11.1-4 20.8 2.7 17.2 19.5z"/>
      </svg>
      Continue with Telegram
    </a>

    <a href="/api/auth/oauth/google" class="btn w-full">
      <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 488 512">
        <path fill="currentColor" d="M488 261.8C488 403.3 391.1 504 248 504 110.8 504 0 393.2 0 256S110.8 8 248 8c66.8 0 123 24.5 166.3 64.9l-67.5 64.9C258.5 52.6 94.3 116.6 94.3 256c0 86.5 69.1 156.6 153.7 156.6 98.2 0 135-70.4 140.8-106.9H248v-85.3h236.1c2.3 12.7 3.9 24.9 3.9 41.4z"/>
      </svg>
      Continue with Google
    </a>
  </div>

  <p class="text-center mt-4">
    Don't have an account?
    <a href="/auth/register" class="link link-primary">Sign up</a>
  </p>
</div> 