<script lang="ts">
  import { formatTime } from '$lib/format';
  import { t } from '$lib/i18n';
  import 'uplot/dist/uPlot.min.css';
  import EmptyState from './ui/EmptyState.svelte';
  import PanelError from './PanelError.svelte';
  import Skeleton from './ui/Skeleton.svelte';
  import type { LedgerPoint } from './today';
  import type uPlot from 'uplot';
  interface Props {
    points: LedgerPoint[];
    range?: '24h' | '7d';
    height?: 160 | 240;
    status?: 'ready' | 'loading' | 'error';
    summary: string;
    onrange?: (range: '24h' | '7d') => void;
    onretry?: () => void;
  }

  let {
    points,
    range = '24h',
    height = 240,
    status = 'ready',
    summary,
    onrange,
    onretry
  }: Props = $props();

  type Plot = { setData: (data: number[][]) => void; destroy: () => void };
  let host = $state<HTMLDivElement>();
  let plot: Plot | undefined;
  let plotReady = $state(false);
  let activeIndex = $state(0);

  const series = $derived([
    points.map((point) => point.ts_ms / 1000),
    points.map((point) => point.cash_usd),
    points.map((point) => point.shadow_usd)
  ]);
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
    if (event.key !== 'ArrowRight' && event.key !== 'ArrowLeft') return;
    event.preventDefault();
    const delta = event.key === 'ArrowRight' ? 1 : -1;
    activeIndex = (activeIndex + delta + points.length) % Math.max(1, points.length);
  }

  $effect(() => {
    const element = host;
    const data = series;
    let cancelled = false;
    let created: Plot | undefined;

    if (!element || status !== 'ready' || data[0].length === 0) {
      plotReady = false;
      return;
    }

    const plotSeries: uPlot.AlignedData = [data[0], data[1], data[2]];
    void import('uplot')
      .then(({ default: UPlot }) => {
        if (cancelled || !element.isConnected) return;
        created = new UPlot(
          {
            width: element.clientWidth || 640,
            height: height - 72,
            scales: { x: { time: true } },
            axes: [
              {
                stroke: 'var(--ink-10)',
                grid: { show: false },
                font: '11px "IBM Plex Mono", monospace',
                values: (_: unknown, ticks: number[]) =>
                  ticks.map((tick) => formatTime(new Date(tick * 1000)))
              },
              {
                stroke: 'var(--ink-10)',
                grid: { stroke: 'var(--hairline)', width: 1 },
                font: '11px "IBM Plex Mono", monospace',
                values: (_: unknown, ticks: number[]) => ticks.map((tick) => `$${tick.toFixed(2)}`)
              }
            ],
            series: [
              { label: t('nav.timeline') },
              { label: t('today.kpi.cash'), stroke: 'var(--attention)', width: 2, points: { show: false } },
              { label: t('today.kpi.shadow'), stroke: 'var(--ink-10)', width: 1, points: { show: false } }
            ],
            legend: { show: false }
          },
          plotSeries,
          element
        ) as unknown as Plot;
        plot = created;
        plotReady = true;
      })
      .catch(() => {
        if (!cancelled) plotReady = false;
      });

    return () => {
      cancelled = true;
      created?.destroy();
      if (plot === created) plot = undefined;
      plotReady = false;
    };
  });
</script>

<section class="ledger" data-testid="ledger-chart" aria-label={summary}>
  <div class="header">
    <div class="legend" aria-hidden="true">
      <span class="cash">{t('today.kpi.cash')}</span>
      <span class="shadow">{t('today.kpi.shadow')}</span>
    </div>
    <div class="ranges" role="tablist" aria-label={t('chart.summary')}>
      <button
        type="button"
        role="tab"
        aria-selected={range === '24h'}
        onclick={() => onrange?.('24h')}
      >
        {t('today.chart.24h')}
      </button>
      <button
        type="button"
        role="tab"
        aria-selected={range === '7d'}
        onclick={() => onrange?.('7d')}
      >
        {t('today.chart.7d')}
      </button>
    </div>
  </div>

  {#if status === 'loading'}
    <div class="loading" aria-busy="true"><Skeleton shape="block" height="100%" /></div>
  {:else if status === 'error'}
    <PanelError onretry={() => onretry?.()} />
  {:else if points.length === 0}
    <EmptyState title={t('chart.empty.title')} body={t('chart.empty.body')} />
  {:else}
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div
      class="plot"
      role="img"
      tabindex="0"
      aria-label={summary}
      data-active={activeIndex}
      onkeydown={handleKeydown}
    >
      <div class="uplot-host" bind:this={host} class:hidden={plotReady}></div>
      {#if !plotReady}
        <svg viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true">
          <polyline points={polyline('shadow_usd')} fill="none" stroke="var(--ink-10)" stroke-width="1" />
          <polyline points={polyline('cash_usd')} fill="none" stroke="var(--attention)" stroke-width="1" />
        </svg>
      {/if}
    </div>
    <table class="sr-only" aria-label={summary}>
      <caption class="sr-only">{summary}</caption>
      <thead><tr><th scope="col">{t('nav.timeline')}</th><th scope="col">{t('today.kpi.cash')}</th></tr></thead>
      <tbody>
        {#each points as point, index (point.ts_ms)}
          <tr><td>{formatTime(new Date(point.ts_ms))}</td><td>${point.cash_usd.toFixed(2)}</td></tr>
        {/each}
      </tbody>
    </table>
  {/if}
</section>

<style>
  .ledger {
    min-width: 0;
    padding: 16px;
    border-radius: var(--radius-card);
    background: var(--surface);
    box-shadow: var(--shadow-card);
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    min-height: 32px;
  }

  .legend {
    display: flex;
    gap: 16px;
    font-size: 12px;
    line-height: 16px;
    color: var(--ink-11);
  }

  .cash::before,
  .shadow::before {
    display: inline-block;
    width: 12px;
    height: 2px;
    margin-right: 6px;
    vertical-align: middle;
    content: '';
  }

  .cash::before {
    background: var(--attention);
  }

  .shadow::before {
    background: var(--ink-10);
  }

  .ranges {
    display: flex;
    gap: 16px;
  }

  .ranges button {
    height: 32px;
    padding: 0;
    border: 0;
    border-bottom: 2px solid transparent;
    background: transparent;
    color: var(--ink-10);
    font-size: 12px;
    cursor: pointer;
  }

  .ranges button[aria-selected='true'] {
    border-bottom-color: var(--ink-12);
    color: var(--ink-12);
  }
  .ranges button:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }


  .plot {
    position: relative;
    min-height: 160px;
    margin-top: 8px;
  }

  .uplot-host :global(.uplot) {
    font-family: var(--font-mono);
  }

  .hidden {
    display: none;
  }

  .loading {
    min-height: 160px;
  }

  svg {
    width: 100%;
    height: 160px;
  }

  @media (min-width: 768px) {
    .plot,
    .loading {
      min-height: 240px;
    }

    svg {
      height: 240px;
    }
  }
</style>
