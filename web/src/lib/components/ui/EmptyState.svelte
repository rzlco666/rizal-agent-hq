<script lang="ts">
  import type { Component } from 'svelte';
  import Button from './Button.svelte';

  interface Props {
    title: string;
    body: string;
    icon?: Component;
    action?: { label: string; onclick: () => void };
  }

  let { title, body, icon: Icon, action }: Props = $props();
</script>

<div class="empty-state">
  {#if Icon}<span class="icon"><Icon size={18} strokeWidth={1.5} /></span>{/if}
  <p class="title">{title}</p>
  <p class="body">{body}</p>
  {#if action}
    <Button variant="secondary" onclick={action.onclick}>{action.label}</Button>
  {/if}
</div>

<style>
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    max-width: 360px;
    margin: 0 auto;
    padding: 32px;
    text-align: center;
  }

  .icon {
    display: inline-flex;
    margin-bottom: 8px;
    color: var(--ink-10);
  }

  p {
    margin: 0;
  }

  .title {
    font-size: 13px;
    line-height: 20px;
    color: var(--ink-11);
  }

  .body {
    font-size: 13px;
    line-height: 20px;
    color: var(--ink-10);
  }

  :global(button) {
    margin-top: 12px;
  }
</style>
