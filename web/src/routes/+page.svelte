<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import {
    getAgents,
    getAlerts,
    getCosts,
    getOverview,
    getTimeline,
    getUsage,
    type AgentPage,
    type AlertView,
    type CostReport,
    type EventPage,
    type Overview,
    type UsageReport
  } from '$lib/api/client';
  import AgentTable from '$lib/components/AgentTable.svelte';
  import ConnectionBanner from '$lib/components/ConnectionBanner.svelte';
  import EmptyToday from '$lib/components/EmptyToday.svelte';
  import KpiStrip from '$lib/components/KpiStrip.svelte';
  import LedgerChart from '$lib/components/LedgerChart.svelte';
  import PanelError from '$lib/components/PanelError.svelte';
  import { buildLedgerPoints } from '$lib/components/today';
  import { formatDate } from '$lib/format';
  import { t } from '$lib/i18n';
  import { attention } from '$lib/stores/attention.svelte';
  import { createLiveStore, type LiveSocket } from '$lib/stores/live.svelte';

  type Panel = 'overview' | 'agents' | 'alerts' | 'costs' | 'usage' | 'timeline';

  const empty = $derived(page.url.searchParams.get('empty') === '1');

  let overview = $state<Overview | null>(null);
  let agents = $state<AgentPage['items']>([]);
  let alerts = $state<AlertView[]>([]);
  let costs = $state<CostReport | null>(null);
  let usage = $state<UsageReport | null>(null);
  let events = $state<EventPage['items']>([]);
  let failed = $state<Record<Panel, boolean>>({
    overview: false,
    agents: false,
    alerts: false,
    costs: false,
    usage: false,
    timeline: false
  });
  let loading = $state(true);
  let range = $state<'24h' | '7d'>('24h');

  const tokens = $derived(
    (usage?.total.input_tokens ?? 0) + (usage?.total.output_tokens ?? 0)
  );
  function ledgerInput(event: EventPage['items'][number]) {
    const payload = event.payload;
    if (!payload || typeof payload !== 'object') return null;
    return {
      ts_ms: event.event_ts_ms,
      cash_usd: 'cash_usd' in payload ? Number(payload.cash_usd) : 0,
      shadow_usd: 'shadow_usd' in payload ? Number(payload.shadow_usd) : 0
    };
  }

  const ledgerPoints = $derived(
    buildLedgerPoints(
      events
        .filter((event) => event.kind === 'usage_recorded')
        .map(ledgerInput)
        .filter((point): point is NonNullable<typeof point> => point !== null),
      range
    )
  );

  function browserSocket(url: string): LiveSocket {
    const socket = new WebSocket(new URL(url, window.location.href).href.replace(/^http/, 'ws'));
    return {
      get url() {
        return socket.url;
      },
      set url(_value: string) {},
      set onopen(handler: (() => void) | null) {
        socket.onopen = handler;
      },
      set onmessage(handler: ((event: { data: unknown }) => void) | null) {
        socket.onmessage = handler
          ? (event) => handler({ data: event.data })
          : null;
      },
      set onclose(handler: (() => void) | null) {
        socket.onclose = handler;
      },
      set onerror(handler: (() => void) | null) {
        socket.onerror = handler ? () => handler() : null;
      },
      close: () => socket.close()
    };
  }

  const live = createLiveStore(browserSocket, () => loadAll());

  async function loadAll() {
    loading = true;
    const [overviewResult, agentsResult, alertsResult, costsResult, usageResult, timelineResult] =
      await Promise.allSettled([
        getOverview(),
        getAgents(),
        getAlerts('open'),
        getCosts(),
        getUsage(undefined, { group_by: 'agent', window: range }),
        getTimeline(undefined, 500)
      ]);

    failed = {
      overview: overviewResult.status === 'rejected',
      agents: agentsResult.status === 'rejected',
      alerts: alertsResult.status === 'rejected',
      costs: costsResult.status === 'rejected',
      usage: usageResult.status === 'rejected',
      timeline: timelineResult.status === 'rejected'
    };

    if (overviewResult.status === 'fulfilled') {
      overview = overviewResult.value;
      live.hydrate({ seq: overview.seq, agents: overview.agents });
    }
    if (agentsResult.status === 'fulfilled') agents = agentsResult.value.items;
    if (alertsResult.status === 'fulfilled') alerts = alertsResult.value;
    if (costsResult.status === 'fulfilled') costs = costsResult.value;
    if (usageResult.status === 'fulfilled') usage = usageResult.value;
    if (timelineResult.status === 'fulfilled') events = timelineResult.value.items;
    attention.setAlerts(alerts, overview?.counts.alerts_open ?? alerts.length);
    loading = false;
  }

  function changeRange(next: '24h' | '7d') {
    range = next;
    void loadAll();
  }

  onMount(() => {
    if (empty) {
      loading = false;
      return;
    }
    void loadAll();
    void live.open();
  });
</script>

<svelte:head>
  <title>{t('today.title')} · {t('app.name')}</title>
</svelte:head>

<section class="page">
  <header class="page-head">
    <h1>{t('today.title')}</h1>
    <span class="meta mono">{formatDate(new Date())}</span>
  </header>

  <ConnectionBanner onretry={() => void live.open()} />

  {#if empty}
    <EmptyToday />
  {:else}
    {#if failed.overview}
      <PanelError onretry={() => void loadAll()} />
    {:else}
      <KpiStrip
        working={overview?.counts.working ?? 0}
        waiting={overview?.counts.waiting ?? 0}
        {tokens}
        cash={costs?.cash_usd ?? 0}
        shadow={costs?.shadow_usd ?? 0}
        {loading}
      />
    {/if}

    {#if failed.agents}
      <PanelError onretry={() => void loadAll()} />
    {:else}
      <AgentTable {agents} {alerts} {loading} />
    {/if}

    <div class="ledger-slot">
      {#if failed.usage || failed.timeline}
        <PanelError onretry={() => void loadAll()} />
      {:else}
        <LedgerChart
          points={ledgerPoints}
          {range}
          summary={t('chart.summary')}
          status={failed.costs || loading ? 'loading' : 'ready'}
          onrange={changeRange}
          onretry={() => void loadAll()}
        />
      {/if}
    </div>
  {/if}
</section>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 24px;
    max-width: 1408px;
    margin: 0 auto;
  }

  .page-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 16px;
  }

  h1 {
    margin: 0;
    font-size: 32px;
    line-height: 40px;
    font-weight: 400;
    letter-spacing: -0.02em;
    color: var(--ink-12);
  }

  .meta {
    color: var(--ink-10);
    font-size: 12px;
    line-height: 16px;
  }

  .ledger-slot {
    min-width: 0;
  }

  @media (max-width: 767px) {
    h1 {
      font-size: 24px;
      line-height: 32px;
    }

    .page {
      gap: 16px;
    }
  }
</style>
