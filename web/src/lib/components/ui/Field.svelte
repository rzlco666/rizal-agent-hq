<script lang="ts">
  import { t } from '$lib/i18n';
  import Input from './Input.svelte';

  interface Props {
    label: string;
    value?: string;
    type?: 'text' | 'search' | 'number' | 'password' | 'url' | 'email';
    placeholder?: string;
    hint?: string;
    error?: string;
    required?: boolean;
    disabled?: boolean;
    readonly?: boolean;
    mono?: boolean;
    name?: string;
    autocomplete?: string;
    oninput?: (value: string) => void;
    onblur?: () => void;
    onsubmit?: () => void;
  }

  let {
    label,
    value = $bindable(''),
    type = 'text',
    placeholder,
    hint,
    error,
    required = false,
    disabled = false,
    readonly = false,
    mono = false,
    name,
    autocomplete,
    oninput,
    onblur,
    onsubmit
  }: Props = $props();

  const uid = $props.id();
  const describedBy = $derived(hint || error ? `${uid}-hint` : undefined);

  function handleInput(event: Event) {
    oninput?.((event.currentTarget as HTMLInputElement).value);
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter') {
      event.preventDefault();
      onsubmit?.();
    } else if (event.key === 'Escape' && type === 'search') {
      value = '';
    }
  }
</script>

<div class="field" data-error={error ? '' : undefined}>
  <label for={uid} class="label">
    {label}{#if required}<span class="required"> — {t('field.required')}</span>{/if}
  </label>
  <Input
    id={uid}
    {value}
    {type}
    {placeholder}
    {disabled}
    {readonly}
    {mono}
    {name}
    {autocomplete}
    invalid={Boolean(error)}
    aria-describedby={describedBy}
    oninput={handleInput}
    onblur={() => onblur?.()}
    onkeydown={handleKeydown}
  />
  {#if hint || error}
    <p id={`${uid}-hint`} class="hint" class:error={Boolean(error)}>
      {error ?? hint}
    </p>
  {/if}
</div>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .label {
    font-size: 13px;
    line-height: 20px;
    color: var(--ink-11);
  }

  .required {
    color: var(--ink-10);
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
</style>
