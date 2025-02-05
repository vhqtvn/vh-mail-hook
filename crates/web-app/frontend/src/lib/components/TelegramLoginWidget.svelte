<script lang="ts">
  import { onMount } from 'svelte';
  import { auth } from '$lib/stores/auth';
  import { post } from '$lib/api';
  import type { TelegramUser } from '$lib/types/telegram';
  import { getTelegramBotName } from '$lib/config';

  export let size: 'large' | 'medium' | 'small' = 'large';
  export let botName: string | undefined = undefined;
  export let showUserPic = true;
  export let cornerRadius = 20;
  export let requestAccess = 'write';
  export let action: 'login' | 'register' | 'connect' = 'login';
  export let onSuccess: () => void = () => {};
  export let onError: (error: string) => void = () => {};

  let widgetContainer: HTMLDivElement;
  let error: string | null = null;
  let actualBotName = botName || getTelegramBotName();

  onMount(() => {
    if (!actualBotName) {
      console.error('TelegramLoginWidget: No bot name provided and TELEGRAM_BOT_NAME not set in runtime config');
      onError('Telegram login is not configured (TELEGRAM_BOT_NAME not set)');
      return;
    }
    console.log('TelegramLoginWidget mounted, botName:', actualBotName);

    // Create the global callback
    (window as any).onTelegramAuth = async (user: TelegramUser) => {
      console.log('Telegram callback triggered with user:', user);
      try {
        console.log('Sending request to server:', {
          url: '/api/auth/telegram/verify',
          data: { ...user, action }
        });

        const response = await post('/api/auth/telegram/verify', 
          { ...user, action }, 
          { requireAuth: action === 'connect' }
        );
        console.log('Server response:', response);
        
        if (response.success && response.data) {
          console.log('Authentication successful, redirecting...');
          if (action === 'register') {
            await auth.register(response.data.token, response.data.user);
          } else if (action === 'login') {
            await auth.login(response.data.token, response.data.user);
          }
          // For connect action, call success callback
          if (action === 'connect') {
            onSuccess();
          }
        } else {
          error = response.error || 'Authentication failed';
          onError(error);
          console.error('Auth failed:', response.error);
        }
      } catch (err) {
        error = err instanceof Error ? err.message : 'Authentication failed';
        onError(error);
        console.error('Telegram auth error:', err);
      }
    };

    // Create the widget using direct HTML
    const script = document.createElement('script');
    script.async = true;
    script.src = 'https://telegram.org/js/telegram-widget.js?22';
    script.dataset.telegramLogin = actualBotName;
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

<div class="flex flex-col items-center gap-2 w-full flex-center">
  <div class="telegram-login-widget" bind:this={widgetContainer}></div>
  {#if error}
    <p class="text-error text-sm">{error}</p>
  {/if}
</div>

<style>
  .telegram-login-widget :global(iframe) {
    opacity: 1;
  }
</style> 