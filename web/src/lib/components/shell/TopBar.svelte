<script lang="ts">
  import { page } from '$app/state';
  import { Monitor, Moon, Sun } from 'lucide-svelte';
  import { t } from '$lib/i18n';
  import { ui, type Locale, type Theme } from '$lib/stores/ui.svelte';
  import Keycap from '../ui/Keycap.svelte';
  import { isActive, navItems } from './nav';

  interface Props {
    onopenAttention: () => void;
  }

  let { onopenAttention }: Props = $props();

  const titleKey = $derived(
    navItems.find((item) => isActive(page.url.pathname, item.href))?.labelKey ?? 'app.name'
  );
  const themes: Theme[] = ['system', 'day', 'night'];
  const themeIcons = { system: Monitor, day: Sun, night: Moon };
  const locales: Locale[] = ['id', 'en', 'zh'];
  const CurrentIcon = $derived(themeIcons[ui.theme]);
  const localeLabels: Record<Locale, string> = { id: 'ID', en: 'EN', zh: '中文' };

  function cycleTheme() {
    const next = themes[(themes.indexOf(ui.theme) + 1) % themes.length];
    ui.theme = next;
  }
</script>

<header>
  <a class="brand" href="/">{t('app.name')}</a>
  <span class="route-title">{t(titleKey)}</span>
  <div class="spacer"></div>

  <button
    type="button"
    class="command-trigger"
    onclick={() => (ui.paletteOpen = true)}
  >
    <span>{t('topbar.search')}</span>
    <Keycap keys={['Ctrl', 'K']} />
    <span class="sr-only">{t('topbar.palette')}</span>
  </button>

  <button type="button" class="theme" onclick={cycleTheme}>
    <CurrentIcon size={16} strokeWidth={1.5} aria-hidden="true" />
    <span>{t(`theme.${ui.theme}` as 'theme.day')}</span>
    <span class="sr-only">{t('topbar.theme')}</span>
  </button>

  <div class="locales" role="group" aria-label={t('topbar.locale')}>
    {#each locales as locale (locale)}
      <button
        type="button"
        class:active={ui.locale === locale}
        aria-pressed={ui.locale === locale}
        onclick={() => (ui.locale = locale)}
      >
        {localeLabels[locale]}
      </button>
    {/each}
  </div>

  <span class="connection" data-connection={ui.connection}>
    <span class="dot" aria-hidden="true"></span>
    <span>
      {ui.connection === 'live' ? t('topbar.connection.live') : t('topbar.connection.offline')}
    </span>
  </span>

  <button type="button" class="attention" aria-label={`${t('topbar.attention')} 0`} onclick={onopenAttention}>
    <span class="attention-count mono" aria-hidden="true">0</span>
  </button>
</header>

<style>
  header {
    display: flex;
    align-items: center;
    gap: 16px;
    height: 52px;
    flex: none;
    padding: 0 20px;
    border-bottom: 1px solid var(--hairline);
    background: var(--surface);
  }

  .brand {
    color: var(--ink-12);
    font-size: 13px;
    line-height: 20px;
    font-weight: 500;
    text-decoration: none;
    white-space: nowrap;
  }

  .route-title {
    color: var(--ink-10);
    font-size: 13px;
    line-height: 20px;
  }

  .spacer {
    flex: 1;
  }

  button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 36px;
    padding: 0 8px;
    border: 1px solid transparent;
    border-radius: var(--radius-control);
    background: transparent;
    color: var(--ink-11);
    font-size: 12px;
    line-height: 16px;
  }

  button:hover {
    background: var(--hover);
  }

  .command-trigger {
    width: 240px;
    justify-content: space-between;
    border-color: var(--control-edge);
    background: var(--well);
  }

  .locales {
    display: flex;
    gap: 2px;
  }

  .locales button {
    min-width: 32px;
    justify-content: center;
  }

  .locales button.active {
    color: var(--ink-12);
    box-shadow: inset 0 -2px 0 var(--ink-12);
  }

  .connection {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--ink-10);
    font-size: 12px;
    line-height: 16px;
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: var(--radius-dot);
    background: var(--success);
  }

  .connection[data-connection='offline'] .dot {
    background: var(--critical);
  }

  .attention {
    position: relative;
    width: 36px;
    justify-content: center;
    border-color: var(--hairline);
  }

  .attention-count {
    color: var(--ink-12);
  }

  @media (max-width: 1279px) {
    .route-title,
    .command-trigger span:first-child,
    .connection span:last-child,
    .theme span:not(.sr-only) {
      display: none;
    }

    .command-trigger {
      width: 68px;
    }
  }

  @media (max-width: 767px) {
    header {
      gap: 10px;
      padding: 0 16px;
    }

    .locales,
    .connection {
      display: none;
    }
  }
</style>
