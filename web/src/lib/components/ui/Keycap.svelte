<script lang="ts">
  interface Props {
    keys: string[];
    label?: string;
  }

  let { keys, label }: Props = $props();
</script>

<span class="keycap-group" aria-label={label}>
  {#each keys as key, i (i)}
    {#if i > 0}<span class="separator" aria-hidden="true"> </span>{/if}
    <kbd>{key}</kbd>
  {/each}
</span>

<style>
  .keycap-group {
    display: inline-flex;
    align-items: center;
    gap: 2px;
  }

  kbd {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 18px;
    height: 18px;
    padding: 0 4px;
    border: 1px solid var(--hairline);
    border-radius: var(--radius-keycap);
    background: var(--well);
    color: var(--ink-11);
    font-family: var(--font-mono);
    font-size: 11px;
    line-height: 16px;
  }

  .separator {
    width: 4px;
  }
</style>
