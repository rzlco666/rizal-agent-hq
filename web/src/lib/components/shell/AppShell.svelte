<script lang="ts">
  import { goto } from '$app/navigation';
  import type { Snippet } from 'svelte';

  import { t } from '$lib/i18n';
  import type { PaletteAction } from '$lib/keyboard/palette';
  import { attention } from '$lib/stores/attention.svelte';
  import { ui } from '$lib/stores/ui.svelte';
  import AttentionRail from '../domain/AttentionRail.svelte';
  import CommandPalette from '../ui/CommandPalette.svelte';
  import Drawer from '../ui/Drawer.svelte';
  import Toaster from '../ui/Toaster.svelte';
  import OfflineBanner from './OfflineBanner.svelte';
  import Sidebar from './Sidebar.svelte';
  import TopBar from './TopBar.svelte';
  import TokenGate from './TokenGate.svelte';
  import { navItems } from './nav';

  interface Props {
    children: Snippet;
  }

  let { children }: Props = $props();
  let attentionOpen = $state(false);

  const actions = $derived.by<PaletteAction[]>(() => [
    ...navItems.map((item) => ({
      id: `navigate-${item.id}`,
      label: t(item.labelKey),
      group: 'navigate' as const,
      keywords: [item.id, item.href],
      run: () => goto(item.href)
    })),
    {
      id: 'view-theme',
      label: t('topbar.theme'),
      group: 'view' as const,
      keywords: ['tema', 'theme', 'dark', 'light'],
      run: () => { ui.theme = ui.theme === 'night' ? 'day' : 'night'; }
    },
    {
      id: 'setting-locale',
      label: t('topbar.locale'),
      group: 'setting' as const,
      keywords: ['bahasa', 'language', 'locale'],
      run: () => { ui.locale = ui.locale === 'id' ? 'en' : 'id'; }
    }
  ]);

  function handleShortcut(event: KeyboardEvent) {
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
      event.preventDefault();
      ui.paletteOpen = !ui.paletteOpen;
    }
  }

  function openAttention(item: { id: string; href?: string }) {
    attentionOpen = false;
    if (item.href) void goto(item.href);
  }
</script>

<svelte:window onkeydown={handleShortcut} />

<a class="skip-link" href="#main">{t('action.view')}</a>
<TokenGate />

<div class="app">
  <TopBar onopenAttention={() => (attentionOpen = true)} />
  <OfflineBanner />
  <div class="body">
    <Sidebar />
    <main id="main" data-testid="shell-main" tabindex="-1">
      {@render children()}
    </main>
    <div class="rail-slot">
      <AttentionRail
        alerts={attention.items}
        onopen={openAttention}
        onack={(item) => void attention.ack(item.id)}
        onsnooze={(item) => void attention.snooze(item.id)}
      />
    </div>
  </div>
</div>

<Drawer open={attentionOpen} title={t('attention.title')} onclose={() => (attentionOpen = false)}>
  <div class="sheet-rail">
    <AttentionRail
      alerts={attention.items}
      onopen={openAttention}
      onack={(item) => void attention.ack(item.id)}
      onsnooze={(item) => void attention.snooze(item.id)}
    />
  </div>
</Drawer>

<CommandPalette
  open={ui.paletteOpen}
  {actions}
  recent={[]}
  onclose={() => (ui.paletteOpen = false)}
/>
<Toaster />

<style>
  .skip-link {
    position: absolute;
    top: -48px;
    left: 8px;
    z-index: 200;
    padding: 8px 12px;
    border-radius: var(--radius-control);
    background: var(--raised);
    color: var(--ink-12);
  }

  .skip-link:focus {
    top: 8px;
  }

  .app {
    display: flex;
    flex-direction: column;
    min-height: 100vh;
  }

  .body {
    display: flex;
    flex: 1;
    min-height: 0;
  }

  main {
    flex: 1;
    min-width: 0;
    padding: 24px 32px 32px;
    outline: none;
  }

  main:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }

  .rail-slot {
    display: none;
    min-height: 0;
  }

  .sheet-rail :global(.rail) {
    width: 100%;
    border-left: 0;
  }

  @media (min-width: 1280px) {
    .rail-slot {
      display: flex;
    }
  }

  @media (max-width: 1023px) {
    main {
      padding: 20px 24px 32px;
    }
  }

  @media (max-width: 767px) {
    main {
      padding: 16px 20px 72px;
    }
  }
</style>
