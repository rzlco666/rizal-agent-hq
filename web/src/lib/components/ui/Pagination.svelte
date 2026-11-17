<script lang="ts">
  import { t } from '$lib/i18n';
  import Button from './Button.svelte';

  interface Props {
    page: number;
    pageSize: number;
    total: number;
    onchange: (page: number) => void;
  }

  let { page, pageSize, total, onchange }: Props = $props();
  const from = $derived(total === 0 ? 0 : (page - 1) * pageSize + 1);
  const to = $derived(Math.min(page * pageSize, total));
  const atStart = $derived(page <= 1);
  const atEnd = $derived(to >= total);
</script>

<nav class="pagination" aria-label={t('pagination.nav')}>
  <span class="range mono">
    {t('pagination.range', { from, to, total })}
  </span>
  <Button variant="secondary" size="sm" disabled={atStart} onclick={() => onchange(page - 1)}>
    {t('pagination.prev')}
  </Button>
  <Button variant="secondary" size="sm" disabled={atEnd} onclick={() => onchange(page + 1)}>
    {t('pagination.next')}
  </Button>
</nav>

<style>
  .pagination {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 0;
  }

  .range {
    font-size: 12px;
    line-height: 16px;
    color: var(--ink-10);
  }
</style>
