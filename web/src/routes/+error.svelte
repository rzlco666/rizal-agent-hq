<script lang="ts">
  import { page } from '$app/state';
  import { t } from '$lib/i18n';
  import Button from '$lib/components/ui/Button.svelte';

  const requestId = $derived(page.error?.requestId ?? null);
</script>

<svelte:head>
  <title>{t('error.title')} · {t('app.name')}</title>
</svelte:head>

<section class="error-page">
  <span class="status mono">{page.status}</span>
  <h1>{t('error.title')}</h1>
  <p>{t('error.body')}</p>
  {#if requestId}
    <p class="request-id">{t('error.request_id')}: <span class="mono">{requestId}</span></p>
  {/if}
  <div class="actions">
    <Button variant="primary" onclick={() => location.reload()}>{t('action.reload')}</Button>
    <Button variant="secondary" onclick={() => history.back()}>{t('action.back')}</Button>
  </div>
</section>

<style>
  .error-page {
    max-width: 480px;
    margin: 48px auto;
    padding: 32px;
    border-radius: var(--radius-card);
    background: var(--surface);
    box-shadow: var(--shadow-card);
  }

  .status {
    color: var(--ink-10);
    font-size: 12px;
  }

  h1 {
    margin: 8px 0;
    font-size: 20px;
    line-height: 28px;
    color: var(--ink-12);
  }

  p {
    margin: 0 0 12px;
    color: var(--ink-11);
  }

  .request-id {
    color: var(--ink-10);
  }

  .actions {
    display: flex;
    gap: 8px;
    margin-top: 20px;
  }
</style>
