<script lang="ts">
  import { t } from '$lib/i18n';
  import { X } from 'lucide-svelte';
  import type { ToastItem } from '$lib/stores/toast.svelte';
  import { dismissToast } from '$lib/stores/toast.svelte';
  import IconButton from './IconButton.svelte';
  import Button from './Button.svelte';

  interface Props {
    item: ToastItem;
  }

  let { item }: Props = $props();

  $effect(() => {
    if (item.tone === 'critical') return;
    const timer = window.setTimeout(
      () => dismissToast(item.id),
      item.timeoutMs ?? 4000
    );
    return () => window.clearTimeout(timer);
  });

  function runAction() {
    item.action?.run();
    dismissToast(item.id);
  }
</script>

<div class="toast" role={item.tone === 'critical' ? 'alert' : 'status'} data-tone={item.tone}>
  <span class="mark" aria-hidden="true"></span>
  <span class="message">{item.message}</span>
  {#if item.action}
    <Button variant="ghost" size="sm" onclick={runAction}>{item.action.label}</Button>
  {/if}
  <IconButton
    icon={X}
    label={t('toast.close')}
    size="sm"
    onclick={() => dismissToast(item.id)}
  />
</div>

<style>
  .toast {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 320px;
    max-width: 480px;
    padding: 12px 12px 12px 16px;
    border-radius: var(--radius-card);
    background: var(--raised);
    box-shadow: var(--shadow-menu);
  }

  .mark {
    width: 8px;
    height: 8px;
    flex: none;
    border-radius: var(--radius-dot);
    background: var(--info);
  }

  .toast[data-tone='success'] .mark { background: var(--success); }
  .toast[data-tone='critical'] .mark { background: var(--critical); }

  .message {
    flex: 1;
    color: var(--ink-12);
  }

  @media (max-width: 767px) {
    .toast {
      min-width: 0;
      width: calc(100vw - 32px);
    }
  }
</style>
