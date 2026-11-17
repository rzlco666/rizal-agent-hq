<script lang="ts">
  import { Search } from 'lucide-svelte';
  import { tick } from 'svelte';
  import { t, type TranslationKey } from '$lib/i18n';
  import { lockScroll, trapTab } from '$lib/keyboard/focus';
  import type { PaletteAction } from '$lib/keyboard/palette';
  import Keycap from './Keycap.svelte';


  interface Props {
    open: boolean;
    actions: PaletteAction[];
    recent: string[];
    onclose: () => void;
  }

  let { open, actions, recent = [], onclose }: Props = $props();

  const uid = $props.id();
  const groups = ['navigate', 'agent', 'alert', 'approval', 'view', 'setting'] as const;
  let query = $state('');
  let activeIndex = $state(0);
  let input = $state<HTMLInputElement | undefined>();
  let dialog = $state<HTMLElement | undefined>();
  let previous: HTMLElement | null = null;
  let unlock: (() => void) | null = null;

  function score(action: PaletteAction, text: string): number {
    if (!text) return 1;
    const candidates = [action.label, ...(action.keywords ?? [])].map((value) =>
      value.toLowerCase()
    );
    if (candidates.some((value) => value.startsWith(text))) return 3;
    if (candidates.some((value) => value.includes(text))) return 2;
    return -1;
  }

  const ranked = $derived.by(() => {
    const text = query.trim().toLowerCase();
    return actions
      .map((action) => ({
        action,
        value:
          score(action, text) + (text && recent.includes(action.id) ? 1 : 0)
      }))
      .filter((entry) => entry.value > 0)
      .sort((a, b) => b.value - a.value)
      .map((entry) => entry.action);
  });

  const grouped = $derived(
    groups
      .map((group) => ({
        group,
        items: ranked.filter((action) => action.group === group).slice(0, 8)
      }))
      .filter((entry) => entry.items.length > 0)
  );

  const flat = $derived(grouped.flatMap((entry) => entry.items));

  $effect(() => {
    if (!open) return;
    query = '';
    activeIndex = 0;
    previous = document.activeElement as HTMLElement | null;
    unlock = lockScroll();
    void tick().then(() => input?.focus());
    return () => {
      unlock?.();
      unlock = null;
      previous?.focus();
      previous = null;
    };
  });

  function move(delta: number) {
    if (flat.length === 0) return;
    activeIndex = (activeIndex + delta + flat.length) % flat.length;
  }

  async function activate() {
    const action = flat[activeIndex];
    if (!action || action.disabledReason) return;
    onclose();
    await action.run();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (dialog) trapTab(event, dialog);
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      move(1);
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      move(-1);
    } else if (event.key === 'Home') {
      event.preventDefault();
      activeIndex = 0;
    } else if (event.key === 'End') {
      event.preventDefault();
      activeIndex = Math.max(0, flat.length - 1);
    } else if (event.key === 'Enter') {
      event.preventDefault();
      void activate();
    } else if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      onclose();
    }
  }
</script>

{#if open}
  <div class="scrim" aria-hidden="true"></div>
  <div
    class="palette"
    role="dialog"
    tabindex="-1"
    aria-modal="true"
    aria-label={t('palette.title')}
    onkeydown={handleKeydown}
  >
    <div class="search">
      <Search size={16} strokeWidth={1.5} aria-hidden="true" />
      <input
        bind:this={input}
        bind:value={query}
        role="combobox"
        aria-label={t('palette.placeholder')}
        aria-expanded="true"
        aria-controls={`${uid}-results`}
        aria-activedescendant={flat[activeIndex] ? `${uid}-option-${flat[activeIndex].id}` : undefined}
        autocomplete="off"
        placeholder={t('palette.placeholder')}
      />
      <Keycap keys={['Esc']} label={t('palette.close')} />
    </div>
    <div class="results" id={`${uid}-results`} role="listbox" aria-label={t('palette.title')}>
      {#if flat.length === 0}
        <p class="empty">{t('palette.empty')}</p>
      {:else}
        {#each grouped as entry (entry.group)}
          <div role="group" aria-label={t(`palette.group.${entry.group}` as TranslationKey)}>
            <p class="group-label">{t(`palette.group.${entry.group}` as TranslationKey)}</p>
            {#each entry.items as action (action.id)}
              {@const index = flat.indexOf(action)}
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <div
                id={`${uid}-option-${action.id}`}
                role="option"
                tabindex="-1"
                aria-selected={index === activeIndex}
                aria-disabled={Boolean(action.disabledReason)}
                class="option"
                onclick={() => {
                  activeIndex = index;
                  void activate();
                }}
                onmousemove={() => (activeIndex = index)}
              >
                <span>{action.label}</span>
                {#if action.disabledReason}
                  <span class="reason">{action.disabledReason}</span>
                {:else if action.shortcut}
                  <Keycap keys={[action.shortcut]} />
                {/if}
              </div>
            {/each}
          </div>
        {/each}
      {/if}
    </div>
  </div>
{/if}

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 110;
    background: var(--scrim);
  }

  .palette {
    position: fixed;
    top: 12vh;
    left: 50%;
    z-index: 111;
    transform: translateX(-50%);
    width: min(560px, calc(100vw - 32px));
    max-height: 76vh;
    overflow: hidden;
    border-radius: var(--radius-modal);
    background: var(--raised);
    box-shadow: var(--shadow-modal);
  }

  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 44px;
    padding: 0 12px;
    border-bottom: 1px solid var(--hairline);
    color: var(--ink-10);
  }

  input {
    flex: 1;
    height: 100%;
    border: 0;
    background: transparent;
    color: var(--ink-12);
    font-size: 14px;
  }

  input:focus {
    outline: none;
  }

  .search:focus-within {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }

  .results {
    max-height: calc(76vh - 44px);
    overflow-y: auto;
    padding: 8px;
  }

  .group-label {
    margin: 8px 8px 4px;
    font-size: 12px;
    line-height: 16px;
    color: var(--ink-10);
  }

  .option {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    height: 40px;
    padding: 0 8px;
    border-radius: var(--radius-control);
    color: var(--ink-12);
    cursor: pointer;
  }

  .option[aria-selected='true'] {
    background: var(--attention-tint);
    box-shadow: inset 2px 0 0 var(--attention);
  }

  .option[aria-disabled='true'] {
    opacity: 0.5;
  }

  .reason {
    font-size: 12px;
    color: var(--ink-10);
  }

  .empty {
    margin: 0;
    padding: 24px 12px;
    text-align: center;
    color: var(--ink-10);
  }
</style>
