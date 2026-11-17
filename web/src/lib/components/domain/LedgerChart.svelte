<script lang="ts">
  import { t } from '$lib/i18n';
  import { formatTime } from '$lib/format';
  import Button from '../ui/Button.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import Skeleton from '../ui/Skeleton.svelte';

  export interface ChartPoint {
    ts_ms: number;
    cash_usd: number;
    shadow_usd: number;
    requests?: number;
  }

  interface Props {
    points: ChartPoint[];
    series?: ('cash' | 'shadow')[];
    height?: 160 | 240 | 320;
    unit?: 'usd' | 'requests' | 'tokens';
    window?: '24h' | '7d' | '30d';
    state?: 'ready' | 'loading' | 'empty' | 'error';
    summary: string;
    onretry?: () => void;
  }

  let {
    points,
    series = ['cash', 'shadow'],
    height = 240,
    unit = 'usd',
    state: chartState = 'ready',
    summary,
    onretry
  }: Props = $props();

  let activeIndex = $state(0);
  let showTable = $state(false);

  const maxValue = $derived(
    Math.max(1, ...points.flatMap((point) => [point.cash_usd, point.shadow_usd]))
  );

  function polyline(key: 'cash_usd' | 'shadow_usd') {
    if (points.length < 2) return '';
    return points
      .map((point, index) => {
        const x = (index / (points.length - 1)) * 100;
        const y = 100 - (point[key] / maxValue) * 100;
        return `${x.toFixed(2)},${y.toFixed(2)}`;
      })
      .join(' ');
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'ArrowRight' || event.key === 'ArrowLeft') {
      event.preventDefault();
      const delta = event.key === 'ArrowRight' ? 1 : -1;
      activeIndex = (activeIndex + delta + points.length) % Math.max(1, points.length);
    }
  }
</script>

<figure class="chart" aria-label={summary} style:height={`${height}px`}>
  <figcaption class="sr-only">{summary}</figcaption>

  {#if chartState === 'loading'}
    <div class="placeholder" aria-busy="true"><Skeleton shape="block" height="100%" /></div>
  {:else if chartState === 'empty'}
    <EmptyState title={t('chart.empty.title')} body={t('chart.empty.body')} />
  {:else if chartState === 'error'}
    <EmptyState
      title={t('chart.error.title')}
      body={t('chart.error.body')}
      action={onretry ? { label: t('chart.retry'), onclick: onretry } : undefined}
    />
  {:else}
    <div class="toolbar">
      <span class="legend">
        {#if series.includes('cash')}<span class="cash">{t('nav.costs')}</span>{/if}
        {#if series.includes('shadow')}<span class="shadow">{t('chart.summary')}</span>{/if}
      </span>
      <Button variant="ghost" size="sm" onclick={() => (showTable = !showTable)}>
        {t('chart.table')}
      </Button>
    </div>
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div
      class="plot"
      role="img"
      tabindex="0"
      aria-label={`${t('chart.summary')}: ${summary}`}
      data-active={activeIndex}
      onkeydown={handleKeydown}
    >
      <svg viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true">
        {#if series.includes('shadow')}
          <polyline points={polyline('shadow_usd')} fill="none" stroke="var(--info)" stroke-width="1" />
        {/if}
        {#if series.includes('cash')}
          <polyline points={polyline('cash_usd')} fill="none" stroke="var(--ink-12)" stroke-width="1" />
        {/if}
      </svg>
      <span class="sr-only">{summary}</span>
    </div>
    {#if showTable}
      <table aria-label={summary}>
        <caption class="sr-only">{summary}</caption>
        <thead><tr><th scope="col">{t('nav.timeline')}</th><th scope="col">{t('nav.costs')}</th></tr></thead>
        <tbody>
          {#each points as point, index (point.ts_ms)}
            <tr data-active={index === activeIndex}>
              <td>{formatTime(new Date(point.ts_ms), 'id')}</td>
              <td class="num">{unit === 'usd' ? point.cash_usd.toFixed(2) : point.cash_usd}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  {/if}
</figure>

<style>
  .chart {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin: 0;
    padding: 16px;
    border-radius: var(--radius-card);
    background: var(--surface);
    box-shadow: var(--shadow-card);
  }

  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }

  .legend {
    display: flex;
    gap: 16px;
    font-size: 12px;
    line-height: 16px;
    color: var(--ink-10);
  }

  .placeholder {
    flex: 1;
  }

  .plot {
    flex: 1;
    min-height: 0;
    outline-offset: 2px;
  }

  svg {
    width: 100%;
    height: 100%;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 12px;
  }

  th,
  td {
    padding: 4px 8px;
    border-bottom: 1px solid var(--hairline);
    text-align: left;
  }

  tr[data-active='true'] td {
    background: var(--attention-tint);
  }
</style>
