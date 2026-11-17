<script lang="ts">
  import type { AgentPage, AlertView } from '$lib/api/client';
  import { t, type TranslationKey } from '$lib/i18n';
  import AgentMonogram from './domain/AgentMonogram.svelte';
  import StatusDot from './ui/StatusDot.svelte';
  import Skeleton from './ui/Skeleton.svelte';
  import { canonicalStatus, sortAgents } from './today';

  type Status = 'bekerja' | 'berpikir' | 'menunggu-rizal' | 'santai' | 'offline' | 'error';

  interface Props {
    agents: AgentPage['items'];
    alerts?: AlertView[];
    loading?: boolean;
  }

  let { agents, alerts = [], loading = false }: Props = $props();

  let expanded = $state(false);
  const rows = $derived(
    sortAgents(
      agents.map((agent) => ({
        ...agent,
        alerts: alerts.filter((alert) => alert.subject.includes(agent.id)).length
      }))
    )
  );
  const visible = $derived(expanded ? rows : rows.slice(0, 6));

  function asStatus(value: string): Status {
    const normalized = canonicalStatus(value);
    return (['bekerja', 'berpikir', 'menunggu-rizal', 'santai', 'offline', 'error'] as const).find(
      (status) => status === normalized
    ) ?? 'offline';
  }

  function activity(value: string): string {
    return t(`today.status.${asStatus(value)}` as TranslationKey);
  }
</script>

<section class="agent-table" aria-label={t('today.table.agent')} aria-busy={loading || undefined}>
  <table>
    <caption class="sr-only">{t('today.table.agent')}</caption>
    <thead>
      <tr>
        <th scope="col">{t('today.table.agent')}</th>
        <th scope="col">{t('today.table.status')}</th>
        <th scope="col">{t('today.table.now')}</th>
        <th scope="col">{t('today.table.model')}</th>
        <th scope="col" class="num">{t('today.table.today')}</th>
        <th scope="col" class="num">{t('today.table.alerts')}</th>
      </tr>
    </thead>
    <tbody>
      {#if loading && agents.length === 0}
        {#each Array(6) as _, index (index)}
          <tr class="skeleton-row"><td colspan="6"><Skeleton shape="row" lines={1} /></td></tr>
        {/each}
      {:else}
        {#each visible as agent (agent.id)}
          <tr>
            <td colspan="6">
              <a
                class="row-link"
                data-testid="agent-row"
                data-agent-id={agent.id}
                href={`/agents/${encodeURIComponent(agent.id)}`}
              >
                <span class="identity">
                  <AgentMonogram name={agent.name} status={asStatus(agent.status)} decorative />
                  <span class="names">
                    <span class="name">{agent.name}</span>
                    <span class="role">agent</span>
                  </span>
                </span>
                <span class="status-cell">
                  <span data-testid="status-dot"><StatusDot status={asStatus(agent.status)} /></span>
                  <span class="sr-only">{agent.status}</span>
                </span>
                <span data-testid="activity" class="activity" title={activity(agent.status)}>
                  {activity(agent.status)}
                </span>
                <span class="mono muted">—</span>
                <span class="mono num muted">—</span>
                <span class="mono num">{agent.alerts || '—'}</span>
              </a>
            </td>
          </tr>
        {/each}
      {/if}
    </tbody>
  </table>

  {#if rows.length > 6}
    <button type="button" class="expand" onclick={() => (expanded = !expanded)}>
      {expanded ? t('today.table.less') : t('today.table.more')}
    </button>
  {/if}
</section>

<style>
  .agent-table {
    min-width: 0;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    table-layout: fixed;
  }

  th {
    height: 32px;
    padding: 0 8px;
    border-bottom: 1px solid var(--hairline);
    color: var(--ink-10);
    font-size: 11px;
    font-weight: 500;
    text-align: left;
  }

  td {
    height: 44px;
    padding: 0 8px;
    border-bottom: 1px solid var(--hairline);
    font-size: 13px;
    line-height: 20px;
  }

  .num {
    text-align: right;
  }

  .row-link {
    display: grid;
    grid-template-columns: minmax(160px, 2fr) 90px minmax(180px, 3fr) 72px 72px 48px;
    align-items: center;
    gap: 8px;
    min-height: 44px;
    color: inherit;
    text-decoration: none;
  }

  .row-link:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }

  .expand:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }


  .row-link:hover .name {
    color: var(--ink-12);
    background: var(--hover);
  }

  .identity {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }

  .names {
    display: flex;
    min-width: 0;
    flex-direction: column;
  }

  .status-cell {
    display: flex;
    align-items: center;
    gap: 6px;
  }


  .name {
    overflow: hidden;
    font-weight: 500;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .role {
    color: var(--ink-10);
    font-size: 11px;
    line-height: 14px;
  }

  .activity {
    display: block;
    overflow: hidden;
    max-width: 64ch;
    color: var(--ink-11);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .muted {
    color: var(--ink-10);
  }

  .mono {
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
  }

  .expand {
    margin-top: 12px;
    padding: 4px 0;
    border: 0;
    background: transparent;
    color: var(--ink-11);
    font-size: 13px;
    text-decoration: underline;
    text-underline-offset: 3px;
  }

  .skeleton-row td {
    padding-block: 4px;
  }

  @media (max-width: 767px) {
    .identity .role,
    th:nth-child(4),
    th:nth-child(5),
    .row-link > span:nth-child(4),
    .row-link > span:nth-child(5) {
      display: none;
    }

    .row-link {
      grid-template-columns: minmax(0, 2fr) 72px minmax(0, 2fr) 32px;
    }

    td {
      height: 56px;
    }
  }
</style>
