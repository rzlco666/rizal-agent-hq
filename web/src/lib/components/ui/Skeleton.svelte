<script lang="ts">
  interface Props {
    shape?: 'text' | 'block' | 'row';
    width?: string;
    height?: string;
    lines?: number;
  }

  let { shape = 'text', width = '100%', height, lines = 1 }: Props = $props();

  const heights: Record<string, string> = {
    text: '12px',
    block: '64px',
    row: '38px'
  };
</script>

<div class="skeleton-stack" aria-hidden="true">
  {#each Array(lines) as _, i (i)}
    <div
      class="skeleton"
      style:width
      style:height={height ?? heights[shape]}
      style:border-radius={shape === 'row' ? 'var(--radius-control)' : 'var(--radius-keycap)'}
    ></div>
  {/each}
</div>

<style>
  .skeleton-stack {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .skeleton {
    background: var(--well);
    border-radius: var(--radius-keycap);
  }
</style>
