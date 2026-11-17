<script lang="ts">
  interface Props {
    tone?: 'neutral' | 'attention' | 'critical' | 'success' | 'info';
    label: string;
    count?: number;
    dot?: boolean;
    unit?: string;
  }

  let { tone = 'neutral', label, count, dot = false, unit = '' }: Props = $props();
</script>

<span class="badge" data-tone={tone}>
  {#if dot}<span class="dot" aria-hidden="true"></span>{/if}
  <span>{label}</span>
  {#if count !== undefined}
    <span class="count mono">{count}<span class="sr-only"> {unit}</span></span>
  {/if}
</span>

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 20px;
    padding: 0 6px;
    border-radius: var(--radius-keycap);
    background: var(--well);
    color: var(--ink-11);
    font-size: 12px;
    line-height: 16px;
    font-weight: 500;
  }

  .badge[data-tone='attention'],
  .badge[data-tone='critical'],
  .badge[data-tone='success'],
  .badge[data-tone='info'] {
    color: var(--ink-12);
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: var(--radius-dot);
    background: var(--ink-10);
  }

  .badge[data-tone='attention'] .dot { background: var(--attention); }
  .badge[data-tone='critical'] .dot { background: var(--critical); }
  .badge[data-tone='success'] .dot { background: var(--success); }
  .badge[data-tone='info'] .dot { background: var(--info); }

  .count {
    font-size: 12px;
  }
</style>
