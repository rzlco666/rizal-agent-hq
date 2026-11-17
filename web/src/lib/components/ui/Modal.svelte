<script lang="ts">
  import type { Snippet } from 'svelte';
  import { X } from 'lucide-svelte';
  import { t } from '$lib/i18n';
  import { focusFirst, lockScroll, trapTab } from '$lib/keyboard/focus';
  import Button from './Button.svelte';
  import IconButton from './IconButton.svelte';

  interface Props {
    open: boolean;
    title: string;
    size?: 'confirm' | 'form';
    confirmLabel?: string;
    confirmTone?: 'primary' | 'destructive';
    busy?: boolean;
    onclose: () => void;
    onconfirm?: () => void;
    children: Snippet;
    description?: string;
  }

  let {
    open,
    title,
    size = 'form',
    confirmLabel,
    confirmTone = 'primary',
    busy = false,
    onclose,
    onconfirm,
    children,
    description
  }: Props = $props();

  const uid = $props.id();
  let dialog = $state<HTMLElement | undefined>();
  let previous: HTMLElement | null = null;
  let unlock: (() => void) | null = null;

  $effect(() => {
    if (open) {
      previous = document.activeElement as HTMLElement | null;
      unlock = lockScroll();
      queueMicrotask(() => {
        const root = document.getElementById(`${uid}-dialog`);
        if (!root) return;
        if (size === 'confirm') root.querySelector<HTMLElement>('button[data-action="cancel"]')?.focus();
        else focusFirst(root);
      });
    } else {
      unlock?.();
      unlock = null;
      previous?.focus();
      previous = null;
    }
    return () => unlock?.();
  });

  function handleKeydown(event: KeyboardEvent) {
    trapTab(event, event.currentTarget as HTMLElement);
    if (event.key === 'Escape' && !busy) {
      event.preventDefault();
      event.stopPropagation();
      onclose();
    }
  }
</script>

{#if open}
  <div class="scrim" aria-hidden="true"></div>
  <div
    id={`${uid}-dialog`}
    class="modal"
    data-size={size}
    role="dialog"
    tabindex="-1"
    aria-modal="true"
    aria-labelledby={`${uid}-title`}
    aria-describedby={description ? `${uid}-description` : undefined}
    onkeydown={handleKeydown}
  >
    <div class="header">
      <h2 id={`${uid}-title`}>{title}</h2>
      <IconButton icon={X} label={t('action.close')} size="sm" onclick={onclose} />
    </div>
    {#if description}<p id={`${uid}-description`} class="description">{description}</p>{/if}
    <div class="body">{@render children()}</div>
    {#if onconfirm || size === 'confirm'}
      <div class="footer">
        <Button
          variant="secondary"
          data-action="cancel"
          onclick={onclose}
        >{t('action.cancel')}</Button>
        <Button
          variant={confirmTone}
          data-action="confirm"
          loading={busy}
          disabled={busy}
          onclick={onconfirm}
        >
          {confirmLabel ?? t('action.confirm')}
        </Button>
      </div>
    {/if}
  </div>
{/if}

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 100;
    background: var(--scrim);
  }

  .modal {
    position: fixed;
    top: 50%;
    left: 50%;
    z-index: 101;
    transform: translate(-50%, -50%);
    width: min(620px, calc(100vw - 32px));
    max-height: calc(100vh - 64px);
    overflow-y: auto;
    padding: 16px;
    border-radius: var(--radius-modal);
    background: var(--raised);
    box-shadow: var(--shadow-modal);
  }

  .modal[data-size='confirm'] {
    width: min(460px, calc(100vw - 32px));
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }

  h2 {
    margin: 0;
    font-size: 20px;
    line-height: 20px;
    font-weight: 600;
    color: var(--ink-12);
  }

  .description,
  .body {
    color: var(--ink-11);
  }

  .description {
    margin: 12px 0 0;
    font-size: 13px;
    line-height: 20px;
  }

  .body {
    padding: 16px 0;
    font-size: 16px;
    line-height: 24px;
  }

  .footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding-top: 12px;
    border-top: 1px solid var(--hairline);
  }

  .modal :global([data-action='confirm'][data-variant='destructive']) {
    background: var(--critical);
    color: var(--on-critical);
  }
</style>
