<script lang="ts">
  import { ChevronDown } from 'lucide-svelte';

  interface SelectOption {
    value: string;
    label: string;
    disabled?: boolean;
  }

  interface Props {
    label: string;
    value: string;
    options: SelectOption[];
    hint?: string;
    error?: string;
    disabled?: boolean;
    onchange?: (value: string) => void;
  }

  let {
    label,
    value = $bindable(),
    options,
    hint,
    error,
    disabled = false,
    onchange
  }: Props = $props();

  const uid = $props.id();
  const describedBy = $derived(hint || error ? `${uid}-hint` : undefined);

  function handleChange(event: Event) {
    value = (event.currentTarget as HTMLSelectElement).value;
    onchange?.(value);
  }
</script>

<div class="select-field">
  <label class="label" for={uid}>{label}</label>
  <div class="control" data-invalid={error ? '' : undefined}>
    <select
      id={uid}
      {value}
      {disabled}
      aria-invalid={error ? 'true' : undefined}
      aria-describedby={describedBy}
      onchange={handleChange}
    >
      {#each options as option (option.value)}
        <option value={option.value} disabled={option.disabled}>{option.label}</option>
      {/each}
    </select>
    <ChevronDown size={16} strokeWidth={1.5} aria-hidden="true" />
  </div>
  {#if hint || error}
    <p id={`${uid}-hint`} class="hint" class:error={Boolean(error)}>{error ?? hint}</p>
  {/if}
</div>

<style>
  .select-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .label {
    font-size: 13px;
    line-height: 20px;
    color: var(--ink-11);
  }

  .control {
    position: relative;
    display: flex;
    align-items: center;
    height: 36px;
    border: 1px solid var(--control-edge);
    border-radius: var(--radius-control);
    background: var(--well);
    color: var(--ink-11);
  }

  .control:focus-within {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
    border-color: var(--ink-11);
  }

  .control[data-invalid] {
    border-color: var(--critical);
    outline: 2px solid var(--critical);
    outline-offset: 1px;
  }

  select {
    appearance: none;
    width: 100%;
    height: 100%;
    padding: 0 32px 0 10px;
    border: 0;
    background: transparent;
    color: var(--ink-12);
    font-size: 13px;
    line-height: 20px;
  }

  .control :global(svg) {
    position: absolute;
    right: 10px;
    pointer-events: none;
  }

  .hint {
    margin: 0;
    min-height: 16px;
    font-size: 12px;
    line-height: 16px;
    color: var(--ink-10);
  }

  .hint.error {
    color: var(--critical);
  }

  @media (max-width: 767px) {
    .control {
      height: 44px;
    }

    select {
      font-size: 16px;
    }
  }
</style>
