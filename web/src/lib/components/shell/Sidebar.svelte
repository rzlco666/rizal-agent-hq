<script lang="ts">
  import { page } from '$app/state';
  import { Ellipsis } from 'lucide-svelte';
  import { t } from '$lib/i18n';
  import Drawer from '../ui/Drawer.svelte';
  import { isActive, mobileNavIds, navItems, type NavItem } from './nav';

  let moreOpen = $state(false);

  const mobileItems = $derived(navItems.filter((item) => mobileNavIds.includes(item.id)));
  const otherItems = $derived(navItems.filter((item) => !mobileNavIds.includes(item.id)));

  function link(item: NavItem) {
    return t(item.labelKey);
  }
</script>

<nav class="sidebar" data-testid="nav-rail" aria-label={t('nav.main')}>
  <p class="section-label">{t('nav.main')}</p>
  <ul>
    {#each navItems as item (item.id)}
      {@const active = isActive(page.url.pathname, item.href)}
      <li>
        <a href={item.href} aria-current={active ? 'page' : undefined} class:active>
          <span class="icon"><item.icon size={16} strokeWidth={1.5} /></span>
          <span class="label">{link(item)}</span>
        </a>
      </li>
    {/each}
  </ul>
</nav>

<nav class="bottom-nav" data-testid="bottom-nav" aria-label={t('nav.main')}>
  {#each mobileItems as item (item.id)}
    {@const active = isActive(page.url.pathname, item.href)}
    <a href={item.href} aria-current={active ? 'page' : undefined} class:active>
      <item.icon size={16} strokeWidth={1.5} />
      <span>{link(item)}</span>
    </a>
  {/each}
  <button type="button" onclick={() => (moreOpen = true)} aria-expanded={moreOpen}>
    <Ellipsis size={16} strokeWidth={1.5} />
    <span>{t('nav.more')}</span>
  </button>
</nav>

<Drawer open={moreOpen} title={t('nav.more')} onclose={() => (moreOpen = false)}>
  <ul class="more-list">
    {#each otherItems as item (item.id)}
      <li>
        <a href={item.href} onclick={() => (moreOpen = false)}>
          <item.icon size={16} strokeWidth={1.5} />
          <span>{link(item)}</span>
        </a>
      </li>
    {/each}
  </ul>
</Drawer>

<style>
  .sidebar {
    display: none;
    flex-direction: column;
    width: 248px;
    flex: none;
    padding: 16px 8px;
    border-right: 1px solid var(--hairline);
    background: var(--surface);
  }

  .section-label {
    margin: 0 0 8px 8px;
    font-size: 12px;
    line-height: 16px;
    color: var(--ink-10);
  }

  ul {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .sidebar a {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 32px;
    padding: 0 8px;
    border-radius: var(--radius-control);
    color: var(--ink-11);
    text-decoration: none;
  }

  .sidebar a.active {
    background: var(--raised);
    color: var(--ink-12);
    box-shadow: inset 2px 0 0 var(--attention);
  }

  .icon {
    display: inline-flex;
    width: 24px;
    flex: none;
  }

  .label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .bottom-nav {
    position: fixed;
    bottom: 0;
    left: 0;
    right: 0;
    z-index: 40;
    display: flex;
    height: 56px;
    align-items: center;
    justify-content: space-around;
    border-top: 1px solid var(--hairline);
    background: var(--surface);
  }

  .bottom-nav a,
  .bottom-nav button {
    display: flex;
    flex: 1;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    height: 100%;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--ink-10);
    font-size: 11px;
    line-height: 14px;
    text-decoration: none;
  }

  .bottom-nav a.active {
    color: var(--ink-12);
    box-shadow: inset 0 2px 0 var(--attention);
  }

  .more-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .more-list a {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 44px;
    padding: 0 8px;
    border-radius: var(--radius-control);
    color: var(--ink-12);
    text-decoration: none;
  }

  .more-list a:hover {
    background: var(--hover);
  }

  @media (min-width: 768px) {
    .sidebar {
      display: flex;
    }

    .bottom-nav {
      display: none;
    }
  }

  @media (min-width: 768px) and (max-width: 1279px) {
    .sidebar {
      width: 64px;
    }

    .sidebar .label,
    .section-label {
      display: none;
    }

    .icon {
      width: 100%;
      justify-content: center;
    }
  }
</style>
