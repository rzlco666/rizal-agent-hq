<script lang="ts">
  import type { Snippet } from 'svelte';
  interface TabItem {
    id: string;
    label: string;
    count?: number;
    disabled?: boolean;
  }

  interface Props {
    tabs: TabItem[];
    value: string;
    ariaLabel?: string;
    onchange?: (id: string) => void;
    children?: Snippet;
  }

  let {
    tabs,
    value = $bindable(),
    ariaLabel = 'Tabs',
    onchange,
    children
  }: Props = $props();

  const uid = $props.id();
  let tabNodes = $state<Record<string, HTMLButtonElement>>({});

  function select(id: string) {
    value = id;
    onchange?.(id);
  }

  function handleKeydown(event: KeyboardEvent, index: number) {
    const enabled = tabs.map((tab, i) => ({ tab, i })).filter(({ tab }) => !tab.disabled);
    if (enabled.length === 0) return;
    const current = enabled.findIndex(({ i }) => i === index);
    let next = current;
    if (event.key === 'ArrowRight') next = (current + 1) % enabled.length;
    else if (event.key === 'ArrowLeft') next = (current - 1 + enabled.length) % enabled.length;
    else if (event.key === 'Home') next = 0;
    else if (event.key === 'End') next = enabled.length - 1;
    else if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      if (!tabs[index].disabled) select(tabs[index].id);
      return;
    } else return;

    event.preventDefault();
    const target = enabled[next];
    select(target.tab.id);
    tabNodes[target.tab.id]?.focus();
  }
</script>

<div class="tabs">
  <div class="tablist" role="tablist" aria-label={ariaLabel}>
    {#each tabs as tab, i (tab.id)}
      <button
        bind:this={tabNodes[tab.id]}
        type="button"
        role="tab"
        id={`${uid}-tab-${tab.id}`}
        aria-selected={value === tab.id}
        aria-controls={`${uid}-panel-${tab.id}`}
        aria-disabled={tab.disabled || undefined}
        tabindex={value === tab.id ? 0 : -1}
        class="tab"
        onclick={() => !tab.disabled && select(tab.id)}
        onkeydown={(event) => handleKeydown(event, i)}
      >
        <span>{tab.label}</span>
        {#if tab.count !== undefined}<span class="count mono">{tab.count}</span>{/if}
      </button>
    {/each}
  </div>
  {#each tabs as tab (tab.id)}
    {#if value === tab.id}
      <div
        role="tabpanel"
        id={`${uid}-panel-${tab.id}`}
        aria-labelledby={`${uid}-tab-${tab.id}`}
        tabindex="0"
        class="panel"
      >
        {@render children?.()}
      </div>
    {/if}
  {/each}
</div>

<style>
  .tabs {
    border-bottom: 1px solid var(--hairline);
  }

  .tablist {
    display: flex;
    align-items: center;
    gap: 16px;
    height: 40px;
  }

  .tab {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 40px;
    padding: 0;
    border: 0;
    border-bottom: 2px solid transparent;
    background: transparent;
    color: var(--ink-10);
    font-size: 13px;
    line-height: 20px;
    font-weight: 500;
  }

  .tab:hover {
    color: var(--ink-11);
  }

  .tab[aria-selected='true'] {
    color: var(--ink-12);
    border-bottom-color: var(--ink-12);
  }

  .tab[aria-disabled='true'] {
    opacity: 0.45;
  }

  .count {
    font-size: 12px;
  }

  .panel:focus {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }
</style>
