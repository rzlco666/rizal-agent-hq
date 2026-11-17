<script lang="ts">
  import type { Snippet } from 'svelte';
  import { X } from 'lucide-svelte';
  import { t } from '$lib/i18n';
  import { focusFirst, lockScroll, trapTab } from '$lib/keyboard/focus';
  import IconButton from './IconButton.svelte';

  interface Props {
    open: boolean;
    title: string;
    onclose: () => void;
    footer?: Snippet;
    children: Snippet;
    side?: 'right' | 'bottom';
  }

  let { open, title, onclose, footer, children, side = 'right' }: Props = $props();

  const uid = $props.id();
  let panel = $state<HTMLElement | undefined>();
  let previous: HTMLElement | null = null;
  let unlock: (() => void) | null = null;

  $effect(() => {
    if (open) {
      previous = document.activeElement as HTMLElement | null;
      unlock = lockScroll();
      queueMicrotask(() => panel && focusFirst(panel));
    } else {
      unlock?.();
      unlock = null;
      previous?.focus();
      previous = null;
    }
    return () => unlock?.();
  });

  function handleKeydown(event: KeyboardEvent) {
    if (panel) trapTab(event, panel);
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      onclose();
    }
  }
</script>

{#if open}
  <div class="scrim" aria-hidden="true"></div>
  <div tabindex="-1"
    class="drawer"
    data-side={side}
    bind:this={panel}
    role="dialog"
    aria-modal="true"
    aria-labelledby={`${uid}-title`}
    onkeydown={handleKeydown}
  >
    <div class="grab">
      <button type="button" class="handle" aria-label={t('action.close')} onclick={onclose}></button>
    </div>
    <header>
      <h2 id={`${uid}-title`}>{title}</h2>
      <IconButton icon={X} label={t('action.close')} size="sm" onclick={onclose} />
    </header>
    <div class="content">{@render children()}</div>
    {#if footer}<footer>{@render footer()}</footer>{/if}
  </div>
{/if}

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 100;
    background: var(--scrim);
  }

  .drawer {
    position: fixed;
    top: 52px;
    right: 0;
    bottom: 0;
    z-index: 101;
    display: flex;
    flex-direction: column;
    width: min(480px, 100vw);
    background: var(--surface);
    border-left: 1px solid var(--hairline);
    box-shadow: var(--shadow-sheet);
  }

  .grab {
    display: none;
  }

  header,
  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex: none;
    padding: 0 16px;
  }

  header {
    height: 56px;
    border-bottom: 1px solid var(--hairline);
  }

  footer {
    height: 64px;
    border-top: 1px solid var(--hairline);
  }

  h2 {
    margin: 0;
    font-size: 16px;
    line-height: 24px;
    font-weight: 500;
    color: var(--ink-12);
  }

  .content {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
    color: var(--ink-11);
  }

  @media (max-width: 767px) {
    .drawer {
      top: auto;
      left: 0;
      width: 100vw;
      max-height: 85vh;
      border-left: 0;
      border-top: 1px solid var(--hairline);
      border-radius: var(--radius-modal) var(--radius-modal) 0 0;
    }

    .grab {
      display: flex;
      justify-content: center;
      flex: none;
      height: 44px;
      align-items: center;
    }

    .handle {
      width: 36px;
      height: 5px;
      padding: 0;
      border: 0;
      border-radius: var(--radius-dot);
      background: var(--control-edge);
    }
  }
</style>
