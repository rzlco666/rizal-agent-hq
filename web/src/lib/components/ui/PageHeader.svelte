<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    title: string;
    description?: string;
    meta?: string;
    actions?: Snippet;
  }

  let { title, description, meta, actions }: Props = $props();
</script>

<header class="page-header">
  <div class="copy">
    <h1>{title}</h1>
    {#if description}<p class="description">{description}</p>{/if}
    {#if meta}<p class="meta">{meta}</p>{/if}
  </div>
  {#if actions}<div class="actions">{@render actions()}</div>{/if}
</header>

<style>
  .page-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
  }

  h1 {
    margin: 0;
    font-size: 20px;
    line-height: 28px;
    font-weight: 500;
    color: var(--ink-12);
  }

  .description {
    margin: 4px 0 0;
    font-size: 13px;
    line-height: 20px;
    color: var(--ink-11);
  }

  .meta {
    margin: 4px 0 0;
    font-size: 12px;
    line-height: 16px;
    color: var(--ink-10);
  }

  @media (max-width: 767px) {
    .page-header {
      flex-direction: column;
    }

    .actions {
      width: 100%;
    }
  }
</style>
