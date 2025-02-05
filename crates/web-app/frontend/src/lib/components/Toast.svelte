{#if visible}
  <div
    class="toast toast-top toast-end"
    transition:fly={{ y: -20, duration: 200 }}
  >
    <div class="alert alert-success">
      <span>{message}</span>
    </div>
  </div>
{/if}

<script lang="ts">
  import { fly } from 'svelte/transition';
  export let message: string;
  export let visible = false;
</script> 