<script lang="ts">
  import { onMount } from 'svelte';
  import { auth } from '$lib/stores/auth';
  import { goto } from '$app/navigation';
  import { get, post } from '$lib/api';
  import ErrorAlert from '$lib/components/ErrorAlert.svelte';
  import { page } from '$app/stores';

  let error: string | null = null;

  onMount(async () => {
    const params = new URLSearchParams(window.location.search);
    const provider = $page.params.provider;

    console.log('Callback params:', params.toString(), 'provider:', provider);

    // provider must be one of github, google
    if (provider !== 'github' && provider !== 'google') {
      error = 'Invalid provider';
      return;
    }

    try {
      const response = await get(`/api/auth/${provider}/callback?${params.toString()}`);
      
      console.log('Auth response:', response);

      if (!response.success || !response.data) {
        error = response.error || 'Authentication failed';
        return;
      }

      const { token, user, redirect_to } = response.data;

      // Set auth token and user if provided
      await auth.login(token, user);

      // Follow the backend's redirection
      await goto(redirect_to);
    } catch (e) {
      console.error('Auth error:', e);
      error = e instanceof Error ? e.message : 'Authentication failed';
    }
  });
</script>

<div class="max-w-md mx-auto mt-8">
  <h1 class="text-2xl font-bold text-center mb-4">Authenticating...</h1>
  
  {#if error}
    <ErrorAlert error={error} />
  {:else}
    <div class="flex justify-center">
      <span class="loading loading-spinner loading-lg"></span>
    </div>
  {/if}
</div> 