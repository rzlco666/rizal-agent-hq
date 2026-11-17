<script lang="ts">
  import { toasts, dismissToast } from '$lib/stores/toast.svelte';
  import Toast from './Toast.svelte';

  const current = $derived(toasts.items[0]);

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && current) {
      event.preventDefault();
      dismissToast(current.id);
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="toaster" aria-live="polite">
  {#if current}
    <Toast item={current} />
  {/if}
</div>

<style>
  .toaster {
    position: fixed;
    left: 50%;
    bottom: 24px;
    z-index: 90;
    transform: translateX(-50%);
    display: flex;
    justify-content: center;
    pointer-events: none;
  }

  .toaster :global(.toast) {
    pointer-events: auto;
  }
</style>
