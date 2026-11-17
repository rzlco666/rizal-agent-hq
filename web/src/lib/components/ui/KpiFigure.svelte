<script lang="ts">
  interface Props {
    label: string;
    value: string;
    unit?: string;
    delta?: string;
    compare?: string;
    href?: string;
  }

  let { label, value, unit, delta, compare, href }: Props = $props();
  const ariaLabel = $derived(
    [label, value, unit, delta, compare].filter(Boolean).join(' ')
  );
</script>

{#if href}
  <a class="kpi" {href} aria-label={ariaLabel}>
    <span class="label">{label}</span>
    <span class="figure"><span class="value mono">{value}</span>{#if unit}<span class="unit">{unit}</span>{/if}</span>
    {#if delta}<span class="delta">{delta}{compare ? ` ${compare}` : ''}</span>{/if}
  </a>
{:else}
  <div class="kpi" role="group" aria-label={ariaLabel}>
    <span class="label">{label}</span>
    <span class="figure"><span class="value mono">{value}</span>{#if unit}<span class="unit">{unit}</span>{/if}</span>
    {#if delta}<span class="delta">{delta}{compare ? ` ${compare}` : ''}</span>{/if}
  </div>
{/if}

<style>
  .kpi {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-height: 88px;
    padding: 12px;
    border-radius: var(--radius-card);
    background: var(--surface);
    box-shadow: var(--shadow-card);
    color: inherit;
    text-decoration: none;
  }

  .label {
    font-size: 12px;
    line-height: 16px;
    color: var(--ink-10);
  }

  .figure {
    display: flex;
    align-items: baseline;
    gap: 6px;
  }

  .value {
    font-size: 24px;
    line-height: 32px;
    font-weight: 500;
    color: var(--ink-12);
  }

  .unit,
  .delta {
    font-size: 12px;
    line-height: 16px;
    color: var(--ink-11);
  }
</style>
