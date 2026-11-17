<script lang="ts">
  import { t } from '$lib/i18n';
</script>

<div class="empty-today" data-testid="empty-today">
  <p class="title">{t('today.empty.title')}</p>
  <p class="body">{t('today.empty.body')}</p>
</div>

<style>
  .empty-today {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-width: 420px;
    padding: 32px 0;
  }

  p {
    margin: 0;
  }

  .title {
    font-size: 15px;
    line-height: 22px;
    color: var(--ink-11);
  }

  .body {
    font-size: 13px;
    line-height: 20px;
    color: var(--ink-10);
  }
</style>
