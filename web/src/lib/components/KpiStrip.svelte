<script lang="ts">
  import { formatNumber, formatTokens, formatUsd } from '$lib/format';
  import { t } from '$lib/i18n';
  import KpiFigure from './ui/KpiFigure.svelte';

  interface Props {
    working: number;
    waiting: number;
    tokens: number;
    cash: number;
    shadow: number;
    loading?: boolean;
  }

  let { working, waiting, tokens, cash, shadow, loading = false }: Props = $props();

  const tokenLabel = $derived(formatTokens(tokens).label);
  const tokenTitle = $derived(formatTokens(tokens).title);
</script>

<section class="kpis" aria-label={t('today.title')} aria-busy={loading || undefined}>
  <div data-testid="kpi-working">
    <KpiFigure label={t('today.kpi.working')} value={loading ? '—' : formatNumber(working)} />
  </div>
  <div data-testid="kpi-waiting">
    <KpiFigure label={t('today.kpi.waiting')} value={loading ? '—' : formatNumber(waiting)} />
  </div>
  <div data-testid="kpi-tokens" title={loading ? undefined : tokenTitle}>
    <KpiFigure label={t('today.kpi.tokens')} value={loading ? '—' : tokenLabel} />
  </div>
  <div data-testid="kpi-cash">
    <KpiFigure label={t('today.kpi.cash')} value={loading ? '—' : formatUsd(cash)} />
  </div>
  <div data-testid="kpi-shadow">
    <KpiFigure label={t('today.kpi.shadow')} value={loading ? '—' : formatUsd(shadow)} />
  </div>
</section>

<style>
  .kpis {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: 16px;
  }

  .kpis > div {
    min-width: 0;
  }

  .kpis :global(.kpi) {
    height: 100%;
  }

  .kpis :global(.value) {
    font-variant-numeric: tabular-nums;
  }

  @media (max-width: 1023px) {
    .kpis {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }
  }

  @media (max-width: 767px) {
    .kpis {
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 8px;
    }

    .kpis > div:last-child {
      grid-column: span 2;
    }
  }
</style>
