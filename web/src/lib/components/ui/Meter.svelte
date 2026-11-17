<script lang="ts">
  interface Props {
    value: number;
    max: number;
    label: string;
    showValue?: boolean;
  }

  let { value, max, label, showValue = true }: Props = $props();
  const percent = $derived(max > 0 ? (value / max) * 100 : 0);
  const level = $derived(percent >= 100 ? 'critical' : percent > 80 ? 'attention' : 'normal');
</script>

<div class="meter">
  <div
    class="track"
    role="progressbar"
    aria-label={label}
    aria-valuemin="0"
    aria-valuemax={max}
    aria-valuenow={value}
    data-level={level}
  >
    <div class="fill" style:width={`${Math.min(100, Math.max(0, percent))}%`}></div>
  </div>
  {#if showValue}<span class="value mono">{Math.round(percent)}%</span>{/if}
</div>

<style>
  .meter {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .track {
    flex: 1;
    height: 6px;
    border-radius: var(--radius-dot);
    background: var(--well);
    overflow: hidden;
  }

  .fill {
    height: 100%;
    background: var(--ink-12);
  }

  .track[data-level='attention'] .fill {
    background: var(--attention);
  }

  .track[data-level='critical'] .fill {
    background: var(--critical);
  }

  .value {
    font-size: 12px;
    line-height: 16px;
    color: var(--ink-11);
  }
</style>
