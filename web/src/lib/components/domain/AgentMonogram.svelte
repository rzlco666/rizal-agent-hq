<script lang="ts">
  interface Props {
    name: string;
    status?: 'bekerja' | 'berpikir' | 'menunggu-rizal' | 'santai' | 'offline' | 'error';
    size?: 20 | 28 | 32;
    decorative?: boolean;
  }

  let { name, status, size = 28, decorative = false }: Props = $props();

  const statusTokens: Record<string, string> = {
    bekerja: 'var(--success)',
    berpikir: 'var(--info)',
    'menunggu-rizal': 'var(--attention)',
    santai: 'var(--ink-10)',
    offline: 'var(--ink-10)',
    error: 'var(--critical)'
  };

  const initials = $derived.by(() => {
    const words = name.trim().split(/\s+/).filter(Boolean);
    const chars = [...(words[0] ?? '')];
    if (words.length > 1) return (chars[0] ?? '') + ([...(words[1] ?? '')][0] ?? '');
    return chars.slice(0, 2).join('');
  });

  const dotSize = $derived(size >= 32 ? 8 : 6);
</script>

<span class="monogram" style:width={`${size}px`} style:height={`${size}px`}>
  <svg
    viewBox="0 0 32 32"
    role={decorative ? undefined : 'img'}
    aria-label={decorative ? undefined : `${name}${status ? ` — ${status}` : ''}`}
    aria-hidden={decorative || undefined}
    focusable="false"
  >
    <rect x="0.5" y="0.5" width="31" height="31" rx="6" fill="var(--well)" stroke="var(--hairline)" />
    <text
      x="16"
      y="17"
      text-anchor="middle"
      dominant-baseline="central"
      fill="var(--ink-12)"
      font-family="var(--font-mono)"
      font-size="12"
      font-weight="500"
    >{initials}</text>
  </svg>
  {#if status}
    <span
      class="status"
      aria-hidden="true"
      style:background={statusTokens[status]}
      style:width={`${dotSize}px`}
      style:height={`${dotSize}px`}
    ></span>
  {/if}
</span>

<style>
  .monogram {
    position: relative;
    display: inline-flex;
    flex: none;
  }

  svg {
    width: 100%;
    height: 100%;
    display: block;
  }

  .status {
    position: absolute;
    right: -1px;
    bottom: -1px;
    border-radius: var(--radius-dot);
    box-shadow: 0 0 0 2px var(--ground);
  }
</style>
