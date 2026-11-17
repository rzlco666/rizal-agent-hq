<script lang="ts">
  import type { Component } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  interface Props extends HTMLButtonAttributes {
    icon: Component;
    label: string;
    variant?: 'ghost' | 'secondary';
    size?: 'sm' | 'md';
    pressed?: boolean;
  }

  let {
    icon: Icon,
    label,
    variant = 'ghost',
    size = 'md',
    pressed,
    class: className = '',
    type = 'button',
    ...rest
  }: Props = $props();
</script>

<button
  {...rest}
  {type}
  aria-label={label}
  title={label}
  aria-pressed={pressed}
  data-variant={variant}
  data-size={size}
  class="icon-button {className}"
>
  <Icon size={16} strokeWidth={1.5} />
</button>

<style>
  .icon-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    flex: none;
    padding: 0;
    border: 1px solid transparent;
    border-radius: var(--radius-control);
    background: transparent;
    color: var(--ink-11);
    transition: background var(--dur-1) var(--ease);
  }

  .icon-button[data-size='sm'] {
    width: 32px;
    height: 32px;
  }

  .icon-button[data-variant='secondary'] {
    background: var(--raised);
    color: var(--ink-12);
    box-shadow: var(--shadow-button);
  }

  .icon-button:hover:not(:disabled) {
    background: var(--hover);
  }

  .icon-button[aria-pressed='true'] {
    background: var(--attention-tint);
    color: var(--attention);
  }

  .icon-button:disabled {
    opacity: 0.45;
  }

  @media (max-width: 767px) {
    .icon-button,
    .icon-button[data-size='sm'] {
      width: 44px;
      height: 44px;
    }
  }
</style>
