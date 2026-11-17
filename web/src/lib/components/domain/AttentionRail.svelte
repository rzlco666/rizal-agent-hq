<script lang="ts">
  import { Bell, Check, Clock } from 'lucide-svelte';
  import { t } from '$lib/i18n';
  import EmptyState from '../ui/EmptyState.svelte';
  import IconButton from '../ui/IconButton.svelte';
  import Skeleton from '../ui/Skeleton.svelte';

  export interface AttentionItem {
    id: string;
    kind: 'approval' | 'alert';
    level?: 'warning' | 'critical';
    title: string;
    meta: string;
    href?: string;
    minutesLeft?: number;
  }

  interface Props {
    items?: AttentionItem[];
    alerts?: AttentionItem[];
    collapsed?: boolean;
    loading?: boolean;
    limit?: number;
    onopen: (item: AttentionItem) => void;
    onack?: (item: AttentionItem) => void;
    onsnooze?: (item: AttentionItem) => void;
  }

  let {
    items = [],
    alerts = [],
    collapsed = false,
    loading = false,
    limit = 3,
    onopen,
    onack,
    onsnooze
  }: Props = $props();

  function rank(item: AttentionItem) {
    if (item.level === 'critical') return 3;
    if (item.kind === 'approval') return 2;
    if (item.level === 'warning') return 1;
    return 0;
  }

  const entries = $derived(
    [...alerts, ...items]
      .filter((item) => item.kind === 'approval' || item.level)
      .sort((a, b) => rank(b) - rank(a))
  );
  const shown = $derived(entries.slice(0, limit));
  const remainder = $derived(Math.max(0, entries.length - limit));

  function groupLabel(item: AttentionItem) {
    return item.kind === 'approval' ? t('attention.decision') : t('attention.alerts');
  }
</script>

<aside
  class="rail"
  class:collapsed
  data-testid="attention-rail"
  aria-label={t('attention.region')}
>
  {#if collapsed}
    <div class="collapsed-strip">
      <Bell size={16} strokeWidth={1.5} aria-hidden="true" />
      <span class="mono" data-testid="attention-count" aria-live="polite">{entries.length}</span>
    </div>
  {:else}
    <header>
      <h2>{t('attention.title')}</h2>
      <span class="count mono" data-testid="attention-count" aria-live="polite">
        <span data-testid="alert-count">{entries.length}</span>
      </span>
    </header>

    <div class="rows">
      {#if loading}
        <Skeleton shape="row" lines={3} />
      {:else if entries.length === 0}
        <EmptyState title={t('attention.empty.title')} body={t('attention.empty.body')} />
      {:else}
        {#each shown as item, index (item.id)}
          {#if index === 0 || groupLabel(item) !== groupLabel(shown[index - 1])}
            <p class="group-label">{groupLabel(item)}</p>
          {/if}
          <div class="row" data-testid="attention-row" data-level={item.level ?? item.kind}>
            <span
              class="dot"
              aria-hidden="true"
              data-level={item.level ?? 'warning'}
            ></span>
            <button type="button" class="row-main" onclick={() => onopen(item)}>
              <span class="title">{item.title}</span>
              <span class="meta">{item.meta}</span>
            </button>
            <span class="actions">
              {#if onack}
                <IconButton icon={Check} label={t('attention.ackShort')} size="sm" onclick={() => onack(item)} />
              {/if}
              {#if onsnooze}
                <IconButton icon={Clock} label={t('attention.snooze')} size="sm" onclick={() => onsnooze(item)} />
              {/if}
            </span>
          </div>
        {/each}
        {#if remainder > 0}
          <button type="button" class="more mono" onclick={() => onopen(entries[limit])}>
            {t('attention.more', { count: remainder })}
          </button>
        {/if}
      {/if}
    </div>
  {/if}
</aside>

<style>
  .rail {
    display: flex;
    flex-direction: column;
    width: 320px;
    flex: none;
    background: var(--ground);
    border-left: 1px solid var(--hairline);
  }

  .rail.collapsed {
    width: 44px;
  }

  .collapsed-strip {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 12px 0;
    color: var(--ink-11);
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 40px;
    flex: none;
    padding: 0 16px;
    border-bottom: 1px solid var(--hairline);
  }

  h2 {
    margin: 0;
    font-size: 13px;
    line-height: 20px;
    font-weight: 500;
    color: var(--ink-12);
  }

  .count {
    color: var(--ink-10);
  }

  .rows {
    display: flex;
    flex-direction: column;
    padding: 8px;
    gap: 4px;
  }

  .group-label {
    margin: 8px 8px 0;
    font-size: 12px;
    line-height: 16px;
    color: var(--ink-10);
  }

  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 72px;
    padding: 8px;
    border-radius: var(--radius-control);
  }

  .row:hover {
    background: var(--hover);
  }

  .dot {
    width: 8px;
    height: 8px;
    flex: none;
    border-radius: var(--radius-dot);
    background: var(--attention);
  }

  .dot[data-level='critical'] {
    background: var(--critical);
  }

  .row-main {
    display: flex;
    flex: 1;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    min-width: 0;
    padding: 0;
    border: 0;
    background: transparent;
    text-align: left;
  }

  .title {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 13px;
    line-height: 20px;
    color: var(--ink-12);
  }

  .meta {
    font-size: 12px;
    line-height: 16px;
    color: var(--ink-10);
  }

  .actions {
    display: flex;
    flex: none;
  }

  .more {
    align-self: flex-start;
    margin: 4px 8px;
    padding: 4px 6px;
    border: 0;
    border-radius: var(--radius-control);
    background: transparent;
    color: var(--attention);
    font-size: 12px;
  }

  .more:hover {
    background: var(--hover);
  }
</style>
