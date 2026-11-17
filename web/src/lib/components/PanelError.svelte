<script lang="ts">
  import { t } from '$lib/i18n';
  import Button from './ui/Button.svelte';

  interface Props {
    onretry: () => void;
  }

  let { onretry }: Props = $props();
</script>

<div class="panel-error" role="alert">
  <p class="title">{t('today.error.title')}</p>
  <p class="body">{t('today.error.body')}</p>
  <Button variant="secondary" size="sm" onclick={onretry}>{t('today.connection.retry')}</Button>
</div>

<style>
  .panel-error {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    padding: 24px 0;
  }

  p {
    margin: 0;
  }

  .title {
    color: var(--ink-11);
    font-size: 13px;
    line-height: 20px;
  }

  .body {
    color: var(--ink-10);
    font-size: 12px;
    line-height: 16px;
  }
</style>
