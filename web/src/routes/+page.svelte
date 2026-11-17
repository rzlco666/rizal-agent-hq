<script lang="ts">
  import { EmptyState } from '$lib/components/ui';
  import { t } from '$lib/i18n';
</script>

<svelte:head>
  <title>{t('nav.today')} · {t('app.name')}</title>
</svelte:head>

<section class="page">
  <h1>{t('today.title')}</h1>
  <EmptyState title={t('today.empty.title')} body={t('today.empty.body')} />
</section>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 24px;
    max-width: 1408px;
    margin: 0 auto;
  }

  h1 {
    margin: 0;
    font-size: 32px;
    line-height: 40px;
    font-weight: 400;
    letter-spacing: -0.02em;
    color: var(--ink-12);
  }

  @media (max-width: 767px) {
    h1 {
      font-size: 24px;
      line-height: 32px;
    }
  }
</style>
