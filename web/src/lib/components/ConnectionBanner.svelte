<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from '$lib/i18n';
  import Button from './ui/Button.svelte';

  interface Props {
    onretry?: () => void;
  }

  let { onretry }: Props = $props();

  let reconnecting = $state(false);

  onMount(() => {
    const offline = () => (reconnecting = true);
    const online = () => (reconnecting = false);
    window.addEventListener('hq-offline', offline);
    window.addEventListener('hq-online', online);
    return () => {
      window.removeEventListener('hq-offline', offline);
      window.removeEventListener('hq-online', online);
    };
  });

  function retry() {
    reconnecting = false;
    onretry?.();
  }
</script>

{#if reconnecting}
  <div class="banner" role="status" data-testid="connection-banner">
    <span aria-live="polite">{t('today.connection.reconnecting')}</span>
    {#if onretry}
      <Button variant="secondary" size="sm" onclick={retry}>{t('today.connection.retry')}</Button>
    {/if}
  </div>
{/if}

<style>
  .banner {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 16px;
    min-height: 32px;
    padding: 4px 16px;
    background: var(--attention-tint);
    color: var(--ink-12);
    font-size: 12px;
    line-height: 16px;
  }
</style>
