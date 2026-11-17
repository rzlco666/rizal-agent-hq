<script lang="ts">
  import { t } from '$lib/i18n';
  import { ui } from '$lib/stores/ui.svelte';
</script>

{#if ui.connection === 'offline'}
  <div class="offline-banner" role="status" data-testid="offline-banner">
    <strong>{t('offline.title')}</strong>
    <span>{t('offline.body')}</span>
  </div>
{/if}

<style>
  .offline-banner {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 12px;
    min-height: 32px;
    padding: 4px 16px;
    background: var(--attention-tint);
    color: var(--ink-12);
    font-size: 12px;
    line-height: 16px;
  }

  strong {
    font-weight: 500;
  }
</style>
