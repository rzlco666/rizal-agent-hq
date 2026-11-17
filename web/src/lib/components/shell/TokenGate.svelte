<script lang="ts">
  import { t } from '$lib/i18n';
  import Button from '../ui/Button.svelte';
  import Field from '../ui/Field.svelte';

  interface Props {
    required?: boolean;
    onSubmit?: (token: string) => void;
    error?: string;
  }

  let { required = false, onSubmit, error }: Props = $props();
  let token = $state('');
</script>

{#if required}
  <div class="token-gate" role="dialog" aria-modal="true" aria-label={t('token.title')}>
    <div class="card">
      <h2>{t('token.title')}</h2>
      <p>{t('token.body')}</p>
      <form
        onsubmit={(event) => {
          event.preventDefault();
          onSubmit?.(token);
        }}
      >
        <Field label={t('token.placeholder')} type="password" bind:value={token} {error} mono />
        <Button type="submit" variant="primary">{t('token.submit')}</Button>
      </form>
    </div>
  </div>
{/if}

<style>
  .token-gate {
    position: fixed;
    inset: 0;
    z-index: 95;
    display: grid;
    place-items: center;
    background: var(--ground);
  }

  .card {
    width: min(420px, calc(100vw - 32px));
    padding: 24px;
    border-radius: var(--radius-modal);
    background: var(--raised);
    box-shadow: var(--shadow-modal);
  }

  h2 {
    margin: 0;
    font-size: 20px;
    line-height: 28px;
    color: var(--ink-12);
  }

  p {
    margin: 8px 0 16px;
    color: var(--ink-11);
  }

  form {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
</style>
