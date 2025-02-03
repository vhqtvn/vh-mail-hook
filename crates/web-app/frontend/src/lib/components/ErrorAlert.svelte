<script lang="ts">
  import { formatErrorMessage } from '$lib/error';

  export let error: unknown | null = null;
  export let className: string = '';

  $: ({ message, debug } = error ? formatErrorMessage(error) : { message: '', debug: undefined });
</script>

{#if error}
  <div class="alert alert-error shadow-lg {className}">
    <div class="flex-1">
      <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6 flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z" />
      </svg>
      <div>
        <h3 class="font-bold">Error</h3>
        <div class="text-sm">{message}</div>
        {#if debug}
          <details class="mt-2">
            <summary class="text-sm cursor-pointer hover:text-error-content/80">Show technical details</summary>
            <pre class="mt-2 text-xs whitespace-pre-wrap font-mono bg-base-300 p-2 rounded">{debug}</pre>
          </details>
        {/if}
      </div>
    </div>
  </div>
{/if} 