<script lang="ts">
  type Status = 'bekerja' | 'berpikir' | 'menunggu-rizal' | 'santai' | 'offline' | 'error';

  interface Props {
    status: Status;
    label?: string;
    size?: 6 | 8;
    showLabel?: boolean;
  }

  let { status, label, size = 8, showLabel = false }: Props = $props();

  const tokens: Record<Status, string> = {
    bekerja: 'var(--success)',
    berpikir: 'var(--info)',
    'menunggu-rizal': 'var(--attention)',
    santai: 'var(--ink-10)',
    offline: 'var(--ink-10)',
    error: 'var(--critical)'
  };

  const statusLabel = $derived(label ?? status);
</script>

<span class="status-dot">
  <span
    class="dot"
    class:ring={status === 'offline'}
    aria-hidden="true"
    style:background={tokens[status]}
    style:width={`${size}px`}
    style:height={`${size}px`}
  ></span>
  {#if showLabel}
    <span class="label">{statusLabel}</span>
  {:else}
    <span class="sr-only">{statusLabel}</span>
  {/if}
</span>

<style>
  .status-dot {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .dot {
    display: inline-block;
    flex: none;
    border-radius: var(--radius-dot);
  }

  .dot.ring {
    box-shadow: 0 0 0 1px var(--hairline-strong);
  }

  .label {
    font-size: 12px;
    line-height: 16px;
    color: var(--ink-11);
  }
</style>
