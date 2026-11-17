<script lang="ts">
  import { t } from '$lib/i18n';
  import type { LiveState } from '$lib/stores/live.svelte';
  import Button from './ui/Button.svelte';

  interface Props {
    state: LiveState;
    onretry?: () => void;
  }

  let { state, onretry }: Props = $props();
</script>

{#if state === 'offline'}
  <div class="banner" role="status" data-testid="connection-banner">
    <span aria-live="polite">{t('today.connection.reconnecting')}</span>
    {#if onretry}
      <Button variant="secondary" size="sm" onclick={onretry}>{t('today.connection.retry')}</Button>
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
