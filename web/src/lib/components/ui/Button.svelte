<script lang="ts">
  import type { ComponentType, Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  interface Props extends HTMLButtonAttributes {
    variant?: 'primary' | 'secondary' | 'ghost' | 'destructive';
    size?: 'sm' | 'md' | 'lg';
    loading?: boolean;
    icon?: ComponentType;
    iconAfter?: ComponentType;
    children?: Snippet;
  }

  let {
    variant = 'secondary',
    size = 'md',
    loading = false,
    icon: Icon,
    iconAfter: IconAfter,
    children,
    class: className = '',
    type = 'button',
    disabled = false,
    onclick,
    ...rest
  }: Props = $props();

  function handleClick(event: MouseEvent & { currentTarget: HTMLButtonElement }) {
    if (loading) {
      event.preventDefault();
      return;
    }
    onclick?.(event);
  }
</script>

<button
  {...rest}
  {type}
  {disabled}
  data-variant={variant}
  data-size={size}
  aria-busy={loading || undefined}
  class="btn {className}"
  onclick={handleClick}
>
  {#if loading}
    <span class="spinner" aria-hidden="true"></span>
  {:else if Icon}
    <span class="icon"><Icon size={16} strokeWidth={1.5} /></span>
  {/if}
  {#if children}<span class="label">{@render children()}</span>{/if}
  {#if IconAfter}<span class="icon"><IconAfter size={16} strokeWidth={1.5} /></span>{/if}
</button>

<style>
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    height: 36px;
    padding: 0 14px;
    border-radius: var(--radius-control);
    background: transparent;
    border: 1px solid transparent;
    color: var(--ink-11);
    font-size: 13px;
    line-height: 20px;
    font-weight: 500;
    white-space: nowrap;
    transition: background var(--dur-1) var(--ease), transform var(--dur-1) var(--ease);
  }

  .btn:has(.icon) {
    padding-inline: 12px;
  }

  .btn[data-size='sm'] {
    height: 32px;
  }

  .btn[data-variant='primary'] {
    background: var(--primary-bg);
    color: var(--primary-fg);
    box-shadow: var(--shadow-button);
  }

  .btn[data-variant='primary']:hover:not(:disabled) {
    background: var(--primary-hover);
  }

  .btn[data-variant='secondary'] {
    background: var(--raised);
    color: var(--ink-12);
    box-shadow: var(--shadow-button);
  }

  .btn[data-variant='secondary']:hover:not(:disabled),
  .btn[data-variant='ghost']:hover:not(:disabled) {
    background: var(--hover);
  }

  .btn[data-variant='destructive'] {
    color: var(--critical);
  }

  .btn[data-variant='destructive']:hover:not(:disabled) {
    background: var(--hover);
  }

  .btn:active:not(:disabled) {
    transform: scale(0.97);
  }

  .btn:disabled {
    opacity: 0.45;
  }

  .icon,
  .spinner {
    display: inline-flex;
    width: 16px;
    height: 16px;
    flex: none;
  }

  .spinner {
    border: 2px solid currentColor;
    border-right-color: transparent;
    border-radius: var(--radius-dot);
    animation: spin 700ms linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  @media (max-width: 767px) {
    .btn,
    .btn[data-size='sm'] {
      height: 44px;
      font-size: 15px;
    }
  }
</style>
