<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    title: string;
    description?: string;
    level?: 2 | 3;
    actions?: Snippet;
    children: Snippet;
  }

  let { title, description, level = 2, actions, children }: Props = $props();
  const heading = $derived(level === 2 ? 'h2' : 'h3');
  const uid = $props.id();
</script>

<section class="section" aria-labelledby={uid}>
  <div class="head">
    <svelte:element this={heading} id={uid} class="title">{title}</svelte:element>
    {#if description}<p>{description}</p>{/if}
  </div>
  {#if actions}<div class="actions">{@render actions()}</div>{/if}
  <div class="content">{@render children()}</div>
</section>

<style>
  .section {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 16px;
    margin: 32px 0;
  }

  .head {
    grid-column: 1;
  }

  .actions {
    grid-column: 2;
  }

  .content {
    grid-column: 1 / -1;
  }

  .title {
    margin: 0;
    font-size: 20px;
    line-height: 28px;
    font-weight: 500;
    color: var(--ink-12);
  }

  .title:is(h3) {
    font-size: 16px;
    line-height: 24px;
  }

  p {
    margin: 4px 0 0;
    font-size: 13px;
    line-height: 20px;
    color: var(--ink-11);
  }
</style>
