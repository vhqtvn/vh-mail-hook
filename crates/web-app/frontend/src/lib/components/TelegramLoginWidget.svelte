<script lang="ts">
  import { onMount } from 'svelte';
  import { auth } from '$lib/stores/auth';
  import { post } from '$lib/api';
  import type { TelegramUser } from '$lib/types/telegram';

  export let size: 'large' | 'medium' | 'small' = 'large';
  export let botName: string;
  export let showUserPic = true;
  export let cornerRadius = 20;
  export let requestAccess = 'write';

  let widgetContainer: HTMLDivElement;
  let error: string | null = null;

  onMount(() => {
    console.log('TelegramLoginWidget mounted, botName:', botName);

    // Create the global callback
    (window as any).onTelegramAuth = async (user: TelegramUser) => {
      console.log('Telegram callback triggered with user:', user);
      try {
        console.log('Sending request to server:', {
          url: '/api/auth/telegram/verify',
          data: user
        });

        const response = await post('/api/auth/telegram/verify', user, { requireAuth: false });
        console.log('Server response:', response);
        
        if (response.success && response.data) {
          console.log('Login successful, redirecting...');
          await auth.login(response.data.token, response.data.user);
        } else {
          error = response.error || 'Authentication failed';
          console.error('Auth failed:', response.error);
        }
      } catch (err) {
        error = err instanceof Error ? err.message : 'Authentication failed';
        console.error('Telegram auth error:', err);
      }
    };

    // Create the widget using direct HTML
    const script = document.createElement('script');
    script.async = true;
    script.src = 'https://telegram.org/js/telegram-widget.js?22';
    script.dataset.telegramLogin = botName;
    script.dataset.size = size;
    script.dataset.userpic = showUserPic.toString();
    script.dataset.radius = cornerRadius.toString();
    script.dataset.requestAccess = requestAccess;
    script.dataset.onauth = 'onTelegramAuth(user)';

    // Clear any existing content
    widgetContainer.innerHTML = '';
    widgetContainer.appendChild(script);

    return () => {
      // Cleanup
      if (widgetContainer) {
        widgetContainer.innerHTML = '';
      }
      delete (window as any).onTelegramAuth;
    };
  });
</script>

<div class="flex flex-col items-center gap-2">
  <div bind:this={widgetContainer}></div>
  {#if error}
    <p class="text-error text-sm">{error}</p>
  {/if}
</div>

<style>
  div {
    display: inline-block;
  }
</style> 