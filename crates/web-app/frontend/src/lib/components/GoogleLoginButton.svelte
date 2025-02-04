<script lang="ts">
  import { auth } from '$lib/stores/auth';
  import { post } from '$lib/api';
  import { page } from '$app/stores';

  export let action: 'login' | 'register' | 'connect' = 'login';
  export let onSuccess: () => void = () => {};
  export let onError: (error: string) => void = () => {};

  let error: string | null = null;
  let errorElement: HTMLElement;

  function scrollIntoView(node: HTMLElement) {
    errorElement = node;
    setTimeout(() => {
      node.scrollIntoView({ behavior: 'smooth', block: 'center' });
    }, 100); // Small delay to ensure DOM is updated
    return {
      destroy() {}
    };
  }

  const handleGoogleLogin = async () => {
    try {
      // Add redirect_to parameter when connecting from settings
      const redirectParam = action === 'connect' ? `&redirect_to=${encodeURIComponent($page.url.pathname)}` : '';
      
      // Add user ID to state when connecting
      const stateParam = action === 'connect' && $auth?.id ? `&state=${$auth.id}` : '';
      
      // Redirect to Google OAuth login endpoint with action parameter
      window.location.href = `/api/auth/google/login?action=${action}${redirectParam}${stateParam}`;
    } catch (err) {
      error = err instanceof Error ? err.message : 'Authentication failed';
      onError(error);
      console.error('Google auth error:', err);
    }
  };
</script>

<style>
  @keyframes shake {
    0%, 100% { transform: translateX(0); }
    10%, 30%, 50%, 70%, 90% { transform: translateX(-2px); }
    20%, 40%, 60%, 80% { transform: translateX(2px); }
  }

  .error-shake {
    animation: shake 0.5s cubic-bezier(.36,.07,.19,.97) both;
    background-color: rgba(255, 0, 0, 0.1);
    padding: 0.5rem;
    border-radius: 0.25rem;
    border: 1px solid rgba(255, 0, 0, 0.2);
  }
</style>

<div class="flex flex-col items-center gap-2 w-full flex-center">
  <button
    class="flex items-center justify-center gap-2 px-4 py-2 bg-white text-gray-700 border border-gray-300 rounded-md hover:bg-gray-50 transition-colors"
    on:click={handleGoogleLogin}
  >
    <!-- Google Logo SVG -->
    <svg class="w-5 h-5" viewBox="0 0 24 24">
      <path
        d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"
        fill="#4285F4"
      />
      <path
        d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"
        fill="#34A853"
      />
      <path
        d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"
        fill="#FBBC05"
      />
      <path
        d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"
        fill="#EA4335"
      />
    </svg>
    Continue with Google
  </button>
  {#if error}
    <div class="error-shake" use:scrollIntoView>
      <p class="text-error text-sm">{error}</p>
    </div>
  {/if}
</div> 