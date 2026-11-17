<script lang="ts">
  import type { HTMLInputAttributes } from 'svelte/elements';

  interface Props extends HTMLInputAttributes {
    invalid?: boolean;
    mono?: boolean;
  }

  let {
    invalid = false,
    mono = false,
    type = 'text',
    class: className = '',
    value = $bindable(''),
    ...rest
  }: Props = $props();
</script>

<input
  {...rest}
  {type}
  bind:value
  aria-invalid={invalid || undefined}
  data-invalid={invalid || undefined}
  class="input {mono ? 'mono' : ''} {className}"
/>

<style>
  .input {
    width: 100%;
    height: 36px;
    padding: 0 10px;
    border: 1px solid var(--control-edge);
    border-radius: var(--radius-control);
    background: var(--well);
    color: var(--ink-12);
    font-size: 13px;
    line-height: 20px;
  }

  .input::placeholder {
    color: var(--ink-10);
  }

  .input:hover:not(:disabled) {
    border-color: var(--ink-11);
  }

  .input:focus {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
    border-color: var(--ink-11);
  }

  .input[data-invalid] {
    border-color: var(--critical);
    outline: 2px solid var(--critical);
    outline-offset: 1px;
  }

  .input:disabled {
    opacity: 0.45;
  }

  .input:read-only {
    color: var(--ink-11);
  }

  @media (max-width: 767px) {
    .input {
      height: 44px;
      font-size: 16px;
    }
  }
</style>
