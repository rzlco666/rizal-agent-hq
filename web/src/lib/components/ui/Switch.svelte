<script lang="ts">
  interface Props {
    checked: boolean;
    label: string;
    description?: string;
    disabled?: boolean;
    onchange?: (checked: boolean) => void;
  }

  let {
    checked = $bindable(),
    label,
    description,
    disabled = false,
    onchange
  }: Props = $props();

  const uid = $props.id();

  function handleChange(event: Event) {
    checked = (event.currentTarget as HTMLInputElement).checked;
    onchange?.(checked);
  }
</script>

<div class="switch" data-disabled={disabled || undefined}>
  <input
    id={uid}
    type="checkbox"
    role="switch"
    {checked}
    aria-checked={checked}
    {disabled}
    aria-describedby={description ? `${uid}-description` : undefined}
    onchange={handleChange}
  />
  <label for={uid}>
    <span>{label}</span>
    {#if description}<span id={`${uid}-description`} class="description">{description}</span>{/if}
  </label>
</div>

<style>
  .switch {
    display: inline-flex;
    align-items: center;
    gap: 12px;
    min-height: 44px;
  }

  input {
    appearance: none;
    position: relative;
    width: 40px;
    height: 24px;
    flex: none;
    margin: 0;
    border: 1px solid var(--control-edge);
    border-radius: var(--radius-dot);
    background: var(--well);
    cursor: pointer;
  }

  input::after {
    content: '';
    position: absolute;
    top: 1px;
    left: 1px;
    width: 20px;
    height: 20px;
    border-radius: var(--radius-dot);
    background: var(--surface);
    box-shadow: var(--shadow-ring);
    transition: transform var(--dur-1) var(--ease);
  }

  input:checked {
    background: var(--ink-12);
    border-color: var(--ink-12);
  }

  input:checked::after {
    transform: translateX(16px);
  }

  input:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 2px;
    color: var(--ink-12);
    font-size: 13px;
    line-height: 20px;
    cursor: pointer;
  }

  .description {
    color: var(--ink-10);
    font-size: 12px;
    line-height: 16px;
  }

  .switch[data-disabled] {
    opacity: 0.45;
  }

  @media (max-width: 767px) {
    input {
      width: 51px;
      height: 31px;
    }

    input::after {
      width: 23px;
      height: 23px;
    }

    input:checked::after {
      transform: translateX(20px);
    }
  }
</style>
